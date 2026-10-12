//! Anthropic Messages to OpenAI Responses request conversion.

use super::reasoning_bridge::openai_reasoning_item_from_anthropic_block;
use super::transform_content::{
    converted_model, function_tools, message_role, responses_part, system_to_text,
    tool_result_output,
};
use serde_json::{json, Value};

pub fn anthropic_to_codex_responses(
    body: Value,
    cache_key: Option<&str>,
    codex_fast_mode: bool,
) -> Result<Value, String> {
    anthropic_to_responses(body, cache_key, true, codex_fast_mode)
}

pub fn anthropic_to_responses(
    body: Value,
    cache_key: Option<&str>,
    codex_oauth: bool,
    codex_fast_mode: bool,
) -> Result<Value, String> {
    let mut result = json!({});

    if let Some(model) = body.get("model").and_then(Value::as_str) {
        result["model"] = json!(converted_model(model));
    }

    if let Some(system) = body.get("system") {
        let instructions = system_to_text(system);
        if !instructions.is_empty() {
            result["instructions"] = json!(instructions);
        }
    }

    if let Some(messages) = body.get("messages").and_then(Value::as_array) {
        result["input"] = json!(messages_to_input(messages, codex_oauth)?);
    }

    let mut tools = function_tools(&body, false)?;
    for tool in &mut tools {
        tool["parameters"] = clean_schema(tool["parameters"].clone());
    }
    if !tools.is_empty() {
        result["tools"] = json!(tools);
    }

    if let Some(tool_choice) = body.get("tool_choice") {
        result["tool_choice"] = map_tool_choice(tool_choice);
        if codex_oauth {
            if let Some(disable) = tool_choice
                .get("disable_parallel_tool_use")
                .and_then(Value::as_bool)
            {
                result["parallel_tool_calls"] = json!(!disable);
            }
        }
    }

    if let Some(model) = result.get("model").and_then(Value::as_str) {
        if supports_reasoning_effort(model) {
            if let Some(effort) = resolve_reasoning_effort(&body) {
                result["reasoning"] = json!({ "effort": effort });
            }
        }
    }

    if let Some(cache_key) = cache_key {
        result["prompt_cache_key"] = json!(cache_key);
    }
    if codex_oauth {
        result["store"] = json!(false);
        let mut include = body
            .get("include")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if !include
            .iter()
            .any(|value| value.as_str() == Some("reasoning.encrypted_content"))
        {
            include.push(json!("reasoning.encrypted_content"));
        }
        result["include"] = json!(include);
        result["stream"] = json!(true);
        if codex_fast_mode {
            result["service_tier"] = json!("priority");
        }
        let object = result
            .as_object_mut()
            .ok_or("Responses request must be an object")?;
        object.entry("instructions").or_insert(json!(""));
        object.entry("input").or_insert(json!([]));
        object.entry("tools").or_insert(json!([]));
        object.entry("parallel_tool_calls").or_insert(json!(true));
    } else {
        result["stream"] = json!(body.get("stream").and_then(Value::as_bool).unwrap_or(false));
        if let Some(budget) = body.get("max_tokens").and_then(Value::as_u64) {
            result["max_output_tokens"] = json!(budget.max(16));
        }
        for key in ["temperature", "top_p"] {
            if let Some(value) = body.get(key) {
                result[key] = value.clone();
            }
        }
        if let Some(format) = body.pointer("/output_config/format") {
            let mut format = format.clone();
            if format.get("type").and_then(Value::as_str) == Some("json_schema") {
                format
                    .as_object_mut()
                    .ok_or("Output format must be an object")?
                    .entry("name")
                    .or_insert(json!("response"));
            }
            result["text"] = json!({"format":format});
        }
    }

    Ok(result)
}

fn messages_to_input(messages: &[Value], codex_oauth: bool) -> Result<Vec<Value>, String> {
    let mut input = Vec::new();

    for message in messages {
        let role = message_role(message)?;
        // Claude Code can inject mid-conversation system reminders. The
        // ChatGPT Codex backend rejects system messages; retain their text and
        // position as developer instructions instead of hoisting or dropping.
        let role = if codex_oauth && role == "system" {
            "developer"
        } else {
            role
        };
        let message_start = input.len();
        let content = message.get("content").cloned().unwrap_or(Value::Null);

        if let Some(text) = content.as_str() {
            input.push(json!({
                "type": "message",
                "role": role,
                "content": [{ "type": input_text_type(role), "text": text }],
            }));
            continue;
        }

        let Some(blocks) = content.as_array() else {
            input.push(json!({
                "type": "message",
                "role": role,
                "content": [],
            }));
            continue;
        };

        let mut message_content = Vec::new();
        for block in blocks {
            match block.get("type").and_then(Value::as_str) {
                Some("text") => {
                    message_content.push(json!({
                        "type": input_text_type(role),
                        "text": block.get("text").and_then(Value::as_str).unwrap_or(""),
                    }));
                }
                Some("image" | "document") if role != "assistant" => {
                    message_content.push(responses_part(block));
                }
                Some("tool_use") => {
                    flush_content(&mut input, &mut message_content, role);
                    input.push(json!({
                        "type": "function_call",
                        "call_id": block.get("id").and_then(Value::as_str).unwrap_or(""),
                        "name": block.get("name").and_then(Value::as_str).unwrap_or(""),
                        "arguments": block.get("input").cloned().unwrap_or_else(|| json!({})).to_string(),
                    }));
                }
                Some("tool_result") => {
                    flush_content(&mut input, &mut message_content, role);
                    input.push(json!({
                        "type": "function_call_output",
                        "call_id": block.get("tool_use_id").and_then(Value::as_str).unwrap_or(""),
                        "output": tool_result_output(block),
                    }));
                }
                Some("thinking") | Some("redacted_thinking") => {
                    if let Some(reasoning) = openai_reasoning_item_from_anthropic_block(block) {
                        flush_content(&mut input, &mut message_content, role);
                        input.push(reasoning);
                    }
                }
                other => {
                    return Err(format!(
                        "Unsupported Messages content block for Responses: {other:?}"
                    ))
                }
            }
        }

        if !message_content.is_empty() {
            input.push(json!({
                "type": "message",
                "role": role,
                "content": message_content,
            }));
        }
        // An incomplete reasoning-only assistant turn has no valid following
        // generated item; replaying it would make the next Responses call fail.
        if role == "assistant" {
            let mut follower = false;
            for index in (message_start..input.len()).rev() {
                if input[index]["type"] == "reasoning" {
                    if !follower {
                        input.remove(index);
                    }
                } else if input[index]["type"] == "function_call"
                    || input[index]["role"] == "assistant"
                {
                    follower = true;
                }
            }
        }
    }

    Ok(input)
}

fn clean_schema(mut schema: Value) -> Value {
    if let Some(object) = schema.as_object_mut() {
        if object.get("format").and_then(Value::as_str) == Some("uri") {
            object.remove("format");
        }
        if let Some(properties) = object.get_mut("properties").and_then(Value::as_object_mut) {
            for value in properties.values_mut() {
                *value = clean_schema(value.clone());
            }
        }
        if let Some(items) = object.get_mut("items") {
            *items = clean_schema(items.clone());
        }
    }
    schema
}

pub fn is_openai_o_series(model: &str) -> bool {
    model.len() > 1
        && model.starts_with('o')
        && model.as_bytes().get(1).is_some_and(|b| b.is_ascii_digit())
}

/// Detect Responses-compatible models that support reasoning effort.
///
/// Supported families:
/// - o-series: o1, o3, o4-mini, etc.
/// - GPT-5+: gpt-5, gpt-5.1, gpt-5.4, gpt-5-codex, etc.
/// - xAI Grok 4.5+ (`grok-4.x` with numeric minor version x ≥ 5,
///   so future releases like grok-4.10 need no whitelist update); retain the
///   previous `grok-build-*` family for saved providers.
pub fn supports_reasoning_effort(model: &str) -> bool {
    let normalized = model.to_lowercase();
    is_openai_o_series(&normalized)
        || normalized
            .strip_prefix("gpt-")
            .and_then(|rest| rest.chars().next())
            .is_some_and(|c| c.is_ascii_digit() && c >= '5')
        || normalized
            .strip_prefix("grok-4.")
            .and_then(|rest| rest.split(|c: char| !c.is_ascii_digit()).next())
            .and_then(|minor| minor.parse::<u32>().ok())
            .is_some_and(|minor| minor >= 5)
        || normalized.starts_with("grok-build-")
}

/// Detect models whose OpenAI reasoning effort supports a distinct `max` tier.
fn supports_max_reasoning_effort(model: &str) -> bool {
    let normalized = model.to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "gpt-5.6" | "gpt-5.6-sol" | "gpt-5.6-terra" | "gpt-5.6-luna"
    ) || matches!(
        normalized.as_str(),
        "gpt-6-astra" | "gpt-6-sol" | "gpt-6-luna"
    )
}

/// Resolve the appropriate OpenAI `reasoning_effort` from an Anthropic request body.
///
/// Priority:
/// 1. Explicit `output_config.effort` — preserves the user's intent directly.
///    `low`/`medium`/`high`/`xhigh` map 1:1 (`xhigh` is what Claude Code's
///    `/effort xhigh` sends); `max` stays `max` for models that support a
///    distinct max tier, otherwise it falls back to `xhigh`. Unknown values are ignored.
///    When thinking is off (`thinking` absent or `disabled`) the effort drops to
///    the lowest tier: Claude Code keeps sending `effort` with thinking turned
///    off (and Workflow subagents send `disabled` + effort), and forwarding it
///    verbatim would make OpenAI/xAI reason at full strength anyway. `none` /
///    `minimal` are not accepted by every whitelisted model, so the floor is `low`.
///    Models whose omitted `thinking` still means adaptive (the 5.x Claude models) are
///    made explicit before the transform by
///    [`make_default_adaptive_thinking_explicit`], since the mapped model name no
///    longer carries that default.
/// 2. Fallback: `thinking.type` + `budget_tokens`:
///    - `adaptive` → `xhigh` (adaptive = maximum reasoning effort)
///    - `enabled` with budget → `low` (<4 000) / `medium` (4 000–15 999) / `high` (≥16 000)
///    - `enabled` without budget → `high` (conservative default)
///    - `disabled` / absent → `None`
///
/// The result is finally clamped to the tiers the target model accepts
/// (see [`clamp_reasoning_effort_for_model`]).
pub fn resolve_reasoning_effort(body: &Value) -> Option<&'static str> {
    let model = body
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or_default();
    resolve_requested_reasoning_effort(body, converted_model(model))
        .map(|effort| clamp_reasoning_effort_for_model(converted_model(model), effort))
}

fn resolve_requested_reasoning_effort(body: &Value, model: &str) -> Option<&'static str> {
    // --- Priority 1: explicit output_config.effort ---
    if let Some(effort) = body
        .pointer("/output_config/effort")
        .and_then(|v| v.as_str())
    {
        let mapped = match effort {
            "low" => Some("low"),
            "medium" => Some("medium"),
            "high" => Some("high"),
            "xhigh" => Some("xhigh"),
            "max" if supports_max_reasoning_effort(model) => Some("max"),
            "max" => Some("xhigh"),
            _ => None, // unknown value — do not inject
        };
        let thinking_off = matches!(
            body.pointer("/thinking/type").and_then(|t| t.as_str()),
            None | Some("disabled")
        );
        return if thinking_off {
            mapped.map(|_| "low")
        } else {
            mapped
        };
    }

    // --- Priority 2: thinking.type + budget_tokens fallback ---
    let thinking = body.get("thinking")?;
    match thinking.get("type").and_then(|t| t.as_str()) {
        Some("adaptive") => Some("xhigh"),
        Some("enabled") => {
            let budget = thinking.get("budget_tokens").and_then(|b| b.as_u64());
            match budget {
                Some(b) if b < 4_000 => Some("low"),
                Some(b) if b < 16_000 => Some("medium"),
                Some(_) => Some("high"),
                None => Some("high"), // enabled but no budget — assume strong reasoning
            }
        }
        _ => None, // disabled or missing
    }
}

/// Clamp an effort to the tiers the target model accepts (OpenAI model pages):
/// `gpt-5-pro` only accepts `high`; the later `gpt-5.x-pro` models accept
/// `medium`/`high`/`xhigh`, so `low` is raised to `medium`.
fn clamp_reasoning_effort_for_model(model: &str, effort: &'static str) -> &'static str {
    let normalized = model.to_ascii_lowercase();
    if normalized == "gpt-5-pro" || normalized.starts_with("gpt-5-pro-") {
        return "high";
    }
    let is_gpt_pro = normalized.starts_with("gpt-")
        && (normalized.ends_with("-pro") || normalized.contains("-pro-"));
    if is_gpt_pro && effort == "low" {
        return "medium";
    }
    effort
}

// Keep upstream's omitted-thinking defaults before replacing the client model.
pub fn make_default_adaptive_thinking_explicit(body: &mut Value, client_model: Option<&str>) {
    let Some(model) = client_model else {
        return;
    };
    let model = model.trim().to_ascii_lowercase().replace(['.', '_'], "-");
    if body
        .get("thinking")
        .is_some_and(|thinking| !thinking.is_null())
        || ![
            "fable-5",
            "mythos-5",
            "mythos-preview",
            "opus-5",
            "sonnet-5",
        ]
        .iter()
        .any(|name| model.contains(name))
    {
        return;
    }
    if let Some(body) = body.as_object_mut() {
        body.insert("thinking".into(), json!({"type":"adaptive"}));
    }
}

fn flush_content(input: &mut Vec<Value>, content: &mut Vec<Value>, role: &str) {
    if !content.is_empty() {
        input.push(json!({"type":"message", "role":role, "content":std::mem::take(content)}));
    }
}

fn input_text_type(role: &str) -> &str {
    match role {
        "assistant" => "output_text",
        _ => "input_text",
    }
}

fn map_tool_choice(tool_choice: &Value) -> Value {
    match tool_choice {
        Value::String(_) => tool_choice.clone(),
        Value::Object(object) => match object.get("type").and_then(Value::as_str) {
            Some("any") => json!("required"),
            Some("auto") => json!("auto"),
            Some("none") => json!("none"),
            Some("tool") => json!({
                "type": "function",
                "name": object.get("name").and_then(Value::as_str).unwrap_or(""),
            }),
            _ => tool_choice.clone(),
        },
        _ => tool_choice.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_oauth_maps_mid_turn_system_to_developer_without_reordering() {
        let body = json!({"model":"gpt-6.1-sol","system":"identity and rules","messages":[{"role":"user","content":"task"},{"role":"system","content":[{"type":"text","text":"runtime reminder","cache_control":{"type":"ephemeral"}}]},{"role":"assistant","content":"answer"},{"role":"system","content":"updated reminder"}]});
        let codex = anthropic_to_codex_responses(body.clone(), None, false).unwrap();
        let public = anthropic_to_responses(body, None, false, false).unwrap();
        assert_eq!(codex["instructions"], "identity and rules");
        assert_eq!(codex["input"][0]["role"], "user");
        assert_eq!(codex["input"][1]["role"], "developer");
        assert_eq!(codex["input"][1]["content"][0]["text"], "runtime reminder");
        assert_eq!(codex["input"][2]["role"], "assistant");
        assert_eq!(codex["input"][3]["role"], "developer");
        assert_eq!(codex["input"][3]["content"][0]["text"], "updated reminder");
        assert_eq!(public["input"][1]["role"], "system");
        assert_eq!(public["input"][3]["role"], "system");
    }

    #[test]
    fn restores_encrypted_reasoning_in_order_and_drops_orphans() {
        use crate::proxy::responses_aggregate::responses_json_to_anthropic_message;
        for summary in [
            json!([]),
            json!([{"type":"summary_text","text":"Use Read."}]),
        ] {
            let item = json!({"type":"reasoning","id":"rs_1","summary":summary,"encrypted_content":"opaque","status":"completed","content":[{"text":"output-only"}]});
            let response = responses_json_to_anthropic_message(&json!({"status":"completed","output":[item,{"type":"function_call","call_id":"call_1","name":"Read","arguments":"{}"}]})).unwrap();
            let body = json!({"model":"gpt-5","messages":[{"role":"assistant","content":response["content"]},{"role":"user","content":[{"type":"tool_result","tool_use_id":"call_1","content":"OK"}]}]});
            let converted = anthropic_to_responses(body, None, false, false).unwrap();
            assert_eq!(
                converted["input"][0],
                json!({"type":"reasoning","id":"rs_1","summary":summary,"encrypted_content":"opaque"})
            );
            assert_eq!(converted["input"][1]["type"], "function_call");
            assert_eq!(converted["input"][2]["type"], "function_call_output");
            let orphan = anthropic_to_responses(
                json!({"messages":[{"role":"assistant","content":[response["content"][0]]}]}),
                None,
                false,
                false,
            )
            .unwrap();
            assert_eq!(orphan["input"], json!([]));
        }
        let foreign = anthropic_to_responses(json!({"messages":[{"role":"assistant","content":[{"type":"thinking","thinking":"private","signature":"foreign-vendor"},{"type":"text","text":"answer"}]}]}), None, false, false).unwrap();
        assert_eq!(foreign["input"].as_array().unwrap().len(), 1);
        assert_eq!(foreign["input"][0]["content"][0]["text"], "answer");
    }

    #[test]
    fn upstream_effort_and_client_defaults_survive_model_mapping() {
        for (model, thinking, effort, expected) in [
            ("gpt-5", "adaptive", "high", "high"),
            ("gpt-5", "disabled", "high", "low"),
            ("gpt-5-pro", "adaptive", "low", "high"),
            ("gpt-5.6-pro", "disabled", "high", "medium"),
        ] {
            let body = json!({"model":model,"thinking":{"type":thinking},"output_config":{"effort":effort}});
            assert_eq!(resolve_reasoning_effort(&body), Some(expected));
        }
        let mut body = json!({"output_config":{"effort":"high"}});
        make_default_adaptive_thinking_explicit(&mut body, Some("claude-opus-5-5"));
        body["model"] = json!("gpt-5");
        assert_eq!(resolve_reasoning_effort(&body), Some("high"));
        let mut disabled = json!({"thinking":{"type":"disabled"}});
        make_default_adaptive_thinking_explicit(&mut disabled, Some("claude-opus-5-5"));
        assert_eq!(disabled["thinking"]["type"], "disabled");
        let mut old = json!({});
        make_default_adaptive_thinking_explicit(&mut old, Some("claude-opus-4-8"));
        assert!(old.get("thinking").is_none());
    }

    #[test]
    fn codex_parallel_choice_and_includes_match_upstream_contract() {
        for disable in [None, Some(false), Some(true)] {
            let mut body = json!({"model":"gpt-5","include":["existing.marker"],"tool_choice":{"type":"auto"}});
            if let Some(disable) = disable {
                body["tool_choice"]["disable_parallel_tool_use"] = json!(disable);
            }
            let converted = anthropic_to_codex_responses(body, None, false).unwrap();
            assert_eq!(
                converted["parallel_tool_calls"],
                json!(!disable.unwrap_or(false))
            );
            assert_eq!(
                converted["include"],
                json!(["existing.marker", "reasoning.encrypted_content"])
            );
        }
    }

    #[test]
    fn preserves_prompt_roles_and_order_around_tool_blocks() {
        let converted = anthropic_to_responses(json!({"messages":[
            {"role":"system","content":"mid-turn policy"},
            {"role":"assistant","content":[{"type":"text","text":"before"},{"type":"tool_use","id":"call","name":"Read","input":{}},{"type":"text","text":"after"}]},
            {"role":"user","content":[{"type":"tool_result","tool_use_id":"call","is_error":true,"content":[{"type":"tool_reference","tool_name":"mcp__audit__echo"},{"type":"image","source":{"type":"base64","media_type":"image/png","data":"fixture"}}]},{"type":"text","text":"<system-reminder>retain this</system-reminder>"}]}
        ]}), None, false, false).unwrap();
        let input = &converted["input"];
        assert_eq!(input[0]["role"], "system");
        assert_eq!(input[1]["content"][0]["text"], "before");
        assert_eq!(input[2]["type"], "function_call");
        assert_eq!(input[3]["content"][0]["text"], "after");
        assert!(input[4]["output"][0]["text"]
            .as_str()
            .unwrap()
            .contains("error"));
        assert!(input[4]["output"][1]["text"]
            .as_str()
            .unwrap()
            .contains("mcp__audit__echo"));
        assert_eq!(input[4]["output"][2]["type"], "input_image");
        assert_eq!(input[5]["role"], "user");
        assert_eq!(
            input[5]["content"][0]["text"],
            "<system-reminder>retain this</system-reminder>"
        );
    }

    #[test]
    fn public_responses_keeps_budget_format_and_stream_choice() {
        let body = json!({"model":"gpt-5", "max_tokens":800, "temperature":0.5, "stream":false,
            "thinking":{"type":"adaptive"}, "output_config":{"effort":"high","format":{"type":"json_schema","schema":{"type":"object","properties":{}}}},
            "tools":[{"name":"Read","input_schema":{"type":"object","properties":{"offset":{"type":"integer"}}},"defer_loading":true}]});
        let public = anthropic_to_responses(body.clone(), None, false, true).unwrap();
        assert_eq!(public["max_output_tokens"], 800);
        assert_eq!(public["temperature"], 0.5);
        assert_eq!(public["stream"], false);
        assert_eq!(public["text"]["format"]["name"], "response");
        assert_eq!(public["tools"][0]["strict"], false);
        assert!(public["tools"][0].get("description").is_none());
        assert!(public.get("service_tier").is_none());
        let codex = anthropic_to_codex_responses(body, None, false).unwrap();
        assert_eq!(codex["stream"], true);
        assert_eq!(codex["store"], false);
        assert!(codex.get("max_output_tokens").is_none());
    }

    #[test]
    fn converts_codex_request_shape() {
        let body = json!({
            "model": "gpt-5.4",
            "system": "Be concise",
            "max_tokens": 128,
            "temperature": 0.3,
            "messages": [
                {"role": "user", "content": "hello"}
            ],
            "tools": [
                {"name": "lookup", "description": "Lookup", "input_schema": {"type": "object"}}
            ]
        });

        let converted = anthropic_to_codex_responses(body, Some("session"), false).unwrap();
        assert_eq!(converted["model"], "gpt-5.4");
        assert_eq!(converted["instructions"], "Be concise");
        assert_eq!(converted["store"], false);
        assert_eq!(converted["stream"], true);
        assert_eq!(converted["include"], json!(["reasoning.encrypted_content"]));
        assert_eq!(converted["prompt_cache_key"], "session");
        assert!(converted.get("max_output_tokens").is_none());
        assert!(converted.get("temperature").is_none());
        assert_eq!(converted["input"][0]["content"][0]["type"], "input_text");
        assert_eq!(converted["tools"][0]["type"], "function");
    }

    #[test]
    fn can_omit_prompt_cache_key() {
        let body = json!({
            "model": "gpt-5.4",
            "messages": [{"role": "user", "content": "hello"}]
        });

        let converted = anthropic_to_codex_responses(body, None, false).unwrap();

        assert!(converted.get("prompt_cache_key").is_none());
    }

    #[test]
    fn fast_mode_sets_priority_service_tier() {
        let body = json!({
            "model": "gpt-5.4",
            "messages": [{"role": "user", "content": "hello"}]
        });

        let converted = anthropic_to_codex_responses(body, Some("session"), true).unwrap();

        assert_eq!(converted["service_tier"], "priority");
    }

    #[test]
    fn codex_oauth_strips_unsupported_fields_and_forces_stream() {
        let body = json!({
            "model": "gpt-5.4",
            "max_output_tokens": 1024,
            "temperature": 0.3,
            "top_p": 0.9,
            "stream": false,
            "messages": [{"role": "user", "content": "hello"}]
        });

        let converted = anthropic_to_codex_responses(body, Some("session"), false).unwrap();

        assert_eq!(converted["stream"], true);
        assert!(converted.get("max_output_tokens").is_none());
        assert!(converted.get("temperature").is_none());
        assert!(converted.get("top_p").is_none());
    }

    #[test]
    fn lifts_tool_result_items() {
        let body = json!({
            "messages": [{
                "role": "user",
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": "call_1",
                    "content": "done"
                }]
            }]
        });

        let converted = anthropic_to_codex_responses(body, Some("session"), false).unwrap();
        assert_eq!(converted["input"][0]["type"], "function_call_output");
        assert_eq!(converted["input"][0]["call_id"], "call_1");
    }

    #[test]
    fn strips_uri_format_and_maps_reasoning() {
        let body = json!({
            "model": "gpt-5.4",
            "thinking": { "type": "enabled", "budget_tokens": 8000 },
            "messages": [{"role": "user", "content": "hello"}],
            "tools": [{
                "name": "open",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "url": { "type": "string", "format": "uri" }
                    }
                }
            }]
        });

        let converted = anthropic_to_codex_responses(body, Some("session"), false).unwrap();
        assert_eq!(converted["reasoning"]["effort"], "medium");
        assert!(converted["tools"][0]["parameters"]["properties"]["url"]
            .get("format")
            .is_none());
    }

    #[test]
    fn converts_user_image_blocks() {
        let body = json!({
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": "describe"},
                    {"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": "abc"}}
                ]
            }]
        });

        let converted = anthropic_to_codex_responses(body, Some("session"), false).unwrap();
        assert_eq!(converted["input"][0]["content"][1]["type"], "input_image");
        assert_eq!(
            converted["input"][0]["content"][1]["image_url"],
            "data:image/png;base64,abc"
        );
    }
}

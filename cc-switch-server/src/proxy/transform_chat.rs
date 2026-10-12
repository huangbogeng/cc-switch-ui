//! Common Messages -> Chat conversion. Authentication remains provider-specific.
use super::transform_content::{
    converted_model, function_tools, image_url, message_role, system_to_text, tool_result_output,
};
use super::transform_responses::{resolve_reasoning_effort, supports_reasoning_effort};
use serde_json::{json, Value};

pub fn anthropic_to_openai_chat(
    body: Value,
    requested_stream: bool,
    preserve_reasoning: bool,
) -> Result<Value, String> {
    let mut result = json!({});
    if let Some(model) = body.get("model").and_then(Value::as_str) {
        result["model"] = json!(converted_model(model));
    }
    let mut messages = Vec::new();
    if let Some(system) = body.get("system") {
        let text = system_to_text(system);
        if !text.is_empty() {
            messages.push(json!({"role":"system", "content":text}));
        }
    }
    for message in body
        .get("messages")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let role = message_role(message)?;
        let content = message.get("content").unwrap_or(&Value::Null);
        if let Some(text) = content.as_str() {
            messages.push(json!({"role":role,"content":text}));
            continue;
        }
        let mut parts = Vec::new();
        let mut tool_calls = Vec::new();
        let mut media = Vec::new();
        let mut reasoning = Vec::new();
        for block in content.as_array().into_iter().flatten() {
            match block.get("type").and_then(Value::as_str) {
                Some("text") => parts.push(json!({"type":"text","text":block.get("text").and_then(Value::as_str).unwrap_or("")})),
                Some("image") => {
                    let url = image_url(block).ok_or("Unsupported image source")?;
                    parts.push(json!({"type":"image_url","image_url":{"url":url}}));
                }
                Some("tool_use") => tool_calls.push(json!({"id":block.get("id"),"type":"function","function":{"name":block.get("name"),"arguments":block.get("input").cloned().unwrap_or_else(||json!({})).to_string()}})),
                Some("tool_result") => {
                    let output = tool_result_output(block);
                    if output.as_array().is_some_and(|parts| parts.iter().any(|part| part["type"] == "input_file")) {
                        return Err("Chat tool results cannot carry documents; use a Responses or Anthropic route".into());
                    }
                    let text = if let Some(text) = output.as_str() { text.to_string() }
                    else {
                        output.as_array().into_iter().flatten().map(|part| {
                            if part["type"] == "input_image" {
                                media.push(json!({"type":"image_url","image_url":{"url":part["image_url"]}}));
                                "[tool result image attached]".to_string()
                            } else { part.get("text").and_then(Value::as_str).map(str::to_string).unwrap_or_else(||part.to_string()) }
                        }).collect::<Vec<_>>().join("\n")
                    };
                    messages.push(json!({"role":"tool","tool_call_id":block.get("tool_use_id"),"content":text}));
                }
                Some("thinking") if preserve_reasoning => {
                    if let Some(text) = block.get("thinking").and_then(Value::as_str) { reasoning.push(text); }
                }
                Some("thinking" | "redacted_thinking") => {},
                // A Chat endpoint has no native document or server-tool carrier.
                // Refuse rather than silently dropping meaningful input.
                other => return Err(format!("Unsupported Messages content block for Chat: {other:?}")),
            }
        }
        // Chat requires results directly after the assistant's tool calls.
        // Sibling reminders remain a user message, never inside tool output.
        if !media.is_empty() {
            messages.push(json!({"role":"user","content":media}));
        }
        if !parts.is_empty() || !tool_calls.is_empty() {
            let content = if parts.iter().all(|p| p["type"] == "text") {
                if parts.is_empty() {
                    Value::Null
                } else {
                    json!(parts
                        .iter()
                        .filter_map(|p| p["text"].as_str())
                        .collect::<Vec<_>>()
                        .join("\n\n"))
                }
            } else {
                json!(parts)
            };
            let mut output = json!({"role":role,"content":content});
            if !tool_calls.is_empty() {
                output["tool_calls"] = json!(tool_calls);
            }
            if role == "assistant" && !reasoning.is_empty() {
                output["reasoning_content"] = json!(reasoning.join("\n\n"));
            }
            messages.push(output);
        }
    }
    result["messages"] = json!(messages);
    for key in ["max_tokens", "temperature", "top_p"] {
        if let Some(value) = body.get(key) {
            let target = if key == "max_tokens"
                && body.get("model").and_then(Value::as_str).is_some_and(|m| {
                    m.starts_with("o1") || m.starts_with("o3") || m.starts_with("o4")
                }) {
                "max_completion_tokens"
            } else {
                key
            };
            result[target] = value.clone();
        }
    }
    if let Some(stop) = body.get("stop_sequences") {
        result["stop"] = stop.clone();
    }
    let tools = function_tools(&body, true)?;
    if !tools.is_empty() {
        result["tools"] = json!(tools);
    }
    if let Some(choice) = body.get("tool_choice") {
        result["tool_choice"] = match choice.get("type").and_then(Value::as_str) {
            Some("any") => json!("required"),
            Some("auto") => json!("auto"),
            Some("none") => json!("none"),
            Some("tool") => json!({"type":"function","function":{"name":choice.get("name")}}),
            _ => choice.clone(),
        };
    }
    if body
        .get("model")
        .and_then(Value::as_str)
        .is_some_and(supports_reasoning_effort)
    {
        if let Some(effort) = resolve_reasoning_effort(&body) {
            result["reasoning_effort"] = json!(effort);
        }
    }
    result["stream"] = json!(requested_stream);
    if requested_stream {
        result["stream_options"] = json!({"include_usage":true});
    }
    Ok(result)
}

pub fn chat_url(base: &str) -> String {
    let base = base.trim_end_matches('/');
    if base.ends_with("/chat/completions") {
        return base.to_string();
    }
    if let Some(base) = base.strip_suffix("/anthropic") {
        return format!("{base}/v1/chat/completions");
    }
    if base.ends_with("/v1") {
        format!("{base}/chat/completions")
    } else {
        format!("{base}/v1/chat/completions")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resolves_prefixed_gateway_base_urls() {
        assert_eq!(
            chat_url("https://openrouter.ai/api"),
            "https://openrouter.ai/api/v1/chat/completions"
        );
        assert_eq!(
            chat_url("https://example.com/api/v1"),
            "https://example.com/api/v1/chat/completions"
        );
    }
    #[test]
    fn keeps_system_role_and_tool_search_result_with_sibling_reminder() {
        let converted=anthropic_to_openai_chat(json!({"messages":[{"role":"system","content":"mid-turn rule"},{"role":"user","content":[{"type":"tool_result","tool_use_id":"search","content":[{"type":"tool_reference","tool_name":"mcp__test__read"}]},{"type":"text","text":"<system-reminder>keep this</system-reminder>"}]}]}),true,false).unwrap();
        assert_eq!(converted["messages"][0]["role"], "system");
        assert!(converted["messages"][1]["content"]
            .as_str()
            .unwrap()
            .contains("mcp__test__read"));
        assert_eq!(converted["messages"][2]["role"], "user");
        assert_eq!(
            converted["messages"][2]["content"],
            "<system-reminder>keep this</system-reminder>"
        );
    }
}

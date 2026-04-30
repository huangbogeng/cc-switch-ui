//! Anthropic Messages to OpenAI Responses request conversion.

use serde_json::{json, Value};

pub fn anthropic_to_codex_responses(body: Value, cache_key: &str) -> Result<Value, String> {
    let mut result = json!({});
    let _ = cache_key;

    if let Some(model) = body.get("model").and_then(Value::as_str) {
        result["model"] = json!(model);
    }

    if let Some(system) = body.get("system") {
        let instructions = system_to_text(system);
        if !instructions.is_empty() {
            result["instructions"] = json!(instructions);
        }
    }

    if let Some(messages) = body.get("messages").and_then(Value::as_array) {
        result["input"] = json!(messages_to_input(messages)?);
    }

    if let Some(tools) = body.get("tools").and_then(Value::as_array) {
        let converted_tools: Vec<Value> = tools
            .iter()
            .filter(|tool| tool.get("type").and_then(Value::as_str) != Some("BatchTool"))
            .map(|tool| {
                json!({
                    "type": "function",
                    "name": tool.get("name").and_then(Value::as_str).unwrap_or(""),
                    "description": tool.get("description").cloned().unwrap_or(Value::Null),
                    "parameters": clean_schema(tool.get("input_schema").cloned().unwrap_or_else(|| json!({}))),
                })
            })
            .collect();
        if !converted_tools.is_empty() {
            result["tools"] = json!(converted_tools);
        }
    }

    if let Some(tool_choice) = body.get("tool_choice") {
        result["tool_choice"] = map_tool_choice(tool_choice);
    }

    if let Some(model) = result.get("model").and_then(Value::as_str) {
        if supports_reasoning_effort(model) {
            if let Some(effort) = resolve_reasoning_effort(&body) {
                result["reasoning"] = json!({ "effort": effort });
            }
        }
    }

    result["store"] = json!(false);
    result["include"] = json!(["reasoning.encrypted_content"]);
    result["stream"] = json!(true);

    let object = result
        .as_object_mut()
        .ok_or_else(|| "Responses request must be a JSON object".to_string())?;
    object
        .entry("instructions".to_string())
        .or_insert(json!(""));
    object.entry("input".to_string()).or_insert(json!([]));
    object.entry("tools".to_string()).or_insert(json!([]));
    object
        .entry("parallel_tool_calls".to_string())
        .or_insert(json!(false));

    // The ChatGPT Codex backend rejects several public Responses API fields.
    object.remove("max_output_tokens");
    object.remove("temperature");
    object.remove("top_p");

    Ok(result)
}

fn system_to_text(system: &Value) -> String {
    if let Some(text) = system.as_str() {
        return text.to_string();
    }

    system
        .as_array()
        .map(|blocks| {
            blocks
                .iter()
                .filter_map(|block| block.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("\n\n")
        })
        .unwrap_or_default()
}

fn messages_to_input(messages: &[Value]) -> Result<Vec<Value>, String> {
    let mut input = Vec::new();

    for message in messages {
        let role = message
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or("user");
        let content = message.get("content").cloned().unwrap_or(Value::Null);

        if let Some(text) = content.as_str() {
            input.push(json!({
                "type": "message",
                "role": map_role(role),
                "content": [{ "type": input_text_type(role), "text": text }],
            }));
            continue;
        }

        let Some(blocks) = content.as_array() else {
            input.push(json!({
                "type": "message",
                "role": map_role(role),
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
                Some("image") if role != "assistant" => {
                    if let Some(source) = block.get("source") {
                        let media_type = source
                            .get("media_type")
                            .and_then(Value::as_str)
                            .unwrap_or("image/png");
                        if let Some(data) = source.get("data").and_then(Value::as_str) {
                            message_content.push(json!({
                                "type": "input_image",
                                "image_url": format!("data:{media_type};base64,{data}"),
                            }));
                        }
                    }
                }
                Some("tool_use") => {
                    input.push(json!({
                        "type": "function_call",
                        "call_id": block.get("id").and_then(Value::as_str).unwrap_or(""),
                        "name": block.get("name").and_then(Value::as_str).unwrap_or(""),
                        "arguments": block.get("input").cloned().unwrap_or_else(|| json!({})).to_string(),
                    }));
                }
                Some("tool_result") => {
                    input.push(json!({
                        "type": "function_call_output",
                        "call_id": block.get("tool_use_id").and_then(Value::as_str).unwrap_or(""),
                        "output": tool_result_output(block),
                    }));
                }
                Some("thinking") | Some("redacted_thinking") => {}
                _ => {}
            }
        }

        if !message_content.is_empty() {
            input.push(json!({
                "type": "message",
                "role": map_role(role),
                "content": message_content,
            }));
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

pub fn supports_reasoning_effort(model: &str) -> bool {
    let lower = model.to_ascii_lowercase();
    lower.starts_with("o1")
        || lower.starts_with("o3")
        || lower.starts_with("o4")
        || lower.starts_with("gpt-5")
}

pub fn resolve_reasoning_effort(body: &Value) -> Option<&'static str> {
    if let Some(effort) = body
        .pointer("/metadata/output_config/effort")
        .and_then(Value::as_str)
        .or_else(|| {
            body.pointer("/output_config/effort")
                .and_then(Value::as_str)
        })
    {
        return match effort {
            "low" => Some("low"),
            "medium" => Some("medium"),
            "high" => Some("high"),
            "max" | "xhigh" => Some("xhigh"),
            _ => None,
        };
    }

    let thinking = body.get("thinking")?;
    match thinking.get("type").and_then(Value::as_str) {
        Some("adaptive") => Some("xhigh"),
        Some("enabled") => match thinking.get("budget_tokens").and_then(Value::as_u64) {
            Some(value) if value <= 4096 => Some("low"),
            Some(value) if value <= 16384 => Some("medium"),
            Some(_) | None => Some("high"),
        },
        _ => None,
    }
}

fn map_role(role: &str) -> &str {
    match role {
        "assistant" => "assistant",
        _ => "user",
    }
}

fn input_text_type(role: &str) -> &str {
    match role {
        "assistant" => "output_text",
        _ => "input_text",
    }
}

fn tool_result_output(block: &Value) -> String {
    let content = block.get("content").cloned().unwrap_or(Value::Null);
    if let Some(text) = content.as_str() {
        return text.to_string();
    }
    if let Some(blocks) = content.as_array() {
        return blocks
            .iter()
            .filter_map(|item| item.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n");
    }
    content.to_string()
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

        let converted = anthropic_to_codex_responses(body, "session").unwrap();
        assert_eq!(converted["model"], "gpt-5.4");
        assert_eq!(converted["instructions"], "Be concise");
        assert_eq!(converted["store"], false);
        assert_eq!(converted["stream"], true);
        assert!(converted.get("prompt_cache_key").is_none());
        assert!(converted.get("max_output_tokens").is_none());
        assert!(converted.get("temperature").is_none());
        assert_eq!(converted["input"][0]["content"][0]["type"], "input_text");
        assert_eq!(converted["tools"][0]["type"], "function");
    }

    #[test]
    fn omits_prompt_cache_key() {
        let body = json!({
            "model": "gpt-5.4",
            "messages": [{"role": "user", "content": "hello"}]
        });
        let long_key = "provider-".repeat(20);

        let converted = anthropic_to_codex_responses(body, &long_key).unwrap();

        assert!(converted.get("prompt_cache_key").is_none());
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

        let converted = anthropic_to_codex_responses(body, "session").unwrap();
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

        let converted = anthropic_to_codex_responses(body, "session").unwrap();
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

        let converted = anthropic_to_codex_responses(body, "session").unwrap();
        assert_eq!(converted["input"][0]["content"][1]["type"], "input_image");
        assert_eq!(
            converted["input"][0]["content"][1]["image_url"],
            "data:image/png;base64,abc"
        );
    }
}

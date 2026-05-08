//! Anthropic Messages to MiniMax OpenAI Chat Completions request conversion.

use cc_switch_lib::providers::{ProviderError, TransformInput, TransformOutput};
use serde_json::{json, Value};

pub fn transform(input: TransformInput) -> Result<TransformOutput, ProviderError> {
    let body = anthropic_to_openai_chat(input.body, input.requested_stream)
        .map_err(ProviderError::TransformFailed)?;

    Ok(TransformOutput {
        body,
        upstream_url: minimax_chat_completions_url(&input.upstream_url),
        headers: vec![],
        method: "POST".to_string(),
    })
}

fn anthropic_to_openai_chat(body: Value, requested_stream: bool) -> Result<Value, String> {
    let mut result = json!({});

    if let Some(model) = body.get("model").and_then(Value::as_str) {
        result["model"] = json!(model);
    }

    let mut messages = Vec::new();
    if let Some(system) = body.get("system") {
        let text = text_from_content(system);
        if !text.is_empty() {
            messages.push(json!({ "role": "system", "content": text }));
        }
    }

    if let Some(input_messages) = body.get("messages").and_then(Value::as_array) {
        for message in input_messages {
            let role = message
                .get("role")
                .and_then(Value::as_str)
                .unwrap_or("user");
            messages.extend(convert_message(
                role,
                message.get("content").unwrap_or(&Value::Null),
            ));
        }
    }
    result["messages"] = json!(messages);

    copy_number(&body, &mut result, "max_tokens");
    copy_number(&body, &mut result, "temperature");
    copy_number(&body, &mut result, "top_p");
    if let Some(stop) = body.get("stop_sequences") {
        result["stop"] = stop.clone();
    }
    if let Some(tools) = body.get("tools").and_then(Value::as_array) {
        let converted: Vec<Value> = tools
            .iter()
            .map(|tool| {
                json!({
                    "type": "function",
                    "function": {
                        "name": tool.get("name").and_then(Value::as_str).unwrap_or(""),
                        "description": tool.get("description").cloned().unwrap_or(Value::Null),
                        "parameters": tool.get("input_schema").cloned().unwrap_or_else(|| json!({})),
                    }
                })
            })
            .collect();
        if !converted.is_empty() {
            result["tools"] = json!(converted);
        }
    }

    result["stream"] = json!(requested_stream);
    if requested_stream {
        result["stream_options"] = json!({ "include_usage": true });
    }
    Ok(result)
}

fn minimax_chat_completions_url(base_url: &str) -> String {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.ends_with("/chat/completions") {
        return trimmed.to_string();
    }
    if let Some(prefix) = trimmed.strip_suffix("/anthropic") {
        return format!("{prefix}/v1/chat/completions");
    }
    if trimmed.ends_with("/v1") {
        return format!("{trimmed}/chat/completions");
    }
    format!("{trimmed}/v1/chat/completions")
}

fn text_from_content(content: &Value) -> String {
    if let Some(text) = content.as_str() {
        return text.to_string();
    }

    content
        .as_array()
        .map(|blocks| {
            blocks
                .iter()
                .filter_map(|block| match block.get("type").and_then(Value::as_str) {
                    Some("text") => block
                        .get("text")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    Some("tool_result") => block
                        .get("content")
                        .map(text_from_content)
                        .filter(|text| !text.is_empty()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n\n")
        })
        .unwrap_or_default()
}

fn convert_message(role: &str, content: &Value) -> Vec<Value> {
    if let Some(text) = content.as_str() {
        if text.is_empty() {
            return vec![];
        }
        return vec![json!({ "role": map_role(role), "content": text })];
    }

    let Some(blocks) = content.as_array() else {
        return vec![];
    };

    let mut output = Vec::new();
    let mut content_parts: Vec<String> = Vec::new();
    let mut tool_calls: Vec<Value> = Vec::new();

    for block in blocks {
        match block.get("type").and_then(Value::as_str) {
            Some("text") => {
                if let Some(text) = block.get("text").and_then(Value::as_str) {
                    if !text.is_empty() {
                        content_parts.push(text.to_string());
                    }
                }
            }
            Some("tool_use") => {
                let id = block.get("id").and_then(Value::as_str).unwrap_or("");
                let name = block.get("name").and_then(Value::as_str).unwrap_or("");
                let input = block.get("input").cloned().unwrap_or_else(|| json!({}));
                tool_calls.push(json!({
                    "id": id,
                    "type": "function",
                    "function": {
                        "name": name,
                        "arguments": serde_json::to_string(&input).unwrap_or_else(|_| "{}".to_string()),
                    }
                }));
            }
            Some("tool_result") => {
                let tool_use_id = block
                    .get("tool_use_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                if tool_use_id.trim().is_empty() {
                    continue;
                }
                let content_text = text_from_content(block.get("content").unwrap_or(&Value::Null));
                output.push(json!({
                    "role": "tool",
                    "tool_call_id": tool_use_id,
                    "content": content_text,
                }));
            }
            _ => {}
        }
    }

    let merged_content = content_parts.join("\n\n");
    if !merged_content.is_empty() || !tool_calls.is_empty() {
        let mut msg = json!({
            "role": map_role(role),
            "content": if merged_content.is_empty() { Value::Null } else { json!(merged_content) },
        });
        if !tool_calls.is_empty() {
            msg["tool_calls"] = json!(tool_calls);
        }
        output.push(msg);
    }
    output
}

fn map_role(role: &str) -> &str {
    match role {
        "assistant" => "assistant",
        "system" => "system",
        _ => "user",
    }
}

fn copy_number(source: &Value, target: &mut Value, key: &str) {
    if let Some(value) = source.get(key) {
        if value.is_number() {
            target[key] = value.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_legacy_anthropic_url_to_chat_completions() {
        assert_eq!(
            minimax_chat_completions_url("https://api.minimaxi.com/anthropic"),
            "https://api.minimaxi.com/v1/chat/completions"
        );
        assert_eq!(
            minimax_chat_completions_url("https://api.minimaxi.com/v1"),
            "https://api.minimaxi.com/v1/chat/completions"
        );
    }

    #[test]
    fn preserves_full_endpoint_and_avoids_double_v1() {
        assert_eq!(
            minimax_chat_completions_url("https://api.minimaxi.com/v1/chat/completions"),
            "https://api.minimaxi.com/v1/chat/completions"
        );
        assert_eq!(
            minimax_chat_completions_url("https://api.minimaxi.com/chat/completions"),
            "https://api.minimaxi.com/chat/completions"
        );
    }

    #[test]
    fn converts_anthropic_messages_to_openai_chat() {
        let input = json!({
            "model": "MiniMax-M2.7",
            "system": "You are terse.",
            "messages": [{ "role": "user", "content": [{ "type": "text", "text": "hi" }] }],
            "max_tokens": 128
        });

        let output = anthropic_to_openai_chat(input, true).unwrap();

        assert_eq!(output["model"], "MiniMax-M2.7");
        assert_eq!(output["messages"][0]["role"], "system");
        assert_eq!(output["messages"][1]["content"], "hi");
        assert_eq!(output["max_tokens"], 128);
        assert_eq!(output["stream"], true);
        assert_eq!(output["stream_options"]["include_usage"], true);
    }

    #[test]
    fn converts_tool_use_and_tool_result_roundtrip_shape() {
        let input = json!({
            "model": "MiniMax-M2.7",
            "messages": [
                {
                    "role": "assistant",
                    "content": [
                        { "type": "text", "text": "Let me call a tool" },
                        { "type": "tool_use", "id": "call_1", "name": "lookup", "input": { "q": "BTC" } }
                    ]
                },
                {
                    "role": "user",
                    "content": [
                        { "type": "tool_result", "tool_use_id": "call_1", "content": "price ok" }
                    ]
                }
            ]
        });

        let output = anthropic_to_openai_chat(input, true).unwrap();
        let messages = output["messages"].as_array().unwrap();

        assert_eq!(messages[0]["role"], "assistant");
        assert_eq!(messages[0]["tool_calls"][0]["id"], "call_1");
        assert_eq!(messages[0]["tool_calls"][0]["function"]["name"], "lookup");
        assert_eq!(messages[1]["role"], "tool");
        assert_eq!(messages[1]["tool_call_id"], "call_1");
    }

    #[test]
    fn tool_result_with_text_keeps_tool_message_ordering() {
        let input = json!({
            "model": "MiniMax-M2.7",
            "messages": [
                {
                    "role": "assistant",
                    "content": [{ "type": "tool_use", "id": "call_1", "name": "lookup", "input": { "q": "BTC" } }]
                },
                {
                    "role": "user",
                    "content": [
                        { "type": "text", "text": "tool result:" },
                        { "type": "tool_result", "tool_use_id": "call_1", "content": "price ok" }
                    ]
                }
            ]
        });

        let output = anthropic_to_openai_chat(input, true).unwrap();
        let messages = output["messages"].as_array().unwrap();
        assert_eq!(messages[1]["role"], "tool");
        assert_eq!(messages[1]["tool_call_id"], "call_1");
    }
}

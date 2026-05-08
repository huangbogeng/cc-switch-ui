//! MiniMax response parsing

use bytes::Bytes;
use cc_switch_lib::providers::{ProviderError, UsageParseResult};
use cc_switch_lib::usage::UsageParser;
use serde_json::{json, Value};

/// Parse OpenAI-format response and extract usage
pub fn transform(body: Bytes, _is_streaming: bool) -> Result<UsageParseResult, ProviderError> {
    let parser = UsageParser::new();
    let record = parser.from_openai_json(&body);
    let transformed = openai_chat_to_anthropic_message(&body)
        .map(|value| serde_json::to_vec(&value).unwrap_or_default())
        .map(Bytes::from)
        .unwrap_or_else(|_| body.clone());

    Ok(UsageParseResult {
        record,
        body: transformed,
    })
}

fn openai_chat_to_anthropic_message(body: &[u8]) -> Result<Value, String> {
    let response: Value = serde_json::from_slice(body).map_err(|e| e.to_string())?;
    let choice = response
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .ok_or_else(|| "OpenAI chat response is missing choices".to_string())?;
    let message = choice.get("message").unwrap_or(choice);
    let content_text = message
        .get("content")
        .and_then(Value::as_str)
        .unwrap_or_default();

    let mut content = Vec::new();
    if !content_text.is_empty() {
        content.push(json!({ "type": "text", "text": content_text }));
    }
    if let Some(tool_calls) = message.get("tool_calls").and_then(Value::as_array) {
        for tool_call in tool_calls {
            let function = tool_call.get("function").unwrap_or(&Value::Null);
            let arguments = function
                .get("arguments")
                .and_then(Value::as_str)
                .unwrap_or("{}");
            let input = serde_json::from_str::<Value>(arguments).unwrap_or_else(|_| json!({}));
            content.push(json!({
                "type": "tool_use",
                "id": tool_call.get("id").and_then(Value::as_str).unwrap_or(""),
                "name": function.get("name").and_then(Value::as_str).unwrap_or(""),
                "input": input,
            }));
        }
    }

    Ok(json!({
        "id": response.get("id").and_then(Value::as_str).unwrap_or(""),
        "type": "message",
        "role": "assistant",
        "model": response.get("model").and_then(Value::as_str).unwrap_or(""),
        "content": content,
        "stop_reason": stop_reason(choice.get("finish_reason").and_then(Value::as_str)),
        "stop_sequence": Value::Null,
        "usage": anthropic_usage(response.get("usage")),
    }))
}

fn anthropic_usage(usage: Option<&Value>) -> Value {
    let Some(usage) = usage else {
        return json!({ "input_tokens": 0, "output_tokens": 0 });
    };
    json!({
        "input_tokens": usage.get("prompt_tokens").and_then(Value::as_u64).unwrap_or(0),
        "output_tokens": usage.get("completion_tokens").and_then(Value::as_u64).unwrap_or(0),
    })
}

fn stop_reason(reason: Option<&str>) -> &'static str {
    match reason {
        Some("length") => "max_tokens",
        Some("tool_calls") | Some("function_call") => "tool_use",
        _ => "end_turn",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_openai_chat_response_to_anthropic_message() {
        let input = br#"{
            "id":"chatcmpl_1",
            "model":"MiniMax-M2.7",
            "choices":[{"finish_reason":"stop","message":{"role":"assistant","content":"hello"}}],
            "usage":{"prompt_tokens":3,"completion_tokens":4}
        }"#;

        let output = openai_chat_to_anthropic_message(input).unwrap();

        assert_eq!(output["type"], "message");
        assert_eq!(output["content"][0]["text"], "hello");
        assert_eq!(output["usage"]["input_tokens"], 3);
        assert_eq!(output["usage"]["output_tokens"], 4);
    }
}

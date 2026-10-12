//! Anthropic Messages to DeepSeek OpenAI Chat Completions request conversion.

use cc_switch_lib::providers::{ProviderError, TransformInput, TransformOutput};
#[cfg(test)]
use serde_json::json;
use serde_json::Value;

pub fn transform(input: TransformInput) -> Result<TransformOutput, ProviderError> {
    let body = anthropic_to_openai_chat(
        rectify_deepseek_thinking(input.body),
        input.requested_stream,
    )
    .map_err(ProviderError::TransformFailed)?;

    Ok(TransformOutput {
        body,
        upstream_url: deepseek_chat_completions_url(&input.upstream_url),
        headers: vec![],
        method: "POST".to_string(),
    })
}

fn rectify_deepseek_thinking(mut body: Value) -> Value {
    if let Some(obj) = body.as_object_mut() {
        obj.remove("thinking");
    }
    if let Some(messages) = body.get_mut("messages").and_then(Value::as_array_mut) {
        for message in messages.iter_mut() {
            if let Some(content) = message.get_mut("content").and_then(Value::as_array_mut) {
                let mut filtered = Vec::with_capacity(content.len());
                for block in content.iter() {
                    let mut cloned = block.clone();
                    if let Some(map) = cloned.as_object_mut() {
                        map.remove("signature");
                    }
                    filtered.push(cloned);
                }
                *content = filtered;
            }
        }
    }
    body
}

fn anthropic_to_openai_chat(body: Value, requested_stream: bool) -> Result<Value, String> {
    crate::proxy::transform_chat::anthropic_to_openai_chat(body, requested_stream, true)
}

fn deepseek_chat_completions_url(base_url: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_legacy_anthropic_url_to_chat_completions() {
        assert_eq!(
            deepseek_chat_completions_url("https://api.deepseek.com/anthropic"),
            "https://api.deepseek.com/v1/chat/completions"
        );
        assert_eq!(
            deepseek_chat_completions_url("https://api.deepseek.com/v1"),
            "https://api.deepseek.com/v1/chat/completions"
        );
    }

    #[test]
    fn strips_thinking_blocks_and_signatures_before_conversion() {
        let input = json!({
            "thinking": {"type": "enabled", "budget_tokens": 2048},
            "messages": [{
                "role": "assistant",
                "content": [
                    {"type": "thinking", "thinking": "secret", "signature": "sig"},
                    {"type": "text", "text": "visible", "signature": "remove-me"},
                    {"type": "tool_use", "id": "call_1", "name": "lookup", "input": {"q": "hi"}}
                ]
            }]
        });

        let output = anthropic_to_openai_chat(rectify_deepseek_thinking(input), true).unwrap();
        let msg = &output["messages"][0];
        assert_eq!(msg["role"], "assistant");
        assert_eq!(msg["content"], "visible");
        assert_eq!(msg["reasoning_content"], "secret");
        assert!(msg.get("signature").is_none());
    }

    #[test]
    fn keeps_assistant_reasoning_content_for_deepseek_thinking_mode() {
        let input = json!({
            "messages": [{
                "role": "assistant",
                "content": [
                    {"type": "thinking", "thinking": "step 1"},
                    {"type": "thinking", "thinking": "step 2"},
                    {"type": "text", "text": "final answer"}
                ]
            }]
        });

        let output = anthropic_to_openai_chat(rectify_deepseek_thinking(input), false).unwrap();
        let msg = &output["messages"][0];
        assert_eq!(msg["role"], "assistant");
        assert_eq!(msg["content"], "final answer");
        assert_eq!(msg["reasoning_content"], "step 1\n\nstep 2");
    }
}

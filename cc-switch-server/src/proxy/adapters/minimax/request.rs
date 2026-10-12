//! Anthropic Messages to MiniMax OpenAI Chat Completions request conversion.

use cc_switch_lib::providers::{ProviderError, TransformInput, TransformOutput};
#[cfg(test)]
use serde_json::json;
use serde_json::Value;

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
    crate::proxy::transform_chat::anthropic_to_openai_chat(body, requested_stream, false)
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

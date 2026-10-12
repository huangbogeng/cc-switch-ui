//! Responses terminal/usage semantics from cc-switch at 28dc5198.
use serde_json::{json, Value};

pub fn terminal_error(response: &Value) -> Option<String> {
    let status = response.get("status").and_then(Value::as_str);
    let error = response.get("error").filter(|value| !value.is_null());
    let prefix = match status {
        Some("failed") => "Responses upstream failed",
        Some("cancelled") => "Responses upstream cancelled",
        _ if error.is_some() => "Responses upstream returned an error",
        _ => return None,
    };
    let message = response
        .pointer("/error/message")
        .or_else(|| response.get("message"))
        .or(error)
        .and_then(Value::as_str)
        .unwrap_or("response generation failed");
    Some(format!("{prefix}: {message}"))
}

pub fn stop_reason(response: &Value, has_tool_use: bool) -> &'static str {
    match response.get("status").and_then(Value::as_str) {
        Some("incomplete") => match response
            .pointer("/incomplete_details/reason")
            .and_then(Value::as_str)
        {
            None | Some("max_output_tokens" | "max_tokens") => "max_tokens",
            _ => "end_turn",
        },
        _ if has_tool_use => "tool_use",
        _ => "end_turn",
    }
}

pub fn usage(usage: Option<&Value>) -> Value {
    let u = match usage {
        Some(v) if !v.is_null() && v.is_object() => v,
        _ => {
            return json!({
                "input_tokens": 0,
                "output_tokens": 0
            })
        }
    };

    // Detect empty object {} and log warning
    if u.as_object().map(|obj| obj.is_empty()).unwrap_or(false) {
        log::warn!("[Responses] Empty usage object received, using defaults");
        return json!({
            "input_tokens": 0,
            "output_tokens": 0
        });
    }

    // Extract input_tokens with OpenAI field name fallback
    // Priority: input_tokens (Anthropic) → prompt_tokens (OpenAI) → 0
    let input = u
        .get("input_tokens")
        .and_then(|v| v.as_u64())
        .or_else(|| {
            let prompt_tokens = u.get("prompt_tokens").and_then(|v| v.as_u64());
            if prompt_tokens.is_some() {
                log::debug!(
                    "[Responses] Using OpenAI field name fallback 'prompt_tokens' for input_tokens"
                );
            }
            prompt_tokens
        })
        .unwrap_or(0);

    // Extract output_tokens with OpenAI field name fallback
    // Priority: output_tokens (Anthropic) → completion_tokens (OpenAI) → 0
    let output = u.get("output_tokens")
        .and_then(|v| v.as_u64())
        .or_else(|| {
            let completion_tokens = u.get("completion_tokens").and_then(|v| v.as_u64());
            if completion_tokens.is_some() {
                log::debug!("[Responses] Using OpenAI field name fallback 'completion_tokens' for output_tokens");
            }
            completion_tokens
        })
        .unwrap_or(0);

    // Log if only one field present (partial object). Streaming chunks legitimately
    // arrive with partial usage, so this stays at debug level to avoid noise.
    if (input == 0 && output > 0) || (input > 0 && output == 0) {
        log::debug!("[Responses] Partial usage object: {:?}", u);
    }

    let mut result = json!({
        "input_tokens": input,
        "output_tokens": output
    });

    // Step 1: OpenAI nested details as fallback for cache tokens
    // OpenAI Responses API: input_tokens_details.cached_tokens
    if let Some(cached) = u
        .pointer("/input_tokens_details/cached_tokens")
        .and_then(|v| v.as_u64())
    {
        result["cache_read_input_tokens"] = json!(cached);
    }
    // OpenAI standard: prompt_tokens_details.cached_tokens
    if let Some(cached) = u
        .pointer("/prompt_tokens_details/cached_tokens")
        .and_then(|v| v.as_u64())
    {
        if result.get("cache_read_input_tokens").is_none() {
            result["cache_read_input_tokens"] = json!(cached);
        }
    }
    // GPT-5.6+ reports cache writes in the nested OpenAI token-details object.
    // Treat writes as Anthropic cache creation so the downstream client and
    // billing layer can distinguish them from fresh input.
    let nested_cache_write = u
        .pointer("/input_tokens_details/cache_write_tokens")
        .and_then(|v| v.as_u64())
        .or_else(|| {
            u.pointer("/prompt_tokens_details/cache_write_tokens")
                .and_then(|v| v.as_u64())
        });
    if let Some(cache_write) = nested_cache_write {
        result["cache_creation_input_tokens"] = json!(cache_write);
    }

    // Step 2: Direct Anthropic-style fields override (authoritative if present)
    // These preserve cache tokens even if input/output_tokens are missing
    if let Some(v) = u.get("cache_read_input_tokens") {
        result["cache_read_input_tokens"] = v.clone();
    }
    if let Some(v) = u.get("cache_creation_input_tokens") {
        result["cache_creation_input_tokens"] = v.clone();
    }
    if let Some(v) = u.get("cache_creation") {
        result["cache_creation"] = v.clone();
    }

    // OpenAI/Responses 的 input(prompt_tokens/input_tokens)含缓存命中，Anthropic input_tokens 不含
    // → 减去 cache_read 与 cache_creation，使其成为 fresh input。本函数在计量意义上是 claude 专属
    // （Codex Responses 透传走 from_codex_response_*，不调用本函数），故可安全在此扣减。三桶互斥，
    // 恒等：input + cache_read + cache_creation == 上游 input(inclusive)。与 build_anthropic_usage_json
    // (#2774) 及 transform_gemini 的 saturating_sub 对称；一处同时覆盖非流式与流式(streaming_responses)。
    let cached = result
        .get("cache_read_input_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let cache_creation = result
        .get("cache_creation_input_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    if cached > 0 || cache_creation > 0 {
        result["input_tokens"] = json!(input.saturating_sub(cached).saturating_sub(cache_creation));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn distinguishes_truncation_from_other_incomplete_reasons() {
        assert_eq!(
            stop_reason(
                &json!({"status":"incomplete","incomplete_details":{"reason":"max_output_tokens"}}),
                false
            ),
            "max_tokens"
        );
        assert_eq!(
            stop_reason(
                &json!({"status":"incomplete","incomplete_details":{"reason":"content_filter"}}),
                false
            ),
            "end_turn"
        );
        assert!(terminal_error(
            &json!({"status":"failed","output":[],"error":{"message":"backend failed"}})
        )
        .unwrap()
        .contains("backend failed"));
    }
    #[test]
    fn cache_buckets_match_anthropic_exclusive_input_semantics() {
        assert_eq!(
            usage(Some(
                &json!({"input_tokens":100,"output_tokens":7,"input_tokens_details":{"cached_tokens":30,"cache_write_tokens":10}})
            )),
            json!({"input_tokens":60,"output_tokens":7,"cache_read_input_tokens":30,"cache_creation_input_tokens":10})
        );
    }
}

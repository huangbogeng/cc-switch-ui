//! Codex response parsing

use bytes::Bytes;
use cc_switch_lib::providers::{ProviderError, UsageParseResult};
use cc_switch_lib::usage::UsageParser;

/// Transform Codex Responses SSE to Anthropic message and extract usage
pub fn transform(body: Bytes, is_streaming: bool) -> Result<UsageParseResult, ProviderError> {
    if is_streaming {
        // Streaming: usage spread across SSE events, cannot extract here
        return Ok(UsageParseResult { record: None, body });
    }

    // Parse usage from non-streaming response
    let parser = UsageParser::new();
    let record = parser.from_openai_json(&body);

    // Transform SSE aggregated response to Anthropic message format
    let transformed = crate::proxy::responses_aggregate::responses_sse_to_anthropic_message(&body)
        .map(|v| serde_json::to_vec(&v).unwrap_or_default())
        .map(Bytes::from)
        .unwrap_or(body);

    Ok(UsageParseResult {
        record,
        body: transformed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streaming_mode_keeps_body_without_usage_record() {
        let body =
            Bytes::from_static(b"event: response.output_text.delta\ndata: {\"delta\":\"hi\"}\n\n");
        let result = transform(body.clone(), true).expect("transform should succeed");
        assert!(result.record.is_none());
        assert_eq!(result.body, body);
    }

    #[test]
    fn non_streaming_extracts_usage() {
        let body = Bytes::from_static(
            br#"{"model":"gpt-5","usage":{"prompt_tokens":7,"completion_tokens":11}}"#,
        );
        let result = transform(body, false).expect("transform should succeed");
        let record = result.record.expect("usage record should exist");
        assert_eq!(record.model, "gpt-5");
        assert_eq!(record.input_tokens, 7);
        assert_eq!(record.output_tokens, 11);
    }
}

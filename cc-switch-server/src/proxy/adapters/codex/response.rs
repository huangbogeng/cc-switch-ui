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

    // Transform SSE aggregated response to Anthropic message format
    let converted = if let Ok(response) = serde_json::from_slice(&body) {
        crate::proxy::responses_aggregate::responses_json_to_anthropic_message(&response)
    } else {
        crate::proxy::responses_aggregate::responses_sse_to_anthropic_message(&body)
    };
    let converted = converted.map_err(ProviderError::ResponseTransformFailed)?;
    let transformed = Bytes::from(
        serde_json::to_vec(&converted)
            .map_err(|error| ProviderError::ResponseTransformFailed(error.to_string()))?,
    );
    let mut record = parser.from_anthropic_json(&transformed);
    if let Some(record) = &mut record {
        record.cache_read_tokens = converted
            .pointer("/usage/cache_read_input_tokens")
            .and_then(serde_json::Value::as_i64);
    }

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
            br#"{"id":"r","model":"gpt-5","status":"completed","output":[],"usage":{"prompt_tokens":7,"completion_tokens":11}}"#,
        );
        let result = transform(body, false).expect("transform should succeed");
        let record = result.record.expect("usage record should exist");
        assert_eq!(record.model, "gpt-5");
        assert_eq!(record.input_tokens, 7);
        assert_eq!(record.output_tokens, 11);
    }
}

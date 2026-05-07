//! Codex response parsing

use cc_switch_lib::providers::{ProviderError, UsageParseResult};
use cc_switch_lib::usage::UsageParser;
use bytes::Bytes;

/// Transform Codex Responses SSE to Anthropic message and extract usage
pub fn transform(
    body: Bytes,
    is_streaming: bool,
) -> Result<UsageParseResult, ProviderError> {
    if is_streaming {
        // Streaming: usage spread across SSE events, cannot extract here
        return Ok(UsageParseResult {
            record: None,
            body,
        });
    }

    // Parse usage from non-streaming response
    let parser = UsageParser::new();
    let record = parser.from_openai_json(&body);

    // Transform SSE aggregated response to Anthropic message format
    let transformed =
        crate::proxy::responses_aggregate::responses_sse_to_anthropic_message(&body)
            .map(|v| serde_json::to_vec(&v).unwrap_or_default())
            .map(Bytes::from)
            .unwrap_or(body);

    Ok(UsageParseResult {
        record,
        body: transformed,
    })
}

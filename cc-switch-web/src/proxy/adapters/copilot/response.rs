//! Copilot response parsing

use cc_switch_lib::providers::{ProviderError, UsageParseResult};
use cc_switch_lib::usage::UsageParser;
use bytes::Bytes;

/// Parse Copilot-format response and extract usage
/// Note: Copilot uses a specific response format, using OpenAI format as fallback
pub fn transform(
    body: Bytes,
    _is_streaming: bool,
) -> Result<UsageParseResult, ProviderError> {
    let parser = UsageParser::new();
    // Try OpenAI format as Copilot uses OpenAI-compatible format
    let record = parser.from_openai_json(&body);

    Ok(UsageParseResult {
        record,
        body,
    })
}

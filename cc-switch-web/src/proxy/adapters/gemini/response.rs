//! Gemini response parsing

use bytes::Bytes;
use cc_switch_lib::providers::{ProviderError, UsageParseResult};
use cc_switch_lib::usage::UsageParser;

/// Parse Gemini-format response and extract usage
/// Note: Gemini usage parsing is not yet fully implemented
pub fn transform(body: Bytes, _is_streaming: bool) -> Result<UsageParseResult, ProviderError> {
    let parser = UsageParser::new();
    // Try OpenAI format as fallback since some Gemini proxies use it
    let record = parser.from_openai_json(&body);

    Ok(UsageParseResult { record, body })
}

//! OpenRouter response parsing

use cc_switch_lib::providers::{ProviderError, UsageParseResult};
use cc_switch_lib::usage::UsageParser;
use bytes::Bytes;

/// Parse OpenAI-format response and extract usage
pub fn transform(
    body: Bytes,
    _is_streaming: bool,
) -> Result<UsageParseResult, ProviderError> {
    let parser = UsageParser::new();
    let record = parser.from_openai_json(&body);

    Ok(UsageParseResult {
        record,
        body,
    })
}

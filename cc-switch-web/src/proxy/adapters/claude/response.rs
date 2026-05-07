//! Claude response parsing

use bytes::Bytes;
use cc_switch_lib::providers::{ProviderError, UsageParseResult};
use cc_switch_lib::usage::UsageParser;

/// Parse Anthropic JSON response and extract usage
pub fn transform(body: Bytes, _is_streaming: bool) -> Result<UsageParseResult, ProviderError> {
    let parser = UsageParser::new();
    let record = parser.from_anthropic_json(&body);

    Ok(UsageParseResult { record, body })
}

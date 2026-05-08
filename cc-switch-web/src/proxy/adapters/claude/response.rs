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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_anthropic_usage_record() {
        let body = Bytes::from_static(
            br#"{"model":"claude-3-5-sonnet","usage":{"input_tokens":12,"output_tokens":34}}"#,
        );
        let result = transform(body.clone(), false).expect("transform should succeed");
        let record = result.record.expect("usage record should exist");
        assert_eq!(record.model, "claude-3-5-sonnet");
        assert_eq!(record.input_tokens, 12);
        assert_eq!(record.output_tokens, 34);
        assert_eq!(result.body, body);
    }
}

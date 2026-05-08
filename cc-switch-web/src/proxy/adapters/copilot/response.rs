//! Copilot response parsing

use bytes::Bytes;
use cc_switch_lib::providers::{ProviderError, UsageParseResult};
use cc_switch_lib::usage::UsageParser;

/// Parse Copilot-format response and extract usage
/// Note: Copilot uses a specific response format, using OpenAI format as fallback
pub fn transform(body: Bytes, _is_streaming: bool) -> Result<UsageParseResult, ProviderError> {
    let parser = UsageParser::new();
    // Try OpenAI format as Copilot uses OpenAI-compatible format
    let record = parser.from_openai_json(&body);

    Ok(UsageParseResult { record, body })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_openai_compatible_usage_for_copilot() {
        let body = Bytes::from_static(
            br#"{"model":"gpt-4o-mini","usage":{"prompt_tokens":5,"completion_tokens":9}}"#,
        );
        let result = transform(body.clone(), false).expect("transform should succeed");
        let record = result.record.expect("usage record should exist");
        assert_eq!(record.model, "gpt-4o-mini");
        assert_eq!(record.input_tokens, 5);
        assert_eq!(record.output_tokens, 9);
        assert_eq!(result.body, body);
    }
}

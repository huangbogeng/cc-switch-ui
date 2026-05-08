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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_openai_compatible_usage_for_gemini_proxy() {
        let body = Bytes::from_static(
            br#"{"model":"gemini-2.5-pro","usage":{"prompt_tokens":8,"completion_tokens":13}}"#,
        );
        let result = transform(body.clone(), false).expect("transform should succeed");
        let record = result.record.expect("usage record should exist");
        assert_eq!(record.model, "gemini-2.5-pro");
        assert_eq!(record.input_tokens, 8);
        assert_eq!(record.output_tokens, 13);
        assert_eq!(result.body, body);
    }
}

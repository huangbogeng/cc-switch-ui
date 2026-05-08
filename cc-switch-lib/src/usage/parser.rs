//! Usage parser for various API response formats

use crate::database::UsageRecord;
use serde::Deserialize;

/// Parser for extracting usage from API responses
pub struct UsageParser;

impl UsageParser {
    /// Create a new UsageParser
    pub fn new() -> Self {
        Self
    }

    /// Extract usage from Anthropic non-streaming JSON response
    pub fn from_anthropic_json(&self, body: &[u8]) -> Option<UsageRecord> {
        let response: AnthropicResponse = serde_json::from_slice(body).ok()?;
        Some(UsageRecord {
            provider_id: String::new(), // Will be set by caller
            model: response.model.unwrap_or_default(),
            input_tokens: response.usage.as_ref()?.input_tokens.unwrap_or(0),
            output_tokens: response.usage.as_ref()?.output_tokens.unwrap_or(0),
            cache_read_tokens: response.usage.as_ref()?.cache_read_tokens,
            request_timestamp: chrono::Utc::now().timestamp(),
        })
    }

    /// Extract usage from Anthropic SSE streaming response
    pub fn from_anthropic_sse(&self, data: &str) -> Option<UsageRecord> {
        // Expected format: "event: message_delta\ndata: {\"usage\":{...}}\n\n"
        let json_start = data.find("data: ")? + 6;
        let json_end = data[json_start..].find('\n').map(|i| json_start + i);
        let json_str = if let Some(end) = json_end {
            &data[json_start..end]
        } else {
            &data[json_start..]
        };

        let delta: AnthropicMessageDelta = serde_json::from_str(json_str).ok()?;
        Some(UsageRecord {
            provider_id: String::new(),
            model: String::new(),
            input_tokens: 0, // SSE doesn't typically include input tokens
            output_tokens: delta.usage.as_ref()?.output_tokens.unwrap_or(0),
            cache_read_tokens: delta.usage.as_ref()?.cache_read_tokens,
            request_timestamp: chrono::Utc::now().timestamp(),
        })
    }

    /// Extract usage from OpenAI/Codex JSON response
    pub fn from_openai_json(&self, body: &[u8]) -> Option<UsageRecord> {
        let response: OpenAIResponse = serde_json::from_slice(body).ok()?;
        let usage = response.usage?;
        let input_tokens = usage.prompt_tokens.or(usage.input_tokens).unwrap_or(0);
        let output_tokens = usage
            .completion_tokens
            .or(usage.output_tokens)
            .unwrap_or_else(|| usage.total_tokens.unwrap_or(0).saturating_sub(input_tokens));
        Some(UsageRecord {
            provider_id: String::new(),
            model: response.model.unwrap_or_default(),
            input_tokens,
            output_tokens,
            cache_read_tokens: None,
            request_timestamp: chrono::Utc::now().timestamp(),
        })
    }
}

impl Default for UsageParser {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
struct AnthropicResponse {
    model: Option<String>,
    usage: Option<AnthropicUsage>,
}

#[derive(Deserialize)]
struct AnthropicUsage {
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    cache_read_tokens: Option<i64>,
}

#[derive(Deserialize)]
struct AnthropicMessageDelta {
    usage: Option<AnthropicUsage>,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    model: Option<String>,
    usage: Option<OpenAIUsage>,
}

#[derive(Deserialize)]
struct OpenAIUsage {
    prompt_tokens: Option<i64>,
    completion_tokens: Option<i64>,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    total_tokens: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::UsageParser;

    #[test]
    fn parses_openai_prompt_completion_usage() {
        let body = br#"{
          "model":"gpt-x",
          "usage":{"prompt_tokens":12,"completion_tokens":34}
        }"#;
        let record = UsageParser::new().from_openai_json(body).expect("usage");
        assert_eq!(record.input_tokens, 12);
        assert_eq!(record.output_tokens, 34);
    }

    #[test]
    fn parses_deepseek_input_output_usage() {
        let body = br#"{
          "model":"deepseek-v4-pro",
          "usage":{"input_tokens":21,"output_tokens":55}
        }"#;
        let record = UsageParser::new().from_openai_json(body).expect("usage");
        assert_eq!(record.input_tokens, 21);
        assert_eq!(record.output_tokens, 55);
    }
}

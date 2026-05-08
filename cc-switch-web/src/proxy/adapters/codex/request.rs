//! Codex request transformation

use cc_switch_lib::providers::{ProviderError, TransformInput, TransformOutput};

/// Transform Anthropic request to Codex Responses format
pub fn transform(input: TransformInput) -> Result<TransformOutput, ProviderError> {
    let transformed = crate::proxy::transform_responses::anthropic_to_codex_responses(
        input.body,
        input.prompt_cache_key.as_deref(),
        input.codex_fast_mode,
    )
    .map_err(ProviderError::TransformFailed)?;

    let mut headers = vec![
        ("originator".to_string(), "cc-switch".to_string()),
        ("accept".to_string(), "text/event-stream".to_string()),
        ("accept-encoding".to_string(), "identity".to_string()),
    ];

    if let Some(ref key) = input.prompt_cache_key {
        headers.push(("x-prompt-cache-key".to_string(), key.clone()));
    }

    Ok(TransformOutput {
        body: transformed,
        upstream_url: input.upstream_url,
        headers,
        method: "POST".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn transforms_anthropic_request_to_responses_shape() {
        let output = transform(TransformInput {
            body: json!({
                "model": "gpt-5",
                "messages": [{"role":"user","content":"hello"}],
                "stream": true
            }),
            upstream_url: "https://chatgpt.com/backend-api/codex/responses".to_string(),
            prompt_cache_key: Some("k1".to_string()),
            requested_stream: true,
            codex_fast_mode: false,
        })
        .expect("transform should succeed");

        assert_eq!(output.method, "POST");
        assert_eq!(
            output.upstream_url,
            "https://chatgpt.com/backend-api/codex/responses"
        );
        assert!(output
            .headers
            .iter()
            .any(|(k, v)| k == "x-prompt-cache-key" && v == "k1"));
    }
}

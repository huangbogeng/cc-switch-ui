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

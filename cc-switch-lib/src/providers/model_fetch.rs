use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const FETCH_TIMEOUT_SECS: u64 = 15;
const ERROR_BODY_MAX_CHARS: usize = 512;

const KNOWN_COMPAT_SUFFIXES: &[&str] = &[
    "/api/claudecode",
    "/api/anthropic",
    "/apps/anthropic",
    "/api/coding",
    "/claudecode",
    "/anthropic",
    "/step_plan",
    "/coding",
    "/claude",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FetchedModel {
    pub id: String,
    pub owned_by: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DetectedApiFormat {
    #[serde(rename = "anthropic")]
    Anthropic,
    #[serde(rename = "openai_chat")]
    OpenAiChat,
    #[serde(rename = "openai_responses")]
    OpenAiResponses,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EndpointProbeResult {
    pub api_format: DetectedApiFormat,
    pub url: String,
    pub status_code: Option<u16>,
    pub supported: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EndpointDetectionResult {
    pub recommended_api_format: Option<DetectedApiFormat>,
    pub probes: Vec<EndpointProbeResult>,
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Option<Vec<ModelEntry>>,
}

#[derive(Debug, Deserialize)]
struct ModelEntry {
    id: String,
    owned_by: Option<String>,
}

pub async fn fetch_models(
    base_url: &str,
    api_key: &str,
    is_full_url: bool,
    models_url_override: Option<&str>,
) -> Result<Vec<FetchedModel>, String> {
    if api_key.trim().is_empty() && !base_url_allows_empty_api_key(base_url) {
        return Err("API Key is required to fetch models".to_string());
    }

    let candidates = build_models_url_candidates(base_url, is_full_url, models_url_override)?;
    let client =
        crate::oauth::new_http_client().map_err(|e| format!("Request setup failed: {e}"))?;
    let mut last_err: Option<String> = None;

    for url in &candidates {
        let mut request = client
            .get(url)
            .timeout(Duration::from_secs(FETCH_TIMEOUT_SECS));
        if !api_key.trim().is_empty() {
            request = request.header("Authorization", format!("Bearer {}", api_key.trim()));
        }

        let response = match request.send().await {
            Ok(response) => response,
            Err(e) => return Err(format!("Request failed: {e}")),
        };

        let status = response.status();
        if status.is_success() {
            let parsed: ModelsResponse = response
                .json()
                .await
                .map_err(|e| format!("Failed to parse response: {e}"))?;

            let mut models: Vec<FetchedModel> = parsed
                .data
                .unwrap_or_default()
                .into_iter()
                .map(|model| FetchedModel {
                    id: model.id,
                    owned_by: model.owned_by,
                })
                .collect();
            models.sort_by(|left, right| left.id.cmp(&right.id));
            return Ok(models);
        }

        if status == StatusCode::NOT_FOUND || status == StatusCode::METHOD_NOT_ALLOWED {
            let body = truncate_body(response.text().await.unwrap_or_default());
            last_err = Some(format!("HTTP {status}: {body}"));
            continue;
        }

        let body = truncate_body(response.text().await.unwrap_or_default());
        return Err(format!("HTTP {status}: {body}"));
    }

    Err(format!(
        "All candidates failed: {}",
        last_err.unwrap_or_else(|| "no candidates".to_string())
    ))
}

pub async fn detect_endpoint_type(
    base_url: &str,
    api_key: &str,
    is_full_url: bool,
) -> Result<EndpointDetectionResult, String> {
    if api_key.trim().is_empty() && !base_url_allows_empty_api_key(base_url) {
        return Err("API Key is required to detect endpoint type".to_string());
    }

    let client =
        crate::oauth::new_http_client().map_err(|e| format!("Request setup failed: {e}"))?;
    let probes = vec![
        probe_endpoint(
            &client,
            DetectedApiFormat::Anthropic,
            anthropic_probe_url(base_url, is_full_url)?,
            api_key,
            anthropic_probe_body(),
        )
        .await,
        probe_endpoint(
            &client,
            DetectedApiFormat::OpenAiChat,
            openai_chat_probe_url(base_url, is_full_url)?,
            api_key,
            openai_chat_probe_body(),
        )
        .await,
        probe_endpoint(
            &client,
            DetectedApiFormat::OpenAiResponses,
            openai_responses_probe_url(base_url, is_full_url)?,
            api_key,
            openai_responses_probe_body(),
        )
        .await,
    ];

    Ok(detect_endpoint_type_result(probes))
}

pub fn build_models_url_candidates(
    base_url: &str,
    is_full_url: bool,
    models_url_override: Option<&str>,
) -> Result<Vec<String>, String> {
    if let Some(raw) = models_url_override {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return Ok(vec![trimmed.to_string()]);
        }
    }

    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err("Base URL is empty".to_string());
    }

    let mut candidates = Vec::new();

    if is_full_url {
        if let Some(index) = trimmed.find("/v1/") {
            candidates.push(format!("{}/v1/models", &trimmed[..index]));
        } else if let Some(index) = trimmed.rfind('/') {
            let root = &trimmed[..index];
            if root.contains("://") && root.len() > root.find("://").unwrap() + 3 {
                candidates.push(format!("{root}/v1/models"));
            }
        }
        if candidates.is_empty() {
            return Err("Cannot derive models endpoint from full URL".to_string());
        }
        return Ok(candidates);
    }

    if trimmed.ends_with("/v1") {
        candidates.push(format!("{trimmed}/models"));
    } else {
        candidates.push(format!("{trimmed}/v1/models"));
    }

    if let Some(stripped) = strip_compat_suffix(trimmed) {
        let root = stripped.trim_end_matches('/');
        if !root.is_empty() && root.contains("://") {
            candidates.push(format!("{root}/v1/models"));
            candidates.push(format!("{root}/models"));
        }
    }

    let mut unique = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        if !unique.iter().any(|existing| existing == &candidate) {
            unique.push(candidate);
        }
    }

    Ok(unique)
}

pub fn detect_endpoint_type_result(probes: Vec<EndpointProbeResult>) -> EndpointDetectionResult {
    let recommended = recommend_api_format(&probes);
    EndpointDetectionResult {
        recommended_api_format: recommended,
        probes,
    }
}

fn recommend_api_format(probes: &[EndpointProbeResult]) -> Option<DetectedApiFormat> {
    let anthropic = probe_supported(probes, DetectedApiFormat::Anthropic);
    let openai_chat = probe_supported(probes, DetectedApiFormat::OpenAiChat);
    let openai_responses = probe_supported(probes, DetectedApiFormat::OpenAiResponses);

    match (anthropic, openai_chat, openai_responses) {
        (true, false, false) => Some(DetectedApiFormat::Anthropic),
        (false, true, false) => Some(DetectedApiFormat::OpenAiChat),
        (false, false, true) => Some(DetectedApiFormat::OpenAiResponses),
        (false, true, true) => Some(DetectedApiFormat::OpenAiChat),
        _ => None,
    }
}

fn probe_supported(probes: &[EndpointProbeResult], format: DetectedApiFormat) -> bool {
    probes
        .iter()
        .find(|probe| probe.api_format == format)
        .map(|probe| probe.supported)
        .unwrap_or(false)
}

fn classify_probe_status(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::OK
            | StatusCode::BAD_REQUEST
            | StatusCode::UNAUTHORIZED
            | StatusCode::FORBIDDEN
            | StatusCode::UNPROCESSABLE_ENTITY
            | StatusCode::TOO_MANY_REQUESTS
    )
}

fn probe_response_detail(status: StatusCode, body: String) -> Option<String> {
    let trimmed = body.trim();
    if trimmed.is_empty() && status == StatusCode::OK {
        None
    } else if trimmed.is_empty() {
        Some(format!("HTTP {}", status.as_u16()))
    } else {
        Some(truncate_body(trimmed.to_string()))
    }
}

fn anthropic_probe_url(base_url: &str, is_full_url: bool) -> Result<String, String> {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err("Base URL is empty".to_string());
    }
    if is_full_url {
        if let Some(index) = trimmed.find("/v1/") {
            return Ok(format!("{}/v1/messages", &trimmed[..index]));
        }
        if let Some(index) = trimmed.rfind('/') {
            let root = &trimmed[..index];
            if root.contains("://") && root.len() > root.find("://").unwrap() + 3 {
                return Ok(format!("{root}/v1/messages"));
            }
        }
        return Err("Cannot derive anthropic endpoint from full URL".to_string());
    }
    if trimmed.ends_with("/v1") {
        Ok(format!("{trimmed}/messages"))
    } else {
        Ok(format!("{trimmed}/v1/messages"))
    }
}

fn openai_chat_probe_url(base_url: &str, is_full_url: bool) -> Result<String, String> {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err("Base URL is empty".to_string());
    }
    if is_full_url {
        if let Some(index) = trimmed.find("/v1/") {
            return Ok(format!("{}/v1/chat/completions", &trimmed[..index]));
        }
        if let Some(index) = trimmed.rfind('/') {
            let root = &trimmed[..index];
            if root.contains("://") && root.len() > root.find("://").unwrap() + 3 {
                return Ok(format!("{root}/v1/chat/completions"));
            }
        }
        return Err("Cannot derive openai chat endpoint from full URL".to_string());
    }
    if let Some(prefix) = trimmed.strip_suffix("/anthropic") {
        return Ok(format!("{prefix}/v1/chat/completions"));
    }
    if trimmed.ends_with("/v1") {
        Ok(format!("{trimmed}/chat/completions"))
    } else {
        Ok(format!("{trimmed}/v1/chat/completions"))
    }
}

fn openai_responses_probe_url(base_url: &str, is_full_url: bool) -> Result<String, String> {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err("Base URL is empty".to_string());
    }
    if is_full_url {
        if let Some(index) = trimmed.find("/v1/") {
            return Ok(format!("{}/v1/responses", &trimmed[..index]));
        }
        if let Some(index) = trimmed.rfind('/') {
            let root = &trimmed[..index];
            if root.contains("://") && root.len() > root.find("://").unwrap() + 3 {
                return Ok(format!("{root}/v1/responses"));
            }
        }
        return Err("Cannot derive openai responses endpoint from full URL".to_string());
    }
    if trimmed.ends_with("/responses") {
        Ok(trimmed.to_string())
    } else if trimmed.ends_with("/v1") {
        Ok(format!("{trimmed}/responses"))
    } else {
        Ok(format!("{trimmed}/v1/responses"))
    }
}

fn truncate_body(body: String) -> String {
    if body.chars().count() <= ERROR_BODY_MAX_CHARS {
        body
    } else {
        let mut truncated: String = body.chars().take(ERROR_BODY_MAX_CHARS).collect();
        truncated.push('…');
        truncated
    }
}

async fn probe_endpoint(
    client: &reqwest::Client,
    api_format: DetectedApiFormat,
    url: String,
    api_key: &str,
    body: serde_json::Value,
) -> EndpointProbeResult {
    let mut request = client
        .post(&url)
        .timeout(Duration::from_secs(FETCH_TIMEOUT_SECS))
        .header("content-type", "application/json")
        .header("accept", "application/json");
    if !api_key.trim().is_empty() {
        request = request.header("authorization", format!("Bearer {}", api_key.trim()));
    }

    match request.json(&body).send().await {
        Ok(response) => {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            EndpointProbeResult {
                api_format,
                url,
                status_code: Some(status.as_u16()),
                supported: classify_probe_status(status),
                error: probe_response_detail(status, body),
            }
        }
        Err(error) => EndpointProbeResult {
            api_format,
            url,
            status_code: None,
            supported: false,
            error: Some(error.to_string()),
        },
    }
}

fn anthropic_probe_body() -> serde_json::Value {
    serde_json::json!({
        "model": "detect-probe",
        "max_tokens": 1,
        "messages": [{ "role": "user", "content": "ping" }],
        "stream": false
    })
}

fn openai_chat_probe_body() -> serde_json::Value {
    serde_json::json!({
        "model": "detect-probe",
        "messages": [{ "role": "user", "content": "ping" }],
        "max_tokens": 1,
        "stream": false
    })
}

fn openai_responses_probe_body() -> serde_json::Value {
    serde_json::json!({
        "model": "detect-probe",
        "input": [{ "role": "user", "content": "ping" }],
        "stream": false
    })
}

fn base_url_allows_empty_api_key(base_url: &str) -> bool {
    let normalized = base_url.trim().to_ascii_lowercase();
    normalized.starts_with("http://127.0.0.1")
        || normalized.starts_with("https://127.0.0.1")
        || normalized.starts_with("http://localhost")
        || normalized.starts_with("https://localhost")
        || normalized.starts_with("http://0.0.0.0")
        || normalized.starts_with("https://0.0.0.0")
}

fn strip_compat_suffix(base_url: &str) -> Option<&str> {
    for suffix in KNOWN_COMPAT_SUFFIXES {
        if base_url.ends_with(*suffix) {
            return Some(&base_url[..base_url.len() - suffix.len()]);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_models_url_for_plain_root() {
        let candidates =
            build_models_url_candidates("https://api.siliconflow.cn", false, None).unwrap();
        assert_eq!(candidates, vec!["https://api.siliconflow.cn/v1/models"]);
    }

    #[test]
    fn builds_models_url_for_full_request_url() {
        let candidates = build_models_url_candidates(
            "https://proxy.example.com/v1/chat/completions",
            true,
            None,
        )
        .unwrap();
        assert_eq!(candidates, vec!["https://proxy.example.com/v1/models"]);
    }

    #[test]
    fn uses_override_when_present() {
        let candidates = build_models_url_candidates(
            "https://api.deepseek.com/anthropic",
            false,
            Some("https://api.deepseek.com/models"),
        )
        .unwrap();
        assert_eq!(candidates, vec!["https://api.deepseek.com/models"]);
    }

    #[test]
    fn strips_known_compat_suffixes() {
        let candidates =
            build_models_url_candidates("https://api.deepseek.com/anthropic", false, None).unwrap();
        assert_eq!(
            candidates,
            vec![
                "https://api.deepseek.com/anthropic/v1/models",
                "https://api.deepseek.com/v1/models",
                "https://api.deepseek.com/models",
            ]
        );
    }

    #[test]
    fn longer_suffix_wins() {
        let candidates =
            build_models_url_candidates("https://api.z.ai/api/anthropic", false, None).unwrap();
        assert_eq!(
            candidates,
            vec![
                "https://api.z.ai/api/anthropic/v1/models",
                "https://api.z.ai/v1/models",
                "https://api.z.ai/models",
            ]
        );
    }

    #[test]
    fn local_base_url_allows_empty_api_key() {
        assert!(base_url_allows_empty_api_key("http://localhost:11434/v1"));
        assert!(base_url_allows_empty_api_key(
            "http://127.0.0.1:8080/anthropic"
        ));
        assert!(!base_url_allows_empty_api_key("https://api.example.com/v1"));
    }

    #[test]
    fn remote_base_url_requires_api_key() {
        let result = futures::executor::block_on(fetch_models(
            "https://api.example.com/v1",
            "",
            false,
            Some("https://api.example.com/models"),
        ));
        assert_eq!(result.unwrap_err(), "API Key is required to fetch models");
    }

    #[test]
    fn recommends_openai_chat_when_chat_and_responses_both_supported() {
        let result = detect_endpoint_type_result(vec![
            EndpointProbeResult {
                api_format: DetectedApiFormat::Anthropic,
                url: "http://localhost/v1/messages".to_string(),
                status_code: Some(404),
                supported: false,
                error: None,
            },
            EndpointProbeResult {
                api_format: DetectedApiFormat::OpenAiChat,
                url: "http://localhost/v1/chat/completions".to_string(),
                status_code: Some(401),
                supported: true,
                error: None,
            },
            EndpointProbeResult {
                api_format: DetectedApiFormat::OpenAiResponses,
                url: "http://localhost/v1/responses".to_string(),
                status_code: Some(422),
                supported: true,
                error: None,
            },
        ]);
        assert_eq!(
            result.recommended_api_format,
            Some(DetectedApiFormat::OpenAiChat)
        );
    }

    #[test]
    fn refuses_to_recommend_when_multiple_incompatible_endpoints_match() {
        let result = detect_endpoint_type_result(vec![
            EndpointProbeResult {
                api_format: DetectedApiFormat::Anthropic,
                url: "http://localhost/v1/messages".to_string(),
                status_code: Some(400),
                supported: true,
                error: None,
            },
            EndpointProbeResult {
                api_format: DetectedApiFormat::OpenAiChat,
                url: "http://localhost/v1/chat/completions".to_string(),
                status_code: Some(401),
                supported: true,
                error: None,
            },
        ]);
        assert_eq!(result.recommended_api_format, None);
    }

    #[test]
    fn classifies_supported_probe_status_codes_like_upstream_stream_check() {
        assert!(classify_probe_status(StatusCode::OK));
        assert!(classify_probe_status(StatusCode::BAD_REQUEST));
        assert!(classify_probe_status(StatusCode::UNAUTHORIZED));
        assert!(classify_probe_status(StatusCode::FORBIDDEN));
        assert!(classify_probe_status(StatusCode::UNPROCESSABLE_ENTITY));
        assert!(classify_probe_status(StatusCode::TOO_MANY_REQUESTS));
        assert!(!classify_probe_status(StatusCode::NOT_FOUND));
        assert!(!classify_probe_status(StatusCode::METHOD_NOT_ALLOWED));
    }

    #[test]
    fn derives_probe_urls_from_base_url() {
        assert_eq!(
            anthropic_probe_url("http://127.0.0.1:11434/v1", false).unwrap(),
            "http://127.0.0.1:11434/v1/messages"
        );
        assert_eq!(
            openai_chat_probe_url("http://127.0.0.1:11434/v1", false).unwrap(),
            "http://127.0.0.1:11434/v1/chat/completions"
        );
        assert_eq!(
            openai_responses_probe_url("http://127.0.0.1:11434/v1", false).unwrap(),
            "http://127.0.0.1:11434/v1/responses"
        );
    }

    #[test]
    fn keeps_response_detail_for_supported_non_success_statuses() {
        assert_eq!(
            probe_response_detail(
                StatusCode::UNAUTHORIZED,
                "{\"detail\":\"auth\"}".to_string()
            ),
            Some("{\"detail\":\"auth\"}".to_string())
        );
        assert_eq!(
            probe_response_detail(
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation failed".to_string()
            ),
            Some("validation failed".to_string())
        );
    }

    #[test]
    fn uses_http_status_fallback_when_body_is_empty() {
        assert_eq!(
            probe_response_detail(StatusCode::NOT_FOUND, String::new()),
            Some("HTTP 404".to_string())
        );
        assert_eq!(probe_response_detail(StatusCode::OK, String::new()), None);
    }
}

//! Provider adapters
//!
//! Concrete implementations of ProviderAdapter for different provider types.
//!
//! Directory structure:
//! - claude/       - Claude API adapter (passthrough)
//! - claude_auth/  - Claude Auth relay adapter
//! - codex/        - Codex OAuth adapter
//! - copilot/      - GitHub Copilot OAuth adapter
//! - deepseek/     - DeepSeek API adapter
//! - gemini/       - Google Gemini API adapter
//! - minimax/      - MiniMax API adapter
//! - openrouter/   - OpenRouter API adapter
//! - siliconflow/  - SiliconFlow API adapter
//!
//! Each adapter handles:
//! - Authentication (get_auth_token)
//! - Request transformation (transform_request)
//! - Response transformation + usage extraction (transform_response)

mod claude;
mod claude_auth;
mod codex;
mod copilot;
mod deepseek;
mod gemini;
mod minimax;
mod openrouter;
mod siliconflow;

pub use claude::ClaudeAdapter;
pub use claude_auth::ClaudeAuthAdapter;
pub use codex::CodexAdapter;
pub use copilot::CopilotAdapter;
pub use deepseek::DeepSeekAdapter;
pub use gemini::GeminiAdapter;
pub use minimax::MiniMaxAdapter;
pub use openrouter::OpenRouterAdapter;
pub use siliconflow::SiliconFlowAdapter;

use cc_switch_lib::oauth::{CodexOAuthManager, CopilotAuthManager};
use cc_switch_lib::providers::ProviderRegistry;
use std::sync::Arc;

/// Initialize the provider registry with all available adapters
pub fn create_registry(
    codex_oauth: Arc<CodexOAuthManager>,
    copilot_auth: Arc<CopilotAuthManager>,
) -> ProviderRegistry {
    let mut registry = ProviderRegistry::new();

    // Register adapters
    registry.register(Arc::new(CodexAdapter::new(codex_oauth)));
    registry.register(Arc::new(ClaudeAdapter::new()));
    registry.register(Arc::new(ClaudeAuthAdapter::new()));
    registry.register(Arc::new(MiniMaxAdapter::new()));
    registry.register(Arc::new(OpenRouterAdapter::new()));
    registry.register(Arc::new(SiliconFlowAdapter::new()));
    registry.register(Arc::new(DeepSeekAdapter::new()));
    registry.register(Arc::new(GeminiAdapter::new()));
    registry.register(Arc::new(CopilotAdapter::new(copilot_auth)));

    registry
}

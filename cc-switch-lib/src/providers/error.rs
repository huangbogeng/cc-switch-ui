//! Provider error types

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("Provider 类型不匹配: {0}")]
    TypeMismatch(String),
    #[error("认证失败: {0}")]
    AuthFailed(String),
    #[error("请求转换失败: {0}")]
    TransformFailed(String),
    #[error("响应转换失败: {0}")]
    ResponseTransformFailed(String),
    #[error("未找到适配的 Provider: {0}")]
    NoAdapterFound(String),
    #[error("Token 获取失败: {0}")]
    TokenFailed(String),
    #[error("无效配置: {0}")]
    InvalidConfig(String),
}

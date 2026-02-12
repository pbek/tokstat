pub mod copilot;
pub mod openrouter;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaInfo {
    pub provider: String,
    pub account_name: String,
    pub usage: TokenUsage,
    pub limits: Option<TokenLimits>,
    pub reset_date: Option<chrono::DateTime<chrono::Utc>>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub tokens_used: Option<u64>,
    pub requests_made: Option<u64>,
    pub cost: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenLimits {
    pub max_tokens: Option<u64>,
    pub max_requests: Option<u64>,
    pub max_cost: Option<f64>,
}

#[async_trait::async_trait]
pub trait Provider: Send + Sync {
    async fn fetch_quota(&self, credentials: &str) -> Result<QuotaInfo>;
    #[allow(dead_code)]
    fn provider_name(&self) -> &str;
}

pub async fn fetch_quota(account: &crate::storage::Account) -> Result<QuotaInfo> {
    let storage = crate::storage::SecureStorage::new()?;
    let credentials = storage.get_credentials(&account.name)?;

    let provider: Box<dyn Provider> = match account.provider.as_str() {
        "copilot" => Box::new(copilot::CopilotProvider),
        "openrouter" => Box::new(openrouter::OpenRouterProvider),
        _ => anyhow::bail!("Unknown provider: {}", account.provider),
    };

    provider.fetch_quota(&credentials).await
}

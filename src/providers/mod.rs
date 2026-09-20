pub mod azure;
pub mod copilot;
pub mod openai;
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub windows: Vec<QuotaWindow>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaWindow {
    pub label: String,
    pub used_percent: f64,
    pub reset_date: Option<chrono::DateTime<chrono::Utc>>,
    pub window_minutes: Option<u64>,
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

    if account.provider == "openai" {
        return openai::fetch_quota_refreshing(&credentials, &storage, &account.name).await;
    }

    let provider: Box<dyn Provider> = match account.provider.as_str() {
        "azure" => Box::new(azure::AzureProvider),
        "copilot" => Box::new(copilot::CopilotProvider),
        "openrouter" => Box::new(openrouter::OpenRouterProvider),
        _ => anyhow::bail!("Unknown provider: {}", account.provider),
    };

    provider.fetch_quota(&credentials).await
}

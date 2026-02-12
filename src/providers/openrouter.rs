use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use super::{Provider, QuotaInfo, TokenUsage, TokenLimits};

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenRouterCredentials {
    pub api_key: String,
}

pub struct OpenRouterProvider;

#[async_trait::async_trait]
impl Provider for OpenRouterProvider {
    async fn fetch_quota(&self, credentials: &str) -> Result<QuotaInfo> {
        let creds: OpenRouterCredentials = serde_json::from_str(credentials)
            .context("Failed to parse OpenRouter credentials")?;
        
        let client = reqwest::Client::new();
        
        // Fetch credits/usage from OpenRouter API
        let response = client
            .get("https://openrouter.ai/api/v1/auth/key")
            .header("Authorization", format!("Bearer {}", creds.api_key))
            .send()
            .await
            .context("Failed to fetch OpenRouter credits")?;
        
        if !response.status().is_success() {
            anyhow::bail!("Failed to fetch OpenRouter quota: {}", response.status());
        }
        
        let key_data: OpenRouterKeyResponse = response.json().await
            .context("Failed to parse OpenRouter response")?;
        
        Ok(QuotaInfo {
            provider: "openrouter".to_string(),
            account_name: "".to_string(), // Will be filled by caller
            usage: TokenUsage {
                tokens_used: None,
                requests_made: None,
                cost: Some(key_data.data.usage),
            },
            limits: Some(TokenLimits {
                max_tokens: None,
                max_requests: None,
                max_cost: key_data.data.limit,
            }),
            reset_date: None,
            last_updated: chrono::Utc::now(),
        })
    }
    
    fn provider_name(&self) -> &str {
        "openrouter"
    }
}

#[derive(Debug, Deserialize)]
struct OpenRouterKeyResponse {
    data: OpenRouterKeyData,
}

#[derive(Debug, Deserialize)]
struct OpenRouterKeyData {
    usage: f64,
    limit: Option<f64>,
}

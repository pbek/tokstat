use super::{Provider, QuotaInfo, TokenLimits, TokenUsage};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CopilotCredentials {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

pub struct CopilotProvider;

#[async_trait::async_trait]
impl Provider for CopilotProvider {
    async fn fetch_quota(&self, credentials: &str) -> Result<QuotaInfo> {
        let creds: CopilotCredentials =
            serde_json::from_str(credentials).context("Failed to parse Copilot credentials")?;

        // Check if token is expired
        let now = chrono::Utc::now();
        let access_token = if now >= creds.expires_at {
            // Token expired, need to refresh
            // For now, return error - refresh logic would go here
            anyhow::bail!("Access token expired, please login again");
        } else {
            creds.access_token.clone()
        };

        let client = reqwest::Client::new();

        // Fetch usage data from GitHub Copilot API
        // Note: This endpoint might change, verify with GitHub's API docs
        let response = client
            .get("https://api.github.com/copilot_internal/v2/usage")
            .header("Authorization", format!("Bearer {}", access_token))
            .header("User-Agent", "ai-quota-monitor")
            .send()
            .await
            .context("Failed to fetch Copilot usage")?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to fetch Copilot quota: {}", response.status());
        }

        let usage_data: CopilotUsageResponse = response
            .json()
            .await
            .context("Failed to parse Copilot usage response")?;

        Ok(QuotaInfo {
            provider: "copilot".to_string(),
            account_name: "".to_string(), // Will be filled by caller
            usage: TokenUsage {
                tokens_used: Some(usage_data.total_tokens_used),
                requests_made: Some(usage_data.total_requests),
                cost: None, // Copilot is subscription-based
            },
            limits: Some(TokenLimits {
                max_tokens: None, // No hard limit for Copilot
                max_requests: None,
                max_cost: None,
            }),
            reset_date: usage_data.billing_cycle_end,
            last_updated: chrono::Utc::now(),
        })
    }

    fn provider_name(&self) -> &str {
        "copilot"
    }
}

#[derive(Debug, Deserialize)]
struct CopilotUsageResponse {
    total_tokens_used: u64,
    total_requests: u64,
    billing_cycle_end: Option<chrono::DateTime<chrono::Utc>>,
}

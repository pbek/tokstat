use super::{Provider, QuotaInfo, TokenLimits, TokenUsage};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct CopilotCredentials {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

pub struct CopilotProvider;

// NOTE: GitHub does not provide a public API for individual Copilot usage data.
// The endpoint used below is an internal endpoint that may not be accessible
// to regular users. Copilot usage is typically only available through:
// 1. The GitHub web interface (https://github.com/settings/copilot)
// 2. Organization-level APIs (for enterprise admins)
// 3. GitHub CLI (gh copilot usage) - which may use internal endpoints
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
        let response = client
            .get("https://api.github.com/copilot_internal/user")
            .header("Authorization", format!("Bearer {}", access_token))
            .header("User-Agent", "ai-quota-monitor")
            .send()
            .await
            .context("Failed to fetch Copilot usage")?;

        if !response.status().is_success() {
            let status = response.status();
            if status == 404 {
                anyhow::bail!(
                    "Copilot usage API not accessible (404). GitHub doesn't provide a public API \
                     for individual Copilot usage. Please check your usage at: \
                     https://github.com/settings/copilot"
                );
            } else if status == 401 || status == 403 {
                anyhow::bail!(
                    "Access denied to Copilot usage API ({}). Your token may not have the required permissions.",
                    status
                );
            } else {
                anyhow::bail!("Failed to fetch Copilot quota: {}", status);
            }
        }

        let usage_data: Value = response
            .json()
            .await
            .context("Failed to parse Copilot usage response")?;

        let premium_snapshot = usage_data
            .get("quota_snapshots")
            .and_then(|snapshots| snapshots.get("premium_interactions"));
        let (premium_used, premium_limit) = if let Some(snapshot) = premium_snapshot {
            let unlimited = snapshot
                .get("unlimited")
                .and_then(Value::as_bool)
                .unwrap_or(false);

            if unlimited {
                (None, None)
            } else {
                let entitlement = snapshot.get("entitlement").and_then(Value::as_u64);
                let remaining = snapshot.get("remaining").and_then(Value::as_u64);

                if let (Some(entitlement), Some(remaining)) = (entitlement, remaining) {
                    let used = entitlement.saturating_sub(remaining);
                    (Some(used), Some(entitlement))
                } else {
                    (None, None)
                }
            }
        } else {
            (None, None)
        };

        let reset_date = usage_data
            .get("quota_reset_date")
            .and_then(Value::as_str)
            .and_then(|value| {
                chrono::DateTime::parse_from_rfc3339(value)
                    .ok()
                    .map(|dt| dt.with_timezone(&chrono::Utc))
            });

        Ok(QuotaInfo {
            provider: "copilot".to_string(),
            account_name: "".to_string(), // Will be filled by caller
            usage: TokenUsage {
                tokens_used: None,
                requests_made: premium_used,
                cost: None, // Copilot is subscription-based
            },
            limits: Some(TokenLimits {
                max_tokens: None,
                max_requests: premium_limit,
                max_cost: None,
            }),
            reset_date,
            last_updated: chrono::Utc::now(),
        })
    }

    fn provider_name(&self) -> &str {
        "copilot"
    }
}

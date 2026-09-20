use super::{Provider, QuotaInfo, QuotaWindow, TokenLimits, TokenUsage};
use anyhow::{Context, Result};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";
const TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAICredentials {
    pub access_token: String,
    pub refresh_token: String,
    pub id_token: String,
    pub account_id: String,
}

pub struct OpenAIProvider;

#[async_trait::async_trait]
impl Provider for OpenAIProvider {
    async fn fetch_quota(&self, credentials: &str) -> Result<QuotaInfo> {
        let creds: OpenAICredentials =
            serde_json::from_str(credentials).context("Failed to parse OpenAI credentials")?;
        fetch_with_credentials(&creds).await
    }

    fn provider_name(&self) -> &str {
        "openai"
    }
}

pub async fn fetch_quota_refreshing(
    credentials: &str,
    storage: &crate::storage::SecureStorage,
    account_name: &str,
) -> Result<QuotaInfo> {
    let mut creds: OpenAICredentials =
        serde_json::from_str(credentials).context("Failed to parse OpenAI credentials")?;

    match fetch_with_credentials(&creds).await {
        Ok(quota) => Ok(quota),
        Err(error) if is_unauthorized(&error) => {
            creds = refresh_credentials(&creds).await?;
            let serialized =
                serde_json::to_string(&creds).context("Failed to serialize OpenAI credentials")?;
            storage
                .store_credentials(account_name, &serialized)
                .context("Failed to store refreshed OpenAI credentials")?;
            fetch_with_credentials(&creds).await
        }
        Err(error) => Err(error),
    }
}

async fn fetch_with_credentials(creds: &OpenAICredentials) -> Result<QuotaInfo> {
    let response = reqwest::Client::new()
        .get(USAGE_URL)
        .bearer_auth(&creds.access_token)
        .header("ChatGPT-Account-Id", &creds.account_id)
        .header("User-Agent", "tokstat")
        .send()
        .await
        .context("Failed to fetch OpenAI subscription usage")?;

    if !response.status().is_success() {
        let status = response.status();
        anyhow::bail!("Failed to fetch OpenAI subscription usage: {status}");
    }

    let usage: UsageResponse = response
        .json()
        .await
        .context("Failed to parse OpenAI subscription usage")?;

    Ok(quota_from_usage(usage))
}

fn quota_from_usage(usage: UsageResponse) -> QuotaInfo {
    let rate_limit = usage.rate_limit.flatten();
    let mut windows = Vec::new();

    if let Some(rate_limit) = rate_limit {
        if let Some(window) = rate_limit.primary_window.flatten() {
            windows.push(map_window(window, "5h limit"));
        }
        if let Some(window) = rate_limit.secondary_window.flatten() {
            windows.push(map_window(window, "Weekly limit"));
        }
    }

    let reset_date = windows.iter().filter_map(|window| window.reset_date).min();

    QuotaInfo {
        provider: "openai".to_string(),
        account_name: String::new(),
        usage: TokenUsage {
            tokens_used: None,
            requests_made: None,
            cost: None,
        },
        limits: Some(TokenLimits {
            max_tokens: None,
            max_requests: None,
            max_cost: None,
        }),
        reset_date,
        windows,
        last_updated: chrono::Utc::now(),
    }
}

fn map_window(window: RateLimitWindow, fallback_label: &str) -> QuotaWindow {
    let window_minutes = window
        .limit_window_seconds
        .map(|seconds| seconds.div_ceil(60));
    let label = match window_minutes {
        Some(300) => "5h limit".to_string(),
        Some(10_080) => "Weekly limit".to_string(),
        _ => fallback_label.to_string(),
    };
    let reset_date = window
        .reset_at
        .and_then(|timestamp| chrono::DateTime::from_timestamp(timestamp, 0));

    QuotaWindow {
        label,
        used_percent: window.used_percent,
        reset_date,
        window_minutes,
    }
}

async fn refresh_credentials(creds: &OpenAICredentials) -> Result<OpenAICredentials> {
    let response = reqwest::Client::new()
        .post(TOKEN_URL)
        .json(&serde_json::json!({
            "grant_type": "refresh_token",
            "client_id": CLIENT_ID,
            "refresh_token": creds.refresh_token,
        }))
        .send()
        .await
        .context("Failed to refresh OpenAI login")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "OpenAI login expired and could not be refreshed; run `tokstat login openai` again"
        );
    }

    let refreshed: TokenResponse = response
        .json()
        .await
        .context("Failed to parse refreshed OpenAI login")?;
    let id_token = refreshed.id_token.unwrap_or_else(|| creds.id_token.clone());

    Ok(OpenAICredentials {
        access_token: refreshed
            .access_token
            .context("OpenAI refresh response did not include an access token")?,
        refresh_token: refreshed
            .refresh_token
            .unwrap_or_else(|| creds.refresh_token.clone()),
        account_id: account_id_from_jwt(&id_token).unwrap_or_else(|| creds.account_id.clone()),
        id_token,
    })
}

fn is_unauthorized(error: &anyhow::Error) -> bool {
    error
        .to_string()
        .contains("OpenAI subscription usage: 401 Unauthorized")
}

pub(crate) fn claims_from_jwt(jwt: &str) -> Result<Value> {
    let payload = jwt.split('.').nth(1).context("Invalid OpenAI ID token")?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .context("Invalid OpenAI ID token encoding")?;
    serde_json::from_slice(&bytes).context("Invalid OpenAI ID token claims")
}

pub(crate) fn account_id_from_jwt(jwt: &str) -> Option<String> {
    claims_from_jwt(jwt)
        .ok()?
        .pointer("/https:~1~1api.openai.com~1auth/chatgpt_account_id")?
        .as_str()
        .map(str::to_string)
}

#[derive(Debug, Deserialize)]
struct UsageResponse {
    #[allow(dead_code)]
    plan_type: Option<String>,
    rate_limit: Option<Option<RateLimitDetails>>,
}

#[derive(Debug, Deserialize)]
struct RateLimitDetails {
    primary_window: Option<Option<RateLimitWindow>>,
    secondary_window: Option<Option<RateLimitWindow>>,
}

#[derive(Debug, Deserialize)]
struct RateLimitWindow {
    used_percent: f64,
    limit_window_seconds: Option<u64>,
    reset_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    id_token: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_five_hour_and_weekly_windows() {
        let usage: UsageResponse = serde_json::from_value(serde_json::json!({
            "plan_type": "plus",
            "rate_limit": {
                "primary_window": {
                    "used_percent": 12.5,
                    "limit_window_seconds": 18000,
                    "reset_at": 1_800_000_000
                },
                "secondary_window": {
                    "used_percent": 64,
                    "limit_window_seconds": 604800,
                    "reset_at": 1_800_100_000
                }
            }
        }))
        .unwrap();

        let quota = quota_from_usage(usage);
        assert_eq!(quota.windows.len(), 2);
        assert_eq!(quota.windows[0].label, "5h limit");
        assert_eq!(quota.windows[0].used_percent, 12.5);
        assert_eq!(quota.windows[1].label, "Weekly limit");
        assert_eq!(quota.windows[1].used_percent, 64.0);
    }
}

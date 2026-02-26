use super::{Provider, QuotaInfo, TokenLimits, TokenUsage};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AzureCredentials {
    pub api_key: String,
    pub resource_name: String,
}

pub struct AzureProvider;

#[async_trait::async_trait]
impl Provider for AzureProvider {
    async fn fetch_quota(&self, credentials: &str) -> Result<QuotaInfo> {
        let creds: AzureCredentials =
            serde_json::from_str(credentials).context("Failed to parse Azure credentials")?;

        let resource_name = if creds.resource_name.is_empty() {
            std::env::var("AZURE_RESOURCE_NAME").context(
                "AZURE_RESOURCE_NAME env var not set and no resource name in credentials",
            )?
        } else {
            creds.resource_name.clone()
        };

        let client = reqwest::Client::new();

        // Fetch models via the data-plane API to verify credentials and gather info
        let models_url = format!(
            "https://{}.openai.azure.com/openai/models?api-version=2024-06-01",
            resource_name
        );

        let response = client
            .get(&models_url)
            .header("api-key", &creds.api_key)
            .send()
            .await
            .context("Failed to connect to Azure OpenAI")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to fetch Azure OpenAI quota: {} - {}", status, body);
        }

        let models: AzureModelsResponse = response
            .json()
            .await
            .context("Failed to parse Azure OpenAI models response")?;

        // model_count is intentionally not used for quota display
        let _ = models.data.len();

        // Try to get token usage from the usage endpoint (may not be available)
        let usage_url = format!(
            "https://{}.openai.azure.com/openai/usage?api-version=2024-06-01",
            resource_name
        );

        let (tokens_used, requests_made) = match client
            .get(&usage_url)
            .header("api-key", &creds.api_key)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                match resp.json::<AzureUsageResponse>().await {
                    Ok(usage) => {
                        let total_tokens: u64 = usage.data.iter().map(|d| d.total_tokens).sum();
                        let total_requests: u64 = usage.data.iter().map(|d| d.total_requests).sum();
                        (Some(total_tokens), Some(total_requests))
                    }
                    Err(_) => (None, None),
                }
            }
            _ => (None, None),
        };

        // Try to list deployments for rate limit info (may not be available on all API versions)
        let deployments_url = format!(
            "https://{}.openai.azure.com/openai/deployments?api-version=2022-12-01",
            resource_name
        );

        let (total_tpm, total_rpm) = match client
            .get(&deployments_url)
            .header("api-key", &creds.api_key)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                match resp.json::<AzureDeploymentsResponse>().await {
                    Ok(deployments) => {
                        let tpm: u64 = deployments
                            .data
                            .iter()
                            .filter_map(|d| d.rate_limits.as_ref())
                            .flat_map(|rl| rl.iter())
                            .filter(|r| r.key == "token")
                            .filter_map(|r| r.renewal_period_tokens)
                            .sum();

                        let rpm: u64 = deployments
                            .data
                            .iter()
                            .filter_map(|d| d.rate_limits.as_ref())
                            .flat_map(|rl| rl.iter())
                            .filter(|r| r.key == "request")
                            .filter_map(|r| r.renewal_period_requests)
                            .sum();

                        (tpm, rpm)
                    }
                    Err(_) => (0, 0),
                }
            }
            _ => (0, 0),
        };

        Ok(QuotaInfo {
            provider: "azure".to_string(),
            account_name: "".to_string(), // Will be filled by caller
            usage: TokenUsage {
                tokens_used,
                requests_made,
                cost: None,
            },
            limits: Some(TokenLimits {
                max_tokens: if total_tpm > 0 { Some(total_tpm) } else { None },
                max_requests: if total_rpm > 0 { Some(total_rpm) } else { None },
                max_cost: None,
            }),
            reset_date: None,
            last_updated: chrono::Utc::now(),
        })
    }

    fn provider_name(&self) -> &str {
        "azure"
    }
}

#[derive(Debug, Deserialize)]
struct AzureModelsResponse {
    data: Vec<AzureModel>,
}

#[derive(Debug, Deserialize)]
struct AzureModel {
    #[allow(dead_code)]
    id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AzureDeploymentsResponse {
    data: Vec<AzureDeployment>,
}

#[derive(Debug, Deserialize)]
struct AzureDeployment {
    #[allow(dead_code)]
    id: Option<String>,
    #[allow(dead_code)]
    model: Option<String>,
    #[serde(default)]
    rate_limits: Option<Vec<AzureRateLimit>>,
}

#[derive(Debug, Deserialize)]
struct AzureRateLimit {
    key: String,
    #[serde(default)]
    renewal_period_tokens: Option<u64>,
    #[serde(default)]
    renewal_period_requests: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct AzureUsageResponse {
    data: Vec<AzureUsageEntry>,
}

#[derive(Debug, Deserialize)]
struct AzureUsageEntry {
    #[serde(default)]
    total_tokens: u64,
    #[serde(default)]
    total_requests: u64,
}

use crate::providers::Provider;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::io::{self, Write};
use std::time::{Duration, Instant};

const AUTH_BASE_URL: &str = "https://auth.openai.com";
const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";

pub async fn login(storage: &crate::storage::SecureStorage, account_name: &str) -> Result<()> {
    println!("\nOpenAI Subscription Login\n");
    println!("Requesting a device code...");

    let client = reqwest::Client::new();
    let device: DeviceCodeResponse = client
        .post(format!("{AUTH_BASE_URL}/api/accounts/deviceauth/usercode"))
        .json(&serde_json::json!({ "client_id": CLIENT_ID }))
        .send()
        .await
        .context("Failed to request an OpenAI device code")?
        .error_for_status()
        .context("OpenAI device login is unavailable")?
        .json()
        .await
        .context("Failed to parse the OpenAI device code")?;

    println!("\n1. Open {AUTH_BASE_URL}/codex/device");
    println!("2. Enter this one-time code: {}", device.user_code);
    super::prompt_to_copy_device_code(&device.user_code);
    println!("\nWaiting for authorization...");
    io::stdout().flush()?;

    let authorization = poll_for_authorization(&client, &device).await?;
    let tokens: TokenResponse = client
        .post(format!("{AUTH_BASE_URL}/oauth/token"))
        .json(&serde_json::json!({
            "grant_type": "authorization_code",
            "client_id": CLIENT_ID,
            "code": authorization.authorization_code,
            "redirect_uri": format!("{AUTH_BASE_URL}/deviceauth/callback"),
            "code_verifier": authorization.code_verifier,
        }))
        .send()
        .await
        .context("Failed to complete OpenAI login")?
        .error_for_status()
        .context("OpenAI rejected the device authorization")?
        .json()
        .await
        .context("Failed to parse OpenAI login tokens")?;

    let account_id = crate::providers::openai::account_id_from_jwt(&tokens.id_token)
        .context("OpenAI login did not include a ChatGPT account ID")?;
    let claims = crate::providers::openai::claims_from_jwt(&tokens.id_token)?;
    let email = claims
        .get("email")
        .or_else(|| claims.pointer("/https:~1~1api.openai.com~1profile/email"))
        .and_then(|value| value.as_str())
        .map(str::to_string);

    let credentials = crate::providers::openai::OpenAICredentials {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        id_token: tokens.id_token,
        account_id,
    };
    let credentials_json =
        serde_json::to_string(&credentials).context("Failed to serialize OpenAI credentials")?;

    crate::providers::openai::OpenAIProvider
        .fetch_quota(&credentials_json)
        .await?;
    storage.store_credentials(account_name, &credentials_json)?;
    storage.save_account(crate::storage::Account {
        name: account_name.to_string(),
        provider: "openai".to_string(),
        email,
        created_at: chrono::Utc::now(),
        last_updated: chrono::Utc::now(),
    })?;

    Ok(())
}

async fn poll_for_authorization(
    client: &reqwest::Client,
    device: &DeviceCodeResponse,
) -> Result<AuthorizationResponse> {
    let started = Instant::now();
    let timeout = Duration::from_secs(15 * 60);
    let interval = Duration::from_secs(device.interval.max(1));

    loop {
        let response = client
            .post(format!("{AUTH_BASE_URL}/api/accounts/deviceauth/token"))
            .json(&serde_json::json!({
                "device_auth_id": device.device_auth_id,
                "user_code": device.user_code,
            }))
            .send()
            .await
            .context("Failed while waiting for OpenAI authorization")?;

        if response.status().is_success() {
            return response
                .json()
                .await
                .context("Failed to parse OpenAI authorization");
        }
        if !matches!(response.status().as_u16(), 403 | 404) {
            anyhow::bail!("OpenAI device authorization failed: {}", response.status());
        }
        if started.elapsed() >= timeout {
            anyhow::bail!("OpenAI device authorization timed out after 15 minutes");
        }
        tokio::time::sleep(interval).await;
    }
}

#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_auth_id: String,
    #[serde(alias = "usercode")]
    user_code: String,
    #[serde(deserialize_with = "deserialize_interval")]
    interval: u64,
}

fn deserialize_interval<'de, D>(deserializer: D) -> std::result::Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Interval {
        Number(u64),
        String(String),
    }

    match Interval::deserialize(deserializer)? {
        Interval::Number(value) => Ok(value),
        Interval::String(value) => value.parse().map_err(serde::de::Error::custom),
    }
}

#[derive(Debug, Deserialize)]
struct AuthorizationResponse {
    authorization_code: String,
    code_verifier: String,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    id_token: String,
    access_token: String,
    refresh_token: String,
}

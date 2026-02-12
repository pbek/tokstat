use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use tokio::time::{sleep, Duration};

const GITHUB_CLIENT_ID: &str = "Iv1.b507a08c87ecfe98"; // GitHub CLI client ID
const GITHUB_DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const GITHUB_SCOPES: &str = "read:user";

#[derive(Debug, Serialize)]
struct DeviceCodeRequest {
    client_id: String,
    scope: String,
}

#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Debug, Serialize)]
struct AccessTokenRequest {
    client_id: String,
    device_code: String,
    grant_type: String,
}

#[derive(Debug, Deserialize)]
struct AccessTokenResponse {
    access_token: Option<String>,
    token_type: Option<String>,
    scope: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

pub async fn login(storage: &crate::storage::SecureStorage, account_name: &str) -> Result<()> {
    println!("\n🔐 GitHub Copilot Login Flow\n");
    
    let client = reqwest::Client::new();
    
    // Step 1: Request device code
    println!("Requesting device code...");
    let device_response = client
        .post(GITHUB_DEVICE_CODE_URL)
        .header("Accept", "application/json")
        .form(&DeviceCodeRequest {
            client_id: GITHUB_CLIENT_ID.to_string(),
            scope: GITHUB_SCOPES.to_string(),
        })
        .send()
        .await
        .context("Failed to request device code")?;
    
    let device_code_data: DeviceCodeResponse = device_response
        .json()
        .await
        .context("Failed to parse device code response")?;
    
    // Step 2: Display code to user
    println!("\n{}", "━".repeat(60));
    println!("Please visit: {}", device_code_data.verification_uri);
    println!("And enter code: {}", device_code_data.user_code);
    println!("{}", "━".repeat(60));
    println!("\nWaiting for authorization...");
    
    // Step 3: Poll for access token
    let interval = Duration::from_secs(device_code_data.interval);
    let max_attempts = device_code_data.expires_in / device_code_data.interval;
    
    let mut access_token = None;
    
    for _attempt in 0..max_attempts {
        sleep(interval).await;
        
        let token_response = client
            .post(GITHUB_TOKEN_URL)
            .header("Accept", "application/json")
            .form(&AccessTokenRequest {
                client_id: GITHUB_CLIENT_ID.to_string(),
                device_code: device_code_data.device_code.clone(),
                grant_type: "urn:ietf:params:oauth:grant-type:device_code".to_string(),
            })
            .send()
            .await
            .context("Failed to request access token")?;
        
        let token_data: AccessTokenResponse = token_response
            .json()
            .await
            .context("Failed to parse token response")?;
        
        if let Some(token) = token_data.access_token {
            access_token = Some(token);
            break;
        }
        
        if let Some(error) = token_data.error {
            match error.as_str() {
                "authorization_pending" => {
                    // Still waiting for user
                    print!(".");
                    io::stdout().flush()?;
                    continue;
                }
                "slow_down" => {
                    // We're polling too fast
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
                "expired_token" => {
                    anyhow::bail!("Device code expired, please try again");
                }
                "access_denied" => {
                    anyhow::bail!("Access denied by user");
                }
                _ => {
                    anyhow::bail!("OAuth error: {} - {}", error, 
                                  token_data.error_description.unwrap_or_default());
                }
            }
        }
    }
    
    let access_token = access_token
        .context("Failed to obtain access token (timeout)")?;
    
    println!("\n✓ Authorization successful!");
    
    // Now we need to exchange this for Copilot tokens
    // This part depends on GitHub's Copilot API authentication
    // For now, we'll store the GitHub token
    
    let credentials = crate::providers::copilot::CopilotCredentials {
        access_token: access_token.clone(),
        refresh_token: String::new(), // GitHub doesn't provide refresh tokens with device flow
        expires_at: chrono::Utc::now() + chrono::Duration::days(90), // GitHub tokens don't expire by default
    };
    
    let credentials_json = serde_json::to_string(&credentials)
        .context("Failed to serialize credentials")?;
    
    storage.store_credentials(account_name, &credentials_json)
        .context("Failed to store credentials")?;
    
    // Store account metadata
    let account = crate::storage::Account {
        name: account_name.to_string(),
        provider: "copilot".to_string(),
        email: None,
        created_at: chrono::Utc::now(),
        last_updated: chrono::Utc::now(),
    };
    
    storage.save_account(account)
        .context("Failed to save account")?;
    
    Ok(())
}

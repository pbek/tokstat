# Adding New Providers

This guide shows you how to add support for new AI providers to the quota monitor.

## Overview

The application uses a pluggable provider architecture. To add a new provider, you need to:

1. Create a provider implementation
2. Add authentication logic
3. Register the provider in the main application

## Step-by-Step Example: Adding Anthropic

### 1. Create Provider Implementation

Create `src/providers/anthropic.rs`:

```rust
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use super::{Provider, QuotaInfo, TokenUsage, TokenLimits};

#[derive(Debug, Serialize, Deserialize)]
pub struct AnthropicCredentials {
    pub api_key: String,
}

pub struct AnthropicProvider;

#[async_trait::async_trait]
impl Provider for AnthropicProvider {
    async fn fetch_quota(&self, credentials: &str) -> Result<QuotaInfo> {
        let creds: AnthropicCredentials = serde_json::from_str(credentials)
            .context("Failed to parse Anthropic credentials")?;

        let client = reqwest::Client::new();

        // Make API request to fetch usage
        // Adjust the endpoint and response parsing based on Anthropic's actual API
        let response = client
            .get("https://api.anthropic.com/v1/usage")
            .header("x-api-key", &creds.api_key)
            .header("anthropic-version", "2023-06-01")
            .send()
            .await
            .context("Failed to fetch Anthropic usage")?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to fetch Anthropic quota: {}", response.status());
        }

        let usage_data: AnthropicUsageResponse = response.json().await
            .context("Failed to parse Anthropic usage response")?;

        Ok(QuotaInfo {
            provider: "anthropic".to_string(),
            account_name: "".to_string(), // Will be filled by caller
            usage: TokenUsage {
                tokens_used: Some(usage_data.tokens_used),
                requests_made: Some(usage_data.requests_made),
                cost: Some(usage_data.estimated_cost),
            },
            limits: Some(TokenLimits {
                max_tokens: usage_data.token_limit,
                max_requests: None,
                max_cost: None,
            }),
            reset_date: usage_data.reset_date,
            last_updated: chrono::Utc::now(),
        })
    }

    fn provider_name(&self) -> &str {
        "anthropic"
    }
}

#[derive(Debug, Deserialize)]
struct AnthropicUsageResponse {
    tokens_used: u64,
    requests_made: u64,
    estimated_cost: f64,
    token_limit: Option<u64>,
    reset_date: Option<chrono::DateTime<chrono::Utc>>,
}
```

### 2. Add Authentication Logic

Create `src/auth/anthropic.rs`:

```rust
use anyhow::{Context, Result};
use std::io::{self, Write};

pub async fn login(storage: &crate::storage::SecureStorage, account_name: &str) -> Result<()> {
    println!("\n🔐 Anthropic Login\n");
    println!("Get your API key from: https://console.anthropic.com/settings/keys\n");

    print!("Enter your Anthropic API key: ");
    io::stdout().flush()?;

    let mut api_key = String::new();
    io::stdin().read_line(&mut api_key)
        .context("Failed to read API key")?;

    let api_key = api_key.trim().to_string();

    if api_key.is_empty() {
        anyhow::bail!("API key cannot be empty");
    }

    // Validate the API key
    println!("\nValidating API key...");

    let client = reqwest::Client::new();
    let response = client
        .get("https://api.anthropic.com/v1/usage")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .send()
        .await
        .context("Failed to validate API key")?;

    if !response.status().is_success() {
        anyhow::bail!("Invalid API key: {}", response.status());
    }

    println!("✓ API key validated successfully!");

    // Store credentials
    let credentials = crate::providers::anthropic::AnthropicCredentials {
        api_key: api_key.clone(),
    };

    let credentials_json = serde_json::to_string(&credentials)
        .context("Failed to serialize credentials")?;

    storage.store_credentials(account_name, &credentials_json)
        .context("Failed to store credentials")?;

    // Store account metadata
    let account = crate::storage::Account {
        name: account_name.to_string(),
        provider: "anthropic".to_string(),
        email: None,
        created_at: chrono::Utc::now(),
        last_updated: chrono::Utc::now(),
    };

    storage.save_account(account)
        .context("Failed to save account")?;

    Ok(())
}
```

### 3. Register the Provider

Update `src/providers/mod.rs`:

```rust
pub mod copilot;
pub mod openrouter;
pub mod anthropic;  // Add this line

// ... rest of the file ...

pub async fn fetch_quota(account: &crate::storage::Account) -> Result<QuotaInfo> {
    let storage = crate::storage::SecureStorage::new()?;
    let credentials = storage.get_credentials(&account.name)?;

    let provider: Box<dyn Provider> = match account.provider.as_str() {
        "copilot" => Box::new(copilot::CopilotProvider),
        "openrouter" => Box::new(openrouter::OpenRouterProvider),
        "anthropic" => Box::new(anthropic::AnthropicProvider),  // Add this line
        _ => anyhow::bail!("Unknown provider: {}", account.provider),
    };

    provider.fetch_quota(&credentials).await
}
```

Update `src/auth/mod.rs`:

```rust
pub mod copilot;
pub mod openrouter;
pub mod anthropic;  // Add this line
```

### 4. Update Main CLI

Update `src/main.rs` to include the new provider in the login command:

```rust
Commands::Login { provider, name } => {
    info!("Logging into {} provider", provider);
    let account_name = name.unwrap_or_else(|| format!("{}_{}", provider, chrono::Utc::now().timestamp()));

    match provider.as_str() {
        "copilot" => {
            auth::copilot::login(&storage, &account_name).await?;
            println!("✓ Successfully logged into GitHub Copilot as '{}'", account_name);
        }
        "openrouter" => {
            auth::openrouter::login(&storage, &account_name).await?;
            println!("✓ Successfully added OpenRouter account '{}'", account_name);
        }
        "anthropic" => {  // Add this block
            auth::anthropic::login(&storage, &account_name).await?;
            println!("✓ Successfully added Anthropic account '{}'", account_name);
        }
        _ => unreachable!("Invalid provider"),
    }
}
```

And update the clap parser to accept the new provider:

```rust
Login {
    /// Provider name (copilot, openrouter, anthropic)
    #[arg(value_parser = ["copilot", "openrouter", "anthropic"])]  // Add "anthropic" here
    provider: String,

    /// Account name/alias
    #[arg(short, long)]
    name: Option<String>,
},
```

### 5. Test the Integration

```bash
# Build
nix develop
cargo build

# Test login
cargo run -- login anthropic --name my-anthropic

# Test quota fetch
cargo run -- refresh my-anthropic

# View in dashboard
cargo run -- dashboard
```

## Tips for Implementation

1. **API Documentation**: Always refer to the provider's official API documentation
2. **Error Handling**: Use proper error messages that help users understand what went wrong
3. **Rate Limiting**: Consider implementing rate limiting for API calls
4. **Caching**: Cache quota data to avoid excessive API calls
5. **Token Refresh**: If the provider uses OAuth, implement token refresh logic
6. **Testing**: Add tests for your provider implementation

## Provider Trait Reference

```rust
#[async_trait::async_trait]
pub trait Provider: Send + Sync {
    /// Fetch quota information from the provider
    async fn fetch_quota(&self, credentials: &str) -> Result<QuotaInfo>;

    /// Return the provider name
    fn provider_name(&self) -> &str;
}
```

## Data Structures

```rust
pub struct QuotaInfo {
    pub provider: String,
    pub account_name: String,
    pub usage: TokenUsage,
    pub limits: Option<TokenLimits>,
    pub reset_date: Option<chrono::DateTime<chrono::Utc>>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

pub struct TokenUsage {
    pub tokens_used: Option<u64>,
    pub requests_made: Option<u64>,
    pub cost: Option<f64>,
}

pub struct TokenLimits {
    pub max_tokens: Option<u64>,
    pub max_requests: Option<u64>,
    pub max_cost: Option<f64>,
}
```

All fields are optional to accommodate different providers' capabilities.

## Contributing

When adding a new provider, please:

1. Test thoroughly with real credentials
2. Add provider documentation to README.md
3. Update QUICKSTART.md with usage examples
4. Consider adding unit tests
5. Submit a pull request with your changes

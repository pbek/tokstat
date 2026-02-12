use anyhow::{Context, Result};
use std::io::{self, Write};

pub async fn login(storage: &crate::storage::SecureStorage, account_name: &str) -> Result<()> {
    println!("\n🔐 OpenRouter Login\n");
    println!("You can find your API key at: https://openrouter.ai/keys\n");
    
    print!("Enter your OpenRouter API key: ");
    io::stdout().flush()?;
    
    let mut api_key = String::new();
    io::stdin().read_line(&mut api_key)
        .context("Failed to read API key")?;
    
    let api_key = api_key.trim().to_string();
    
    if api_key.is_empty() {
        anyhow::bail!("API key cannot be empty");
    }
    
    // Validate the API key by making a test request
    println!("\nValidating API key...");
    
    let client = reqwest::Client::new();
    let response = client
        .get("https://openrouter.ai/api/v1/auth/key")
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .context("Failed to validate API key")?;
    
    if !response.status().is_success() {
        anyhow::bail!("Invalid API key: {}", response.status());
    }
    
    println!("✓ API key validated successfully!");
    
    // Store credentials
    let credentials = crate::providers::openrouter::OpenRouterCredentials {
        api_key: api_key.clone(),
    };
    
    let credentials_json = serde_json::to_string(&credentials)
        .context("Failed to serialize credentials")?;
    
    storage.store_credentials(account_name, &credentials_json)
        .context("Failed to store credentials")?;
    
    // Store account metadata
    let account = crate::storage::Account {
        name: account_name.to_string(),
        provider: "openrouter".to_string(),
        email: None,
        created_at: chrono::Utc::now(),
        last_updated: chrono::Utc::now(),
    };
    
    storage.save_account(account)
        .context("Failed to save account")?;
    
    Ok(())
}

use anyhow::{Context, Result};
use std::io::{self, Write};

pub async fn login(storage: &crate::storage::SecureStorage, account_name: &str) -> Result<()> {
    println!("\n🔐 Azure OpenAI Login\n");
    println!("You need your Azure OpenAI API key and resource name.");
    println!("Find them in the Azure Portal under your Azure OpenAI resource.\n");

    // Get resource name (check env var first)
    let resource_name = match std::env::var("AZURE_RESOURCE_NAME") {
        Ok(name) if !name.is_empty() => {
            println!("Using resource name from AZURE_RESOURCE_NAME: {}", name);
            name
        }
        _ => {
            print!("Enter your Azure resource name: ");
            io::stdout().flush()?;

            let mut resource_name = String::new();
            io::stdin()
                .read_line(&mut resource_name)
                .context("Failed to read resource name")?;

            let resource_name = resource_name.trim().to_string();

            if resource_name.is_empty() {
                anyhow::bail!("Resource name cannot be empty");
            }

            resource_name
        }
    };

    print!("Enter your Azure OpenAI API key: ");
    io::stdout().flush()?;

    let mut api_key = String::new();
    io::stdin()
        .read_line(&mut api_key)
        .context("Failed to read API key")?;

    let api_key = api_key.trim().to_string();

    if api_key.is_empty() {
        anyhow::bail!("API key cannot be empty");
    }

    // Validate by listing models (data-plane endpoint available with api-key auth)
    println!("\nValidating credentials...");

    let client = reqwest::Client::new();
    let url = format!(
        "https://{}.openai.azure.com/openai/models?api-version=2024-06-01",
        resource_name
    );

    let response = client
        .get(&url)
        .header("api-key", &api_key)
        .send()
        .await
        .context("Failed to connect to Azure OpenAI")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Invalid credentials: {} - {}", status, body);
    }

    println!("✓ Credentials validated successfully!");

    // Store credentials
    let credentials = crate::providers::azure::AzureCredentials {
        api_key,
        resource_name,
    };

    let credentials_json =
        serde_json::to_string(&credentials).context("Failed to serialize credentials")?;

    storage
        .store_credentials(account_name, &credentials_json)
        .context("Failed to store credentials")?;

    // Store account metadata
    let account = crate::storage::Account {
        name: account_name.to_string(),
        provider: "azure".to_string(),
        email: None,
        created_at: chrono::Utc::now(),
        last_updated: chrono::Utc::now(),
    };

    storage
        .save_account(account)
        .context("Failed to save account")?;

    Ok(())
}

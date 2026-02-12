mod auth;
mod providers;
mod storage;
mod ui;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use tracing::info;
use tracing_subscriber;
use std::io;

#[derive(Parser)]
#[command(name = "tokstat")]
#[command(about = "Monitor token quotas across multiple AI providers", long_about = None)]
#[command(version)]
struct Cli {
    /// Generate shell completions
    #[arg(long = "generate", value_enum)]
    generator: Option<Shell>,
    
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Add and login to a new provider account
    Login {
        /// Provider name (copilot, openrouter)
        #[arg(value_parser = ["copilot", "openrouter"])]
        provider: String,
        
        /// Account name/alias
        #[arg(short, long)]
        name: Option<String>,
    },
    
    /// List all configured accounts
    List,
    
    /// Show quota dashboard for all accounts
    Dashboard,
    
    /// Remove an account
    Remove {
        /// Account name to remove
        name: String,
    },
    
    /// Refresh quota information
    Refresh {
        /// Specific account name (refreshes all if not specified)
        name: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    
    // Handle shell completion generation
    if let Some(generator) = cli.generator {
        let mut cmd = Cli::command();
        eprintln!("Generating completion file for {generator:?}...");
        generate(generator, &mut cmd, "tokstat", &mut io::stdout());
        return Ok(());
    }
    
    // Ensure a command was provided
    let Some(command) = cli.command else {
        eprintln!("Error: No command provided. Use --help for usage information.");
        std::process::exit(1);
    };
    
    // Initialize storage
    let storage = storage::SecureStorage::new()?;

    match command {
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
                _ => unreachable!("Invalid provider"),
            }
        }
        
        Commands::List => {
            let accounts = storage.list_accounts()?;
            
            if accounts.is_empty() {
                println!("No accounts configured. Use 'tokstat login' to add an account.");
            } else {
                println!("\nConfigured Accounts:");
                println!("{}", "─".repeat(50));
                for account in accounts {
                    println!("  {} ({})", account.name, account.provider);
                }
                println!();
            }
        }
        
        Commands::Dashboard => {
            let accounts = storage.list_accounts()?;
            
            if accounts.is_empty() {
                println!("No accounts configured. Use 'tokstat login' to add an account.");
                return Ok(());
            }
            
            ui::dashboard::run(storage, accounts).await?;
        }
        
        Commands::Remove { name } => {
            storage.remove_account(&name)?;
            println!("✓ Removed account '{}'", name);
        }
        
        Commands::Refresh { name } => {
            if let Some(account_name) = name {
                println!("Refreshing quota for '{}'...", account_name);
                let account = storage.get_account(&account_name)?;
                let quota = providers::fetch_quota(&account).await?;
                println!("{:#?}", quota);
            } else {
                println!("Refreshing all accounts...");
                let accounts = storage.list_accounts()?;
                for account in accounts {
                    println!("\n{} ({}):", account.name, account.provider);
                    match providers::fetch_quota(&account).await {
                        Ok(quota) => println!("  {:#?}", quota),
                        Err(e) => println!("  Error: {}", e),
                    }
                }
            }
        }
    }

    Ok(())
}

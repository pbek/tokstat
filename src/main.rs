mod auth;
mod providers;
mod storage;
mod ui;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use std::io;
use tracing::info;
use tracing_subscriber;

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

    /// Show version information
    Version,
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

    // Handle commands (or fall back to status display)
    // Initialize storage
    let storage = storage::SecureStorage::new()?;

    match cli.command {
        Some(command) => match command {
            Commands::Login { provider, name } => {
                info!("Logging into {} provider", provider);
                let account_name = name
                    .unwrap_or_else(|| format!("{}_{}", provider, chrono::Utc::now().timestamp()));

                match provider.as_str() {
                    "copilot" => {
                        auth::copilot::login(&storage, &account_name).await?;
                        println!(
                            "✓ Successfully logged into GitHub Copilot as '{}'",
                            account_name
                        );
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

            Commands::Version => {
                println!("tokstat {}", env!("CARGO_PKG_VERSION"));
            }
        },
        None => {
            show_token_status(&storage).await?;
        }
    }

    Ok(())
}

async fn show_token_status(storage: &storage::SecureStorage) -> Result<()> {
    let accounts = storage.list_accounts()?;

    if accounts.is_empty() {
        println!("No providers configured. Run 'tokstat login' to add an account.");
        return Ok(());
    }

    println!("\nToken Status\n{}", "═".repeat(60));
    for account in accounts {
        println!("\n• {} ({})", account.name, account.provider);

        match providers::fetch_quota(&account).await {
            Ok(quota) => {
                println!("    {}", describe_requests(&quota));
                println!("    {}", describe_tokens(&quota));
                println!("    {}", describe_cost(&quota));
                println!("    Reset: {}", format_datetime(quota.reset_date));
                println!("    Updated: {}", format_datetime(Some(quota.last_updated)));
            }
            Err(err) => {
                println!("    ⚠️  {}", err);
            }
        }
    }

    Ok(())
}

fn describe_requests(quota: &providers::QuotaInfo) -> String {
    if let Some(requests) = quota.usage.requests_made {
        if let Some(max_requests) = quota.limits.as_ref().and_then(|limits| limits.max_requests) {
            let remaining = max_requests.saturating_sub(requests);
            let percent_used = if max_requests > 0 {
                (requests as f64 / max_requests as f64) * 100.0
            } else {
                0.0
            };
            format!(
                "Requests: {} / {} ({} remaining, {:.1}% used)",
                format_number(requests),
                format_number(max_requests),
                format_number(remaining),
                percent_used
            )
        } else {
            format!("Requests: {}", format_number(requests))
        }
    } else {
        "Requests: N/A".to_string()
    }
}

fn describe_tokens(quota: &providers::QuotaInfo) -> String {
    if let Some(tokens) = quota.usage.tokens_used {
        if let Some(max_tokens) = quota.limits.as_ref().and_then(|limits| limits.max_tokens) {
            let percent_used = if max_tokens > 0 {
                (tokens as f64 / max_tokens as f64) * 100.0
            } else {
                0.0
            };
            format!(
                "Tokens: {} / {} ({:.1}% used)",
                format_number(tokens),
                format_number(max_tokens),
                percent_used
            )
        } else {
            format!("Tokens: {}", format_number(tokens))
        }
    } else {
        "Tokens: N/A".to_string()
    }
}

fn describe_cost(quota: &providers::QuotaInfo) -> String {
    if let Some(cost) = quota.usage.cost {
        if let Some(max_cost) = quota.limits.as_ref().and_then(|limits| limits.max_cost) {
            format!("Cost: ${:.2} / ${:.2}", cost, max_cost)
        } else {
            format!("Cost: ${:.2}", cost)
        }
    } else {
        "Cost: N/A".to_string()
    }
}

fn format_datetime(dt: Option<chrono::DateTime<chrono::Utc>>) -> String {
    dt.map(|value| value.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_else(|| "Unknown".to_string())
}

fn format_number(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

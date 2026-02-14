mod auth;
mod providers;
mod storage;
mod ui;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use colored::*;
use std::io;
use tracing::info;

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
        println!(
            "{} No providers configured. Run {} to add an account.",
            "⚠️".red(),
            "'tokstat login'".cyan().bold()
        );
        return Ok(());
    }

    // Beautiful header
    println!(
        "\n{}",
        "╔══════════════════════════════════════════════════════════════════╗".bright_magenta()
    );
    println!(
        "{}",
        "║                    🚀  TOKEN STATUS DASHBOARD  🚀                ║".bright_magenta()
    );
    println!(
        "{}",
        "╚══════════════════════════════════════════════════════════════════╝".bright_magenta()
    );

    for account in accounts {
        let provider_emoji = match account.provider.as_str() {
            "copilot" => "🤖",
            "openrouter" => "🌐",
            _ => "🔌",
        };

        // Account header with box drawing
        println!(
            "\n{} {}",
            "┌".bright_magenta(),
            "─".repeat(64).bright_magenta()
        );
        println!(
            "{}  {} {} {} ({})",
            "│".bright_magenta(),
            provider_emoji,
            account.name.bold().bright_white(),
            "via".dimmed(),
            account.provider.bright_cyan()
        );
        println!(
            "{} {}",
            "├".bright_magenta(),
            "─".repeat(64).bright_magenta()
        );

        match providers::fetch_quota(&account).await {
            Ok(quota) => {
                // Requests
                if let Some(requests) = quota.usage.requests_made {
                    let requests_info = format_requests_line(&quota, requests);
                    println!("{}  {}", "│".bright_magenta(), requests_info);
                }

                // Tokens
                if let Some(tokens) = quota.usage.tokens_used {
                    let tokens_info = format_tokens_line(&quota, tokens);
                    println!("{}  {}", "│".bright_magenta(), tokens_info);
                }

                // Cost
                if let Some(cost) = quota.usage.cost {
                    let cost_info = format_cost_line(&quota, cost);
                    println!("{}  {}", "│".bright_magenta(), cost_info);
                }

                // Reset date
                let reset_text = format_datetime(quota.reset_date);
                println!(
                    "{}  {} {}",
                    "│".bright_magenta(),
                    "🔄".dimmed(),
                    format!("Reset: {}", reset_text).dimmed()
                );

                // Last updated
                let updated_text = format_datetime(Some(quota.last_updated));
                println!(
                    "{}  {} {}",
                    "│".bright_magenta(),
                    "⏱️ ".dimmed(),
                    format!("Updated: {}", updated_text).dimmed()
                );
            }
            Err(err) => {
                println!(
                    "{}  {} {}",
                    "│".bright_magenta(),
                    "❌".red(),
                    err.to_string().red()
                );
            }
        }

        println!(
            "{} {}",
            "└".bright_magenta(),
            "─".repeat(64).bright_magenta()
        );
    }

    // Footer
    println!(
        "\n{}",
        "💡 Tip: Run 'tokstat dashboard' for an interactive TUI experience".dimmed()
    );

    Ok(())
}

fn format_requests_line(quota: &providers::QuotaInfo, requests: u64) -> String {
    if let Some(max_requests) = quota.limits.as_ref().and_then(|limits| limits.max_requests) {
        let remaining = max_requests.saturating_sub(requests);
        let percent_used = if max_requests > 0 {
            (requests as f64 / max_requests as f64) * 100.0
        } else {
            0.0
        };

        let (icon, color) = get_usage_indicator(percent_used);

        format!(
            "{} {} {} / {}  {} {} ({:.1}%)",
            "📊".to_string(),
            "Requests:".bright_white().bold(),
            format_number(requests).bright_yellow(),
            format_number(max_requests).bright_white(),
            icon,
            format!("{} remaining", format_number(remaining)).color(color),
            percent_used
        )
    } else {
        format!(
            "{} {} {}",
            "📊".to_string(),
            "Requests:".bright_white().bold(),
            format_number(requests).bright_yellow()
        )
    }
}

fn format_tokens_line(quota: &providers::QuotaInfo, tokens: u64) -> String {
    if let Some(max_tokens) = quota.limits.as_ref().and_then(|limits| limits.max_tokens) {
        let percent_used = if max_tokens > 0 {
            (tokens as f64 / max_tokens as f64) * 100.0
        } else {
            0.0
        };

        let (icon, color) = get_usage_indicator(percent_used);

        format!(
            "{} {} {} / {}  {} ({:.1}%)",
            "🔤".to_string(),
            "Tokens:".bright_white().bold(),
            format_number(tokens).bright_yellow(),
            format_number(max_tokens).bright_white(),
            icon.color(color),
            percent_used
        )
    } else {
        format!(
            "{} {} {}",
            "🔤".to_string(),
            "Tokens:".bright_white().bold(),
            format_number(tokens).bright_yellow()
        )
    }
}

fn format_cost_line(quota: &providers::QuotaInfo, cost: f64) -> String {
    if let Some(max_cost) = quota.limits.as_ref().and_then(|limits| limits.max_cost) {
        let percent_used = if max_cost > 0.0 {
            (cost / max_cost) * 100.0
        } else {
            0.0
        };

        let (icon, color) = get_usage_indicator(percent_used);

        format!(
            "{} {} {} / {}  {} ({:.1}%)",
            "💰".to_string(),
            "Cost:".bright_white().bold(),
            format!("${:.2}", cost).bright_yellow(),
            format!("${:.2}", max_cost).bright_white(),
            icon.color(color),
            percent_used
        )
    } else {
        format!(
            "{} {} {}",
            "💰".to_string(),
            "Cost:".bright_white().bold(),
            format!("${:.2}", cost).bright_yellow()
        )
    }
}

fn get_usage_indicator(percent_used: f64) -> (&'static str, &'static str) {
    if percent_used < 50.0 {
        ("✓", "green")
    } else if percent_used < 80.0 {
        ("⚠", "yellow")
    } else {
        ("✗", "red")
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

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

    /// Output data in JSON format (for scripting)
    #[arg(long = "json")]
    json: bool,

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
            show_token_status(&storage, cli.json).await?;
        }
    }

    Ok(())
}

async fn show_token_status(storage: &storage::SecureStorage, json_output: bool) -> Result<()> {
    let accounts = storage.list_accounts()?;

    if accounts.is_empty() {
        if json_output {
            println!("[]");
        } else {
            println!(
                "{} No providers configured. Run {} to add an account.",
                "⚠️".red(),
                "'tokstat login'".cyan().bold()
            );
        }
        return Ok(());
    }

    if json_output {
        // JSON output
        let mut json_accounts = Vec::new();
        for account in accounts {
            let quota_result = providers::fetch_quota(&account).await;
            let account_json = match quota_result {
                Ok(quota) => {
                    serde_json::json!({
                        "name": account.name,
                        "provider": account.provider,
                        "usage": {
                            "requests": quota.usage.requests_made,
                            "tokens": quota.usage.tokens_used,
                            "cost": quota.usage.cost
                        },
                        "limits": quota.limits.as_ref().map(|l| {
                            serde_json::json!({
                                "max_requests": l.max_requests,
                                "max_tokens": l.max_tokens,
                                "max_cost": l.max_cost
                            })
                        }),
                        "reset_date": quota.reset_date.map(|dt| dt.to_rfc3339()),
                        "last_updated": quota.last_updated.to_rfc3339()
                    })
                }
                Err(err) => {
                    serde_json::json!({
                        "name": account.name,
                        "provider": account.provider,
                        "error": err.to_string()
                    })
                }
            };
            json_accounts.push(account_json);
        }
        println!("{}", serde_json::to_string_pretty(&json_accounts)?);
    } else if atty::is(atty::Stream::Stdout) {
        // Fancy CLI output with colors and box drawing (default)
        render_status_fancy_cli(&accounts, storage).await?;
    } else {
        // Plain text fallback when piping
        render_status_text_only(&accounts, storage).await?;
    }

    Ok(())
}

async fn render_status_text_only(
    accounts: &[crate::storage::Account],
    _storage: &storage::SecureStorage,
) -> Result<()> {
    // Fetch all quotas first
    let mut account_data: Vec<(
        &crate::storage::Account,
        anyhow::Result<crate::providers::QuotaInfo>,
    )> = Vec::new();
    for account in accounts {
        let quota_result = crate::providers::fetch_quota(account).await;
        account_data.push((account, quota_result));
    }

    println!("Token Status");
    println!("{}", "=".repeat(60));

    for (account, quota_result) in account_data {
        println!(
            "\n{} ({}) - {}",
            account.name, account.provider, account.name
        );

        match quota_result {
            Ok(quota) => {
                // Requests
                if let Some(requests) = quota.usage.requests_made {
                    if let Some(max_requests) = quota.limits.as_ref().and_then(|l| l.max_requests) {
                        let percent = (requests as f64 / max_requests as f64) * 100.0;
                        println!(
                            "  Requests: {} / {} ({:.1}%)",
                            format_number(requests),
                            format_number(max_requests),
                            percent
                        );
                    } else {
                        println!("  Requests: {}", format_number(requests));
                    }
                }

                // Tokens
                if let Some(tokens) = quota.usage.tokens_used {
                    if let Some(max_tokens) = quota.limits.as_ref().and_then(|l| l.max_tokens) {
                        let percent = (tokens as f64 / max_tokens as f64) * 100.0;
                        println!(
                            "  Tokens: {} / {} ({:.1}%)",
                            format_number(tokens),
                            format_number(max_tokens),
                            percent
                        );
                    } else {
                        println!("  Tokens: {}", format_number(tokens));
                    }
                }

                // Cost
                if let Some(cost) = quota.usage.cost {
                    if let Some(max_cost) = quota.limits.as_ref().and_then(|l| l.max_cost) {
                        let percent = (cost / max_cost) * 100.0;
                        println!("  Cost: ${:.2} / ${:.2} ({:.1}%)", cost, max_cost, percent);
                    } else {
                        println!("  Cost: ${:.2}", cost);
                    }
                }

                println!("  Reset: {}", format_datetime(quota.reset_date));
                println!("  Updated: {}", format_datetime(Some(quota.last_updated)));
            }
            Err(err) => {
                println!("  Error: {}", err);
            }
        }
    }

    Ok(())
}

async fn render_status_fancy_cli(
    accounts: &[crate::storage::Account],
    _storage: &storage::SecureStorage,
) -> Result<()> {
    use colored::*;

    // Fetch all quotas first
    let mut account_data: Vec<(
        &crate::storage::Account,
        anyhow::Result<crate::providers::QuotaInfo>,
    )> = Vec::new();
    for account in accounts {
        let quota_result = crate::providers::fetch_quota(account).await;
        account_data.push((account, quota_result));
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

    for (account, quota_result) in account_data {
        let provider_emoji = match account.provider.as_str() {
            "copilot" => "🤖",
            "openrouter" => "🌐",
            _ => "🔌",
        };

        // Account header with box drawing
        println!(
            "\n{}{}",
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
            "{}{}",
            "├".bright_magenta(),
            "─".repeat(64).bright_magenta()
        );

        match quota_result {
            Ok(quota) => {
                // Requests with visual bar
                if let Some(requests) = quota.usage.requests_made {
                    let requests_info = format_requests_with_bar(&quota, requests);
                    println!("{}  {}", "│".bright_magenta(), requests_info);
                }

                // Tokens with visual bar
                if let Some(tokens) = quota.usage.tokens_used {
                    let tokens_info = format_tokens_with_bar(&quota, tokens);
                    println!("{}  {}", "│".bright_magenta(), tokens_info);
                }

                // Cost with visual bar
                if let Some(cost) = quota.usage.cost {
                    let cost_info = format_cost_with_bar(&quota, cost);
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
            "{}{}",
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

fn format_requests_with_bar(quota: &crate::providers::QuotaInfo, requests: u64) -> String {
    use colored::*;

    if let Some(max_requests) = quota.limits.as_ref().and_then(|limits| limits.max_requests) {
        let remaining = max_requests.saturating_sub(requests);
        let percent_used = if max_requests > 0 {
            (requests as f64 / max_requests as f64) * 100.0
        } else {
            0.0
        };

        let bar_width = 20;
        let filled = (percent_used / 100.0 * bar_width as f64) as usize;
        let empty = bar_width - filled;

        let (color_fn, icon): (fn(&str) -> ColoredString, &str) = if percent_used < 50.0 {
            (|s: &str| s.green(), "✓")
        } else if percent_used < 80.0 {
            (|s: &str| s.yellow(), "⚠")
        } else {
            (|s: &str| s.red(), "✗")
        };

        let bar = format!("{}{}", "█".repeat(filled), "░".repeat(empty));

        format!(
            "{} {} {} / {} {} {} {} ({:.1}%)",
            "📊",
            "Requests:".bright_white().bold(),
            format_number(requests).bright_yellow(),
            format_number(max_requests).bright_white(),
            color_fn(&bar),
            icon,
            format!("{} remaining", format_number(remaining)).normal(),
            percent_used
        )
    } else {
        format!(
            "{} {} {}",
            "📊",
            "Requests:".bright_white().bold(),
            format_number(requests).bright_yellow()
        )
    }
}

fn format_tokens_with_bar(quota: &crate::providers::QuotaInfo, tokens: u64) -> String {
    use colored::*;

    if let Some(max_tokens) = quota.limits.as_ref().and_then(|limits| limits.max_tokens) {
        let percent_used = if max_tokens > 0 {
            (tokens as f64 / max_tokens as f64) * 100.0
        } else {
            0.0
        };

        let bar_width = 20;
        let filled = (percent_used / 100.0 * bar_width as f64) as usize;
        let empty = bar_width - filled;

        let (color_fn, icon): (fn(&str) -> ColoredString, &str) = if percent_used < 50.0 {
            (|s: &str| s.green(), "✓")
        } else if percent_used < 80.0 {
            (|s: &str| s.yellow(), "⚠")
        } else {
            (|s: &str| s.red(), "✗")
        };

        let bar = format!("{}{}", "█".repeat(filled), "░".repeat(empty));

        format!(
            "{} {} {} / {} {} {} ({:.1}%)",
            "🔤",
            "Tokens:".bright_white().bold(),
            format_number(tokens).bright_yellow(),
            format_number(max_tokens).bright_white(),
            color_fn(&bar),
            icon,
            percent_used
        )
    } else {
        format!(
            "{} {} {}",
            "🔤",
            "Tokens:".bright_white().bold(),
            format_number(tokens).bright_yellow()
        )
    }
}

fn format_cost_with_bar(quota: &crate::providers::QuotaInfo, cost: f64) -> String {
    use colored::*;

    if let Some(max_cost) = quota.limits.as_ref().and_then(|limits| limits.max_cost) {
        let percent_used = if max_cost > 0.0 {
            (cost / max_cost) * 100.0
        } else {
            0.0
        };

        let bar_width = 20;
        let filled = (percent_used / 100.0 * bar_width as f64) as usize;
        let empty = bar_width - filled;

        let (color_fn, icon): (fn(&str) -> ColoredString, &str) = if percent_used < 50.0 {
            (|s: &str| s.green(), "✓")
        } else if percent_used < 80.0 {
            (|s: &str| s.yellow(), "⚠")
        } else {
            (|s: &str| s.red(), "✗")
        };

        let bar = format!("{}{}", "█".repeat(filled), "░".repeat(empty));

        format!(
            "{} {} ${:.2} / ${:.2} {} {} ({:.1}%)",
            "💰",
            "Cost:".bright_white().bold(),
            cost,
            max_cost,
            color_fn(&bar),
            icon,
            percent_used
        )
    } else {
        format!("{} {} ${:.2}", "💰", "Cost:".bright_white().bold(), cost)
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

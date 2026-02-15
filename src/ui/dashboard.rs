use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use tokio::time::Duration;

use crate::providers::QuotaInfo;
use crate::storage::{Account, SecureStorage};

const PROVIDERS: &[(&str, &str)] = &[("copilot", "GitHub Copilot"), ("openrouter", "OpenRouter")];

pub async fn run(storage: SecureStorage, accounts: Vec<Account>) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(storage, accounts).await?;
    let res = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    }

    Ok(())
}

enum Mode {
    Viewing,
    Renaming {
        buffer: String,
    },
    CreatingAccount {
        selected_provider: usize,
    },
    CreatingAccountName {
        provider_id: String,
        provider_name: String,
        buffer: String,
    },
    Deleting,
}

struct App {
    #[allow(dead_code)]
    storage: SecureStorage,
    accounts: Vec<Account>,
    quotas: Vec<QuotaInfo>,
    selected_index: usize,
    should_quit: bool,
    status_message: String,
    mode: Mode,
}

impl App {
    async fn new(storage: SecureStorage, accounts: Vec<Account>) -> Result<Self> {
        let mut app = Self {
            storage,
            accounts,
            quotas: Vec::new(),
            selected_index: 0,
            should_quit: false,
            status_message: "Loading...".to_string(),
            mode: Mode::Viewing,
        };

        app.refresh_quotas().await;

        Ok(app)
    }

    async fn refresh_quotas(&mut self) {
        self.status_message = "Refreshing quota information...".to_string();
        self.quotas.clear();
        let mut has_error = false;

        for account in &self.accounts {
            match crate::providers::fetch_quota(account).await {
                Ok(mut quota) => {
                    quota.account_name = account.name.clone();
                    self.quotas.push(quota);
                }
                Err(e) => {
                    self.status_message = format!("Error fetching {}: {}", account.name, e);
                    has_error = true;
                }
            }
        }

        if !has_error {
            self.status_message =
                format!("Last updated: {}", chrono::Local::now().format("%H:%M:%S"));
        }
    }

    fn next(&mut self) {
        if !self.accounts.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.accounts.len();
        }
    }

    fn previous(&mut self) {
        if !self.accounts.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.accounts.len() - 1
            } else {
                self.selected_index - 1
            };
        }
    }
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    let mut last_refresh = std::time::Instant::now();
    let refresh_duration = std::time::Duration::from_secs(60);

    loop {
        terminal.draw(|f| ui(f, app))?;

        // Check for user input
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match &mut app.mode {
                    Mode::Viewing => {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => {
                                app.should_quit = true;
                            }
                            KeyCode::Char('R') => {
                                app.refresh_quotas().await;
                                last_refresh = std::time::Instant::now();
                            }
                            KeyCode::Char('r') => {
                                if let Some(account) = app.accounts.get(app.selected_index) {
                                    app.mode = Mode::Renaming {
                                        buffer: account.name.clone(),
                                    };
                                    app.status_message =
                                        "Renaming mode: press Enter to confirm, Esc to cancel"
                                            .to_string();
                                }
                            }
                            KeyCode::Char('n') => {
                                app.mode = Mode::CreatingAccount {
                                    selected_provider: 0,
                                };
                                app.status_message = "Select provider: ↑↓ to navigate, Enter to select, Esc to cancel".to_string();
                            }
                            KeyCode::Char('d') => {
                                if let Some(account) = app.accounts.get(app.selected_index) {
                                    app.mode = Mode::Deleting;
                                    app.status_message = format!(
                                        "Delete account '{}'? Press Enter to confirm, Esc to cancel",
                                        account.name
                                    );
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                app.next();
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                app.previous();
                            }
                            _ => {}
                        }
                    }
                    Mode::Renaming { buffer } => match key.code {
                        KeyCode::Enter => {
                            if let Some(account) = app.accounts.get(app.selected_index) {
                                let trimmed = buffer.trim();
                                if trimmed.is_empty() {
                                    app.status_message = "Name cannot be empty".to_string();
                                } else if let Err(err) =
                                    app.storage.rename_account(&account.name, trimmed)
                                {
                                    app.status_message = format!("Rename failed: {}", err);
                                } else {
                                    if let Some(target) = app.accounts.get_mut(app.selected_index) {
                                        target.name = trimmed.to_string();
                                    }
                                    app.status_message = "Account renamed".to_string();
                                    app.mode = Mode::Viewing;
                                    app.refresh_quotas().await;
                                    last_refresh = std::time::Instant::now();
                                }
                            }
                        }
                        KeyCode::Esc => {
                            app.mode = Mode::Viewing;
                            app.status_message = "Rename cancelled".to_string();
                        }
                        KeyCode::Backspace => {
                            buffer.pop();
                        }
                        KeyCode::Char(c) => {
                            if !c.is_control() {
                                buffer.push(c);
                            }
                        }
                        _ => {}
                    },
                    Mode::CreatingAccount { selected_provider } => match key.code {
                        KeyCode::Enter => {
                            let (provider_id, provider_name) = PROVIDERS[*selected_provider];
                            // Generate default account name as initial buffer
                            let default_name =
                                format!("{}_{}", provider_id, chrono::Utc::now().timestamp());
                            app.mode = Mode::CreatingAccountName {
                                provider_id: provider_id.to_string(),
                                provider_name: provider_name.to_string(),
                                buffer: default_name,
                            };
                            app.status_message =
                                "Enter account name (optional, press Enter for default):"
                                    .to_string();
                        }
                        KeyCode::Esc => {
                            app.mode = Mode::Viewing;
                            app.status_message = "Account creation cancelled".to_string();
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if *selected_provider < PROVIDERS.len() - 1 {
                                *selected_provider += 1;
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if *selected_provider > 0 {
                                *selected_provider -= 1;
                            }
                        }
                        _ => {}
                    },
                    Mode::CreatingAccountName {
                        provider_id,
                        provider_name,
                        buffer,
                    } => match key.code {
                        KeyCode::Enter => {
                            let account_name = if buffer.trim().is_empty() {
                                format!("{}_{}", provider_id, chrono::Utc::now().timestamp())
                            } else {
                                buffer.trim().to_string()
                            };
                            // Clone the values before changing mode
                            let provider_id = provider_id.clone();
                            let provider_name = provider_name.clone();
                            app.mode = Mode::Viewing;
                            app.status_message = format!("Creating {} account...", provider_name);

                            // Exit terminal UI temporarily to run login flow
                            disable_raw_mode()?;
                            execute!(
                                terminal.backend_mut(),
                                LeaveAlternateScreen,
                                DisableMouseCapture
                            )?;
                            terminal.show_cursor()?;

                            // Run the appropriate login flow
                            let result = match provider_id.as_str() {
                                "copilot" => {
                                    crate::auth::copilot::login(&app.storage, &account_name).await
                                }
                                "openrouter" => {
                                    crate::auth::openrouter::login(&app.storage, &account_name)
                                        .await
                                }
                                _ => unreachable!(),
                            };

                            // Restore terminal UI
                            enable_raw_mode()?;
                            execute!(
                                terminal.backend_mut(),
                                EnterAlternateScreen,
                                EnableMouseCapture
                            )?;

                            match result {
                                Ok(()) => {
                                    // Reload accounts
                                    match app.storage.list_accounts() {
                                        Ok(accounts) => {
                                            app.accounts = accounts;
                                            if !app.accounts.is_empty() {
                                                app.selected_index = app.accounts.len() - 1;
                                            }
                                            app.status_message = format!(
                                                "✓ {} account '{}' added successfully",
                                                provider_name, account_name
                                            );
                                            app.refresh_quotas().await;
                                            last_refresh = std::time::Instant::now();
                                        }
                                        Err(e) => {
                                            app.status_message =
                                                format!("Error reloading accounts: {}", e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    app.status_message = format!("Failed to add account: {}", e);
                                }
                            }
                        }
                        KeyCode::Esc => {
                            app.mode = Mode::Viewing;
                            app.status_message = "Account creation cancelled".to_string();
                        }
                        KeyCode::Backspace => {
                            buffer.pop();
                        }
                        KeyCode::Char(c) => {
                            if !c.is_control() {
                                buffer.push(c);
                            }
                        }
                        _ => {}
                    },
                    Mode::Deleting => match key.code {
                        KeyCode::Enter => {
                            if let Some(account) = app.accounts.get(app.selected_index) {
                                let account_name = account.name.clone();
                                match app.storage.remove_account(&account_name) {
                                    Ok(()) => {
                                        // Reload accounts
                                        match app.storage.list_accounts() {
                                            Ok(accounts) => {
                                                app.accounts = accounts;
                                                if app.accounts.is_empty() {
                                                    app.selected_index = 0;
                                                } else if app.selected_index >= app.accounts.len() {
                                                    app.selected_index = app.accounts.len() - 1;
                                                }
                                                app.quotas.clear();
                                                app.status_message =
                                                    format!("Account '{}' deleted", account_name);
                                                app.refresh_quotas().await;
                                                last_refresh = std::time::Instant::now();
                                            }
                                            Err(e) => {
                                                app.status_message =
                                                    format!("Error reloading accounts: {}", e);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        app.status_message =
                                            format!("Failed to delete account: {}", e);
                                    }
                                }
                            }
                            app.mode = Mode::Viewing;
                        }
                        KeyCode::Esc => {
                            app.mode = Mode::Viewing;
                            app.status_message = "Delete cancelled".to_string();
                        }
                        _ => {}
                    },
                }
            }
        }

        // Auto-refresh every 60 seconds
        if last_refresh.elapsed() >= refresh_duration {
            app.refresh_quotas().await;
            last_refresh = std::time::Instant::now();
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.size());

    // Header with beautiful predefined colors
    let header_text = Line::from(vec![
        Span::styled(
            "tokstat",
            Style::default()
                .fg(Color::LightMagenta)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " - ",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "Token Quota Monitor",
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let header = Paragraph::new(header_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta)),
        );
    f.render_widget(header, chunks[0]);

    // Main content
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[1]);

    // Account list
    render_account_list(f, app, main_chunks[0]);

    // Quota details
    render_quota_details(f, app, main_chunks[1]);

    // Footer
    let footer_text = vec![
        Line::from(vec![
            Span::raw("Press "),
            Span::styled(
                "q",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" to quit, "),
            Span::styled(
                "R",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" to refresh, "),
            Span::styled(
                "r",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" to rename, "),
            Span::styled(
                "n",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" for new, "),
            Span::styled(
                "d",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" to delete, "),
            Span::styled(
                "↑↓",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" to navigate"),
        ]),
        Line::from(app.status_message.as_str()),
    ];

    let footer = Paragraph::new(footer_text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, chunks[2]);

    if let Mode::Renaming { buffer } = &app.mode {
        let area = centered_rect(40, 20, f.size());
        f.render_widget(Clear, area);
        let prompt = Paragraph::new(vec![
            Line::from(Span::styled(
                "Rename Account",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from("Press Enter to confirm, Esc to cancel"),
            Line::from(""),
            Line::from(buffer.as_str()),
        ])
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Rename Account"),
        );
        f.render_widget(prompt, area);
    }

    if let Mode::CreatingAccount { selected_provider } = &app.mode {
        let area = centered_rect(50, 40, f.size());
        f.render_widget(Clear, area);

        let items: Vec<ListItem> = PROVIDERS
            .iter()
            .enumerate()
            .map(|(i, (id, name))| {
                let style = if i == *selected_provider {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Magenta)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let content = format!("{} - {}", id, name);
                ListItem::new(content).style(style)
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Select Provider")
                .title_alignment(Alignment::Center),
        );

        f.render_widget(list, area);
    }

    if let Mode::CreatingAccountName {
        provider_name,
        buffer,
        ..
    } = &app.mode
    {
        let area = centered_rect(50, 25, f.size());
        f.render_widget(Clear, area);

        let prompt = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("Create ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    provider_name,
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(Color::LightMagenta),
                ),
                Span::styled(" Account", Style::default().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from("Enter an optional name (or press Enter for default):"),
            Line::from(buffer.as_str()),
            Line::from(""),
            Line::from(vec![
                Span::styled("Enter", Style::default().fg(Color::Yellow)),
                Span::raw(" to confirm, "),
                Span::styled("Esc", Style::default().fg(Color::Yellow)),
                Span::raw(" to cancel"),
            ]),
        ])
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Account Name")
                .title_alignment(Alignment::Center),
        );
        f.render_widget(prompt, area);
    }

    if let Mode::Deleting = &app.mode {
        let area = centered_rect(50, 30, f.size());
        f.render_widget(Clear, area);

        let account_name = app
            .accounts
            .get(app.selected_index)
            .map(|a| a.name.clone())
            .unwrap_or_default();

        let prompt = Paragraph::new(vec![
            Line::from(Span::styled(
                "Delete Account?",
                Style::default().add_modifier(Modifier::BOLD).fg(Color::Red),
            )),
            Line::from(""),
            Line::from(vec![
                Span::raw("Are you sure you want to delete account "),
                Span::styled(
                    account_name,
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(Color::Yellow),
                ),
                Span::raw("?"),
            ]),
            Line::from(""),
            Line::from("Press Enter to confirm, Esc to cancel"),
        ])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Confirm Delete")
                .title_alignment(Alignment::Center),
        );
        f.render_widget(prompt, area);
    }
}

fn render_account_list(f: &mut Frame, app: &App, area: Rect) {
    if app.accounts.is_empty() {
        let welcome_text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("Welcome to ", Style::default().fg(Color::Gray)),
                Span::styled(
                    "tokstat",
                    Style::default()
                        .fg(Color::LightMagenta)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "No accounts configured yet.",
                Style::default().fg(Color::Gray),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("Press ", Style::default().fg(Color::Gray)),
                Span::styled(
                    "n",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " to add your first account",
                    Style::default().fg(Color::Gray),
                ),
            ]),
        ];

        let welcome = Paragraph::new(welcome_text)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Accounts"));
        f.render_widget(welcome, area);
        return;
    }

    let items: Vec<ListItem> = app
        .accounts
        .iter()
        .enumerate()
        .map(|(i, account)| {
            let style = if i == app.selected_index {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Magenta)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let content = format!("{} ({})", account.name, account.provider);
            ListItem::new(content).style(style)
        })
        .collect();

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Accounts"));

    f.render_widget(list, area);
}

fn render_quota_details(f: &mut Frame, app: &App, area: Rect) {
    if app.accounts.is_empty() {
        // Show getting started guide when no accounts configured
        let guide_text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "Getting Started",
                Style::default()
                    .fg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "tokstat monitors token quotas across multiple AI providers.",
                Style::default().fg(Color::Gray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Supported providers:",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled("  • ", Style::default().fg(Color::Magenta)),
                Span::styled("GitHub Copilot", Style::default()),
                Span::styled(" - AI coding assistant", Style::default().fg(Color::Gray)),
            ]),
            Line::from(vec![
                Span::styled("  • ", Style::default().fg(Color::Magenta)),
                Span::styled("OpenRouter", Style::default()),
                Span::styled(" - LLM API aggregator", Style::default().fg(Color::Gray)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Press ", Style::default().fg(Color::Gray)),
                Span::styled(
                    "n",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " to add an account and get started",
                    Style::default().fg(Color::Gray),
                ),
            ]),
        ];

        let guide = Paragraph::new(guide_text).alignment(Alignment::Left).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Quota Details"),
        );
        f.render_widget(guide, area);
        return;
    }

    if app.quotas.is_empty() {
        // Show error status if available, otherwise show generic message
        let message_text = if app.status_message.starts_with("Error") {
            format!("Failed to fetch quota data\n\n{}", app.status_message)
        } else {
            "No quota data available".to_string()
        };

        let message = Paragraph::new(message_text)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Quota Details"),
            );
        f.render_widget(message, area);
        return;
    }

    if app.selected_index >= app.quotas.len() {
        return;
    }

    let quota = &app.quotas[app.selected_index];
    let account = &app.accounts[app.selected_index];

    let details_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(0)])
        .split(area);

    // Account info
    let info_text = vec![
        Line::from(vec![
            Span::styled("Provider: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(&account.provider),
        ]),
        Line::from(vec![
            Span::styled("Account: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(&account.name),
        ]),
        Line::from(vec![
            Span::styled(
                "Last Updated: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(quota.last_updated.format("%Y-%m-%d %H:%M:%S").to_string()),
        ]),
        Line::from(vec![
            Span::styled(
                "Quota Resets: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(
                quota
                    .reset_date
                    .map(|dt| dt.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| "Unknown".to_string()),
            ),
        ]),
    ];

    let info = Paragraph::new(info_text)
        .block(Block::default().borders(Borders::ALL).title("Account Info"))
        .wrap(Wrap { trim: true });

    f.render_widget(info, details_chunks[0]);

    // Determine which gauges to display and build constraints dynamically
    let has_requests = quota.usage.requests_made.is_some();
    let has_tokens = quota.usage.tokens_used.is_some();
    let has_cost = quota.usage.cost.is_some();
    let gauge_count = [has_requests, has_tokens, has_cost]
        .iter()
        .filter(|&&x| x)
        .count();

    let constraints: Vec<Constraint> = (0..gauge_count)
        .map(|_| Constraint::Length(3))
        .chain(std::iter::once(Constraint::Min(0)))
        .collect();

    // Usage details with beautiful gauges
    let usage_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(details_chunks[1]);

    let mut gauge_index = 0;

    // Requests gauge
    if let Some(requests) = quota.usage.requests_made {
        let (ratio, label, color) =
            if let Some(max_requests) = quota.limits.as_ref().and_then(|l| l.max_requests) {
                let r = requests as f64 / max_requests as f64;
                let remaining = max_requests.saturating_sub(requests);
                let lbl = format!(
                    "Requests: {} / {} ({} remaining)",
                    format_number(requests),
                    format_number(max_requests),
                    format_number(remaining)
                );
                let col = if r < 0.5 {
                    Color::Green
                } else if r < 0.8 {
                    Color::Yellow
                } else {
                    Color::Red
                };
                (r.min(1.0), lbl, col)
            } else {
                (
                    0.0,
                    format!("Requests: {}", format_number(requests)),
                    Color::Gray,
                )
            };

        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .gauge_style(Style::default().fg(color).bg(Color::DarkGray))
            .ratio(ratio)
            .label(label);
        f.render_widget(gauge, usage_chunks[gauge_index]);
        gauge_index += 1;
    }

    // Tokens gauge
    if let Some(tokens) = quota.usage.tokens_used {
        let (ratio, label, color) =
            if let Some(max_tokens) = quota.limits.as_ref().and_then(|l| l.max_tokens) {
                let r = tokens as f64 / max_tokens as f64;
                let lbl = format!(
                    "Tokens: {} / {} ({:.1}%)",
                    format_number(tokens),
                    format_number(max_tokens),
                    r * 100.0
                );
                let col = if r < 0.5 {
                    Color::Green
                } else if r < 0.8 {
                    Color::Yellow
                } else {
                    Color::Red
                };
                (r.min(1.0), lbl, col)
            } else {
                (
                    0.0,
                    format!("Tokens: {}", format_number(tokens)),
                    Color::Gray,
                )
            };

        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .gauge_style(Style::default().fg(color).bg(Color::DarkGray))
            .ratio(ratio)
            .label(label);
        f.render_widget(gauge, usage_chunks[gauge_index]);
        gauge_index += 1;
    }

    // Cost gauge
    if let Some(cost) = quota.usage.cost {
        let (ratio, label, color) =
            if let Some(max_cost) = quota.limits.as_ref().and_then(|l| l.max_cost) {
                let r = cost / max_cost;
                let lbl = format!("Cost: ${:.2} / ${:.2} ({:.1}%)", cost, max_cost, r * 100.0);
                let col = if r < 0.5 {
                    Color::Green
                } else if r < 0.8 {
                    Color::Yellow
                } else {
                    Color::Red
                };
                (r.min(1.0), lbl, col)
            } else {
                (0.0, format!("Cost: ${:.2}", cost), Color::Gray)
            };

        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .gauge_style(Style::default().fg(color).bg(Color::DarkGray))
            .ratio(ratio)
            .label(label);
        f.render_widget(gauge, usage_chunks[gauge_index]);
    }
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

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

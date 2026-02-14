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
    Renaming { buffer: String },
    CreatingAccount { selected_provider: usize },
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

                            // Generate default account name
                            let account_name =
                                format!("{}_{}", provider_id, chrono::Utc::now().timestamp());

                            // Run the appropriate login flow
                            let result = match provider_id {
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

    // Header
    let header = Paragraph::new("tokstat - Token Quota Monitor")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
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
                        .bg(Color::Cyan)
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
}

fn render_account_list(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .accounts
        .iter()
        .enumerate()
        .map(|(i, account)| {
            let style = if i == app.selected_index {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
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
        .constraints([Constraint::Length(6), Constraint::Min(0)])
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
    ];

    let info = Paragraph::new(info_text)
        .block(Block::default().borders(Borders::ALL).title("Account Info"))
        .wrap(Wrap { trim: true });

    f.render_widget(info, details_chunks[0]);

    // Usage details
    let usage_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(details_chunks[1]);

    // Requests made
    if let Some(requests) = quota.usage.requests_made {
        let request_limits = quota.limits.as_ref().and_then(|l| l.max_requests);

        let requests_label = if let Some(max_requests) = request_limits {
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
        };

        let info = Paragraph::new(requests_label)
            .block(Block::default().borders(Borders::ALL).title("Requests"));
        f.render_widget(info, usage_chunks[0]);
    }

    // Tokens used
    if let Some(tokens) = quota.usage.tokens_used {
        let max_tokens = quota
            .limits
            .as_ref()
            .and_then(|l| l.max_tokens)
            .unwrap_or(tokens * 2);

        let ratio = tokens as f64 / max_tokens as f64;
        let label = format!(
            "Tokens: {} / {}",
            format_number(tokens),
            format_number(max_tokens)
        );

        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL))
            .gauge_style(Style::default().fg(get_usage_color(ratio)))
            .ratio(ratio.min(1.0))
            .label(label);

        f.render_widget(gauge, usage_chunks[1]);
    }

    // Cost
    if let Some(cost) = quota.usage.cost {
        let max_cost = quota
            .limits
            .as_ref()
            .and_then(|l| l.max_cost)
            .unwrap_or(cost * 2.0);

        let ratio = cost / max_cost;
        let label = if max_cost > 0.0 {
            format!("Cost: ${:.2} / ${:.2}", cost, max_cost)
        } else {
            format!("Cost: ${:.2}", cost)
        };

        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL))
            .gauge_style(Style::default().fg(get_usage_color(ratio)))
            .ratio(ratio.min(1.0))
            .label(label);

        f.render_widget(gauge, usage_chunks[2]);
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

fn get_usage_color(ratio: f64) -> Color {
    if ratio < 0.5 {
        Color::Green
    } else if ratio < 0.8 {
        Color::Yellow
    } else {
        Color::Red
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

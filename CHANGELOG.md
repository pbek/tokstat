# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-02-14

### Added

- New account creation from dashboard using `n` key
  - Opens provider selector popup with available providers (GitHub Copilot, OpenRouter)
  - Temporarily exits TUI to run the appropriate login flow
  - Automatically reloads accounts and refreshes quota data after successful creation
- Account deletion from dashboard using `d` key
  - Shows confirmation dialog before deleting
  - Press Enter to confirm, Esc to cancel
  - Automatically reloads accounts and quota data after deletion

### Changed

- **Breaking**: Changed keyboard shortcuts in dashboard
  - `R` (uppercase) now refreshes quota information
  - `r` (lowercase) now triggers rename mode
- Fixed alignment of right-side panels in dashboard - requests panel now properly aligned with account info panel
- Added "Requests" title/label to the requests panel for better clarity

## [0.1.0] - 2026-02-13

### Added

- Initial release of tokstat CLI tool for monitoring token quotas across AI providers
- Support for GitHub Copilot provider: OAuth device flow login, token usage tracking
- Support for OpenRouter provider: API key authentication, cost and credit limit monitoring
- Interactive terminal dashboard (TUI) for viewing quota information across all accounts
- CLI commands: login, list, dashboard, remove, refresh, version
- Shell completion support for bash, fish, and zsh
- Secure credential storage using system keyring
- Modular provider architecture for easy addition of new AI service integrations
- Comprehensive error handling and user-friendly output
- Added ability to rename accounts in the TUI dashboard using 'n' key shortcut
- Updated footer in dashboard to show all available shortcuts including the new rename option

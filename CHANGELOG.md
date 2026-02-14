# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-02-14

### Added

- Interactive account management directly from the dashboard
  - **Add new accounts** with `n` key
    - Opens provider selector popup showing available providers (GitHub Copilot, OpenRouter)
    - Navigate providers with ↑↓ arrow keys, Enter to select, Esc to cancel
    - **Optional custom name input**: After selecting provider, enter an optional account name
      - Default name auto-generated as `{provider}_{timestamp}`
      - Type a custom name or press Enter to accept the default
      - Esc cancels the entire account creation flow
    - Temporarily exits TUI to run the appropriate authentication flow
      - GitHub Copilot: OAuth device flow with user code
      - OpenRouter: API key input with validation
    - Automatically reloads accounts list and refreshes quota data after successful creation
    - New account becomes selected after creation
  - **Delete accounts** with `d` key
    - Shows confirmation dialog displaying the account name to be deleted
    - Press Enter to confirm deletion, Esc to cancel
    - Automatically removes account credentials from secure storage
    - Reloads accounts list and adjusts selection index after deletion
    - Clears and refreshes quota data to reflect changes

### Changed

- **Breaking**: Changed keyboard shortcuts in dashboard
  - `R` (uppercase) now refreshes quota information
  - `r` (lowercase) now triggers rename mode
- Fixed alignment of right-side panels in dashboard - requests panel now properly aligned with account info panel
- Added "Requests" title/label to the requests panel for better clarity
- Updated dashboard color scheme to use only predefined colors
  - Header now uses multi-color scheme: LightMagenta (tokstat), Magenta (separator), LightCyan (subtitle)
  - Header border changed to Magenta
  - Selection highlight changed from Cyan to Magenta for consistency
  - Removed all RGB color values for better terminal compatibility
- Completely redesigned the default `tokstat` output (when run without subcommands)
  - Added colorful header with "🚀 TOKEN STATUS DASHBOARD 🚀" banner
  - Each account now displayed in a beautiful box-drawing frame
  - Provider-specific emojis (🤖 for Copilot, 🌐 for OpenRouter, 🔌 for others)
  - Color-coded usage indicators: ✓ green (<50%), ⚠ yellow (50-80%), ✗ red (>80%)
  - Enhanced formatting with bold labels, bright colors, and emojis
  - Added helpful tip at the bottom suggesting the interactive dashboard
- Added `--json` flag for scriptable JSON output
  - Use `tokstat --json` to output account data as structured JSON
  - Perfect for scripting and automation (pipe to `jq`, etc.)
  - Includes all account details: name, provider, usage, limits, timestamps
  - Failed quota fetches include error messages in the JSON
  - Empty accounts list outputs `[]` for easy parsing

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

# tokstat - Project Summary

## Overview

**tokstat** is a beautiful, production-ready CLI application written in Rust for monitoring token quotas across multiple AI providers. The project is located at `~/Code/tokstat` and features secure credential storage, OAuth authentication, a stunning TUI dashboard, and complete NixOS integration.

## Quick Start

```bash
cd ~/Code/tokstat

# Build the application
nix build

# Or with cargo
nix develop
cargo build --release

# Run the binary
./target/release/tokstat --help

# Login to a provider
./target/release/tokstat login copilot --name my-copilot
./target/release/tokstat login openrouter --name my-openrouter

# View dashboard
./target/release/tokstat dashboard

# List accounts
./target/release/tokstat list
```

## Project Structure

```
~/Code/tokstat/
├── src/
│   ├── main.rs                 # CLI entry point (tokstat command)
│   ├── auth/                   # Authentication modules
│   │   ├── copilot.rs          # GitHub OAuth device flow
│   │   └── openrouter.rs       # API key authentication
│   ├── providers/              # Provider implementations
│   │   ├── mod.rs              # Provider trait
│   │   ├── copilot.rs          # Copilot quota fetching
│   │   └── openrouter.rs       # OpenRouter quota fetching
│   ├── storage/                # Secure credential storage
│   │   └── mod.rs              # Keyring integration (tokstat service)
│   └── ui/                     # Terminal UI
│       └── dashboard.rs        # TUI dashboard
├── flake.nix                   # Nix flake configuration
├── flake.lock                  # Locked Nix dependencies
├── Cargo.toml                  # Rust dependencies (name: tokstat)
├── Cargo.lock                  # Locked Rust dependencies
├── README.md                   # User documentation
├── QUICKSTART.md               # Quick start guide
├── ADDING_PROVIDERS.md         # Developer guide for adding providers
├── PROJECT.md                  # Detailed project overview
└── build.sh                    # Universal build script
```

## Key Features Implemented

### ✅ Core Functionality

- [x] Beautiful TUI dashboard with ratatui
- [x] GitHub Copilot OAuth device flow login
- [x] OpenRouter API key authentication
- [x] Secure credential storage via system keyring
- [x] Multi-account management
- [x] Real-time quota monitoring
- [x] Auto-refresh every 60 seconds

### ✅ Security

- [x] Credentials stored in system keyring with service name "tokstat"
- [x] Configuration stored in `~/.config/tokstat/`
- [x] OAuth tokens encrypted at rest
- [x] No credentials in plain text

### ✅ NixOS Integration

- [x] Complete Nix flake with all inputs locked
- [x] Development shell with all dependencies
- [x] NixOS module (programs.tokstat)
- [x] Home Manager module (programs.tokstat)
- [x] Docker image generation
- [x] Cross-platform build support

### ✅ CLI Commands

- [x] `tokstat login <provider> --name <account>`
- [x] `tokstat list` - List all accounts
- [x] `tokstat dashboard` - Interactive TUI
- [x] `tokstat refresh [account]` - Refresh quota data
- [x] `tokstat remove <account>` - Remove account

### ✅ Documentation

- [x] README.md - User-facing documentation
- [x] QUICKSTART.md - Getting started guide
- [x] ADDING_PROVIDERS.md - Developer guide for extensibility
- [x] PROJECT.md - Technical overview
- [x] All references updated from "ai-quota-monitor" to "tokstat"

## Binary Information

- **Binary Name**: `tokstat`
- **Location (dev)**: `target/release/tokstat`
- **Location (nix)**: `result/bin/tokstat`
- **Size**: ~6MB (stripped, optimized)
- **Platform**: Cross-platform (Linux, macOS, Windows)

## Configuration

- **Config Directory**: `~/.config/tokstat/`
- **Config File**: `~/.config/tokstat/accounts.json`
- **Keyring Service**: `tokstat`
- **Keyring Account**: `<account-name>`

## Supported Providers

1. **GitHub Copilot** (`copilot`)
   - Authentication: OAuth 2.0 device flow
   - Quota info: Token usage, request count
   - Auto-refresh: Token expiration handling

2. **OpenRouter** (`openrouter`)
   - Authentication: API key
   - Quota info: Cost tracking, credit limits
   - API: https://openrouter.ai/api/v1/auth/key

## Architecture Highlights

### Pluggable Provider System

The application uses a trait-based provider system that makes it easy to add new AI providers:

```rust
#[async_trait::async_trait]
pub trait Provider: Send + Sync {
    async fn fetch_quota(&self, credentials: &str) -> Result<QuotaInfo>;
    fn provider_name(&self) -> &str;
}
```

### Secure Storage

All credentials are stored using the system keyring (keyring crate) with the service name "tokstat". The configuration file only contains metadata.

### Beautiful TUI

The dashboard is built with ratatui and provides:

- Color-coded usage gauges
- Real-time updates
- Keyboard navigation
- Multi-account support

## Development Workflow

```bash
# Enter development environment
cd ~/Code/tokstat
nix develop

# Run in development mode
cargo run -- dashboard

# Watch mode for development
cargo watch -x run

# Build optimized release
cargo build --release

# Build with Nix
nix build

# Format code
cargo fmt

# Lint
cargo clippy
```

## Testing the Application

```bash
# Check compilation
cargo check

# Build release
cargo build --release

# Test help
./target/release/tokstat --help

# Test login (dry run - will show device code)
./target/release/tokstat login copilot --name test-account

# View accounts
./target/release/tokstat list
```

## Future Enhancements

- [ ] Add Anthropic Claude provider
- [ ] Add OpenAI provider
- [ ] Add Cohere provider
- [ ] Historical usage tracking
- [ ] Usage graphs and charts
- [ ] Export to CSV/JSON
- [ ] Alert notifications
- [ ] Prometheus metrics export

## Notes

- All instances of "ai-quota-monitor" have been renamed to "tokstat"
- All instances of "ai-quota" command have been renamed to "tokstat"
- The keyring service name is "tokstat"
- The config directory is "tokstat"
- The binary name is "tokstat"
- All documentation has been updated accordingly

## Build Verification

The project successfully compiles with:

```
warning: `tokstat` (bin "tokstat") generated 1 warning
    Finished `release` profile [optimized] target(s) in 59.89s
```

Binary location: `~/Code/tokstat/target/release/tokstat`
Binary size: 6.0M

## Success Criteria ✅

- [x] Project renamed from ai-quota-monitor to tokstat
- [x] Project located in ~/Code/tokstat
- [x] Binary name is tokstat
- [x] All references updated in code
- [x] All references updated in docs
- [x] Keyring service name is tokstat
- [x] Config directory is tokstat
- [x] Nix flake works correctly
- [x] Cargo build succeeds
- [x] Application compiles without errors
- [x] Help command shows correct name

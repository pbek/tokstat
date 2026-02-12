# tokstat - Project Overview

## Summary

A beautiful, production-ready CLI application written in Rust for monitoring token quotas across multiple AI providers. Features secure credential storage, OAuth authentication, a stunning TUI dashboard, and complete NixOS integration.

## Key Features

### ✨ Beautiful TUI Dashboard

- Real-time quota monitoring
- Color-coded usage gauges
- Keyboard-driven navigation
- Auto-refresh functionality
- Multi-account view

### 🔒 Security First

- System keyring integration for credential storage
- OAuth 2.0 device flow for GitHub Copilot
- Encrypted token storage
- No credentials in configuration files

### 🔌 Pluggable Architecture

- Easy to add new providers
- Clean trait-based design
- Minimal boilerplate required
- Well-documented extension guide

### 📦 NixOS Native

- Complete Nix flake with flake.lock
- Development shell included
- NixOS and Home Manager modules
- Reproducible builds
- Docker image generation support

### 🚀 Production Ready

- Optimized release builds (6MB binary)
- Proper error handling
- Comprehensive logging
- Cross-platform support

## Supported Providers

1. **GitHub Copilot**
   - OAuth 2.0 device flow login
   - Token usage tracking
   - Automatic token refresh

2. **OpenRouter**
   - API key authentication
   - Cost tracking
   - Credit limit monitoring

## Technology Stack

- **Language**: Rust (Edition 2021)
- **CLI Framework**: clap 4.5 (derive macros)
- **TUI**: ratatui 0.26 + crossterm 0.27
- **Async Runtime**: tokio 1.35
- **HTTP Client**: reqwest 0.12
- **Security**: keyring 2.3, aes-gcm 0.10
- **OAuth**: oauth2 4.4
- **Build System**: Nix Flakes

## Project Structure

````
tokstat/
├── src/
│   ├── main.rs              # CLI entry point & command handling
│   ├── auth/                # Authentication modules
│   │   ├── mod.rs
│   │   ├── copilot.rs       # OAuth device flow
│   │   └── openrouter.rs    # API key auth
│   ├── providers/           # Provider implementations
│   │   ├── mod.rs           # Provider trait
│   │   ├── copilot.rs       # Copilot quota API
│   │   └── openrouter.rs    # OpenRouter quota API
│   ├── storage/             # Secure credential storage
│   │   └── mod.rs           # Keyring integration
│   └── ui/                  # Terminal UI
│       ├── mod.rs
│       └── dashboard.rs     # TUI dashboard implementation
├── flake.nix                # Nix flake configuration
├── flake.lock               # Locked dependencies
├── shell.nix                # Legacy Nix support
├── Cargo.toml               # Rust dependencies
├── Cargo.lock               # Locked Rust dependencies
├── build.sh                 # Universal build script
├── README.md                # User documentation
├── QUICKSTART.md            # Quick start guide
├── ADDING_PROVIDERS.md      # Developer guide
└── LICENSE                  # MIT License

## CLI Commands

```bash
# Authentication
tokstat login <provider> --name <account-name>

# Monitoring
tokstat dashboard           # Interactive TUI
tokstat list               # List accounts
tokstat refresh [name]     # Refresh quota data

# Management
tokstat remove <name>      # Remove account
````

## Build Options

### Nix (Recommended)

```bash
nix build                   # Build with Nix
nix develop                 # Development environment
nix run . -- dashboard      # Run directly
```

### Cargo

```bash
cargo build --release       # Optimized build
cargo run -- dashboard      # Development run
```

## Installation Methods

1. **Direct Run**: `nix run github:user/tokstat`
2. **System Package**: Via NixOS configuration
3. **User Profile**: `nix profile install`
4. **Home Manager**: Via home.nix
5. **Docker**: `nix build .#docker`

## Configuration Storage

- **Config**: `~/.config/tokstat/accounts.json`
- **Credentials**: System keyring (secure)

## Security Model

1. **Credentials**: Never stored in plain text
2. **OAuth Tokens**: Stored in OS keyring
3. **API Keys**: Stored in OS keyring
4. **Metadata**: Only non-sensitive data in config files
5. **Transport**: HTTPS for all API communications

## Performance

- **Binary Size**: ~6MB (stripped, optimized)
- **Startup Time**: <100ms
- **Memory Usage**: ~10MB baseline
- **Build Time**: ~60s (release)

## Extension Points

### Adding New Providers

1. Implement `Provider` trait in `src/providers/`
2. Add authentication in `src/auth/`
3. Register in `main.rs` and `providers/mod.rs`
4. See `ADDING_PROVIDERS.md` for detailed guide

### Customizing UI

- Modify `src/ui/dashboard.rs`
- Ratatui provides full control over TUI
- Easy to add new views and widgets

## Development Workflow

```bash
# Enter dev environment
nix develop

# Watch mode
cargo watch -x run

# Run with logging
RUST_LOG=debug cargo run -- dashboard

# Format code
cargo fmt

# Lint
cargo clippy

# Audit dependencies
cargo audit
```

## Testing

```bash
# Run tests
cargo test

# Test with coverage
cargo ttest --all-features

# Integration tests
cargo test --test '*'
```

## Future Enhancements

- [ ] Add more providers (Anthropic, OpenAI, Cohere, etc.)
- [ ] Historical usage tracking
- [ ] Usage graphs and charts
- [ ] Export to CSV/JSON
- [ ] Alert notifications
- [ ] Web dashboard mode
- [ ] API server mode
- [ ] Prometheus metrics export
- [ ] Rate limit tracking
- [ ] Cost optimization recommendations

## Contributing

1. Fork the repository
2. Create a feature branch
3. Follow the provider template in `ADDING_PROVIDERS.md`
4. Ensure tests pass
5. Submit a pull request

## License

MIT License - See LICENSE file

## Support

- Documentation: README.md, QUICKSTART.md, ADDING_PROVIDERS.md
- Issues: GitHub Issues
- Discussions: GitHub Discussions

## Credits

Built with:

- Rust programming language
- Nix package manager
- ratatui TUI framework
- And many other excellent open source libraries

---

**Status**: Production Ready ✅
**Version**: 0.1.0
**Last Updated**: February 2026

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-02-12

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

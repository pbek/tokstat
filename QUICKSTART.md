# Quick Start Guide

## Prerequisites

- NixOS or Nix package manager installed
- Flakes enabled in your Nix configuration

## Building the Application

### Using Nix (Recommended)

```bash
# Clone the repository
cd tokstat

# Build the application
nix build

# The binary will be at ./result/bin/tokstat
```

### Using Cargo

```bash
# Enter the development shell
nix develop

# Build with cargo
cargo build --release

# Binary will be at ./target/release/tokstat
```

## Quick Start

### 1. Add Your First Provider

#### GitHub Copilot

```bash
nix run . -- login copilot --name my-copilot
```

This will:

1. Display a device code and verification URL
2. Open your browser and enter the code at https://github.com/login/device
3. Authorize the application
4. Store your credentials securely

#### OpenRouter

```bash
nix run . -- login openrouter --name my-openrouter
```

This will:

1. Prompt for your OpenRouter API key
2. Validate the key
3. Store it securely in your system keyring

### 2. View Your Quota Dashboard

```bash
nix run . -- dashboard
```

This will display a beautiful TUI showing:

- All configured accounts
- Real-time token usage
- Visual progress bars
- Auto-refresh every 60 seconds

### 3. List Your Accounts

```bash
nix run . -- list
```

### 4. Manually Refresh Quota Data

```bash
# Refresh all accounts
nix run . -- refresh

# Refresh specific account
nix run . -- refresh my-copilot
```

### 5. Remove an Account

```bash
nix run . -- remove my-copilot
```

## Dashboard Controls

- `↑`/`↓` or `j`/`k` - Navigate between accounts
- `r` - Manually refresh quota data
- `q` or `Esc` - Quit

## Installation Options

### System-Wide Installation

Add to your NixOS configuration:

```nix
# flake.nix
{
  inputs.tokstat.url = "path:/home/omega/Code/tokstat";

  # In your configuration:
  environment.systemPackages = [
    inputs.tokstat.packages.${system}.default
  ];
}
```

### User Profile Installation

```bash
nix profile install .#
```

### Home Manager Integration

```nix
# In your home.nix
{
  inputs.tokstat.url = "path:/home/omega/Code/tokstat";

  # In your home configuration:
  home.packages = [
    inputs.tokstat.packages.${system}.default
  ];
}
```

## Troubleshooting

### Keyring Issues on Linux

If you encounter keyring errors, make sure you have a keyring service running:

```bash
# For GNOME
gnome-keyring-daemon --start

# For KDE
# The keyring is built into KWallet
```

### Permission Denied

Make sure the binary is executable:

```bash
chmod +x ./result/bin/tokstat
```

### Copilot Authentication Failed

1. Make sure you have access to GitHub Copilot
2. Try removing and re-adding your account
3. Check that the device code hasn't expired (expires in ~15 minutes)

## Development

### Run in Development Mode

```bash
nix develop
cargo run -- dashboard
```

### Watch Mode

```bash
nix develop
cargo watch -x run
```

### Run Tests

```bash
nix develop
cargo test
```

## Configuration Location

- **Linux**: `~/.config/tokstat/accounts.json`
- **Credentials**: Stored securely in system keyring

## Security Notes

- Credentials are NEVER stored in plain text
- OAuth tokens are stored in the system keyring
- API keys are stored in the system keyring
- Configuration files only contain metadata (no secrets)

## Next Steps

- Add more provider accounts
- Set up auto-refresh in your shell profile
- Integrate with your monitoring system
- Check quota before running expensive operations

# Installation Guide for tokstat

## Quick Install

### Option 1: Using Nix (Recommended)

```bash
cd ~/Code/tokstat
nix build
sudo ln -s $(pwd)/result/bin/tokstat /usr/local/bin/tokstat
```

### Option 2: Using Cargo

```bash
cd ~/Code/tokstat
nix develop
cargo build --release
sudo ln -s $(pwd)/target/release/tokstat /usr/local/bin/tokstat
```

### Option 3: Install to User Profile

```bash
cd ~/Code/tokstat
nix profile install .
```

## Verify Installation

```bash
tokstat --version
tokstat --help
```

## First Use

```bash
# Add your first provider
tokstat login copilot --name my-copilot

# Or add OpenRouter
tokstat login openrouter --name my-openrouter

# View dashboard
tokstat dashboard
```

## Uninstall

```bash
# If symlinked
sudo rm /usr/local/bin/tokstat

# If installed via nix profile
nix profile remove tokstat

# Clean up config
rm -rf ~/.config/tokstat
```

## Development

```bash
cd ~/Code/tokstat
nix develop
cargo run -- dashboard
```

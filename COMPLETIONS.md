# Shell Completions for tokstat

tokstat includes built-in support for shell completions across multiple shells.

## Automatic Installation (via Nix)

When installing via Nix, completions are automatically installed to the appropriate locations:

```bash
nix build
# Completions are included in the build output
```

Or when installing to your profile:

```bash
nix profile install .
# Completions are automatically available
```

## Manual Installation

### Bash

```bash
# Generate and install
tokstat --generate bash > ~/.local/share/bash-completion/completions/tokstat

# Or for system-wide
sudo tokstat --generate bash > /usr/share/bash-completion/completions/tokstat

# Then source it
source ~/.local/share/bash-completion/completions/tokstat
```

### Zsh

```bash
# Generate and install to your fpath
tokstat --generate zsh > ~/.zsh/completions/_tokstat

# Make sure this directory is in your fpath (add to ~/.zshrc if needed)
fpath=(~/.zsh/completions $fpath)
compinit
```

### Fish

```bash
# Generate and install
tokstat --generate fish > ~/.config/fish/completions/tokstat.fish

# Completions will be automatically loaded
```

### PowerShell

```powershell
# Generate and add to profile
tokstat --generate powershell | Out-File -Append -FilePath $PROFILE
```

### Elvish

```bash
# Generate and install
tokstat --generate elvish > ~/.elvish/lib/completions/tokstat.elv

# Source in rc.elv
use completions/tokstat
```

## Usage

Once installed, you can use tab completion with tokstat commands:

```bash
tokstat <TAB>            # Shows available commands
tokstat login <TAB>      # Shows available providers
tokstat remove <TAB>     # Shows configured accounts
```

## Supported Shells

- Bash
- Zsh
- Fish
- PowerShell
- Elvish

## Testing Completions

To test if completions are working:

```bash
# Type this and press TAB
tokstat <TAB>

# You should see:
# dashboard  list  login  refresh  remove
```

## Troubleshooting

### Bash completions not working

Make sure bash-completion package is installed:

```bash
# On NixOS
nix-shell -p bash-completion

# On Ubuntu/Debian
sudo apt install bash-completion

# On macOS
brew install bash-completion
```

### Zsh completions not working

Make sure you've run `compinit` in your `.zshrc`:

```bash
# Add to ~/.zshrc
autoload -U compinit
compinit
```

### Fish completions not working

Make sure the completions directory exists:

```bash
mkdir -p ~/.config/fish/completions
```

## Development

To regenerate completions during development:

```bash
cargo build --release
./target/release/tokstat --generate bash > completions/tokstat.bash
./target/release/tokstat --generate zsh > completions/_tokstat
./target/release/tokstat --generate fish > completions/tokstat.fish
```

## NixOS Integration

The Nix flake automatically handles completion installation. When you install tokstat via:

- `nix build` - Completions are in `result/share/bash-completion/completions/`
- `nix profile install` - Completions are automatically integrated
- NixOS module - Completions are system-wide

The postInstall phase in flake.nix uses `installShellFiles` to properly install completions for all supported shells.

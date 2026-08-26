# Installation Guide

This guide will help you install and set up autocomplete-rs on your system.

## System Requirements

### Supported Platforms

- **macOS** 11.0 (Big Sur) or later
- **Linux** (Ubuntu 20.04+, Fedora 35+, Arch, etc.)
- **Windows** via WSL2 (planned for Phase 4)

### Supported Shells

- **Zsh** 5.8+ (Phase 1 - available now)
- **Bash** 4.0+ (Phase 4 - coming soon)
- **Fish** 3.0+ (Phase 4 - coming soon)

### Supported Terminals

Works with all modern terminals:

- iTerm2
- Alacritty
- Kitty
- WezTerm
- Ghostty
- Terminal.app
- GNOME Terminal
- Konsole
- And more!

## Installation Methods

### Method 1: Install from Crates.io (Recommended)

**Note:** Not yet published. Will be available once the first release ships.

```sh
cargo install autocomplete-rs
```

This will:

- Download and compile the latest release
- Install binary to `~/.cargo/bin/`
- Make `autocomplete-rs` available in your PATH

### Method 2: Install from Source (Current)

For early adopters and contributors:

#### Step 1: Install Rust

If you don't have Rust installed:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Verify installation:

```sh
rustc --version  # Should meet the rust-version in Cargo.toml
```

#### Step 2: Clone Repository

```sh
git clone https://github.com/jbabin91/autocomplete-rs.git
cd autocomplete-rs
```

#### Step 3: Build Release Binary

```sh
cargo build --release
```

This creates `target/release/autocomplete-rs` (~5-10MB)

#### Step 4: Install Binary (Optional)

```sh
# Option A: Copy to ~/.cargo/bin (recommended)
cp target/release/autocomplete-rs ~/.cargo/bin/

# Option B: Copy to /usr/local/bin (system-wide)
sudo cp target/release/autocomplete-rs /usr/local/bin/

# Option C: Add to PATH
export PATH="$PWD/target/release:$PATH"
```

Verify installation:

```sh
autocomplete-rs --version
```

### Method 3: Pre-built Binaries

**Note:** Available once the first release ships. cargo-dist builds binaries
for each release.

Download from
[GitHub Releases](https://github.com/jbabin91/autocomplete-rs/releases):

```sh
# cargo-dist provides a shell installer
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/jbabin91/autocomplete-rs/releases/latest/download/autocomplete-rs-installer.sh | sh
```

Or use `cargo binstall` (downloads pre-built binary instead of compiling):

```sh
cargo binstall autocomplete-rs
```

### Method 4: Homebrew (macOS/Linux)

**Note:** Available once the first release ships and the
[homebrew tap](https://github.com/jbabin91/homebrew-tap) is created.

```sh
brew install jbabin91/tap/autocomplete-rs
```

## Shell Integration

After installing the binary, you need to integrate with your shell.

### Zsh Integration

#### Automatic Installation

The easiest way:

```sh
autocomplete-rs install zsh
```

This will:

1. Create `~/.config/autocomplete-rs/` directory
2. Add integration to `~/.zshrc`
3. Set default key binding (Alt+Space)

Restart your shell or run:

```sh
source ~/.zshrc
```

#### Manual Installation

If you prefer manual setup:

**Step 1:** Download integration script

```sh
mkdir -p ~/.config/autocomplete-rs
curl -o ~/.config/autocomplete-rs/zsh.zsh \
  https://raw.githubusercontent.com/jbabin91/autocomplete-rs/main/shell-integration/zsh.zsh
```

Or copy from source:

```sh
mkdir -p ~/.config/autocomplete-rs
cp shell-integration/zsh.zsh ~/.config/autocomplete-rs/
```

**Step 2:** Add to `~/.zshrc`

```sh
# Load autocomplete-rs
if [ -f ~/.config/autocomplete-rs/zsh.zsh ]; then
  source ~/.config/autocomplete-rs/zsh.zsh
fi
```

**Step 3:** Reload shell

```sh
exec zsh
# or
source ~/.zshrc
```

#### Verify Integration

Test the installation:

1. Type a command: `git checkout`
2. Press **Alt+Space** (or your configured key binding)
3. You should see completions appear (once specs are implemented)

Check that the widget is loaded:

```sh
bindkey | grep autocomplete
# Should show: "^[ " _autocomplete_rs_widget
```

### Bash Integration (Coming in Phase 4)

Not yet implemented. Will use readline's `bind -x`:

```sh
autocomplete-rs install bash
```

### Fish Integration (Coming in Phase 4)

Not yet implemented. Will use fish's completion system:

```sh
autocomplete-rs install fish
```

## Starting the Daemon

The daemon starts automatically when you first trigger a completion.

### Manual Daemon Start

To start the daemon manually:

```sh
autocomplete-rs daemon &
```

This is useful for:

- Debugging
- Pre-warming the daemon
- Custom socket paths

### Check Daemon Status

```sh
# Check if daemon is running
ps aux | grep autocomplete-rs

# Check if socket exists
ls -la ~/.autocomplete-rs/daemon.sock

# Test daemon connection
echo '{"buffer":"git checkout ","cursor":13}' | nc -U ~/.autocomplete-rs/daemon.sock
```

### Stop the Daemon

```sh
# Kill daemon
pkill autocomplete-rs

# Or remove socket (daemon will exit)
rm ~/.autocomplete-rs/daemon.sock
```

The daemon will auto-restart on next completion request.

## Customization

### Change Key Binding

Edit `~/.config/autocomplete-rs/zsh.zsh` or add to `~/.zshrc`:

```sh
# Use Ctrl+Space instead of Alt+Space
bindkey '^@' _autocomplete_rs_widget

# Use Tab (replaces default completion)
bindkey '^I' _autocomplete_rs_widget
```

Common key codes:

- `^I` - Tab
- `^@` - Ctrl+Space
- `^[` - Alt+Space (default)
- `^[[` - Escape

### Custom Socket Path

Start daemon with custom path:

```sh
autocomplete-rs daemon --socket ~/.cache/my-autocomplete.sock &
```

Update shell integration to use same path:

```sh
# In zsh.zsh or .zshrc
export AUTOCOMPLETE_RS_SOCKET="$HOME/.cache/my-autocomplete.sock"
```

## Uninstallation

### Remove Binary

```sh
# If installed via cargo
rm ~/.cargo/bin/autocomplete-rs

# If installed to /usr/local/bin
sudo rm /usr/local/bin/autocomplete-rs
```

### Remove Shell Integration

**Zsh:**

Remove from `~/.zshrc`:

```sh
# Delete these lines:
if [ -f ~/.config/autocomplete-rs/zsh.zsh ]; then
  source ~/.config/autocomplete-rs/zsh.zsh
fi
```

**Remove config directory:**

```sh
rm -rf ~/.config/autocomplete-rs
```

**Remove cache:**

```sh
rm -rf ~/.cache/autocomplete-rs
```

**Stop daemon:**

```sh
pkill autocomplete-rs
rm ~/.autocomplete-rs/daemon.sock
```

**Reload shell:**

```sh
exec zsh
```

## Upgrading

### From Crates.io

```sh
cargo install autocomplete-rs --force
```

### From Source

```sh
cd autocomplete-rs
git pull origin main
cargo build --release
cp target/release/autocomplete-rs ~/.cargo/bin/
```

Restart daemon to use new version:

```sh
pkill autocomplete-rs
# Will auto-restart on next completion
```

## Verification

After installation, verify everything works:

### 1. Check Binary

```sh
autocomplete-rs --version
# Should show version number
```

### 2. Check Daemon

```sh
# Start daemon
autocomplete-rs daemon &

# Wait a moment
sleep 1

# Check it's running
ps aux | grep autocomplete-rs

# Check socket exists
ls -la ~/.autocomplete-rs/daemon.sock
```

### 3. Test Completion (Manual)

```sh
# Send test request
autocomplete-rs complete "git checkout " 13

# Should return JSON with suggestions
```

### 4. Test Shell Integration

In your terminal:

1. Type: `git checkout`
2. Press: Alt+Space
3. Expect: Completion UI appears (once specs implemented)

If nothing appears, check [Troubleshooting](troubleshooting.md).

## Next Steps

- Check [Troubleshooting](troubleshooting.md) if you encounter issues
- See [GitHub Issues](https://github.com/jbabin91/autocomplete-rs/issues)
  for known issues

## Getting Help

- **Installation Issues:** Check [Troubleshooting](troubleshooting.md)
- **Bug Reports:** File on
  [GitHub Issues](https://github.com/jbabin91/autocomplete-rs/issues)
- **Questions:**
  [GitHub Issues](https://github.com/jbabin91/autocomplete-rs/issues)

Welcome to autocomplete-rs! 🚀

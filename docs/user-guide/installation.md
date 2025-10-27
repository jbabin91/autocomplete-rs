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

**Note:** Not yet available. Project is in early development (pre-release).

Once released:

```bash
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

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Verify installation:

```bash
rustc --version  # Should be 1.85.0 or later
```

#### Step 2: Clone Repository

```bash
git clone https://github.com/YOUR_USERNAME/autocomplete-rs.git
cd autocomplete-rs
```

#### Step 3: Build Release Binary

```bash
cargo build --release
```

This creates `target/release/autocomplete-rs` (~5-10MB)

#### Step 4: Install Binary

```bash
# Option A: Copy to ~/.cargo/bin (recommended)
cp target/release/autocomplete-rs ~/.cargo/bin/

# Option B: Copy to /usr/local/bin (system-wide)
sudo cp target/release/autocomplete-rs /usr/local/bin/

# Option C: Add to PATH
export PATH="$PWD/target/release:$PATH"
```

Verify installation:

```bash
autocomplete-rs --version
```

### Method 3: Pre-built Binaries (Future)

**Note:** Not yet available.

Once released, download from GitHub releases:

```bash
# macOS (Intel)
curl -L https://github.com/YOUR_USERNAME/autocomplete-rs/releases/latest/download/autocomplete-rs-x86_64-apple-darwin.tar.gz | tar xz

# macOS (Apple Silicon)
curl -L https://github.com/YOUR_USERNAME/autocomplete-rs/releases/latest/download/autocomplete-rs-aarch64-apple-darwin.tar.gz | tar xz

# Linux (x86_64)
curl -L https://github.com/YOUR_USERNAME/autocomplete-rs/releases/latest/download/autocomplete-rs-x86_64-unknown-linux-gnu.tar.gz | tar xz

# Install
sudo mv autocomplete-rs /usr/local/bin/
```

### Method 4: Package Managers (Future)

**Homebrew (macOS/Linux):**

```bash
brew install autocomplete-rs
```

**AUR (Arch Linux):**

```bash
yay -S autocomplete-rs
```

## Shell Integration

After installing the binary, you need to integrate with your shell.

### Zsh Integration

#### Automatic Installation

The easiest way:

```bash
autocomplete-rs install zsh
```

This will:

1. Create `~/.config/autocomplete-rs/` directory
2. Add integration to `~/.zshrc`
3. Set default key binding (Alt+Space)

Restart your shell or run:

```bash
source ~/.zshrc
```

#### Manual Installation

If you prefer manual setup:

**Step 1:** Download integration script

```bash
mkdir -p ~/.config/autocomplete-rs
curl -o ~/.config/autocomplete-rs/zsh.zsh \
  https://raw.githubusercontent.com/YOUR_USERNAME/autocomplete-rs/main/shell-integration/zsh.zsh
```

Or copy from source:

```bash
mkdir -p ~/.config/autocomplete-rs
cp shell-integration/zsh.zsh ~/.config/autocomplete-rs/
```

**Step 2:** Add to `~/.zshrc`

```bash
# Load autocomplete-rs
if [ -f ~/.config/autocomplete-rs/zsh.zsh ]; then
  source ~/.config/autocomplete-rs/zsh.zsh
fi
```

**Step 3:** Reload shell

```bash
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

```bash
bindkey | grep autocomplete
# Should show: "^[ " _autocomplete_rs_widget
```

### Bash Integration (Coming in Phase 4)

Not yet implemented. Will use readline's `bind -x`:

```bash
autocomplete-rs install bash
```

### Fish Integration (Coming in Phase 4)

Not yet implemented. Will use fish's completion system:

```bash
autocomplete-rs install fish
```

## Starting the Daemon

The daemon starts automatically when you first trigger a completion.

### Manual Daemon Start

To start the daemon manually:

```bash
autocomplete-rs daemon /tmp/autocomplete-rs.sock &
```

This is useful for:

- Debugging
- Pre-warming the daemon
- Custom socket paths

### Check Daemon Status

```bash
# Check if daemon is running
ps aux | grep autocomplete-rs

# Check if socket exists
ls -la /tmp/autocomplete-rs.sock

# Test daemon connection
echo '{"buffer":"git checkout ","cursor":13}' | nc -U /tmp/autocomplete-rs.sock
```

### Stop the Daemon

```bash
# Kill daemon
pkill autocomplete-rs

# Or remove socket (daemon will exit)
rm /tmp/autocomplete-rs.sock
```

The daemon will auto-restart on next completion request.

## Configuration

Create config file (optional):

```bash
mkdir -p ~/.config/autocomplete-rs
cat > ~/.config/autocomplete-rs/config.toml << 'EOF'
# Socket path (default: /tmp/autocomplete-rs.sock)
socket_path = "/tmp/autocomplete-rs.sock"

# Theme (mocha, macchiato, frappe, latte)
# Note: Themes coming in Phase 3
theme = "mocha"

# Maximum suggestions to show
max_suggestions = 10

# Key binding (Alt+Space by default)
keybinding = "\\e "  # Alt+Space
EOF
```

**Note:** Configuration system coming in Phase 3.

## Customization

### Change Key Binding

Edit `~/.config/autocomplete-rs/zsh.zsh` or add to `~/.zshrc`:

```bash
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

```bash
autocomplete-rs daemon ~/.cache/my-autocomplete.sock &
```

Update shell integration to use same path:

```bash
# In zsh.zsh or .zshrc
export AUTOCOMPLETE_RS_SOCKET="$HOME/.cache/my-autocomplete.sock"
```

## Uninstallation

### Remove Binary

```bash
# If installed via cargo
rm ~/.cargo/bin/autocomplete-rs

# If installed to /usr/local/bin
sudo rm /usr/local/bin/autocomplete-rs
```

### Remove Shell Integration

**Zsh:**

Remove from `~/.zshrc`:

```bash
# Delete these lines:
if [ -f ~/.config/autocomplete-rs/zsh.zsh ]; then
  source ~/.config/autocomplete-rs/zsh.zsh
fi
```

**Remove config directory:**

```bash
rm -rf ~/.config/autocomplete-rs
```

**Remove cache:**

```bash
rm -rf ~/.cache/autocomplete-rs
```

**Stop daemon:**

```bash
pkill autocomplete-rs
rm /tmp/autocomplete-rs.sock
```

**Reload shell:**

```bash
exec zsh
```

## Upgrading

### From Crates.io (Future)

```bash
cargo install autocomplete-rs --force
```

### From Source

```bash
cd autocomplete-rs
git pull origin main
cargo build --release
cp target/release/autocomplete-rs ~/.cargo/bin/
```

Restart daemon to use new version:

```bash
pkill autocomplete-rs
# Will auto-restart on next completion
```

## Verification

After installation, verify everything works:

### 1. Check Binary

```bash
autocomplete-rs --version
# Should show version number
```

### 2. Check Daemon

```bash
# Start daemon
autocomplete-rs daemon /tmp/autocomplete-rs.sock &

# Wait a moment
sleep 1

# Check it's running
ps aux | grep autocomplete-rs

# Check socket exists
ls -la /tmp/autocomplete-rs.sock
```

### 3. Test Completion (Manual)

```bash
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

- Read [Configuration Guide](configuration.md) to customize behavior
- Check [Troubleshooting](troubleshooting.md) if you encounter issues
- See [GitHub Issues](https://github.com/YOUR_USERNAME/autocomplete-rs/issues)
  for known issues

## Getting Help

- **Installation Issues:** Check [Troubleshooting](troubleshooting.md)
- **Bug Reports:** File on
  [GitHub Issues](https://github.com/YOUR_USERNAME/autocomplete-rs/issues)
- **Questions:** Ask on
  [GitHub Discussions](https://github.com/YOUR_USERNAME/autocomplete-rs/discussions)

Welcome to autocomplete-rs! 🚀

# Configuration Guide

This guide covers how to customize autocomplete-rs to fit your workflow.

**Note:** Full configuration system coming in Phase 3. Basic customization
available now.

## Configuration File

### Location

```sh
~/.config/autocomplete-rs/config.toml
```

### Creating Config File

```bash
mkdir -p ~/.config/autocomplete-rs
cat > ~/.config/autocomplete-rs/config.toml << 'EOF'
# autocomplete-rs configuration
# See: https://github.com/YOUR_USERNAME/autocomplete-rs/blob/main/docs/user-guide/configuration.md

[general]
# Socket path for daemon communication
socket_path = "/tmp/autocomplete-rs.sock"

# Maximum number of suggestions to display
max_suggestions = 10

# Enable debug logging
debug = false

[theme]
# Theme name: mocha, macchiato, frappe, latte
# (Phase 3 - coming soon)
name = "mocha"

# Custom colors (optional)
# [theme.custom]
# border = "#b4befe"
# selected_bg = "#89b4fa"
# selected_fg = "#1e1e2e"
# text = "#cdd6f4"
# description = "#bac2de"

[keybindings]
# Trigger completion (shell-specific format)
# zsh: "\\e " = Alt+Space, "^I" = Tab, "^@" = Ctrl+Space
trigger = "\\e "

[performance]
# Daemon startup timeout (milliseconds)
daemon_timeout = 1000

# Request timeout (milliseconds)
request_timeout = 100

[ui]
# Show descriptions in dropdown
show_descriptions = true

# Show suggestion types (flag, argument, command)
show_types = true

# Maximum dropdown height (lines)
max_height = 15

# Dropdown width mode: "auto", "fixed", or number
width = "auto"
EOF
```

## General Settings

### Socket Path

Where the daemon listens for requests:

```toml
[general]
socket_path = "/tmp/autocomplete-rs.sock"
```

**Options:**

- `/tmp/autocomplete-rs.sock` (default)
- `~/.cache/autocomplete-rs/daemon.sock` (user-specific)
- Custom path

**When to change:**

- Multiple users on same system
- Different instances for testing
- Custom temp directory

### Max Suggestions

Limit how many completions to show:

```toml
[general]
max_suggestions = 10
```

**Default:** 10 **Range:** 1-100 **Recommended:** 10-20 for best UX

More suggestions = more scrolling required

### Debug Mode

Enable verbose logging:

```toml
[general]
debug = true
```

**Output location:** stderr

View logs:

```bash
autocomplete-rs daemon /tmp/autocomplete-rs.sock 2> /tmp/autocomplete-debug.log &
```

## Theme Configuration (Phase 3)

### Built-in Themes

**Catppuccin Mocha** (default - dark):

```toml
[theme]
name = "mocha"
```

**Catppuccin Macchiato** (dark):

```toml
[theme]
name = "macchiato"
```

**Catppuccin Frappe** (dark):

```toml
[theme]
name = "frappe"
```

**Catppuccin Latte** (light):

```toml
[theme]
name = "latte"
```

### Custom Theme Colors

Override specific colors:

```toml
[theme]
name = "mocha"

[theme.custom]
# Border color (hex or rgb)
border = "#b4befe"

# Selected item background
selected_bg = "#89b4fa"

# Selected item text
selected_fg = "#1e1e2e"

# Normal text
text = "#cdd6f4"

# Description text
description = "#bac2de"

# Error text
error = "#f38ba8"

# Warning text
warning = "#f9e2af"
```

**Color formats:**

- Hex: `"#rrggbb"`
- RGB: `"rgb(255, 255, 255)"`
- Named (limited): `"red"`, `"blue"`, etc.

### Terminal Compatibility

autocomplete-rs auto-detects terminal capabilities:

- **Truecolor (24-bit):** Full RGB colors
- **256 color:** Maps RGB to closest 256 color
- **16 color:** Fallback to basic ANSI colors

Check your terminal's capability:

```bash
echo $COLORTERM  # Should show "truecolor" for full color support
```

## Key Bindings

### Trigger Key

Change the key that triggers autocomplete:

```toml
[keybindings]
trigger = "\\e " # Alt+Space (default)
```

**Common options:**

| Key        | Code     | Notes                           |
| ---------- | -------- | ------------------------------- |
| Alt+Space  | `"\\e "` | Default, doesn't conflict       |
| Ctrl+Space | `"^@"`   | May conflict with terminal      |
| Tab        | `"^I"`   | Replaces default tab completion |
| Ctrl+K     | `"^K"`   | Custom binding                  |

**Warning:** Using Tab (`^I`) disables zsh's default completion system.

### Shell-Specific Binding

The trigger key is configured in the shell integration:

**Zsh (`~/.zshrc` or `~/.config/autocomplete-rs/zsh.zsh`):**

```bash
bindkey '^[ ' _autocomplete_rs_widget  # Alt+Space
```

**Bash (coming in Phase 4):**

```bash
bind -x '"\e ": autocomplete_rs_complete'
```

**Fish (coming in Phase 4):**

```fish
bind \e\  autocomplete_rs_complete
```

## UI Customization

### Show Descriptions

Toggle description text:

```toml
[ui]
show_descriptions = true
```

**With descriptions:**

```text
┌─ Completions ───────────────────┐
│ → checkout  Switch branches     │
│   commit    Record changes      │
│   push      Upload to remote    │
└─────────────────────────────────┘
```

**Without descriptions:**

```text
┌─ Completions ────┐
│ → checkout       │
│   commit         │
│   push           │
└──────────────────┘
```

### Show Types

Display suggestion types (flag, argument, subcommand):

```toml
[ui]
show_types = true
```

**With types:**

```text
┌─ Completions ────────────────────────┐
│ → checkout    [cmd] Switch branches  │
│   -b          [opt] Create new       │
│   main        [arg] Branch name      │
└──────────────────────────────────────┘
```

**Without types:**

```text
┌─ Completions ────────────────────┐
│ → checkout    Switch branches    │
│   -b          Create new         │
│   main        Branch name        │
└──────────────────────────────────┘
```

### Dropdown Dimensions

Control dropdown size:

```toml
[ui]
# Maximum height in lines
max_height = 15

# Width: "auto", "fixed", or number of columns
width = "auto"

# Minimum width (columns)
min_width = 40

# Maximum width (columns, or "terminal" for full width)
max_width = 80
```

**Width modes:**

- `"auto"` - Fit content, respect min/max
- `"fixed"` - Always use max_width
- `60` - Specific column count

## Performance Tuning

### Daemon Timeout

How long to wait for daemon startup:

```toml
[performance]
daemon_timeout = 1000 # milliseconds
```

**Default:** 1000ms (1 second) **Range:** 100-5000ms

If daemon doesn't respond within timeout:

- Error message shown
- Graceful degradation (no completions)

### Request Timeout

Maximum time to wait for completion response:

```toml
[performance]
request_timeout = 100 # milliseconds
```

**Default:** 100ms **Range:** 50-500ms **Recommended:** 100-200ms

Longer timeout = more patient waiting Shorter timeout = faster failure, but may
miss slow completions

### Caching (Phase 2)

Spec caching configuration:

```toml
[performance]
# LRU cache size (number of specs)
spec_cache_size = 50

# Cache TTL (seconds, 0 = never expire)
spec_cache_ttl = 0
```

## Shell-Specific Configuration

### Zsh Options

Add to `~/.zshrc`:

```bash
# Disable autocomplete if in specific directories
precmd() {
  if [[ $PWD == /slow/nfs/mount/* ]]; then
    unset AUTOCOMPLETE_RS_ENABLE
  else
    export AUTOCOMPLETE_RS_ENABLE=1
  fi
}

# Custom socket path
export AUTOCOMPLETE_RS_SOCKET="$HOME/.cache/autocomplete.sock"

# Custom config path
export AUTOCOMPLETE_RS_CONFIG="$HOME/.config/autocomplete-rs/my-config.toml"

# Disable for non-interactive shells
[[ $- != *i* ]] && return
```

### Bash Options (Phase 4)

Coming soon.

### Fish Options (Phase 4)

Coming soon.

## Advanced Configuration

### Multiple Profiles

Create different configs for different contexts:

```bash
# Work profile
~/.config/autocomplete-rs/config-work.toml

# Personal profile
~/.config/autocomplete-rs/config-personal.toml
```

Switch profiles:

```bash
export AUTOCOMPLETE_RS_CONFIG="$HOME/.config/autocomplete-rs/config-work.toml"
```

### Per-Project Configuration

Create `.autocomplete-rs.toml` in project root:

```toml
# Project-specific settings
[general]
max_suggestions = 20 # More suggestions for this project

[ui]
show_descriptions = false # Minimal UI
```

**Note:** Per-project configs coming in Phase 3.

### Environment Variables

Override config with environment variables:

```bash
# Socket path
export AUTOCOMPLETE_RS_SOCKET="/tmp/my-autocomplete.sock"

# Config file
export AUTOCOMPLETE_RS_CONFIG="$HOME/.config/my-config.toml"

# Debug mode
export AUTOCOMPLETE_RS_DEBUG=1

# Max suggestions
export AUTOCOMPLETE_RS_MAX_SUGGESTIONS=20

# Theme
export AUTOCOMPLETE_RS_THEME="latte"
```

Priority (highest to lowest):

1. Environment variables
2. Per-project config
3. User config (`~/.config/autocomplete-rs/config.toml`)
4. Default values

## Config Validation

Validate your config:

```bash
autocomplete-rs config validate
```

**Output:**

```text
✓ Config file: /Users/you/.config/autocomplete-rs/config.toml
✓ Syntax: Valid TOML
✓ Socket path: /tmp/autocomplete-rs.sock (writable)
✓ Theme: mocha (valid)
✓ Max suggestions: 10 (valid range)
✓ Key binding: \e  (valid)

Config is valid!
```

**Note:** Validation command coming in Phase 3.

## Example Configurations

### Minimal (Fast)

```toml
[general]
max_suggestions = 5

[ui]
show_descriptions = false
show_types = false
max_height = 10

[performance]
request_timeout = 50
```

### Verbose (Informative)

```toml
[general]
max_suggestions = 20
debug = true

[ui]
show_descriptions = true
show_types = true
max_height = 20
width = 100

[performance]
request_timeout = 200
```

### Light Theme (Catppuccin Latte)

```toml
[theme]
name = "latte"

[ui]
show_descriptions = true
show_types = true
```

### Custom Dark Theme

```toml
[theme]
name = "custom"

[theme.custom]
border = "#7aa2f7" # Blue
selected_bg = "#7aa2f7"
selected_fg = "#1a1b26" # Dark bg
text = "#c0caf5" # Light text
description = "#9aa5ce"
error = "#f7768e" # Red
warning = "#e0af68" # Yellow
```

## Troubleshooting Config

### Config Not Loading

Check file location:

```bash
ls -la ~/.config/autocomplete-rs/config.toml
```

Check syntax:

```bash
# Install TOML linter
cargo install taplo-cli

# Lint config
taplo lint ~/.config/autocomplete-rs/config.toml
```

### Colors Not Working

Check terminal support:

```bash
# Should show "truecolor" or "24bit"
echo $COLORTERM

# Test true color
awk 'BEGIN{
    s="/\\/\\/\\/\\/\\"; s=s s s s s s s s;
    for (colnum = 0; colnum<77; colnum++) {
        r = 255-(colnum*255/76);
        g = (colnum*510/76);
        b = (colnum*255/76);
        if (g>255) g = 510-g;
        printf "\033[48;2;%d;%d;%dm", r,g,b;
        printf "\033[38;2;%d;%d;%dm", 255-r,255-g,255-b;
        printf "%s\033[0m", substr(s,colnum+1,1);
    }
    printf "\n";
}'
```

If colors look wrong, terminal may not support truecolor. Use 256-color theme.

### Performance Issues

If completions are slow:

1. **Reduce max_suggestions:**

   ```toml
   max_suggestions = 5
   ```

2. **Disable descriptions:**

   ```toml
   show_descriptions = false
   ```

3. **Increase cache size (Phase 2):**

   ```toml
   spec_cache_size = 100
   ```

4. **Check daemon is running:**

   ```bash
   ps aux | grep autocomplete-rs
   ```

## Resetting Configuration

Remove config to use defaults:

```bash
mv ~/.config/autocomplete-rs/config.toml ~/.config/autocomplete-rs/config.toml.backup
```

Restart daemon:

```bash
pkill autocomplete-rs
```

Next completion will use defaults.

## Next Steps

- See [Troubleshooting](troubleshooting.md) for common issues
- Check [Installation Guide](installation.md) for setup
- Read [ADR-0005](../adr/0005-ratatui-for-tui.md) for TUI technical details

## Getting Help

- **Config Questions:**
  [GitHub Discussions](https://github.com/YOUR_USERNAME/autocomplete-rs/discussions)
- **Bug Reports:**
  [GitHub Issues](https://github.com/YOUR_USERNAME/autocomplete-rs/issues)
- **Feature Requests:**
  [GitHub Discussions](https://github.com/YOUR_USERNAME/autocomplete-rs/discussions/categories/ideas)

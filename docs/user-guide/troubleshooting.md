# Troubleshooting Guide

This guide helps resolve common issues with autocomplete-rs.

## Quick Diagnostics

Run these commands to check system health:

```bash
# 1. Check binary is installed
which autocomplete-rs
autocomplete-rs --version

# 2. Check daemon is running
ps aux | grep autocomplete-rs

# 3. Check socket exists
ls -la /tmp/autocomplete-rs.sock

# 4. Check shell integration loaded
bindkey | grep autocomplete  # zsh
```

If any fail, see relevant section below.

## Installation Issues

### Binary Not Found

**Symptom:** `command not found: autocomplete-rs`

**Causes:**

- Binary not in PATH
- Binary not installed
- PATH not updated after installation

**Solutions:**

1. **Check installation:**

   ```bash
   ls ~/.cargo/bin/autocomplete-rs
   # or
   ls /usr/local/bin/autocomplete-rs
   ```

2. **Add to PATH:**

   ```bash
   # Add to ~/.zshrc or ~/.bashrc
   export PATH="$HOME/.cargo/bin:$PATH"

   # Reload shell
   source ~/.zshrc
   ```

3. **Reinstall:**

   ```bash
   cargo install autocomplete-rs --force
   # or build from source
   cd autocomplete-rs && cargo build --release
   cp target/release/autocomplete-rs ~/.cargo/bin/
   ```

### Build Failures

**Symptom:** `cargo build` fails

**Common errors:**

**1. Rust version too old:**

```text
error[E0658]: use of unstable library feature 'edition_2024'
```

**Solution:**

```bash
rustup update stable
rustc --version  # Should be 1.85.0+
```

**2. Missing dependencies:**

```text
error: linker 'cc' not found
```

**Solution (macOS):**

```bash
xcode-select --install
```

**Solution (Ubuntu/Debian):**

```bash
sudo apt-get install build-essential
```

**Solution (Fedora):**

```bash
sudo dnf install gcc
```

**3. deno_ast build errors:**

```text
error: failed to compile deno_ast
```

**Solution:** deno_ast is temporarily disabled. This is expected until Phase 2.

Verify in `Cargo.toml`:

```toml
[build-dependencies]
# deno_ast = "0.40"  # Disabled until Phase 2
```

## Daemon Issues

### Daemon Won't Start

**Symptom:** No completions appear, daemon not running

**Diagnosis:**

```bash
# Try starting daemon manually
autocomplete-rs daemon /tmp/autocomplete-rs.sock

# Check for errors
RUST_LOG=debug autocomplete-rs daemon /tmp/autocomplete-rs.sock
```

**Common errors:**

**1. Address already in use:**

```text
Error: Address already in use (os error 48)
```

**Solution:**

```bash
# Kill existing daemon
pkill autocomplete-rs

# Remove stale socket
rm /tmp/autocomplete-rs.sock

# Restart
autocomplete-rs daemon /tmp/autocomplete-rs.sock &
```

**2. Permission denied:**

```text
Error: Permission denied (os error 13)
```

**Solution:**

```bash
# Check socket directory permissions
ls -la /tmp/

# Use user-writable location
mkdir -p ~/.cache/autocomplete-rs
autocomplete-rs daemon ~/.cache/autocomplete-rs/daemon.sock &

# Update config to match
```

**3. No such file or directory:**

```text
Error: No such file or directory (os error 2)
```

**Solution:**

```bash
# Create directory
mkdir -p /tmp

# Or use alternative path
mkdir -p ~/.cache/autocomplete-rs
```

### Daemon Crashes

**Symptom:** Daemon runs briefly then exits

**Diagnosis:**

```bash
# Run in foreground with debug logging
RUST_LOG=debug autocomplete-rs daemon /tmp/autocomplete-rs.sock

# Check system logs (macOS)
log show --predicate 'process == "autocomplete-rs"' --last 1m

# Check system logs (Linux)
journalctl -u autocomplete-rs --since "1 minute ago"
```

**Common causes:**

- Out of memory (unlikely with <50MB usage)
- Panic in code (bug - please report!)
- Signal received (SIGTERM, SIGKILL)

**Solution:**

1. Capture logs and file
   [GitHub issue](https://github.com/YOUR_USERNAME/autocomplete-rs/issues)
2. Include rust backtrace:

   ```bash
   RUST_BACKTRACE=1 autocomplete-rs daemon /tmp/autocomplete-rs.sock
   ```

### Daemon Unresponsive

**Symptom:** Daemon running but no completions

**Diagnosis:**

```bash
# Test daemon directly
echo '{"buffer":"git checkout","cursor":13}' | nc -U /tmp/autocomplete-rs.sock

# Should return JSON response
```

**If no response:**

1. **Check socket path matches:**

   ```bash
   # Daemon socket
   ls -la /tmp/autocomplete-rs.sock

   # Shell integration uses same path?
   grep AUTOCOMPLETE_RS_SOCKET ~/.zshrc
   ```

2. **Check for deadlock (bug):**

   ```bash
   # Get daemon PID
   ps aux | grep autocomplete-rs | grep daemon

   # Attach debugger (macOS)
   sudo lldb -p <PID>
   (lldb) bt all  # backtrace of all threads

   # Or send SIGQUIT to dump stack
   kill -QUIT <PID>
   ```

3. **Restart daemon:**

   ```bash
   pkill autocomplete-rs
   autocomplete-rs daemon /tmp/autocomplete-rs.sock &
   ```

## Shell Integration Issues

### No Completions Appear

**Symptom:** Press trigger key, nothing happens

**Diagnosis:**

1. **Check widget is loaded:**

   ```bash
   # Zsh
   zle -la | grep autocomplete
   # Should show: _autocomplete_rs_widget
   ```

2. **Check key binding:**

   ```bash
   bindkey | grep autocomplete
   # Should show: "^[ " _autocomplete_rs_widget (or your binding)
   ```

3. **Test widget directly:**

   ```zsh
   # Type a command
   git checkout

   # Manually trigger widget
   zle _autocomplete_rs_widget

   # Should show completions
   ```

**Solutions:**

**Widget not loaded:**

```bash
# Check integration is sourced
cat ~/.zshrc | grep autocomplete-rs

# Should have:
source ~/.config/autocomplete-rs/zsh.zsh

# If missing, run installer
autocomplete-rs install zsh

# Or add manually
echo 'source ~/.config/autocomplete-rs/zsh.zsh' >> ~/.zshrc
source ~/.zshrc
```

**Key binding wrong:**

```bash
# Check what Alt+Space is bound to
bindkey "^[ "

# Should show: _autocomplete_rs_widget

# If not, rebind:
bindkey '^[ ' _autocomplete_rs_widget
```

**Daemon not running:**

```bash
ps aux | grep autocomplete-rs

# If not running, widget should auto-start it
# If auto-start fails, start manually:
autocomplete-rs daemon /tmp/autocomplete-rs.sock &
```

### Wrong Completions

**Symptom:** Completions don't match current context

**Note:** This is expected in early versions. Spec parsing (Phase 2) not yet
implemented.

**Current behavior:**

- Hardcoded test completions
- No context awareness
- No spec matching

**Coming in Phase 2:**

- Full Fig spec support
- Context-aware suggestions
- 600+ CLI tools

**Workaround:** Wait for Phase 2 release or contribute to spec parser
implementation!

### Completions Too Slow

**Symptom:** >1 second delay before completions appear

**Target:** <20ms total latency

**Diagnosis:**

1. **Time each component:**

   ```bash
   # Daemon startup
   time autocomplete-rs daemon /tmp/test.sock &

   # Request latency
   time echo '{"buffer":"git","cursor":3}' | nc -U /tmp/autocomplete-rs.sock

   # Full flow
   time (trigger completion in shell)
   ```

2. **Check daemon is running:**

   ```bash
   ps aux | grep autocomplete-rs
   ```

**If daemon starts each time (slow):**

- Daemon should persist, not start per-request
- Check if daemon is being killed
- Verify socket path consistency

**If request is slow:**

- Should be <10ms
- Check CPU usage: `top -pid $(pgrep autocomplete-rs)`
- File bug report with timing data

**Solutions:**

1. **Keep daemon warm:**

   ```bash
   # Start daemon at shell startup
   if ! pgrep autocomplete-rs > /dev/null; then
     autocomplete-rs daemon /tmp/autocomplete-rs.sock &
   fi
   ```

2. **Reduce max suggestions:**

   ```toml
   # ~/.config/autocomplete-rs/config.toml
   [general]
   max_suggestions = 5
   ```

3. **Disable descriptions (Phase 3):**

   ```toml
   [ui]
   show_descriptions = false
   ```

## Display Issues

### UI Appears in Wrong Location

**Symptom:** Dropdown offset from cursor

**This should not happen!** Direct terminal control prevents positioning bugs.

**If it does:**

- This is a bug, please report
- Include terminal type and font settings
- Include screenshot

**Workaround:** None - file
[issue](https://github.com/YOUR_USERNAME/autocomplete-rs/issues) with details.

### UI Overlaps Prompt

**Symptom:** Completion UI covers command prompt

**Expected behavior:** UI should push prompt down temporarily

**If overlapping:**

- May be terminal emulator quirk
- Try different terminal
- Report issue with terminal name/version

### Colors Look Wrong

**Symptom:** Weird colors, garbled output, or no colors

**Diagnosis:**

1. **Check terminal color support:**

   ```bash
   echo $COLORTERM
   # Should be: "truecolor" or "24bit"

   # Check TERM
   echo $TERM
   # Should be: "xterm-256color" or better
   ```

2. **Test color output:**

   ```bash
   # Simple color test
   printf "\033[38;2;255;0;0mRed\033[0m "
   printf "\033[38;2;0;255;0mGreen\033[0m "
   printf "\033[38;2;0;0;255mBlue\033[0m\n"

   # Should show red, green, blue text
   ```

**Solutions:**

**Terminal doesn't support truecolor:**

```bash
# Use 256-color mode (automatic fallback)
# Or use basic ANSI theme
```

**TERM variable wrong:**

```bash
# Add to ~/.zshrc
export TERM=xterm-256color

# For modern terminals
export COLORTERM=truecolor
```

**Terminal.app (macOS) limitations:**

- Supports 256 colors, not truecolor
- Use built-in theme (auto-adapts)

**iTerm2, Alacritty, Kitty, WezTerm:**

- Full truecolor support
- Should work perfectly

### Text Garbled or Flickers

**Symptom:** Screen artifacts, flickering, mangled text

**Causes:**

- Terminal not handling escape codes correctly
- Timing issues
- Screen redraw bugs

**Solutions:**

1. **Clear screen:**

   ```bash
   clear
   ```

2. **Reset terminal:**

   ```bash
   reset
   ```

3. **Update terminal:**
   - Old terminals may have bugs
   - Try latest version

4. **Disable UI temporarily:**

   ```bash
   # Unset widget
   bindkey -r '^[ '

   # Test without autocomplete
   ```

5. **Report bug:**
   - Include terminal type and version
   - Include screenshot or asciinema recording

## Performance Issues

### High CPU Usage

**Symptom:** autocomplete-rs using excessive CPU

**Expected:** <1% when idle, <10% during completion

**If higher:**

1. **Check for busy loop:**

   ```bash
   # Sample daemon
   sudo sample $(pgrep autocomplete-rs) 5

   # Check output for hot functions
   ```

2. **Check for stuck request:**

   ```bash
   # Send SIGQUIT to dump state
   kill -QUIT $(pgrep autocomplete-rs)

   # Check output
   ```

3. **Restart daemon:**

   ```bash
   pkill autocomplete-rs
   ```

4. **File bug** with sampling data

### High Memory Usage

**Symptom:** autocomplete-rs using >100MB

**Expected:** <50MB with all specs loaded (Phase 2)

**If higher:**

1. **Check actual usage:**

   ```bash
   ps aux | grep autocomplete-rs
   # Look at RSS column (real memory)
   ```

2. **Check for memory leak:**

   ```bash
   # Install heaptrack (Linux)
   heaptrack autocomplete-rs daemon /tmp/autocomplete-rs.sock

   # Use Instruments (macOS)
   # Profile > Allocations
   ```

3. **Restart daemon periodically:**

   ```bash
   # Cron job to restart daily
   0 0 * * * pkill autocomplete-rs
   ```

4. **File bug** with memory profile

## Compatibility Issues

### Doesn't Work on My Terminal

**Supported terminals:**

- iTerm2
- Alacritty
- Kitty
- WezTerm
- Ghostty
- Terminal.app
- GNOME Terminal
- Konsole
- Terminator

**If your terminal not listed:**

1. **Check ANSI support:**

   ```bash
   # Test basic escape codes
   printf "\033[1;32mGreen Bold\033[0m\n"
   ```

2. **Try anyway:**
   - Should work on any ANSI-compatible terminal
   - May have minor visual issues

3. **Report compatibility:**
   - File issue with terminal name/version
   - Include screenshots
   - We'll add to supported list

### Doesn't Work with My Shell

**Currently supported:**

- Zsh 5.8+ (Phase 1 - available now)

**Coming soon:**

- Bash 4.0+ (Phase 4)
- Fish 3.0+ (Phase 4)

**Workarounds for bash/fish:**

- Wait for Phase 4
- Contribute integration (see [Contributing](../development/contributing.md))

**Other shells (nushell, elvish, xonsh):**

- Not currently planned
- Community contributions welcome

### Conflicts with Other Tools

**Symptom:** autocomplete-rs breaks or is broken by other tools

**Common conflicts:**

**1. Fig/Amazon Q:**

- Both try to provide completions
- Uninstall Fig/Amazon Q first
- Or disable their autocomplete

**2. zsh-autosuggestions:**

- May interfere with key bindings
- Usually works fine together
- If issues, try different trigger key

**3. fzf tab completion:**

- May conflict if using same key (Tab)
- Use different trigger key:

  ```bash
  bindkey '^[ ' _autocomplete_rs_widget  # Alt+Space
  ```

**4. Custom ZLE widgets:**

- Check for widget name conflicts
- Rename if needed:

  ```bash
  zle -N my_autocomplete_widget _autocomplete_rs_widget
  bindkey '^[ ' my_autocomplete_widget
  ```

## Error Messages

### "Socket connection refused"

**Cause:** Daemon not running

**Solution:**

```bash
autocomplete-rs daemon /tmp/autocomplete-rs.sock &
```

### "Request timeout"

**Cause:** Daemon not responding within timeout

**Solutions:**

1. Check daemon is running: `ps aux | grep autocomplete-rs`
2. Increase timeout in config (Phase 3)
3. Restart daemon

### "Invalid response from daemon"

**Cause:** Protocol mismatch or daemon crash

**Solutions:**

1. Restart daemon
2. Check versions match (binary and shell integration)
3. File bug with logs

### "Spec not found"

**Cause:** Completion spec missing (Phase 2+)

**Solutions:**

1. Check spec file exists
2. Rebuild to re-embed specs
3. File issue if spec should exist

## Getting Help

### Before Asking

1. Check this troubleshooting guide
2. Search
   [existing issues](https://github.com/YOUR_USERNAME/autocomplete-rs/issues)
3. Try with debug logging:

   ```bash
   RUST_LOG=debug autocomplete-rs daemon /tmp/autocomplete-rs.sock 2> /tmp/debug.log
   ```

4. Gather version info:

   ```bash
   autocomplete-rs --version
   rustc --version
   echo $SHELL
   echo $TERM
   ```

### Where to Ask

**Bug Reports:**
[GitHub Issues](https://github.com/YOUR_USERNAME/autocomplete-rs/issues/new)

**Questions:**
[GitHub Discussions](https://github.com/YOUR_USERNAME/autocomplete-rs/discussions)

**Security Issues:** Email maintainers (see README)

### What to Include

**For bugs:**

- autocomplete-rs version
- Operating system and version
- Terminal type and version
- Shell type and version
- Steps to reproduce
- Expected vs actual behavior
- Logs (with RUST_LOG=debug)
- Screenshots (if visual issue)

**For questions:**

- What you're trying to do
- What you've tried
- Relevant config

## Still Stuck?

If none of the above helps:

1. **Enable maximum debugging:**

   ```bash
   RUST_LOG=trace RUST_BACKTRACE=full autocomplete-rs daemon /tmp/autocomplete-rs.sock 2> /tmp/full-debug.log
   ```

2. **Trigger the issue**

3. **File detailed bug report** with all logs and system info

4. **Be patient** - maintainers will respond within 2-3 days

Thank you for helping improve autocomplete-rs!

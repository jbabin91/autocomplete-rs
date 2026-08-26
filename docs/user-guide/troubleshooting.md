# Troubleshooting Guide

This guide helps resolve common issues with autocomplete-rs.

## Quick Diagnostics

Run these commands to check system health:

```sh
# 1. Check binary is installed
which autocomplete-rs
autocomplete-rs --version

# 2. Check daemon is running
ps aux | grep autocomplete-rs

# 3. Check socket exists
ls -la ~/.autocomplete-rs/daemon.sock

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

   ```sh
   ls ~/.cargo/bin/autocomplete-rs
   # or
   ls /usr/local/bin/autocomplete-rs
   ```

2. **Add to PATH:**

   ```sh
   # Add to ~/.zshrc or ~/.bashrc
   export PATH="$HOME/.cargo/bin:$PATH"

   # Reload shell
   source ~/.zshrc
   ```

3. **Reinstall:**

   ```sh
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

```sh
rustup update stable
rustc --version  # Should meet the rust-version in Cargo.toml
```

**2. Missing dependencies:**

```text
error: linker 'cc' not found
```

**Solution (macOS):**

```sh
xcode-select --install
```

**Solution (Ubuntu/Debian):**

```sh
sudo apt-get install build-essential
```

**Solution (Fedora):**

```sh
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

```sh
# Try starting daemon manually
autocomplete-rs daemon

# Check for errors
RUST_LOG=debug autocomplete-rs daemon
```

**Common errors:**

**1. Address already in use:**

```text
Error: Address already in use (os error 48)
```

**Solution:**

```sh
# Kill existing daemon
pkill autocomplete-rs

# Remove stale socket
rm ~/.autocomplete-rs/daemon.sock

# Restart
autocomplete-rs daemon &
```

**2. Permission denied:**

```text
Error: Permission denied (os error 13)
```

**Solution:**

```sh
# Check socket directory permissions
ls -la /tmp/

# Use user-writable location
mkdir -p ~/.cache/autocomplete-rs
autocomplete-rs daemon --socket ~/.cache/autocomplete-rs/daemon.sock &

# Update config to match
```

**3. No such file or directory:**

```text
Error: No such file or directory (os error 2)
```

**Solution:**

```sh
# Create directory
mkdir -p /tmp

# Or use alternative path
mkdir -p ~/.cache/autocomplete-rs
```

### Daemon Crashes

**Symptom:** Daemon runs briefly then exits

**Diagnosis:**

```sh
# Run in foreground with debug logging
RUST_LOG=debug autocomplete-rs daemon

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
   [GitHub issue](https://github.com/jbabin91/autocomplete-rs/issues)
2. Include rust backtrace:

   ```sh
   RUST_BACKTRACE=1 autocomplete-rs daemon
   ```

### Daemon Unresponsive

**Symptom:** Daemon running but no completions

**Diagnosis:**

```sh
# Test daemon directly
echo '{"buffer":"git checkout","cursor":13}' | nc -U ~/.autocomplete-rs/daemon.sock

# Should return JSON response
```

**If no response:**

1. **Check socket path matches:**

   ```sh
   # Daemon socket
   ls -la ~/.autocomplete-rs/daemon.sock

   # Shell integration uses same path?
   grep AUTOCOMPLETE_RS_SOCKET ~/.zshrc
   ```

2. **Check for deadlock (bug):**

   ```sh
   # Get daemon PID
   ps aux | grep autocomplete-rs | grep daemon

   # Attach debugger (macOS)
   sudo lldb -p <PID>
   (lldb) bt all  # backtrace of all threads

   # Or send SIGQUIT to dump stack
   kill -QUIT <PID>
   ```

3. **Restart daemon:**

   ```sh
   pkill autocomplete-rs
   autocomplete-rs daemon &
   ```

## Shell Integration Issues

### No Completions Appear

**Symptom:** Press trigger key, nothing happens

**Diagnosis:**

1. **Check widget is loaded:**

   ```sh
   # Zsh
   zle -la | grep autocomplete
   # Should show: _autocomplete_rs_widget
   ```

2. **Check key binding:**

   ```sh
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

```sh
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

```sh
# Check what Alt+Space is bound to
bindkey "^[ "

# Should show: _autocomplete_rs_widget

# If not, rebind:
bindkey '^[ ' _autocomplete_rs_widget
```

**Daemon not running:**

```sh
ps aux | grep autocomplete-rs

# If not running, widget should auto-start it
# If auto-start fails, start manually:
autocomplete-rs daemon &
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

   ```sh
   # Daemon startup
   time autocomplete-rs daemon --socket /tmp/test.sock &

   # Request latency
   time echo '{"buffer":"git","cursor":3}' | nc -U ~/.autocomplete-rs/daemon.sock

   # Full flow
   time (trigger completion in shell)
   ```

2. **Check daemon is running:**

   ```sh
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

   ```sh
   # Start daemon at shell startup
   if ! pgrep autocomplete-rs > /dev/null; then
     autocomplete-rs daemon &
   fi
   ```

## Performance Issues

### High CPU Usage

**Symptom:** autocomplete-rs using excessive CPU

**Expected:** <1% when idle, <10% during completion

**If higher:**

1. **Check for busy loop:**

   ```sh
   # Sample daemon
   sudo sample $(pgrep autocomplete-rs) 5

   # Check output for hot functions
   ```

2. **Check for stuck request:**

   ```sh
   # Send SIGQUIT to dump state
   kill -QUIT $(pgrep autocomplete-rs)

   # Check output
   ```

3. **Restart daemon:**

   ```sh
   pkill autocomplete-rs
   ```

4. **File bug** with sampling data

### High Memory Usage

**Symptom:** autocomplete-rs using excessive memory

**If higher than expected:**

1. **Check actual usage:**

   ```sh
   ps aux | grep autocomplete-rs
   # Look at RSS column (real memory)
   ```

2. **Restart daemon:**

   ```sh
   pkill autocomplete-rs
   ```

3. **File bug** with memory usage data

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

   ```sh
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

**1. Kiro CLI (formerly Fig/Amazon Q):**

- Both try to provide completions
- Uninstall Kiro CLI first
- Or disable their autocomplete

**2. zsh-autosuggestions:**

- May interfere with key bindings
- Usually works fine together
- If issues, try different trigger key

**3. fzf tab completion:**

- May conflict if using same key (Tab)
- Use different trigger key:

  ```sh
  bindkey '^[ ' _autocomplete_rs_widget  # Alt+Space
  ```

**4. Custom ZLE widgets:**

- Check for widget name conflicts
- Rename if needed:

  ```sh
  zle -N my_autocomplete_widget _autocomplete_rs_widget
  bindkey '^[ ' my_autocomplete_widget
  ```

## Error Messages

### "Socket connection refused"

**Cause:** Daemon not running

**Solution:**

```sh
autocomplete-rs daemon &
```

### "Request timeout"

**Cause:** Daemon not responding within timeout

**Solutions:**

1. Check daemon is running: `ps aux | grep autocomplete-rs`
2. Restart daemon

### "Invalid response from daemon"

**Cause:** Protocol mismatch or daemon crash

**Solutions:**

1. Restart daemon
2. Check versions match (binary and shell integration)
3. File bug with logs

## Getting Help

### Before Asking

1. Check this troubleshooting guide
2. Search
   [existing issues](https://github.com/jbabin91/autocomplete-rs/issues)
3. Try with debug logging:

   ```sh
   RUST_LOG=debug autocomplete-rs daemon 2> /tmp/debug.log
   ```

4. Gather version info:

   ```sh
   autocomplete-rs --version
   rustc --version
   echo $SHELL
   echo $TERM
   ```

### Where to Ask

**Bug Reports:**
[GitHub Issues](https://github.com/jbabin91/autocomplete-rs/issues/new)

**Questions:**
[GitHub Issues](https://github.com/jbabin91/autocomplete-rs/issues)

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

   ```sh
   RUST_LOG=trace RUST_BACKTRACE=full autocomplete-rs daemon 2> /tmp/full-debug.log
   ```

2. **Trigger the issue**

3. **File detailed bug report** with all logs and system info

4. **Be patient** - maintainers will respond within 2-3 days

Thank you for helping improve autocomplete-rs!

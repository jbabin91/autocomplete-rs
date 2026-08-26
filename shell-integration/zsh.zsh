#!/usr/bin/env zsh
# autocomplete-rs ZLE integration for zsh

# Socket path for daemon communication. Not under /tmp: that directory is
# world-writable, so another local user could pre-create the path and accept
# our connections. Must match paths::default_socket_path().
# $HOME is required even when the socket is overridden: the daemon still resolves its log
# and database paths from it, and refuses to start without one.
if [[ -z "$HOME" ]]; then
    print -u2 "autocomplete-rs: \$HOME is not set; completions disabled"
    return 1
fi
: ${AUTOCOMPLETE_RS_SOCKET:="$HOME/.autocomplete-rs/daemon.sock"}

# Widget function that gets called on trigger
_autocomplete_rs_widget() {
    # Get current buffer and cursor position
    local buffer="$BUFFER"
    local cursor="$CURSOR"

    # Ensure daemon is running
    _autocomplete_rs_ensure_daemon

    # Call autocomplete-rs complete command
    # This will show the inline dropdown and return the selected completion
    local completion=$(autocomplete-rs complete "$buffer" --cursor "$cursor" --socket "$AUTOCOMPLETE_RS_SOCKET" 2>/dev/null)

    # If a completion was selected, insert it
    if [[ -n "$completion" ]]; then
        # Find the last word/token to replace
        local before="${buffer[1,$cursor]}"
        local after="${buffer[$((cursor+1)),-1]}"

        # Simple word boundary detection (space or start of line)
        local word_start=1
        for ((i=$cursor; i>=1; i--)); do
            if [[ "${buffer[$i]}" == " " ]]; then
                word_start=$((i+1))
                break
            fi
        done

        # Replace the current word with the completion
        local prefix="${buffer[1,$((word_start-1))]}"
        BUFFER="${prefix}${completion} ${after}"
        CURSOR=$((${#prefix} + ${#completion} + 1))

        # Refresh the line
        zle reset-prompt
    fi
}

# Register the widget
zle -N _autocomplete_rs_widget

# Bind to a key (Alt+Space by default, customize as needed)
bindkey '^[ ' _autocomplete_rs_widget  # Alt+Space

# Auto-start daemon if not running
_autocomplete_rs_ensure_daemon() {
    [[ -S "$AUTOCOMPLETE_RS_SOCKET" ]] && return 0
    # Only try once per shell: a daemon that refuses to start (bad socket directory,
    # unwritable path) would otherwise be respawned on every trigger, silently.
    (( $+_autocomplete_rs_start_attempted )) && return 1
    typeset -g _autocomplete_rs_start_attempted=1

    # stdout is discarded; stderr is kept so a refusal to start is visible.
    autocomplete-rs daemon --socket "$AUTOCOMPLETE_RS_SOCKET" >/dev/null &!

    # Poll rather than sleep a fixed interval: startup is ~200-300ms on a debug build, so
    # any single short sleep reports a healthy daemon as failed.
    local i
    for i in {1..40}; do
        [[ -S "$AUTOCOMPLETE_RS_SOCKET" ]] && return 0
        sleep 0.05
    done
    print -u2 "autocomplete-rs: daemon did not start; see ~/.autocomplete-rs/logs/"
    return 1
}

# Start daemon on shell init
_autocomplete_rs_ensure_daemon

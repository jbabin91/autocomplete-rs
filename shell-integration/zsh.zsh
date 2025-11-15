#!/usr/bin/env zsh
# autocomplete-rs ZLE integration for zsh

# Socket path for daemon communication
AUTOCOMPLETE_RS_SOCKET="${AUTOCOMPLETE_RS_SOCKET:-/tmp/autocomplete-rs.sock}"

# Widget function that gets called on trigger
_autocomplete_rs_widget() {
    # Get current buffer and cursor position
    local buffer="$BUFFER"
    local cursor="$CURSOR"

    # Ensure daemon is running
    _autocomplete_rs_ensure_daemon

    # Call autocomplete-rs complete command
    # This will show the TUI dropdown and return the selected completion
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
    if [[ ! -S "$AUTOCOMPLETE_RS_SOCKET" ]]; then
        # Start daemon in background
        autocomplete-rs daemon --socket "$AUTOCOMPLETE_RS_SOCKET" >/dev/null 2>&1 &!
        # Give it a moment to start
        sleep 0.1
    fi
}

# Start daemon on shell init
_autocomplete_rs_ensure_daemon

#!/usr/bin/env zsh
# autocomplete-rs ZLE integration for zsh

# Socket path for daemon communication
AUTOCOMPLETE_RS_SOCKET="/tmp/autocomplete-rs.sock"

# Widget function that gets called on trigger
_autocomplete_rs_widget() {
    # Get current buffer and cursor position
    local buffer="$BUFFER"
    local cursor="$CURSOR"

    # TODO: Call autocomplete-rs daemon via Unix socket
    # For now, just a placeholder
    # local suggestions=$(echo "$buffer" | nc -U $AUTOCOMPLETE_RS_SOCKET)

    # TODO: Parse suggestions and display TUI dropdown
    # The Rust daemon will handle TUI rendering directly

    # Trigger the autocomplete-rs complete command
    # This will show the TUI dropdown at the correct position
    # autocomplete-rs complete "$buffer" --cursor $cursor
}

# Register the widget
zle -N _autocomplete_rs_widget

# Bind to a key (Alt+Space by default, customize as needed)
bindkey '^[ ' _autocomplete_rs_widget  # Alt+Space

# Auto-start daemon if not running
_autocomplete_rs_ensure_daemon() {
    if [[ ! -S "$AUTOCOMPLETE_RS_SOCKET" ]]; then
        autocomplete-rs daemon --socket "$AUTOCOMPLETE_RS_SOCKET" &!
    fi
}

# Start daemon on shell init
_autocomplete_rs_ensure_daemon

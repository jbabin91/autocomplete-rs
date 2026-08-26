# Shell Integration Development Rules

## ZSH Widget (`zsh.zsh`)

- ZLE widget registered with `zle -N _autocomplete_rs_widget`
- Default key binding: `Alt+Space` (`'^[ '`)
- Captures `$BUFFER` (full command line) and `$CURSOR` (position)
- Calls `autocomplete-rs complete "$buffer" --cursor "$cursor"` as subprocess
- Parses stdout for selected completion text
- Updates `$BUFFER` and `$CURSOR` after insertion, then `zle reset-prompt`

## Daemon Auto-Start

- Checks socket existence: `[[ ! -S "$AUTOCOMPLETE_RS_SOCKET" ]]`
- Starts in background with `&!` (disown)
- Waits 0.1s for startup
- Also runs on shell init (not just on first trigger)

## Word Boundary Detection

- Simple backward scan from cursor position for space character
- Replaces the current word (from last space to cursor) with completion
- Preserves text before and after the completion point

## Shell Script Conventions

- Use `#!/usr/bin/env zsh` shebang
- Quote all variable expansions: `"$BUFFER"` not `$BUFFER`
- Redirect stderr to /dev/null for daemon calls: `2>/dev/null`
- Use `local` for all function-scoped variables

## Future Shells (Phase 4)

- `bash.sh` — readline-based, uses `COMP_WORDS`, `COMP_LINE`, `COMP_POINT`, `COMPREPLY`
- `fish.fish` — native completion system, uses `commandline -c`, `complete -c`
- All shells use the same Unix socket protocol to communicate with daemon

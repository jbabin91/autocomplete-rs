# ADR-0009: DSR Cursor Positioning

**Status:** Accepted — see [Erratum (2026-08-26)](#erratum-2026-08-26)
**Date:** 2026-08-25
**Decision Makers:** Project Team
**Technical Story:** Make the terminal report its own cursor position instead of
deriving the caret from the Accessibility API
**Amends:** [ADR-0008](0008-native-overlay-dropdown.md) (Native Overlay Dropdown)

## Context

ADR-0008 chose native overlay windows and specified a positioning pipeline that
derives the caret from platform APIs:

1. Query terminal window bounds (macOS: Accessibility API `AXPosition`/`AXSize`)
2. Get terminal grid dimensions (TIOCGWINSZ)
3. Compute cell pixel position
4. Flip above if near edge

That ADR listed "shell integration assists" as a *mitigation* — the widget
"can pass cursor coordinates to the daemon, reducing reliance on platform APIs."
This ADR promotes that mitigation to the primary path, because evidence from a
shipping implementation says the Accessibility route does not survive contact
with real terminals.

### Evidence

[Lori](https://lori-app.sh) is a closed-source macOS overlay autocomplete whose
marketing says it "anchors to your cursor caret using the Accessibility API."
Its shipped zsh integration (`Lori.app/Contents/Resources/Shell/lori.zsh`,
v0.2.2) does something else, and says so in a comment:

> At line-init the prompt is drawn and the user hasn't typed yet, so we can ask
> the terminal where the cursor is (`\e[6n` → `\e[row;colR`) and read the reply
> WITHOUT risking swallowing a keystroke. Send it as the input origin; the app
> combines it with the buffer to place the popup on ANY terminal — no
> Accessibility cursor data needed. Feature-detected: if the first probe goes
> unanswered, stop probing (the app then falls back to its AX-based positioning).

DSR is primary; Accessibility is the fallback. The same file carries three
separate workarounds for Accessibility misbehaviour, each with a stated cause:

- **Ctrl-L erases scrollback** (`\e[3J`) because "Ghostty keeps scrolled-off
  lines in its AX buffer, which otherwise inflates the cursor's line number."
  The integration accepts losing scroll-up history to keep positioning correct.
- **Geometry re-capture is gated** behind a `__lori_cmd_ran` flag because the
  "terminal's content-rect geometry is reliable at a fresh prompt AFTER a
  command (and at session start), but jitters after Ctrl-L." When the flag is
  clear it keeps the frozen (correct) geometry rather than trusting a fresh read.
- **`clear` and `tput clear` are special-cased** in `preexec` for the same
  reason — they wipe the screen like Ctrl-L, so their prompt is not a valid
  capture point.

Lori's own published support matrix shows what the residual AX dependency costs:
Ghostty "fully optimized," iTerm2/Terminal/kitty/Alacritty "best-effort,"
Cursor/Hyper/VS Code "completion works, positioning limited."

This is convergent with ADR-0008's original reasoning rather than contrary to
it. ADR-0008 correctly judged that Fig's positioning bugs were implementation
quality, not an architectural flaw in overlays. It was wrong only about which
input feeds the overlay: the overlay concept is sound, the AX-derived *caret*
is not.

## Decision

**The terminal reports its own cursor position via DSR (Device Status Report);
the Accessibility API is responsible only for the terminal window rect.**

### Positioning pipeline (revised)

```text
1. Shell widget queries cursor cell    ── ESC[6n → ESC[row;colR   (primary)
      │  at ZLE line-init, and after Ctrl-L
      │  feature-detected; disabled permanently if unanswered
      ▼
2. Daemon receives CUR{pid, row, col, cmd_ran}
      │
      ▼
3. Platform backend supplies window rect ── AX / X11 / Win32     (unchanged)
      │  re-captured only when cmd_ran is set
      ▼
4. positioning.rs: cell → pixel, edge detection, flip-above      (unchanged)
```

When step 1 yields nothing, step 2 is skipped and `positioning.rs` falls back to
the ADR-0008 pipeline: derive the caret from window rect + TIOCGWINSZ.

### Why DSR is safe to read at `line-init`

The risk with `ESC[6n` is racing the user's keystrokes — the reply arrives on
stdin and can be consumed by, or consume, real input. At `line-init` the prompt
has just been drawn and the buffer is empty, so there is no keystroke in flight.
A short read timeout (100ms) bounds the damage if the terminal never answers,
and the feature-detect latch means an unanswering terminal is probed exactly
once per session rather than once per prompt.

### Consequences for the platform backends

`OverlayBackend` keeps its window-rect responsibility on every platform. The
caret half of `PositioningError` becomes a fallback path rather than the
main one. Wayland benefits most: ADR-0008 noted Wayland "does not allow querying
other windows' positions by design" and required shell integration to report
coordinates — under this ADR that is no longer a Wayland special case, it is
what every platform does.

## Consequences

### Positive

- **Works on terminals we have not tuned for.** DSR is a VT100-era escape
  sequence; support is near-universal, including inside tmux and over SSH.
  Positioning stops being a per-terminal integration project.
- **Removes the largest source of overlay drift.** The caret comes from the
  component that actually owns it. No AX scrollback inflation, no content-rect
  jitter feeding into cell math.
- **Smaller Accessibility surface.** AX is queried per prompt for a window rect
  that changes rarely, not per keystroke for a caret that changes constantly.
- **Degrades rather than breaks.** A terminal that ignores DSR lands on the
  ADR-0008 pipeline, which is what we would have shipped anyway.

### Negative

- **Two positioning paths to maintain and test.** Mitigated by keeping the
  fallback identical to ADR-0008's pipeline rather than inventing a second one.
- **DSR requires `/dev/tty` access and a raw-ish read in the widget.** More
  shell-side complexity than reading `$COLUMNS`, and it is shell-specific code
  we now owe for bash and fish as well as zsh.
- **Ctrl-L still needs handling.** `line-init` does not fire on Ctrl-L, so the
  cached origin would stay at the old row. We must re-probe explicitly.
- **Accessibility permission is still required on macOS** for the window rect,
  so the onboarding prompt in ADR-0008 does not go away.

### Mitigations

- **Feature-detect once, latch off.** Never probe a terminal that ignored the
  first `ESC[6n`.
- **Bound the read.** 100ms timeout, character-at-a-time until `R`.
- **Gate geometry re-capture** on "a real command ran since the last prompt,"
  and treat screen-clearing commands as non-capture points. Freezing known-good
  geometry beats trusting a jittery fresh read.

## Alternatives Considered

### Keep AX-derived caret (ADR-0008 as written)

Rejected on the evidence above: the one shipping product using this approach
quietly abandoned it as the primary path, and its remaining AX dependency still
produces a tiered support matrix.

### PTY proxy with in-grid ANSI rendering (ghost-complete's approach)

[ghost-complete](https://github.com/StanMarek/ghost-complete) sits between
terminal and shell as a PTY proxy and renders popups as ANSI inside the grid, so
cursor position is known exactly and never queried. Its FAQ names AX drift —
"the kind of drift reported with tools like Amazon Q / Kiro" — as the reason.

Rejected because it re-adopts inline rendering, which ADR-0006 and ADR-0008
already rejected on UX grounds (grid-limited rendering, no true overlay), and
because a PTY proxy owns the entire I/O stream — a much larger blast radius than
a widget plus a daemon. DSR gets us the positional accuracy that motivates the
proxy without taking ownership of the data path.

### Terminal-specific cursor-position escape sequences

Some terminals expose richer position/geometry reporting than DSR. Rejected as a
primary mechanism for the same reason AX was: it makes positioning a per-terminal
integration. Worth revisiting only as an optional precision upgrade for terminals
where DSR's cell granularity proves insufficient.

## References

- ADR-0008: [Native Overlay Dropdown](0008-native-overlay-dropdown.md)
- Lori v0.2.2 shell integration — `Lori.app/Contents/Resources/Shell/lori.zsh`
  (inspected from the notarized DMG; `Info.plist` credits "Completion specs
  derived from Fig's autocomplete (MIT)")
- ghost-complete architecture: <https://github.com/StanMarek/ghost-complete>
- DSR / CPR: ECMA-48 §8.3.35, VT100 `ESC[6n`
- Tracking issue: `autocomplete-rs-17o`

## Erratum (2026-08-26)

The original text is preserved above; two of its characterizations of
ghost-complete were corrected by a code-level review of its v0.19.0 source
(see [fig-successor-cohort.md](../research/fig-successor-cohort.md)):

- "cursor position is known exactly and never queried" is wrong. ghost-complete
  dead-reckons the cursor from a VT parse of the output stream and reconciles it
  by emitting its own `ESC[6n` queries, arbitrated against other programs' DSRs
  through an owner-tagged FIFO queue (`crates/gc-parser/src/state.rs`). Its two
  shipped positioning bugs (#58, #64) came from that arbitration — field
  evidence that directly informs this ADR's implementation.
- Its FAQ's "no fragile shell internals to hook into" framing is contradicted
  by its shipped zsh plugin, which installs a `zle-line-pre-redraw` hook and
  reports `$BUFFER`/`$CURSOR` in a private OSC escape. The rejection rationale
  above (PTY proxy owns the entire I/O stream) stands; the premise that
  in-grid rendering removes the need for shell integration does not.

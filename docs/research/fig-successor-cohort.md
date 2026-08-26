# Fig-Successor Cohort: Deep Dive

> Compiled August 2026. Companion to
> [terminal-autocomplete-landscape.md](terminal-autocomplete-landscape.md) (February 2026),
> which covered Fig, inshellisense, Amazon Q/Kiro, carapace, and Warp. This document covers
> the 2025–2026 cohort of open-source successors: four projects that launched after Fig's
> sunset, three of which explicitly position against Fig or Amazon Q. Code-level findings
> cite `file:line` in each project's repository at the version noted per section; quotes
> come from those checkouts. Behavioral claims about these projects are inferred from
> reading their source, changelogs, and issue trackers — none of the four tools was
> executed for this research — and their performance numbers are their own reported
> measurements, not ours.

## Table of Contents

- [Executive Summary](#executive-summary)
- [1. Comparison Matrix](#1-comparison-matrix)
- [2. IRIS — PTY Wrapper](#2-iris--pty-wrapper)
- [3. deja — Hardened ZLE Plugin](#3-deja--hardened-zle-plugin)
- [4. flyline — In-Process Bash Plugin](#4-flyline--in-process-bash-plugin)
- [5. ghost-complete — PTY Proxy + zsh Plugin](#5-ghost-complete--pty-proxy--zsh-plugin)
- [6. Cross-Cutting Findings](#6-cross-cutting-findings)
- [7. Recommendations for autocomplete-rs](#7-recommendations-for-autocomplete-rs)

---

## Executive Summary

Four projects, four different architectures — and none of them is ours. Each picked a more
invasive integration point than a daemon plus a thin widget, and each pays a characteristic
bug tax for it:

| Architecture | Project | Characteristic failure mode |
| --- | --- | --- |
| PTY wrapper (owns the tty) | IRIS | Bricked terminals (p10k), corrupted readline state, drawing over other TUIs |
| ZLE widget-wrapping | deja | Wedged line editor from wrapping `zle -C` widgets; a per-prompt re-bind arms race |
| In-process cdylib in bash | flyline | Allocator corruption from bash's non-thread-safe `malloc`; fork-per-completion as the only safe concurrency |
| PTY proxy + zsh plugin | ghost-complete | Proxy hang freezes the entire terminal (Ghostty, issue #131) |

Three convergent lessons:

1. **Nobody made byte-stream buffer reconstruction work.** IRIS keylogs and patches its
   guess with `$LBUFFER` pushes; ghost-complete's README claims "no shell internals" while
   its code installs a `zle-line-pre-redraw` hook that reports `$BUFFER`/`$CURSOR` per
   redraw. Every project that needs the buffer ends up asking the shell for it. Our
   protocol does this from the start.
2. **DSR/cursor-position queries on a shared tty need ownership arbitration.** ghost-complete
   shipped two production bugs (atuin broken, z4h hanging 5 s) from eating CPR replies meant
   for other programs; flyline documents that a crash between `ESC[6n` and the reply garbles
   the terminal. This lands directly on ADR-0009.
3. **Ranking has converged on frecency + context signals; the spec corpus has not
   converged at all.** Approaches range from ~520 hand-ported Go files (IRIS) to 711
   lazy-loaded Fig JSON specs (ghost-complete) to "inherit bash-completion and synthesize
   from `--help`" (flyline). ghost-complete's lazy-loading numbers (eager parse of the Fig
   corpus ≈ 333 MB RSS; lazy ≈ 5 MB) are the single most actionable data point for our
   spec phase.

None of the four uses a native overlay window, and none uses the Accessibility API.
ghost-complete names AX drift ("the kind of drift reported with tools like Amazon Q /
Kiro") as its reason for staying in-grid — the same evidence base as ADR-0009, resolved
differently: they took over the I/O stream to know the cursor; we ask the terminal via DSR
and keep the stream untouched.

## 1. Comparison Matrix

Star counts and activity observed 2026-08-26 via `gh repo view`.

| | IRIS | deja | flyline | ghost-complete |
| --- | --- | --- | --- | --- |
| Language | Go | Go | Rust | Rust |
| Stars | 1,241 | 597 | 1,079 | 276 |
| Created | 2026-04 | 2026-04 | 2025-10 | 2026-03 |
| Status | very active | active | very active | quiet since 2026-07 |
| Architecture | PTY wrapper (`exec`s over the shell) | daemon + ZLE plugin | cdylib inside bash | PTY proxy + zsh plugin |
| UI | inline ANSI box + ghost text | ghost text (`POSTDISPLAY`) | inline ratatui viewport | inline ANSI popup |
| Cursor position | inferred by parsing shell output | n/a (ZLE renders) | DSR `ESC[6n` (CPR) | VT dead-reckoning + CPR reconcile |
| Completion source | ~571 compiled-in specs in Go (Fig-derived) + cobra probing + AI | shell history only | bash compspecs + `--help` synthesis + AI agents | 711 embedded Fig JSON specs + providers |
| Ranking | 5-signal blend, transitions, cwd frecency | 4-signal blend (fuzzy/frecency/dir/sequence) | fuzzy only, no frecency | nucleo fuzzy + tiered priority + scoped frecency |
| Shells | zsh, bash, fish | zsh only | bash only | zsh (bash/fish manual-trigger) |
| Platforms | macOS/Linux | macOS/Linux | Linux/macOS | macOS only |

Notable non-overlap: deja and flyline never mention Fig/Amazon Q/Kiro at all (verified by
grep); they position against zsh-autosuggestions/atuin and ble.sh/inshellisense
respectively. Only IRIS and ghost-complete are Fig successors in intent.

## 2. IRIS — PTY Wrapper

<https://github.com/versenilvis/IRIS> · Go, 0BSD, ~43.6k lines (tests included), v0.6.x
(observed at 2026-08-24 push).

**Model:** the rc file literally `exec`s iris over the user's shell (`root/init.go:36-39`);
iris respawns the real shell inside a PTY it owns, holds the outer terminal in raw mode,
and interposes on every keystroke. A watchdog parent provides crash recovery and a rescue
shell (`root/root.go:96-180`). "No background daemons" (README) is technically true —
iris is a *foreground* man-in-the-middle instead.

**Positioning without DSR:** zero occurrences of `ESC[6n`. Cursor position is inferred by
re-parsing the shell's own output — simulating `\r`, backspace, tab stops, CUF/CUB/CHA,
skipping OSC/DCS (`integration/overlay.go:91-197`). Repaint racing is the dominant
rendering problem: a 12 ms "pty quiet" settle timer capped at 150 ms, plus an echo-marker
scheme that waits for the shell to echo back the last 12 characters before drawing
(`root/wrapper.go:160-189,449-478`). Their positioning bug tail (multiplexer ghost smear
\#121, CJK misrendering #106) is the cost of never asking the terminal where the cursor is.

**Buffer state:** a byte-level `naiveBuffer` guess (ASCII printables only,
`root/wrapper.go:1384-1505`), corrected by a three-line zsh hook that pushes `$LBUFFER`
over an inherited pipe fd on every ZLE redraw (`root/init.go:42-68`). Bash gets no such
hook — pure keylogging plus `TIOCGPGRP` polling.

**Completions:** ~520 hand-written Go spec files registering ~571 specs, compiled into
the binary, structurally
derived from Fig's corpus (they ported Fig's own `create-completion-spec` spec —
`commands/js/create_completion_spec.go:8-18`) with no visible MIT attribution. Plus:
cobra `__complete` probing for unspecced Go binaries — statically verified via
`debug/buildinfo`, run under `Setsid` with a 300 ms timeout, cached by binary mtime
(`spec/cobra_complete.go:87-173`); shell aliases (rescanned every keystroke — an
anti-pattern they haven't fixed); atuin/zoxide databases; optional AI ghost text
(Groq/Ollama, 500 ms debounce).

**Ranking — the most developed of the four:**
`0.30·priority + 0.25·context + 0.15·frecency + 0.10·transition + 0.20·match_quality`
(`internal/scoring/scorer.go:33-39`), with:

- **cwd-scoped frecency** (UNIQUE(cmd,cwd)) falling back to global × 0.7, step buckets
  100/50/20/5/1 by age (their dev docs claim exponential decay; the code disagrees)
- **workflow transitions:** a `command_transitions` table of (prev-skeleton →
  next-skeleton, cwd) pairs, where a skeleton is the command stripped to `git commit`
  form; the previous command boosts likely followers (`scoring/skeleton.go`,
  `wrapper.go:744-763`)
- **workspace context rules:** in a git repo `git status/diff/...` +40, mentions of the
  current branch +60, `git init` inside a repo −50 (`scoring/context_rules.go:27-80`)
- a transparent `ScoreBreakdown` struct keeping per-signal components inspectable

**Parsing:** one quote-aware split loop with no operator handling at all — iris cannot
complete the right-hand side of a pipe or after `&&` (`spec/utils.go:9-44`). Our FSM
tokenizer is strictly ahead here.

**Bug tax:** the tracker is a catalog of PTY-interposition failure modes — p10k's instant
prompt spawning three iris processes and killing the terminal (#137), Warp's bootstrap
heredoc torn by raw mode (#100, open), the overlay painting over atuin's Ctrl-R UI
(#138), arrow keys dying in bash after Ctrl-Left because byte-forwarding corrupted
readline state (#114, open), input duplication in nvim's terminal (#109, open). Every
terminal, prompt framework, and TUI is a new compatibility cell patched one bug report at
a time.

## 3. deja — Hardened ZLE Plugin

<https://github.com/Giammarco-Ferranti/deja> · Go, MIT, ~7.4k lines Go (tests included) +
a 1,237-line zsh script (observed at 2026-08-10 push).

**Scope:** not a Fig-style completer — a zsh-autosuggestions replacement (history ghost
text) with a Go daemon. No specs, no tokenizer, no dropdown. Its value to us is
shell-integration and daemon craft, which is the most polished of the cohort.

**IPC:** two request encodings share one Unix socket, disambiguated by peeking one byte —
`{` routes to JSON, anything else to a line-oriented text protocol
(`internal/daemon/server.go:152-169`). The text protocol exists so **zsh can talk to the
daemon directly via `zmodload zsh/net/socket` — no fork/exec per keystroke**: newline
framing, 0x1F field separators, three escapes with strict rejection of unknown ones,
respond-then-close because "zsh cannot half-close a socket to signal EOF"
(`internal/daemon/text.go:1-54`). This took their keystroke path from 29.6 ms to 2.9 ms
(issue #80, a measured overhaul).

**Discipline on the keystroke path:** every ZLE socket read is bounded (`read -t 0.5`) so
a wedged daemon degrades to "no suggestion," never a frozen line editor
(`internal/shell/zsh.sh:415-435`); history recording is fire-and-forget with the ack
deliberately unread (`zsh.sh:997-1003`); paste bursts skip fetches entirely via
`$PENDING`/`$KEYS_QUEUED_COUNT` (`zsh.sh:493-503`).

**Ranking:** `1.0·fuzzy + 0.4·frecency + 0.3·dir_affinity + 0.5·sequence`
(`internal/scorer/scorer.go:108-111`). The fuzzy fix worth copying: subsequence match plus
a **cap on the maximum gap between matched characters** — their loose/smart/tight presets
are just gap caps 8/4/1, which fixed "fuzzy matching is too broad" (#55). Empty buffer
sets fuzzy to a constant, so frecency+dir+sequence predict the *next* command at a fresh
prompt. exit_code and duration are recorded but never scored — latent signal nobody in
this cohort uses.

**Operational craft:**

- stale-socket handling by probe-then-reclaim: on bind failure, ping the existing socket
  with a 50 ms timeout; only remove and rebind if it doesn't answer
  (`internal/daemon/server.go:40-53,98-114`) — more robust than a pidfile alone
- version-skew plan for a long-lived daemon: cached shell script stamped with the
  generating binary's size-mtime-inode, zstat-compared per shell (0.083 ms, no fork),
  regenerated in the background for the *next* shell (`zsh.sh:1197-1225`); an old daemon
  behind a live socket is detected by protocol probe and degrades to the slow path
- privacy filtering **in the widget**: `HIST_IGNORE_SPACE`/`HISTORY_IGNORE` re-implemented
  in `preexec` so secrets never enter another process's argv or `ps`, with the
  prev-command chain broken too, plus a server-side backstop (`zsh.sh:948-979`)
- SQLite: `SetMaxOpenConns(1)` to serialize, WAL with a 10-minute TRUNCATE-checkpoint loop
  (one install held a 28 MB WAL with zero live frames), DB tightened to 0600 *before*
  first write so WAL/SHM sidecars inherit the mode (`internal/store/store.go:84-120`)

**Bug tax:** their worst bugs are pure ZLE-integration wounds — wrapping `zle -C`
completion widgets wedged the terminal outright (#46/#47); zsh corrupts `region_highlight`
entries extending into `POSTDISPLAY` (fixed with zsh 5.9 `memo=` tags plus a
`zle-line-pre-redraw` repair hook that treats ghost text as derived state); an
unconditional `zmodload zsh/stat` shadowed `/usr/bin/stat` and broke user scripts (#100,
open). deja wraps ~600 widgets and re-verifies bindings every prompt to survive framework
churn — the clearest evidence in the cohort that rendering *outside* ZLE (our overlay) is
the right call, and that our widget should stay minimal.

## 4. flyline — In-Process Bash Plugin

<https://github.com/HalFrgrd/flyline> · Rust, ~50k lines (all crates, tests included;
~26k non-test under `src/`), v1.7.1. Source MIT, distributed binaries GPLv3 (they link
bash symbols).

**Model:** a cdylib loaded into bash's own process (`enable -f libflyline.so flyline`)
that rewrites bash's `bash_input` reader function pointers and replaces readline entirely
(`src/lib.rs:316-503`). Raw FFI against bash internals (`rl_line_buffer`,
`programmable_completions`, `evalstring`, … — 885 lines of extern declarations), all
serialized behind a process-wide reentrant lock.

**The tar pit, on record:** bash's `malloc` shadows glibc's process-wide and is not
thread-safe, so even glibc's own thread-exit hooks corrupted the allocator when flyline
used background threads (issue #891 — "Flyline can't control what glibc calls!"). The fix
was to **fork() the entire bash process per completion**: the child inherits a
copy-on-write snapshot of all bash state, runs the user's compspec functions safely,
returns results over a msgpack pipe, and is SIGKILLed by `Drop` when superseded
(`src/subshell_ipc.rs:232-356`). Plus mimalloc as global allocator, SIGCHLD guards, and
`catch_unwind` on every C callback so a panic degrades to EOF instead of killing the
user's shell (`src/lib.rs:95-152`).

**DSR in production — the cohort's closest prior art for ADR-0009:** flyline anchors its
inline viewport by sending `ESC[6n` and reading the CPR reply. Hard-won rules:

- **defer terminal interrogation at startup** — DA2/CPR/focus-tracking queries at t=0
  break some terminals and slow SSH links, so they sit behind a configurable delay
  (`src/app/mod.rs:1867-1893`)
- **debounce CPR after resize** — 150 ms, added after resize storms on Termux caused
  repeated screen resets (#889; `mod.rs:778-787`, "Getting cursor pos can be slow via ssh")
- **a crash between sending `ESC[6n` and reading the reply leaves the CPR echoed as
  garbage in the terminal** (#891 discussion) — the request/response pair must be
  crash-safe and bounded

**Perceived latency without debounce:** suggestions restart on every keystroke; the stale
fork is killed by dropping its handle, and a `Keep / Restart{carry_over} / Discard /
Update` state machine refines the visible list in place for ±1-character edits, carrying
old suggestions as placeholders so the popup never blanks (`src/app/mod.rs:2160-2340`).
Measured completion latency is displayed *in the popup itself* (`N.Nms`,
`src/app/ui.rs:1799-1802`). This is the best perceived-latency model in the cohort.

**Completions — generate, don't curate:** no spec corpus. Bash's own programmable
completion runs in the fork (the whole bash-completion ecosystem for free); when the
compspec falls through to the useless filename default (`compspec_was_useful`,
`src/shell/bash/funcs.rs:640-642`), flyline offers to *synthesize* a compspec by parsing
the command's `--help`/man output (their `flycomp` crate). Descriptions ride a zero-schema
convention: tab-separated suffixes on completion strings, ANSI allowed. Also a pluggable
AI agent mode (claude/copilot/codex CLIs returning JSON suggestion arrays).

**Ranking:** forked-skim fuzzy only. **No frecency anywhere** — the inline suggestion is
literally the most recent prefix match; exit status, cwd, and duration are captured in a
JSONL Start/End event log (`src/history/backend.rs:13-38`) but never used for ranking. An
open gap the whole cohort leaves us.

**Other techniques:** DA2-based terminal fingerprinting ported from ble.sh
(kitty/ghostty/wezterm/zellij/screen/foot/VTE/xterm by numeric params,
`src/term_info.rs:300-380`); DEC 2026 synchronized-output around every draw; a docker test
matrix against real bash 3.2.57→5.3.

## 5. ghost-complete — PTY Proxy + zsh Plugin

<https://github.com/StanMarek/ghost-complete> · Rust, MIT, 9-crate workspace, ~97k lines
(tests included), v0.19.0 (last real commit 2026-07-05; quiet since). macOS-only, zsh-primary. Already cited
by ADR-0009; the deep dive corrects the record and yields the most transferable mechanisms.

**Their ADR-0001 is falsified by the code.** The README FAQ claims "no zle widget
conflicts … no fragile shell internals to hook into," and their ADR-0001 goes further:
"We do not hook `zle-line-pre-redraw`." The shipped plugin installs exactly that hook
(`shell/ghost-complete.zsh:219-258`), with `$WIDGET`-preserving chaining added after their
own issue #64 (z4h hung 5 s, p10k RPROMPT broke). More importantly, **the command buffer
is not reconstructed from the byte stream**: zsh reports `$BUFFER`/`$CURSOR` after every
redraw via a private OSC 7772 escape, percent-encoded against a strict allow-list because
unencoded `;`/BEL/ESC corrupt the OSC envelope, with a proptest suite guarding the
round-trip. Prompt boundaries come from shell-emitted OSC 133; cwd from OSC 7; an env
snapshot rides OSC 7773 each prompt (stripped from the stream before the terminal sees
it — exported credentials transit the output stream, with no credential filter: the
budget deliberately *prioritizes* `AWS_`/`GITHUB_`-prefixed variables. A design we
should not copy).
Consequence: bash and fish get manual trigger only. The honest architecture is *PTY proxy
for rendering + input interception, zsh plugin for state* — when ADR-0009 cites
ghost-complete, cite the code, not the FAQ.

**CPR ownership arbitration — the mechanism ADR-0009 must inherit:** the proxy emits
`ESC[6n` to reconcile its dead-reckoned cursor, but atuin, z4h, and crossterm apps issue
their own DSRs on the same tty, and terminals answer in order. Two production bugs (#58:
atuin "cursor position could not be read"; #64: z4h Ctrl-L hung) were the proxy eating
replies meant for others. The fix is an **owner-tagged CPR FIFO queue**
(`crates/gc-parser/src/state.rs:61-66`): every outstanding `6n` is tagged `Ours` or
`Shell`; each arriving reply dispatches as sync-ours / forward-to-pty / defensive-forward,
with rollback tokens for failed writes and a 30 s stale prune
(`crates/gc-pty/src/proxy.rs:364-505,640-679`). Two adjacent races they also close:
positioning is suppressed after a buffer report **until the next display-changing byte**
proves the ZLE redraw landed (`state.rs:99-106`), and every popup render passes an
epoch/ticket check so a frame computed before newer output can't land late (issue #23's
artifact class).

**Spec corpus at scale — the numbers we need** (all self-reported in their docs): 711
Fig-converted JSON specs embedded zstd-19 (47 MB JSON → 3.7 MB archive; binary 103 MB →
11.8 MB). They report eager parsing cost ≈ 333 MB RSS (the AWS spec alone is ~36 MB
minified with ~17k subcommands); lazy registration ~183 µs and ~5 MB heap (their
changelog says ~2 MB — their own docs disagree), with sticky parse failures, an alias
index built at registration, `Arc<CompletionSpec>` returns to permit idle eviction, and
~150 ms worst-case first touch. Generators come in five classes — Rust-native, templates
(`filepaths`/`folders`), script generators with a declarative transform pipeline
(`split_lines`, `regex_extract`, `json_extract`, …), script templates
(`{current_token}` substitution), and a bounded QuickJS sandbox
(rquickjs; memory/stack/time caps; kill switch) — and their Fig converter *lowers* simple
`postProcess` JS bodies into native transforms so most specs never touch JS. This
validates and refines the QuickJS plan in
[terminal-autocomplete-landscape.md §8.4](terminal-autocomplete-landscape.md).

**Ranking:** nucleo (Helix's matcher; their architecture docs report ~6× faster than skim, <1 ms at
10k candidates); sort key `(non-history-first, fuzzy desc, priority desc, alpha)` with
per-kind base priorities (GitBranch 80 … History 10); frecency as a single decayed scalar
(72 h half-life, decay-then-add-1, ceiling against clock jumps, versioned store) scoped by
`(command, kind, text)` so `--help` under git doesn't boost docker; history
hard-partitioned below everything else regardless of boost.

**Bug tax:** worst case on record — v0.16.0 froze Ghostty entirely (#131, open): when a
proxy in the critical path hangs, the terminal is unusable, not merely uncompleted. Also
shell-script/binary version skew producing a popup on alternate keystrokes (#104/#107),
and a per-terminal capability matrix (a whole crate, profiles, unknown terminals
refused by default behind an opt-in `multi_terminal` override) that Linux/Windows
support would multiply.

## 6. Cross-Cutting Findings

### 6.1 Architecture: the cohort validates ours by counterexample

Every project chose an integration point that owns more of the terminal than ours does,
and each one's worst bugs come precisely from that ownership: IRIS #137 (terminal dead),
flyline #891 (allocator corruption), ghost-complete #131 (terminal frozen), deja #46/#47
(line editor wedged). Our *intended* failure mode is "widget times out, no suggestions,
shell untouched" — the daemon is not in the keystroke-delivery path. Today that property
is only half-built: the daemon side has 1 s read/write timeouts (`src/daemon/handler.rs`),
but the widget's `$(autocomplete-rs complete …)` call and the client's `read_line` are
both unbounded, so a daemon that accepts and then wedges can still block the widget —
closing that gap belongs to R4. Once closed, the property is a genuine differentiator;
every design decision that would put the daemon or overlay into the critical path should
be measured against it.

The flip side: their invasiveness buys capabilities we must earn differently. flyline
inherits every bash compspec; ghost-complete knows the cursor cell exactly; IRIS works
over SSH and in a Linux VT. Our answers are, respectively, generators/specs, DSR
(ADR-0009), and — honestly — nothing built yet: **SSH/tmux is the one axis where the entire
in-grid cohort beats a native overlay**, since our window can only track a local
terminal. Worth stating as an accepted limitation rather than discovering it later.
The credible degradation is in-ZLE rendering (ghost text via `POSTDISPLAY`, deja's
mechanism) when no local display exists — from inside the widget, dodging the
redraw-fighting the out-of-shell cohort pays for — not a PTY proxy.

### 6.2 DSR positioning: prior art converges on four rules

Between flyline (DSR in production), ghost-complete (CPR arbitration), and Lori (ADR-0009's
original evidence), the field data for our `ESC[6n` implementation is:

1. **Arbitrate ownership.** z4h, p10k, atuin, and any crossterm app issue DSRs on the same
   tty; replies arrive in FIFO order. Reading "the next CPR" without tracking who asked
   breaks real setups (ghost-complete #58/#64). Our widget must bound its wait, and drain
   or re-emit replies that aren't ours. Test explicitly against p10k, z4h, and atuin.
2. **Never leave a DSR outstanding across a failure path.** A crash between request and
   reply leaves the CPR echoed as garbage at the prompt (flyline #891 discussion). Timeout
   plus drain on every exit path.
3. **Defer and debounce.** Interrogating the terminal at t=0 breaks some emulators and
   slow SSH links (flyline's delayed startup); resize storms need a debounce before
   re-querying (flyline's 150 ms after Termux resize storms, #889).
4. **Don't anchor to stale geometry.** ghost-complete suppresses positioning after a
   buffer change until output proves the redraw landed (`buffer_pending_display`); Lori
   gates geometry re-capture on "a command actually ran." Same race, two mitigations —
   ADR-0009's `cmd_ran` flag covers the Lori half; the redraw-landed half is new input for
   implementation.

### 6.3 Completion data: curate lazily, synthesize as fallback

- **Embedded Fig corpus works at 711-spec scale only if lazy** (ghost-complete reports
  333 MB eager vs ~5 MB lazy; zstd-19 embed). Compiling specs into code (IRIS's ~520 Go
  files)
  couples every new tool to a binary release — our runtime-JSON plan is right.
- **Lower JS before sandboxing it.** ghost-complete converts simple Fig `postProcess`
  bodies to declarative native transforms; QuickJS is the escape hatch, not the default.
- **Synthesis fills the long tail.** flyline detects a useless compspec fallback and
  offers to parse `--help`/man into a spec; IRIS probes cobra binaries with
  `__complete` under `Setsid` with a 300 ms timeout, statically gated by
  `debug/buildinfo`. Both are cheap coverage multipliers a curated corpus can't match.
- **Local-project providers are high-leverage:** cargo workspace members/targets, npm
  scripts, Makefile targets (ghost-complete) — spec-independent completions for the
  commands developers actually type all day.

### 6.4 Ranking: a convergent recipe, with one open gap

Three of four converge on *linear blend of fuzzy match + frecency + context signals, with
deterministic tie-breaks*:

- fuzzy with a **max-gap cap** beats raw subsequence scoring (deja #55)
- frecency as a **single decayed scalar** (ghost-complete: 72 h half-life,
  decay-then-add) or step buckets (IRIS) — both simpler than storing event windows
- **scope frecency** by cwd (IRIS, deja) and by `(command, kind, text)` (ghost-complete)
  so boosts don't leak across commands
- **command-sequence signals** are the differentiator: deja's `prev→next` counts and
  IRIS's skeleton transitions both predict the *next command at an empty prompt* — a
  feature Fig never had
- **tier before boost:** ghost-complete hard-partitions history below spec results no
  matter the score — prevents frecency from drowning correctness

The open gap: **everyone records exit codes; nobody ranks on them.** deja and flyline both
capture exit status and duration and use neither. Down-weighting commands that failed in
this cwd is an obvious, cheap differentiator.

### 6.5 Shell-integration invariants (the widget contract)

From deja's craft plus ghost-complete's skew bugs, invariants our `zsh.zsh` should adopt:

- every read on the keystroke path is bounded; recording is fire-and-forget
- privacy filtering (`HIST_IGNORE_SPACE`, `HISTORY_IGNORE`) runs in the widget so secrets
  never leave the shell process, with a daemon-side backstop — before we ever record history
- version-stamp the widget/daemon protocol and detect skew loudly (ghost-complete
  #104/#107 was silent skew; deja stat-stamps the generated script and probes the daemon)
- minimal widget footprint: no `zmodload` side effects (deja #100), never touch
  `zle -C` completion widgets (deja #46/#47), chain rather than replace existing hooks
  preserving `$WIDGET` (ghost-complete #64)
- socket reclaim by probe-then-remove complements our pidfile (deja's 50 ms ping)

### 6.6 Perceived latency

flyline's model — no debounce, cancel superseded work by drop, carry the previous list as
placeholder during ±1-char refinements, show measured latency in the UI — maps onto our
architecture as planned work (R5): a per-request cancellation generation in the daemon
(today `DaemonState` holds a single daemon-wide `CancellationToken`) and an overlay that
keeps the stale list until fresh results arrive. deja's numbers show where shell-side
milliseconds actually go: fork/exec per invocation (~27 ms) dwarfs everything else, and
our client does not avoid it at all today — the widget forks the `autocomplete-rs` binary
on every trigger. A `zsocket`-style no-fork client path would need a line-oriented
sibling encoding on our socket (deja's one-byte-peek dual protocol shows both can
coexist).

## 7. Recommendations for autocomplete-rs

Recommendations, not decisions — each needs explicit confirmation before it becomes
tracked work. Ordered by leverage against current phase (spec system + ADR-0009
implementation are the active fronts).

| # | Recommendation | Evidence | Lands in |
| --- | --- | --- | --- |
| R1 | Fold the four DSR field rules (§6.2: ownership arbitration, crash-safe request/reply, defer+debounce, redraw-landed gating) into the ADR-0009 implementation (`autocomplete-rs-17o`); test against p10k/z4h/atuin | ghost-complete #58/#64, flyline #889/#891 | shell-integration, daemon |
| R2 | Spec system: lazy registration + alias index + sticky failures from day one; zstd-embedded corpus; lower Fig `postProcess` to native transforms, QuickJS only as escape hatch | ghost-complete numbers (333 MB→5 MB; 47 MB→3.7 MB) | parser/engine |
| R3 | Ranking layer: linear blend with max-gap-capped fuzzy, single-scalar decayed frecency scoped (cwd, command, kind), sequence/transition signal, hard tiering above history — and rank on recorded exit codes (the gap nobody fills) | deja `scorer.go`, IRIS transitions, ghost-complete `frecency.rs` | engine + storage |
| R4 | Widget contract hardening (§6.5): bounded reads, fire-and-forget records, widget-side privacy filtering, protocol version stamp with loud skew detection, no-`zmodload` rule — codify in `docs/conventions/shell-integration.md` | deja #46/#47/#77/#100, ghost-complete #104/#107 | shell-integration |
| R5 | Perceived-latency model: no debounce; per-request cancellation generation in the daemon; overlay carries over the stale list during refinement; optional latency readout in the overlay | flyline `mod.rs:2160-2340`, `ui.rs:1799` | daemon, overlay |
| R6 | Second-tier generators: `--help`/man synthesis fallback and cobra `__complete` probing (buildinfo-gated, `Setsid`, 300 ms timeout, mtime-keyed cache); local-project providers (cargo/npm/make) | flyline flycomp, IRIS `cobra_complete.go`, ghost-complete providers | engine |
| R7 | Storage: record Start/End command events with exit status/duration/cwd/session (we have the schema seams); verify turso WAL sidecar permissions and checkpoint behavior against deja's findings | flyline `backend.rs`, deja #77 + checkpoint loop | storage |
| R8 | Docs hygiene: state the SSH/tmux limitation of a native overlay as an overlay limitation with a planned in-ZLE fallback (§6.1); ADR-0009 says ghost-complete's cursor position is "known exactly and never queried," which §5 disproves (VT dead-reckoning + CPR reconciliation) — correct via an amending note or superseding ADR, not an in-place edit, since accepted ADRs are immutable | §5, §6.1 | docs/adr |

### Priority read

R1 is on the critical path (ADR-0009 implementation is already tracked as
`autocomplete-rs-17o`) and cheap to fold in now versus painful to retrofit. R2 decides
the shape of the spec system before its first line is written — the one-way door in this
list. R3–R7 are additive and can each become an independently scheduled issue. R8 is an
hour of editing.

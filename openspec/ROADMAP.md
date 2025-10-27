# autocomplete-rs Roadmap

## Priority & Phases

This document tracks the priority and dependencies of all planned changes.

## Phase 1: MVP (Minimum Viable Product)

**Goal:** Working end-to-end autocomplete for git command **Timeline:** Weeks
1-2 **Blockers:** None

### Priority 1A: Foundation Architecture

**Change:** `add-foundation-architecture` **Why First:** Provides core
infrastructure (daemon, TUI, shell integration) needed by all other features
**Dependencies:** None **Status:** Planned

### Priority 1B: MVP Parser

**Change:** `implement-mvp-parser` **Why Second:** Implements basic parsing with
hardcoded git spec to prove the system works **Dependencies:**
`add-foundation-architecture` (needs daemon + TUI) **Status:** Planned

**Phase 1 Complete When:**

- ✅ Daemon running and responding to requests
- ✅ TUI displays suggestions and handles keyboard navigation
- ✅ Zsh widget triggers completions
- ✅ `git che<TAB>` shows `checkout` in dropdown
- ✅ End-to-end latency < 20ms

---

## Phase 2: Scale to 600+ CLI Tools

**Goal:** Support all Fig completion specs **Timeline:** Weeks 3-4 **Blockers:**
Phase 1 complete

### Priority 2: Fig Spec Parser

**Change:** `add-fig-spec-parser` **Why Third:** Unlocks 600+ CLI tools by
parsing existing Fig specs **Dependencies:** `implement-mvp-parser` (replaces
hardcoded specs) **Status:** Planned

**Phase 2 Complete When:**

- ✅ Build system parses TypeScript specs
- ✅ Specs compiled to MessagePack and embedded
- ✅ Runtime loads specs efficiently
- ✅ Top 20 CLIs work (git, npm, docker, cargo, etc.)
- ✅ Binary size < 15MB with all specs

---

## Phase 3: Polish & User Experience

**Goal:** Beautiful, customizable UI **Timeline:** Week 5 **Blockers:** Phase 2
complete

### Priority 3: Theme Support

**Change:** `add-theme-support` **Why Fourth:** Improves user experience, makes
tool feel native **Dependencies:** `add-foundation-architecture` (modifies TUI)
**Status:** Planned

**Phase 3 Complete When:**

- ✅ Catppuccin themes available (all 4 variants)
- ✅ Theme config file works
- ✅ Runtime theme switching works
- ✅ Screenshots in README showcase themes

---

## Phase 4: Universal Shell Support

**Goal:** Work on all major shells **Timeline:** Weeks 6-7 **Blockers:** Phase 3
complete (feature complete for zsh first)

### Priority 4: Multi-Shell Support

**Change:** `add-multi-shell-support` **Why Fifth:** Expands user base, true
"universal" autocomplete **Dependencies:** All previous phases (core system must
be solid) **Status:** Planned

**Phase 4 Complete When:**

- ✅ Bash integration works
- ✅ Fish integration works
- ✅ Installation command supports all shells
- ✅ Documentation covers all shells
- ✅ All three shells can run concurrently on same system

---

## Dependency Graph

```text
Phase 1 (MVP):
  add-foundation-architecture (Priority 1A)
         ↓
  implement-mvp-parser (Priority 1B)

Phase 2 (Scale):
         ↓
  add-fig-spec-parser (Priority 2)

Phase 3 (Polish):
         ↓
  add-theme-support (Priority 3)

Phase 4 (Universal):
         ↓
  add-multi-shell-support (Priority 4)
```

## Implementation Strategy

### Week-by-Week Plan

**Week 1-2: Phase 1 (MVP)**

1. Implement daemon with Unix socket
2. Build Ratatui TUI with dropdown
3. Create zsh ZLE widget
4. Implement parser with tokenization
5. Add hardcoded git spec
6. Test end-to-end
7. Measure performance (<20ms total)

**Week 3-4: Phase 2 (Scale)**

1. Set up withfig/autocomplete submodule
2. Implement TypeScript parser with deno_ast
3. Generate Rust structs from parsed specs
4. Compile to MessagePack
5. Implement spec loader with caching
6. Test top 20 CLIs
7. Optimize binary size

**Week 5: Phase 3 (Polish)**

1. Implement Theme struct and registry
2. Add Catppuccin variants (4 themes)
3. Create config file system
4. Implement runtime theme switching
5. Update TUI to use themes
6. Take screenshots for README

**Week 6-7: Phase 4 (Universal)**

1. Research bash readline
2. Implement bash integration
3. Research fish completions
4. Implement fish integration
5. Update install command
6. Test all shells
7. Document everything

## Success Metrics

### Phase 1

- Completion latency: < 20ms
- Daemon memory usage: < 50MB
- Works in all major terminals

### Phase 2

- Support: 600+ CLI tools
- Binary size: < 15MB
- Spec load time: < 1ms

### Phase 3

- Themes: 4+ included
- Config reload: < 100ms
- User satisfaction: Positive feedback on appearance

### Phase 4

- Shell support: zsh, bash, fish
- Installation: 1-command for all shells
- Cross-shell: All work concurrently

## Future Considerations (Post-Phase 4)

### Potential Phase 5: Advanced Features

- **Dynamic generators**: Run shell commands for dynamic completions
- **Fuzzy matching**: Better partial match algorithm
- **History integration**: Learn from command history
- **PowerShell support**: Windows support
- **Plugin system**: User-defined completion specs

### Potential Phase 6: Performance & Scale

- **Incremental parsing**: Only parse changed specs
- **Distributed cache**: Share compiled specs across machines
- **WebAssembly**: Spec execution in WASM for safety
- **Language server protocol**: IDE integration

## Notes

- Each phase builds on previous phase
- Can't skip phases without losing features
- Testing is part of each phase, not separate
- Documentation updates happen in each phase
- Performance validation happens at end of each phase

## Current Status

- Phase 1: **Not Started** (planned)
- Phase 2: **Not Started** (blocked by Phase 1)
- Phase 3: **Not Started** (blocked by Phase 2)
- Phase 4: **Not Started** (blocked by Phase 3)

---

Last Updated: 2025-10-25

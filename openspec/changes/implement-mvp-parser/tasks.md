# Implementation Tasks

## 1. Parser Foundation

- [ ] 1.1 Implement command buffer tokenizer (split on whitespace, handle
      quotes)
- [ ] 1.2 Create token types (Command, Subcommand, Option, Argument)
- [ ] 1.3 Build parser state machine to determine completion context
- [ ] 1.4 Add cursor position tracking to determine what to complete
- [ ] 1.5 Write unit tests for tokenizer

## 2. Git Completion Spec

- [ ] 2.1 Create `src/specs/mod.rs` with CompletionSpec trait
- [ ] 2.2 Implement hardcoded git spec in `src/specs/git.rs`
  - [ ] Basic commands (checkout, commit, add, push, pull, status)
  - [ ] Common options (-m, -b, --amend, etc.)
  - [ ] Subcommand matching
- [ ] 2.3 Write unit tests for git spec matching

## 3. Parser-Spec Integration

- [ ] 3.1 Implement spec lookup by command name
- [ ] 3.2 Create completion generator that matches tokens against spec
- [ ] 3.3 Return ranked suggestions based on current context
- [ ] 3.4 Handle partial matches and fuzzy filtering
- [ ] 3.5 Add integration tests for parser + git spec

## 4. Daemon Integration

- [ ] 4.1 Define message format for daemon requests/responses (JSON)
- [ ] 4.2 Implement daemon request handler
  - [ ] Receive buffer + cursor position
  - [ ] Call parser to get suggestions
  - [ ] Return JSON response with suggestions
- [ ] 4.3 Add error handling for malformed requests
- [ ] 4.4 Test daemon end-to-end with mock Unix socket client

## 5. TUI Integration

- [ ] 5.1 Wire TUI to receive suggestions from daemon response
- [ ] 5.2 Test TUI rendering with sample git completions
- [ ] 5.3 Verify keyboard navigation works

## 6. End-to-End Testing

- [ ] 6.1 Test with zsh widget: `git che[TAB]` → shows `checkout`
- [ ] 6.2 Test with zsh widget: `git commit -[TAB]` → shows `-m`, `--amend`
- [ ] 6.3 Verify TUI positioning and selection
- [ ] 6.4 Measure and verify performance (<20ms total)

## 7. Documentation

- [ ] 7.1 Update README with MVP status
- [ ] 7.2 Document how to test manually
- [ ] 7.3 Add example usage to README

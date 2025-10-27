# Implementation Tasks

## 1. Bash Integration

- [ ] 1.1 Research bash readline completion system
- [ ] 1.2 Create bash.sh integration script
- [ ] 1.3 Implement readline COMP_WORDS parsing
- [ ] 1.4 Capture COMP_LINE and COMP_POINT
- [ ] 1.5 Send request to daemon via Unix socket
- [ ] 1.6 Parse daemon response
- [ ] 1.7 Populate COMPREPLY with suggestions
- [ ] 1.8 Bind to configurable key (default: Tab or Alt+Space)
- [ ] 1.9 Test with common commands (git, npm, docker)

## 2. Fish Integration

- [ ] 2.1 Research fish completion system
- [ ] 2.2 Create fish.fish integration script
- [ ] 2.3 Implement commandline -c parsing
- [ ] 2.4 Capture current token and cursor position
- [ ] 2.5 Send request to daemon via Unix socket
- [ ] 2.6 Parse daemon response
- [ ] 2.7 Use fish's complete -c to register completions
- [ ] 2.8 Test with common commands
- [ ] 2.9 Handle fish-specific completion features

## 3. Shell Detection

- [ ] 3.1 Implement shell detection in install command
- [ ] 3.2 Detect from $SHELL environment variable
- [ ] 3.3 Detect from process name
- [ ] 3.4 Allow manual shell specification

## 4. Installation System

- [ ] 4.1 Update `autocomplete-rs install` to support bash
- [ ] 4.2 Update `autocomplete-rs install` to support fish
- [ ] 4.3 Add integration to appropriate RC file (.bashrc, config.fish)
- [ ] 4.4 Handle existing installations (don't duplicate)
- [ ] 4.5 Provide uninstall command

## 5. Protocol Compatibility

- [ ] 5.1 Verify daemon protocol works for all shells
- [ ] 5.2 Handle shell-specific quirks if needed
- [ ] 5.3 Test concurrent connections from different shells

## 6. Testing

- [ ] 6.1 Test bash integration manually
- [ ] 6.2 Test fish integration manually
- [ ] 6.3 Test shell switching on same system
- [ ] 6.4 Test all three shells concurrently
- [ ] 6.5 Verify performance for each shell

## 7. Documentation

- [ ] 7.1 Document bash installation
- [ ] 7.2 Document fish installation
- [ ] 7.3 Update README with multi-shell support
- [ ] 7.4 Add shell-specific troubleshooting

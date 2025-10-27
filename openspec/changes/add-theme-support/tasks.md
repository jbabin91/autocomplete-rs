# Implementation Tasks

## 1. Theme System Foundation

- [ ] 1.1 Create Theme struct with color definitions
  - [ ] background
  - [ ] foreground
  - [ ] selected_background
  - [ ] selected_foreground
  - [ ] border
  - [ ] description_text
- [ ] 1.2 Create ThemeRegistry for managing themes
- [ ] 1.3 Implement default theme (works on all terminals)

## 2. Catppuccin Themes

- [ ] 2.1 Implement Catppuccin Mocha variant
- [ ] 2.2 Implement Catppuccin Macchiato variant
- [ ] 2.3 Implement Catppuccin Frappe variant
- [ ] 2.4 Implement Catppuccin Latte variant
- [ ] 2.5 Use official Catppuccin color palette values

## 3. Configuration

- [ ] 3.1 Create theme configuration file format (TOML)
- [ ] 3.2 Implement config file loader
- [ ] 3.3 Support `~/.config/autocomplete-rs/theme.toml`
- [ ] 3.4 Handle missing config (use default theme)
- [ ] 3.5 Validate theme configuration

## 4. TUI Integration

- [ ] 4.1 Update TUI to accept Theme parameter
- [ ] 4.2 Apply theme colors to dropdown background
- [ ] 4.3 Apply theme colors to selected item
- [ ] 4.4 Apply theme colors to text and descriptions
- [ ] 4.5 Apply theme colors to borders

## 5. Runtime Theme Switching

- [ ] 5.1 Watch config file for changes
- [ ] 5.2 Reload theme on config change
- [ ] 5.3 Notify daemon of theme change
- [ ] 5.4 Update TUI with new theme without restart

## 6. CLI Commands

- [ ] 6.1 Add `autocomplete-rs theme list` command
- [ ] 6.2 Add `autocomplete-rs theme set <name>` command
- [ ] 6.3 Add `autocomplete-rs theme preview <name>` command
- [ ] 6.4 Update config file when theme is set

## 7. Testing

- [ ] 7.1 Visual tests for each Catppuccin variant
- [ ] 7.2 Test theme fallback behavior
- [ ] 7.3 Test runtime theme switching
- [ ] 7.4 Test invalid theme config handling

## 8. Documentation

- [ ] 8.1 Document available themes
- [ ] 8.2 Document theme configuration format
- [ ] 8.3 Add theme showcase screenshots to README
- [ ] 8.4 Document how to create custom themes

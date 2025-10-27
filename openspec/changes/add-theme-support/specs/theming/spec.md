# Theming Specification

## ADDED Requirements

### Requirement: Theme Definition

The system SHALL support themes with customizable color schemes for the TUI.

#### Scenario: Theme structure

- **WHEN** defining a theme
- **THEN** theme includes colors for background, foreground,
  selected_background, selected_foreground, border, description

#### Scenario: Default theme

- **WHEN** no theme is configured
- **THEN** default theme is used (works on all terminals)

### Requirement: Catppuccin Support

The system SHALL include all four Catppuccin theme variants.

#### Scenario: Mocha variant

- **WHEN** user selects Catppuccin Mocha
- **THEN** dropdown uses Mocha color palette

#### Scenario: Macchiato variant

- **WHEN** user selects Catppuccin Macchiato
- **THEN** dropdown uses Macchiato color palette

#### Scenario: Frappe variant

- **WHEN** user selects Catppuccin Frappe
- **THEN** dropdown uses Frappe color palette

#### Scenario: Latte variant

- **WHEN** user selects Catppuccin Latte
- **THEN** dropdown uses Latte color palette (light theme)

### Requirement: Theme Configuration

The system SHALL load theme settings from a configuration file.

#### Scenario: Config file location

- **WHEN** looking for theme config
- **THEN** system checks `~/.config/autocomplete-rs/theme.toml`

#### Scenario: Valid configuration

- **GIVEN** config file with `theme = "catppuccin-mocha"`
- **WHEN** system loads config
- **THEN** Catppuccin Mocha theme is applied

#### Scenario: Missing config file

- **WHEN** config file doesn't exist
- **THEN** default theme is used without error

#### Scenario: Invalid theme name

- **GIVEN** config specifies non-existent theme
- **WHEN** system loads config
- **THEN** warning is logged and default theme is used

### Requirement: Runtime Theme Switching

The system SHALL allow users to change themes without restarting the daemon.

#### Scenario: Change theme via CLI

- **WHEN** user runs `autocomplete-rs theme set catppuccin-mocha`
- **THEN** theme is changed immediately and config file is updated

#### Scenario: Config file watch

- **WHEN** user manually edits theme.toml
- **THEN** daemon detects change and reloads theme

#### Scenario: Theme preview

- **WHEN** user runs `autocomplete-rs theme preview catppuccin-latte`
- **THEN** temporary dropdown shows Latte theme (no config change)

### Requirement: Theme Registry

The system SHALL provide a registry of available themes.

#### Scenario: List themes

- **WHEN** user runs `autocomplete-rs theme list`
- **THEN** all available themes are displayed (default, catppuccin-mocha,
  catppuccin-macchiato, catppuccin-frappe, catppuccin-latte)

#### Scenario: Get theme by name

- **WHEN** requesting theme "catppuccin-mocha"
- **THEN** Mocha theme definition is returned

### Requirement: TUI Color Application

The system SHALL apply theme colors to all TUI elements.

#### Scenario: Dropdown background

- **GIVEN** theme with specific background color
- **WHEN** dropdown renders
- **THEN** background uses theme color

#### Scenario: Selected item highlighting

- **GIVEN** theme with selected_background color
- **WHEN** item is selected
- **THEN** selected item uses theme highlight color

#### Scenario: Text colors

- **GIVEN** theme with foreground color
- **WHEN** text is rendered
- **THEN** text uses theme foreground color

#### Scenario: Description colors

- **GIVEN** theme with description_text color
- **WHEN** descriptions are shown
- **THEN** descriptions use distinct color from main text

### Requirement: Custom Theme Support

The system SHALL allow users to define custom themes in configuration.

#### Scenario: Custom theme definition

- **GIVEN** user adds custom theme to config
- **WHEN** theme is selected
- **THEN** custom colors are applied

#### Scenario: Color format

- **WHEN** specifying colors in config
- **THEN** hex format (#RRGGBB) or RGB(r,g,b) is supported

#### Scenario: Partial theme override

- **GIVEN** custom theme with only some colors defined
- **WHEN** theme is loaded
- **THEN** undefined colors fall back to default theme

### Requirement: Terminal Compatibility

The system SHALL ensure themes work across different terminal color
capabilities.

#### Scenario: 256-color terminal

- **WHEN** running in 256-color terminal
- **THEN** theme colors are accurately rendered

#### Scenario: True-color terminal

- **WHEN** running in 24-bit true-color terminal
- **THEN** exact theme colors are rendered

#### Scenario: 16-color fallback

- **WHEN** running in basic 16-color terminal
- **THEN** theme colors map to closest ANSI colors

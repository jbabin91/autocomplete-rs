# TUI Specification

## ADDED Requirements

### Requirement: Dropdown Rendering

The TUI SHALL display completion suggestions in a dropdown menu using Ratatui.

#### Scenario: Display suggestions

- **GIVEN** suggestions list `["checkout", "cherry-pick", "clean"]`
- **WHEN** TUI renders
- **THEN** all suggestions are visible in dropdown

#### Scenario: Empty suggestions

- **GIVEN** empty suggestions list
- **WHEN** TUI renders
- **THEN** no dropdown is displayed (or shows "No completions")

#### Scenario: Many suggestions

- **GIVEN** 50+ suggestions
- **WHEN** TUI renders
- **THEN** dropdown shows scrollable list with visible subset

### Requirement: Keyboard Navigation

The TUI SHALL support keyboard navigation for selecting suggestions.

#### Scenario: Navigate down

- **GIVEN** dropdown with 3 suggestions, first item selected
- **WHEN** user presses Down arrow
- **THEN** second item is selected

#### Scenario: Navigate up

- **GIVEN** dropdown with 3 suggestions, second item selected
- **WHEN** user presses Up arrow
- **THEN** first item is selected

#### Scenario: Wrap at boundaries

- **GIVEN** dropdown with 3 suggestions, last item selected
- **WHEN** user presses Down arrow
- **THEN** first item is selected (wraps around)

#### Scenario: Confirm selection

- **GIVEN** dropdown with item selected
- **WHEN** user presses Enter
- **THEN** selected suggestion is returned

#### Scenario: Cancel selection

- **GIVEN** dropdown is displayed
- **WHEN** user presses Esc
- **THEN** TUI closes with no selection

### Requirement: Visual Highlighting

The TUI SHALL visually highlight the currently selected item.

#### Scenario: Selected item styling

- **GIVEN** dropdown with second item selected
- **WHEN** rendering
- **THEN** selected item has distinct color/style (yellow + bold)

#### Scenario: Unselected items

- **GIVEN** dropdown with items
- **WHEN** rendering
- **THEN** unselected items use default style

### Requirement: Position Calculation

The TUI SHALL position the dropdown menu relative to the cursor position in the
terminal.

#### Scenario: Dropdown below cursor

- **GIVEN** cursor at row 10, column 20
- **WHEN** TUI renders dropdown
- **THEN** dropdown appears below cursor position

#### Scenario: Near bottom of screen

- **GIVEN** cursor near bottom of terminal
- **WHEN** TUI renders dropdown
- **THEN** dropdown appears above cursor (to stay visible)

#### Scenario: Near right edge

- **GIVEN** cursor near right edge of terminal
- **WHEN** TUI renders dropdown
- **THEN** dropdown is shifted left to fit on screen

### Requirement: Description Display

The TUI SHALL display description text alongside each suggestion.

#### Scenario: Suggestion with description

- **GIVEN** suggestion `{text: "checkout", description: "Switch branches"}`
- **WHEN** rendering item
- **THEN** both text and description are visible

#### Scenario: Suggestion without description

- **GIVEN** suggestion with no description
- **WHEN** rendering item
- **THEN** only suggestion text is shown

### Requirement: Performance

The TUI SHALL render and update within 10 milliseconds.

#### Scenario: Initial render

- **WHEN** TUI first displays suggestions
- **THEN** render time is less than 10ms

#### Scenario: Navigation update

- **WHEN** user navigates between items
- **THEN** re-render time is less than 5ms

### Requirement: Terminal Compatibility

The TUI SHALL work correctly in all major terminal emulators.

#### Scenario: iTerm2 rendering

- **WHEN** running in iTerm2
- **THEN** dropdown renders correctly with proper colors

#### Scenario: Alacritty rendering

- **WHEN** running in Alacritty
- **THEN** dropdown renders correctly with proper colors

#### Scenario: Kitty rendering

- **WHEN** running in Kitty
- **THEN** dropdown renders correctly with proper colors

#### Scenario: VSCode terminal rendering

- **WHEN** running in VSCode integrated terminal
- **THEN** dropdown renders correctly with proper colors

# Shell Tokenization Research for autocomplete-rs

**Date:** 2026-02-06
**Purpose:** Document shell tokenization rules and implementation strategies for building the autocomplete-rs command-line parser.

---

## Executive Summary

This research document provides comprehensive analysis of shell tokenization rules, cursor position handling, context analysis for completions, and implementation recommendations for the autocomplete-rs parser. The goal is to enable accurate, fast (<5ms) parsing of shell command buffers to provide intelligent autocomplete suggestions.

**Key Findings:**

1. POSIX shell tokenization is context-dependent and specified "negatively" (by delimitation rules rather than recognition rules)
2. Zsh adds significant complexity beyond POSIX with extended globbing, non-default word splitting behavior, and additional quoting forms
3. Existing Rust crates (shlex, shell-words, shellwords) provide POSIX-compliant tokenization but lack zsh-specific features
4. For MVP, a simplified tokenizer focusing on common cases can achieve >90% accuracy
5. State machine (FSM) approach is recommended for performance and maintainability
6. Fig/Inshellisense use declarative specs rather than deep parsing - a viable alternative approach

---

## 1. Zsh Tokenization Rules

### 1.1 Shell Grammar Overview

Zsh follows a hierarchical grammar structure:

```text
List → Sublist → Pipeline → Simple Command
```

**Simple Commands:**

- Optional parameter assignments followed by blank-separated words with optional redirections
- First word is the command; remaining words are arguments
- Format: `[VAR=value ...] command [arg1 arg2 ...] [< input] [> output]`

**Pipelines:**

- Connect commands using `|` (stdout) or `|&` (stdout + stderr)
- Example: `cmd1 | cmd2` or `cmd1 |& cmd2`
- `coproc` keyword enables bidirectional communication

**Sublists:**

- Combine pipelines with `&&` (conditional AND) or `||` (conditional OR)
- Both operators are left-associative with equal precedence
- Example: `cmd1 && cmd2 || cmd3`

**Lists:**

- Sublists terminated by `;`, `&`, `&|`, `&!`, or newline
- `;` and newline: wait for completion
- `&`: background execution (returns zero status)

### 1.2 Shell Metacharacters

**Operators/Delimiters:**

- `|` - pipe stdout
- `|&` - pipe stdout and stderr
- `&&` - conditional AND
- `||` - conditional OR
- `;` - command separator
- `&` - background execution
- `()` - subshell execution
- `{}` - command grouping
- `<`, `>`, `>>`, `<<<` - redirections
- `<(...)`, `>(...)` - process substitution

**Reserved Words:**

```text
do done esac then elif else fi for case if while function repeat time
until select coproc nocorrect foreach end ! [[ { } declare export
float integer local readonly typeset
```

### 1.3 Quoting Rules

**Single Quotes (`'...'`):**

- Preserve literal value of all characters
- Cannot contain a single quote (even escaped)
- No expansions occur inside single quotes

**Double Quotes (`"..."`):**

- Parameter and command substitution occur
- Backslash quotes: `\`, `'`, `"`, `$`, and first char of `$histchars` (default `!`)
- Glob expansion suppressed
- Word splitting suppressed (in zsh by default anyway)

**ANSI-C Quoting (`$'...'`):**

- Backslash escape sequences interpreted per ANSI C standard
- Common sequences:
  - `\n` - newline
  - `\t` - tab
  - `\\` - backslash
  - `\'` - single quote
  - `\"` - double quote
  - `\nnn` - octal value (1-3 digits)
  - `\xHH` - hex value (1-2 digits)
  - `\uHHHH` - Unicode (1-4 hex digits)
  - `\UHHHHHHHH` - Unicode (1-8 hex digits)
- Expanded result is treated as single-quoted
- Supported by ksh, bash, zsh (POSIX SUS issue 7)

**Backslash Escaping:**

- Single backslash before a character quotes that character
- Makes special characters literal
- Line continuation: `\` at end of line

**Backticks (`` `...` ``):**

- Legacy command substitution syntax
- Equivalent to `$(...)`
- Nesting requires escaping: `` `cmd1 \`cmd2\`` ``

### 1.4 Word Splitting Rules

**Critical zsh difference:** Word splitting is **disabled by default** in zsh, unlike bash/sh.

**IFS Variable:**

- Default value: `<space><tab><newline>`
- Controls field splitting after parameter expansion
- In zsh, only applies when `SH_WORD_SPLIT` option is set or `${=VAR}` expansion flag is used

**Field Splitting Behavior (when enabled):**

1. **Default IFS:** Sequences of space, tab, newline at beginning/end are ignored; any sequence of IFS characters not at beginning/end delimits words

2. **Custom IFS:** Whitespace characters (space, tab) in IFS are ignored at beginning/end; non-whitespace IFS characters (plus adjacent IFS whitespace) delimit fields

3. **Empty IFS:** No word splitting occurs

**Zsh-specific flags:**

- `${=VAR}` - force word splitting for this expansion
- `${==VAR}` - force word splitting using default IFS regardless of actual IFS value

### 1.5 Variable Expansion

**Basic Forms:**

- `$VAR` - simple variable reference
- `${VAR}` - braces protect variable name from following characters

**When Braces Required:**

- Variable name would be ambiguous: `${var}name` vs `$varname`
- Using expansion operators (below)

**Parameter Expansion Operators:**

- `${var:-default}` - use default if var is null/unset
- `${var:=default}` - assign default if var is null/unset
- `${var:?message}` - error if var is null/unset
- `${var:+alternate}` - use alternate if var is set
- `${#var}` - length of var
- `${var#pattern}` - remove shortest prefix matching pattern
- `${var##pattern}` - remove longest prefix matching pattern
- `${var%pattern}` - remove shortest suffix matching pattern
- `${var%%pattern}` - remove longest suffix matching pattern
- `${var/pattern/replacement}` - replace first match
- `${var//pattern/replacement}` - replace all matches

**Command Substitution:**

- `$(command)` - modern syntax (preferred)
- `` `command` `` - legacy syntax
- Output is captured and substituted
- Trailing newlines are removed
- Nesting with `$()` is cleaner: `$(cmd1 $(cmd2))`

**Arithmetic Expansion:**

- `$((expression))` - evaluate arithmetic expression
- Example: `$((1 + 2))` → `3`

**Process Substitution:**

- `<(command)` - creates temporary file descriptor for command output
- `>(command)` - creates temporary file descriptor for command input
- Example: `diff <(cmd1) <(cmd2)`
- Implemented via `/dev/fd/$fd` or named pipes (FIFOs)

### 1.6 Glob Expansion

**Basic Wildcards:**

- `*` - matches zero or more characters
- `?` - matches any single character
- `[abc]` - matches any character in the set
- `[a-z]` - matches any character in the range
- `[!abc]` - matches any character NOT in the set

**Zsh Extended Globbing** (requires `setopt EXTENDED_GLOB`):

- `**/*` - recursive directory match (any depth)
- `^pattern` - negation
- `pattern1~pattern2` - match pattern1 except pattern2
- `(pattern)` - grouping
- `pattern#` - zero or more occurrences
- `pattern##` - one or more occurrences
- `(pattern1|pattern2)` - alternation

**Brace Expansion:**

- `{a,b,c}` - expands to: `a b c`
- `{1..5}` - expands to: `1 2 3 4 5`
- `{a..z}` - expands to: `a b c ... z`
- `file{.txt,.md}` - expands to: `file.txt file.md`
- Not glob matching - generates all combinations regardless of file existence
- Range operator `[]` only matches existing files; brace expansion generates all

---

## 2. POSIX Shell Tokenization

### 2.1 Token Recognition Process

POSIX shell tokenization is **context-dependent** and specified **"negatively"** - by characterizing how tokens are delimited rather than how they're recognized.

**Rule Application Order** (apply first matching rule):

1. **End of input** - current token (if any) is delimited

2. **Operator continuation** - if previous character was part of an operator and current character can extend it, they merge into one operator

3. **Operator termination** - if current character cannot extend the previous operator, that operator token ends

4. **Quoting effects** - backslash, single-quote, or double-quote (if unquoted) affects quoting for subsequent characters

5. **Expansion recognition** - recognize parameter expansion (`$`, `${`), command substitution (`$(`, backtick), arithmetic expansion (`$((`)

6. **Blank character handling** - unquoted blank delimits any token containing the previous character; current character discarded

7. **Comment processing** - `#` and all subsequent characters until newline are discarded (newline itself not part of comment)

### 2.2 Token Categories

After delimitation, tokens are categorized:

1. **NEWLINE** - literal newline character
2. **IO_NUMBER** - string of digits followed by `<` or `>`
3. **Operators** - special multi-character sequences
4. **Reserved words** - in grammatical command position
5. **Alias names** - checked for alias substitution
6. **TOKEN** - everything else (words)

**Common Operators:**

- `&&` `||` - logical operators
- `;;` - case terminator
- `<<` `>>` - here-document, append redirection
- `<&` `>&` - duplicate file descriptor
- `<>` - open for reading and writing
- `<<-` - here-document with tab stripping
- `>|` - force clobber

### 2.3 Quoting Mechanisms

**Three quoting mechanisms preserve literal values:**

1. **Escape character (backslash)** - preserves literal value of next character (except newline)

2. **Single quotes** - preserve literal value of all enclosed characters (cannot contain single quote)

3. **Double quotes** - preserve literal value except `$`, backtick, `\`, and (sometimes) `!`

### 2.4 Here-Documents

When `<<` operator recognized, subsequent lines become here-document content:

```bash
command << DELIMITER
line 1
line 2
DELIMITER
```

**Rules:**

- Delimiter can be quoted to suppress expansion
- `<<-` strips leading tabs from content lines
- Content parsed under special rules until delimiter line matched
- Delimiter line cannot have leading/trailing spaces (unless using `<<-` for tabs)

---

## 3. Cursor Position Handling

### 3.1 Cursor Context States

**Mid-word (partial token):**

```text
git che|ckout main
    ^^^
```

- Cursor at position within a token
- Need to identify: token start, token end, cursor offset within token
- Completions should match prefix before cursor (`che`)
- Some systems complete full word, others just prefix

**End-of-word:**

```text
git checkout |
             ^
```

- Cursor immediately after whitespace
- Beginning new argument/option
- Completions based on what command/subcommand expects next

**Incomplete quote:**

```text
git commit -m "fix bug|
                      ^
```

- Cursor inside unclosed quote
- Need to recognize quoted context
- Suppress expansions appropriately

**Incomplete flag:**

```text
git commit --ver|
                ^
```

- Partial long option
- Match against available options starting with `--ver`

### 3.2 Zsh Completion System Context

**Key Variables:**

- `$BUFFER` - entire command line buffer
- `$CURSOR` - cursor position (0-indexed byte offset)
- `$LBUFFER` - buffer content left of cursor
- `$RBUFFER` - buffer content right of cursor
- `$words` - array of words on command line
- `$CURRENT` - index of word containing cursor

**Context String Format:**

```text
:completion:function:completer:command:argument:tag
```

Example: `:completion::complete:git:argument-1:branch-names`

**Fields:**

1. Literal "completion"
2. Function name (usually blank)
3. Completer name
4. Command being completed
5. Argument position
6. Tag (type of completion)

**COMPLETE_IN_WORD Option:**

- Default: cursor moves to end of word
- With option: cursor stays put, completes on both sides
- `expand-or-complete-prefix` - only complete what's before cursor

### 3.3 Bash Completion Context

**Key Variables:**

- `$COMP_LINE` - entire command line
- `$COMP_POINT` - cursor position (byte offset)
- `$COMP_WORDS` - array of words (after word splitting)
- `$COMP_CWORD` - index of word containing cursor
- `$1` - command name
- `$2` - word being completed
- `$3` - word preceding word being completed

**Colon Handling:**

- Bash treats `:` as starting new completion token
- Problematic for PATH-like variables
- Workaround: escape with backslash

**COMPREPLY Array:**

- Completion functions populate this array
- One completion per array element

---

## 4. Context Analysis for Completions

### 4.1 Completion Types

**Command/Subcommand:**

```text
git |           → complete subcommands (checkout, commit, pull, ...)
git checkout |  → complete arguments for checkout
```

**Options/Flags:**

```text
git commit -|     → complete short options (-m, -a, ...)
git commit --|    → complete long options (--message, --amend, ...)
```

**Option Arguments:**

```text
git commit -m |   → complete message (or suggest input)
git checkout |    → complete branch names, file paths
```

**Filenames:**

```text
cat |        → complete filenames
ls -la |     → complete filenames
cd |         → complete directories only
```

### 4.2 Pipe Chain Handling

```text
cmd1 | cmd2 |
       ^^^^^ complete for cmd2, not cmd1
```

**Strategy:**

1. Split buffer on unquoted `|`
2. Find which segment contains cursor
3. Parse only that segment
4. Context is for rightmost command in that segment

**Example:**

```text
cat file.txt | grep "pattern" | wc |
                                ^^^ cursor here
                                    complete for wc, not cat or grep
```

### 4.3 Redirection Handling

```text
command > fi|    → complete filenames
command 2>|      → complete filenames
command < |      → complete filenames (input files)
```

**Strategy:**

1. Detect redirection operator before cursor
2. Override completion type to files
3. For `>` and `>>`, might suggest new filename or existing

### 4.4 Command Chaining

**Sequential (`;`):**

```text
cmd1; cmd2; |
            ^ complete for new command
```

**Conditional (`&&`, `||`):**

```text
make && ./binary |
                 ^ complete for ./binary
make || echo |
             ^ complete for echo
```

**Strategy:**

- Split on unquoted `;`, `&&`, `||`
- Find segment containing cursor
- Parse only that segment

### 4.5 Subshell Context

```text
(cmd1; cmd2 |)
            ^ complete for cmd2 within subshell
```

**Strategy:**

- Track parenthesis nesting
- Parse innermost subshell containing cursor
- Context isolated from outer shell

### 4.6 Environment Variable Assignments

```text
VAR=value cmd |
              ^ complete for cmd

VAR=|         → complete values (or filenames)
```

**Strategy:**

1. Detect `VAR=value` pattern before command
2. Skip assignment tokens
3. First non-assignment word is command

### 4.7 Command Wrappers (sudo, env, etc.)

```text
sudo git checkout |
                  ^ complete for git checkout, not sudo
```

**Common wrappers:**

- `sudo`, `doas` - privilege escalation
- `env`, `nice`, `ionice` - environment/priority
- `time`, `timeout` - timing/limits
- `nohup` - detach from terminal
- `xargs` - argument builder

**Strategy:**

- Maintain list of known wrapper commands
- Skip wrapper and its options
- Parse wrapped command

---

## 5. Existing Parser Approaches

### 5.1 Zsh Completion System

**Architecture:**

1. **compinit** - initialization function
2. **\_complete** - main completion function
3. **Context building** - progressive from generic to specific
4. **Tag system** - classify completions (files, options, commands, etc.)
5. **Styles** - configure behavior via `zstyle`

**Parsing Strategy:**

- Delegates to command-specific completion functions
- Each function understands its command's syntax
- Uses context to determine what to complete
- Not a general-purpose parser - expert system

**Performance:**

- Can be slow for complex completions
- Lazy loading of completion functions
- Caching of expensive operations

### 5.2 Bash Completion

**Architecture:**

1. **bash-completion** package
2. **Programmable completion** system
3. **compgen** builtin
4. **Custom completion functions**

**Parsing Strategy:**

- Uses `$COMP_WORDS` array (pre-tokenized by bash)
- Completion functions examine words and position
- Generate completions via `compgen` or manually
- Not sophisticated parsing - mostly pattern matching

**Tokenization:**

- Bash does the tokenization
- Completions work with word array
- Colon handling quirk (treats `:` as word boundary)

### 5.3 Fish Shell

**Architecture:**

- Built-in completion system
- Command definitions in fish scripts
- Parser integrated with evaluator

**Tokenization:**

- `commandline` command provides tokens
- `commandline --tokenize` - split into tokens
- `commandline --tokenize-raw` - without unescaping
- `commandline --current-token` - token at cursor

**Parsing:**

- Fish has its own grammar (not POSIX)
- Simpler syntax (no word splitting by default)
- Better error messages and completion behavior

### 5.4 Fig (Amazon Q) / Inshellisense

**Architecture:**

- **Declarative specifications** rather than parsing
- TypeScript spec files define command structure
- Accessibility API (macOS) for window positioning
- Shell integration to capture buffer

**Spec Format:**

```typescript
{
  name: "git",
  subcommands: [
    {
      name: "checkout",
      args: [
        { name: "branch", generators: branchGenerator }
      ]
    }
  ],
  options: [
    { name: ["-m", "--message"], args: { name: "msg" } }
  ]
}
```

**Advantages:**

- No complex parsing needed
- Community-contributed specs
- Scales to 600+ CLI tools
- Fast and maintainable

**Disadvantages:**

- Requires spec for every command
- May not handle edge cases well
- Spec maintenance burden

**Key Insight:**
Fig/Inshellisense largely **avoid the parsing problem** by using declarative specs. The parser only needs to:

1. Split on whitespace (respecting quotes)
2. Track cursor position
3. Match against spec structure

This is a **pragmatic alternative** to full shell parsing.

---

## 6. Rust Crates for Shell Tokenization

### 6.1 shlex (Recommended)

**Stats:**

- 20,687,441 downloads/month
- Used in 55,840 crates
- MIT or Apache-2.0 license

**Features:**

- POSIX shell word splitting
- Quoting and escaping
- `no_std` support (via disabled `std` feature)
- Byte string variants (`bytes` module)
- Tested against bash, zsh, dash, Busybox ash, mksh, fish, Python shlex, C wordexp

**API:**

```rust
// Parsing
let mut lexer = shlex::Shlex::new("echo 'hello world'");
for word in lexer {
    println!("{}", word);
}

// Convenience
let words = shlex::split("echo 'hello world'").unwrap();

// Quoting
let quoted = shlex::try_quote("hello world").unwrap();
let joined = shlex::try_join(["echo", "hello world"]).unwrap();
```

**Limitations:**

- POSIX-focused (doesn't handle zsh-specific syntax)
- **Security issue:** Does not quote control characters (cannot be quoted portably)
- No cursor position / partial token support
- No zsh features: `$'...'` quoting, extended globs, process substitution

**Performance:**

- Micro-optimized (UTF-8 oblivious, works on bytes)
- Should be sub-millisecond for typical buffers

### 6.2 shell-words

**Stats:**

- Compatible with GLib's `g_shell_parse_argv`
- Similar to Python's `shlex.split` in POSIX mode

**Features:**

- POSIX shell word parsing
- Comment support
- Simple API

**API:**

```rust
let words = shell_words::split("echo 'hello world'").unwrap();
```

**Limitations:**

- Similar to shlex
- Less popular / fewer tests

### 6.3 shellwords

**Description:**

- Parse strings according to UNIX Bourne shell word parsing rules

**Features:**

- POSIX Bourne shell compatible
- Split and join functions

**Limitations:**

- Bourne shell only (older than POSIX)
- Less feature-complete

### 6.4 Comparison

| Feature         | shlex | shell-words | shellwords   |
| --------------- | ----- | ----------- | ------------ |
| Downloads/month | 20M+  | ~500K       | ~200K        |
| POSIX compliant | ✅    | ✅          | Bourne shell |
| Quoting         | ✅    | ✅          | ✅           |
| `no_std`        | ✅    | ❌          | ❌           |
| Zsh features    | ❌    | ❌          | ❌           |
| Cursor support  | ❌    | ❌          | ❌           |

**Recommendation:** Use **shlex** as a foundation, extend for zsh-specific features and cursor handling.

---

## 7. Edge Cases to Handle

### 7.1 Aliases

```text
alias ll='ls -la'
ll |
   ^ complete for ls arguments, not ll
```

**Challenge:**

- Need to expand aliases to understand actual command
- Aliases can contain pipes, redirections, etc.
- Recursive alias expansion
- User-defined aliases unknown to daemon

**Strategies:**

1. **Ignore aliases** (MVP) - complete based on literal command
2. **Shell integration** - have shell send expanded command line
3. **Alias database** - maintain known aliases, expand during parse

### 7.2 Shell Functions

```text
myfunc() { git "$@"; }
myfunc commit |
              ^ should complete for git commit
```

**Challenge:**

- Function bodies unknown to daemon
- Would need to evaluate function to understand behavior

**Strategy:**

- MVP: ignore (treat as opaque command)
- Advanced: allow users to define completion specs for functions

### 7.3 Here-Documents

```text
cat << EOF
line 1
line 2|
      ^ inside here-document, don't complete
EOF
```

**Strategy:**

1. Track `<<` or `<<-` operator
2. Record delimiter
3. Suppress completion until delimiter line
4. Edge case: delimiter in quotes (`<< "EOF"`) - no expansion

### 7.4 Multi-line Commands

```text
git commit \
  -m "message" \
  --amend |
          ^ continue previous line
```

**Strategy:**

1. Track backslash at end of line
2. Concatenate lines (remove `\` and newline)
3. Parse as single command

**Alternative (simpler):**

- Only complete current line
- Assume user understands context from previous lines

### 7.5 Subshells and Command Grouping

```text
(cd /tmp && ls |)
                ^ complete for ls in subshell context

{ cmd1; cmd2 |; }
             ^ complete for cmd2 in group
```

**Strategy:**

1. Track nesting depth of `()` and `{}`
2. Parse innermost context containing cursor
3. Respect scoping (subshell has different working directory)

**Simplification (MVP):**

- Ignore nesting, complete based on current command

### 7.6 Environment Variables Before Command

```text
PATH=/custom/path git |
                      ^ complete for git

DEBUG=1 VAR=value |
                  ^ complete command names
```

**Strategy:**

1. Detect `VAR=value` pattern (regex: `[A-Za-z_][A-Za-z0-9_]*=.*`)
2. Skip assignment tokens
3. First non-assignment is command

### 7.7 Command Wrappers (sudo, time, etc.)

```text
sudo -u user git checkout |
                          ^ complete for git checkout

time -p make test |
                  ^ complete for make test
```

**Strategy:**

1. Maintain list of known wrappers
2. Skip wrapper and its options
3. Parse wrapped command

**Known wrappers:**

- `sudo [-u user] [-g group] [-H] ...`
- `env [VAR=value ...] [command]`
- `nice [-n level] [command]`
- `time [-p] [command]`
- `timeout [duration] [command]`
- `nohup [command]`
- `xargs [options] [command]` (special case - command is partial)

### 7.8 Incomplete Quotes

```text
git commit -m "hello world
                          ^ unclosed quote
```

**Strategy:**

1. Track quote state (none, single, double, ANSI-C)
2. If quote unclosed at cursor, recognize quoted context
3. Suppress expansions, offer quote completion

**Error handling:**

- Could return error
- Or: treat as incomplete token, suggest closing quote

### 7.9 Globs

```text
ls *.txt |
         ^ after glob pattern
```

**Strategy (MVP):**

- Don't expand globs during parsing
- Treat `*.txt` as literal token
- Completion after glob might suggest filenames matching pattern

**Advanced:**

- Expand glob to see what files match
- Completions based on matched files

---

## 8. Implementation Strategy

### 8.1 Minimal Viable Tokenizer (MVP)

**Goal:** <5ms parsing, 90%+ accuracy for common cases

**What to include:**

1. **Basic tokenization**
   - Whitespace splitting
   - Single and double quote handling
   - Backslash escaping
   - Basic operators: `|`, `;`, `&&`, `||`, `&`, `<`, `>`, `>>`

2. **Cursor position tracking**
   - Identify token containing cursor
   - Determine if at word boundary or mid-word
   - Return prefix before cursor

3. **Context detection**
   - Command vs subcommand vs argument
   - After `-` or `--` → option
   - After `|` or `&&` or `||` or `;` → new command
   - After redirection → filename

4. **Simple quoting**
   - Single quotes (no escapes)
   - Double quotes (basic escapes: `\"`, `\\`, `\$`)
   - Backslash escaping

**What to skip (for MVP):**

- `$'...'` ANSI-C quoting
- Variable expansion
- Command substitution
- Arithmetic expansion
- Process substitution
- Glob expansion
- Here-documents
- Brace expansion
- Alias expansion
- Function detection
- Subshell nesting
- Complex word splitting (assume default IFS)

### 8.2 State Machine Design

**States:**

```rust
enum TokenizerState {
    Normal,              // default state
    InSingleQuote,       // inside '...'
    InDoubleQuote,       // inside "..."
    Escaped,             // after backslash
    InOperator,          // reading multi-char operator
}

enum TokenType {
    Word,                // regular word/argument
    Operator,            // |, &&, ||, ;, &, <, >, etc.
    Whitespace,          // spaces, tabs (usually discarded)
}

struct Token {
    token_type: TokenType,
    content: String,
    start: usize,        // byte offset in buffer
    end: usize,          // byte offset in buffer
}
```

**Algorithm:**

```rust
fn tokenize(buffer: &str, cursor: usize) -> TokenizeResult {
    let mut state = TokenizerState::Normal;
    let mut tokens = Vec::new();
    let mut current_token = String::new();
    let mut token_start = 0;

    for (pos, ch) in buffer.char_indices() {
        match state {
            TokenizerState::Normal => {
                match ch {
                    ' ' | '\t' | '\n' => {
                        if !current_token.is_empty() {
                            tokens.push(create_token(&current_token, token_start, pos));
                            current_token.clear();
                        }
                        token_start = pos + 1;
                    },
                    '\'' => {
                        state = TokenizerState::InSingleQuote;
                        current_token.push(ch);
                    },
                    '"' => {
                        state = TokenizerState::InDoubleQuote;
                        current_token.push(ch);
                    },
                    '\\' => {
                        state = TokenizerState::Escaped;
                    },
                    '|' | '&' | ';' | '<' | '>' => {
                        // Check for multi-char operators
                        // ... operator handling logic ...
                    },
                    _ => {
                        current_token.push(ch);
                    }
                }
            },
            // ... other states ...
        }
    }

    // Find token containing cursor
    let cursor_token = tokens.iter()
        .find(|t| t.start <= cursor && cursor <= t.end);

    TokenizeResult {
        tokens,
        cursor_token,
        cursor_offset: cursor_token.map(|t| cursor - t.start),
    }
}
```

### 8.3 Context Analysis

**After tokenization:**

```rust
fn analyze_context(tokens: &[Token], cursor_pos: usize) -> CompletionContext {
    // Find cursor position in token stream
    let cursor_idx = find_cursor_token_index(tokens, cursor_pos);

    // Look backwards to determine context
    let mut idx = cursor_idx;
    loop {
        match tokens[idx].token_type {
            TokenType::Operator => {
                // After pipe/chain, completing new command
                if matches!(tokens[idx].content.as_str(), "|" | ";" | "&&" | "||") {
                    return CompletionContext::Command;
                }
                // After redirection, completing filename
                if matches!(tokens[idx].content.as_str(), "<" | ">" | ">>") {
                    return CompletionContext::Filename;
                }
            },
            TokenType::Word => {
                // Check if previous word starts with -/--
                if tokens[idx].content.starts_with("--") {
                    return CompletionContext::OptionArgument(tokens[idx].content.clone());
                }
                if tokens[idx].content.starts_with("-") {
                    return CompletionContext::OptionArgument(tokens[idx].content.clone());
                }

                // Otherwise, need to count words to determine position
                let word_count = count_command_words(&tokens[0..idx]);
                if word_count == 0 {
                    return CompletionContext::Command;
                } else if word_count == 1 {
                    return CompletionContext::Subcommand;
                } else {
                    return CompletionContext::Argument(word_count - 1);
                }
            },
            _ => {}
        }

        if idx == 0 { break; }
        idx -= 1;
    }

    CompletionContext::Command
}
```

### 8.4 Performance Optimizations

**Target: <5ms total parsing time**

1. **Avoid regex** - use simple character matching
2. **Single pass** - tokenize and analyze in one pass if possible
3. **Minimize allocations** - reuse buffers, use string slices
4. **Early exit** - stop parsing after cursor position
5. **Lazy evaluation** - don't expand variables/globs unless needed

**Benchmarking:**

```rust
#[bench]
fn bench_tokenize_simple(b: &mut Bencher) {
    let buffer = "git commit -m 'message' --amend";
    b.iter(|| tokenize(buffer, buffer.len()));
}

#[bench]
fn bench_tokenize_complex(b: &mut Bencher) {
    let buffer = "sudo env VAR=value git checkout feature/branch && npm test";
    b.iter(|| tokenize(buffer, buffer.len()));
}
```

**Expected performance:**

- Simple commands: <1ms
- Complex commands: 1-3ms
- Very complex (nested, quoted): 3-5ms

### 8.5 Extending shlex

Rather than building from scratch, extend **shlex** crate:

**Advantages:**

- Battle-tested POSIX tokenization
- Already optimized
- Well-maintained

**Extensions needed:**

1. **Cursor tracking**

   ```rust
   pub struct CursorAwareShlex<'a> {
       inner: shlex::Shlex<'a>,
       cursor: usize,
       current_token_start: usize,
   }
   ```

2. **Partial token support**

   ```rust
   pub struct PartialToken {
       content: String,
       prefix: String,      // before cursor
       suffix: String,      // after cursor
       is_complete: bool,
   }
   ```

3. **Zsh quoting extensions**

   ```rust
   fn handle_ansi_c_quote(input: &str) -> Result<String> {
       // Parse $'...' syntax
       // Interpret escape sequences
   }
   ```

4. **Operator tracking**

   ```rust
   enum Operator {
       Pipe,           // |
       PipeAll,        // |&
       And,            // &&
       Or,             // ||
       Semicolon,      // ;
       Background,     // &
       // ...
   }
   ```

---

## 9. Recommendations

### 9.1 Phase 1: MVP (Current Sprint)

**Scope:**

- Basic tokenization using shlex as foundation
- Simple cursor position tracking
- Context detection for: command, subcommand, option, argument, filename
- Handle single quotes, double quotes, backslash escaping
- Detect pipe chains, command chaining (`&&`, `||`, `;`)

**Implementation:**

1. Fork or wrap shlex crate
2. Add cursor position tracking
3. Implement context analysis
4. Write comprehensive tests
5. Benchmark to ensure <5ms

**Success metrics:**

- 90%+ accuracy on common git/npm/cargo commands
- <5ms parsing time
- Handles quoted arguments correctly
- Detects pipe chains and command position

### 9.2 Phase 2: Zsh Features

**Scope:**

- `$'...'` ANSI-C quoting
- Extended glob patterns (basic)
- Process substitution recognition
- Better operator handling (`|&`, `&|`, etc.)

**Implementation:**

- Extend tokenizer state machine
- Add quote type tracking
- Recognize but don't expand advanced features

### 9.3 Phase 3: Advanced Parsing

**Scope:**

- Variable expansion (at least detect `$VAR`, `${VAR}`)
- Command substitution recognition
- Here-document handling
- Subshell nesting
- Alias expansion (requires shell integration)

**Implementation:**

- More complex state machine
- Possibly switch to proper parser (nom, pest, lalrpop)
- Shell integration protocol for aliases

### 9.4 Alternative: Declarative Spec Approach

**Consider Fig/Inshellisense model:**

**Pros:**

- Simpler parsing (just whitespace + quotes)
- Community can contribute specs
- Faster to implement
- Easier to maintain

**Cons:**

- Requires spec for every command
- Might miss edge cases
- Less "smart" than full parsing

**Recommendation:**

- **Hybrid approach** - simple tokenizer + declarative specs
- Tokenizer handles shell syntax (quotes, pipes, etc.)
- Specs define command structure
- Best of both worlds

### 9.5 Testing Strategy

**Unit tests:**

- Tokenization accuracy
- Quote handling
- Operator detection
- Cursor position tracking

**Integration tests:**

- Real command lines (git, npm, cargo, etc.)
- Edge cases (nested quotes, escaped characters, etc.)
- Performance benchmarks

**Test corpus:**

```rust
const TEST_CASES: &[(&str, usize, CompletionContext)] = &[
    ("git ", 4, CompletionContext::Subcommand),
    ("git commit -m ", 15, CompletionContext::OptionArgument("-m")),
    ("git commit -m 'hello ", 21, CompletionContext::OptionArgument("-m")),
    ("ls | grep ", 10, CompletionContext::Command),
    ("sudo git checkout ", 18, CompletionContext::Argument(1)),
    // ... hundreds more ...
];
```

---

## 10. References

### Documentation

- [Zsh: Shell Grammar](https://zsh.sourceforge.io/Doc/Release/Shell-Grammar.html)
- [Zsh: Expansion](https://zsh.sourceforge.io/Doc/Release/Expansion.html)
- [Zsh: Completion System](https://zsh.sourceforge.io/Doc/Release/Completion-System.html)
- [POSIX Shell Command Language](https://pubs.opengroup.org/onlinepubs/9699919799/utilities/V3_chap02.html)
- [Bash Reference Manual: Programmable Completion](https://www.gnu.org/software/bash/manual/html_node/Programmable-Completion.html)
- [Fish Shell: Commandline](https://fishshell.com/docs/current/cmds/commandline.html)

### Articles & Guides

- [A Guide to Zsh Completion with Examples](https://thevaluable.dev/zsh-completion-guide-examples/)
- [A Pragmatic Approach to Shell Completion](https://rsteube.github.io/blog/2022/a-pragmatic-approach-to-shell-completion.html)
- [How Bash Completion Works](https://tuzz.tech/blog/how-bash-completion-works)
- [WordSplitting - Greg's Wiki](https://mywiki.wooledge.org/WordSplitting)
- [Quotes - Greg's Wiki](https://mywiki.wooledge.org/Quotes)
- [IFS - Greg's Wiki](https://mywiki.wooledge.org/IFS)

### Technical Specifications

- [ANSI-C Quoting (Bash Reference Manual)](https://www.gnu.org/software/bash/manual/html_node/ANSI_002dC-Quoting.html)
- [Command Substitution - Wikipedia](https://en.wikipedia.org/wiki/Command_substitution)
- [Process Substitution - Wikipedia](https://en.wikipedia.org/wiki/Process_substitution)
- [Process Substitution (Bash Reference Manual)](https://www.gnu.org/software/bash/manual/html_node/Process-Substitution.html)

### Rust Crates

- [shlex - crates.io](https://crates.io/crates/shlex/1.3.0)
- [shlex - docs.rs](https://docs.rs/shlex/latest/shlex/)
- [rust-shlex GitHub](https://github.com/comex/rust-shlex)
- [shell-words - crates.io](https://crates.io/crates/shell-words)
- [shellwords - crates.io](https://crates.io/crates/shellwords)

### Parser Implementation

- [Lexerific - Streaming tokenizer with FSM](https://github.com/loverly/lexerific)
- [LIPS Scheme: Finite-State Machine Lexer](https://lips.js.org/blog/lexer)
- [FSM Application: Lexical Analysis](https://swaminathanj.github.io/fsm/lexer.html)
- [C++ Simple Lexer Using FSM](https://www.programmingnotes.org/4699/cpp-simple-lexer-using-a-finite-state-machine/)

### Related Projects

- [Fig Autocomplete](https://fig.gitbook.io/fig/autocomplete)
- [GitHub - withfig/autocomplete](https://github.com/withfig/autocomplete)
- [GitHub - microsoft/inshellisense](https://github.com/microsoft/inshellisense)
- [bash-completion GitHub](https://github.com/scop/bash-completion)
- [argcomplete - Bash/zsh tab completion for argparse](https://kislyuk.github.io/argcomplete/)

### Shell Resources

- [Bash Heredoc Tutorial](https://phoenixnap.com/kb/bash-heredoc)
- [The Meaning of IFS in Bash Scripting](https://www.baeldung.com/linux/ifs-shell-variable)
- [Globbing Wildcard Characters with Zsh](https://www.techrepublic.com/article/globbing-wildcard-characters-with-zsh/)
- [Mastering Globbing in Zsh](https://www.tweakyourterminal.com/mastering-globbing-in-zsh-an-in-depth-guide/)

---

## Appendix A: Grammar Samples

### A.1 Zsh Simple Command Grammar

```text
simple_command : [assignments] command_word [arguments] [redirections]

assignments    : assignment+
assignment     : NAME=value

command_word   : WORD

arguments      : argument+
argument       : WORD | quoted_word | expansion

redirections   : redirection+
redirection    : [IO_NUMBER] redir_op filename
redir_op       : '<' | '>' | '>>' | '<<<' | '<>' | ...

quoted_word    : SINGLE_QUOTED | DOUBLE_QUOTED | ANSI_C_QUOTED

expansion      : parameter_exp | command_sub | arith_exp | process_sub
parameter_exp  : '$' NAME | '${' param_spec '}'
command_sub    : '$(' command ')' | '`' command '`'
arith_exp      : '$((' expression '))'
process_sub    : '<(' command ')' | '>(' command ')'
```

### A.2 POSIX Token Recognition Pseudo-Code

```text
tokenize(input):
    tokens = []
    current_token = ""
    state = NORMAL

    for each character c in input:
        if state == NORMAL:
            if c is whitespace:
                if current_token not empty:
                    tokens.append(current_token)
                    current_token = ""
            else if c is operator_start:
                if current_token not empty:
                    tokens.append(current_token)
                    current_token = ""
                state = IN_OPERATOR
                current_token = c
            else if c == '\'' or c == '"' or c == '\\':
                state = IN_QUOTE
                current_token += c
            else:
                current_token += c

        else if state == IN_QUOTE:
            # ... handle quoting ...

        else if state == IN_OPERATOR:
            # ... handle operators ...

    if current_token not empty:
        tokens.append(current_token)

    return tokens
```

### A.3 Completion Context Decision Tree

```text
determine_context(tokens, cursor_position):
    cursor_token = find_token_at(cursor_position)

    # Look at previous token
    prev_token = get_previous_token(cursor_token)

    if prev_token is None:
        return COMMAND

    if prev_token is OPERATOR:
        if prev_token in ['|', '&&', '||', ';']:
            return COMMAND
        if prev_token in ['<', '>', '>>']:
            return FILENAME

    if prev_token starts with '-':
        if option_takes_argument(prev_token):
            return OPTION_ARGUMENT

    # Count non-option words
    word_count = 0
    for token in tokens:
        if token is WORD and not starts_with('-'):
            word_count++

    if word_count == 1:
        return SUBCOMMAND
    else:
        return ARGUMENT
```

---

## Appendix B: Test Cases

### B.1 Basic Tokenization

| Input                 | Expected Tokens                  |
| --------------------- | -------------------------------- |
| `echo hello`          | `["echo", "hello"]`              |
| `echo 'hello world'`  | `["echo", "hello world"]`        |
| `echo "hello world"`  | `["echo", "hello world"]`        |
| `echo hello\ world`   | `["echo", "hello world"]`        |
| `git commit -m "fix"` | `["git", "commit", "-m", "fix"]` |

### B.2 Cursor Position

| Input           | Cursor | Expected Context | Expected Prefix |
| --------------- | ------ | ---------------- | --------------- |
| `git`           | 4      | Subcommand       | `""`            |
| `git che`       | 7      | Subcommand       | `"che"`         |
| `git checkout`  | 14     | Argument(1)      | `""`            |
| `git commit -m` | 15     | OptionArg("-m")  | `""`            |
| `ls \| grep`    | 10     | Command          | `""`            |

### B.3 Quoting Edge Cases

| Input                  | Expected Tokens                                 |
| ---------------------- | ----------------------------------------------- |
| `echo 'can'"'"'t'`     | `["echo", "can't"]`                             |
| `echo "hello\"world"`  | `["echo", "hello\"world"]`                      |
| `echo $'hello\nworld'` | `["echo", "hello\nworld"]` (newline literal)    |
| `echo 'unclosed`       | Error or `["echo", "unclosed"]` with quote flag |

### B.4 Operators

| Input            | Expected Tokens            |
| ---------------- | -------------------------- |
| `cmd1 \| cmd2`   | `["cmd1", "\|", "cmd2"]`   |
| `cmd1 && cmd2`   | `["cmd1", "&&", "cmd2"]`   |
| `cmd1 \|\| cmd2` | `["cmd1", "\|\|", "cmd2"]` |
| `cmd1 ; cmd2`    | `["cmd1", ";", "cmd2"]`    |
| `cmd > file`     | `["cmd", ">", "file"]`     |

### B.5 Complex Commands

| Input                      | Expected Parsing                                                 |
| -------------------------- | ---------------------------------------------------------------- |
| `sudo git commit -m "msg"` | Command: git, Subcommand: commit, Options: ["-m"], Args: ["msg"] |
| `ls \| grep foo \| wc -l`  | Pipeline: [ls, grep foo, wc -l]                                  |
| `VAR=val cmd arg`          | Assignment: VAR=val, Command: cmd, Args: [arg]                   |
| `(cd /tmp && ls)`          | Subshell: "cd /tmp && ls"                                        |

---

## Appendix C: Performance Targets

### C.1 Benchmark Goals

| Operation                | Target Time | Max Time |
| ------------------------ | ----------- | -------- |
| Tokenize simple command  | <0.5ms      | 1ms      |
| Tokenize complex command | <2ms        | 5ms      |
| Context analysis         | <1ms        | 2ms      |
| **Total parse time**     | **<3ms**    | **5ms**  |

### C.2 Memory Constraints

- Token buffer: <10KB for typical commands
- Context state: <1KB
- Total memory: <100KB per request

### C.3 Accuracy Goals

| Scenario         | MVP Target | Final Target |
| ---------------- | ---------- | ------------ |
| Simple commands  | 95%+       | 99%+         |
| Quoted arguments | 90%+       | 98%+         |
| Pipe chains      | 85%+       | 95%+         |
| Complex nesting  | 70%+       | 90%+         |
| Edge cases       | 50%+       | 80%+         |

---

**Document End**

_For questions or updates, contact the maintainer or file an issue._

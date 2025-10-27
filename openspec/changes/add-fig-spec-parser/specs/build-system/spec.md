# Build System Specification

## ADDED Requirements

### Requirement: Fig Repository Integration

The build system SHALL clone and reference the withfig/autocomplete repository.

#### Scenario: Submodule initialization

- **WHEN** project is cloned
- **THEN** `git submodule update --init` fetches withfig/autocomplete

#### Scenario: Pinned version

- **WHEN** build runs
- **THEN** specific commit of withfig/autocomplete is used (not HEAD)

### Requirement: TypeScript Parsing

The build system SHALL parse TypeScript completion specs using deno_ast.

#### Scenario: Parse valid spec

- **GIVEN** valid TypeScript spec file `git.ts`
- **WHEN** build system parses file
- **THEN** AST is successfully generated

#### Scenario: Extract subcommands

- **GIVEN** spec with subcommands array
- **WHEN** build system parses spec
- **THEN** all subcommands are extracted

#### Scenario: Extract options

- **GIVEN** spec with options array
- **WHEN** build system parses spec
- **THEN** all options with names and descriptions are extracted

#### Scenario: Handle parse errors

- **GIVEN** malformed TypeScript file
- **WHEN** build system attempts parse
- **THEN** build fails with clear error message indicating file and line

### Requirement: Spec Compilation

The build system SHALL convert parsed specs to MessagePack format.

#### Scenario: Serialize spec

- **GIVEN** parsed git spec
- **WHEN** build system serializes to MessagePack
- **THEN** binary data is generated in OUT_DIR

#### Scenario: Spec index generation

- **WHEN** all specs are compiled
- **THEN** index file is created mapping command names to spec files

#### Scenario: Compression

- **WHEN** comparing MessagePack to JSON
- **THEN** MessagePack size is at least 30% smaller

### Requirement: Binary Embedding

The build system SHALL embed compiled specs in the final binary.

#### Scenario: Include specs

- **WHEN** binary is built
- **THEN** all compiled specs are included via `include_bytes!`

#### Scenario: Binary size

- **WHEN** measuring binary size impact
- **THEN** total increase is less than 10MB for 600+ specs

### Requirement: Build Performance

The build system SHALL complete spec compilation within reasonable time.

#### Scenario: Initial build

- **WHEN** building from clean state
- **THEN** spec compilation completes in less than 60 seconds

#### Scenario: Incremental build

- **WHEN** rebuilding with no spec changes
- **THEN** specs are not recompiled (cache hit)

#### Scenario: Parallel processing

- **WHEN** multiple CPU cores available
- **THEN** specs are parsed in parallel

### Requirement: Build Caching

The build system SHALL cache parsed specs to speed up rebuilds.

#### Scenario: Cache creation

- **WHEN** first build completes
- **THEN** cache directory is created with parsed specs

#### Scenario: Cache invalidation

- **WHEN** Fig spec file changes
- **THEN** only changed spec is reparsed

#### Scenario: Clean build

- **WHEN** `cargo clean` is run
- **THEN** cache is cleared

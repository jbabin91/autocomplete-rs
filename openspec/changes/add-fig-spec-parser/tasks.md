# Implementation Tasks

## 1. Repository Setup

- [ ] 1.1 Clone withfig/autocomplete as git submodule
- [ ] 1.2 Pin to specific commit for reproducibility
- [ ] 1.3 Add submodule init to build instructions

## 2. TypeScript Parsing

- [ ] 2.1 Enable deno_ast in Cargo.toml build-dependencies
- [ ] 2.2 Implement TypeScript file reader in build.rs
- [ ] 2.3 Parse TypeScript AST for Fig.Spec objects
- [ ] 2.4 Extract subcommands, options, args from AST
- [ ] 2.5 Handle generators (dynamic completions) - stub for Phase 3
- [ ] 2.6 Extract descriptions and icons

## 3. Rust Struct Generation

- [ ] 3.1 Define Rust structs matching Fig spec format
  - [ ] CompletionSpec
  - [ ] Subcommand
  - [ ] Option
  - [ ] Argument
  - [ ] Generator (stub)
- [ ] 3.2 Derive Serialize/Deserialize for all types
- [ ] 3.3 Convert parsed TypeScript to Rust structs

## 4. MessagePack Compilation

- [ ] 4.1 Serialize each spec to MessagePack
- [ ] 4.2 Write compiled specs to OUT_DIR
- [ ] 4.3 Generate index file mapping command names to spec files
- [ ] 4.4 Calculate and log total binary size impact

## 5. Spec Embedding

- [ ] 5.1 Use include_bytes! to embed specs in binary
- [ ] 5.2 Create SpecRegistry with lazy loading
- [ ] 5.3 Implement get_spec(command_name) lookup
- [ ] 5.4 Add caching for frequently accessed specs

## 6. Runtime Loading

- [ ] 6.1 Implement spec deserializer from MessagePack
- [ ] 6.2 Create spec lookup API
- [ ] 6.3 Handle spec not found errors
- [ ] 6.4 Add spec validation at load time

## 7. Parser Integration

- [ ] 7.1 Update parser to use SpecRegistry
- [ ] 7.2 Remove hardcoded git spec
- [ ] 7.3 Test with git, npm, docker, cargo specs
- [ ] 7.4 Verify performance is still <5ms

## 8. Build Optimization

- [ ] 8.1 Cache parsed specs to speed up rebuilds
- [ ] 8.2 Only reparse changed spec files
- [ ] 8.3 Parallelize spec parsing
- [ ] 8.4 Add progress reporting during build

## 9. Testing

- [ ] 9.1 Unit tests for TypeScript parser
- [ ] 9.2 Test MessagePack round-trip
- [ ] 9.3 Test spec loading for top 20 CLIs
- [ ] 9.4 Benchmark build time
- [ ] 9.5 Benchmark binary size increase

## 10. Documentation

- [ ] 10.1 Document build process
- [ ] 10.2 Add troubleshooting guide
- [ ] 10.3 Update README with supported CLIs count

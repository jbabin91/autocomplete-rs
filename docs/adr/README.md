# Architecture Decision Records (ADRs)

This directory contains Architecture Decision Records (ADRs) documenting the key
technical decisions made in autocomplete-rs.

## What is an ADR?

An ADR is a document that captures an important architectural decision along
with its context and consequences. ADRs help us:

- Remember why we made certain choices
- Onboard new contributors faster
- Avoid revisiting settled decisions
- Learn from past decisions

## ADR Format

Each ADR follows this structure:

1. **Title** - Short noun phrase (e.g., "Use Rust")
2. **Status** - Proposed, Accepted, Deprecated, Superseded
3. **Context** - What situation led to this decision?
4. **Decision** - What did we decide to do?
5. **Consequences** - What are the results (good and bad)?
6. **Alternatives Considered** - What other options did we evaluate?

## ADR Index

| ADR                                     | Title                   | Status   | Date       |
| --------------------------------------- | ----------------------- | -------- | ---------- |
| [0001](0001-use-rust.md)                | Use Rust                | Accepted | 2025-10-25 |
| [0002](0002-daemon-architecture.md)     | Daemon Architecture     | Accepted | 2025-10-25 |
| [0003](0003-build-time-spec-parsing.md) | Build-time Spec Parsing | Accepted | 2025-10-25 |
| [0004](0004-direct-terminal-control.md) | Direct Terminal Control | Accepted | 2025-10-25 |
| [0005](0005-ratatui-for-tui.md)         | Ratatui for TUI         | Accepted | 2025-10-25 |

## When to Write an ADR

Create an ADR when making decisions about:

- Programming languages or frameworks
- Architectural patterns
- External dependencies
- Data formats or protocols
- Build or deployment processes
- Performance vs complexity tradeoffs

## How to Add an ADR

1. Copy the ADR template
2. Number it sequentially (next number)
3. Fill in all sections
4. Get feedback from team
5. Mark as "Accepted" when decision is final
6. Update this index

## References

- [ADR GitHub Organization](https://adr.github.io/)
- [Documenting Architecture Decisions](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions)

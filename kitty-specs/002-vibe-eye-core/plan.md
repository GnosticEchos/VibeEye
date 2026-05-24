# Implementation Plan: VibeEye Core (Phase 1)
**Branch**: `main` | **Date**: 2026-04-18 | **Spec**: [spec.md](./spec.md)

## Summary
Establish the foundational Rust binary (`vibe-eye`) that embeds the Servo 0.1.0 engine. The implementation focuses on episodic web ingestion (Navigate -> Capture -> Exit) and implements the Sonar pattern for autonomous capability discovery.

## Technical Context
**Language/Version**: Rust 1.80+ (edition 2021)
**Primary Dependencies**: 
- `servo` 0.1.0 (Headless rendering core)
- `clap` 4.5 (CLI command tree)
- `tokio` (Async runtime)
- `serde_json` (Serialization)
**Storage**: N/A (Stateless)
**Testing**: `cargo test` + High-Fidelity Pre-commit Hook.

## Project Structure
```
crates/
├── vibeeye-core/      # Domain types, browser abstractions
├── vibeeye-app/       # Shared library: browser, navigation, content extraction, Sonar
├── vibeeye-cli/       # Thin CLI wrapper (--help-tree, clap commands)
└── vibeeye-mcp/       # Thin MCP wrapper (JSON-RPC server)
```

### Crate Responsibilities
- **vibeeye-core**: Core domain types, error definitions, trait interfaces
- **vibeeye-app**: Browser engine integration, content capture, tool implementations, `SonarDiscovery` trait
- **vibeeye-cli**: Clap-based CLI, `--help-tree` support, thin wrapper over `vibeeye-app`
- **vibeeye-mcp**: MCP server, JSON-RPC transport, thin wrapper over `vibeeye-app`

## High-Fidelity Design: The Sonar Pattern
Every CLI command will implement a `SonarDiscovery` trait to enable the `--help-tree` requirement.
- **Reflection**: Commands return a JSON map of their arguments and metadata.
- **Aggregation**: The conductor recursively traverses the command tree to build the machine-readable "Map of the Eye."

## Implementation Roadmap
1. **WP01: Workspace & Core Init**: Setup workspace `Cargo.toml`, `vibeeye-core` crate with domain types.
2. **WP02: App Library & Sonar Core**: Create `vibeeye-app` crate with `SonarDiscovery` trait and tool registry.
3. **WP03: CLI Thin Interface**: Create `vibeeye-cli` crate with `--help-tree` support via clap.
4. **WP04: MCP Thin Interface**: Create `vibeeye-mcp` crate with JSON-RPC server.
5. **WP05: Browser Engine & Headless Nav**: Integrate Servo 0.1.0 into `vibeeye-app`.
6. **WP06: Content Capture**: Implement Markdown distillation and DOM dump in `vibeeye-app`.
7. **WP07: Integration & Validation**: E2E tests, CLI/MCP parity verification.

## Success Criteria
- [x] `vibe-eye --help-tree -f json` returns high-fidelity map.
- [x] Successful document ingestion from docs.rs.
- [x] Tool exits cleanly with code 0.
- [x] Workspace builds with `cargo build --workspace`.
- [x] CLI and MCP tool parity verified via automated test.

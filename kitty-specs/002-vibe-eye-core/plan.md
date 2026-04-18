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
src/
├── browser/         # Servo WebView lifecycle and navigation
├── cli/             # Command definitions and Sonar logic
├── mcp/             # JSON-RPC tool mappings
├── discovery/       # SonarDiscovery trait and JSON reflection
└── main.rs          # Process conductor
```

## High-Fidelity Design: The Sonar Pattern
Every CLI command will implement a `SonarDiscovery` trait to enable the `--help-tree` requirement.
- **Reflection**: Commands return a JSON map of their arguments and metadata.
- **Aggregation**: The conductor recursively traverses the command tree to build the machine-readable "Map of the Eye."

## Implementation Roadmap
1.  **WP01: Workspace Init**: Setup `Cargo.toml`, add dependencies, and minimal boot loop.
2.  **WP02: Command Tree**: Define `browse`, `follow`, and `help-tree` commands.
3.  **WP03: Servo Integration**: Implement off-screen rendering and sequential navigation.
4.  **WP04: Content Capture**: Implement DOM extraction to Markdown (basic) and raw HTML.
5.  **WP05: MCP Surface**: Expose tools via JSON-RPC.

## Success Criteria
- [x] `vibe-eye --help-tree -f json` returns high-fidelity map.
- [x] Successful document ingestion from docs.rs.
- [x] Tool exits cleanly with code 0.

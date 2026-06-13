# Tasks: VibeEye Core (Phase 1)

This document provides the machine-readable roadmap for the VibeEye workspace.

## WP01: Workspace & Core Init
Setup workspace `Cargo.toml` and create `vibeeye-core` crate with domain types.
- **Dependencies**: None
- **Requirements**: FR-07, C-01
- **Outputs**: `Cargo.toml` (workspace), `crates/vibeeye-core/`

## WP02: App Library & Sonar Core
Create `vibeeye-app` crate with `TypedTool`/`Tool` traits and tool registry pattern.
- **Dependencies**: WP01
- **Requirements**: FR-06, FR-08
- **Outputs**: `crates/vibeeye-app/`, `TypedTool`/`Tool` traits

## WP03: CLI Thin Interface
Create `vibeeye-cli` crate - thin clap wrapper over `vibeeye-app` with `--help-tree`.
- **Dependencies**: WP02
- **Requirements**: FR-06
- **Outputs**: `crates/vibeeye-cli/`, `vibe-eye` binary

## WP04: MCP Thin Interface
Create `vibeeye-mcp` crate - thin MCP server wrapper over `vibeeye-app`.
- **Dependencies**: WP02
- **Requirements**: FR-08
- **Outputs**: `crates/vibeeye-mcp/`, `vibeeye-mcp` binary

## WP05: Browser Engine & Headless Nav
Integrate Servo 0.1.0 into `vibeeye-app` for off-screen rendering.
- **Dependencies**: WP02
- **Requirements**: FR-01, FR-02, FR-03
- **Outputs**: Browser engine module in `vibeeye-app`

## WP06: Content Capture
Implement Markdown distillation and DOM dump in `vibeeye-app`.
- **Dependencies**: WP05
- **Requirements**: FR-04, FR-05
- **Outputs**: Content extraction modules in `vibeeye-app`

## WP07: Integration & Validation
E2E tests and CLI/MCP parity verification.
- **Dependencies**: WP03, WP04, WP06
- **Requirements**: FR-06, FR-08, NFR-02, NFR-03
- **Outputs**: `tests/`, parity test suite

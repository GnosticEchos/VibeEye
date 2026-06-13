# Development Guidelines: VibeEye

## Project Overview
A Rust-native headless browser for agentic content extraction using Servo.
**Tech Stack**: Rust 1.86+, Servo 0.1.0, Clap 4.5, Tokio 1.45, SurrealDB 3.1.

## Architectural Mandates
- **Thin Interface**: Separate the core browser engine (`src/browser/`) from interface handlers (`src/cli/`, `src/mcp/`).
- **Headless First**: Zero dependencies on X11 or Wayland.

## The Sonar Pattern
- **Mandatory**: EVERY new command struct MUST implement the `TypedTool` trait.
- **Reflection**: Commands must return a JSON map of arguments and metadata for autonomous agent discovery.
- **Verification**: `vibe-eye --help-tree -f json` must always return a complete, valid capability map.

## Coding Standards
- **Quality Gate**: Every commit MUST pass `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.
- **Memory Constraint**: Active page loads MUST NOT exceed 500MB RAM.
- **Exit Strategy**: All missions must end with a clean process exit and exit code 0.

## Feature Gating
- **surrealdb feature**: All database operations (persistence, search, import/export).
- **embeddings feature**: Semantic chunking, embedding generation, vector/hybrid search (implies surrealdb).
- Default build (no features) — browser tools + crawl to stdout/directory work without external dependencies.

## Prohibitions
- **NO Chromium**: Do not introduce any WebKit/Chromium-based libraries (Playwright, Puppeteer).
- **NO Magic**: Do not use `.env` files for configuration. Use OS Keyring or CLI arguments.
- **NO Drift**: Implement only what is specified in the active Work Package.

# Development Guidelines: VibeEye

## Project Overview
A Rust-native browser tool for sovereign ingestion using Servo 0.1.0. 
**Tech Stack**: Rust 1.80+, Servo 0.1.0, Clap 4.5.

## Architectural Mandates
- **Thin Interface**: Separate the core browser engine (`src/browser/`) from interface handlers (`src/cli/`, `src/mcp/`).
- **Headless First**: Zero dependencies on X11 or Wayland.

## The Sonar Pattern
- **Mandatory**: EVERY new command struct MUST implement the `SonarDiscovery` trait.
- **Reflection**: Commands must return a JSON map of arguments and metadata for autonomous agent discovery.
- **Verification**: `vibe-eye --help-tree -f json` must always return a complete, valid capability map.

## Coding Standards
- **Quality Gate**: Every commit MUST pass `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.
- **Memory Constraint**: Active page loads MUST NOT exceed 500MB RAM.
- **Exit Strategy**: All missions must end with a clean process exit and exit code 0.

## Prohibitions
- **NO Chromium**: Do not introduce any WebKit/Chromium-based libraries (Playwright, Puppeteer).
- **NO Magic**: Do not use `.env` files for configuration. Use OS Keyring or CLI arguments.
- **NO Drift**: Implement only what is specified in the active Work Package.

# Research: VibeEye Core Architecture

## Decision: Servo 0.1.0 as Primary Engine
- **Rationale**: Rust-native, headless support, and parallel layout (Stylo). Released April 13, 2026.
- **Evidence**: [S01] - Official Servo Release Notes.

## Decision: The Sonar Pattern for Self-Reflection
- **Rationale**: Enables autonomous agent discovery via machine-readable help trees.
- **Evidence**: [S02] - Vibe Platform Governance Doctrine.

## Decision: Standalone Rust Binary
- **Rationale**: Minimal resource footprint compared to Node/Playwright.
- **Evidence**: [S03] - Comparative memory audit (Theoretical: < 500MB).

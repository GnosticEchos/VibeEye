# Tasks: VibeEye Core (Phase 1)

This document provides the machine-readable roadmap for the VibeEye core binary.

## WP01: Workspace & Engine Init
Establish the Rust workspace and perform a minimal boot of the Servo 0.1.0 engine.
- **Dependencies**: None
- **Requirements**: FR-01, C-01

## WP02: Command Tree & Sonar Core
Implement the clap-based command hierarchy and the foundational Sonar capability.
- **Dependencies**: WP01
- **Requirements**: FR-06

## WP03: Headless Navigation
Implement the core browser logic for navigation and DOM dumping.
- **Dependencies**: WP01
- **Requirements**: FR-01, FR-02, FR-03

## WP04: MCP Interface
Expose the browser capabilities via the Model Context Protocol.
- **Dependencies**: WP02, WP03
- **Requirements**: FR-04, FR-05

## WP05: Integration & Help-Tree Validation
Verify the high-fidelity self-reflection capabilities of the final binary.
- **Dependencies**: WP04
- **Requirements**: FR-06, NFR-02

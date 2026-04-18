# Spec: VibeEye Core (Phase 1)
**Status:** [RATIFIED]
**Mission:** 002-vibe-eye-core

## 1. Objective
Establish the foundational Rust binary (`vibe-eye`) that embeds the Servo 0.1.0 engine. The focus is purely on episodic web ingestion: navigating to public technical/legal pages, following basic links, capturing the content, and exiting.

## 2. Functional Requirements
### 2.1 Navigation & Ingestion
- **FR-01**: **Headless Boot**: Use the `servo` 0.1.0 library to initialize an off-screen rendering context.
- **FR-02**: **Sequential Navigation**: Support navigating to a URL and following specific links to reach documentation deep-links.
- **FR-03**: **Process Exit**: The tool MUST exit cleanly once the capture mission is complete.

### 2.2 Content Capture
- **FR-04**: **Markdown Distillation**: Extract the rendered DOM into high-fidelity Markdown for agent context.
- **FR-05**: **DOM Dump**: Support raw HTML/DOM output for diagnostic verification.

### 2.3 Self-Reflection (Sonar)
- **FR-06**: **Help Tree**: Implement the `--help-tree -f json` sonar capability for autonomous agent discovery.

## 3. Non-Functional Requirements
- **NFR-01**: **Resource Efficiency**: Peak memory usage MUST remain < 500MB during page load.
- **NFR-02**: **Zero Externalities**: No dependency on X11, Wayland, or external scraping proxies.

## 4. Constraints
- **C-01**: **Release Affinity**: MUST use Servo 0.1.0.
- **C-02**: **No Authentication**: Login portals and session persistence are out-of-scope for Phase 1.
- **C-03**: **No Layout Reasoning**: Visual layout analysis (e.g., footnote relocation) is deferred to vision workers.

## 5. Success Criteria
- [ ] `vibe-eye --help-tree -f json` returns a machine-readable capability map.
- [ ] Agent can retrieve technical documentation from a modern site (e.g., docs.rs) via command line.
- [ ] Tool exits with code 0 after successful Markdown capture.

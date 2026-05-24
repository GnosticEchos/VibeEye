---
work_package_id: WP04
title: MCP Thin Interface
dependencies:
- WP02
requirement_refs:
- FR-08
planning_base_branch: main
merge_target_branch: main
branch_strategy: Planning artifacts for this feature were generated on main. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into main unless the human explicitly redirects the landing branch.
subtasks: []
history: []
execution_mode: exclusive
owned_files:
- crates/vibeeye-mcp/Cargo.toml
- crates/vibeeye-mcp/src/main.rs
- crates/vibeeye-mcp/src/mcp_server.rs
tags: []
---
# WP04: MCP Thin Interface
Create `vibeeye-mcp` crate - thin MCP server wrapper over `vibeeye-app`.

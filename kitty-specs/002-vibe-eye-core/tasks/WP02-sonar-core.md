---
work_package_id: WP02
title: Command Tree & Sonar Core
dependencies:
- WP01
requirement_refs:
- FR-06
planning_base_branch: main
merge_target_branch: main
branch_strategy: Planning artifacts for this feature were generated on main. During /spec-kitty.implement this WP may branch from a dependency-specific base, but completed changes must merge back into main unless the human explicitly redirects the landing branch.
subtasks: []
history: []
execution_mode: exclusive
owned_files:
- src/cli/**
- src/discovery/**
tags: []
---
# WP02: Command Tree & Sonar Core
Implement the clap-based command hierarchy and the foundational Sonar capability.

# Data Model: VibeEye Core

## Workspace Structure

### Crate: vibeeye-core
Core domain types shared across all workspace crates.

#### Entities

**BrowserContext**
- **Role**: Maintains the Servo rendering loop and page state.
- **Attributes**: viewport_size, user_agent, is_headless.

**NavigationState**
- **Role**: Tracks current URL, history, and pending navigation.
- **Attributes**: current_url, history_stack, pending_url.

### Crate: vibeeye-app
Shared library containing all business logic.

#### Entities

**ToolRegistry**
- **Role**: Registry of all available tools for CLI and MCP.
- **Attributes**: tools: Vec<ToolDefinition>, sonar_metadata.

**SonarDiscovery**
- **Role**: Trait for reflective capability discovery.
- **Attributes**: command_name, description, arguments, metadata.

**RenderedBuffer**
- **Role**: In-memory snapshot of the DOM.
- **Attributes**: html_content, markdown_content, accessibility_tree, timestamp.

### Crate: vibeeye-cli
Thin CLI wrapper - no new entities, delegates to vibeeye-app.

### Crate: vibeeye-mcp
Thin MCP wrapper - no new entities, delegates to vibeeye-app.

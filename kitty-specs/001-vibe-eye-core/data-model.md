# Data Model: VibeEye Core

## Entities

### BrowserContext
- **Role**: Maintains the Servo rendering loop and page state.
- **Attributes**: viewport_size, user_agent, is_headless.

### CommandTree
- **Role**: Recursive registry of all CLI and MCP capabilities.
- **Attributes**: command_name, description, arguments, sonar_metadata.

### RenderedBuffer
- **Role**: In-memory snapshot of the DOM.
- **Attributes**: html_content, accessibility_tree, timestamp.

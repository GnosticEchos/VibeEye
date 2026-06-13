//! WP07: Interface parity test.
//!
//! Verifies that CLI `--help-tree -f json` exposes the same tools as the
//! shared `vibeeye_app::ToolRegistry` (which powers the MCP `tools/list`).

use std::collections::HashMap;

#[test]
fn cli_help_tree_matches_tool_registry() {
    // --- CLI side ---
    let opts = vibeeye_cli::help_tree::HelpTreeOpts {
        output: vibeeye_cli::help_tree::HelpTreeOutputFormat::Json,
        ..Default::default()
    };
    let cli_json =
        vibeeye_cli::help_tree::generate_json_for_path::<vibeeye_cli::cli::Cli>(&opts, &[])
            .unwrap();

    let cli_subcommands = cli_json
        .get("subcommands")
        .and_then(|s| s.as_array())
        .expect("CLI help-tree should have subcommands");

    let mut cli_tools: HashMap<String, String> = HashMap::new();
    for cmd in cli_subcommands {
        let name = cmd
            .get("name")
            .and_then(|n| n.as_str())
            .expect("subcommand missing name")
            .to_string();
        let desc = cmd
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();
        cli_tools.insert(name, desc);
    }

    // --- App / MCP side ---
    let registry = vibeeye_app::tool_registry::ToolRegistry::new();
    let app_tools = registry.discover_all();

    let mut app_tools_map: HashMap<String, String> = HashMap::new();
    for meta in app_tools {
        let name = meta.name;
        let desc = meta.description;
        app_tools_map.insert(name, desc);
    }

    // --- Name mapping: CLI → App ---
    let mapping = [
        ("navigate", "browser_navigate"),
        ("snapshot", "browser_snapshot"),
        ("extract", "browser_extract"),
    ];

    // Verify all mapped CLI subcommands exist (allow extra CLI-only commands like crawl)
    for (cli_name, app_name) in &mapping {
        let cli_desc = cli_tools
            .get(*cli_name)
            .unwrap_or_else(|| panic!("CLI missing subcommand: {cli_name}"));
        let app_desc = app_tools_map
            .get(*app_name)
            .unwrap_or_else(|| panic!("ToolRegistry missing tool: {app_name}"));

        // Descriptions should be semantically aligned (not necessarily identical,
        // but should contain the same core concept).
        assert!(
            cli_desc.to_lowercase().contains(&cli_name.to_lowercase())
                || app_desc.to_lowercase().contains(&cli_name.to_lowercase()),
            "Description mismatch for '{cli_name}' → '{app_name}': CLI='{cli_desc}', APP='{app_desc}'"
        );
    }

    // ToolRegistry should contain exactly the mapped tools (no orphaned MCP tools)
    assert_eq!(
        app_tools_map.len(),
        mapping.len(),
        "ToolRegistry should contain exactly {} tools",
        mapping.len()
    );
}

//! Introspective `--help-tree` for clap-based CLIs.
//!
//! Derived from the HelpTree reference implementation.

use clap::{Args, Command, CommandFactory};
use serde_json::{Value, json};
use std::collections::HashSet;

pub mod render;
pub mod theme;

pub use theme::{
    HelpTreeColor, HelpTreeInvocation, HelpTreeOpts, HelpTreeOutputFormat, HelpTreeStyle,
    HelpTreeTheme,
};

/// Reusable clap arguments for `--help-tree` discovery flags.
///
/// Flatten this into your top-level CLI struct with `#[command(flatten)]`.
#[derive(Clone, Debug, Args)]
pub struct HelpTreeArgs {
    #[arg(
        long = "help-tree",
        help = "Print a recursive command map derived from framework metadata"
    )]
    pub help_tree: bool,

    #[arg(
        long = "tree-depth",
        short = 'L',
        help = "Limit --help-tree recursion depth"
    )]
    pub tree_depth: Option<usize>,

    #[arg(
        long = "tree-ignore",
        short = 'I',
        help = "Exclude subtrees/commands from --help-tree output"
    )]
    pub tree_ignore: Vec<String>,

    #[arg(
        long = "tree-all",
        short = 'a',
        help = "Include hidden subcommands in --help-tree output"
    )]
    pub tree_all: bool,

    #[arg(
        long = "tree-output",
        help = "Output format (text or json)",
        value_enum
    )]
    pub tree_output: Option<HelpTreeOutputFormat>,

    /// Tree text styling mode (`rich` uses bold/italic + optional colour).
    #[arg(long = "tree-style", value_enum, default_value = "rich")]
    pub tree_style: HelpTreeStyle,

    /// Tree colour mode (`auto` uses ANSI colours only on TTY output).
    #[arg(long = "tree-color", value_enum, default_value = "auto")]
    pub tree_color: HelpTreeColor,
}

/// Scan `argv` for `--help-tree` and its related flags.
///
/// Returns `Ok(None)` if `--help-tree` is not present.
pub fn parse_help_tree_invocation(argv: &[String]) -> Result<Option<HelpTreeInvocation>, String> {
    // Fast path: if `--help-tree` is not present, none of the other flags
    // belong to us (e.g. `-f text` is for a subcommand, not help-tree).
    if !argv.iter().any(|a| a == "--help-tree") {
        return Ok(None);
    }

    let mut state = ParseState {
        help_tree: true,
        ..Default::default()
    };

    let mut idx = 0;
    while idx < argv.len() {
        idx = process_one_arg(&mut state, argv, idx)?;
        idx += 1;
    }

    Ok(Some(state.into_invocation()))
}

type FlagHandler =
    fn(&mut ParseState, argv: &[String], idx: usize, arg: &str) -> Result<usize, String>;

static FLAG_HANDLERS: &[(&str, FlagHandler)] = &[
    ("--help-tree", handle_help_tree),
    ("--tree-depth", handle_tree_depth),
    ("-L", handle_tree_depth),
    ("--tree-ignore", handle_tree_ignore),
    ("-I", handle_tree_ignore),
    ("--tree-all", handle_tree_all),
    ("-a", handle_tree_all),
    ("--tree-output", handle_tree_output),
    ("--tree-style", handle_tree_style),
    ("--tree-color", handle_tree_color),
    ("--format", handle_format),
    ("-f", handle_format),
];

fn process_one_arg(state: &mut ParseState, argv: &[String], idx: usize) -> Result<usize, String> {
    let arg = &argv[idx];
    if let Some((_, handler)) = FLAG_HANDLERS
        .iter()
        .find(|&&(token, _)| token == arg.as_str())
    {
        return handler(state, argv, idx, arg);
    }
    if let Some((_, handler)) = FLAG_HANDLERS.iter().find(|&&(token, _)| {
        arg.starts_with(token) && arg.len() > token.len() && arg.as_bytes()[token.len()] == b'='
    }) {
        return handler(state, argv, idx, arg);
    }
    if arg.starts_with('-') {
        Ok(idx)
    } else {
        state.path.push(arg.clone());
        Ok(idx)
    }
}

fn handle_help_tree(
    state: &mut ParseState,
    _argv: &[String],
    idx: usize,
    _arg: &str,
) -> Result<usize, String> {
    state.help_tree = true;
    Ok(idx)
}

fn handle_tree_depth(
    state: &mut ParseState,
    argv: &[String],
    idx: usize,
    arg: &str,
) -> Result<usize, String> {
    let value = parse_usize(argv, idx + 1, arg)?;
    state.depth_limit = Some(value);
    Ok(idx + 1)
}

fn handle_tree_ignore(
    state: &mut ParseState,
    argv: &[String],
    idx: usize,
    arg: &str,
) -> Result<usize, String> {
    let value = parse_string(argv, idx + 1, arg)?;
    state.ignore.push(value);
    Ok(idx + 1)
}

fn handle_tree_all(
    state: &mut ParseState,
    _argv: &[String],
    idx: usize,
    _arg: &str,
) -> Result<usize, String> {
    state.tree_all = true;
    Ok(idx)
}

fn handle_tree_output(
    state: &mut ParseState,
    argv: &[String],
    idx: usize,
    _arg: &str,
) -> Result<usize, String> {
    let value = parse_string(argv, idx + 1, "--tree-output")?;
    state.output = Some(match value.as_str() {
        "text" => HelpTreeOutputFormat::Text,
        "json" => HelpTreeOutputFormat::Json,
        _ => return Err(format!("Invalid --tree-output value: '{value}'")),
    });
    Ok(idx + 1)
}

fn handle_format(
    state: &mut ParseState,
    argv: &[String],
    idx: usize,
    arg: &str,
) -> Result<usize, String> {
    let value = parse_string(argv, idx + 1, arg)?;
    state.output = Some(parse_format_value(value)?);
    Ok(idx + 1)
}

fn handle_tree_style(
    state: &mut ParseState,
    argv: &[String],
    idx: usize,
    arg: &str,
) -> Result<usize, String> {
    let value = if let Some((_, v)) = arg.split_once('=') {
        v.to_string()
    } else {
        parse_string(argv, idx + 1, "--tree-style")?
    };
    state.style = Some(match value.as_str() {
        "plain" => HelpTreeStyle::Plain,
        "rich" => HelpTreeStyle::Rich,
        _ => return Err(format!("Invalid --tree-style value: '{value}'")),
    });
    Ok(if arg.contains('=') { idx } else { idx + 1 })
}

fn handle_tree_color(
    state: &mut ParseState,
    argv: &[String],
    idx: usize,
    arg: &str,
) -> Result<usize, String> {
    let value = if let Some((_, v)) = arg.split_once('=') {
        v.to_string()
    } else {
        parse_string(argv, idx + 1, "--tree-color")?
    };
    state.color = Some(match value.as_str() {
        "auto" => HelpTreeColor::Auto,
        "always" => HelpTreeColor::Always,
        "never" => HelpTreeColor::Never,
        _ => return Err(format!("Invalid --tree-color value: '{value}'")),
    });
    Ok(if arg.contains('=') { idx } else { idx + 1 })
}

#[derive(Default)]
struct ParseState {
    help_tree: bool,
    depth_limit: Option<usize>,
    ignore: Vec<String>,
    tree_all: bool,
    output: Option<HelpTreeOutputFormat>,
    style: Option<HelpTreeStyle>,
    color: Option<HelpTreeColor>,
    path: Vec<String>,
}

impl ParseState {
    fn into_invocation(self) -> HelpTreeInvocation {
        HelpTreeInvocation {
            opts: HelpTreeOpts {
                depth_limit: self.depth_limit,
                ignore: self.ignore,
                tree_all: self.tree_all,
                output: self.output.unwrap_or(HelpTreeOutputFormat::Text),
                style: self.style.unwrap_or(HelpTreeStyle::Rich),
                color: self.color.unwrap_or(HelpTreeColor::Auto),
                theme: HelpTreeTheme::default(),
            },
            path: self.path,
        }
    }
}

fn parse_string(argv: &[String], idx: usize, arg: &str) -> Result<String, String> {
    argv.get(idx)
        .cloned()
        .ok_or_else(|| format!("Missing value for '{arg}'"))
}

fn parse_usize(argv: &[String], idx: usize, arg: &str) -> Result<usize, String> {
    let value = parse_string(argv, idx, arg)?;
    value
        .parse::<usize>()
        .map_err(|_| format!("Invalid value for '{arg}': {value}"))
}

fn parse_format_value(value: String) -> Result<HelpTreeOutputFormat, String> {
    match value.as_str() {
        "json" => Ok(HelpTreeOutputFormat::Json),
        _ => Err(format!("Invalid --format value: '{value}'")),
    }
}

/// Generate the JSON help-tree for the command identified by `requested_path`.
///
/// `CF` is your clap `CommandFactory` derive (usually the top-level CLI struct).
/// An empty `requested_path` renders the full tree from the root.
///
/// This is the pure function used by `run_for_path` and by tests.
pub fn generate_json_for_path<CF: CommandFactory>(
    opts: &HelpTreeOpts,
    requested_path: &[String],
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let mut cmd = CF::command();
    cmd.build();

    let (selected, _resolved_path) = select_command_by_path(&cmd, requested_path);

    let ignore: HashSet<String> = opts.ignore.iter().cloned().collect();
    let omit_help_tree_discovery_flags = !requested_path.is_empty();

    command_to_json(
        selected,
        &ignore,
        opts.tree_all,
        opts.depth_limit,
        0,
        omit_help_tree_discovery_flags,
    )
}

/// Run help-tree rooted at the command identified by `requested_path`.
///
/// Prints the result to stdout.
pub fn run_for_path<CF: CommandFactory>(
    opts: HelpTreeOpts,
    requested_path: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = CF::command();
    cmd.build();

    match opts.output {
        HelpTreeOutputFormat::Json => {
            let value = generate_json_for_path::<CF>(&opts, requested_path)?;
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        HelpTreeOutputFormat::Text => {
            let (selected, _resolved_path) = select_command_by_path(&cmd, requested_path);
            let ignore: HashSet<String> = opts.ignore.iter().cloned().collect();
            println!(
                "{}",
                render::command_to_text(
                    selected,
                    &ignore,
                    opts.tree_all,
                    opts.depth_limit,
                    0,
                    &opts
                )
            );
            println!();
            println!(
                "Use `{} <COMMAND> --help` for full details on arguments and flags.",
                cmd.get_name()
            );
        }
    }

    Ok(())
}

fn select_command_by_path<'a>(root: &'a Command, tokens: &[String]) -> (&'a Command, Vec<String>) {
    let mut current = root;
    let mut resolved = Vec::new();

    for token in tokens {
        let maybe_next = current
            .get_subcommands()
            .find(|sub| sub.get_name() == token.as_str());
        let Some(next) = maybe_next else {
            break;
        };
        resolved.push(next.get_name().to_string());
        current = next;
    }

    (current, resolved)
}

fn command_to_json(
    cmd: &Command,
    ignore: &HashSet<String>,
    tree_all: bool,
    depth_limit: Option<usize>,
    depth: usize,
    omit_help_tree_flags: bool,
) -> Result<Value, Box<dyn std::error::Error>> {
    let mut map = serde_json::Map::new();
    map.insert("name".to_string(), json!(cmd.get_name()));
    map.insert("type".to_string(), json!("command"));
    map.insert(
        "description".to_string(),
        json!(cmd.get_about().map(|a| a.to_string()).unwrap_or_default()),
    );

    let (args, opts) = collect_args_and_opts(cmd, omit_help_tree_flags);
    insert_if_not_empty(&mut map, "arguments", args);
    insert_if_not_empty(&mut map, "options", opts);

    let subcommands = collect_subcommands(
        cmd,
        ignore,
        tree_all,
        depth_limit,
        depth,
        omit_help_tree_flags,
    )?;
    insert_if_not_empty(&mut map, "subcommands", subcommands);

    Ok(Value::Object(map))
}

fn insert_if_not_empty(map: &mut serde_json::Map<String, Value>, key: &str, values: Vec<Value>) {
    if !values.is_empty() {
        map.insert(key.to_string(), Value::Array(values));
    }
}

fn collect_args_and_opts(cmd: &Command, omit_help_tree_flags: bool) -> (Vec<Value>, Vec<Value>) {
    let mut args = Vec::new();
    let mut opts = Vec::new();

    for arg in cmd.get_arguments() {
        if should_skip_arg(arg, omit_help_tree_flags) {
            continue;
        }
        if arg.is_positional() {
            args.push(positional_to_json(arg));
        } else {
            opts.push(option_to_json(arg));
        }
    }

    (args, opts)
}

fn should_skip_arg(arg: &clap::Arg, omit_help_tree_flags: bool) -> bool {
    let id = arg.get_id().as_str();
    id == "help" || id == "version" || (omit_help_tree_flags && id.starts_with("tree_"))
}

fn positional_to_json(arg: &clap::Arg) -> Value {
    let mut map = serde_json::Map::new();
    map.insert("name".to_string(), json!(arg.get_id().to_string()));
    map.insert("type".to_string(), json!("argument"));
    map.insert("required".to_string(), json!(arg.is_required_set()));
    Value::Object(map)
}

fn option_to_json(arg: &clap::Arg) -> Value {
    let mut map = serde_json::Map::new();
    map.insert("name".to_string(), json!(arg.get_id().to_string()));
    map.insert("type".to_string(), json!("option"));
    map.insert("required".to_string(), json!(arg.is_required_set()));
    map.insert("takes_value".to_string(), json!(!arg.is_positional()));
    if let Some(long) = arg.get_long() {
        map.insert("long".to_string(), json!(long));
    }
    if let Some(short) = arg.get_short() {
        map.insert("short".to_string(), json!(format!("{short}")));
    }
    if let Some(default) = arg.get_default_values().first() {
        map.insert("default".to_string(), json!(default.to_string_lossy()));
    }
    if let Some(help) = arg.get_help() {
        map.insert("description".to_string(), json!(help.to_string()));
    }
    Value::Object(map)
}

fn collect_subcommands(
    cmd: &Command,
    ignore: &HashSet<String>,
    tree_all: bool,
    depth_limit: Option<usize>,
    depth: usize,
    omit_help_tree_flags: bool,
) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    if let Some(limit) = depth_limit {
        if depth >= limit {
            return Ok(Vec::new());
        }
    }

    let mut subcommands = Vec::new();
    for sub in cmd.get_subcommands() {
        if should_skip_subcommand(sub, ignore, tree_all) {
            continue;
        }
        subcommands.push(command_to_json(
            sub,
            ignore,
            tree_all,
            depth_limit,
            depth + 1,
            omit_help_tree_flags,
        )?);
    }
    Ok(subcommands)
}

fn should_skip_subcommand(sub: &Command, ignore: &HashSet<String>, tree_all: bool) -> bool {
    sub.get_name() == "help" || ignore.contains(sub.get_name()) || (!tree_all && sub.is_hide_set())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_no_help_tree() {
        let args: Vec<String> = vec!["navigate".to_string(), "https://example.com".to_string()];
        let result = parse_help_tree_invocation(&args).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_help_tree_basic() {
        let args = vec!["--help-tree".to_string()];
        let result = parse_help_tree_invocation(&args).unwrap().unwrap();
        assert!(result.opts.depth_limit.is_none());
        assert_eq!(result.opts.output, HelpTreeOutputFormat::Text);
    }

    #[test]
    fn test_parse_help_tree_json() {
        let args = vec![
            "--help-tree".to_string(),
            "-f".to_string(),
            "json".to_string(),
        ];
        let result = parse_help_tree_invocation(&args).unwrap().unwrap();
        assert_eq!(result.opts.output, HelpTreeOutputFormat::Json);
    }

    #[test]
    fn test_parse_help_tree_depth() {
        let args = vec!["--help-tree".to_string(), "-L".to_string(), "2".to_string()];
        let result = parse_help_tree_invocation(&args).unwrap().unwrap();
        assert_eq!(result.opts.depth_limit, Some(2));
    }

    #[test]
    fn test_parse_help_tree_ignore() {
        let args = vec![
            "--help-tree".to_string(),
            "-I".to_string(),
            "help".to_string(),
        ];
        let result = parse_help_tree_invocation(&args).unwrap().unwrap();
        assert_eq!(result.opts.ignore, vec!["help"]);
    }

    #[test]
    fn test_parse_help_tree_all_flags() {
        let args = vec![
            "--help-tree".to_string(),
            "-a".to_string(),
            "--tree-output".to_string(),
            "json".to_string(),
        ];
        let result = parse_help_tree_invocation(&args).unwrap().unwrap();
        assert!(result.opts.tree_all);
        assert_eq!(result.opts.output, HelpTreeOutputFormat::Json);
    }

    #[test]
    fn test_parse_help_tree_path() {
        let args = vec!["navigate".to_string(), "--help-tree".to_string()];
        let result = parse_help_tree_invocation(&args).unwrap().unwrap();
        assert_eq!(result.path, vec!["navigate"]);
    }

    #[test]
    fn test_parse_invalid_depth() {
        let args = vec![
            "--help-tree".to_string(),
            "-L".to_string(),
            "abc".to_string(),
        ];
        assert!(parse_help_tree_invocation(&args).is_err());
    }

    #[test]
    fn test_select_command_by_path_root() {
        use clap::CommandFactory;
        #[derive(clap::Parser)]
        #[command(name = "test")]
        struct TestCli {}
        let cmd = TestCli::command();
        let (selected, _) = super::select_command_by_path(&cmd, &[]);
        assert_eq!(selected.get_name(), "test");
    }

    #[test]
    fn test_command_to_json_basic() {
        use clap::Command;
        let cmd = Command::new("test").about("A test command");
        let value = command_to_json(&cmd, &HashSet::new(), false, None, 0, false).unwrap();
        assert_eq!(value["name"], "test");
        assert_eq!(value["type"], "command");
        assert_eq!(value["description"], "A test command");
    }

    #[test]
    fn test_command_to_json_with_args() {
        use clap::{Arg, Command};
        let cmd = Command::new("test")
            .arg(Arg::new("url").help("Target URL").required(true))
            .arg(
                Arg::new("verbose")
                    .short('v')
                    .long("verbose")
                    .help("Be verbose"),
            );
        let value = command_to_json(&cmd, &HashSet::new(), false, None, 0, false).unwrap();
        let args = value["arguments"].as_array().unwrap();
        assert_eq!(args.len(), 1);
        assert_eq!(args[0]["name"], "url");

        let opts = value["options"].as_array().unwrap();
        assert_eq!(opts.len(), 1);
        assert_eq!(opts[0]["name"], "verbose");
        assert_eq!(opts[0]["short"], "v");
    }

    #[test]
    fn test_command_to_json_with_subcommands() {
        use clap::Command;
        let cmd = Command::new("parent").subcommand(Command::new("child").about("Child cmd"));
        let value = command_to_json(&cmd, &HashSet::new(), false, None, 0, false).unwrap();
        let subs = value["subcommands"].as_array().unwrap();
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0]["name"], "child");
    }

    #[test]
    fn test_command_to_json_depth_limit() {
        use clap::Command;
        let cmd = Command::new("parent").subcommand(Command::new("child").about("Child cmd"));
        let value = command_to_json(&cmd, &HashSet::new(), false, Some(0), 0, false).unwrap();
        assert!(value.get("subcommands").is_none());
    }

    #[test]
    fn test_command_to_json_omit_flags() {
        use clap::{Arg, Command};
        let cmd = Command::new("test").arg(Arg::new("tree_depth").long("tree-depth"));
        let value = command_to_json(&cmd, &HashSet::new(), false, None, 0, true).unwrap();
        assert!(value.get("options").is_none());
    }

    #[test]
    fn test_collect_args_and_opts_empty() {
        use clap::Command;
        let cmd = Command::new("test");
        let (args, opts) = collect_args_and_opts(&cmd, false);
        assert!(args.is_empty());
        assert!(opts.is_empty());
    }

    #[test]
    fn test_collect_args_and_opts_positional() {
        use clap::{Arg, Command};
        let cmd = Command::new("test").arg(Arg::new("url"));
        let (args, opts) = collect_args_and_opts(&cmd, false);
        assert_eq!(args.len(), 1);
        assert!(opts.is_empty());
    }

    #[test]
    fn test_collect_args_and_opts_skips_help() {
        use clap::{Arg, Command};
        let cmd = Command::new("test")
            .arg(Arg::new("help").long("help"))
            .arg(Arg::new("url"));
        let (args, opts) = collect_args_and_opts(&cmd, false);
        assert_eq!(args.len(), 1);
        assert!(opts.is_empty());
    }

    #[test]
    fn test_insert_if_not_empty() {
        let mut map = serde_json::Map::new();
        insert_if_not_empty(&mut map, "key", vec![]);
        assert!(map.get("key").is_none());
        insert_if_not_empty(&mut map, "key", vec![json!("value")]);
        assert!(map.get("key").is_some());
    }

    #[test]
    fn test_format_flag_line_short_and_long() {
        use clap::{Arg, Command};
        let cmd = Command::new("test").arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Be verbose"),
        );
        let arg = cmd.get_arguments().next().unwrap();
        let opts = HelpTreeOpts::default();
        let line = render::format_flag_line(arg, "", &opts);
        assert!(line.contains("-v"));
        assert!(line.contains("--verbose"));
        assert!(line.contains("Be verbose"));
    }

    #[test]
    fn test_format_flag_line_long_only() {
        use clap::{Arg, Command};
        let cmd = Command::new("test").arg(Arg::new("output").long("output").help("Output path"));
        let arg = cmd.get_arguments().next().unwrap();
        let opts = HelpTreeOpts::default();
        let line = render::format_flag_line(arg, "", &opts);
        // Tree prefix contains dashes, so just assert short flag not present as "-o "
        assert!(!line.contains("-o, "));
        assert!(line.contains("--output"));
        assert!(line.contains("Output path"));
    }

    #[test]
    fn test_format_flag_line_with_value_hint() {
        use clap::{Arg, Command, ValueHint};
        let cmd = Command::new("test").arg(
            Arg::new("config")
                .long("config")
                .help("Config file")
                .value_hint(ValueHint::FilePath)
                .value_name("FILE")
                .num_args(1),
        );
        let arg = cmd.get_arguments().next().unwrap();
        let opts = HelpTreeOpts::default();
        let line = render::format_flag_line(arg, "", &opts);
        assert!(line.contains("--config"));
        assert!(line.contains("<")); // value placeholder
        assert!(line.contains("Config file"));
    }

    #[test]
    fn test_format_flag_line_id_fallback() {
        use clap::{Arg, Command};
        let cmd =
            Command::new("test").arg(Arg::new("my-flag").help("A custom flag without short/long"));
        let arg = cmd.get_arguments().next().unwrap();
        let opts = HelpTreeOpts::default();
        let line = render::format_flag_line(arg, "", &opts);
        assert!(line.contains("my-flag"));
        assert!(line.contains("A custom flag without short/long"));
    }

    // ── process_one_arg tests ───────────────────────────────────────────

    #[test]
    fn test_process_help_tree_flag() {
        let mut state = ParseState::default();
        let argv: Vec<String> = vec!["--help-tree".into()];
        let idx = process_one_arg(&mut state, &argv, 0).unwrap();
        assert_eq!(idx, 0);
        assert!(state.help_tree);
    }

    #[test]
    fn test_process_tree_all_flag() {
        let mut state = ParseState::default();
        let argv: Vec<String> = vec!["-a".into()];
        let idx = process_one_arg(&mut state, &argv, 0).unwrap();
        assert_eq!(idx, 0);
        assert!(state.tree_all);
    }

    #[test]
    fn test_process_depth_flag() {
        let mut state = ParseState::default();
        let argv: Vec<String> = vec!["-L".into(), "3".into()];
        let idx = process_one_arg(&mut state, &argv, 0).unwrap();
        assert_eq!(idx, 1);
        assert_eq!(state.depth_limit, Some(3));
    }

    #[test]
    fn test_process_ignore_flag() {
        let mut state = ParseState::default();
        let argv: Vec<String> = vec!["-I".into(), "help".into()];
        let idx = process_one_arg(&mut state, &argv, 0).unwrap();
        assert_eq!(idx, 1);
        assert_eq!(state.ignore, vec!["help"]);
    }

    #[test]
    fn test_process_format_flag() {
        let mut state = ParseState::default();
        let argv: Vec<String> = vec!["-f".into(), "json".into()];
        let idx = process_one_arg(&mut state, &argv, 0).unwrap();
        assert_eq!(idx, 1);
        assert_eq!(state.output, Some(HelpTreeOutputFormat::Json));
    }

    #[test]
    fn test_process_tree_output_flag() {
        let mut state = ParseState::default();
        let argv: Vec<String> = vec!["--tree-output".into(), "json".into()];
        let idx = process_one_arg(&mut state, &argv, 0).unwrap();
        assert_eq!(idx, 1);
        assert_eq!(state.output, Some(HelpTreeOutputFormat::Json));
    }

    #[test]
    fn test_process_unknown_flag_ignored() {
        let mut state = ParseState::default();
        let argv: Vec<String> = vec!["--unknown-flag".into()];
        let idx = process_one_arg(&mut state, &argv, 0).unwrap();
        assert_eq!(idx, 0);
    }

    #[test]
    fn test_process_path_token() {
        let mut state = ParseState::default();
        let argv: Vec<String> = vec!["navigate".into()];
        let idx = process_one_arg(&mut state, &argv, 0).unwrap();
        assert_eq!(idx, 0);
        assert_eq!(state.path, vec!["navigate"]);
    }
}

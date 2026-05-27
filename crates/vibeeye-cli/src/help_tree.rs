//! Introspective `--help-tree` for clap-based CLIs.
//!
//! Derived from the HelpTree reference implementation.

use clap::{Args, Command, CommandFactory};
use serde_json::{Value, json};
use std::collections::HashSet;

/// Output format for `--help-tree`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
pub enum HelpTreeOutputFormat {
    Text,
    Json,
}

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
}

/// Parsed result from scanning argv for `--help-tree`.
#[derive(Clone, Debug)]
pub struct HelpTreeInvocation {
    pub opts: HelpTreeOpts,
    pub path: Vec<String>,
}

/// Options controlling help-tree behavior.
#[derive(Clone, Debug)]
pub struct HelpTreeOpts {
    pub depth_limit: Option<usize>,
    pub ignore: Vec<String>,
    pub tree_all: bool,
    pub output: HelpTreeOutputFormat,
}

impl Default for HelpTreeOpts {
    fn default() -> Self {
        Self {
            depth_limit: None,
            ignore: Vec::new(),
            tree_all: false,
            output: HelpTreeOutputFormat::Text,
        }
    }
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

/// Process a single argv token and any following value tokens.
/// Returns the index of the last consumed token.
fn process_one_arg(
    state: &mut ParseState,
    argv: &[String],
    idx: usize,
) -> Result<usize, String> {
    let arg = &argv[idx];
    match arg.as_str() {
        "--help-tree" => {
            state.help_tree = true;
            Ok(idx)
        }
        "--tree-depth" | "-L" => {
            parse_usize_arg(state, argv, idx, arg, |s, v| s.depth_limit = Some(v))
        }
        "--tree-ignore" | "-I" => {
            parse_string_arg(state, argv, idx, arg, |s, v| s.ignore.push(v))
        }
        "--tree-all" | "-a" => {
            state.tree_all = true;
            Ok(idx)
        }
        "--tree-output" => parse_tree_output_arg(state, argv, idx),
        "--format" | "-f" => parse_format_arg(state, argv, idx, arg),
        token if token.starts_with('-') => Ok(idx),
        token => {
            state.path.push(token.to_string());
            Ok(idx)
        }
    }
}

fn parse_usize_arg(
    state: &mut ParseState,
    argv: &[String],
    idx: usize,
    arg: &str,
    setter: impl FnOnce(&mut ParseState, usize),
) -> Result<usize, String> {
    let next = idx + 1;
    let value = parse_usize(argv, next, arg)?;
    setter(state, value);
    Ok(next)
}

fn parse_string_arg(
    state: &mut ParseState,
    argv: &[String],
    idx: usize,
    arg: &str,
    setter: impl FnOnce(&mut ParseState, String),
) -> Result<usize, String> {
    let next = idx + 1;
    let value = parse_string(argv, next, arg)?;
    setter(state, value);
    Ok(next)
}

fn parse_tree_output_arg(
    state: &mut ParseState,
    argv: &[String],
    idx: usize,
) -> Result<usize, String> {
    let next = idx + 1;
    let value = parse_tree_output(argv, next)?;
    state.output = Some(value);
    Ok(next)
}

fn parse_format_arg(
    state: &mut ParseState,
    argv: &[String],
    idx: usize,
    arg: &str,
) -> Result<usize, String> {
    let next = idx + 1;
    let value = parse_string(argv, next, arg)?;
    state.output = Some(parse_format_value(value)?);
    Ok(next)
}

#[derive(Default)]
struct ParseState {
    help_tree: bool,
    depth_limit: Option<usize>,
    ignore: Vec<String>,
    tree_all: bool,
    output: Option<HelpTreeOutputFormat>,
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

fn parse_tree_output(argv: &[String], idx: usize) -> Result<HelpTreeOutputFormat, String> {
    let value = parse_string(argv, idx, "--tree-output")?;
    match value.as_str() {
        "text" => Ok(HelpTreeOutputFormat::Text),
        "json" => Ok(HelpTreeOutputFormat::Json),
        _ => Err(format!("Invalid --tree-output value: '{value}'")),
    }
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
                command_to_text(selected, &ignore, opts.tree_all, opts.depth_limit, 0)
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

const TREE_ALIGN_WIDTH: usize = 28;
const MIN_DOTS: usize = 4;

fn command_to_text(
    cmd: &Command,
    ignore: &HashSet<String>,
    tree_all: bool,
    depth_limit: Option<usize>,
    depth: usize,
) -> String {
    let mut lines = Vec::new();
    render_command_text(cmd, ignore, tree_all, depth_limit, depth, &mut lines, "");
    lines.join("\n")
}

fn render_command_text(
    cmd: &Command,
    ignore: &HashSet<String>,
    tree_all: bool,
    depth_limit: Option<usize>,
    depth: usize,
    lines: &mut Vec<String>,
    prefix: &str,
) {
    if let Some(limit) = depth_limit {
        if depth > limit {
            return;
        }
    }
    if ignore.contains(cmd.get_name()) {
        return;
    }

    lines.push(format_command_line(cmd, prefix, depth));

    let flags: Vec<_> = cmd
        .get_arguments()
        .filter(|a| !a.is_positional())
        .filter(|a| {
            let id = a.get_id().as_str();
            id != "help" && id != "version"
        })
        .collect();

    let subcommands: Vec<_> = cmd
        .get_subcommands()
        .filter(|sub| tree_all || !sub.is_hide_set())
        .filter(|sub| !ignore.contains(sub.get_name()))
        .filter(|sub| sub.get_name() != "help")
        .collect();

    let total_children = flags.len() + subcommands.len();

    for (i, arg) in flags.iter().enumerate() {
        let is_last = i == total_children - 1;
        let next_prefix = next_tree_prefix(prefix, depth, is_last);
        lines.push(format_flag_line(arg, &next_prefix));
    }

    for (i, sub) in subcommands.iter().enumerate() {
        let idx = flags.len() + i;
        let is_last = idx == total_children - 1;
        let next_prefix = next_tree_prefix(prefix, depth, is_last);
        render_command_text(
            sub,
            ignore,
            tree_all,
            depth_limit,
            depth + 1,
            lines,
            &next_prefix,
        );
    }
}

fn format_command_line(cmd: &Command, prefix: &str, depth: usize) -> String {
    let mut line = prefix.to_string();
    if depth > 0 {
        line.push_str("├── ");
    }
    line.push_str(cmd.get_name());

    let arg_names: Vec<String> = cmd
        .get_positionals()
        .map(|a| format!("<{}>", a.get_id()))
        .collect();
    if !arg_names.is_empty() {
        line.push(' ');
        line.push_str(&arg_names.join(" "));
    }

    let desc = cmd.get_about().map(|a| a.to_string()).unwrap_or_default();
    pad_and_append_description(&mut line, &desc);
    line
}

fn format_flag_line(arg: &clap::Arg, prefix: &str) -> String {
    let mut line = prefix.to_string();
    line.push_str("├── ");

    let mut parts = Vec::new();
    if let Some(short) = arg.get_short() {
        parts.push(format!("-{short}"));
    }
    if let Some(long) = arg.get_long() {
        parts.push(format!("--{long}"));
    }
    if parts.is_empty() {
        parts.push(arg.get_id().to_string());
    }

    line.push_str(&parts.join(", "));

    let value_names = arg.get_value_names();
    let has_value = arg.get_value_hint() != clap::ValueHint::Unknown
        || arg.get_num_args().is_some_and(|n| n.min_values() > 0);
    if has_value {
        if let Some(names) = value_names {
            if let Some(name) = names.iter().next() {
                line.push(' ');
                line.push_str(&format!("<{name}>"));
            }
        }
    }

    let desc = arg.get_help().map(|h| h.to_string()).unwrap_or_default();
    pad_and_append_description(&mut line, &desc);
    line
}

fn pad_and_append_description(line: &mut String, desc: &str) {
    let content_len = line.len();
    let padding_needed = TREE_ALIGN_WIDTH.saturating_sub(content_len);
    let dots = padding_needed.max(MIN_DOTS);
    line.push_str(&" ".repeat(dots));
    line.push(' ');
    line.push_str(desc);
}

fn next_tree_prefix(prefix: &str, depth: usize, is_last: bool) -> String {
    if depth == 0 {
        String::new()
    } else if is_last {
        format!("{prefix}    ")
    } else {
        format!("{prefix}│   ")
    }
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
}

//! Help-tree text rendering with optional ANSI styling.

use super::theme::{HelpTreeOpts, style_text};
use clap::Command;
use std::collections::HashSet;

const TREE_ALIGN_WIDTH: usize = 28;
const MIN_DOTS: usize = 4;

/// Build the full text tree for a command.
pub fn command_to_text(
    cmd: &Command,
    ignore: &HashSet<String>,
    tree_all: bool,
    depth_limit: Option<usize>,
    depth: usize,
    opts: &HelpTreeOpts,
) -> String {
    let mut lines = Vec::new();
    render_command_text(
        cmd,
        ignore,
        tree_all,
        depth_limit,
        depth,
        &mut lines,
        "",
        opts,
    );
    lines.join("\n")
}

/// Recursively render a command and its children (flags + subcommands).
#[allow(clippy::too_many_arguments)]
fn render_command_text(
    cmd: &Command,
    ignore: &HashSet<String>,
    tree_all: bool,
    depth_limit: Option<usize>,
    depth: usize,
    lines: &mut Vec<String>,
    prefix: &str,
    opts: &HelpTreeOpts,
) {
    if let Some(limit) = depth_limit {
        if depth > limit {
            return;
        }
    }
    if ignore.contains(cmd.get_name()) {
        return;
    }

    lines.push(format_command_line(cmd, prefix, depth, opts));

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
        lines.push(format_flag_line(arg, &next_prefix, opts));
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
            opts,
        );
    }
}

/// Render a single command line (name + positionals + description).
fn format_command_line(cmd: &Command, prefix: &str, depth: usize, opts: &HelpTreeOpts) -> String {
    let mut line = prefix.to_string();
    if depth > 0 {
        line.push_str("├── ");
    }

    let name = style_text(cmd.get_name(), &opts.theme.command, opts);
    line.push_str(&name);

    let arg_names: Vec<String> = cmd
        .get_positionals()
        .map(|a| format!("<{}>", a.get_id()))
        .collect();
    if !arg_names.is_empty() {
        let suffix = style_text(
            &format!(" {}", arg_names.join(" ")),
            &opts.theme.options,
            opts,
        );
        line.push_str(&suffix);
    }

    let desc = cmd.get_about().map(|a| a.to_string()).unwrap_or_default();
    pad_and_append_description(&mut line, &desc, opts);
    line
}

/// Render a single flag line (short/long + value names + description).
pub(crate) fn format_flag_line(arg: &clap::Arg, prefix: &str, opts: &HelpTreeOpts) -> String {
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

    let meta = style_text(&parts.join(", "), &opts.theme.options, opts);
    line.push_str(&meta);

    let value_names = arg.get_value_names();
    let has_value = arg.get_value_hint() != clap::ValueHint::Unknown
        || arg.get_num_args().is_some_and(|n| n.min_values() > 0);
    if has_value {
        if let Some(names) = value_names {
            if let Some(name) = names.iter().next() {
                let value = style_text(&format!(" <{name}>"), &opts.theme.options, opts);
                line.push_str(&value);
            }
        }
    }

    let desc = arg.get_help().map(|h| h.to_string()).unwrap_or_default();
    pad_and_append_description(&mut line, &desc, opts);
    line
}

/// Append a right-aligned description to a tree line.
fn pad_and_append_description(line: &mut String, desc: &str, opts: &HelpTreeOpts) {
    let content_len = line.len();
    let padding_needed = TREE_ALIGN_WIDTH.saturating_sub(content_len);
    let dots = padding_needed.max(MIN_DOTS);
    line.push_str(&" ".repeat(dots));
    line.push(' ');
    let styled = style_text(desc, &opts.theme.description, opts);
    line.push_str(&styled);
}

/// Compute the prefix for the next tree level.
fn next_tree_prefix(prefix: &str, depth: usize, is_last: bool) -> String {
    if depth == 0 {
        String::new()
    } else if is_last {
        format!("{prefix}    ")
    } else {
        format!("{prefix}│   ")
    }
}

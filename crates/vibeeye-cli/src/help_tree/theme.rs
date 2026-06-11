//! Help-tree styling: ANSI emphasis + 24-bit colour support.

use std::io::IsTerminal;

/// Whether to apply text styling at all.
#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HelpTreeStyle {
    /// No bold/italic/colour.
    Plain,
    /// Bold/italic + optional colour.
    Rich,
}

/// When to emit ANSI colour codes.
#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HelpTreeColor {
    /// Colour only when stdout is a TTY.
    Auto,
    /// Always emit colour codes.
    Always,
    /// Never emit colour codes.
    Never,
}

/// Text emphasis variants.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextEmphasis {
    Normal,
    Bold,
    Italic,
    BoldItalic,
}

/// Theme for a single token type (command, option, description).
#[derive(Clone, Debug)]
pub struct TextTokenTheme {
    pub emphasis: TextEmphasis,
    pub color_hex: Option<String>,
}

impl TextTokenTheme {
    pub fn normal() -> Self {
        Self {
            emphasis: TextEmphasis::Normal,
            color_hex: None,
        }
    }
}

/// Full help-tree theme.
#[derive(Clone, Debug)]
pub struct HelpTreeTheme {
    pub command: TextTokenTheme,
    pub options: TextTokenTheme,
    pub description: TextTokenTheme,
}

impl Default for HelpTreeTheme {
    fn default() -> Self {
        Self {
            command: TextTokenTheme {
                emphasis: TextEmphasis::Bold,
                color_hex: Some("#7ee7e6".to_string()),
            },
            options: TextTokenTheme::normal(),
            description: TextTokenTheme {
                emphasis: TextEmphasis::Italic,
                color_hex: Some("#90a2af".to_string()),
            },
        }
    }
}

/// Parsed result from scanning argv for `--help-tree`.
#[derive(Clone, Debug)]
pub struct HelpTreeInvocation {
    pub opts: HelpTreeOpts,
    pub path: Vec<String>,
}

/// Options controlling help-tree behaviour.
#[derive(Clone, Debug)]
pub struct HelpTreeOpts {
    pub depth_limit: Option<usize>,
    pub ignore: Vec<String>,
    pub tree_all: bool,
    pub output: HelpTreeOutputFormat,
    pub style: HelpTreeStyle,
    pub color: HelpTreeColor,
    pub theme: HelpTreeTheme,
}

impl Default for HelpTreeOpts {
    fn default() -> Self {
        Self {
            depth_limit: None,
            ignore: Vec::new(),
            tree_all: false,
            output: HelpTreeOutputFormat::Text,
            style: HelpTreeStyle::Rich,
            color: HelpTreeColor::Auto,
            theme: HelpTreeTheme::default(),
        }
    }
}

/// Output format for `--help-tree`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
pub enum HelpTreeOutputFormat {
    Text,
    Json,
}

/// Wrap `text` with ANSI sequences according to `token` and `opts`.
pub fn style_text(text: &str, token: &TextTokenTheme, opts: &HelpTreeOpts) -> String {
    if opts.style == HelpTreeStyle::Plain
        || (matches!(token.emphasis, TextEmphasis::Normal) && token.color_hex.is_none())
    {
        return text.to_string();
    }

    let mut codes: Vec<String> = Vec::new();
    match token.emphasis {
        TextEmphasis::Normal => {}
        TextEmphasis::Bold => codes.push("1".to_string()),
        TextEmphasis::Italic => codes.push("3".to_string()),
        TextEmphasis::BoldItalic => {
            codes.push("1".to_string());
            codes.push("3".to_string());
        }
    }

    if should_use_color(opts) {
        if let Some(hex) = token.color_hex.as_deref() {
            if let Some((r, g, b)) = parse_hex_rgb(hex) {
                codes.push(format!("38;2;{r};{g};{b}"));
            }
        }
    }

    if codes.is_empty() {
        text.to_string()
    } else {
        format!("\x1b[{}m{text}\x1b[0m", codes.join(";"))
    }
}

/// Return true when ANSI colour should be emitted.
pub fn should_use_color(opts: &HelpTreeOpts) -> bool {
    match opts.color {
        HelpTreeColor::Always => true,
        HelpTreeColor::Never => false,
        HelpTreeColor::Auto => std::io::stdout().is_terminal(),
    }
}

/// Parse a `#RRGGBB` hex string into RGB triple.
pub fn parse_hex_rgb(color_hex: &str) -> Option<(u8, u8, u8)> {
    let hex = color_hex.trim();
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some((r, g, b))
}

/// Parse an emphasis string from TOML config.
pub fn parse_tree_emphasis(value: Option<&str>, fallback: TextEmphasis) -> TextEmphasis {
    match value.map(|v| v.trim().to_ascii_lowercase()) {
        Some(style) if style == "normal" => TextEmphasis::Normal,
        Some(style) if style == "bold" => TextEmphasis::Bold,
        Some(style) if style == "italic" => TextEmphasis::Italic,
        Some(style)
            if style == "bold_italic"
                || style == "bold-italic"
                || style == "italic_bold"
                || style == "italic-bold" =>
        {
            TextEmphasis::BoldItalic
        }
        _ => fallback,
    }
}

/// Build a `HelpTreeTheme` from optional TOML config, falling back to defaults.
pub fn theme_from_config(
    cfg: Option<&vibeeye_app::config::HelpTreeConfig>,
    base: &HelpTreeTheme,
) -> HelpTreeTheme {
    HelpTreeTheme {
        command: token_theme_from_config(cfg.and_then(|v| v.command.as_ref()), &base.command),
        options: token_theme_from_config(cfg.and_then(|v| v.options.as_ref()), &base.options),
        description: token_theme_from_config(
            cfg.and_then(|v| v.description.as_ref()),
            &base.description,
        ),
    }
}

fn token_theme_from_config(
    config: Option<&vibeeye_app::config::TextThemeConfig>,
    fallback: &TextTokenTheme,
) -> TextTokenTheme {
    TextTokenTheme {
        emphasis: parse_tree_emphasis(config.and_then(|c| c.style.as_deref()), fallback.emphasis),
        color_hex: config
            .and_then(|c| c.color.as_ref())
            .cloned()
            .or_else(|| fallback.color_hex.clone()),
    }
}

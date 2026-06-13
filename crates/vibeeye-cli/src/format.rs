//! Format query results for CLI output.

use crate::cli::OutputFormat;

const TABLE_CELL_MAX: usize = 500;
const MARKDOWN_CELL_MAX: usize = 2_000;

pub fn format_value(value: &serde_json::Value, format: OutputFormat) -> String {
    match format {
        OutputFormat::Json => serde_json::to_string_pretty(value).unwrap_or_default(),
        OutputFormat::Table => format_table(value),
        OutputFormat::Markdown => format_markdown(value),
    }
}

fn format_table(value: &serde_json::Value) -> String {
    use comfy_table::Table;

    let mut table = Table::new();

    if let Some(arr) = value.as_array() {
        return format_table_array(&mut table, arr);
    }
    if let Some(obj) = value.as_object() {
        return format_table_object(&mut table, obj);
    }
    format_cell(value)
}

fn format_table_array(table: &mut comfy_table::Table, arr: &[serde_json::Value]) -> String {
    if arr.is_empty() {
        return "(no results)".to_string();
    }
    if let Some(first) = arr.first().and_then(|v| v.as_object()) {
        let headers: Vec<String> = first.keys().cloned().collect();
        table.set_header(headers.clone());
        for item in arr {
            if let Some(obj) = item.as_object() {
                let row: Vec<String> = headers
                    .iter()
                    .map(|h| {
                        obj.get(h)
                            .map(|v| format_cell_with_limit(v, TABLE_CELL_MAX))
                            .unwrap_or_default()
                    })
                    .collect();
                table.add_row(row);
            }
        }
    }
    table.to_string()
}

fn format_table_object(
    table: &mut comfy_table::Table,
    obj: &serde_json::Map<String, serde_json::Value>,
) -> String {
    for (k, v) in obj {
        table.add_row(vec![k.clone(), format_cell_with_limit(v, TABLE_CELL_MAX)]);
    }
    table.to_string()
}

fn format_markdown(value: &serde_json::Value) -> String {
    if let Some(arr) = value.as_array() {
        return format_markdown_array(arr);
    }
    if let Some(obj) = value.as_object() {
        return format_markdown_object(obj);
    }
    format!("{}\n", format_cell_with_limit(value, MARKDOWN_CELL_MAX))
}

fn format_markdown_array(arr: &[serde_json::Value]) -> String {
    if arr.is_empty() {
        return "*(no results)*\n".to_string();
    }
    let mut out = String::new();
    for (i, item) in arr.iter().enumerate() {
        out.push_str(&format!("### Result {}\n\n", i + 1));
        if let Some(obj) = item.as_object() {
            for (k, v) in obj {
                out.push_str(&format!(
                    "- **{}**: {}\n",
                    k,
                    format_cell_with_limit(v, MARKDOWN_CELL_MAX)
                ));
            }
        } else {
            out.push_str(&format!(
                "- {}\n",
                format_cell_with_limit(item, MARKDOWN_CELL_MAX)
            ));
        }
        out.push('\n');
    }
    out
}

fn format_markdown_object(obj: &serde_json::Map<String, serde_json::Value>) -> String {
    let mut out = String::new();
    for (k, v) in obj {
        out.push_str(&format!(
            "- **{}**: {}\n",
            k,
            format_cell_with_limit(v, MARKDOWN_CELL_MAX)
        ));
    }
    out
}

fn truncate_string(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }

    let end = s
        .char_indices()
        .nth(max)
        .map(|(idx, _)| idx)
        .unwrap_or(s.len());
    format!("{}...", &s[..end])
}

fn format_cell(value: &serde_json::Value) -> String {
    format_cell_with_limit(value, TABLE_CELL_MAX)
}

fn format_cell_with_limit(value: &serde_json::Value, max: usize) -> String {
    match value {
        serde_json::Value::String(s) => truncate_string(s, max),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Array(a) => format!("[{} items]", a.len()),
        serde_json::Value::Object(o) => format!("{{{}}} keys", o.len()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_cell_preserves_short_strings() {
        let value = serde_json::Value::String("short".to_string());
        assert_eq!(format_cell_with_limit(&value, 80), "short");
    }

    #[test]
    fn truncate_string_preserves_utf8_boundaries() {
        assert_eq!(truncate_string("aé🙂bc", 3), "aé🙂...");
    }

    #[test]
    fn format_cell_handles_non_string_values() {
        assert_eq!(format_cell(&serde_json::json!(42)), "42");
        assert_eq!(format_cell(&serde_json::json!(true)), "true");
        assert_eq!(format_cell(&serde_json::Value::Null), "null");
        assert_eq!(format_cell(&serde_json::json!([1, 2, 3])), "[3 items]");
        assert_eq!(format_cell(&serde_json::json!({"a": 1})), "{1} keys");
    }

    #[test]
    fn markdown_format_uses_markdown_limit() {
        let long = "x".repeat(MARKDOWN_CELL_MAX + 10);
        let output = format_value(
            &serde_json::json!([{ "chunk_text": long }]),
            OutputFormat::Markdown,
        );

        assert!(output.contains("### Result 1"));
        assert!(output.contains("chunk_text"));
        assert!(output.contains("..."));
        assert!(output.len() < MARKDOWN_CELL_MAX + 250);
    }

    #[test]
    fn table_format_uses_table_limit() {
        let long = "x".repeat(TABLE_CELL_MAX + 10);
        let output = format_value(
            &serde_json::json!([{ "chunk_text": long }]),
            OutputFormat::Table,
        );

        assert!(output.contains("chunk_text"));
        assert!(output.contains("..."));
        assert!(output.len() < 3_000);
    }
}

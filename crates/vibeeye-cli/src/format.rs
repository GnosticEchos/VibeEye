//! Format query results for CLI output.

use crate::cli::OutputFormat;

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
                    .map(|h| obj.get(h).map(format_cell).unwrap_or_default())
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
        table.add_row(vec![k.clone(), format_cell(v)]);
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
    format!("{}\n", format_cell(value))
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
                out.push_str(&format!("- **{}**: {}\n", k, format_cell(v)));
            }
        } else {
            out.push_str(&format!("- {}\n", format_cell(item)));
        }
        out.push('\n');
    }
    out
}

fn format_markdown_object(obj: &serde_json::Map<String, serde_json::Value>) -> String {
    let mut out = String::new();
    for (k, v) in obj {
        out.push_str(&format!("- **{}**: {}\n", k, format_cell(v)));
    }
    out
}

fn truncate_string(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max])
    } else {
        s.to_string()
    }
}

fn format_cell(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => truncate_string(s, 80),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Array(a) => format!("[{} items]", a.len()),
        serde_json::Value::Object(o) => format!("{{{}}} keys", o.len()),
    }
}

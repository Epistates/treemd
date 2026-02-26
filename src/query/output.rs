//! Output formatting for query results.

use super::OutputFormat;
use super::value::Value;

/// Format query results according to the specified format.
pub fn format(values: &[Value], format: OutputFormat) -> String {
    match format {
        OutputFormat::Plain => format_plain(values),
        OutputFormat::Json => format_json(values, false),
        OutputFormat::JsonPretty => format_json(values, true),
        OutputFormat::JsonLines => format_json_lines(values),
        OutputFormat::Markdown => format_markdown(values),
        OutputFormat::Tree => format_tree(values),
    }
}

fn format_plain(values: &[Value]) -> String {
    values
        .iter()
        .map(format_plain_value)
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_plain_value(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => {
            if n.fract() == 0.0 {
                (*n as i64).to_string()
            } else {
                n.to_string()
            }
        }
        Value::String(s) => s.clone(),
        Value::Array(a) => a
            .iter()
            .map(format_plain_value)
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Object(o) => o
            .iter()
            .map(|(k, v)| format!("{}: {}", k, format_plain_value(v)))
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Heading(h) => {
            format!("{} {}", "#".repeat(h.level as usize), h.text)
        }
        Value::Code(c) => {
            let lang = c.language.as_deref().unwrap_or("");
            format!("```{}\n{}\n```", lang, c.content)
        }
        Value::Link(l) => {
            format!("[{}]({})", l.text, l.url)
        }
        Value::Image(i) => {
            format!("![{}]({})", i.alt, i.src)
        }
        Value::Table(t) => {
            let mut lines = Vec::new();
            lines.push(format!("| {} |", t.headers.join(" | ")));
            lines.push(format!(
                "| {} |",
                t.headers
                    .iter()
                    .map(|_| "---")
                    .collect::<Vec<_>>()
                    .join(" | ")
            ));
            for row in &t.rows {
                lines.push(format!("| {} |", row.join(" | ")));
            }
            lines.join("\n")
        }
        Value::List(l) => l
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let prefix = if l.ordered {
                    format!("{}.", i + 1)
                } else {
                    "-".to_string()
                };
                let checkbox = match item.checked {
                    Some(true) => "[x] ",
                    Some(false) => "[ ] ",
                    None => "",
                };
                format!("{} {}{}", prefix, checkbox, item.content)
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Blockquote(b) => b
            .content
            .lines()
            .map(|line| format!("> {}", line))
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Paragraph(p) => p.content.clone(),
        Value::Document(d) => {
            format!(
                "Document: {} headings, {} words",
                d.heading_count, d.word_count
            )
        }
        Value::FrontMatter(fm) => serde_json::to_string_pretty(fm).unwrap_or_default(),
    }
}

fn format_json(values: &[Value], pretty: bool) -> String {
    // Convert to JSON-compatible structure
    let json_values: Vec<serde_json::Value> = values.iter().map(value_to_json).collect();

    let output = if json_values.len() == 1 {
        json_values.into_iter().next().unwrap()
    } else {
        serde_json::Value::Array(json_values)
    };

    if pretty {
        serde_json::to_string_pretty(&output).unwrap_or_default()
    } else {
        serde_json::to_string(&output).unwrap_or_default()
    }
}

fn format_json_lines(values: &[Value]) -> String {
    values
        .iter()
        .map(|v| serde_json::to_string(&value_to_json(v)).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n")
}

fn value_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Null => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Number(n) => serde_json::json!(n),
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::Array(a) => serde_json::Value::Array(a.iter().map(value_to_json).collect()),
        Value::Object(o) => {
            let map: serde_json::Map<String, serde_json::Value> = o
                .iter()
                .map(|(k, v)| (k.clone(), value_to_json(v)))
                .collect();
            serde_json::Value::Object(map)
        }
        Value::Heading(h) => {
            serde_json::json!({
                "type": "heading",
                "level": h.level,
                "text": h.text,
                "line": h.line,
            })
        }
        Value::Code(c) => {
            serde_json::json!({
                "type": "code",
                "language": c.language,
                "content": c.content,
                "start_line": c.start_line,
                "end_line": c.end_line,
            })
        }
        Value::Link(l) => {
            serde_json::json!({
                "type": "link",
                "text": l.text,
                "url": l.url,
                "link_type": l.link_type.as_str(),
            })
        }
        Value::Image(i) => {
            serde_json::json!({
                "type": "image",
                "alt": i.alt,
                "src": i.src,
                "title": i.title,
            })
        }
        Value::Table(t) => {
            serde_json::json!({
                "type": "table",
                "headers": t.headers,
                "rows": t.rows,
            })
        }
        Value::List(l) => {
            serde_json::json!({
                "type": "list",
                "ordered": l.ordered,
                "items": l.items.iter().map(|i| {
                    serde_json::json!({
                        "content": i.content,
                        "checked": i.checked,
                    })
                }).collect::<Vec<_>>(),
            })
        }
        Value::Blockquote(b) => {
            serde_json::json!({
                "type": "blockquote",
                "content": b.content,
            })
        }
        Value::Paragraph(p) => {
            serde_json::json!({
                "type": "paragraph",
                "content": p.content,
            })
        }
        Value::Document(d) => {
            serde_json::json!({
                "type": "document",
                "heading_count": d.heading_count,
                "word_count": d.word_count,
            })
        }
        Value::FrontMatter(fm) => {
            let map: serde_json::Map<String, serde_json::Value> = fm
                .iter()
                .map(|(k, v)| (k.clone(), value_to_json(v)))
                .collect();
            serde_json::Value::Object(map)
        }
    }
}

fn format_markdown(values: &[Value]) -> String {
    values
        .iter()
        .map(format_markdown_value)
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn format_markdown_value(value: &Value) -> String {
    match value {
        Value::Heading(h) => h.raw_md.clone(),
        Value::Code(c) => {
            let lang = c.language.as_deref().unwrap_or("");
            format!("```{}\n{}\n```", lang, c.content)
        }
        _ => format_plain_value(value),
    }
}

fn format_tree(values: &[Value]) -> String {
    use crate::Config;
    let config = Config::load();
    let compact = config.is_compact_tree();

    let mut output = String::new();

    for (i, value) in values.iter().enumerate() {
        let is_last = i == values.len() - 1;
        format_tree_value(value, "", is_last, compact, &mut output);
    }

    output
}

fn format_tree_value(
    value: &Value,
    prefix: &str,
    is_last: bool,
    compact: bool,
    output: &mut String,
) {
    let (connector, continuation) = if compact {
        // Compact/gapless style
        if is_last {
            ("└──", "   ")
        } else {
            ("├──", "│  ")
        }
    } else {
        // Spaced style (default)
        if is_last {
            ("└─ ", "    ")
        } else {
            ("├─ ", "│   ")
        }
    };
    let child_prefix = format!("{}{}", prefix, continuation);

    match value {
        Value::Heading(h) => {
            output.push_str(&format!(
                "{}{}{} {}\n",
                prefix,
                connector,
                "#".repeat(h.level as usize),
                h.text
            ));
        }
        Value::Array(arr) => {
            output.push_str(&format!("{}{}[\n", prefix, connector));
            for (i, item) in arr.iter().enumerate() {
                format_tree_value(item, &child_prefix, i == arr.len() - 1, compact, output);
            }
            output.push_str(&format!("{}]\n", child_prefix));
        }
        Value::Object(obj) => {
            output.push_str(&format!("{}{}{{\n", prefix, connector));
            let len = obj.len();
            for (i, (k, v)) in obj.iter().enumerate() {
                output.push_str(&format!("{}{}: ", child_prefix, k));
                if matches!(v, Value::Object(_) | Value::Array(_)) {
                    output.push('\n');
                    format_tree_value(
                        v,
                        &format!("{}  ", child_prefix),
                        i == len - 1,
                        compact,
                        output,
                    );
                } else {
                    output.push_str(&format!("{}\n", v.to_text()));
                }
            }
            output.push_str(&format!("{}}}\n", child_prefix));
        }
        _ => {
            output.push_str(&format!("{}{}{}\n", prefix, connector, value.to_text()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::value::HeadingValue;

    #[test]
    fn test_format_plain_heading() {
        let heading = Value::Heading(HeadingValue {
            level: 2,
            text: "Test".to_string(),
            offset: 0,
            line: 1,
            content: String::new(),
            raw_md: "## Test".to_string(),
            index: 0,
        });

        let output = format(&[heading], OutputFormat::Plain);
        assert_eq!(output, "## Test");
    }

    #[test]
    fn test_format_json() {
        let values = vec![Value::Number(42.0), Value::String("hello".to_string())];
        let output = format(&values, OutputFormat::Json);
        assert!(output.contains("42"));
        assert!(output.contains("hello"));
    }
}

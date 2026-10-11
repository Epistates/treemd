//! Output formatting for query results.

use super::OutputFormat;
use super::value::{Value, exact_integer};

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
        Value::Number(n) => match exact_integer(*n) {
            Some(i) => i.to_string(),
            None => n.to_string(),
        },
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
        Value::FrontMatter(_) => serde_json::to_string_pretty(&value.to_json()).unwrap_or_default(),
    }
}

fn format_json(values: &[Value], pretty: bool) -> String {
    // Always an array, whatever the count, so a consumer can parse the
    // output without knowing in advance how many values a query returns.
    // A single result used to print bare, and `.link` versus `.link[0]`
    // changed the shape of the document. `jsonl` is the one-value form.
    let output = serde_json::Value::Array(values.iter().map(Value::to_json).collect());

    if pretty {
        serde_json::to_string_pretty(&output).unwrap_or_default()
    } else {
        serde_json::to_string(&output).unwrap_or_default()
    }
}

fn format_json_lines(values: &[Value]) -> String {
    values
        .iter()
        .map(|v| serde_json::to_string(&v.to_json()).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n")
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
    fn whole_numbers_print_as_integers_and_large_ones_are_not_clamped() {
        let json = |n: f64| format(&[Value::Number(n)], OutputFormat::Json);
        let plain = |n: f64| format(&[Value::Number(n)], OutputFormat::Plain);

        assert_eq!(json(1.0), "[1]");
        assert_eq!(json(-3.0), "[-3]");
        assert_eq!(json(1.5), "[1.5]");
        assert_eq!(plain(2.0), "2");
        assert_eq!(plain(1e20), "100000000000000000000");
        assert_eq!(plain(0.25), "0.25");
    }

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

    /// The JSON shape used to depend on the result count: one value printed
    /// bare, several as an array.
    #[test]
    fn json_output_is_an_array_whatever_the_count() {
        let one = [Value::Number(1.0)];
        let two = [Value::Number(1.0), Value::Number(2.0)];

        assert_eq!(format(&[], OutputFormat::Json), "[]");
        assert_eq!(format(&one, OutputFormat::Json), "[1]");
        assert_eq!(format(&two, OutputFormat::Json), "[1,2]");
        assert_eq!(format(&[], OutputFormat::JsonPretty), "[]");
        assert_eq!(format(&one, OutputFormat::JsonPretty), "[\n  1\n]");
        // jsonl stays one value per line.
        assert_eq!(format(&one, OutputFormat::JsonLines), "1");
        assert_eq!(format(&two, OutputFormat::JsonLines), "1\n2");
    }

    #[test]
    fn test_format_json() {
        let values = vec![Value::Number(42.0), Value::String("hello".to_string())];
        let output = format(&values, OutputFormat::Json);
        assert!(output.contains("42"));
        assert!(output.contains("hello"));
    }
}

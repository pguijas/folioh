//! ReST admonitions in API prose. The input remains data, including unknown kinds.

use folioh_mdx::{code_fence_flags, escape_jsx_attr, escape_mdx};

use crate::headings::shift_headings;

fn directive(line: &str) -> Option<(&str, &str)> {
    let (kind, body) = line.strip_prefix(".. ")?.split_once("::")?;
    (!kind.is_empty()
        && kind
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'))
    .then_some((kind, body.trim()))
}

fn identity(kind: &str) -> (&str, String) {
    let (tone, title) = match kind {
        "note" => ("note", "Note"),
        "warning" => ("warning", "Warning"),
        "tip" => ("tip", "Tip"),
        "seealso" => ("info", "See also"),
        "deprecated" => ("warning", "Deprecated"),
        other => return ("note", format!("Directive: {other}")),
    };
    (tone, title.to_string())
}

fn indentation(line: &str) -> usize {
    line.len() - line.trim_start_matches([' ', '\t']).len()
}

fn plain(text: &str, floor: usize) -> String {
    escape_mdx(&shift_headings(text, floor))
}

/// `None` leaves the ordinary prose path byte-identical. Fenced and indented
/// code is never interpreted as a directive; a directive starts in column zero.
pub(crate) fn render(text: &str, floor: usize) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let fenced = code_fence_flags(&lines);
    let mut parts = Vec::new();
    let (mut cursor, mut start) = (0, 0);
    while cursor < lines.len() {
        let Some((kind, argument)) = (!fenced[cursor])
            .then(|| directive(lines[cursor]))
            .flatten()
        else {
            cursor += 1;
            continue;
        };
        let preceding = lines[start..cursor].join("\n");
        if !preceding.trim().is_empty() {
            parts.push(plain(preceding.trim_matches('\n'), floor));
        }
        let mut end = cursor + 1;
        while end < lines.len() && (lines[end].trim().is_empty() || indentation(lines[end]) > 0) {
            end += 1;
        }
        let body_lines = &lines[cursor + 1..end];
        let indent = body_lines
            .iter()
            .filter(|line| !line.trim().is_empty())
            .map(|line| indentation(line))
            .min()
            .unwrap_or(0);
        let body = body_lines
            .iter()
            .map(|line| {
                if line.trim().is_empty() {
                    ""
                } else {
                    &line[indent..]
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let body = if argument.is_empty() {
            body
        } else {
            format!("{argument}\n{body}")
        };
        let body = body.trim_matches('\n');
        let body = render(body, floor).unwrap_or_else(|| plain(body, floor));
        let (tone, title) = identity(kind);
        parts.push(format!(
            "<Callout type=\"{tone}\" title=\"{}\">\n\n{body}\n\n</Callout>",
            escape_jsx_attr(&title)
        ));
        cursor = end;
        start = end;
    }
    if parts.is_empty() {
        return None;
    }
    let trailing = lines[start..].join("\n");
    if !trailing.trim().is_empty() {
        parts.push(plain(trailing.trim_matches('\n'), floor));
    }
    Some(parts.join("\n\n"))
}

#[cfg(test)]
#[path = "rest_tests.rs"]
mod tests;

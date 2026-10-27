//! NumPy-style sections (`Parameters` underlined with dashes). Port of
//! `docstring_parser/numpydoc.py`.

use std::sync::LazyLock;

use regex::Regex;

use super::{cleandoc, splitlines, Docstring, Meta, ParseError, ParsedStyle};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Param,
    Raises,
    Returns,
    Yields,
    Examples,
    Plain,
    Deprecated,
}

use Kind::*;

const SECTIONS: &[(&str, &str, Kind)] = &[
    ("Parameters", "param", Param),
    ("Params", "param", Param),
    ("Arguments", "param", Param),
    ("Args", "param", Param),
    ("Other Parameters", "other_param", Param),
    ("Other Params", "other_param", Param),
    ("Other Arguments", "other_param", Param),
    ("Other Args", "other_param", Param),
    ("Receives", "receives", Param),
    ("Receive", "receives", Param),
    ("Raises", "raises", Raises),
    ("Raise", "raises", Raises),
    ("Warns", "warns", Raises),
    ("Warn", "warns", Raises),
    ("Attributes", "attribute", Param),
    ("Attribute", "attribute", Param),
    ("Returns", "returns", Returns),
    ("Return", "returns", Returns),
    ("Yields", "yields", Yields),
    ("Yield", "yields", Yields),
    ("Examples", "examples", Examples),
    ("Example", "examples", Examples),
    ("Warnings", "warnings", Plain),
    ("Warning", "warnings", Plain),
    ("See Also", "see_also", Plain),
    ("Related", "see_also", Plain),
    ("Notes", "notes", Plain),
    ("Note", "notes", Plain),
    ("References", "references", Plain),
    ("Reference", "references", Plain),
    ("deprecated", "deprecation", Deprecated),
];

/// One capture group per section, in `SECTIONS` order.
static TITLES: LazyLock<Regex> = LazyLock::new(|| {
    let alternatives: Vec<String> = SECTIONS
        .iter()
        .map(|(title, _, kind)| match kind {
            Deprecated => format!(r"^\.\.\s*({title})\s*::"),
            _ => format!(r"^({title})\s*?\n{}\s*$", "-".repeat(title.len())),
        })
        .collect();
    Regex::new(&format!("(?m){}", alternatives.join("|"))).expect("numpy titles")
});
static KV: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^[^\s].*$").expect("kv"));
static PARAM_KEY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?P<name>.*?)(?:\s*:\s*(?P<type>.*?))?$").expect("param key"));
static PARAM_OPTIONAL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?P<type>.*?)(?:, optional|\(optional\))$").expect("optional"));
static PARAM_DEFAULT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<type>.*?)(?:, default|\(default\))(?: |=| = |= |: )*(?P<value>.*)$")
        .expect("default")
});
// `(?<!\S)` has no lookbehind here: the preceding whitespace (or start) is consumed.
static PARAM_DEFAULT_IN_DESC: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?:^|\s)[Dd]efault(?:s to |(?:\s*(?:is|[=:])\s*|\s+))(?P<value>(?:['"]).*?(?:['"])|[\w\-\.]*\w)"#,
    )
    .expect("default in desc")
});
static RETURN_KEY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:(?P<name>.*?)\s*:\s*)?(?P<type>.*?)$").expect("return key"));

pub(crate) fn parse(text: &str) -> Result<Docstring, ParseError> {
    if text.is_empty() {
        return Ok(Docstring::from_description("", ParsedStyle::Numpy));
    }
    let text = cleandoc(text);
    let (desc_chunk, meta_chunk) = match TITLES.find(&text) {
        Some(m) => text.split_at(m.start()),
        None => (text.as_str(), ""),
    };
    let mut doc = Docstring::from_description(desc_chunk, ParsedStyle::Numpy);

    let matches: Vec<(usize, usize, usize)> = TITLES
        .captures_iter(meta_chunk)
        .map(|c| {
            let whole = c.get(0).expect("match");
            let index = (1..=SECTIONS.len())
                .find(|&i| c.get(i).is_some())
                .expect("one title group matched")
                - 1;
            (whole.start(), whole.end(), index)
        })
        .collect();
    for (j, &(start, end, index)) in matches.iter().enumerate() {
        let next_start = matches.get(j + 1).map_or(meta_chunk.len(), |m| m.0);
        let body = &meta_chunk[end..next_start];
        let (_, key, kind) = SECTIONS[index];
        match kind {
            Param | Raises | Returns | Yields => parse_kv(body, key, kind, &mut doc.meta),
            Examples => parse_examples(body, &mut doc.meta),
            Plain => doc.meta.push(Meta::Other {
                key: key.to_string(),
                description: body.trim().to_string(),
            }),
            Deprecated => {
                doc.meta.push(Meta::Directive {
                    text: meta_chunk[start..next_start].trim().to_string(),
                });
            }
        }
    }
    Ok(doc)
}

/// Keys are the column-zero lines; each value runs to the next key.
fn parse_kv(body: &str, key: &str, kind: Kind, meta: &mut Vec<Meta>) {
    let keys: Vec<(usize, usize)> = KV.find_iter(body).map(|m| (m.start(), m.end())).collect();
    for (j, &(start, end)) in keys.iter().enumerate() {
        let value_end = keys.get(j + 1).map_or(body.len(), |k| k.0);
        let item_key = &body[start..end];
        let value = cleandoc(&body[end..value_end]);
        let description = value.trim().to_string();
        meta.push(match kind {
            Param => param_item(key, item_key, &value, description),
            Raises => Meta::Raises {
                type_name: Some(item_key.to_string()).filter(|k| !k.is_empty()),
                description,
            },
            _ => {
                let caps = RETURN_KEY.captures(item_key);
                Meta::Returns {
                    type_name: caps
                        .as_ref()
                        .and_then(|c| c.name("type"))
                        .map(|m| m.as_str().to_string()),
                    return_name: caps
                        .as_ref()
                        .and_then(|c| c.name("name"))
                        .map(|m| m.as_str().to_string()),
                    description,
                    is_generator: kind == Yields,
                }
            }
        });
    }
}

fn param_item(section_key: &str, key: &str, value: &str, description: String) -> Meta {
    let mut arg_name = String::new();
    let mut type_name: Option<String> = None;
    let mut is_optional = None;
    let mut default = None;
    if let Some(caps) = PARAM_KEY.captures(key) {
        arg_name = caps
            .name("name")
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        if let Some(ty) = caps.name("type") {
            let mut ty = ty.as_str().to_string();
            match PARAM_OPTIONAL.captures(&ty) {
                Some(opt) => {
                    ty = opt["type"].to_string();
                    is_optional = Some(true);
                }
                None => is_optional = Some(false),
            }
            if let Some(def) = PARAM_DEFAULT.captures(&ty) {
                is_optional = Some(true);
                default = Some(def["value"].to_string());
                ty = def["type"].to_string();
            }
            type_name = Some(ty);
        }
    }
    if !value.is_empty() && default.is_none() {
        default = PARAM_DEFAULT_IN_DESC
            .captures(value)
            .map(|c| c["value"].to_string());
    }
    Meta::Param {
        key: section_key.to_string(),
        arg_name,
        type_name,
        description,
        is_optional,
        default,
    }
}

/// Runs of `>>>` lines become the snippet, the following prose the description.
fn parse_examples(body: &str, meta: &mut Vec<Meta>) {
    let dedented = dedent(body);
    let mut lines: std::collections::VecDeque<&str> =
        splitlines(dedented.trim()).into_iter().collect();
    while !lines.is_empty() {
        let mut snippet = Vec::new();
        while lines.front().is_some_and(|l| l.starts_with(">>>")) {
            snippet.push(lines.pop_front().expect("front"));
        }
        let mut description = Vec::new();
        while lines.front().is_some_and(|l| !l.starts_with(">>>")) {
            description.push(lines.pop_front().expect("front"));
        }
        meta.push(Meta::Example {
            snippet: (!snippet.is_empty()).then(|| snippet.join("\n")),
            description: description.join("\n"),
        });
    }
}

/// `textwrap.dedent`: the common leading blank prefix of the non-blank lines is removed.
fn dedent(text: &str) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut margin: Option<&str> = None;
    for line in &lines {
        if line.chars().all(|c| c == ' ' || c == '\t') {
            continue;
        }
        let indent = &line[..line.len() - line.trim_start_matches([' ', '\t']).len()];
        margin = Some(match margin {
            None => indent,
            Some(m) => {
                let common = m
                    .char_indices()
                    .zip(indent.chars())
                    .find(|((_, a), b)| a != b)
                    .map_or(m.len().min(indent.len()), |((i, _), _)| i);
                &m[..common]
            }
        });
    }
    let margin = margin.unwrap_or("");
    lines
        .iter()
        .map(|line| {
            if line.chars().all(|c| c == ' ' || c == '\t') {
                ""
            } else {
                line.strip_prefix(margin).unwrap_or(line)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

//! `/** ... */` comments: the prose before the first tag, then the
//! `@param`, `@returns`, `@yields`, `@throws`, `@deprecated` and `@example` blocks.
//! JSDoc is where a JavaScript signature keeps its types, so this is what
//! fills `ArgIR.ty`, `ReturnIR` and `RaiseIR`.

use folioh_ir::DocstringIR;

/// One `@param`: the name as written, plus whatever the tag declared.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: String,
    pub ty: String,
    pub default: String,
    pub description: String,
}

/// A parsed doc comment.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct JsDoc {
    pub description: String,
    pub params: Vec<Param>,
    pub returns_type: String,
    pub returns_description: String,
    pub is_generator: bool,
    pub throws: Vec<(String, String)>,
    pub deprecated: String,
    pub examples: Vec<String>,
}

impl JsDoc {
    /// The `@param` for a parameter name, if the comment declared one.
    pub fn param(&self, name: &str) -> Option<&Param> {
        self.params.iter().find(|param| param.name == name)
    }

    /// The prose as a docstring: first line short, the rest long, with a
    /// deprecation notice appended as its own paragraph.
    pub fn docstring(&self) -> DocstringIR {
        let (short, rest) = match self.description.split_once('\n') {
            Some((short, rest)) => (short.trim(), rest.trim()),
            None => (self.description.trim(), ""),
        };
        let mut long = Vec::new();
        if !rest.is_empty() {
            long.push(rest.to_string());
        }
        if !self.deprecated.is_empty() {
            long.push(format!("**Deprecated:** {}", self.deprecated));
        }
        DocstringIR {
            short_description: short.to_string(),
            long_description: long.join("\n\n"),
            examples: self.examples.clone(),
            notes: Vec::new(),
        }
    }
}

/// Parse one `/** ... */` comment. A tag outside the documented set is
/// dropped: the guide lists what Folioh reads and promises nothing else.
pub fn parse(raw: &str) -> JsDoc {
    let body = raw
        .trim()
        .trim_start_matches("/**")
        .trim_end_matches("*/")
        .lines()
        .map(strip_star)
        .collect::<Vec<_>>()
        .join("\n");

    let mut description: Vec<&str> = Vec::new();
    let mut blocks: Vec<String> = Vec::new();
    for line in body.lines() {
        if line.trim_start().starts_with('@') {
            blocks.push(line.trim_start().to_string());
        } else if let Some(block) = blocks.last_mut() {
            block.push('\n');
            block.push_str(line);
        } else {
            description.push(line);
        }
    }

    let mut doc = JsDoc {
        description: inline_tags(description.join("\n").trim()),
        ..JsDoc::default()
    };
    for block in &blocks {
        let (tag, ty, rest) = split_tag(block);
        let rest = if tag == "example" {
            rest
        } else {
            inline_tags(&rest)
        };
        match tag {
            "param" => doc.params.push(param(&ty, &rest)),
            "return" | "returns" | "yield" | "yields" => {
                doc.returns_type = ty;
                doc.returns_description = rest;
                doc.is_generator = matches!(tag, "yield" | "yields");
            }
            "throw" | "throws" => doc.throws.push((ty, rest)),
            "deprecated" => {
                doc.deprecated = if rest.is_empty() {
                    "Deprecated.".to_string()
                } else {
                    rest
                }
            }
            "example" => doc.examples.push(rest),
            _ => {}
        }
    }
    doc
}

/// `" * text"` reads as `"text"`: one space after the star belongs to the
/// comment, any further indentation belongs to the author.
fn strip_star(line: &str) -> &str {
    match line.trim_start().strip_prefix('*') {
        Some(rest) => rest.strip_prefix(' ').unwrap_or(rest),
        None => line,
    }
}

/// `@tag {type} rest` into its three pieces; the type is optional.
fn split_tag(block: &str) -> (&str, String, String) {
    let block = block.trim_start_matches('@');
    let (tag, rest) = block
        .split_once(|c: char| c.is_whitespace())
        .unwrap_or((block, ""));
    let rest = rest.trim_start();
    if !rest.starts_with('{') {
        return (tag, String::new(), rest.trim().to_string());
    }
    // The type ends at the brace that closes the first: `{{a: number}}`.
    let mut depth = 0;
    for (at, c) in rest.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    let ty = rest[1..at].trim().to_string();
                    return (tag, ty, rest[at + 1..].trim().to_string());
                }
            }
            _ => {}
        }
    }
    (tag, String::new(), rest.trim().to_string())
}

/// `{@link Target}`, `{@link Target|text}` and `{@link Target text}` (and
/// `@linkcode`, `@linkplain`) as Markdown: the text when there is one, the
/// target as code otherwise, a link when the target is a URL.
fn inline_tags(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("{@link") {
        let Some(len) = rest[start..].find('}') else {
            break;
        };
        out.push_str(&rest[..start]);
        let inner = &rest[start + 2..start + len];
        let (tag, body) = inner
            .split_once(|c: char| c.is_whitespace())
            .unwrap_or((inner, ""));
        if !["link", "linkcode", "linkplain"].contains(&tag) {
            out.push_str(&rest[start..start + len + 1]);
            rest = &rest[start + len + 1..];
            continue;
        }
        let body = body.trim();
        let (target, label) = match body.split_once('|') {
            Some((target, label)) => (target.trim(), label.trim()),
            None => match body.split_once(char::is_whitespace) {
                Some((target, label)) => (target, label.trim()),
                None => (body, ""),
            },
        };
        let url = target.contains("://");
        out.push_str(&match (url, label.is_empty()) {
            (true, true) => format!("[{target}]({target})"),
            (true, false) => format!("[{label}]({target})"),
            (false, true) => format!("`{target}`"),
            (false, false) => label.to_string(),
        });
        rest = &rest[start + len + 1..];
    }
    out.push_str(rest);
    out
}

/// `name - description` or `[name=default] - description`.
fn param(ty: &str, rest: &str) -> Param {
    let (name, rest) = match rest.strip_prefix('[') {
        Some(optional) => optional.split_once(']').unwrap_or((optional, "")),
        None => rest
            .split_once(|c: char| c.is_whitespace())
            .unwrap_or((rest, "")),
    };
    let (name, default) = name.split_once('=').unwrap_or((name, ""));
    let description = rest.trim_start().trim_start_matches('-').trim();
    Param {
        name: name.trim().to_string(),
        ty: ty.to_string(),
        default: default.trim().to_string(),
        description: description.to_string(),
    }
}

#[cfg(test)]
#[path = "jsdoc_tests.rs"]
mod tests;

//! Docstring text to parsed sections: Google, NumPy, ReST and epydoc, with the
//! per-docstring `auto` detection. The style parsers are Rust ports of the
//! `docstring_parser` 0.18.0 library.

mod cleandoc;
mod epydoc;
mod google;
mod numpy;
mod rest;

pub use cleandoc::cleandoc;

use crate::ir::DocstringIR;

/// The configured parser: `auto` detects per docstring, the others are forced.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum DocstringStyle {
    #[default]
    Auto,
    Google,
    Numpy,
}

/// Exact match on `google` | `numpy` | `auto`; anything else is Google.
pub fn resolve_style(configured: &str) -> DocstringStyle {
    match configured {
        "google" => DocstringStyle::Google,
        "numpy" => DocstringStyle::Numpy,
        "auto" => DocstringStyle::Auto,
        _ => DocstringStyle::Google,
    }
}

/// Which parser produced a result (diagnostics only).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ParsedStyle {
    Google,
    Numpy,
    Rest,
    Epydoc,
}

/// The section a parameter item came from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ParamSection {
    Param,
    Attribute,
    OtherParam,
    Receives,
}

/// One documented parameter or attribute.
#[derive(Clone, PartialEq, Debug)]
pub struct DocParam {
    pub arg_name: String,
    pub type_name: Option<String>,
    pub description: String,
    pub is_optional: Option<bool>,
    pub default: Option<String>,
    pub section: ParamSection,
}

/// The first documented return or yield.
#[derive(Clone, PartialEq, Debug)]
pub struct DocReturns {
    pub type_name: Option<String>,
    pub description: String,
    pub is_generator: bool,
    pub return_name: Option<String>,
}

/// One documented exception (NumPy `Warns` items included).
#[derive(Clone, PartialEq, Debug)]
pub struct DocRaises {
    pub type_name: Option<String>,
    pub description: String,
}

/// A parsed docstring; strings are `""` when absent.
#[derive(Clone, PartialEq, Debug)]
pub struct ParsedDocstring {
    pub short_description: String,
    pub long_description: String,
    pub params: Vec<DocParam>,
    pub returns: Option<DocReturns>,
    pub raises: Vec<DocRaises>,
    pub examples: Vec<String>,
    pub notes: Vec<String>,
    pub style: ParsedStyle,
}

/// Parse the already `cleandoc`'ed docstring. Never fails: a malformed docstring
/// degrades to the first-line/rest split with no sections.
pub fn parse(text: &str, style: DocstringStyle) -> ParsedDocstring {
    let result = match style {
        DocstringStyle::Google => google::parse(text),
        DocstringStyle::Numpy => numpy::parse(text),
        DocstringStyle::Auto => parse_auto(text),
    };
    match result {
        Ok(doc) => project(doc),
        Err(_) => fallback(text),
    }
}

/// The `DocstringIR` projection: short, long, examples, notes.
pub fn to_docstring_ir(parsed: &ParsedDocstring) -> DocstringIR {
    DocstringIR {
        short_description: parsed.short_description.clone(),
        long_description: parsed.long_description.clone(),
        examples: parsed.examples.clone(),
        notes: parsed.notes.clone(),
    }
}

/// A style parser rejecting its input; swallowed by `auto` when another style succeeds.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct ParseError(pub(crate) String);

/// The `docstring_parser` model: one ordered `meta` list per docstring.
#[derive(Clone, PartialEq, Debug)]
pub(crate) enum Meta {
    Param {
        key: String,
        arg_name: String,
        type_name: Option<String>,
        description: String,
        is_optional: Option<bool>,
        default: Option<String>,
    },
    Returns {
        type_name: Option<String>,
        description: String,
        is_generator: bool,
        return_name: Option<String>,
    },
    Raises {
        type_name: Option<String>,
        description: String,
    },
    Example {
        snippet: Option<String>,
        description: String,
    },
    /// A directive whose argument and indentation must survive style parsing.
    Directive { text: String },
    /// Any other section (`note`, `notes`, `see_also`, `deprecation`, ...).
    Other { key: String, description: String },
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Docstring {
    pub(crate) short_description: Option<String>,
    pub(crate) long_description: Option<String>,
    pub(crate) meta: Vec<Meta>,
    pub(crate) style: ParsedStyle,
}

impl Docstring {
    /// The description split every style shares: first line, then the rest stripped.
    pub(crate) fn from_description(desc_chunk: &str, style: ParsedStyle) -> Docstring {
        let (short, long) = match desc_chunk.split_once('\n') {
            Some((short, rest)) => (short, Some(rest.trim())),
            None => (desc_chunk, None),
        };
        Docstring {
            short_description: Some(short).filter(|s| !s.is_empty()).map(str::to_string),
            long_description: long.filter(|s| !s.is_empty()).map(str::to_string),
            meta: Vec::new(),
            style,
        }
    }
}

/// ReST, Google, NumPy, epydoc in that order; most meta items wins, earliest on ties.
fn parse_auto(text: &str) -> Result<Docstring, ParseError> {
    let attempts = [
        rest::parse(text),
        google::parse(text),
        numpy::parse(text),
        epydoc::parse(text),
    ];
    let mut best: Option<Docstring> = None;
    let mut last_error = None;
    for attempt in attempts {
        match attempt {
            Ok(doc) => {
                if best.as_ref().is_none_or(|b| doc.meta.len() > b.meta.len()) {
                    best = Some(doc);
                }
            }
            Err(err) => last_error = Some(err),
        }
    }
    best.ok_or_else(|| last_error.expect("four attempts"))
}

fn project(doc: Docstring) -> ParsedDocstring {
    let mut out = ParsedDocstring {
        short_description: doc.short_description.unwrap_or_default(),
        long_description: doc.long_description.unwrap_or_default(),
        params: Vec::new(),
        returns: None,
        raises: Vec::new(),
        examples: Vec::new(),
        notes: Vec::new(),
        style: doc.style,
    };
    for item in doc.meta {
        match item {
            Meta::Param {
                key,
                arg_name,
                type_name,
                description,
                is_optional,
                default,
            } => out.params.push(DocParam {
                arg_name,
                type_name,
                description,
                is_optional,
                default,
                section: param_section(&key),
            }),
            Meta::Returns {
                type_name,
                description,
                is_generator,
                return_name,
            } => {
                if out.returns.is_none() {
                    out.returns = Some(DocReturns {
                        type_name,
                        description,
                        is_generator,
                        return_name,
                    });
                }
            }
            Meta::Raises {
                type_name,
                description,
            } => out.raises.push(DocRaises {
                type_name,
                description,
            }),
            Meta::Example {
                snippet,
                description,
            } => {
                // NumPy `>>>` snippets are kept: the docs promise examples
                // render as code.
                let text = [snippet.as_deref().unwrap_or(""), description.as_str()]
                    .into_iter()
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join("\n");
                let text = text.trim();
                if !text.is_empty() {
                    out.examples.push(text.to_string());
                }
            }
            Meta::Directive { text } => {
                if !out.long_description.is_empty() {
                    out.long_description.push_str("\n\n");
                }
                out.long_description.push_str(&text);
            }
            Meta::Other { key, description } => {
                if key == "note" || key == "notes" {
                    out.notes.push(description);
                } else if let Some(label) = prose_label(&key) {
                    let description = description.trim();
                    if !description.is_empty() {
                        let paragraph = format!("**{label}:** {description}");
                        if !out.long_description.is_empty() {
                            out.long_description.push_str("\n\n");
                        }
                        out.long_description.push_str(&paragraph);
                    }
                }
            }
        }
    }
    plain_roles_everywhere(&mut out);
    out
}

/// Every prose field with its Sphinx roles made plain Markdown.
fn plain_roles_everywhere(out: &mut ParsedDocstring) {
    out.short_description = plain_roles(&out.short_description);
    out.long_description = plain_roles(&out.long_description);
    for param in &mut out.params {
        param.description = plain_roles(&param.description);
    }
    if let Some(returns) = &mut out.returns {
        returns.description = plain_roles(&returns.description);
    }
    for raises in &mut out.raises {
        raises.description = plain_roles(&raises.description);
    }
    for note in &mut out.notes {
        *note = plain_roles(note);
    }
}

/// Sphinx cross-reference roles as Markdown: `` :class:`Foo` `` and
/// `` :py:meth:`~pkg.Foo.bar` `` read as code (`Foo`, `bar`), `` :ref:`text
/// <target>` `` as its text. Anything else keeps its backticks.
pub(crate) fn plain_roles(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find(":`") {
        // The role name runs back from `:` over letters, digits, `-` and `:`.
        let head = &rest[..at];
        let name_start = head
            .char_indices()
            .rev()
            .take_while(|(_, c)| c.is_ascii_alphanumeric() || *c == '-' || *c == ':' || *c == '_')
            .last()
            .map_or(at, |(i, _)| i);
        let role = &head[name_start..];
        let Some(close) = rest[at + 2..].find('`') else {
            break;
        };
        let body = &rest[at + 2..at + 2 + close];
        let after = at + 2 + close + 1;
        if !role.starts_with(':') || role.len() < 2 {
            out.push_str(&rest[..after]);
            rest = &rest[after..];
            continue;
        }
        let name = role
            .trim_start_matches(':')
            .rsplit(':')
            .next()
            .unwrap_or_default();
        let shown = match body.rsplit_once('<') {
            Some((text, target)) if target.ends_with('>') && !text.trim().is_empty() => {
                text.trim().to_string()
            }
            _ => {
                let target = body.trim_start_matches('!');
                match target.strip_prefix('~') {
                    Some(path) => path.rsplit('.').next().unwrap_or(path).to_string(),
                    None => target.to_string(),
                }
            }
        };
        out.push_str(&head[..name_start]);
        if ["ref", "doc", "term", "abbr", "download"].contains(&name) {
            out.push_str(&shown);
        } else {
            out.push('`');
            out.push_str(&shown);
            out.push('`');
        }
        rest = &rest[after..];
    }
    out.push_str(rest);
    out
}

/// The sections that read as prose: each one closes the long description as a
/// labelled paragraph, the way a JSDoc `@deprecated` does. Any other unknown
/// section (`:meta private:`, `@version`) is not documentation.
fn prose_label(key: &str) -> Option<&'static str> {
    match key {
        "deprecation" | "deprecated" => Some("Deprecated"),
        "warnings" | "warning" => Some("Warning"),
        "see_also" | "seealso" | "see" => Some("See also"),
        "references" => Some("References"),
        "todo" => Some("Todo"),
        _ => None,
    }
}

fn param_section(key: &str) -> ParamSection {
    match key {
        "attribute" | "ivar" | "cvar" | "var" => ParamSection::Attribute,
        "other_param" => ParamSection::OtherParam,
        "receives" => ParamSection::Receives,
        _ => ParamSection::Param,
    }
}

/// When the style parser fails: first line, rest, nothing else.
fn fallback(text: &str) -> ParsedDocstring {
    let stripped = text.trim();
    let (short, long) = match stripped.split_once('\n') {
        Some((short, rest)) => (short, rest.trim()),
        None => (stripped, ""),
    };
    ParsedDocstring {
        short_description: short.to_string(),
        long_description: long.to_string(),
        params: Vec::new(),
        returns: None,
        raises: Vec::new(),
        examples: Vec::new(),
        notes: Vec::new(),
        style: ParsedStyle::Google,
    }
}

/// `str.splitlines()`: every line boundary Python recognises, no trailing empty line.
pub(crate) fn splitlines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((i, ch)) = chars.next() {
        let is_break = matches!(
            ch,
            '\n' | '\r'
                | '\x0b'
                | '\x0c'
                | '\x1c'
                | '\x1d'
                | '\x1e'
                | '\u{85}'
                | '\u{2028}'
                | '\u{2029}'
        );
        if !is_break {
            continue;
        }
        lines.push(&text[start..i]);
        let mut end = i + ch.len_utf8();
        if ch == '\r' && chars.peek().is_some_and(|&(_, next)| next == '\n') {
            chars.next();
            end += 1;
        }
        start = end;
    }
    if start < text.len() {
        lines.push(&text[start..]);
    }
    lines
}

/// `first_line + "\n" + cleandoc(rest)` when the description spans lines.
pub(crate) fn clean_continuation(desc: &str) -> String {
    match desc.split_once('\n') {
        Some((first, rest)) => format!("{first}\n{}", cleandoc(rest)),
        None => desc.to_string(),
    }
}

#[cfg(test)]
mod tests;

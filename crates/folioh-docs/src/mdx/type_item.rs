//! One `TypeIR` section: labelled heading, signature fence, prose, the
//! fields/variants tables and its methods.

use folioh_ir::{TypeIR, TypeKind, VarIR};
use folioh_mdx::escape_mdx;

use super::function::render_function;
use super::{prose, tail, Page};
use crate::routes::{member_anchor, type_anchor};

/// A GFM cell: whitespace collapsed, MDX-escaped, pipes escaped.
pub(super) fn cell(text: &str) -> String {
    escape_mdx(&text.split_whitespace().collect::<Vec<_>>().join(" ")).replace('|', "\\|")
}

/// A GFM cell holding code: one line, pipes escaped, nothing else touched
/// (MDX reads a code span literally). A backtick in the text widens the span.
fn code_cell(text: &str) -> String {
    let text = text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace('|', "\\|");
    if text.contains('`') {
        format!("`` {text} ``")
    } else {
        format!("`{text}`")
    }
}

/// A value longer than this is cut: a table cell holds a literal, not a body.
const VALUE_CHARS: usize = 60;

/// A GFM member table: name, type, the value column when any member has one,
/// and the description. `cell` renders the description (MDX-escaped here, plain
/// in the llms writer); name, type and value are code.
pub(crate) fn member_table(label: &str, members: &[VarIR], cell: fn(&str) -> String) -> String {
    let with_value = members.iter().any(|member| !member.value.is_empty());
    let (head, rule) = if with_value {
        (
            format!("| {label} | Type | Value | Description |"),
            "| --- | --- | --- | --- |",
        )
    } else {
        (
            format!("| {label} | Type | Description |"),
            "| --- | --- | --- |",
        )
    };
    let mut rows = vec![head, rule.to_string()];
    rows.extend(members.iter().map(|member| {
        let code = |text: &str| {
            if text.is_empty() {
                String::new()
            } else {
                code_cell(text)
            }
        };
        let value = if with_value {
            format!(" {} |", code(&shorten(&member.value)))
        } else {
            String::new()
        };
        format!(
            "| {} | {} |{value} {} |",
            code_cell(&member.name),
            code(&member.ty),
            cell(&member.description)
        )
    }));
    rows.join("\n")
}

/// A value on one line, cut after `VALUE_CHARS` characters with an ellipsis.
fn shorten(value: &str) -> String {
    let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
    match value.char_indices().nth(VALUE_CHARS) {
        Some((at, _)) => format!("{}…", value[..at].trim_end()),
        None => value,
    }
}

/// The signature fence of a type: its attributes as written, then the item.
/// An impl's bases are the trait it implements, not attributes.
pub(crate) fn type_signature(item: &TypeIR) -> String {
    let attributes = match item.kind {
        TypeKind::Impl => &[][..],
        _ => &item.bases[..],
    };
    attributes
        .iter()
        .map(String::as_str)
        .chain(std::iter::once(item.signature.as_str()))
        .collect::<Vec<_>>()
        .join("\n")
}

/// An impl names both sides of its contract; other types keep their kind label.
pub(crate) fn type_title(item: &TypeIR, name: fn(&str) -> String) -> String {
    if let (TypeKind::Impl, Some(on)) = (item.kind, item.bases.first()) {
        format!("impl {} for {}", name(on), name(&item.name))
    } else {
        format!("{} {}", item.kind.label(), name(&item.name))
    }
}

pub(super) fn render_type(item: &TypeIR, level: usize, page: &Page) -> String {
    let source = if item.source_file.is_empty() {
        String::new()
    } else {
        page.source_link(&item.source_file, item.line_number)
    };
    let id = type_anchor(item);
    let mut parts = vec![format!(
        "{} {}{source} [#{id}]\n",
        "#".repeat(level),
        type_title(item, |name| format!("`{name}`"))
    )];
    if !item.signature.is_empty() {
        parts.push(page.fence(&type_signature(item)));
    }
    parts.extend(prose(&item.docstring, level + 1));
    if !item.fields.is_empty() {
        parts.push(format!("{}\n", member_table("Field", &item.fields, cell)));
    }
    if !item.variants.is_empty() {
        parts.push(format!(
            "{}\n",
            member_table("Variant", &item.variants, cell)
        ));
    }
    parts.extend(
        item.methods.iter().map(|method| {
            render_function(method, level + 1, page, &member_anchor(&id, &method.name))
        }),
    );
    parts.extend(tail(&item.docstring, page));
    parts.join("\n")
}

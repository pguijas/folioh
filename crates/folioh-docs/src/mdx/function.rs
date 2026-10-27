//! One function or method section.

use folioh_ir::signature::display_signature;
use folioh_ir::{ArgIR, ArgKind, FunctionIR, FunctionKind};
use folioh_mdx::escape_mdx;

use super::{prose, render_param_table, tail, Page};

/// `*name` / `**name` for the variadic kinds, the plain name otherwise.
pub(crate) fn display_name(arg: &ArgIR) -> String {
    match arg.kind {
        ArgKind::VarPositional => format!("*{}", arg.name),
        ArgKind::VarKeyword => format!("**{}", arg.name),
        _ => arg.name.clone(),
    }
}

/// ` - description`, escaped; nothing when there is no description.
fn described(description: &str) -> String {
    if description.trim().is_empty() {
        String::new()
    } else {
        format!(" - {}", escape_mdx(description))
    }
}

/// `**Label:** code - description`; a missing type leaves the description
/// alone after the label, and nothing to say leaves no line.
fn labelled(label: &str, code: Option<String>, description: &str) -> Option<String> {
    let text = match code {
        Some(code) => format!("{code}{}", described(description)),
        None if description.trim().is_empty() => return None,
        None => escape_mdx(description),
    };
    Some(format!("**{label}:** {text}\n"))
}

/// The decorators a kind badge already shows.
const KIND_DECORATORS: [&str; 3] = ["property", "staticmethod", "classmethod"];

/// The signature fence text: every decorator or attribute the kind badge does
/// not show, as the source writes it, then the signature.
pub(crate) fn decorated_signature(func: &FunctionIR, language: &str) -> String {
    let is_property = func.kind == FunctionKind::Property;
    let mut lines: Vec<String> = func
        .decorators
        .iter()
        .filter(|decorator| {
            let head = decorator.split('(').next().unwrap_or(decorator);
            !KIND_DECORATORS.contains(&head.rsplit('.').next().unwrap_or(head))
        })
        .map(|decorator| match language {
            "python" => format!("@{decorator}"),
            _ => decorator.clone(),
        })
        .collect();
    lines.push(display_signature(func, !is_property));
    lines.join("\n")
}

/// `id` is the heading id the symbol index points at (`add`, `calculator-add`).
pub(super) fn render_function(func: &FunctionIR, level: usize, page: &Page, id: &str) -> String {
    let is_property = func.kind == FunctionKind::Property;
    let mut parts = Vec::new();
    match func.kind {
        FunctionKind::Staticmethod => parts.push("`@staticmethod`\n".to_string()),
        FunctionKind::Classmethod => parts.push("`@classmethod`\n".to_string()),
        FunctionKind::Property => parts.push("`@property`\n".to_string()),
        FunctionKind::Function | FunctionKind::Method => {}
    }
    parts.push(format!(
        "{} `{}`{} [#{id}]\n",
        "#".repeat(level),
        func.name,
        page.source_link(&func.source_file, func.line_number)
    ));
    parts.push(page.fence(&decorated_signature(func, page.language)));
    parts.extend(prose(&func.docstring, level + 1));
    if !is_property {
        let table = render_param_table(func, page.opts.symbol_index, &page.current_module);
        if !table.is_empty() {
            parts.push(format!("{table}\n"));
        }
    }
    if let Some(returns) = &func.returns {
        let linked = (!returns.ty.is_empty()).then(|| match page.resolve(&returns.ty) {
            Some(href) => format!("[`{}`]({href})", returns.ty),
            None => format!("`{}`", returns.ty),
        });
        let label = if returns.is_generator {
            "Yields"
        } else if is_property {
            "Type"
        } else {
            "Returns"
        };
        parts.extend(labelled(label, linked, &returns.description));
    }
    for raise in &func.raises {
        let exception = (!raise.exception.is_empty()).then(|| format!("`{}`", raise.exception));
        parts.extend(labelled("Raises", exception, &raise.description));
    }
    parts.extend(tail(&func.docstring, page));
    parts.join("\n")
}

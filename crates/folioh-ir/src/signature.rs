//! The Python signature string rendered from `FunctionIR.args`.

use crate::ir::{ArgKind, FunctionIR};

/// Render a Python signature: `def name(params) -> type`, or just `name` without parens.
pub fn render_signature(func: &FunctionIR, show_parens: bool) -> String {
    if !show_parens {
        return func.name.clone();
    }
    let has_var_positional = func.args.iter().any(|a| a.kind == ArgKind::VarPositional);
    let last_positional_only = func
        .args
        .iter()
        .rposition(|a| a.kind == ArgKind::PositionalOnly);

    let mut params: Vec<String> = Vec::with_capacity(func.args.len() + 2);
    let mut keyword_separator_inserted = false;
    for (index, arg) in func.args.iter().enumerate() {
        if arg.kind == ArgKind::KeywordOnly && !keyword_separator_inserted {
            keyword_separator_inserted = true;
            if !has_var_positional {
                params.push("*".to_string());
            }
        }
        let mut part = match arg.kind {
            ArgKind::VarPositional => format!("*{}", arg.name),
            ArgKind::VarKeyword => format!("**{}", arg.name),
            _ => arg.name.clone(),
        };
        if !arg.ty.is_empty() {
            part.push_str(&format!(": {}", arg.ty));
        }
        if let Some(default) = &arg.default {
            part.push_str(&format!(" = {default}"));
        }
        params.push(part);
        if last_positional_only == Some(index) && index + 1 < func.args.len() {
            params.push("/".to_string());
        }
    }

    let prefix = if func.is_async { "async def" } else { "def" };
    let mut signature = format!("{prefix} {}({})", func.name, params.join(", "));
    // A docstring-only `Returns:` has no type, so the signature gets no
    // trailing ` -> ` (omitted by design).
    // A yielded item type is not a return annotation: generators retain
    // only the annotation written in their source signature.
    if let Some(returns) = &func.returns {
        let ty = if returns.is_generator {
            returns.return_annotation.as_deref().unwrap_or("")
        } else {
            &returns.ty
        };
        if !ty.is_empty() {
            signature.push_str(&format!(" -> {ty}"));
        }
    }
    signature
}

/// The raw source signature when a parser recorded one, else `render_signature`.
pub fn display_signature(func: &FunctionIR, show_parens: bool) -> String {
    if !func.signature.is_empty() {
        return func.signature.clone();
    }
    render_signature(func, show_parens)
}

#[cfg(test)]
#[path = "signature_tests.rs"]
mod tests;

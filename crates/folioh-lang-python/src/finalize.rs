//! `ModuleRaw` to `ModuleIR`: parse every docstring once and merge its parameters,
//! returns and raises into the signature data.

use folioh_ir::docstring::{self, DocParam, DocstringStyle, ParamSection, ParsedDocstring};
use folioh_ir::{
    ArgIR, ArgKind, ClassIR, FunctionIR, Language, ModuleIR, RaiseIR, ReturnIR, VarIR,
};

use crate::raw::{ClassRaw, FunctionRaw, ModuleRaw, VarRaw};

/// Finalize a raw module with one resolved docstring style.
pub fn finalize_module(raw: ModuleRaw, style: DocstringStyle) -> ModuleIR {
    let docstring = parse(raw.docstring_raw.as_deref(), style);
    let constants = documented_vars(raw.constants, &docstring.params);
    ModuleIR {
        name: raw.name,
        docstring: docstring::to_docstring_ir(&docstring),
        classes: raw
            .classes
            .into_iter()
            .map(|c| finalize_class(c, style))
            .collect(),
        functions: raw
            .functions
            .into_iter()
            .map(|f| finalize_function(f, style))
            .collect(),
        constants,
        source_file: raw.source_file,
        language: Language::Python,
        types: Vec::new(),
    }
}

fn parse(docstring_raw: Option<&str>, style: DocstringStyle) -> ParsedDocstring {
    docstring::parse(docstring_raw.unwrap_or(""), style)
}

/// The variables a scope declares, described by the `Attributes` section of
/// its docstring (the annotation wins over a documented type), then the
/// documented attributes it never declares, such as those `__init__` sets.
fn documented_vars(declared: Vec<VarRaw>, params: &[DocParam]) -> Vec<VarIR> {
    let attributes: Vec<&DocParam> = params
        .iter()
        .filter(|p| p.section == ParamSection::Attribute)
        .collect();
    let find = |name: &str| attributes.iter().rev().find(|p| p.arg_name == name);
    let mut vars: Vec<VarIR> = declared
        .into_iter()
        .map(|raw| {
            let doc = find(&raw.name);
            VarIR {
                ty: if raw.var_type.is_empty() {
                    doc.and_then(|d| d.type_name.clone()).unwrap_or_default()
                } else {
                    raw.var_type
                },
                description: doc.map(|d| d.description.clone()).unwrap_or_default(),
                name: raw.name,
                value: raw.value,
            }
        })
        .collect();
    for attribute in attributes {
        if vars.iter().all(|v| v.name != attribute.arg_name) {
            vars.push(VarIR {
                name: attribute.arg_name.clone(),
                ty: attribute.type_name.clone().unwrap_or_default(),
                value: String::new(),
                description: attribute.description.clone(),
            });
        }
    }
    vars
}

fn finalize_class(raw: ClassRaw, style: DocstringStyle) -> ClassIR {
    let docstring = parse(raw.docstring_raw.as_deref(), style);
    ClassIR {
        name: raw.name,
        bases: raw.bases,
        decorators: raw.decorators,
        docstring: docstring::to_docstring_ir(&docstring),
        methods: raw
            .methods
            .into_iter()
            .map(|m| finalize_function(m, style))
            .collect(),
        class_vars: documented_vars(raw.class_vars, &docstring.params),
        inner_classes: raw
            .inner_classes
            .into_iter()
            .map(|c| finalize_class(c, style))
            .collect(),
        source_file: raw.source_file,
        line_number: raw.line_number,
    }
}

fn finalize_function(raw: FunctionRaw, style: DocstringStyle) -> FunctionIR {
    let parsed = parse(raw.docstring_raw.as_deref(), style);
    let args = raw
        .args
        .into_iter()
        .map(|a| {
            let doc = doc_param(&parsed.params, &a.name, a.kind);
            let ty = if a.annotation.is_empty() {
                doc.and_then(|d| d.type_name.clone()).unwrap_or_default()
            } else {
                a.annotation
            };
            ArgIR {
                name: a.name,
                ty,
                default: a.default,
                description: doc.map(|d| d.description.clone()).unwrap_or_default(),
                kind: a.kind,
            }
        })
        .collect();
    let returns = if raw.returns_annotation.is_empty() && parsed.returns.is_none() {
        None
    } else {
        let doc = parsed.returns.as_ref();
        let is_generator = doc.is_some_and(|r| r.is_generator);
        Some(ReturnIR {
            return_annotation: (is_generator && !raw.returns_annotation.is_empty())
                .then(|| raw.returns_annotation.clone()),
            ty: if is_generator || raw.returns_annotation.is_empty() {
                doc.and_then(|r| r.type_name.clone()).unwrap_or_default()
            } else {
                raw.returns_annotation
            },
            description: doc.map(|r| r.description.clone()).unwrap_or_default(),
            is_generator,
        })
    };
    let raises = parsed
        .raises
        .iter()
        .map(|r| RaiseIR {
            exception: r.type_name.clone().unwrap_or_default(),
            description: r.description.clone(),
        })
        .collect();
    FunctionIR {
        name: raw.name,
        args,
        returns,
        raises,
        decorators: raw.decorators,
        docstring: docstring::to_docstring_ir(&parsed),
        is_async: raw.is_async,
        source_file: raw.source_file,
        line_number: raw.line_number,
        kind: raw.kind,
        signature: String::new(),
        visibility: String::new(),
    }
}

/// The last docstring item with this name wins; `*args`/`**kwargs` also match their
/// documented starred spelling.
fn doc_param<'a>(params: &'a [DocParam], name: &str, kind: ArgKind) -> Option<&'a DocParam> {
    let find = |wanted: &str| params.iter().rev().find(|p| p.arg_name == wanted);
    find(name).or_else(|| match kind {
        ArgKind::VarPositional => find(&format!("*{name}")),
        ArgKind::VarKeyword => find(&format!("**{name}")),
        _ => None,
    })
}

#[cfg(test)]
#[path = "finalize_tests.rs"]
mod tests;

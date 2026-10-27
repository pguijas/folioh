//! `syn` items to `ModuleIR`. A signature is the source as written collapsed to
//! one line, a doc comment is the Markdown as written, and anything that is not
//! plain `pub`, is `#[doc(hidden)]` or is macro-generated is not documentation.
//! Rust carries
//! its argument and return prose in the body of the doc comment, so `args`,
//! `returns` and `raises` stay empty and the signature holds the types.

use std::collections::HashSet;
use std::ops::Range;
use std::path::PathBuf;

use folioh_ir::{
    DocstringIR, FunctionIR, FunctionKind, Language, ModuleIR, TypeIR, TypeKind, VarIR, REEXPORT,
};
use proc_macro2::Span;
use syn::spanned::Spanned;
use syn::{
    Attribute, Fields, ImplItem, Item, ItemEnum, ItemImpl, ItemStruct, ItemTrait, ItemType, Meta,
    Signature, TraitItem, Visibility,
};

use crate::doc_attributes::doc_text;
use crate::error::RustSyntaxError;

/// One module's items in source order. `name` is the module path
/// (`demo_crate::models`), `source_file` what the IR reports as its file.
pub fn parse_source(
    source: &str,
    name: &str,
    source_file: &str,
) -> Result<ModuleIR, RustSyntaxError> {
    let file = syn::parse_file(source).map_err(|err| {
        let at = err.span().start();
        RustSyntaxError {
            path: PathBuf::from(source_file),
            message: err.to_string(),
            line: at.line as u32,
            column: at.column as u32 + 1,
        }
    })?;
    let mut module = ModuleIR {
        name: name.to_string(),
        docstring: docstring(&doc_text(&file.attrs, source_file)),
        classes: Vec::new(),
        functions: Vec::new(),
        constants: Vec::new(),
        source_file: source_file.to_string(),
        language: Language::Rust,
        types: Vec::new(),
    };
    let mut private = HashSet::new();
    private_types(&file.items, &mut private);
    let ctx = Ctx {
        src: source,
        file: source_file,
        private: &private,
    };
    for item in &file.items {
        push(&ctx, item, &mut module);
    }
    Ok(module)
}

/// What every item of one file is read against.
struct Ctx<'a> {
    src: &'a str,
    file: &'a str,
    /// The types and traits this file declares without publishing them.
    private: &'a HashSet<String>,
}

/// One item into the module it belongs in; one that is not published goes nowhere.
fn push(ctx: &Ctx, item: &Item, module: &mut ModuleIR) {
    let (src, file) = (ctx.src, ctx.file);
    match item {
        Item::Fn(f) if published(&f.vis, &f.attrs) => {
            let range = start(src, &f.attrs, f.span())..open(f.block.brace_token.span.open());
            module
                .functions
                .push(function(src, &f.sig, &f.vis, &f.attrs, range, file));
        }
        Item::Const(c) if published(&c.vis, &c.attrs) => module.constants.push(VarIR {
            name: c.ident.to_string(),
            ty: text(src, c.ty.span()),
            value: text(src, c.expr.span()),
            description: doc_text(&c.attrs, file),
        }),
        Item::Static(s) if published(&s.vis, &s.attrs) => module.constants.push(VarIR {
            name: s.ident.to_string(),
            ty: text(src, s.ty.span()),
            value: text(src, s.expr.span()),
            description: doc_text(&s.attrs, file),
        }),
        // A `pub use` is the only item whose name is its whole statement: what
        // it re-exports is documented where it is defined.
        Item::Use(u) if published(&u.vis, &u.attrs) => module.constants.push(VarIR {
            name: slice(
                src,
                start(src, &u.attrs, u.span())..u.semi_token.span().byte_range().end,
            ),
            ty: REEXPORT.to_string(),
            value: String::new(),
            description: String::new(),
        }),
        Item::Struct(s) if published(&s.vis, &s.attrs) => {
            module.types.push(struct_type(src, s, file))
        }
        Item::Enum(e) if published(&e.vis, &e.attrs) => module.types.push(enum_type(src, e, file)),
        Item::Trait(t) if published(&t.vis, &t.attrs) => {
            module.types.push(trait_type(src, t, file))
        }
        Item::Type(t) if published(&t.vis, &t.attrs) => module.types.push(alias_type(src, t, file)),
        // An impl of a type or trait this file keeps private is as private.
        Item::Impl(i) if !hidden(&i.attrs) && !implements_private(i, ctx.private) => {
            module.types.push(impl_type(src, i, file))
        }
        // An inline module has no file of its own: its items belong to the
        // page of the file that declares it.
        Item::Mod(m) if published(&m.vis, &m.attrs) => {
            for inner in m.content.iter().flat_map(|(_, items)| items) {
                push(ctx, inner, module);
            }
        }
        _ => {}
    }
}

fn struct_type(src: &str, s: &ItemStruct, file: &str) -> TypeIR {
    let from = start(src, &s.attrs, s.span());
    let to = match &s.fields {
        Fields::Named(f) => open(f.brace_token.span.open()),
        Fields::Unnamed(f) => open(f.paren_token.span.open()),
        Fields::Unit => s
            .semi_token
            .map_or_else(|| s.span().byte_range().end, |t| open(t.span())),
    };
    TypeIR {
        name: s.ident.to_string(),
        kind: TypeKind::Struct,
        docstring: docstring(&doc_text(&s.attrs, file)),
        fields: fields(src, &s.fields, file),
        variants: Vec::new(),
        methods: Vec::new(),
        bases: decorators(src, &s.attrs),
        signature: slice(src, from..to),
        visibility: visibility(src, &s.vis),
        source_file: file.to_string(),
        line_number: line_at(src, from),
    }
}

fn enum_type(src: &str, e: &ItemEnum, file: &str) -> TypeIR {
    let from = start(src, &e.attrs, e.span());
    TypeIR {
        name: e.ident.to_string(),
        kind: TypeKind::Enum,
        docstring: docstring(&doc_text(&e.attrs, file)),
        fields: Vec::new(),
        variants: e
            .variants
            .iter()
            .filter(|v| !hidden(&v.attrs))
            .map(|v| VarIR {
                name: v.ident.to_string(),
                ty: match &v.fields {
                    Fields::Unit => String::new(),
                    fields => text(src, fields.span()),
                },
                value: v
                    .discriminant
                    .as_ref()
                    .map_or_else(String::new, |(_, expr)| text(src, expr.span())),
                description: doc_text(&v.attrs, file),
            })
            .collect(),
        methods: Vec::new(),
        bases: decorators(src, &e.attrs),
        signature: slice(src, from..open(e.brace_token.span.open())),
        visibility: visibility(src, &e.vis),
        source_file: file.to_string(),
        line_number: line_at(src, from),
    }
}

fn trait_type(src: &str, t: &ItemTrait, file: &str) -> TypeIR {
    let from = start(src, &t.attrs, t.span());
    TypeIR {
        name: t.ident.to_string(),
        kind: TypeKind::Trait,
        docstring: docstring(&doc_text(&t.attrs, file)),
        // An associated type reads as a field of type `type`; an associated
        // constant as a field of its own type.
        fields: t
            .items
            .iter()
            .filter_map(|item| match item {
                TraitItem::Type(ty) if !hidden(&ty.attrs) => Some(VarIR {
                    name: ty.ident.to_string(),
                    ty: "type".to_string(),
                    value: String::new(),
                    description: doc_text(&ty.attrs, file),
                }),
                TraitItem::Const(c) if !hidden(&c.attrs) => Some(VarIR {
                    name: c.ident.to_string(),
                    ty: text(src, c.ty.span()),
                    value: c
                        .default
                        .as_ref()
                        .map_or_else(String::new, |(_, expr)| text(src, expr.span())),
                    description: doc_text(&c.attrs, file),
                }),
                _ => None,
            })
            .collect(),
        variants: Vec::new(),
        methods: t
            .items
            .iter()
            .filter_map(|item| match item {
                TraitItem::Fn(f) if !hidden(&f.attrs) => {
                    let to = match &f.default {
                        Some(block) => open(block.brace_token.span.open()),
                        None => f
                            .semi_token
                            .map_or_else(|| f.span().byte_range().end, |t| open(t.span())),
                    };
                    let range = start(src, &f.attrs, f.span())..to;
                    Some(function(
                        src,
                        &f.sig,
                        &Visibility::Inherited,
                        &f.attrs,
                        range,
                        file,
                    ))
                }
                _ => None,
            })
            .collect(),
        bases: decorators(src, &t.attrs),
        signature: slice(src, from..open(t.brace_token.span.open())),
        visibility: visibility(src, &t.vis),
        source_file: file.to_string(),
        line_number: line_at(src, from),
    }
}

fn alias_type(src: &str, t: &ItemType, file: &str) -> TypeIR {
    let from = start(src, &t.attrs, t.span());
    TypeIR {
        name: t.ident.to_string(),
        kind: TypeKind::TypeAlias,
        docstring: docstring(&doc_text(&t.attrs, file)),
        fields: Vec::new(),
        variants: Vec::new(),
        methods: Vec::new(),
        bases: decorators(src, &t.attrs),
        signature: slice(src, from..open(t.semi_token.span())),
        visibility: visibility(src, &t.vis),
        source_file: file.to_string(),
        line_number: line_at(src, from),
    }
}

/// An `impl` block is a type of its own, named after the type it implements
/// for; the trait it implements is its base. An inherent block publishes its
/// public methods, a trait block all of them: the trait already is the contract.
fn impl_type(src: &str, i: &ItemImpl, file: &str) -> TypeIR {
    let from = start(src, &i.attrs, i.span());
    let base = i.trait_.as_ref().map(|(_, path, _)| text(src, path.span()));
    TypeIR {
        name: text(src, i.self_ty.span()),
        kind: TypeKind::Impl,
        docstring: docstring(&doc_text(&i.attrs, file)),
        fields: Vec::new(),
        variants: Vec::new(),
        methods: i
            .items
            .iter()
            .filter_map(|item| match item {
                ImplItem::Fn(f) if (base.is_some() || public(&f.vis)) && !hidden(&f.attrs) => {
                    let range =
                        start(src, &f.attrs, f.span())..open(f.block.brace_token.span.open());
                    Some(function(src, &f.sig, &f.vis, &f.attrs, range, file))
                }
                _ => None,
            })
            .collect(),
        bases: base.into_iter().collect(),
        signature: slice(src, from..open(i.brace_token.span.open())),
        visibility: String::new(),
        source_file: file.to_string(),
        line_number: line_at(src, from),
    }
}

fn function(
    src: &str,
    sig: &Signature,
    vis: &Visibility,
    attrs: &[Attribute],
    range: Range<usize>,
    file: &str,
) -> FunctionIR {
    FunctionIR {
        name: sig.ident.to_string(),
        args: Vec::new(),
        returns: None,
        raises: Vec::new(),
        decorators: decorators(src, attrs),
        docstring: docstring(&doc_text(attrs, file)),
        is_async: sig.asyncness.is_some(),
        source_file: file.to_string(),
        line_number: line_at(src, range.start),
        kind: FunctionKind::Function,
        signature: slice(src, range),
        visibility: visibility(src, vis),
    }
}

/// The public fields of a struct; a tuple field is named by its position.
fn fields(src: &str, fields: &Fields, file: &str) -> Vec<VarIR> {
    fields
        .iter()
        .enumerate()
        .filter(|(_, field)| published(&field.vis, &field.attrs))
        .map(|(index, field)| VarIR {
            name: field
                .ident
                .as_ref()
                .map_or_else(|| index.to_string(), ToString::to_string),
            ty: text(src, field.ty.span()),
            value: String::new(),
            description: doc_text(&field.attrs, file),
        })
        .collect()
}

/// `pub`, `pub(crate)`, `pub(super)` as written; a private item has none.
fn visibility(src: &str, vis: &Visibility) -> String {
    match vis {
        Visibility::Inherited => String::new(),
        vis => text(src, vis.span()),
    }
}

/// Plain `pub`: `pub(crate)`, `pub(super)` and `pub(in ..)` stay inside the crate.
fn public(vis: &Visibility) -> bool {
    matches!(vis, Visibility::Public(_))
}

/// What `rustdoc` publishes: a `pub` item that is not `#[doc(hidden)]`.
pub(crate) fn published(vis: &Visibility, attrs: &[Attribute]) -> bool {
    public(vis) && !hidden(attrs)
}

/// `#[doc(hidden)]`, alone or beside other `doc` arguments.
fn hidden(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| match &attr.meta {
        Meta::List(list) if list.path.is_ident("doc") => list
            .tokens
            .to_string()
            .split(',')
            .any(|arg| arg.trim() == "hidden"),
        _ => false,
    })
}

/// The types and traits some items declare without publishing, inline modules
/// included: an impl of one of them is not public API either.
fn private_types(items: &[Item], names: &mut HashSet<String>) {
    for item in items {
        let (ident, shown) = match item {
            Item::Struct(s) => (&s.ident, published(&s.vis, &s.attrs)),
            Item::Enum(e) => (&e.ident, published(&e.vis, &e.attrs)),
            Item::Union(u) => (&u.ident, published(&u.vis, &u.attrs)),
            Item::Trait(t) => (&t.ident, published(&t.vis, &t.attrs)),
            Item::Type(t) => (&t.ident, published(&t.vis, &t.attrs)),
            Item::Mod(m) => {
                if let Some((_, inner)) = &m.content {
                    private_types(inner, names);
                }
                continue;
            }
            _ => continue,
        };
        if !shown {
            names.insert(ident.to_string());
        }
    }
}

/// Whether an impl is for a private type of this file, or of a private trait.
fn implements_private(i: &ItemImpl, private: &HashSet<String>) -> bool {
    let last = |path: &syn::Path| path.segments.last().map(|s| s.ident.to_string());
    let self_name = match i.self_ty.as_ref() {
        syn::Type::Path(p) if p.qself.is_none() => last(&p.path),
        _ => None,
    };
    let trait_name = i.trait_.as_ref().and_then(|(_, path, _)| last(path));
    [self_name, trait_name]
        .into_iter()
        .flatten()
        .any(|name| private.contains(&name))
}

/// The first line and the rest, as every language's prose splits; the blank
/// lines around the rest are not part of it.
fn docstring(text: &str) -> DocstringIR {
    let (short, long) = text.split_once('\n').unwrap_or((text, ""));
    DocstringIR {
        short_description: short.to_string(),
        long_description: long.trim_matches('\n').to_string(),
        ..DocstringIR::default()
    }
}

/// Every attribute but the doc comments, as written.
fn decorators(src: &str, attrs: &[Attribute]) -> Vec<String> {
    attrs
        .iter()
        .filter(|attr| !attr.path().is_ident("doc"))
        .map(|attr| text(src, attr.span()))
        .collect()
}

/// Where an item's own text starts: after its attributes, doc comments included.
fn start(src: &str, attrs: &[Attribute], span: Span) -> usize {
    match attrs.iter().map(|a| a.span().byte_range().end).max() {
        Some(end) => end + (src[end..].len() - src[end..].trim_start().len()),
        None => span.byte_range().start,
    }
}

/// Where a body begins: the byte the delimiter or the `;` sits on.
fn open(span: Span) -> usize {
    span.byte_range().start
}

/// The source a span covers, collapsed to one line.
fn text(src: &str, span: Span) -> String {
    slice(src, span.byte_range())
}

/// The source between two offsets, collapsed to one line: a signature reads the
/// same whatever the wrapping it was written with.
fn slice(src: &str, range: Range<usize>) -> String {
    src[range].split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The 1-based line a byte offset falls on.
fn line_at(src: &str, offset: usize) -> u32 {
    (src[..offset].bytes().filter(|byte| *byte == b'\n').count() + 1) as u32
}

#[cfg(test)]
#[path = "parse_tests.rs"]
mod tests;

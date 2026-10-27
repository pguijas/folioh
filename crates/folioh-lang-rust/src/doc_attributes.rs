//! Doc attributes are read as source, without evaluating cfg or expanding code.
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use syn::parse::Parser;
use syn::visit::Visit;
use syn::{punctuated::Punctuated, Attribute, Expr, Lit, Meta, Token};

fn values(meta: &Meta, visit: &mut impl FnMut(&Expr)) {
    match meta {
        Meta::NameValue(value) if value.path.is_ident("doc") => visit(&value.value),
        Meta::List(list) if list.path.is_ident("cfg_attr") => {
            let mut tokens = list.tokens.clone().into_iter();
            // Groups keep commas inside the unevaluated predicate together.
            for token in tokens.by_ref() {
                if matches!(token, proc_macro2::TokenTree::Punct(ref punctuation) if punctuation.as_char() == ',')
                {
                    break;
                }
            }
            if let Ok(args) =
                Punctuated::<Meta, Token![,]>::parse_terminated.parse2(tokens.collect())
            {
                for attr in &args {
                    values(attr, visit);
                }
            }
        }
        _ => {}
    }
}

fn include_path(expr: &Expr, source: &Path) -> Option<PathBuf> {
    let Expr::Macro(expr) = expr else { return None };
    if !expr.mac.path.is_ident("include_str") {
        return None;
    }
    let args = expr
        .mac
        .parse_body_with(Punctuated::<syn::LitStr, Token![,]>::parse_terminated)
        .ok()?;
    if args.len() != 1 {
        return None;
    }
    Some(
        source
            .parent()
            .unwrap_or(Path::new("."))
            .join(args.first()?.value()),
    )
}

pub(crate) fn doc_text(attrs: &[Attribute], source_file: &str) -> String {
    let mut text = String::new();
    for attr in attrs {
        values(&attr.meta, &mut |expr| {
            let value = if let Expr::Lit(syn::ExprLit {
                lit: Lit::Str(value),
                ..
            }) = expr
            {
                Some(value.value())
            } else if let Some(path) = include_path(expr, Path::new(source_file)) {
                match std::fs::read_to_string(&path) {
                    Ok(value) => Some(value),
                    Err(err) => {
                        eprintln!("Warning: Rust documentation file {} included from {} could not be read: {}", path.display(), source_file, err);
                        None
                    }
                }
            } else {
                None
            };
            if let Some(value) = value {
                text.push_str(value.strip_prefix(' ').unwrap_or(&value));
                text.push('\n');
            }
        });
    }
    crate::rustdoc::plain_markdown(&text)
}

/// Literal files named by doc attributes, including missing files. This does
/// not execute macros or evaluate conditional compilation predicates.
pub fn doc_dependencies(source_file: &Path) -> Vec<PathBuf> {
    struct Dependencies<'a> {
        source: &'a Path,
        paths: BTreeSet<PathBuf>,
    }
    impl<'ast> Visit<'ast> for Dependencies<'_> {
        fn visit_attribute(&mut self, attr: &'ast Attribute) {
            values(&attr.meta, &mut |expr| {
                if let Some(path) = include_path(expr, self.source) {
                    self.paths.insert(path);
                }
            });
        }
    }
    let Ok(source) = std::fs::read_to_string(source_file) else {
        return Vec::new();
    };
    let Ok(file) = syn::parse_file(&source) else {
        return Vec::new();
    };
    let mut deps = Dependencies {
        source: source_file,
        paths: BTreeSet::new(),
    };
    deps.visit_file(&file);
    deps.paths.into_iter().collect()
}

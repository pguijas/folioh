//! IR to MDX: the module page and the `api-reference/index` overview.
//! Strings only; `folioh-site` writes them.

mod class;
mod function;
pub(crate) mod rest;
mod type_item;

pub(crate) use function::{decorated_signature, display_name};
pub(crate) use type_item::{member_table, type_signature, type_title};

use folioh_config::API_REFERENCE_SLUG;
use folioh_ir::{DocstringIR, FunctionIR, ModuleIR, VarIR, REEXPORT};
use folioh_mdx::{escape_mdx, render_frontmatter};
use indexmap::IndexMap;
use serde_json::{json, Value};

use crate::headings::shift_headings;
use crate::routes::{anchor_slug, module_route, symbol_prefix};
use crate::xref::{resolve_type_link, SymbolIndex};

/// What a module page needs besides the IR: source links and cross-references.
#[derive(Debug, Clone, Copy, Default)]
pub struct RenderOptions<'a> {
    /// `project.repo`; empty disables source links.
    pub repo_url: &'a str,
    /// `<project_dir>/` (trailing slash) stripped off source paths, or empty.
    pub source_root: &'a str,
    /// `project.repo_ref`; `None` or blank means `main`.
    pub source_ref: Option<&'a str>,
    /// The published symbol index; `None` (or empty) renders no links.
    pub symbol_index: Option<&'a SymbolIndex>,
}

/// One page's rendering context, shared by the function, class and type renderers.
pub(crate) struct Page<'a> {
    opts: RenderOptions<'a>,
    current_module: String,
    language: &'static str,
}

impl Page<'_> {
    fn source_link(&self, source_file: &str, line: u32) -> String {
        source_link(
            self.opts.repo_url,
            source_file,
            line,
            self.opts.source_root,
            self.opts.source_ref,
        )
    }

    fn resolve(&self, type_str: &str) -> Option<String> {
        let index = self.opts.symbol_index?;
        resolve_type_link(type_str, index, &self.current_module)
    }

    fn fence(&self, code: &str) -> String {
        format!("```{}\n{code}\n```\n", self.language)
    }
}

/// ` <SourceLink href="<repo>/blob/<ref>/<rel>#L<line>" />`, or `""` without a repo.
pub fn source_link(
    repo_url: &str,
    source_file: &str,
    line: u32,
    source_root: &str,
    source_ref: Option<&str>,
) -> String {
    if repo_url.is_empty() {
        return String::new();
    }
    let rel_path = match source_file.strip_prefix(source_root) {
        Some(rest) if !source_root.is_empty() => rest.trim_start_matches('/'),
        _ => source_file,
    };
    let git_ref = source_ref
        .map(str::trim)
        .filter(|r| !r.is_empty())
        .unwrap_or("main");
    format!(
        " <SourceLink href=\"{}/blob/{git_ref}/{rel_path}#L{line}\" />",
        repo_url.trim_end_matches('/')
    )
}

/// `<ParamTable args={[...]} />` (JSON, two-space indent, UTF-8), `""` without args.
pub fn render_param_table(
    func: &FunctionIR,
    index: Option<&SymbolIndex>,
    current_module: &str,
) -> String {
    if func.args.is_empty() {
        return String::new();
    }
    let index = index.filter(|i| !i.is_empty());
    let entries: Vec<Value> = func
        .args
        .iter()
        .map(|arg| {
            let mut entry = json!({
                "name": function::display_name(arg),
                "type": if arg.ty.is_empty() { "Any" } else { arg.ty.as_str() },
                "default": arg.default.as_deref().unwrap_or(""),
                "description": arg.description,
            });
            if let Some(href) = index
                .filter(|_| !arg.ty.is_empty())
                .and_then(|index| resolve_type_link(&arg.ty, index, current_module))
            {
                entry["href"] = Value::String(href);
            }
            entry
        })
        .collect();
    format!("<ParamTable args={{{}}} />", pretty(&entries))
}

/// The MDX page of one module: frontmatter, heading, prose, examples and
/// notes, constants, re-exports, classes, functions, then types.
pub fn module_to_mdx(module: &ModuleIR, opts: &RenderOptions) -> String {
    let mut fm = IndexMap::new();
    fm.insert("title".to_string(), module.name.clone());
    if !module.docstring.short_description.is_empty() {
        fm.insert(
            "description".to_string(),
            module.docstring.short_description.clone(),
        );
    }
    // An empty index counts as no index: no links anywhere.
    let opts = RenderOptions {
        symbol_index: opts.symbol_index.filter(|i| !i.is_empty()),
        ..*opts
    };
    let page = Page {
        opts,
        current_module: symbol_prefix(module),
        language: module.language.id(),
    };
    let mut parts = vec![
        render_frontmatter(&fm),
        format!(
            "# {}{}\n",
            module.name,
            page.source_link(&module.source_file, 1)
        ),
    ];
    parts.extend(prose(&module.docstring, 2));
    parts.extend(tail(&module.docstring, &page));
    let (reexports, constants): (Vec<VarIR>, Vec<VarIR>) = module
        .constants
        .iter()
        .cloned()
        .partition(|constant| constant.ty == REEXPORT);
    if !constants.is_empty() {
        parts.push("## Constants\n".to_string());
        parts.push(format!(
            "{}\n",
            member_table("Constant", &constants, type_item::cell)
        ));
    }
    if !reexports.is_empty() {
        parts.push("## Re-exports\n".to_string());
        let statements: Vec<&str> = reexports.iter().map(|r| r.name.as_str()).collect();
        parts.push(page.fence(&statements.join("\n")));
    }
    if !module.classes.is_empty() {
        parts.push("## Classes\n".to_string());
        parts.extend(
            module
                .classes
                .iter()
                .map(|c| class::render_class(c, 3, &page, &anchor_slug(&c.name))),
        );
    }
    if !module.functions.is_empty() {
        parts.push("## Functions\n".to_string());
        parts.extend(
            module
                .functions
                .iter()
                .map(|f| function::render_function(f, 3, &page, &anchor_slug(&f.name))),
        );
    }
    if !module.types.is_empty() {
        parts.push("## Types\n".to_string());
        parts.extend(
            module
                .types
                .iter()
                .map(|t| type_item::render_type(t, 3, &page)),
        );
    }
    parts.join("\n")
}

/// The `api-reference/index` overview: one `<ApiReferenceIndex>` over the
/// modules sorted by name, with their class, function and type counts.
/// Nextra treats it as the folder's page so breadcrumbs use the overview
/// route instead of the first language group, which may have no index page.
pub fn api_reference_index_to_mdx(modules: &[ModuleIR]) -> String {
    let mut fm = IndexMap::new();
    fm.insert("asIndexPage".to_string(), json!(true));
    fm.insert("title".to_string(), json!("Source Code"));
    fm.insert(
        "description".to_string(),
        json!("Generated source code documentation for project modules."),
    );
    let mut parts = vec![render_frontmatter(&fm)];
    if modules.is_empty() {
        parts.push("# Source Code\n".to_string());
        parts.push("No source modules were found.\n".to_string());
        return parts.join("\n");
    }
    let mut sorted: Vec<&ModuleIR> = modules.iter().collect();
    sorted.sort_by(|a, b| a.name.cmp(&b.name));
    let entries: Vec<Value> = sorted
        .iter()
        .map(|module| {
            let route = module_route(module);
            let relative = route
                .strip_prefix(&format!("{API_REFERENCE_SLUG}/"))
                .unwrap_or(&route);
            json!({
                "name": module.name,
                "description": module.docstring.short_description,
                "href": format!("./{relative}/"),
                "classCount": module.classes.len(),
                "functionCount": module.functions.len(),
                "typeCount": module.types.len(),
            })
        })
        .collect();
    parts.push(format!(
        "<ApiReferenceIndex modules={{{}}} />\n",
        pretty(&entries)
    ));
    parts.join("\n")
}

/// A docstring's examples as fences, then its notes under one label.
fn tail(docstring: &DocstringIR, page: &Page) -> Vec<String> {
    let mut parts: Vec<String> = docstring.examples.iter().map(|ex| page.fence(ex)).collect();
    if !docstring.notes.is_empty() {
        parts.push("**Notes:**\n".to_string());
        parts.extend(docstring.notes.iter().map(|note| {
            format!(
                "{}\n",
                rest::render(note, 1).unwrap_or_else(|| escape_mdx(note))
            )
        }));
    }
    parts
}

/// Short and long description as escaped prose parts.
/// The summary and body, their headings no shallower than `floor`.
fn prose(docstring: &DocstringIR, floor: usize) -> Vec<String> {
    let combined = [&docstring.short_description, &docstring.long_description]
        .into_iter()
        .filter(|text| !text.is_empty())
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("\n\n");
    if let Some(rendered) = rest::render(&combined, floor) {
        return vec![format!("{rendered}\n")];
    }
    [&docstring.short_description, &docstring.long_description]
        .into_iter()
        .filter(|text| !text.is_empty())
        .map(|text| format!("{}\n", escape_mdx(&shift_headings(text, floor))))
        .collect()
}

/// `json.dumps(indent=2)`: two-space indent, UTF-8 kept.
fn pretty(value: &[Value]) -> String {
    serde_json::to_string_pretty(value).expect("JSON values serialise")
}

/// `json.dumps(list)` with Python's default `", "` separator over pre-rendered items.
fn compact_list<I: IntoIterator<Item = String>>(items: I) -> String {
    format!("[{}]", items.into_iter().collect::<Vec<_>>().join(", "))
}

/// A JSON string literal.
fn json_str(text: &str) -> String {
    serde_json::to_string(text).expect("strings serialise")
}

#[cfg(test)]
mod tests;

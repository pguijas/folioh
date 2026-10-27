//! `llms.txt` and `llms-full.txt` (llmstxt.org) as strings. Gated docs and
//! modules are skipped here.

use folioh_ir::{
    ClassIR, DocstringIR, FunctionIR, FunctionKind, ModuleIR, TypeIR, VarIR, REEXPORT,
};
use folioh_mdx::{mdx_to_markdown, Frontmatter, MarkdownPage};
use serde_yaml_ng::Value;

use crate::features::{
    disabled_api_feature_for_module_in, disabled_doc_feature_for_route, experimental,
    DISABLED_API_MODULES,
};
use crate::headings::shift_headings;
use crate::mdx::{decorated_signature, display_name, member_table, type_signature, type_title};
use crate::routes::{module_route, route_base};

/// The config values the llms writers read.
#[derive(Debug, Clone, Copy)]
pub struct LlmsContext<'a> {
    pub project_name: &'a str,
    /// `landing.hero.description`; empty emits no blockquote.
    pub landing_hero_description: &'a str,
    /// `project.url`; empty keeps links site-relative.
    pub site_url: &'a str,
    /// `template.docs_route_base`; blank means `/docs`.
    pub docs_route_base: &'a str,
    /// Stripped off source citations; empty keeps only file names of absolute paths.
    pub project_dir: &'a str,
    /// Module prefix -> gating feature; `DISABLED_API_MODULES` (empty) in this release.
    pub api_gates: &'a [(&'a str, &'a str)],
}

impl Default for LlmsContext<'_> {
    fn default() -> Self {
        LlmsContext {
            project_name: "",
            landing_hero_description: "",
            site_url: "",
            docs_route_base: "",
            project_dir: "",
            api_gates: &DISABLED_API_MODULES,
        }
    }
}

fn gated(module: &ModuleIR, ctx: Option<&LlmsContext>) -> bool {
    let gates = ctx.map_or(&DISABLED_API_MODULES[..], |c| c.api_gates);
    disabled_api_feature_for_module_in(&module.name, gates, &experimental()).is_some()
}

/// Whitespace collapsed onto one line.
fn inline(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn cell(text: &str) -> String {
    inline(text).replace('|', "\\|")
}

fn absolute(path: String, site_url: &str) -> String {
    if site_url.is_empty() {
        path
    } else {
        format!("{}{path}", site_url.trim_end_matches('/'))
    }
}

/// Public URL of a doc route: `index` pages fold onto their directory.
pub fn doc_link(route: &str, site_url: &str, docs_route_base: &str) -> String {
    let base = route_base(docs_route_base);
    let route = route.trim_matches('/');
    let path = match route {
        "" | "index" => format!("{base}/"),
        _ => format!("{base}/{}/", route.strip_suffix("/index").unwrap_or(route)),
    };
    absolute(path, site_url)
}

/// Public URL of a module page.
pub fn api_link(module: &ModuleIR, site_url: &str, docs_route_base: &str) -> String {
    absolute(
        format!("{}/{}/", route_base(docs_route_base), module_route(module)),
        site_url,
    )
}

/// `path:line` relative to the project; an unrooted absolute path keeps only its file name.
pub fn source_citation(source_file: &str, line: u32, project_dir: &str) -> String {
    if source_file.is_empty() {
        return String::new();
    }
    let root = project_dir.trim_end_matches('/');
    let path = match source_file.strip_prefix(&format!("{root}/")) {
        Some(rest) if !root.is_empty() => rest,
        _ if source_file.starts_with('/') => source_file.rsplit('/').next().unwrap_or(""),
        _ => source_file,
    };
    if line > 0 {
        format!("{path}:{line}")
    } else {
        path.to_string()
    }
}

/// A frontmatter scalar as Python's `str()` would show it.
fn scalar(frontmatter: &Frontmatter, key: &str) -> Option<String> {
    match frontmatter.get(key)? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(true) => Some("True".to_string()),
        Value::Bool(false) => Some("False".to_string()),
        _ => None,
    }
}

fn title_of(doc: &MarkdownPage) -> String {
    scalar(&doc.frontmatter, "title").unwrap_or_else(|| doc.route.clone())
}

/// `llms.txt`: project header, hero blockquote, doc links, module links.
pub fn generate_llms_txt(ctx: &LlmsContext, modules: &[ModuleIR], docs: &[MarkdownPage]) -> String {
    let mut lines = vec![format!("# {}", ctx.project_name), String::new()];
    if !ctx.landing_hero_description.is_empty() {
        lines.push(format!("> {}", inline(ctx.landing_hero_description)));
        lines.push(String::new());
    }
    if !docs.is_empty() {
        lines.push("## Docs".to_string());
        for doc in docs {
            if disabled_doc_feature_for_route(&doc.route).is_some() {
                continue;
            }
            let link = doc_link(&doc.route, ctx.site_url, ctx.docs_route_base);
            let description = scalar(&doc.frontmatter, "description")
                .map(|d| inline(&d))
                .unwrap_or_default();
            let suffix = if description.is_empty() {
                String::new()
            } else {
                format!(": {description}")
            };
            lines.push(format!("- [{}]({link}){suffix}", title_of(doc)));
        }
        lines.push(String::new());
    }
    if !modules.is_empty() {
        lines.push("## API Reference".to_string());
        for module in modules {
            if gated(module, Some(ctx)) {
                continue;
            }
            let link = api_link(module, ctx.site_url, ctx.docs_route_base);
            let description = inline(&module.docstring.short_description);
            let suffix = if description.is_empty() {
                String::new()
            } else {
                format!(": {description}")
            };
            lines.push(format!("- [{}]({link}){suffix}", module.name));
        }
        lines.push(String::new());
    }
    lines.join("\n")
}

fn heading_block(heading: &str, citation: &str, url: &str) -> String {
    let mut lines = vec![heading.to_string()];
    if !url.is_empty() {
        lines.push(format!("URL: {url}"));
    }
    if !citation.is_empty() {
        lines.push(format!("Source: {citation}"));
    }
    lines.join("\n")
}

/// The summary and body, their headings no shallower than `floor`.
fn prose_blocks(docstring: &DocstringIR, floor: usize) -> Vec<String> {
    let combined = [&docstring.short_description, &docstring.long_description]
        .into_iter()
        .filter(|text| !text.is_empty())
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("\n\n");
    if let Some(rendered) = crate::mdx::rest::render(&combined, floor) {
        return vec![mdx_to_markdown(&rendered)];
    }
    [&docstring.short_description, &docstring.long_description]
        .into_iter()
        .filter(|text| !text.is_empty())
        .map(|text| shift_headings(text, floor))
        .collect()
}

fn tail_blocks(docstring: &DocstringIR, language: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    if !docstring.examples.is_empty() {
        blocks.push("**Examples:**".to_string());
        blocks.extend(
            docstring
                .examples
                .iter()
                .map(|ex| format!("```{language}\n{ex}\n```")),
        );
    }
    if !docstring.notes.is_empty() {
        let mut lines = vec!["**Notes:**".to_string()];
        lines.extend(
            docstring
                .notes
                .iter()
                .map(|note| match crate::mdx::rest::render(note, 1) {
                    Some(rendered) => mdx_to_markdown(&rendered),
                    None => format!("- {}", inline(note)),
                }),
        );
        blocks.push(lines.join("\n"));
    }
    blocks
}

fn arg_table(func: &FunctionIR) -> String {
    let mut rows = vec![
        "| Parameter | Type | Default | Description |".to_string(),
        "| --- | --- | --- | --- |".to_string(),
    ];
    rows.extend(func.args.iter().map(|arg| {
        let name = display_name(arg);
        let ty = if arg.ty.is_empty() {
            "Any"
        } else {
            arg.ty.as_str()
        };
        let default = arg
            .default
            .as_deref()
            .map(|d| format!("`{}`", cell(d)))
            .unwrap_or_default();
        format!(
            "| `{}` | `{}` | {default} | {} |",
            cell(&name),
            cell(ty),
            cell(&arg.description)
        )
    }));
    rows.join("\n")
}

/// The signature fence text: the kind decorator the page shows as a badge
/// (`@staticmethod`, `@classmethod`, `@property`), then the signature with
/// its other decorators.
fn function_code(func: &FunctionIR, language: &str) -> String {
    let kind = match func.kind {
        FunctionKind::Staticmethod => "@staticmethod\n",
        FunctionKind::Classmethod => "@classmethod\n",
        FunctionKind::Property => "@property\n",
        FunctionKind::Function | FunctionKind::Method => "",
    };
    format!("{kind}{}", decorated_signature(func, language))
}

/// The class fence text: its decorators as the source writes them, then
/// `class Name(Base, ...)`, or `class Name extends Base` in JavaScript.
fn class_code(cls: &ClassIR, language: &str) -> String {
    let mut lines: Vec<String> = cls
        .decorators
        .iter()
        .map(|decorator| match language {
            "python" => format!("@{decorator}"),
            _ => decorator.clone(),
        })
        .collect();
    let bases = cls.bases.join(", ");
    lines.push(match (language, bases.is_empty()) {
        (_, true) => format!("class {}", cls.name),
        ("javascript", false) => format!("class {} extends {bases}", cls.name),
        (_, false) => format!("class {}({bases})", cls.name),
    });
    lines.join("\n")
}

fn function_blocks(
    func: &FunctionIR,
    heading: &str,
    project_dir: &str,
    language: &str,
) -> Vec<String> {
    let is_property = func.kind == FunctionKind::Property;
    let mut blocks = vec![
        heading_block(
            &format!("{heading} {}", func.name),
            &source_citation(&func.source_file, func.line_number, project_dir),
            "",
        ),
        format!("```{language}\n{}\n```", function_code(func, language)),
    ];
    blocks.extend(prose_blocks(&func.docstring, heading.len() + 1));
    if !is_property && !func.args.is_empty() {
        blocks.push(arg_table(func));
    }
    if let Some(returns) = &func.returns {
        let label = if returns.is_generator {
            "Yields"
        } else if is_property {
            "Type"
        } else {
            "Returns"
        };
        let text = typed(&returns.ty, &inline(&returns.description));
        if !text.is_empty() {
            blocks.push(format!("**{label}:** {text}"));
        }
    }
    if !func.raises.is_empty() {
        let mut lines = vec!["**Raises:**".to_string()];
        lines.extend(
            func.raises
                .iter()
                .map(|r| typed(&r.exception, &inline(&r.description)))
                .filter(|text| !text.is_empty())
                .map(|text| format!("- {text}")),
        );
        if lines.len() > 1 {
            blocks.push(lines.join("\n"));
        }
    }
    blocks.extend(tail_blocks(&func.docstring, language));
    blocks
}

/// `` `type` - description ``, either half alone when the other is missing.
fn typed(ty: &str, description: &str) -> String {
    match (ty.is_empty(), description.is_empty()) {
        (true, _) => description.to_string(),
        (false, true) => format!("`{ty}`"),
        (false, false) => format!("`{ty}` - {description}"),
    }
}

/// A class at `level` (`##` for a module's own), its inner classes one deeper.
fn class_blocks(cls: &ClassIR, level: usize, project_dir: &str, language: &str) -> Vec<String> {
    let heading = "#".repeat(level);
    let mut blocks = vec![
        heading_block(
            &format!("{heading} {}", cls.name),
            &source_citation(&cls.source_file, cls.line_number, project_dir),
            "",
        ),
        format!("```{language}\n{}\n```", class_code(cls, language)),
    ];
    blocks.extend(prose_blocks(&cls.docstring, level + 1));
    blocks.extend(tail_blocks(&cls.docstring, language));
    if !cls.class_vars.is_empty() {
        blocks.push(member_table("Attribute", &cls.class_vars, cell));
    }
    let member = "#".repeat(level + 1);
    for method in &cls.methods {
        blocks.extend(function_blocks(method, &member, project_dir, language));
    }
    for inner in &cls.inner_classes {
        blocks.extend(class_blocks(inner, level + 1, project_dir, language));
    }
    blocks
}

fn type_blocks(item: &TypeIR, project_dir: &str, language: &str) -> Vec<String> {
    let mut blocks = vec![heading_block(
        &format!("## {}", type_title(item, str::to_owned)),
        &source_citation(&item.source_file, item.line_number, project_dir),
        "",
    )];
    if !item.signature.is_empty() {
        blocks.push(format!("```{language}\n{}\n```", type_signature(item)));
    }
    blocks.extend(prose_blocks(&item.docstring, 3));
    if !item.fields.is_empty() {
        blocks.push(member_table("Field", &item.fields, cell));
    }
    if !item.variants.is_empty() {
        blocks.push(member_table("Variant", &item.variants, cell));
    }
    blocks.extend(tail_blocks(&item.docstring, language));
    for method in &item.methods {
        blocks.extend(function_blocks(method, "###", project_dir, language));
    }
    blocks
}

fn module_blocks(module: &ModuleIR, ctx: Option<&LlmsContext>) -> Vec<String> {
    let project_dir = ctx.map_or("", |c| c.project_dir);
    let url = ctx.map_or(String::new(), |c| {
        api_link(module, c.site_url, c.docs_route_base)
    });
    let language = module.language.id();
    let mut blocks = vec![heading_block(
        &format!("# {}", module.name),
        &source_citation(&module.source_file, 0, project_dir),
        &url,
    )];
    blocks.extend(prose_blocks(&module.docstring, 2));
    blocks.extend(tail_blocks(&module.docstring, language));
    let (reexports, constants): (Vec<VarIR>, Vec<VarIR>) = module
        .constants
        .iter()
        .cloned()
        .partition(|constant| constant.ty == REEXPORT);
    if !constants.is_empty() {
        blocks.push(format!(
            "## Constants\n\n{}",
            member_table("Constant", &constants, cell)
        ));
    }
    if !reexports.is_empty() {
        let statements: Vec<&str> = reexports.iter().map(|r| r.name.as_str()).collect();
        blocks.push(format!(
            "## Re-exports\n\n```{language}\n{}\n```",
            statements.join("\n")
        ));
    }
    for cls in &module.classes {
        blocks.extend(class_blocks(cls, 2, project_dir, language));
    }
    for func in &module.functions {
        blocks.extend(function_blocks(func, "##", project_dir, language));
    }
    for item in &module.types {
        blocks.extend(type_blocks(item, project_dir, language));
    }
    blocks
}

/// `^#\s+\S`: the line is the document's own H1.
fn is_h1(line: &str) -> bool {
    line.strip_prefix('#')
        .is_some_and(|rest| rest.starts_with(char::is_whitespace) && !rest.trim_start().is_empty())
}

fn doc_blocks(doc: &MarkdownPage, ctx: Option<&LlmsContext>) -> Vec<String> {
    // Imports, exports and JSX may sit above the H1 in the authored source.
    let content = mdx_to_markdown(&doc.content);
    let content = content.trim();
    let (first_line, rest) = content.split_once('\n').unwrap_or((content, ""));
    let (heading, body) = if is_h1(first_line) {
        (first_line.to_string(), rest.trim_matches('\n'))
    } else {
        (format!("# {}", title_of(doc)), content)
    };
    let url = ctx.map_or(String::new(), |c| {
        doc_link(&doc.route, c.site_url, c.docs_route_base)
    });
    let mut blocks = vec![heading_block(&heading, "", &url)];
    if !body.is_empty() {
        blocks.push(body.to_string());
    }
    blocks
}

/// `llms-full.txt`: every published doc, then every published module, as
/// sections joined by `\n\n---\n\n`. No trailing newline; nothing at all -> `""`.
pub fn generate_llms_full_txt(
    modules: &[ModuleIR],
    docs: &[MarkdownPage],
    ctx: Option<&LlmsContext>,
) -> String {
    let mut sections = Vec::new();
    for doc in docs {
        if disabled_doc_feature_for_route(&doc.route).is_none() {
            sections.push(doc_blocks(doc, ctx).join("\n\n"));
        }
    }
    for module in modules {
        if !gated(module, ctx) {
            sections.push(module_blocks(module, ctx).join("\n\n"));
        }
    }
    sections.join("\n\n---\n\n")
}

#[cfg(test)]
#[path = "llms_tests.rs"]
mod tests;

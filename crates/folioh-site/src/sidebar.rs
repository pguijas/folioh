//! Nextra `_meta.ts` generation: root ordering, per-directory files, the
//! `Source Code` tree grouped by language, hidden and unlisted entries.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use folioh_config::{language_label, slug_label, slugify};
use indexmap::IndexMap;

use crate::{re, Result, SiteError};

/// One published Markdown page as the sidebar sees it.
pub struct SidebarPage<'a> {
    /// Content route without docs base or extension (`components/callout`).
    pub route: &'a str,
    /// Frontmatter title (already "frontmatter, else first H1").
    pub title: Option<&'a str>,
    /// Hidden from the sidebar, published everywhere else.
    pub unlisted: bool,
}

/// One parsed source module as the sidebar sees it.
pub struct SidebarModule<'a> {
    /// Route below `api-reference/`, e.g. `mylib/core` or `rust/geo/shapes`.
    pub route_below_api: &'a str,
    /// The parser's language id (`python`, `rust`, ...).
    pub language: &'a str,
}

/// Everything `generate_meta_files` needs.
pub struct SidebarInput<'a> {
    /// `docs.yaml` `nav`.
    pub nav: &'a [String],
    /// Every published module, Python first.
    pub modules: &'a [SidebarModule<'a>],
    /// Every published Markdown page in discovery order.
    pub pages: &'a [SidebarPage<'a>],
    /// `sidebar.default_collapsed`; folders start collapsed when true.
    pub default_collapsed: bool,
    /// Directory-relative child route names read from `_meta.yaml`.
    pub directory_orders: &'a DirectoryOrders,
}

/// A `_meta.ts` value.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(untagged)]
pub enum MetaValue {
    /// `true`/`false` (the `collapsed` flag); deserialize before YAML string coercion.
    Bool(bool),
    /// A page title or expanded folder title.
    Str(String),
    /// A nested object (hidden entry, collapsed folder, separator).
    Object(Meta),
}

/// An ordered `_meta.ts` object.
pub type Meta = IndexMap<String, MetaValue>;

const SOURCE_CODE_SLUG: &str = "api-reference";
const SOURCE_CODE_TITLE: &str = "Source Code";

/// Directory routes mapped to ordered immediate child routes.
pub type DirectoryOrders = BTreeMap<String, Vec<String>>;

/// Metadata inputs under the same documentation roots used by Markdown discovery.
pub fn directory_order_files(roots: &[String]) -> Result<Vec<PathBuf>> {
    fn walk(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        for entry in std::fs::read_dir(dir).map_err(|e| SiteError::io(dir, e))? {
            let entry = entry.map_err(|e| SiteError::io(dir, e))?;
            let path = entry.path();
            if entry
                .file_type()
                .map_err(|e| SiteError::io(&path, e))?
                .is_dir()
            {
                walk(&path, files)?;
            } else if entry.file_name() == "_meta.yaml" {
                files.push(path);
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    for root in roots.iter().map(Path::new).filter(|p| p.is_dir()) {
        walk(root, &mut files)?;
    }
    files.sort();
    Ok(files)
}

/// Read and validate each directory's order, without requiring an index page.
pub fn read_directory_orders(roots: &[String]) -> Result<DirectoryOrders> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Metadata {
        order: Vec<serde_yaml_ng::Value>,
    }
    let mut result = DirectoryOrders::new();
    let mut owners = BTreeMap::new();
    // Read separately per root: overlapping declarations must not silently win.
    for root in roots {
        for path in directory_order_files(std::slice::from_ref(root))? {
            let fail = |message: String| SiteError::Value(format!("{}: {message}", path.display()));
            let raw = std::fs::read_to_string(&path).map_err(|e| SiteError::io(&path, e))?;
            let metadata: Metadata =
                serde_yaml_ng::from_str(&raw).map_err(|e| fail(e.to_string()))?;
            let relative = path.strip_prefix(root).expect("metadata under its root");
            let index = folioh_mdx::source_route(&relative.with_file_name("index.md"));
            let directory = index.strip_suffix("/index").unwrap_or("").to_string();
            if let Some(first) = owners.insert(directory.clone(), path.clone()) {
                return Err(fail(format!(
                    "duplicate sidebar order for directory '{directory}'; also declared in {}",
                    first.display()
                )));
            }
            let mut order = Vec::new();
            let mut seen = BTreeSet::new();
            for value in metadata.order {
                let name = value
                    .as_str()
                    .ok_or_else(|| fail("order entries must be strings".into()))?;
                if name.chars().any(char::is_control)
                    || name.is_empty()
                    || name.trim() != name
                    || matches!(name, "." | "..")
                    || name.contains(['/', '\\'])
                    || name.ends_with(".md")
                    || name.ends_with(".mdx")
                {
                    return Err(fail(format!(
                        "order entries must be immediate child routes without extensions: {name:?}"
                    )));
                }
                let route = folioh_mdx::source_route(Path::new(&format!("{name}.md")));
                if !seen.insert(route.clone()) {
                    return Err(fail(format!("duplicate order entry: {name:?}")));
                }
                order.push(route);
            }
            result.insert(directory, order);
        }
    }
    Ok(result)
}

fn apply_directory_order(mut meta: Meta, order: Option<&Vec<String>>) -> Meta {
    let mut ordered = Meta::new();
    for slug in order.into_iter().flatten() {
        if let Some(value) = meta.shift_remove(slug) {
            ordered.insert(slug.clone(), value);
        }
    }
    ordered.extend(meta);
    ordered
}

/// Apply declarations again when a built-in adds metadata after page generation.
fn order_meta_ts(
    directory: &str,
    text: &str,
    config: &folioh_config::DocsConfig,
    orders: &DirectoryOrders,
    mut fallback: Meta,
) -> Result<String> {
    let Some(order) = orders.get(directory) else {
        return Ok(text.to_string());
    };
    let body = text
        .trim()
        .strip_prefix("export default")
        .unwrap_or(text)
        .trim()
        .trim_end_matches(';');
    let mut meta: Meta = if body.is_empty() {
        std::mem::take(&mut fallback)
    } else {
        serde_yaml_ng::from_str(body).map_err(|e| {
            SiteError::Value(format!(
                "cannot apply _meta.yaml order to {directory}/_meta.ts: {e}"
            ))
        })?
    };
    for slug in order {
        if !meta.contains_key(slug) {
            if let Some(value) = fallback.shift_remove(slug) {
                meta.insert(slug.clone(), value);
            }
        }
    }
    let hidden_index = if !directory.is_empty()
        && matches!(meta.get("index"), Some(MetaValue::Object(value)) if value.get("display") == Some(&MetaValue::Str("hidden".into())))
    {
        meta.shift_remove("index")
    } else {
        None
    };
    let mut meta = apply_directory_order(meta, Some(order));
    if directory.is_empty() {
        meta = apply_nav_order(meta, &config.nav);
    }
    if let Some(index) = hidden_index {
        meta.shift_insert(0, "index".into(), index);
    }
    Ok(meta_to_ts(&meta))
}

/// Order the completed tree, including pages emitted after the source pass.
pub fn apply_directory_orders(builder: &crate::builder::SiteBuilder<'_>) -> Result<()> {
    let orders = read_directory_orders(&builder.config.source.docs)?;
    if orders.is_empty() {
        return Ok(());
    }
    let mut documents = Vec::new();
    for path in crate::fs::files_under(&builder.content_dir)
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == "mdx"))
    {
        let mut page =
            folioh_mdx::parse_markdown_file(&path).map_err(|e| SiteError::Value(e.to_string()))?;
        page.route = folioh_mdx::source_route(
            &path
                .strip_prefix(&builder.content_dir)
                .expect("content page")
                .with_extension("md"),
        );
        documents.push(page);
    }
    let pages: Vec<_> = documents
        .iter()
        .map(|page| SidebarPage {
            route: &page.route,
            title: page.frontmatter.get("title").and_then(|v| v.as_str()),
            unlisted: page.unlisted,
        })
        .collect();
    let docs = DocMeta::collect(&pages, builder.config.sidebar.default_collapsed, &orders);
    for directory in orders.keys() {
        if !builder.content_dir.join(directory).is_dir() {
            continue;
        }
        let parts: Vec<_> = directory
            .split('/')
            .filter(|p| !p.is_empty())
            .map(str::to_string)
            .collect();
        let fallback = docs.build_meta_for_dir(&parts);
        let text = order_meta_ts(
            directory,
            &builder.read_meta(directory)?,
            builder.config,
            &orders,
            fallback,
        )?;
        builder.write_meta(directory, &text)?;
    }
    Ok(())
}

/// The emoji class stripped from sidebar labels.
pub fn emoji_pattern() -> regex::Regex {
    re("[\u{200d}\u{2300}-\u{23ff}\u{2600}-\u{27bf}\u{fe0e}\u{fe0f}\u{1f000}-\u{1faff}]+")
}

/// Emoji-free, whitespace-collapsed sidebar label with a fallback.
pub fn sidebar_title(title: Option<&str>, fallback: &str) -> String {
    sidebar_title_with(title, fallback, &emoji_pattern())
}

/// `sidebar_title` with the emoji pattern compiled by the caller.
pub fn sidebar_title_with(title: Option<&str>, fallback: &str, emoji: &regex::Regex) -> String {
    let raw = match title {
        Some(t) if !t.trim().is_empty() => t,
        _ => fallback,
    };
    let cleaned = emoji.replace_all(raw, "");
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        cleaned
    }
}

fn escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

fn value_lines(value: &MetaValue, indent: usize) -> Vec<String> {
    match value {
        MetaValue::Object(entries) => {
            let mut lines = vec!["{".to_string()];
            let pad = " ".repeat(indent + 2);
            for (key, child) in entries {
                let rendered = value_lines(child, indent + 2);
                if rendered.len() == 1 {
                    lines.push(format!("{pad}\"{}\": {},", escape(key), rendered[0]));
                } else {
                    lines.push(format!("{pad}\"{}\": {}", escape(key), rendered[0]));
                    lines.extend(rendered[1..rendered.len() - 1].iter().cloned());
                    lines.push(format!("{pad}{},", rendered[rendered.len() - 1]));
                }
            }
            lines.push("}".to_string());
            lines
        }
        MetaValue::Bool(b) => vec![b.to_string()],
        MetaValue::Str(s) => vec![format!("\"{}\"", escape(s))],
    }
}

/// `export default { ... }` serialiser; no trailing newline.
pub fn meta_to_ts(meta: &Meta) -> String {
    let mut lines = value_lines(&MetaValue::Object(meta.clone()), 0);
    lines[0] = "export default {".to_string();
    lines.join("\n")
}

fn hidden() -> MetaValue {
    MetaValue::Object(IndexMap::from([(
        "display".to_string(),
        MetaValue::Str("hidden".to_string()),
    )]))
}

fn folder_meta(title: &str, default_collapsed: bool) -> MetaValue {
    if !default_collapsed {
        return MetaValue::Str(title.to_string());
    }
    MetaValue::Object(IndexMap::from([
        ("title".to_string(), MetaValue::Str(title.to_string())),
        (
            "theme".to_string(),
            MetaValue::Object(IndexMap::from([(
                "collapsed".to_string(),
                MetaValue::Bool(true),
            )])),
        ),
    ]))
}

type DirPath = Vec<String>;

struct DocMeta {
    pages_by_dir: BTreeMap<DirPath, IndexMap<String, String>>,
    dirs_by_dir: BTreeMap<DirPath, IndexMap<String, String>>,
    unlisted: BTreeSet<Vec<String>>,
    default_collapsed: bool,
    directory_orders: DirectoryOrders,
}

impl DocMeta {
    fn collect(pages: &[SidebarPage], default_collapsed: bool, orders: &DirectoryOrders) -> Self {
        let mut this = DocMeta {
            pages_by_dir: BTreeMap::new(),
            dirs_by_dir: BTreeMap::new(),
            unlisted: BTreeSet::new(),
            default_collapsed,
            directory_orders: orders.clone(),
        };
        let emoji = emoji_pattern();
        for page in pages {
            let parts: Vec<String> = page
                .route
                .split('/')
                .filter(|p| !p.is_empty())
                .map(str::to_string)
                .collect();
            let Some((slug, dir_path)) = parts.split_last() else {
                continue;
            };
            let title = sidebar_title_with(page.title, &slug_label(slug), &emoji);
            this.pages_by_dir
                .entry(dir_path.to_vec())
                .or_default()
                .insert(slug.clone(), title);
            if page.unlisted {
                this.unlisted.insert(parts.clone());
            }
            for (index, dir_slug) in dir_path.iter().enumerate() {
                this.dirs_by_dir
                    .entry(dir_path[..index].to_vec())
                    .or_default()
                    .entry(dir_slug.clone())
                    .or_insert_with(|| slug_label(dir_slug));
            }
        }
        let pages_by_dir = &this.pages_by_dir;
        for (parent, children) in this.dirs_by_dir.iter_mut() {
            for (child_slug, title) in children.iter_mut() {
                let mut child_path = parent.clone();
                child_path.push(child_slug.clone());
                if let Some(index_title) =
                    pages_by_dir.get(&child_path).and_then(|p| p.get("index"))
                {
                    *title = index_title.clone();
                }
            }
        }
        this
    }

    fn subtree_unlisted(&self, dir: &[String]) -> bool {
        self.pages_by_dir.iter().all(|(page_dir, pages)| {
            !page_dir.starts_with(dir)
                || pages.keys().all(|slug| {
                    let mut full = page_dir.clone();
                    full.push(slug.clone());
                    self.unlisted.contains(&full)
                })
        })
    }

    fn entry_value(&self, path: &[String], slug: &str, title: &str) -> MetaValue {
        let mut full = path.to_vec();
        full.push(slug.to_string());
        if self
            .dirs_by_dir
            .get(path)
            .map(|d| d.contains_key(slug))
            .unwrap_or(false)
        {
            if self.subtree_unlisted(&full) {
                return hidden();
            }
            return folder_meta(title, self.default_collapsed);
        }
        if self.unlisted.contains(&full) {
            return hidden();
        }
        MetaValue::Str(title.to_string())
    }

    fn build_meta_for_dir(&self, path: &[String]) -> Meta {
        let mut remaining: IndexMap<String, String> = IndexMap::new();
        if let Some(pages) = self.pages_by_dir.get(path) {
            remaining.extend(pages.iter().map(|(k, v)| (k.clone(), v.clone())));
        }
        if let Some(dirs) = self.dirs_by_dir.get(path) {
            for (k, v) in dirs {
                remaining.insert(k.clone(), v.clone());
            }
        }
        let mut ordered = Meta::new();
        if !path.is_empty() && remaining.shift_remove("index").is_some() {
            ordered.insert("index".to_string(), hidden());
        }
        let entries = remaining
            .into_iter()
            .map(|(slug, title)| {
                let value = self.entry_value(path, &slug, &title);
                (slug, value)
            })
            .collect();
        ordered.extend(apply_directory_order(
            entries,
            self.directory_orders.get(&path.join("/")),
        ));
        ordered
    }

    fn all_dirs(&self) -> BTreeSet<DirPath> {
        self.pages_by_dir
            .keys()
            .chain(self.dirs_by_dir.keys())
            .cloned()
            .collect()
    }
}

fn move_to_end(meta: &mut Meta, slug: &str) {
    if let Some(value) = meta.shift_remove(slug) {
        meta.insert(slug.to_string(), value);
    }
}

fn apply_nav_order(meta: Meta, nav: &[String]) -> Meta {
    if nav.is_empty() {
        return meta;
    }
    let mut aliases: BTreeMap<String, String> = BTreeMap::new();
    for (key, value) in &meta {
        aliases.insert(slugify(key), key.clone());
        let title = match value {
            MetaValue::Str(s) => Some(s),
            MetaValue::Object(o) => match o.get("title") {
                Some(MetaValue::Str(s)) => Some(s),
                _ => None,
            },
            MetaValue::Bool(_) => None,
        };
        if let Some(title) = title {
            aliases.insert(slugify(title), key.clone());
        }
    }
    let mut ordered = Meta::new();
    for item in nav {
        let mut slug = slugify(item);
        if slug == "guide" {
            for (key, value) in &meta {
                if key != SOURCE_CODE_SLUG {
                    ordered.entry(key.clone()).or_insert_with(|| value.clone());
                }
            }
            continue;
        }
        if slug == "source-code" {
            slug = SOURCE_CODE_SLUG.to_string();
        }
        if let Some(key) = aliases.get(&slug) {
            ordered
                .entry(key.clone())
                .or_insert_with(|| meta[key].clone());
        }
    }
    for (key, value) in meta {
        ordered.entry(key).or_insert(value);
    }
    ordered
}

enum Tree {
    Leaf,
    Node(IndexMap<String, Tree>),
}

fn build_module_tree(modules: &[SidebarModule]) -> IndexMap<String, Tree> {
    let mut tree = IndexMap::new();
    let mut sorted: Vec<&SidebarModule> = modules.iter().collect();
    sorted.sort_by_key(|m| m.language != "python");
    for module in sorted {
        let parts: Vec<&str> = module.route_below_api.split('/').collect();
        let mut node = &mut tree;
        for part in &parts[..parts.len() - 1] {
            let entry = node
                .entry(part.to_string())
                .or_insert_with(|| Tree::Node(IndexMap::new()));
            if matches!(entry, Tree::Leaf) {
                *entry = Tree::Node(IndexMap::new());
            }
            node = match entry {
                Tree::Node(children) => children,
                Tree::Leaf => unreachable!(),
            };
        }
        node.entry(parts[parts.len() - 1].to_string())
            .or_insert(Tree::Leaf);
    }
    tree
}

fn language_groups(modules: &[SidebarModule]) -> (Meta, BTreeMap<String, String>) {
    let mut languages: Vec<&str> = Vec::new();
    let mut sorted: Vec<&SidebarModule> = modules.iter().collect();
    sorted.sort_by_key(|m| m.language != "python");
    for module in sorted {
        if !languages.contains(&module.language) {
            languages.push(module.language);
        }
    }
    let titles = languages
        .iter()
        .filter(|l| **l != "python")
        .map(|l| (l.to_string(), language_label(l)))
        .collect();
    let mut leading = Meta::new();
    if languages.len() > 1 && languages.contains(&"python") {
        leading.insert(
            "---python".to_string(),
            MetaValue::Object(IndexMap::from([
                ("type".to_string(), MetaValue::Str("separator".to_string())),
                (
                    "title".to_string(),
                    MetaValue::Str(language_label("python")),
                ),
            ])),
        );
    }
    (leading, titles)
}

fn generate_meta_from_tree(
    tree: &IndexMap<String, Tree>,
    prefix: &str,
    result: &mut BTreeMap<String, String>,
    default_collapsed: bool,
    leading: &Meta,
    titles: &BTreeMap<String, String>,
    orders: &DirectoryOrders,
) {
    let mut meta = Meta::new();
    for (key, subtree) in tree {
        match subtree {
            Tree::Node(children) if !children.is_empty() => {
                let title = titles.get(key).cloned().unwrap_or_else(|| slug_label(key));
                meta.insert(key.clone(), folder_meta(&title, default_collapsed));
                generate_meta_from_tree(
                    children,
                    &format!("{prefix}/{key}"),
                    result,
                    default_collapsed,
                    &Meta::new(),
                    titles,
                    orders,
                );
            }
            _ => {
                meta.insert(key.clone(), MetaValue::Str(slug_label(key)));
            }
        }
    }
    let mut ordered = leading.clone();
    ordered.extend(apply_directory_order(meta, orders.get(prefix)));
    result.insert(format!("{prefix}/_meta.ts"), meta_to_ts(&ordered));
}

/// Every `_meta.ts` the content tree needs: content-relative path to file text.
pub fn generate_meta_files(input: &SidebarInput) -> BTreeMap<String, String> {
    let mut result = BTreeMap::new();
    let mut root = Meta::new();
    if !input.pages.is_empty() {
        let docs = DocMeta::collect(input.pages, input.default_collapsed, input.directory_orders);
        root = docs.build_meta_for_dir(&[]);
        for dir in docs.all_dirs().into_iter().filter(|d| !d.is_empty()) {
            result.insert(
                format!("{}/_meta.ts", dir.join("/")),
                meta_to_ts(&docs.build_meta_for_dir(&dir)),
            );
        }
    }
    if !input.modules.is_empty() {
        root.insert(
            SOURCE_CODE_SLUG.to_string(),
            folder_meta(SOURCE_CODE_TITLE, input.default_collapsed),
        );
    }
    move_to_end(&mut root, SOURCE_CODE_SLUG);
    let root = apply_directory_order(root, input.directory_orders.get(""));
    let root = apply_nav_order(root, input.nav);
    result.insert("_meta.ts".to_string(), meta_to_ts(&root));
    if !input.modules.is_empty() {
        let tree = build_module_tree(input.modules);
        let (leading, titles) = language_groups(input.modules);
        generate_meta_from_tree(
            &tree,
            SOURCE_CODE_SLUG,
            &mut result,
            input.default_collapsed,
            &leading,
            &titles,
            input.directory_orders,
        );
    }
    result
}

#[cfg(test)]
#[path = "sidebar_tests.rs"]
mod tests;

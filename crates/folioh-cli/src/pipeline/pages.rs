//! Incremental page generation shared by `folioh build` and a watcher batch:
//! the skip rule over source hash, symbol index and build context, removed
//! routes, page-local image assets with their stale sweep, sidebar metadata
//! and the `api-reference/index` page. Nothing is written before every page
//! rendered and every asset validated.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

use folioh_config::{canonicalize_lenient, DocsConfig};
use folioh_docs::{
    api_reference_index_to_mdx, build_symbol_index, disabled_api_feature_for_module,
    disabled_doc_feature_for_route, module_route, module_to_mdx, RenderOptions,
};
use folioh_ir::ModuleIR;
use folioh_mdx::{markdown_to_mdx, MarkdownPage};
use folioh_site::builder::{Manifest, SiteBuilder};
use folioh_site::fs::sha256_hex;
use folioh_site::sidebar::{
    generate_meta_files, read_directory_orders, SidebarInput, SidebarModule, SidebarPage,
};
use folioh_site::template::BuildContext;
use serde_json::{json, Map, Value};

use super::source_hash::source_hash as hash_file;
use super::{BuildError, Report};
use crate::ui::text::YELLOW;

/// What `generate_content_pages` needs.
pub struct GenerateArgs<'a, 'c> {
    pub builder: &'a mut SiteBuilder<'c>,
    /// The resolved config.
    pub config: &'a DocsConfig,
    pub modules: &'a [ModuleIR],
    pub docs: &'a [MarkdownPage],
    pub project_dir: &'a Path,
    pub build_context: &'a BuildContext,
    pub clean: bool,
    pub verbose: bool,
    /// The manifest loaded once per build; `None` loads it here (or starts empty under `clean`).
    pub prev_manifest: Option<&'a Manifest>,
    /// The watcher's pass-1 hashes keyed by source path; `None` hashes here.
    pub source_hashes: Option<&'a HashMap<String, String>>,
}

/// What the page step produced.
#[derive(Debug, Clone)]
pub struct GenerationResult {
    pub build_context: BuildContext,
    /// Manifest entries keyed by source path: docs in input order, then modules.
    pub sources: Map<String, Value>,
    pub skipped: usize,
    pub total_pages: usize,
}

impl GenerationResult {
    /// Routes owned by the source pipeline, including its API overview.
    pub fn source_routes(&self) -> BTreeSet<String> {
        self.sources
            .values()
            .filter_map(|entry| entry.get("route").and_then(Value::as_str))
            .map(str::to_owned)
            .chain(std::iter::once("api-reference/index".to_owned()))
            .collect()
    }
}

/// Modules whose feature gate is open.
pub fn published_modules(modules: &[ModuleIR]) -> Vec<&ModuleIR> {
    modules
        .iter()
        .filter(|m| disabled_api_feature_for_module(&m.name).is_none())
        .collect()
}

/// Docs whose feature gate is open.
pub fn published_docs(docs: &[MarkdownPage]) -> Vec<&MarkdownPage> {
    docs.iter()
        .filter(|d| disabled_doc_feature_for_route(&d.route).is_none())
        .collect()
}

/// The page-local images a doc references.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct DocAssets {
    /// Targets that cannot be copied, with the reason in parentheses.
    pub missing: Vec<String>,
    /// `(relative target as written, resolved source file)` in discovery order.
    pub assets: Vec<(String, PathBuf)>,
}

/// Whether `target` crosses a symlink at or below the lexical `root`.
pub fn path_traverses_symlink(root: &Path, target: &Path) -> bool {
    let Ok(relative) = target.strip_prefix(root) else {
        return true;
    };
    if root.is_symlink() {
        return true;
    }
    let mut current = root.to_path_buf();
    for part in relative.components() {
        current.push(part);
        if current.is_symlink() {
            return true;
        }
    }
    false
}

/// `![alt](target "title")` images beside a page, with code blanked out first
/// so a page documenting the grammar does not warn.
pub fn doc_asset_sources(doc: &MarkdownPage) -> DocAssets {
    let mut out = DocAssets::default();
    if doc.source_file.is_empty() {
        return out;
    }
    let source_dir = Path::new(&doc.source_file)
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let image =
        regex::Regex::new(r#"!\[[^\]]*\]\(\s*<?([^)\s>]+)>?(?:\s+["'][^)]*)?\s*\)"#).unwrap();
    let prose = blank_code_spans(&blank_fenced_code(&doc.content));
    let mut seen = HashSet::new();
    for capture in image.captures_iter(&prose) {
        let raw = &capture[1];
        let target = raw
            .split('#')
            .next()
            .unwrap_or("")
            .split('?')
            .next()
            .unwrap_or("")
            .trim();
        if target.is_empty() || !seen.insert(target.to_string()) {
            continue;
        }
        if target.contains("://")
            || target.starts_with('/')
            || target.starts_with("data:")
            || target.starts_with('#')
        {
            continue;
        }
        if Path::new(target)
            .components()
            .any(|c| c == Component::ParentDir)
        {
            out.missing
                .push(format!("{target} (escapes the docs directory)"));
            continue;
        }
        let lexical = source_dir.join(target);
        if path_traverses_symlink(&source_dir, &lexical) {
            out.missing
                .push(format!("{target} (symlinks are not published)"));
            continue;
        }
        let source = canonicalize_lenient(&lexical);
        if !source.starts_with(canonicalize_lenient(&source_dir)) {
            out.missing
                .push(format!("{target} (escapes the docs directory)"));
            continue;
        }
        if !source.is_file() {
            out.missing.push(target.to_string());
            continue;
        }
        out.assets.push((target.to_string(), source));
    }
    out
}

/// Replace every fenced block (``` or ~~~ runs at line start, closed by a
/// line starting with the same run) with a space; an open fence stays.
fn blank_fenced_code(text: &str) -> String {
    fn fence_run(line: &str) -> Option<&str> {
        let first = line.chars().next()?;
        if first != '`' && first != '~' {
            return None;
        }
        let len = line.chars().take_while(|c| *c == first).count();
        (len >= 3).then(|| &line[..len])
    }
    let mut out = String::with_capacity(text.len());
    let mut lines = text.split_inclusive('\n').peekable();
    while let Some(line) = lines.next() {
        let Some(fence) = fence_run(line) else {
            out.push_str(line);
            continue;
        };
        let mut block = vec![line];
        let mut closed = false;
        for inner in lines.by_ref() {
            block.push(inner);
            if inner.starts_with(fence) {
                closed = true;
                break;
            }
        }
        if closed {
            out.push(' ');
        } else {
            out.extend(block);
        }
    }
    out
}

/// Replace every backtick code span (runs of any length) with a space.
fn blank_code_spans(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            let start = i;
            while i < bytes.len() && bytes[i] == b'`' {
                i += 1;
            }
            let ticks = &text[start..i];
            // The span closes at the next run of exactly the same length.
            let mut search = i;
            let mut closed = None;
            while let Some(found) = text[search..].find(ticks) {
                let at = search + found;
                let end = at + ticks.len();
                if end < bytes.len() && bytes[end] == b'`' {
                    let mut run_end = end;
                    while run_end < bytes.len() && bytes[run_end] == b'`' {
                        run_end += 1;
                    }
                    search = run_end;
                    continue;
                }
                closed = Some(end);
                break;
            }
            match closed {
                Some(end) => {
                    out.push(' ');
                    i = end;
                }
                None => out.push_str(ticks),
            }
        } else {
            let ch = text[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

fn field<'v>(entry: &'v Value, key: &str) -> Option<&'v str> {
    entry.get(key).and_then(Value::as_str)
}

/// Sidebar `_meta.ts` files and the `api-reference/index` page.
pub fn write_meta_pages(
    builder: &mut SiteBuilder<'_>,
    config: &DocsConfig,
    published_modules: &[&ModuleIR],
    published_docs: &[&MarkdownPage],
) -> Result<(), BuildError> {
    let routes: Vec<String> = published_modules.iter().map(|m| module_route(m)).collect();
    let modules: Vec<SidebarModule> = published_modules
        .iter()
        .zip(&routes)
        .map(|(module, route)| SidebarModule {
            route_below_api: route.strip_prefix("api-reference/").unwrap_or(route),
            language: module.language.id(),
        })
        .collect();
    let pages: Vec<SidebarPage> = published_docs
        .iter()
        .map(|doc| SidebarPage {
            route: &doc.route,
            title: doc.frontmatter.get("title").and_then(|t| t.as_str()),
            unlisted: doc.unlisted,
        })
        .collect();
    let directory_orders = read_directory_orders(&config.source.docs)?;
    let meta_files = generate_meta_files(&SidebarInput {
        nav: &config.nav,
        modules: &modules,
        pages: &pages,
        default_collapsed: config.sidebar.default_collapsed,
        directory_orders: &directory_orders,
    });
    let keep: BTreeSet<String> = meta_files.keys().cloned().collect();
    builder.prune_meta_files(&keep)?;
    for (path, text) in &meta_files {
        let directory = path.strip_suffix("/_meta.ts").unwrap_or("");
        builder.write_meta(directory, text)?;
    }
    if published_modules.is_empty() {
        builder.remove_page("api-reference/index")?;
    } else {
        let owned: Vec<ModuleIR> = published_modules.iter().map(|m| (*m).clone()).collect();
        builder.write_page("api-reference/index", &api_reference_index_to_mdx(&owned))?;
    }
    Ok(())
}

/// Render every page that changed, then apply: metadata, removals, assets,
/// the stale asset sweep, page writes and the route registration of skipped
/// pages.
pub fn generate_content_pages(
    args: GenerateArgs<'_, '_>,
    report: &Report,
) -> Result<GenerationResult, BuildError> {
    let GenerateArgs {
        builder,
        config,
        modules,
        docs,
        project_dir,
        build_context,
        clean,
        verbose,
        prev_manifest,
        source_hashes,
    } = args;
    let total_candidates = docs.len() + modules.len();
    let spinner = report.spinner("Pages", "generating content", Some(total_candidates));
    let advance = || {
        if let Some(spinner) = &spinner {
            spinner.advance(1);
        }
    };
    let published_modules = published_modules(modules);
    let published_docs = published_docs(docs);
    let total_pages = published_docs.len() + published_modules.len();
    let owned_modules: Vec<ModuleIR> = published_modules.iter().map(|m| (*m).clone()).collect();
    let symbol_index = build_symbol_index(&owned_modules, &config.template.docs_route_base);
    let sorted_index: std::collections::BTreeMap<&String, &String> = symbol_index.iter().collect();
    let symbol_hash = sha256_hex(serde_json::to_string(&sorted_index).unwrap().as_bytes());

    let loaded;
    let prev = match prev_manifest {
        Some(prev) => prev,
        None if clean => {
            loaded = Manifest::default();
            &loaded
        }
        None => {
            loaded = builder.load_manifest()?;
            &loaded
        }
    };
    let prev_sources = &prev.sources;
    let context_changed = prev.build.as_ref() != Some(build_context);
    let hash_of = |key: &str| -> String {
        match source_hashes.and_then(|h| h.get(key)) {
            Some(hash) => hash.clone(),
            None => hash_file(Path::new(key)),
        }
    };
    let prev_field = |key: &str, name: &str| -> Option<String> {
        prev_sources
            .get(key)
            .and_then(|entry| field(entry, name))
            .map(str::to_string)
    };

    let mut new_sources = Map::new();
    let mut pages: Vec<(String, String)> = Vec::new();
    let mut unchanged: Vec<String> = Vec::new();
    let mut removed: BTreeSet<String> = BTreeSet::new();
    let mut assets: Vec<(&MarkdownPage, DocAssets)> = Vec::new();
    let mut verbose_lines = Vec::new();
    let mut skipped = 0;

    for doc in docs {
        let key = doc.source_file.as_str();
        let hash = hash_of(key);
        new_sources.insert(
            key.to_string(),
            json!({"hash": hash, "route": doc.route, "assets": []}),
        );
        if disabled_doc_feature_for_route(&doc.route).is_some() {
            removed.insert(doc.route.clone());
            advance();
            continue;
        }
        assets.push((doc, doc_asset_sources(doc)));
        if !hash.is_empty()
            && prev_field(key, "hash").as_deref() == Some(hash.as_str())
            && !context_changed
            && builder.page_exists(&doc.route)?
            && builder.page_markdown_exists(&doc.route)?
        {
            unchanged.push(doc.route.clone());
            skipped += 1;
            advance();
            continue;
        }
        if verbose {
            verbose_lines.push(format!("  Writing page: {}", doc.route));
        }
        pages.push((doc.route.clone(), markdown_to_mdx(doc)));
        advance();
    }

    let source_root = format!("{}/", project_dir.display());
    let render = RenderOptions {
        repo_url: &config.project.repo,
        source_root: &source_root,
        source_ref: Some(&config.project.repo_ref),
        symbol_index: Some(&symbol_index),
    };
    for module in modules {
        let key = module.source_file.as_str();
        let hash = hash_of(key);
        let route = module_route(module);
        new_sources.insert(
            key.to_string(),
            json!({
                "hash": hash,
                "route": route,
                "symbols": symbol_hash,
                "language": module.language.id(),
            }),
        );
        if disabled_api_feature_for_module(&module.name).is_some() {
            if verbose {
                verbose_lines.push(format!("  Skipping disabled API module: {}", module.name));
            }
            removed.insert(route);
            advance();
            continue;
        }
        if !hash.is_empty()
            && prev_field(key, "hash").as_deref() == Some(hash.as_str())
            && !context_changed
            && prev_field(key, "symbols").as_deref() == Some(symbol_hash.as_str())
            && builder.page_exists(&route)?
            && builder.page_markdown_exists(&route)?
        {
            unchanged.push(route);
            skipped += 1;
            advance();
            continue;
        }
        if verbose {
            verbose_lines.push(format!("  Writing page: {route}"));
        }
        pages.push((route, module_to_mdx(module, &render)));
        advance();
    }

    let current_routes: HashSet<&str> = new_sources
        .values()
        .filter_map(|entry| field(entry, "route"))
        .collect();
    for old in prev_sources.values() {
        if let Some(route) = field(old, "route") {
            if !current_routes.contains(route) {
                removed.insert(route.to_string());
            }
        }
    }

    // Validate every asset destination before the first write.
    let mut destinations: HashMap<PathBuf, (String, &str)> = HashMap::new();
    for (doc, doc_assets) in &assets {
        for (relative, source) in &doc_assets.assets {
            let destination = builder.page_asset_path(&doc.route, relative)?;
            let digest = sha256_hex(&std::fs::read(source)?);
            if let Some((existing, owner)) = destinations.get(&destination) {
                if *existing != digest {
                    return Err(BuildError::Failed(format!(
                        "Asset destination collision at {}: {owner} and {} contain different bytes",
                        destination.display(),
                        doc.source_file
                    )));
                }
            }
            destinations.insert(destination, (digest, &doc.source_file));
        }
    }

    write_meta_pages(builder, config, &published_modules, &published_docs)?;
    for route in &removed {
        builder.remove_page(route)?;
    }
    for (doc, doc_assets) in &assets {
        for missing in &doc_assets.missing {
            report.styled(
                YELLOW,
                &format!("warning: {}: image not found: {missing}", doc.route),
            );
        }
        for (relative, source) in &doc_assets.assets {
            builder.copy_page_asset(&doc.route, relative, source)?;
        }
        let relatives: Vec<&str> = doc_assets.assets.iter().map(|(r, _)| r.as_str()).collect();
        new_sources[&doc.source_file]["assets"] = json!(relatives);
    }
    let mut current_assets: HashSet<PathBuf> = HashSet::new();
    for entry in new_sources.values() {
        let route = field(entry, "route").unwrap_or_default();
        for relative in entry
            .get("assets")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(relative) = relative.as_str() {
                current_assets.insert(builder.page_asset_path(route, relative)?);
            }
        }
    }
    for old in prev_sources.values() {
        let old_assets: Vec<&str> = match old.get("assets") {
            None => Vec::new(),
            Some(Value::Array(items)) if items.iter().all(Value::is_string) => {
                items.iter().filter_map(Value::as_str).collect()
            }
            Some(_) => {
                return Err(BuildError::Failed(
                    "Invalid generated asset ownership in build manifest".to_string(),
                ))
            }
        };
        let route = field(old, "route").unwrap_or_default();
        for relative in old_assets {
            let path = builder.page_asset_path(route, relative)?;
            if !current_assets.contains(&path) {
                builder.remove_page_asset(route, relative)?;
            }
        }
    }
    for (route, mdx) in &pages {
        builder.write_page(route, mdx)?;
    }
    for route in &unchanged {
        builder.register_route(route);
    }
    drop(spinner);
    for line in verbose_lines {
        report.line(&line);
    }
    Ok(GenerationResult {
        build_context: build_context.clone(),
        sources: new_sources,
        skipped,
        total_pages,
    })
}

#[cfg(test)]
#[path = "pages_tests.rs"]
mod tests;

//! `SiteBuilder`: the one write surface into the workspace. Pages and their
//! Markdown mirrors, page and static assets, `_meta.ts` files, the manifest,
//! the authoring contract, LLM files and robots pointers, the search index,
//! preview examples and public files.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use folioh_config::{canonicalize_lenient, join_lexical, DocsConfig, Tracer};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::extensions::ExtensionEmitter;
use crate::fs::{
    files_under, posix, remove_dir_all_if_exists, remove_file_if_exists, sha256_hex,
    write_text_if_changed,
};
use crate::inject::{resolve_base_path_from_env, Injection, TemplateConfigInjector};
use crate::json;
use crate::re;
use crate::runtime::{FrontendRuntime, InstallResult};
use crate::template::{docs_route_base, BuildContext, TemplateWorkspace};
use crate::{Result, SiteError};
use folioh_plugins::{
    render_authoring_contract, render_mdx_contract_module, AssetBuilder, ComponentDefinition,
    ExtensionRegistry, PluginError, BUILTIN_COMPONENTS, FOLIOH_AUTHORING_CONTRACT_PATH,
};

/// `.build/.folioh-manifest.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(
        default,
        deserialize_with = "lenient_context",
        skip_serializing_if = "Option::is_none"
    )]
    /// The build context of the previous build; absent (a changed context)
    /// for a fresh manifest or one whose `build` object does not parse.
    pub build: Option<BuildContext>,
    #[serde(default)]
    /// Source entries of the incremental build (`hash`, `route`, `symbols`, `assets`, `language`).
    pub sources: Map<String, Value>,
}

/// An incomplete `build` object (`{}`, a Python-era shape) reads as `None`:
/// any previous context that is not this binary's counts as changed.
fn lenient_context<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<BuildContext>, D::Error> {
    let value = Option::<Value>::deserialize(deserializer)?;
    Ok(value.and_then(|v| serde_json::from_value(v).ok()))
}

/// One search document for `lib/search-index.ts`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SearchDocument {
    /// The docs URL of the page.
    pub url: String,
    /// Frontmatter title, first heading or last route segment.
    pub title: String,
    /// The page prose with code, tags and expressions removed.
    pub content: String,
}

type SearchSignature = (i128, i128, u64, u64, String);

/// What one pass over `docs/examples/` did: an example is rebuilt only when
/// its digest moved, so a warm build reports mostly reuse.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PreviewExamples {
    pub built: usize,
    pub reused: usize,
    /// Published outputs whose example is gone.
    pub swept: usize,
}

/// A nested example build the binary runs for `write_preview_examples`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewBuildRequest {
    /// The example project (a Folioh project with its own `docs.yaml`).
    pub project_dir: PathBuf,
    /// `public/_folioh/examples/<name>` in the parent workspace.
    pub output_dir: PathBuf,
    /// `.build/.preview-examples/<name>`; `node_modules` survives between builds.
    pub build_dir: PathBuf,
    /// The `FOLIOH_BASE_PATH` the nested build runs under (also set in the environment).
    pub base_path: String,
}

/// A repository file served verbatim from the site root (`public/`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicFile {
    /// The repository file, relative to the project dir.
    pub source: PathBuf,
    /// Path below `public/`.
    pub dest: String,
}

fn io(path: &Path) -> impl Fn(std::io::Error) -> SiteError + '_ {
    move |e| SiteError::io(path, e)
}

/// The search-text patterns, compiled once per `write_search_index`.
struct SearchPatterns {
    clean: regex::Regex,
    frontmatter: regex::Regex,
    title: regex::Regex,
    heading: regex::Regex,
    fence: regex::Regex,
    tag: regex::Regex,
    expression: regex::Regex,
}

impl SearchPatterns {
    fn new() -> Self {
        SearchPatterns {
            clean: re(r"[\[\]()`*_#>|]"),
            frontmatter: re(r"(?s)\A---\n(.*?)\n---"),
            title: re(r"(?m)^title:\s*(.+?)\s*$"),
            heading: re(r"(?m)^#\s+(.+?)\s*$"),
            fence: re(r"(?s)```.*?```"),
            tag: re(r"<[^>]+>"),
            expression: re(r"\{[^{}]*\}"),
        }
    }

    /// Drop Markdown punctuation and collapse whitespace.
    fn clean(&self, value: &str) -> String {
        let text = self.clean.replace_all(value, " ");
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// Frontmatter `title:`, else the first `# ` heading, else the last route segment.
    fn title(&self, raw: &str, route: &str) -> String {
        if let Some(frontmatter) = self.frontmatter.captures(raw) {
            if let Some(title) = self.title.captures(&frontmatter[1]) {
                let cleaned = self.clean(&title[1]);
                let cleaned = cleaned.trim_matches(|c| c == '"' || c == '\'');
                return if cleaned.is_empty() {
                    route.to_string()
                } else {
                    cleaned.to_string()
                };
            }
        }
        if let Some(heading) = self.heading.captures(raw) {
            let cleaned = self.clean(&heading[1]);
            return if cleaned.is_empty() {
                route.to_string()
            } else {
                cleaned
            };
        }
        let last = route
            .strip_suffix("/index")
            .unwrap_or(route)
            .rsplit('/')
            .next()
            .unwrap_or("");
        if last.is_empty() {
            "Docs".to_string()
        } else {
            last.to_string()
        }
    }

    /// Prose without frontmatter, fenced code, tags or `{}` expressions.
    fn content(&self, raw: &str) -> String {
        let text = self.frontmatter.replace(raw, " ");
        let text = self.fence.replace_all(&text, " ");
        let text = self.tag.replace_all(&text, " ");
        let text = self.expression.replace_all(&text, " ");
        self.clean(&text)
    }
}

/// The site workspace writer.
pub struct SiteBuilder<'a> {
    /// The resolved `docs.yaml`.
    pub config: &'a DocsConfig,
    /// The template the workspace copies from.
    pub template_dir: PathBuf,
    /// The workspace (`.build/`).
    pub build_dir: PathBuf,
    /// `build_dir/content`.
    pub content_dir: PathBuf,
    /// The static export destination (`config.output_dir`).
    pub output_dir: PathBuf,
    /// `folioh serve`: LLM files plugins write land in `public/`, not the output dir.
    pub serve: bool,
    /// `build-versions` sets the version being built.
    pub current_version_path: String,
    /// `FOLIOH_TRACE`: every text artefact this builder writes emits `file_write`.
    pub tracer: Option<Arc<Tracer>>,
    runtime: Box<dyn FrontendRuntime>,
    emitted_routes: BTreeSet<String>,
    view_routes: BTreeSet<String>,
    search_documents: HashMap<PathBuf, (SearchSignature, SearchDocument)>,
    contract_components: Option<Vec<ComponentDefinition>>,
    base_path: String,
}

impl<'a> SiteBuilder<'a> {
    /// A builder over `template_dir` and `build_dir`; `runtime` runs pnpm/next, or nothing in tests.
    pub fn new(
        config: &'a DocsConfig,
        template_dir: &Path,
        build_dir: &Path,
        runtime: Box<dyn FrontendRuntime>,
    ) -> Self {
        SiteBuilder {
            config,
            template_dir: template_dir.to_path_buf(),
            build_dir: build_dir.to_path_buf(),
            content_dir: build_dir.join("content"),
            output_dir: PathBuf::from(&config.output_dir),
            serve: false,
            current_version_path: String::new(),
            tracer: None,
            runtime,
            emitted_routes: BTreeSet::new(),
            view_routes: BTreeSet::new(),
            search_documents: HashMap::new(),
            contract_components: None,
            base_path: resolve_base_path_from_env(config),
        }
    }

    /// `write_text_if_changed` plus the `file_write` trace event when it wrote.
    fn write_text(&self, path: &Path, content: &str) -> Result<()> {
        if write_text_if_changed(path, content).map_err(io(path))? {
            if let Some(tracer) = &self.tracer {
                tracer.trace(
                    "file_write",
                    &[
                        ("path", Value::from(path.to_string_lossy().into_owned())),
                        ("bytes", Value::from(content.len())),
                    ],
                );
            }
        }
        Ok(())
    }

    /// The deploy base path resolved at construction.
    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    /// `docs.yaml` route base without a trailing `/`.
    pub fn docs_route_base(&self) -> String {
        docs_route_base(self.config)
    }

    /// `build_dir/.folioh-manifest.json`.
    pub fn manifest_path(&self) -> PathBuf {
        self.build_dir.join(".folioh-manifest.json")
    }

    /// The previous manifest; `{"sources": {}}` when absent.
    pub fn load_manifest(&self) -> Result<Manifest> {
        let path = self.manifest_path();
        match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text)
                .map_err(|e| SiteError::Value(format!("{}: {e}", path.display()))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Manifest::default()),
            Err(e) => Err(SiteError::io(&path, e)),
        }
    }

    /// `json.dumps(manifest, indent=2)` without a trailing newline, write-if-changed.
    pub fn save_manifest(&self, manifest: &Manifest) -> Result<()> {
        let path = self.manifest_path();
        self.write_text(&path, &json::pretty(manifest))?;
        Ok(())
    }

    /// Reset the emitted routes, copy the template and inject `docs.yaml`.
    pub fn prepare(&mut self, clean: bool) -> Result<Injection> {
        self.emitted_routes.clear();
        TemplateWorkspace::new(&self.template_dir, &self.build_dir, Some(&self.content_dir))
            .prepare(clean)?;
        let mut injector =
            TemplateConfigInjector::new(self.config, &self.build_dir, &self.template_dir);
        injector.current_version_path = self.current_version_path.clone();
        injector.inject()
    }

    /// Emit the registry into the workspace and rewrite the contract module.
    pub fn apply_extensions(&mut self, registry: &ExtensionRegistry) -> Result<()> {
        let uses_custom_template =
            !self.config.template.path.is_empty() && self.config.template.overlay_path.is_empty();
        ExtensionEmitter {
            build_dir: self.build_dir.clone(),
            inject_builtins: !uses_custom_template,
            project_dir: self.config.project_dir.clone(),
        }
        .apply(registry)?;
        self.view_routes = registry.views.keys().cloned().collect();
        let components: Vec<ComponentDefinition> = registry.components.values().cloned().collect();
        let lib_dir = self.build_dir.join("lib");
        std::fs::create_dir_all(&lib_dir).map_err(io(&lib_dir))?;
        let path = lib_dir.join("folioh-mdx-contract.ts");
        self.write_text(&path, &render_mdx_contract_module(&components))?;
        self.contract_components = Some(components);
        Ok(())
    }

    /// Site-absolute routes of registry views (`/roadmap`).
    pub fn view_routes(&self) -> BTreeSet<String> {
        self.view_routes.clone()
    }

    fn content_root(&self) -> PathBuf {
        canonicalize_lenient(&self.content_dir)
    }

    fn page_path(&self, route: &str) -> PathBuf {
        if route.is_empty() || route == "index" {
            self.content_dir.join("index.mdx")
        } else {
            join_lexical(&self.content_dir, &format!("{route}.mdx"))
        }
    }

    fn contained_page(&self, route: &str, verb: &str) -> Result<PathBuf> {
        let target = canonicalize_lenient(&self.page_path(route));
        if !target.starts_with(self.content_root()) {
            return Err(SiteError::Value(format!(
                "Route would {verb} outside content directory: {route}"
            )));
        }
        Ok(target)
    }

    /// Write `content/<route>.mdx`, its Markdown mirror, and register the route.
    /// Relative links are resolved to site-absolute docs routes first.
    pub fn write_page(&mut self, route: &str, content: &str) -> Result<()> {
        let target = self.contained_page(route, "write")?;
        std::fs::create_dir_all(target.parent().unwrap()).map_err(io(&target))?;
        let content = crate::links::resolve_relative_links(content, route, &self.docs_route_base());
        let content = content.as_str();
        self.write_text(&target, content)?;
        self.write_markdown_mirror(route, content)?;
        self.register_route(route);
        Ok(())
    }

    /// The on-disk text of a page (plugins compare it before a warm refresh).
    pub fn read_page(&self, route: &str) -> Result<String> {
        let target = self.contained_page(route, "access")?;
        std::fs::read_to_string(&target).map_err(io(&target))
    }

    /// Whether `content/<route>.mdx` exists.
    pub fn page_exists(&self, route: &str) -> Result<bool> {
        Ok(self.contained_page(route, "access")?.exists())
    }

    /// Routes of the pages on disk under `prefix` (`""` = root), sorted by path.
    pub fn list_pages(&self, prefix: &str) -> Result<Vec<String>> {
        let root = self.content_root();
        let base = if prefix.is_empty() {
            root.clone()
        } else {
            canonicalize_lenient(&root.join(prefix))
        };
        if !base.starts_with(&root) {
            return Err(SiteError::Value(format!(
                "Prefix would list outside content directory: {prefix}"
            )));
        }
        if !base.is_dir() {
            return Ok(Vec::new());
        }
        Ok(files_under(&base)
            .into_iter()
            .filter(|p| p.extension().map(|e| e == "mdx").unwrap_or(false))
            .map(|p| posix(&p.strip_prefix(&root).unwrap_or(&p).with_extension("")))
            .collect())
    }

    /// Remove the page, its mirror and its route; a missing page is fine.
    pub fn remove_page(&mut self, route: &str) -> Result<()> {
        let target = self.contained_page(route, "access")?;
        remove_file_if_exists(&target).map_err(io(&target))?;
        self.remove_markdown_mirror(route)?;
        self.emitted_routes.remove(route);
        Ok(())
    }

    /// Where a page asset lands; rejects escapes and generated-file names.
    pub fn page_asset_path(&self, route: &str, relative: &str) -> Result<PathBuf> {
        let content_root = self.content_root();
        let page_dir = canonicalize_lenient(self.page_path(route).parent().unwrap());
        let target = canonicalize_lenient(&page_dir.join(relative));
        if !page_dir.starts_with(&content_root)
            || target == page_dir
            || !target.starts_with(&page_dir)
        {
            return Err(SiteError::Value(format!("Asset would be written outside the content directory: {relative} (from route {route})")));
        }
        let suffix = target
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let name = target
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if suffix == "mdx" || name == "_meta.ts" {
            return Err(SiteError::Value(format!(
                "Asset path is reserved for generated content: {relative}"
            )));
        }
        Ok(target)
    }

    /// Copy a file beside its page; identical bytes are left untouched.
    pub fn copy_page_asset(&self, route: &str, relative: &str, source: &Path) -> Result<()> {
        let target = self.page_asset_path(route, relative)?;
        std::fs::create_dir_all(target.parent().unwrap()).map_err(io(&target))?;
        let source_bytes = std::fs::read(source).map_err(io(source))?;
        if std::fs::read(&target)
            .map(|existing| existing == source_bytes)
            .unwrap_or(false)
        {
            return Ok(());
        }
        std::fs::copy(source, &target).map_err(io(&target))?;
        Ok(())
    }

    /// Delete one page asset if present; same guards as `page_asset_path`.
    pub fn remove_page_asset(&self, route: &str, relative: &str) -> Result<()> {
        let target = self.page_asset_path(route, relative)?;
        if target.is_file() {
            std::fs::remove_file(&target).map_err(io(&target))?;
        }
        Ok(())
    }

    fn public_target(&self, relative: &str, verb: &str) -> Result<PathBuf> {
        let public_root = canonicalize_lenient(&self.build_dir.join("public"));
        let target = canonicalize_lenient(&public_root.join(relative));
        if target == public_root || !target.starts_with(&public_root) {
            return Err(SiteError::Value(format!(
                "Static {verb} outside the public directory: {relative}"
            )));
        }
        Ok(target)
    }

    /// Copy a file into `public/`, served verbatim.
    pub fn copy_static_asset(&self, relative: &str, source: &Path) -> Result<()> {
        let target = self.public_target(relative, "asset would be written")?;
        std::fs::create_dir_all(target.parent().unwrap()).map_err(io(&target))?;
        std::fs::copy(source, &target).map_err(io(&target))?;
        Ok(())
    }

    /// Drop a subtree of `public/` before republishing it.
    pub fn remove_static_tree(&self, relative: &str) -> Result<()> {
        let target = self.public_target(relative, "tree would be removed")?;
        if target.is_dir() {
            remove_dir_all_if_exists(&target).map_err(io(&target))?;
        }
        Ok(())
    }

    /// Copy repository files into `public/` (the agent guide, `install.sh`);
    /// a source outside the project, in `.build/` or in the output dir is refused.
    pub fn copy_public_files(&self, files: &[PublicFile]) -> Result<()> {
        for file in files {
            let source = folioh_config::resolve_contained_dir(
                &file.source,
                &self.config.project_dir,
                Path::new(&self.config.output_dir),
                "public",
                false,
            )
            .map_err(|e| SiteError::Value(e.to_string()))?;
            if !source.is_file() {
                return Err(SiteError::NotFound(format!(
                    "public file not found: {}",
                    source.display()
                )));
            }
            self.copy_static_asset(&file.dest, &source)?;
        }
        Ok(())
    }

    /// Record a route as live so link checking accepts it, without writing it.
    pub fn register_route(&mut self, route: &str) {
        self.emitted_routes.insert(route.to_string());
    }

    /// A copy of the routes written or registered since the last `prepare`.
    pub fn emitted_routes(&self) -> BTreeSet<String> {
        self.emitted_routes.clone()
    }

    /// Record plugin pages and remove unchanged orphans after plugin dispatch.
    /// The source pipeline's routes are protected, including authored pages
    /// replacing a previously generated route on this build.
    pub fn sweep_plugin_pages(&mut self, source_routes: &BTreeSet<String>) -> Result<()> {
        let ledger = self.build_dir.join(".folioh-plugin-pages.json");
        let previous = crate::generated::load(&ledger)?;
        let mut current = BTreeMap::new();
        for route in self.emitted_routes.difference(source_routes) {
            let path = self.contained_page(route, "access")?;
            if let Some(digest) = crate::generated::digest(&path)? {
                current.insert(route.clone(), digest);
            }
        }
        for (route, digest) in previous {
            if self.emitted_routes.contains(&route) {
                continue;
            }
            let path = self.contained_page(&route, "remove")?;
            if crate::generated::digest(&path)?.as_ref() == Some(&digest) {
                self.remove_page(&route)?;
            }
        }
        self.write_text(&ledger, &json::pretty(&current))
    }

    /// Roll back to a snapshot taken with `emitted_routes`.
    pub fn restore_emitted_routes(&mut self, routes: BTreeSet<String>) {
        self.emitted_routes = routes;
    }

    fn meta_dir(&self, directory: &str, verb: &str) -> Result<PathBuf> {
        let dir = if directory.is_empty() {
            self.content_dir.clone()
        } else {
            join_lexical(&self.content_dir, directory)
        };
        let resolved = canonicalize_lenient(&dir);
        if !resolved.starts_with(self.content_root()) {
            return Err(SiteError::Value(format!(
                "Directory would {verb} outside content directory: {directory}"
            )));
        }
        Ok(resolved)
    }

    /// Write `content/<directory>/_meta.ts` verbatim (`""` = root).
    pub fn write_meta(&self, directory: &str, meta_ts: &str) -> Result<()> {
        let dir = self.meta_dir(directory, "write")?;
        std::fs::create_dir_all(&dir).map_err(io(&dir))?;
        let path = dir.join("_meta.ts");
        self.write_text(&path, meta_ts)?;
        Ok(())
    }

    /// The `_meta.ts` text of a directory; `""` when absent.
    pub fn read_meta(&self, directory: &str) -> Result<String> {
        let path = self.meta_dir(directory, "access")?.join("_meta.ts");
        match std::fs::read_to_string(&path) {
            Ok(text) => Ok(text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
            Err(e) => Err(SiteError::io(&path, e)),
        }
    }

    /// Remove every `_meta.ts` under a directory.
    pub fn remove_meta_tree(&self, directory: &str) -> Result<()> {
        let dir = self.meta_dir(directory, "access")?;
        if !dir.exists() {
            return Ok(());
        }
        for path in files_under(&dir)
            .into_iter()
            .filter(|p| p.file_name().map(|n| n == "_meta.ts").unwrap_or(false))
        {
            std::fs::remove_file(&path).map_err(io(&path))?;
        }
        Ok(())
    }

    /// Delete every `content/**/_meta.ts` not in `keep` (content-relative posix paths).
    pub fn prune_meta_files(&self, keep: &BTreeSet<String>) -> Result<()> {
        let root = self.content_root();
        for path in files_under(&self.content_dir)
            .into_iter()
            .filter(|p| p.file_name().map(|n| n == "_meta.ts").unwrap_or(false))
        {
            let rel = posix(path.strip_prefix(&self.content_dir).unwrap_or(&path));
            if keep.contains(&rel) {
                continue;
            }
            if !canonicalize_lenient(&path).starts_with(&root) {
                return Err(SiteError::Value(format!(
                    "Metadata outside content directory: {}",
                    path.display()
                )));
            }
            std::fs::remove_file(&path).map_err(io(&path))?;
        }
        Ok(())
    }

    /// `public/_folioh/markdown`.
    pub fn markdown_root(&self) -> PathBuf {
        self.build_dir
            .join("public")
            .join("_folioh")
            .join("markdown")
    }

    /// `index.md` for `""`/`index`, else `<route>.md`.
    pub fn markdown_path(&self, route: &str) -> PathBuf {
        if route.is_empty() || route == "index" {
            self.markdown_root().join("index.md")
        } else {
            join_lexical(&self.markdown_root(), &format!("{route}.md"))
        }
    }

    fn contained_mirror(&self, route: &str, verb: &str) -> Result<PathBuf> {
        let target = canonicalize_lenient(&self.markdown_path(route));
        if !target.starts_with(canonicalize_lenient(&self.markdown_root())) {
            return Err(SiteError::Value(format!(
                "Route would {verb} outside page markdown directory: {route}"
            )));
        }
        Ok(target)
    }

    /// Write the lossy Markdown mirror of a page.
    pub fn write_markdown_mirror(&self, route: &str, mdx: &str) -> Result<PathBuf> {
        let target = self.contained_mirror(route, "write")?;
        std::fs::create_dir_all(target.parent().unwrap()).map_err(io(&target))?;
        self.write_text(&target, &folioh_mdx::mdx_to_markdown(mdx))?;
        Ok(target)
    }

    /// Whether the page's Markdown mirror exists (the incremental skip needs both files).
    pub fn page_markdown_exists(&self, route: &str) -> Result<bool> {
        Ok(self.contained_mirror(route, "access")?.exists())
    }

    /// Delete the mirror if present.
    pub fn remove_markdown_mirror(&self, route: &str) -> Result<()> {
        let target = self.contained_mirror(route, "access")?;
        remove_file_if_exists(&target).map_err(io(&target))
    }

    /// `content/<route>` as the docs URL the site serves it at.
    pub fn content_route_to_docs_url(&self, route: &str) -> String {
        let base = self.docs_route_base();
        if route.is_empty() || route == "index" {
            return format!("{base}/");
        }
        format!("{base}/{}/", route.strip_suffix("/index").unwrap_or(route))
    }

    /// Write `public/_folioh/contract.json`; `generatedAt` advances only when
    /// the rest of the payload changed.
    pub fn write_authoring_contract(
        &self,
        config_keys: &BTreeSet<String>,
        generated_at: &str,
    ) -> Result<PathBuf> {
        let target = self
            .build_dir
            .join("public")
            .join(FOLIOH_AUTHORING_CONTRACT_PATH);
        std::fs::create_dir_all(target.parent().unwrap()).map_err(io(&target))?;
        let routes: BTreeSet<String> = self
            .emitted_routes
            .iter()
            .map(|r| self.content_route_to_docs_url(r))
            .collect();
        let components = self
            .contract_components
            .as_deref()
            .unwrap_or(&BUILTIN_COMPONENTS);
        // A copied template opts in with this marker. Imported JSON also lets
        // the dev server update links when a source route appears or disappears.
        let frontend_routes = self.build_dir.join("lib/folioh-routes.json");
        let owns_routes = std::fs::read_to_string(&frontend_routes)
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .is_some_and(|data| data.get("foliohRoutes").and_then(Value::as_bool) == Some(true));
        if owns_routes {
            self.write_text(
                &frontend_routes,
                &format!(
                    "{}\n",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "foliohRoutes": true,
                        "docsRouteBase": self.docs_route_base(),
                        "apiReference": self.emitted_routes.contains("api-reference/index"),
                        "routes": &routes,
                    }))
                    .expect("route data serialises")
                ),
            )?;
        }
        let candidate = render_authoring_contract(
            env!("CARGO_PKG_VERSION"),
            generated_at,
            components,
            config_keys.iter().cloned(),
            routes,
        );
        if let Ok(existing) = std::fs::read_to_string(&target) {
            let strip = |text: &str| -> Option<Map<String, Value>> {
                let mut object = serde_json::from_str::<Value>(text)
                    .ok()?
                    .as_object()?
                    .clone();
                object.remove("generatedAt");
                Some(object)
            };
            if let (Some(existing), Some(proposed)) = (strip(&existing), strip(&candidate)) {
                if existing == proposed {
                    return Ok(target);
                }
            }
        }
        self.write_text(&target, &candidate)?;
        Ok(target)
    }

    /// Write or delete `llms.txt`/`llms-full.txt` (into `public/` when
    /// serving, else the output dir) and point `robots.txt` at the ones written.
    pub fn write_llm_files(
        &self,
        llms_txt: Option<&str>,
        llms_full_txt: Option<&str>,
        serve: bool,
    ) -> Result<()> {
        let destination = if serve {
            self.build_dir.join("public")
        } else {
            self.output_dir.clone()
        };
        std::fs::create_dir_all(&destination).map_err(io(&destination))?;
        let mut written = Vec::new();
        for (name, content) in [("llms.txt", llms_txt), ("llms-full.txt", llms_full_txt)] {
            let target = destination.join(name);
            match content {
                None => remove_file_if_exists(&target).map_err(io(&target))?,
                Some(text) => {
                    self.write_text(&target, text)?;
                    written.push(name);
                }
            }
        }
        let robots = destination.join("robots.txt");
        if written.is_empty() || !robots.exists() {
            return Ok(());
        }
        let mut content = std::fs::read_to_string(&robots).map_err(io(&robots))?;
        let lines: Vec<String> = written
            .iter()
            .filter(|name| !content.contains(&format!("# {name}:")))
            .map(|name| format!("# {name}: {}", self.artifact_url(name)))
            .collect();
        if lines.is_empty() {
            return Ok(());
        }
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        self.write_text(&robots, &format!("{content}{}\n", lines.join("\n")))?;
        Ok(())
    }

    fn artifact_url(&self, name: &str) -> String {
        let site_url = self.config.project.url.trim_end_matches('/');
        let prefix = if site_url.starts_with("http") {
            site_url
        } else {
            self.base_path.trim_end_matches('/')
        };
        format!("{prefix}/{name}")
    }

    fn search_signature(path: &Path) -> Option<SearchSignature> {
        let meta = std::fs::metadata(path).ok()?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let mtime = meta.mtime() as i128 * 1_000_000_000 + meta.mtime_nsec() as i128;
            let ctime = meta.ctime() as i128 * 1_000_000_000 + meta.ctime_nsec() as i128;
            Some((mtime, ctime, meta.len(), meta.ino(), String::new()))
        }
        #[cfg(not(unix))]
        {
            // Without inode and ctime a replaced file with equal size and mtime
            // is re-read only if its mtime differs; never stale by design here
            // because the Windows signature omits the fields and re-reads.
            let _ = meta;
            None
        }
    }

    /// Write `lib/search-index.ts` from `content/**/*.mdx`, reusing unchanged pages.
    pub fn write_search_index(&mut self) -> Result<()> {
        let index_path = self.build_dir.join("lib").join("search-index.ts");
        std::fs::create_dir_all(index_path.parent().unwrap()).map_err(io(&index_path))?;
        let mut documents = Vec::new();
        let mut current = HashMap::new();
        let patterns = SearchPatterns::new();
        if self.config.search.enabled && self.content_dir.exists() {
            let base = self.docs_route_base();
            for page in files_under(&self.content_dir) {
                let name = page
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if !name.ends_with(".mdx") || name.starts_with('_') {
                    continue;
                }
                let signature = Self::search_signature(&page).map(|mut s| {
                    s.4 = base.clone();
                    s
                });
                if let (Some(signature), Some((cached_signature, cached))) =
                    (&signature, self.search_documents.get(&page))
                {
                    if cached_signature == signature {
                        documents.push(cached.clone());
                        current.insert(page.clone(), (signature.clone(), cached.clone()));
                        continue;
                    }
                }
                let raw = std::fs::read_to_string(&page).map_err(io(&page))?;
                let route = posix(
                    &page
                        .strip_prefix(&self.content_dir)
                        .unwrap_or(&page)
                        .with_extension(""),
                );
                let document = SearchDocument {
                    url: self.content_route_to_docs_url(&route),
                    title: patterns.title(&raw, &route),
                    content: patterns.content(&raw),
                };
                if let Some(signature) = signature {
                    current.insert(page.clone(), (signature, document.clone()));
                }
                documents.push(document);
            }
        }
        let content = format!(
            "export interface FoliohSearchDocument {{\n  url: string\n  title: string\n  content: string\n}}\n\nexport const foliohSearchPlaceholder = {}\n\nexport const foliohSearchDocuments: FoliohSearchDocument[] = {}\n",
            json::string(&self.config.search.placeholder),
            json::pretty(&documents)
        );
        self.write_text(&index_path, &content)?;
        self.search_documents = current;
        Ok(())
    }

    /// The example projects under `examples_dir`, sorted: a directory with a
    /// plain name and its own `docs.yaml`. Empty when the directory is missing.
    pub fn preview_example_dirs(examples_dir: &Path) -> Vec<PathBuf> {
        let valid_name = re(r"^[A-Za-z0-9][A-Za-z0-9_-]*$");
        let mut dirs: Vec<PathBuf> = std::fs::read_dir(examples_dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.is_dir()
                    && p.join("docs.yaml").is_file()
                    && p.file_name()
                        .is_some_and(|n| valid_name.is_match(&n.to_string_lossy()))
            })
            .collect();
        dirs.sort();
        dirs
    }

    /// Build every Folioh project under `examples_dir` into
    /// `public/_folioh/examples/<name>/` through `run_build`, plus its manifest.
    pub fn write_preview_examples(
        &self,
        examples_dir: &Path,
        run_build: &mut dyn FnMut(PreviewBuildRequest) -> Result<()>,
    ) -> Result<PreviewExamples> {
        let target_root = self
            .build_dir
            .join("public")
            .join("_folioh")
            .join("examples");
        if !examples_dir.is_dir() {
            remove_dir_all_if_exists(&target_root).map_err(io(&target_root))?;
            return Ok(PreviewExamples::default());
        }
        let mut summary = PreviewExamples::default();
        let mut live: BTreeSet<String> = BTreeSet::new();
        for example_dir in Self::preview_example_dirs(examples_dir) {
            let name = example_dir
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            live.insert(name.clone());
            let target_dir = target_root.join(&name);
            let digest = Self::preview_example_digest(
                &example_dir,
                &self.preview_example_base_path(&name),
                &self.preview_example_site_url(),
            );
            // An example is a whole nested build: its own install and its own
            // static export. Rebuilding one nobody touched is the most
            // expensive thing a warm build can do, so it is not done.
            if Self::preview_example_is_current(&target_dir, &digest) {
                summary.reused += 1;
                continue;
            }
            std::fs::create_dir_all(&target_root).map_err(io(&target_root))?;
            self.build_preview_example_project(&example_dir, &target_dir, run_build)?;
            self.write_preview_example_manifest(&example_dir, &target_dir, &digest)?;
            summary.built += 1;
        }
        // An example that was deleted leaves its published output behind.
        if let Ok(published) = std::fs::read_dir(&target_root) {
            for entry in published.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if entry.path().is_dir() && !live.contains(&name) {
                    remove_dir_all_if_exists(&entry.path()).map_err(io(&entry.path()))?;
                    summary.swept += 1;
                }
            }
        }
        Ok(summary)
    }

    /// The SHA-256 of an example's sources, the Folioh version that builds them,
    /// the template they build against, the base path and the public origin the nested build
    /// bakes in: the five things that decide whether its published output is
    /// still the output of this repository.
    ///
    /// The version, not `generator_fingerprint`, which is the hash of the
    /// running executable. Hashing the executable would invalidate every
    /// example on every `cargo build`, which is exactly the person this cache
    /// exists for.
    fn preview_example_digest(example_dir: &Path, base_path: &str, site_url: &str) -> String {
        let mut parts: Vec<u8> = Vec::new();
        parts.extend_from_slice(b"folioh-preview-example-v4\0");
        parts.extend_from_slice(env!("CARGO_PKG_VERSION").as_bytes());
        parts.push(0);
        parts.extend_from_slice(crate::template::TEMPLATE_HASH.as_bytes());
        parts.push(0);
        parts.extend_from_slice(base_path.as_bytes());
        parts.push(0);
        parts.extend_from_slice(site_url.as_bytes());
        parts.push(0);
        for source in Self::preview_example_source_paths(example_dir) {
            let rel = source.strip_prefix(example_dir).unwrap_or(&source);
            let name = posix(rel);
            parts.extend_from_slice(&(name.len() as u64).to_be_bytes());
            parts.extend_from_slice(name.as_bytes());
            let bytes = std::fs::read(&source).unwrap_or_default();
            parts.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
            parts.extend_from_slice(&bytes);
        }
        sha256_hex(&parts)
    }

    /// The base path an example's nested build is served under.
    fn preview_example_base_path(&self, name: &str) -> String {
        format!(
            "{}/_folioh/examples/{name}",
            self.base_path.trim_end_matches('/')
        )
    }

    fn preview_example_site_url(&self) -> String {
        let url = std::env::var("FOLIOH_PREVIEW_URL")
            .ok()
            .filter(|url| !url.is_empty())
            .or_else(|| std::env::var("FOLIOH_EXAMPLE_SITE_URL").ok())
            .unwrap_or_else(|| self.config.project.url.clone());
        if !url.contains('\0') && (url.starts_with("https://") || url.starts_with("http://")) {
            url
        } else {
            String::new()
        }
    }

    /// Whether the published example already carries this digest.
    fn preview_example_is_current(target_dir: &Path, digest: &str) -> bool {
        if !target_dir.join("index.html").is_file() {
            return false;
        }
        let Ok(text) = std::fs::read_to_string(target_dir.join("manifest.json")) else {
            return false;
        };
        serde_json::from_str::<Value>(&text)
            .ok()
            .and_then(|m| m.get("digest").and_then(Value::as_str).map(str::to_string))
            .is_some_and(|found| found == digest)
    }

    /// The nested build for one example: reset its workspace, set
    /// `FOLIOH_BASE_PATH` for the duration, run the build entry.
    pub fn build_preview_example_project(
        &self,
        example_dir: &Path,
        target_dir: &Path,
        run_build: &mut dyn FnMut(PreviewBuildRequest) -> Result<()>,
    ) -> Result<()> {
        let name = example_dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let example_build_dir = self.build_dir.join(".preview-examples").join(&name);
        Self::reset_preview_example_workspace(&example_build_dir)
            .map_err(io(&example_build_dir))?;
        self.share_node_modules(&example_build_dir)?;
        let base_path = self.preview_example_base_path(&name);
        // Turbopack refuses a `node_modules` link that leaves its root, so the
        // nested build's root is this workspace, which holds both the shared
        // `node_modules` and the example's own directory.
        let previous = [
            (
                "FOLIOH_EXAMPLE_SITE_URL",
                std::env::var_os("FOLIOH_EXAMPLE_SITE_URL"),
            ),
            ("FOLIOH_BASE_PATH", std::env::var_os("FOLIOH_BASE_PATH")),
            (
                "FOLIOH_TURBOPACK_ROOT",
                std::env::var_os("FOLIOH_TURBOPACK_ROOT"),
            ),
        ];
        let site_url = self.preview_example_site_url();
        std::env::set_var("FOLIOH_EXAMPLE_SITE_URL", site_url);
        std::env::set_var("FOLIOH_BASE_PATH", &base_path);
        std::env::set_var("FOLIOH_TURBOPACK_ROOT", &self.build_dir);
        let result = run_build(PreviewBuildRequest {
            project_dir: example_dir.to_path_buf(),
            output_dir: target_dir.to_path_buf(),
            build_dir: example_build_dir,
            base_path,
        });
        for (name, value) in previous {
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
        result
    }

    /// One `node_modules` for the site and its examples: the workspace's own,
    /// installed now if nothing has needed it yet, linked into the example's
    /// workspace so the nested build finds its lockfile satisfied and skips
    /// `pnpm install`. A workspace that already holds a `node_modules` keeps
    /// it; a platform that refuses the link installs its own, as before.
    fn share_node_modules(&self, example_build_dir: &Path) -> Result<()> {
        let link = example_build_dir.join("node_modules");
        if link.exists() || link.is_symlink() {
            return Ok(());
        }
        let shared = self.build_dir.join("node_modules");
        if !shared.exists() {
            self.install_deps(&mut |_| {})?;
        }
        if !shared.is_dir() {
            return Ok(());
        }
        std::fs::create_dir_all(example_build_dir).map_err(io(example_build_dir))?;
        #[cfg(unix)]
        let _ = std::os::unix::fs::symlink(&shared, &link);
        #[cfg(windows)]
        let _ = std::os::windows::fs::symlink_dir(&shared, &link);
        Ok(())
    }

    /// Delete the generated parts of an example workspace, keeping `node_modules`.
    pub fn reset_preview_example_workspace(example_build_dir: &Path) -> std::io::Result<()> {
        for name in [
            ".folioh-build.log",
            ".folioh-manifest.json",
            ".next",
            "content",
            "out",
            "public",
        ] {
            let path = example_build_dir.join(name);
            if path.is_dir() {
                std::fs::remove_dir_all(&path)?;
            } else {
                remove_file_if_exists(&path)?;
            }
        }
        Ok(())
    }

    /// The example's source files (sorted), excluding build residue, dotfiles
    /// and the hand-written previews.
    pub fn preview_example_source_paths(example_dir: &Path) -> Vec<PathBuf> {
        const IGNORED_DIRS: [&str; 7] = [
            ".build",
            ".git",
            ".next",
            "__pycache__",
            "_site",
            "node_modules",
            "out",
        ];
        const IGNORED_FILES: [&str; 2] = ["design-reference.html", "preview.html"];
        files_under(example_dir)
            .into_iter()
            .filter(|path| {
                let rel = path.strip_prefix(example_dir).unwrap_or(path);
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                !IGNORED_FILES.contains(&name.as_str())
                    && !rel.components().any(|c| {
                        let part = c.as_os_str().to_string_lossy();
                        IGNORED_DIRS.contains(&part.as_ref()) || part.starts_with('.')
                    })
            })
            .collect()
    }

    fn preview_example_language(path: &Path) -> String {
        let suffix = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        match suffix.as_str() {
            "md" | "mdx" => "markdown".to_string(),
            "yml" | "yaml" => "yaml".to_string(),
            "py" => "python".to_string(),
            "ts" | "tsx" => "tsx".to_string(),
            "json" => "json".to_string(),
            "" => "text".to_string(),
            other => other.to_string(),
        }
    }

    fn write_preview_example_manifest(
        &self,
        example_dir: &Path,
        target_dir: &Path,
        digest: &str,
    ) -> Result<()> {
        let name = example_dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let files_dir = target_dir.join("files");
        let mut files = Vec::new();
        for source in Self::preview_example_source_paths(example_dir) {
            let rel = source.strip_prefix(example_dir).unwrap_or(&source);
            let target = files_dir.join(rel);
            std::fs::create_dir_all(target.parent().unwrap()).map_err(io(&target))?;
            std::fs::copy(&source, &target).map_err(io(&target))?;
            let rel_url = posix(rel);
            files.push(serde_json::json!({
                "path": rel_url,
                "url": format!("/_folioh/examples/{name}/files/{rel_url}"),
                "language": Self::preview_example_language(&source),
            }));
        }
        let manifest = target_dir.join("manifest.json");
        std::fs::create_dir_all(target_dir).map_err(io(target_dir))?;
        self.write_text(
            &manifest,
            &format!(
                "{}\n",
                json::pretty(&serde_json::json!({"digest": digest, "files": files}))
            ),
        )?;
        Ok(())
    }

    /// `FrontendRuntime::install_deps` for this workspace.
    pub fn install_deps(&self, on_phase: &mut dyn FnMut(&str)) -> Result<InstallResult> {
        self.runtime
            .install_deps(&self.template_dir, &self.build_dir, on_phase)
    }

    /// Run the frontend build (log at `log_path` or `.build/.folioh-build.log`),
    /// then copy `out/` to the output dir and rewrite it for `file://`.
    /// Returns the rewriter's warnings.
    pub fn build(
        &self,
        log_path: Option<&Path>,
        on_line: &mut dyn FnMut(&str),
    ) -> Result<Vec<String>> {
        let default_log = self.build_dir.join(".folioh-build.log");
        self.runtime
            .build(&self.build_dir, log_path.unwrap_or(&default_log), on_line)?;
        self.copy_static_output()
    }

    /// Replace the output dir with `out/` and run the static rewriter.
    pub fn copy_static_output(&self) -> Result<Vec<String>> {
        let out = self.build_dir.join("out");
        if !out.exists() {
            return Ok(Vec::new());
        }
        remove_dir_all_if_exists(&self.output_dir).map_err(io(&self.output_dir))?;
        crate::fs::copy_tree_all(&out, &self.output_dir).map_err(io(&self.output_dir))?;
        crate::rewriter::StaticAssetRewriter::new(&self.output_dir)
            .fix_asset_paths_with_base_path(&self.base_path)
            .map_err(io(&self.output_dir))
    }

    /// Start `next dev` on `port`; its output lines stream to `on_line`.
    pub fn serve(
        &self,
        port: u16,
        kill_existing: bool,
        on_line: Box<dyn FnMut(&str) + Send>,
    ) -> Result<Option<std::process::Child>> {
        self.runtime
            .serve(&self.build_dir, port, kill_existing, on_line)
    }
}

fn plugin_error(error: SiteError) -> PluginError {
    Box::new(error)
}

/// The write surface the built-ins' `emit_assets` use.
impl AssetBuilder for SiteBuilder<'_> {
    fn build_dir(&self) -> &Path {
        &self.build_dir
    }
    fn output_dir(&self) -> &Path {
        &self.output_dir
    }
    fn page_exists(&self, route: &str) -> std::result::Result<bool, PluginError> {
        SiteBuilder::page_exists(self, route).map_err(plugin_error)
    }
    fn read_page(&self, route: &str) -> std::result::Result<String, PluginError> {
        SiteBuilder::read_page(self, route).map_err(plugin_error)
    }
    fn write_page(&mut self, route: &str, mdx: &str) -> std::result::Result<(), PluginError> {
        SiteBuilder::write_page(self, route, mdx).map_err(plugin_error)
    }
    fn remove_page(&mut self, route: &str) -> std::result::Result<(), PluginError> {
        SiteBuilder::remove_page(self, route).map_err(plugin_error)
    }
    fn list_pages(&self, prefix: &str) -> std::result::Result<Vec<String>, PluginError> {
        SiteBuilder::list_pages(self, prefix).map_err(plugin_error)
    }
    fn register_route(&mut self, route: &str) {
        SiteBuilder::register_route(self, route)
    }
    fn emitted_routes(&self) -> BTreeSet<String> {
        SiteBuilder::emitted_routes(self)
    }
    fn restore_emitted_routes(&mut self, snapshot: BTreeSet<String>) {
        SiteBuilder::restore_emitted_routes(self, snapshot)
    }
    fn copy_static_asset(
        &mut self,
        relative: &str,
        source: &Path,
    ) -> std::result::Result<(), PluginError> {
        SiteBuilder::copy_static_asset(self, relative, source).map_err(plugin_error)
    }
    fn remove_static_tree(&mut self, relative: &str) -> std::result::Result<(), PluginError> {
        SiteBuilder::remove_static_tree(self, relative).map_err(plugin_error)
    }
    fn write_meta(
        &mut self,
        directory: &str,
        meta_ts: &str,
    ) -> std::result::Result<(), PluginError> {
        SiteBuilder::write_meta(self, directory, meta_ts).map_err(plugin_error)
    }
    fn read_meta(&self, directory: &str) -> std::result::Result<String, PluginError> {
        SiteBuilder::read_meta(self, directory).map_err(plugin_error)
    }
    fn write_llm_files(
        &mut self,
        llms_txt: Option<&str>,
        llms_full_txt: Option<&str>,
    ) -> std::result::Result<(), PluginError> {
        SiteBuilder::write_llm_files(self, llms_txt, llms_full_txt, self.serve)
            .map_err(plugin_error)
    }
}

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;

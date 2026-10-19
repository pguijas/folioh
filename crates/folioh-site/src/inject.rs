//! `docs.yaml` values injected into the copied template by literal and
//! marker-block substitution, the generated `lib/` and `theme/` modules, the
//! `app/docs` relocation and the deploy base path.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use folioh_config::{join_lexical, DocsConfig, ThemeHeader};
use serde_json::{Map, Value};

use crate::fs::{
    collect_copyable_files, copy_tree, files_under, posix, remove_dir_all_if_exists,
    write_text_if_changed, INJECTED_ROOT_FILES,
};
use crate::json;
use crate::re;
use crate::template::docs_route_base;
use crate::theme::{
    check_theme_preset, declared_preset_ids, generate_typescript_contract,
    render_project_theme_module, validate_and_raise,
};
use crate::{Result, SiteError};
use folioh_plugins::{
    render_mdx_contract_module, Landing, LandingPageData, BUILTIN_COMPONENTS,
    FOLIOH_MDX_CONTRACT_VERSION,
};

/// The exact module written to `app/page.tsx` when the landing is off.
const DOCS_INDEX_WRAPPER: &str = "import DocsLayout from \"__DOCS_APP_PATH__/layout\"\nimport DocsPage, { generateMetadata as generateDocsMetadata } from \"__DOCS_APP_PATH__/[[...mdxPath]]/page\"\n\nfunction rootDocsProps() {\n  return {\n    params: Promise.resolve({ mdxPath: [] as string[] }),\n  }\n}\n\nexport async function generateMetadata() {\n  return generateDocsMetadata(rootDocsProps())\n}\n\nexport default function Home() {\n  return (\n    <DocsLayout>\n      <DocsPage {...rootDocsProps()} />\n    </DocsLayout>\n  )\n}\n";

/// What one injection run did.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Injection {
    /// Build-relative posix paths the injector wrote, sorted.
    pub injected_files: Vec<String>,
    /// `(target, reason)` for optional targets that were absent or unconfigured.
    pub skipped: Vec<(String, String)>,
    /// Warnings raised while rendering (an off-scale theme radius).
    pub warnings: Vec<String>,
}

fn io(path: &Path) -> impl Fn(std::io::Error) -> SiteError + '_ {
    move |e| SiteError::io(path, e)
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// `segment` percent-encoded outside the URL-safe characters.
fn url_path_segment(segment: &str) -> String {
    segment
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

/// Where a marker sits on its line of TSX/JS: code or JSX text, inside a
/// double-quoted literal opened at a byte offset, or inside a template literal.
#[derive(Clone, Copy)]
enum MarkerContext {
    Code,
    Double(usize),
    Template,
}

fn marker_context(content: &str, pos: usize) -> MarkerContext {
    let line_start = content[..pos].rfind('\n').map_or(0, |i| i + 1);
    let mut state = MarkerContext::Code;
    let mut chars = content[line_start..pos].char_indices();
    while let Some((i, c)) = chars.next() {
        state = match (state, c) {
            (MarkerContext::Code, '"') => MarkerContext::Double(line_start + i),
            (MarkerContext::Code, '`') => MarkerContext::Template,
            (MarkerContext::Double(_) | MarkerContext::Template, '\\') => {
                chars.next();
                continue;
            }
            (MarkerContext::Double(_), '"') | (MarkerContext::Template, '`') => MarkerContext::Code,
            (state, _) => state,
        };
    }
    state
}

/// A JSON string literal without its quotes.
fn json_inner(value: &str) -> String {
    let quoted = json::string(value);
    quoted[1..quoted.len() - 1].to_string()
}

/// `marker` replaced by `value` escaped for where each occurrence sits, so
/// a project name like `A "B" {c} <d>` can never break the page source:
///
/// - a whole `"marker"` literal becomes a JSON string, or `{json}` right after
///   an attribute `=`; a whole `'marker'` literal becomes a JSON string;
/// - inside a longer double-quoted literal it takes JSON escapes, or entity
///   escapes when the literal is a JSX attribute value (`alt="…"`);
/// - inside a template literal it takes template escapes;
/// - anywhere else it is JSX text and becomes `{json}`.
///
/// Theme packages and custom templates keep the bare markers they have always
/// used; each one gets the escaping its position needs.
pub(crate) fn substitute(content: &str, marker: &str, value: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut cursor = 0;
    for (pos, _) in content.match_indices(marker) {
        if pos < cursor {
            continue;
        }
        let end = pos + marker.len();
        let after = &content[end..];
        let replacement = match marker_context(content, pos) {
            MarkerContext::Double(open) if open + 1 == pos && after.starts_with('"') => {
                out.push_str(&content[cursor..open]);
                cursor = end + 1;
                if content[..open].ends_with('=') {
                    format!("{{{}}}", json::string(value))
                } else {
                    json::string(value)
                }
            }
            MarkerContext::Double(open) => {
                out.push_str(&content[cursor..pos]);
                cursor = end;
                if content[..open].ends_with('=') {
                    html_escape(value)
                } else {
                    json_inner(value)
                }
            }
            MarkerContext::Template => {
                out.push_str(&content[cursor..pos]);
                cursor = end;
                value
                    .replace('\\', "\\\\")
                    .replace('`', "\\`")
                    .replace("${", "\\${")
            }
            MarkerContext::Code if content[..pos].ends_with('\'') && after.starts_with('\'') => {
                out.push_str(&content[cursor..pos - 1]);
                cursor = end + 1;
                json::string(value)
            }
            MarkerContext::Code => {
                out.push_str(&content[cursor..pos]);
                cursor = end;
                format!("{{{}}}", json::string(value))
            }
        };
        out.push_str(&replacement);
    }
    out.push_str(&content[cursor..]);
    out
}

/// `theme.header` sets any of the action keys (`repo`, `theme_toggle`,
/// `action_label`, `action_href`, `search`).
fn header_has_actions(header: &ThemeHeader) -> bool {
    header.repo.is_some()
        || header.theme_toggle.is_some()
        || header.action_label.is_some()
        || header.action_href.is_some()
        || header.search.is_some()
}

/// `theme.header` sets nothing at all (the bundled placeholder markup stays).
fn header_is_empty(header: &ThemeHeader) -> bool {
    header.brand.is_none() && header.badge.is_none() && !header_has_actions(header)
}

/// Deploy base path: `FOLIOH_BASE_PATH` (even empty) > `deploy.base_path` >
/// GitHub Pages inference > `""`. `project.url` never feeds it.
pub fn resolve_base_path(config: &DocsConfig, env: &dyn Fn(&str) -> Option<String>) -> String {
    if let Some(value) = env("FOLIOH_BASE_PATH") {
        return normalize_base_path(&value);
    }
    if !config.deploy.base_path.is_empty() {
        return normalize_base_path(&config.deploy.base_path);
    }
    let provider = config.deploy.provider.trim().to_lowercase();
    let env_provider = env("FOLIOH_DEPLOY_PROVIDER")
        .unwrap_or_default()
        .trim()
        .to_lowercase();
    let on_actions = env("GITHUB_ACTIONS")
        .unwrap_or_default()
        .trim()
        .to_lowercase()
        == "true";
    if (provider == "github-pages" || env_provider == "github-pages") && on_actions {
        let repository = env("GITHUB_REPOSITORY").unwrap_or_default();
        let (owner, repo) = repository.split_once('/').unwrap_or((&repository, ""));
        if owner.is_empty()
            || repo.is_empty()
            || repo.to_lowercase() == format!("{}.github.io", owner.to_lowercase())
        {
            return String::new();
        }
        return normalize_base_path(repo);
    }
    String::new()
}

/// `resolve_base_path` against the process environment.
pub fn resolve_base_path_from_env(config: &DocsConfig) -> String {
    resolve_base_path(config, &|key| std::env::var(key).ok())
}

/// `""` or `/`-prefixed without a trailing `/`.
pub fn normalize_base_path(value: &str) -> String {
    folioh_config::normalize_base_path(&serde_yaml_ng::Value::String(value.to_string()))
}

/// Whether a plugin section publishes a view at `/` (`routes.public`).
pub fn plugin_view_owns_root(config: &DocsConfig) -> bool {
    config.extra.values().any(|section| {
        section
            .get("routes")
            .and_then(|routes| routes.get("public"))
            .and_then(Value::as_str)
            .map(str::trim)
            .map(|s| !s.is_empty() && format!("/{}", s.trim_matches('/')) == "/")
            .unwrap_or(false)
    })
}

/// Runs the injection steps over a prepared `.build/`.
pub struct TemplateConfigInjector<'a> {
    config: &'a DocsConfig,
    build_dir: PathBuf,
    template_dir: PathBuf,
    /// `build-versions` passes the version being built; plain builds pass `""`.
    pub current_version_path: String,
    theme_package_files: BTreeSet<PathBuf>,
    /// The directory the overlay came from: for a fetched package the cache
    /// entry, which no config field names.
    theme_package_dir: Option<PathBuf>,
    result: Injection,
}

impl<'a> TemplateConfigInjector<'a> {
    /// An injector over a prepared `build_dir`; `template_dir` supplies the pristine `next.config.mjs`.
    pub fn new(config: &'a DocsConfig, build_dir: &Path, template_dir: &Path) -> Self {
        TemplateConfigInjector {
            config,
            build_dir: build_dir.to_path_buf(),
            template_dir: template_dir.to_path_buf(),
            current_version_path: String::new(),
            theme_package_files: BTreeSet::new(),
            theme_package_dir: None,
            result: Injection::default(),
        }
    }

    /// Run every injection step in order.
    pub fn inject(mut self) -> Result<Injection> {
        let name = self.config.project.name.clone();
        self.apply_theme_package()?;
        self.inject_root_layout(&name)?;
        self.inject_docs_layout(&name)?;
        self.inject_docs_route_page(&name)?;
        self.inject_og_image(&name)?;
        self.inject_landing_page(&name)?;
        self.inject_previews_page(&name)?;
        self.inject_sitemap()?;
        self.inject_search_postbuild()?;
        self.inject_theme_config()?;
        self.inject_dark_mode()?;
        self.inject_preset_catalog()?;
        self.inject_next_config()?;
        self.inject_versions()?;
        self.write_template_context()?;
        self.relocate_docs_route()?;
        self.result.injected_files.sort();
        Ok(self.result)
    }

    fn name(&self) -> &str {
        &self.config.project.name
    }
    fn monogram(&self) -> String {
        self.name()
            .chars()
            .take(2)
            .collect::<String>()
            .to_lowercase()
    }
    /// `__PROJECT_NAME__` and `__PROJECT_MONOGRAM__` escaped where they land.
    fn substitute_identity(&self, content: &str, name: &str) -> String {
        substitute(
            &substitute(content, "__PROJECT_NAME__", name),
            "__PROJECT_MONOGRAM__",
            &self.monogram(),
        )
    }
    fn site_url(&self) -> String {
        self.config.project.url.trim_end_matches('/').to_string()
    }
    fn base(&self) -> String {
        docs_route_base(self.config)
    }
    fn route(&self, suffix: &str) -> String {
        let suffix = suffix.trim_matches('/');
        if suffix.is_empty() {
            format!("{}/", self.base())
        } else {
            format!("{}/{suffix}", self.base())
        }
    }
    fn route_segments(&self) -> Vec<String> {
        self.base()
            .trim_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect()
    }

    fn record(&mut self, path: &Path) {
        let rel = path.strip_prefix(&self.build_dir).unwrap_or(path);
        let rel = posix(rel);
        if !self.result.injected_files.contains(&rel) {
            self.result.injected_files.push(rel);
        }
    }
    fn skip(&mut self, target: &str, reason: &str) {
        self.result
            .skipped
            .push((target.to_string(), reason.to_string()));
    }
    fn read(&self, path: &Path) -> Result<String> {
        std::fs::read_to_string(path).map_err(io(path))
    }
    fn write(&mut self, path: &Path, content: &str) -> Result<()> {
        write_text_if_changed(path, content).map_err(io(path))?;
        self.record(path);
        Ok(())
    }
    fn package_owns(&self, rel: &str) -> bool {
        self.theme_package_files.contains(Path::new(rel))
    }
    fn project_path(&self, raw: &str) -> PathBuf {
        join_lexical(&self.config.project_dir, raw)
    }

    fn apply_theme_package(&mut self) -> Result<()> {
        let remote = self.config.theme.package_remote.as_ref();
        let package_path = match remote {
            Some(remote) => crate::theme_package::materialise(remote)?,
            None => {
                let package = &self.config.theme.package_path;
                if package.is_empty() {
                    return Ok(());
                }
                PathBuf::from(package)
            }
        };
        if !package_path.exists() {
            return Err(SiteError::NotFound(format!(
                "Theme package not found: {}",
                package_path.display()
            )));
        }
        if !package_path.is_dir() {
            return Err(SiteError::Value(format!(
                "Theme package must be a directory: {}",
                package_path.display()
            )));
        }
        let resolved = folioh_config::canonicalize_lenient(&package_path);
        if resolved.starts_with(folioh_config::canonicalize_lenient(&self.build_dir)) {
            return Err(SiteError::Value(
                "theme.package cannot point inside the build directory".to_string(),
            ));
        }
        self.theme_package_files = collect_copyable_files(&package_path, "theme.package")?;
        validate_and_raise(&package_path, remote.is_some())?;
        copy_tree(&package_path, &self.build_dir, &INJECTED_ROOT_FILES)
            .map_err(io(&self.build_dir))?;
        self.theme_package_dir = Some(package_path);
        Ok(())
    }

    fn inject_root_layout(&mut self, name: &str) -> Result<()> {
        let path = self.build_dir.join("app/layout.tsx");
        if !path.exists() {
            self.skip("app/layout.tsx", "file not present");
            return Ok(());
        }
        let mut content = self.read(&path)?;
        content = substitute(&content, "__PROJECT_NAME__", name);
        content = substitute(
            &content,
            "__PROJECT_DESCRIPTION__",
            &format!("Documentation for {name}"),
        );
        let site_url = self.site_url();
        if !site_url.is_empty() && !content.contains("metadataBase:") {
            content = content.replacen(
                "export const metadata = {\n",
                &format!(
                    "export const metadata = {{\n  metadataBase: new URL({}),\n",
                    json::string(&site_url)
                ),
                1,
            );
        }
        content = substitute(&content, "__SITE_URL__", &site_url);
        self.inject_favicon()?;
        self.write(&path, &content)
    }

    fn inject_favicon(&mut self) -> Result<()> {
        let default_icon = self.build_dir.join("app/icon.svg");
        if !self.config.theme.favicon.is_empty() {
            let source = self.project_path(&self.config.theme.favicon);
            if source.exists() {
                let ext = source
                    .extension()
                    .map(|e| format!(".{}", e.to_string_lossy()))
                    .unwrap_or_else(|| ".svg".to_string());
                let dest = self.build_dir.join("app").join(format!("icon{ext}"));
                std::fs::create_dir_all(dest.parent().unwrap()).map_err(io(&dest))?;
                std::fs::copy(&source, &dest).map_err(io(&dest))?;
                if ext != ".svg" && default_icon.exists() {
                    std::fs::remove_file(&default_icon).map_err(io(&default_icon))?;
                }
                return self.write_project_icon(None);
            }
        }
        let mut project_icon = None;
        if default_icon.exists() {
            let mut content = self
                .read(&default_icon)?
                .replace("__PROJECT_MONOGRAM__", &html_escape(&self.monogram()));
            // Only a marked, generated icon uses the site's configured preset.
            // Custom favicons and unmarked template/package icons stay theirs.
            let marker = re(r#"(<svg\b[^>]*\bdata-folioh-icon=")(?:default|pastel)(")"#);
            if self.config.theme.favicon.is_empty() && marker.is_match(&content) {
                let shape = if self.config.theme.preset == "pastel" {
                    "pastel"
                } else {
                    "default"
                };
                content = marker
                    .replace(&content, format!("${{1}}{shape}${{2}}"))
                    .into_owned();
                project_icon = Some(content.clone());
            }
            write_text_if_changed(&default_icon, &content).map_err(io(&default_icon))?;
        }
        self.write_project_icon(project_icon.as_deref())
    }

    fn write_project_icon(&mut self, svg: Option<&str>) -> Result<()> {
        self.write(
            &self.build_dir.join("theme/project-icon.ts"),
            &format!(
                "export const projectIconSvg: string | null = {}\n",
                json::compact(&svg)
            ),
        )
    }

    fn header_logo(&self) -> String {
        let header = &self.config.theme.header;
        if header_is_empty(header) {
            return String::new();
        }
        let brand = header
            .brand
            .as_deref()
            .filter(|b| !b.is_empty())
            .unwrap_or(self.name());
        let mut lines = vec![format!(
            "<span className=\"text-sm font-semibold tracking-tight\">{{{}}}</span>",
            json::string(brand)
        )];
        if let Some(badge) = header.badge.as_deref().filter(|b| !b.is_empty()) {
            lines.push(format!(
                "<span className=\"rounded-full bg-primary/10 px-2 py-0.5 text-[10px] font-medium text-primary\">{{{}}}</span>",
                json::string(badge)
            ));
        }
        lines.join("\n")
    }

    fn header_actions(&self) -> String {
        let header = &self.config.theme.header;
        if !header_has_actions(header) {
            return String::new();
        }
        let repo_href = header
            .repo
            .as_deref()
            .filter(|r| !r.is_empty())
            .unwrap_or(&self.config.project.repo);
        let mut props = Vec::new();
        if !repo_href.is_empty() {
            props.push(format!("repoHref={{{}}}", json::string(repo_href)));
        }
        if header.theme_toggle == Some(true) && self.config.theme.dark_mode {
            props.push("themeToggle".to_string());
        }
        if let (Some(href), Some(label)) = (
            header.action_href.as_deref().filter(|s| !s.is_empty()),
            header.action_label.as_deref().filter(|s| !s.is_empty()),
        ) {
            props.push(format!("actionHref={{{}}}", json::string(href)));
            props.push(format!("actionLabel={{{}}}", json::string(label)));
        }
        if props.is_empty() {
            return "<VersionSelector />".to_string();
        }
        let prop_lines: Vec<String> = props.iter().map(|p| format!("  {p}")).collect();
        format!(
            "<ProjectHeaderActions\n{}\n/>\n<VersionSelector />",
            prop_lines.join("\n")
        )
    }

    fn indent_block(indent: &str, block: &str) -> String {
        block
            .lines()
            .map(|line| format!("{indent}{line}"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// `theme.logo` as the root-relative URL of its copy in `public/`, which
    /// both navbars show; `None` without a logo.
    fn logo_src(&self) -> Option<String> {
        let file = Path::new(&self.config.theme.logo).file_name()?;
        Some(format!("/{}", url_path_segment(&file.to_string_lossy())))
    }

    /// The `<img>` for `theme.logo`, copied to `public/`: plain content,
    /// since Nextra already links the whole logo slot to the home page.
    fn logo_image(&self) -> Option<String> {
        Some(format!(
            "<img src={{(process.env.NEXT_PUBLIC_FOLIOH_BASE_PATH ?? \"\") + {}}} alt=\"\" className=\"h-7 w-auto\" />",
            json::string(&self.logo_src()?)
        ))
    }

    fn inject_header_logo(&self, content: &str) -> String {
        let mut logo = self.header_logo();
        if let Some(image) = self.logo_image() {
            // The image takes the monogram's place beside the name.
            let words = if logo.is_empty() {
                format!(
                    "<span className=\"text-sm font-semibold tracking-tight\">{{{}}}</span>",
                    json::string(self.name())
                )
            } else {
                logo
            };
            logo = format!("{image}\n{words}");
        }
        if logo.is_empty() {
            return re(r"(?m)^[ \t]*\{/\* __PROJECT_HEADER_LOGO_(?:START|END)__ \*/\}\n?")
                .replace_all(content, "")
                .into_owned();
        }
        re(r"(?ms)([ \t]*)\{/\* __PROJECT_HEADER_LOGO_START__ \*/\}\n.*?^[ \t]*\{/\* __PROJECT_HEADER_LOGO_END__ \*/\}")
            .replace_all(content, |caps: &regex::Captures| Self::indent_block(&caps[1], &logo))
            .into_owned()
    }

    fn inject_header_actions(&self, content: &str) -> String {
        let actions = self.header_actions();
        if actions.is_empty() {
            let content = re(r"(?m)^[ \t]*// __PROJECT_HEADER_ACTION_IMPORTS_(?:START|END)__\n?")
                .replace_all(content, "");
            return re(r"(?m)^[ \t]*\{/\* __PROJECT_HEADER_ACTIONS_(?:START|END)__ \*/\}\n?")
                .replace_all(&content, "")
                .into_owned();
        }
        let content = re(r"(?ms)([ \t]*)// __PROJECT_HEADER_ACTION_IMPORTS_START__\n.*?^[ \t]*// __PROJECT_HEADER_ACTION_IMPORTS_END__")
            .replace_all(content, |caps: &regex::Captures| format!("{}import {{ ProjectHeaderActions }} from \"@/components/project-header-actions\"", &caps[1]));
        re(r"(?ms)([ \t]*)\{/\* __PROJECT_HEADER_ACTIONS_START__ \*/\}\n.*?^[ \t]*\{/\* __PROJECT_HEADER_ACTIONS_END__ \*/\}")
            .replace_all(&content, |caps: &regex::Captures| Self::indent_block(&caps[1], &actions))
            .into_owned()
    }

    fn inject_repo_link(&self, content: &str) -> String {
        let repo = &self.config.project.repo;
        if repo.is_empty() {
            let content = re(r"(?ms)^[ \t]*// __PROJECT_REPO_IMPORTS_START__\n.*?^[ \t]*// __PROJECT_REPO_IMPORTS_END__\n?").replace_all(content, "");
            let content = re(r"(?ms)^[ \t]*\{/\* __PROJECT_REPO_LINK_START__ \*/\}\n.*?^[ \t]*\{/\* __PROJECT_REPO_LINK_END__ \*/\}\n?").replace_all(&content, "");
            return substitute(&content, "__PROJECT_REPO__", "");
        }
        let content = substitute(content, "__PROJECT_REPO__", repo);
        re(r"(?m)^[ \t]*(?:// __PROJECT_REPO_IMPORTS_(?:START|END)__|\{/\* __PROJECT_REPO_LINK_(?:START|END)__ \*/\})\n?")
            .replace_all(&content, "")
            .into_owned()
    }

    fn inject_search(&self, content: &str) -> String {
        if !self.config.search.enabled || self.config.theme.header.search == Some(false) {
            if content.contains("search={") {
                return content.to_string();
            }
            return content.replacen(
                "pageMap={await getPageMap",
                "search={null}\n      pageMap={await getPageMap",
                1,
            );
        }
        let import_line = "import { SearchCommand } from \"@/components/search-command\"";
        let mut content = content.to_string();
        if !content.contains(import_line) {
            let page_map_import = "import { getPageMap } from \"nextra/page-map\"";
            content = if content.contains(page_map_import) {
                content.replacen(
                    page_map_import,
                    &format!("{page_map_import}\n{import_line}"),
                    1,
                )
            } else {
                format!("{import_line}\n{content}")
            };
        }
        if content.contains("search={") {
            return content;
        }
        let component = if self.config.search.placeholder.is_empty() {
            "<SearchCommand />".to_string()
        } else {
            format!(
                "<SearchCommand placeholder=\"{}\" />",
                html_escape(&self.config.search.placeholder)
            )
        };
        content.replacen(
            "pageMap={await getPageMap",
            &format!("search={{{component}}}\n      pageMap={{await getPageMap"),
            1,
        )
    }

    fn inject_docs_layout(&mut self, name: &str) -> Result<()> {
        let path = self.build_dir.join("app/docs/layout.tsx");
        if !path.exists() {
            self.skip("app/docs/layout.tsx", "file not present");
            return Ok(());
        }
        let mut content = self.read(&path)?;
        content = self.substitute_identity(&content, name);
        content = self.inject_header_logo(&content);
        content = self.inject_header_actions(&content);
        let og = json::string(&self.route("opengraph-image"));
        content = content
            .replace("url: \"/docs/opengraph-image\"", &format!("url: {og}"))
            .replace(
                "images: [\"/docs/opengraph-image\"]",
                &format!("images: [{og}]"),
            )
            .replace(
                "getPageMap(\"/docs\")",
                &format!("getPageMap({})", json::string(&self.base())),
            );
        content = self.inject_repo_link(&content);
        // Never inherit Nextra's feedback defaults: a repository alone does
        // not enable feedback, and an opt-in without a repository has no target.
        let repo = &self.config.project.repo;
        let repo_props = if repo.is_empty() {
            "editLink={null}\n            feedback={{ content: null }}".to_string()
        } else {
            format!(
                "docsRepositoryBase={{{}}}\n            editLink={{null}}",
                json::string(repo)
            )
        };
        let repo_props = if !repo.is_empty() {
            let feedback = if self.config.feedback.enabled {
                "feedback={{ content: \"Feedback\", labels: \"feedback\" }}"
            } else {
                "feedback={{ content: null }}"
            };
            format!("{repo_props}\n            {feedback}")
        } else {
            repo_props
        };
        content = content.replace(
            "footer={<Footer />}",
            &format!("{repo_props}\n            footer={{<Footer />}}"),
        );
        if !self.config.theme.logo.is_empty() {
            let logo = self.project_path(&self.config.theme.logo);
            let Some(file) = logo.file_name().filter(|_| logo.is_file()) else {
                return Err(SiteError::NotFound(format!(
                    "theme.logo does not exist: {}",
                    logo.display()
                )));
            };
            let dest = self.build_dir.join("public").join(file);
            std::fs::create_dir_all(dest.parent().unwrap()).map_err(io(&dest))?;
            std::fs::copy(&logo, &dest).map_err(io(&dest))?;
        }
        content = self.inject_search(&content);
        self.write(&path, &content)
    }

    fn inject_docs_route_page(&mut self, name: &str) -> Result<()> {
        let path = self.build_dir.join("app/docs/[[...mdxPath]]/page.jsx");
        if !path.exists() {
            self.skip("app/docs/[[...mdxPath]]/page.jsx", "file not present");
            return Ok(());
        }
        let og = self.route("opengraph-image");
        let index_path = self.route("");
        let canonical = if self.config.landing_enabled || plugin_view_owns_root(self.config) {
            index_path.clone()
        } else {
            "/".to_string()
        };
        let mut content = self.read(&path)?;
        content = substitute(&content, "__PROJECT_NAME__", name);
        content = substitute(
            &content,
            "__PROJECT_DESCRIPTION__",
            &format!("Documentation for {name}"),
        );
        content = substitute(&content, "__SITE_URL__", &self.site_url());
        content = content
            .replace(
                "from \"../../../mdx-components\"",
                "from \"@/mdx-components\"",
            )
            .replace(
                "`${siteUrl}/docs/opengraph-image`",
                &format!("`${{siteUrl}}{og}`"),
            )
            .replace("\"/docs/opengraph-image\"", &json::string(&og))
            .replace(
                "docsIndexCanonicalPath === \"/\" ? \"/\" : \"/docs/\"",
                &format!(
                    "docsIndexCanonicalPath === \"/\" ? \"/\" : {}",
                    json::string(&index_path)
                ),
            )
            .replace(
                "`/docs/${mdxPath.join(\"/\")}/`",
                &format!("`{}/${{mdxPath.join(\"/\")}}/`", self.base()),
            );
        let content = substitute(&content, "__DOCS_INDEX_CANONICAL_PATH__", &canonical);
        self.write(&path, &content)
    }

    fn inject_previews_page(&mut self, name: &str) -> Result<()> {
        let path = self.build_dir.join("app/previews/layout.tsx");
        if !path.exists() {
            return Ok(());
        }
        let mut content = self.substitute_identity(&self.read(&path)?, name);
        content = self.inject_repo_link(&content);
        content = self.inject_search(&content);
        write_text_if_changed(&path, &content).map_err(io(&path))?;
        Ok(())
    }

    fn inject_og_image(&mut self, name: &str) -> Result<()> {
        for rel in ["app/opengraph-image.tsx", "app/docs/opengraph-image.tsx"] {
            let path = self.build_dir.join(rel);
            if !path.exists() {
                self.skip(rel, "optional Open Graph image file not present");
                continue;
            }
            let content = substitute(
                &self.substitute_identity(&self.read(&path)?, name),
                "__PROJECT_DESCRIPTION__",
                &format!("Documentation for {name}"),
            );
            self.write(&path, &content)?;
        }
        Ok(())
    }

    /// The landing built-in's derived page values (disabled defaults when
    /// `landing:` is absent, which still feed the navbar).
    fn landing_data(&self) -> LandingPageData {
        LandingPageData::derive(&Landing::from_config(self.config), self.config)
    }

    /// The landing navbar with the project's name, logo and calls to action.
    /// Its light/dark toggle stays only when `theme.header.theme_toggle` is
    /// true and dark mode is on, as in the docs navbar; otherwise the mode
    /// lives in the theme picker alone.
    fn inject_landing_navbar(&mut self, name: &str) -> Result<()> {
        let path = self.build_dir.join("components/landing-navbar.tsx");
        if !path.exists() {
            self.skip(
                "components/landing-navbar.tsx",
                "optional landing navbar file not present",
            );
            return Ok(());
        }
        let data = self.landing_data();
        let secondary_json = data
            .cta_secondary_link
            .as_deref()
            .map(json::string)
            .unwrap_or_else(|| "null".to_string());
        let logo_json = self
            .logo_src()
            .map(|src| json::string(&src))
            .unwrap_or_else(|| "null".to_string());
        let content = self
            .read(&path)?
            .replace("__PROJECT_NAME_JSON__", &json::string(name))
            .replace("__PROJECT_MONOGRAM_JSON__", &json::string(&self.monogram()))
            .replace("__PROJECT_LOGO_JSON__", &logo_json)
            .replace(
                "__LANDING_CTA_PRIMARY_TEXT_JSON__",
                &json::compact(&data.cta_primary_text),
            )
            .replace(
                "__LANDING_CTA_PRIMARY_LINK_JSON__",
                &json::compact(&data.cta_primary_link),
            )
            .replace(
                "__LANDING_CTA_SECONDARY_TEXT_JSON__",
                &json::compact(&data.cta_secondary_text),
            )
            .replace("__LANDING_CTA_SECONDARY_LINK_JSON__", &secondary_json);
        let content = substitute(
            &self.substitute_identity(&content, name),
            "__LANDING_CTA_SECONDARY_LINK__",
            data.cta_secondary_link.as_deref().unwrap_or(""),
        );
        let keep_toggle =
            self.config.theme.header.theme_toggle == Some(true) && self.config.theme.dark_mode;
        let content = if keep_toggle {
            content
        } else {
            re(r"(?m)^[ \t]*<ThemeToggle />\n")
                .replace_all(&content, "")
                .into_owned()
        };
        self.write(&path, &content)
    }

    /// Each landing marker with its value, and whether the value is text to
    /// escape where it lands (`true`) or JSON to paste as is (`false`).
    fn landing_replacements(&self) -> Vec<(&'static str, String, bool)> {
        let data = self.landing_data();
        let text = |value: &Value| match value.as_str() {
            Some(text) => (text.to_string(), true),
            None => (json::compact(value), false),
        };
        let secondary_json = data
            .cta_secondary_link
            .as_deref()
            .map(json::string)
            .unwrap_or_else(|| "null".to_string());
        let mut replacements: Vec<(&'static str, String, bool)> = [
            ("__PROJECT_NAME_JSON__", json::string(&data.name)),
            ("__PROJECT_MONOGRAM_JSON__", json::string(&data.monogram)),
            ("__PROJECT_VERSION_JSON__", json::string(&data.version)),
            ("__LANDING_TAGLINE_JSON__", json::string(&data.tagline)),
            (
                "__LANDING_NOTICE_TEXT_JSON__",
                json::string(&data.notice_text),
            ),
            (
                "__LANDING_NOTICE_LINK_JSON__",
                json::string(&data.notice_link),
            ),
            ("__LANDING_HEADLINE_JSON__", json::compact(&data.headline)),
            (
                "__LANDING_DESCRIPTION_JSON__",
                json::compact(&data.description),
            ),
            (
                "__LANDING_CTA_PRIMARY_TEXT_JSON__",
                json::compact(&data.cta_primary_text),
            ),
            (
                "__LANDING_CTA_PRIMARY_LINK_JSON__",
                json::compact(&data.cta_primary_link),
            ),
            (
                "__LANDING_CTA_SECONDARY_TEXT_JSON__",
                json::compact(&data.cta_secondary_text),
            ),
            ("__LANDING_CTA_SECONDARY_LINK_JSON__", secondary_json),
            (
                "__LANDING_HERO_VARIANT_JSON__",
                json::string(&data.hero_variant),
            ),
            ("__LANDING_SECTIONS__", json::compact(&data.sections)),
            (
                "__LANDING_INSTALL_COMMANDS__",
                json::compact(&data.install_commands),
            ),
            ("__LANDING_FEATURES__", json::compact(&data.features)),
        ]
        .into_iter()
        .map(|(marker, value)| (marker, value, false))
        .collect();
        for (marker, (value, escape)) in [
            ("__PROJECT_NAME__", (data.name.clone(), true)),
            ("__PROJECT_MONOGRAM__", (data.monogram.clone(), true)),
            ("__LANDING_TAGLINE__", (data.tagline.clone(), true)),
            ("__LANDING_HEADLINE__", text(&data.headline)),
            ("__LANDING_DESCRIPTION__", text(&data.description)),
            ("__LANDING_CTA_PRIMARY_TEXT__", text(&data.cta_primary_text)),
            ("__LANDING_CTA_PRIMARY_LINK__", text(&data.cta_primary_link)),
            (
                "__LANDING_CTA_SECONDARY_TEXT__",
                text(&data.cta_secondary_text),
            ),
            (
                "__LANDING_CTA_SECONDARY_LINK__",
                (data.cta_secondary_link.clone().unwrap_or_default(), true),
            ),
        ] {
            replacements.push((marker, value, escape));
        }
        replacements
    }

    fn inject_landing_page(&mut self, name: &str) -> Result<()> {
        self.inject_landing_navbar(name)?;
        if !self.config.landing_enabled {
            if !plugin_view_owns_root(self.config) {
                self.inject_docs_index_page()?;
            }
            return Ok(());
        }
        let replacements = self.landing_replacements();
        let apply = |text: &str| {
            replacements
                .iter()
                .fold(text.to_string(), |acc, (marker, value, escape)| {
                    if *escape {
                        substitute(&acc, marker, value)
                    } else {
                        acc.replace(marker, value)
                    }
                })
        };
        let page_path = self.build_dir.join("app/page.tsx");
        if page_path.exists() {
            let content = apply(&self.read(&page_path)?);
            self.write(&page_path, &content)?;
        } else {
            self.skip("app/page.tsx", "optional landing page file not present");
        }
        let app_dir = self.build_dir.join("app");
        for path in files_under(&app_dir)
            .into_iter()
            .filter(|p| p.file_name().map(|n| n == "page.tsx").unwrap_or(false))
        {
            if path == page_path {
                continue;
            }
            let text = self.read(&path)?;
            if !replacements
                .iter()
                .any(|(marker, _, _)| text.contains(marker))
            {
                continue;
            }
            let content = apply(&text);
            self.write(&path, &content)?;
        }
        Ok(())
    }

    fn inject_docs_index_page(&mut self) -> Result<()> {
        let path = self.build_dir.join("app/page.tsx");
        if !path.exists() {
            return Ok(());
        }
        let docs_app_path = format!("./{}", self.route_segments().join("/"));
        self.write(
            &path,
            &DOCS_INDEX_WRAPPER.replace("__DOCS_APP_PATH__", &docs_app_path),
        )
    }

    fn inject_sitemap(&mut self) -> Result<()> {
        let include_docs_index = self.config.landing_enabled || plugin_view_owns_root(self.config);
        for route in ["sitemap.ts", "robots.ts"] {
            let path = self.build_dir.join("app").join(route);
            if !path.exists() {
                self.skip(
                    &format!("app/{route}"),
                    "optional sitemap/robots file not present",
                );
                continue;
            }
            let mut content = self.read(&path)?;
            content = substitute(&content, "__SITE_URL__", &self.site_url());
            content = substitute(&content, "__DOCS_ROUTE_BASE__", &self.base());
            let content = content.replace(
                "__INCLUDE_DOCS_INDEX__",
                if include_docs_index { "true" } else { "false" },
            );
            self.write(&path, &content)?;
        }
        Ok(())
    }

    fn inject_search_postbuild(&mut self) -> Result<()> {
        if self.config.search.enabled {
            return Ok(());
        }
        let path = self.build_dir.join("package.json");
        if !path.exists() {
            self.skip("package.json", "file not present");
            return Ok(());
        }
        let mut package: Value = serde_json::from_str(&self.read(&path)?)
            .map_err(|e| SiteError::Value(format!("{}: {e}", path.display())))?;
        let Some(scripts) = package.get_mut("scripts").and_then(Value::as_object_mut) else {
            return Ok(());
        };
        scripts.remove("postbuild");
        let content = format!("{}\n", json::pretty(&package));
        self.write(&path, &content)
    }

    fn inject_theme_config(&mut self) -> Result<()> {
        self.write_project_theme_module()?;
        let rel = "components/theme-configurator.tsx";
        if self.package_owns(rel) {
            self.skip(rel, "provided by theme.package overlay");
            return Ok(());
        }
        let path = self.build_dir.join(rel);
        if !path.exists() {
            self.skip(rel, "optional theme configurator file not present");
            return Ok(());
        }
        // A template or an overlay that replaces the configurator owns its
        // preset list; otherwise the id must be one the drawer shows.
        let overlay = &self.config.template.overlay_path;
        let overlay_dir = (!overlay.is_empty()).then(|| self.project_path(overlay));
        let overlay_owns = overlay_dir
            .as_ref()
            .is_some_and(|dir| dir.join(rel).is_file());
        if self.config.template.path.is_empty() && !overlay_owns {
            let mut declared: Vec<String> = [self.theme_package_dir.clone(), overlay_dir]
                .iter()
                .flatten()
                .flat_map(|dir| declared_preset_ids(dir))
                .collect();
            declared.sort();
            declared.dedup();
            check_theme_preset(self.config, &declared)?;
        }
        let replacement = format!(
            "const configuredDefaultPresetId = {}",
            json::string(&self.config.theme.preset)
        );
        let content = self
            .read(&path)?
            .replace(
                "const configuredDefaultPresetId = \"organic-editorial\" // __FOLIOH_THEME_PRESET__",
                &replacement,
            )
            .replace(
                "const configuredDefaultPresetId = \"pastel\" // __FOLIOH_THEME_PRESET__",
                &replacement,
            );
        self.write(&path, &content)
    }

    fn inject_preset_catalog(&mut self) -> Result<()> {
        let path = self.build_dir.join("theme/preset-catalog.ts");
        if !path.exists() {
            return Ok(());
        }
        let content = self.read(&path)?.replace(
            "export const extraPresetsEnabled: boolean = false // __FOLIOH_EXTRA_PRESETS__",
            &format!(
                "export const extraPresetsEnabled: boolean = {}",
                self.config.theme.extra_presets
            ),
        );
        self.write(&path, &content)
    }

    /// `theme.dark_mode: false`: the theme provider forces light. A provider
    /// without the marker (a theme package's or a custom template's own)
    /// keeps dark mode, with a warning saying so.
    fn inject_dark_mode(&mut self) -> Result<()> {
        if self.config.theme.dark_mode {
            return Ok(());
        }
        const MARKER: &str = "const darkModeEnabled: boolean = true // __FOLIOH_DARK_MODE__";
        let provider = self.build_dir.join("components/theme-provider.tsx");
        let content = if provider.exists() {
            self.read(&provider)?
        } else {
            String::new()
        };
        if !content.contains(MARKER) {
            self.result.warnings.push(
                "theme.dark_mode: false has no effect: components/theme-provider.tsx does not carry the // __FOLIOH_DARK_MODE__ marker, so dark mode stays available".to_string(),
            );
            return Ok(());
        }
        self.write(
            &provider,
            &content.replace(MARKER, "const darkModeEnabled: boolean = false"),
        )?;
        Ok(())
    }

    fn write_project_theme_module(&mut self) -> Result<()> {
        let theme_dir = self.build_dir.join("theme");
        std::fs::create_dir_all(&theme_dir).map_err(io(&theme_dir))?;
        if self.package_owns("theme/project-theme.ts") {
            self.skip(
                "theme/project-theme.ts",
                "provided by theme.package overlay",
            );
        } else {
            let mut warnings = Vec::new();
            let module = render_project_theme_module(self.config, &mut warnings);
            self.result.warnings.extend(warnings);
            self.write(&theme_dir.join("project-theme.ts"), &module)?;
        }
        self.write(
            &theme_dir.join("theme-contract.generated.ts"),
            &generate_typescript_contract(),
        )
    }

    fn inject_next_config(&mut self) -> Result<()> {
        let source = match self.theme_package_dir.as_ref() {
            // The directory the overlay actually used, which for a fetched
            // package is a cache entry no config field names.
            Some(package) if self.package_owns("next.config.mjs") => {
                package.join("next.config.mjs")
            }
            _ => self.template_dir.join("next.config.mjs"),
        };
        if !source.exists() {
            self.skip("next.config.mjs", "file not present");
            return Ok(());
        }
        let base = self.base();
        let mut content = self
            .read(&source)?
            .replace(
                "const configuredBasePath = '' // __FOLIOH_BASE_PATH__",
                &format!(
                    "const configuredBasePath = {}",
                    json::string(&resolve_base_path_from_env(self.config))
                ),
            )
            .replace("__FOLIOH_DOCS_ROUTE_BASE__", &base);
        content = re(r#"contentDirBasePath:\s*(?:'/docs'|"/docs")"#)
            .replace_all(&content, |_: &regex::Captures| {
                format!("contentDirBasePath: {}", json::string(&base))
            })
            .into_owned();
        if !content.contains("NEXT_PUBLIC_FOLIOH_DOCS_ROUTE_BASE") {
            content = content.replace(
                "NEXT_PUBLIC_FOLIOH_BASE_PATH: basePath ?? \"\",",
                &format!("NEXT_PUBLIC_FOLIOH_BASE_PATH: basePath ?? \"\",\n    NEXT_PUBLIC_FOLIOH_DOCS_ROUTE_BASE: {},", json::string(&base)),
            );
        }
        let i18n = &self.config.i18n;
        if !i18n.locales.is_empty() && !i18n.default_locale.is_empty() {
            let codes: Vec<String> = i18n
                .locales
                .iter()
                .map(|l| format!("'{}'", l.get("code").and_then(Value::as_str).unwrap_or("")))
                .collect();
            content = content.replace(
                "__I18N_CONFIG__",
                &format!(
                    "i18n: {{\n    locales: [{}],\n    defaultLocale: '{}',\n  }},",
                    codes.join(", "),
                    i18n.default_locale
                ),
            );
        } else {
            content = content
                .replace("__I18N_CONFIG__\n", "")
                .replace("__I18N_CONFIG__", "");
        }
        let target = self.build_dir.join("next.config.mjs");
        if write_text_if_changed(&target, &content).map_err(io(&target))? {
            self.record(&target);
        }
        Ok(())
    }

    fn inject_versions(&mut self) -> Result<()> {
        let path = self.build_dir.join("components/version-selector.tsx");
        if !path.exists() {
            self.skip(
                "components/version-selector.tsx",
                "optional version selector file not present",
            );
            return Ok(());
        }
        let versions: Vec<Value> = self
            .config
            .versions
            .iter()
            .map(|v| {
                let path = v
                    .get("path")
                    .cloned()
                    .unwrap_or_else(|| Value::String(String::new()));
                let mut entry = Map::new();
                entry.insert(
                    "label".into(),
                    v.get("label").cloned().unwrap_or_else(|| path.clone()),
                );
                entry.insert("path".into(), path);
                if let Some(default_path) = v.get("default_path").filter(|d| match d {
                    Value::Null | Value::Bool(false) => false,
                    Value::String(s) => !s.is_empty(),
                    _ => true,
                }) {
                    entry.insert("defaultPath".into(), default_path.clone());
                }
                Value::Object(entry)
            })
            .collect();
        let content = self
            .read(&path)?
            .replace("__VERSIONS__", &json::compact(&versions))
            .replace(
                "__CURRENT_VERSION_PATH__",
                &json::string(&self.current_version_path),
            );
        self.write(&path, &content)
    }

    fn write_template_context(&mut self) -> Result<()> {
        let project = serde_json::json!({
            "name": self.config.project.name,
            "version": self.config.project.version,
            "repo": self.config.project.repo,
            "repoRef": self.config.project.repo_ref,
            "url": self.config.project.url,
        });
        let docs = serde_json::json!({"routeBase": self.base(), "mdxContractVersion": FOLIOH_MDX_CONTRACT_VERSION});
        let params = Value::Object(self.config.template.params.clone());
        let context = serde_json::json!({
            "project": project,
            "docs": docs,
            "template": {"params": params, "docsRouteBase": self.base(), "mdxContractVersion": FOLIOH_MDX_CONTRACT_VERSION},
        });
        let lib_dir = self.build_dir.join("lib");
        std::fs::create_dir_all(&lib_dir).map_err(io(&lib_dir))?;
        let content = format!(
            "export const foliohProject = {} as const\n\nexport const foliohTemplateParams = {} as const\n\nexport const foliohDocs = {} as const\n\nexport const foliohTemplateContext = {} as const\n",
            json::pretty(&project),
            json::pretty(&params),
            json::pretty(&docs),
            json::pretty(&context)
        );
        self.write(&lib_dir.join("folioh-template.ts"), &content)?;
        let module = render_mdx_contract_module(&BUILTIN_COMPONENTS);
        self.write(&lib_dir.join("folioh-mdx-contract.ts"), &module)
    }

    fn relocate_docs_route(&mut self) -> Result<()> {
        let source = self.build_dir.join("app/docs");
        let target = self
            .route_segments()
            .iter()
            .fold(self.build_dir.join("app"), |dir, seg| dir.join(seg));
        if source == target || !source.exists() {
            return Ok(());
        }
        if target.starts_with(&source) {
            if target.join("[[...mdxPath]]/page.jsx").exists() {
                let first = target
                    .strip_prefix(&source)
                    .ok()
                    .and_then(|rel| rel.components().next())
                    .map(|c| source.join(c.as_os_str()));
                if let Some(residue) = first {
                    remove_dir_all_if_exists(&residue).map_err(io(&residue))?;
                }
            }
            let temp = self.build_dir.join("app/__folioh_docs_route");
            remove_dir_all_if_exists(&temp).map_err(io(&temp))?;
            std::fs::rename(&source, &temp).map_err(io(&temp))?;
            std::fs::create_dir_all(target.parent().unwrap()).map_err(io(&target))?;
            std::fs::rename(&temp, &target).map_err(io(&target))?;
            return Ok(());
        }
        remove_dir_all_if_exists(&target).map_err(io(&target))?;
        std::fs::create_dir_all(target.parent().unwrap()).map_err(io(&target))?;
        std::fs::rename(&source, &target).map_err(io(&target))?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "inject_tests.rs"]
mod tests;

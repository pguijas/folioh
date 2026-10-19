//! `DocsConfig`: the typed `docs.yaml`, its loaders, `parse_docs_config` and
//! `resolve_paths`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_yaml_ng::{Mapping, Value};

use crate::components::{parse_components, ComponentsConfig};
use crate::error::{ConfigError, Loaded};
use crate::features::{is_feature_enabled_in, EXPERIMENTAL_ENV_VAR};
use crate::paths::{canonicalize_lenient, join_string, resolve_contained_dir, resolve_output_dir};
use crate::source::{parse_source, LanguageSource, SourceConfig};
use crate::template::{normalize_base_path, parse_template, TemplateConfig};
use crate::theme::{parse_theme, repo_url, ThemeConfig};
use crate::value::{bool_field, get, string_field, string_list, warn_unknown_keys};

/// The validated `docs.yaml`. Paths are as written until `resolve_paths`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DocsConfig {
    pub project: ProjectConfig,
    pub source: SourceConfig,
    /// `output:` as written (relative) until `resolve_paths`, absolute after.
    pub output_dir: String,
    pub theme: ThemeConfig,
    pub template: TemplateConfig,
    pub nav: Vec<String>,
    /// `public:` as written: project-relative files served from the site root.
    pub public: Vec<String>,
    pub sidebar: SidebarConfig,
    pub llm: LlmConfig,
    pub search: SearchConfig,
    pub feedback: FeedbackConfig,
    pub deploy: DeployConfig,
    pub components: ComponentsConfig,
    /// Empty unless `FOLIOH_EXPERIMENTAL` names `i18n`.
    pub i18n: I18nConfig,
    /// Empty unless `FOLIOH_EXPERIMENTAL` names `versions`.
    pub versions: Vec<serde_json::Value>,
    /// `true` iff `docs.yaml` has a `landing:` key; the landing built-in
    /// refines it from `landing.enabled` / the bool shorthand.
    pub landing_enabled: bool,
    /// Normalised sections written by the built-ins (`landing`, `roadmap`,
    /// `openapi`); folioh-config never writes to it.
    pub extra: BTreeMap<String, serde_json::Value>,
    /// Absolute, canonical project directory.
    pub project_dir: PathBuf,
}

/// `project:`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectConfig {
    pub name: String,
    pub version: String,
    pub repo: String,
    pub repo_ref: String,
    pub url: String,
}

/// `sidebar:`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SidebarConfig {
    pub default_collapsed: bool,
}

/// `llm:`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LlmConfig {
    pub generate_llms_txt: bool,
    pub generate_llms_full_txt: bool,
}

/// `search:`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SearchConfig {
    pub enabled: bool,
    pub placeholder: String,
}

/// `feedback:` is opt-in, independently of the repository link.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct FeedbackConfig {
    pub enabled: bool,
}

/// `deploy:`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct DeployConfig {
    pub provider: String,
    pub base_path: String,
}

/// `i18n:` (experimental).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct I18nConfig {
    pub default_locale: String,
    pub locales: Vec<serde_json::Value>,
}

/// Read and validate `docs.yaml`; the project directory is the file's parent.
pub fn load_docs_config(path: &Path) -> Result<Loaded<DocsConfig>, ConfigError> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    load_docs_config_in(path, parent)
}

/// Read and validate `docs.yaml` against an explicit project directory
/// (`--project-dir` / the positional argument).
pub fn load_docs_config_in(
    path: &Path,
    project_dir: &Path,
) -> Result<Loaded<DocsConfig>, ConfigError> {
    load_docs_config_with_keys(path, project_dir, &[])
}

/// Load Docs configuration with the keys declared by the registered extensions.
pub fn load_docs_config_with_keys(
    path: &Path,
    project_dir: &Path,
    extension_keys: &[&str],
) -> Result<Loaded<DocsConfig>, ConfigError> {
    if !path.exists() {
        return Err(ConfigError::NotFound(path.to_path_buf()));
    }
    let text = std::fs::read_to_string(path).map_err(|e| ConfigError::Read(e.to_string()))?;
    let document: Value =
        serde_yaml_ng::from_str(&text).map_err(|e| ConfigError::Yaml(e.to_string()))?;
    let mapping = match document {
        Value::Null => Mapping::new(),
        Value::Mapping(mapping) => mapping,
        _ => {
            return Err(ConfigError::Yaml(format!(
                "Config file must contain a mapping: {}",
                path.display()
            )))
        }
    };
    let mut warnings = Vec::new();
    let experimental = std::env::var(EXPERIMENTAL_ENV_VAR).unwrap_or_default();
    let config = parse_docs_config_with_keys(
        &mapping,
        &canonicalize_lenient(project_dir),
        &experimental,
        extension_keys,
        &mut warnings,
    )?;
    let raw = match serde_json::to_value(Value::Mapping(mapping)) {
        Ok(serde_json::Value::Object(raw)) => raw,
        _ => {
            return Err(ConfigError::Yaml(format!(
                "Config file must contain a mapping with string keys: {}",
                path.display()
            )))
        }
    };
    Ok(Loaded {
        config,
        warnings,
        raw,
    })
}

/// Raw mapping to typed config, reading `FOLIOH_EXPERIMENTAL` from the environment.
pub fn parse_docs_config(
    raw: &Mapping,
    project_dir: &Path,
    warnings: &mut Vec<String>,
) -> Result<DocsConfig, ConfigError> {
    let experimental = std::env::var(EXPERIMENTAL_ENV_VAR).unwrap_or_default();
    parse_docs_config_with(raw, project_dir, &experimental, warnings)
}

/// `(feature, key, what)` for the `docs.yaml` keys `FOLIOH_EXPERIMENTAL` gates.
const GATED_KEYS: [(&str, &str, &str); 2] = [
    ("i18n", "i18n", "translated docs"),
    ("versions", "versions", "versioned docs"),
];

const PROJECT_KEYS: [&str; 5] = ["name", "version", "repo", "repo_ref", "url"];
const DEPLOY_KEYS: [&str; 2] = ["provider", "base_path"];
const SIDEBAR_KEYS: [&str; 1] = ["default_collapsed"];
const LLM_KEYS: [&str; 2] = ["generate_llms_txt", "generate_llms_full_txt"];
const SEARCH_KEYS: [&str; 2] = ["enabled", "placeholder"];

fn core_mapping(raw: &Mapping, key: &str) -> Result<Mapping, ConfigError> {
    match raw.get(key) {
        None => Ok(Mapping::new()),
        Some(Value::Mapping(mapping)) => Ok(mapping.clone()),
        Some(_) => Err(ConfigError::field(key, format!("{key} must be a mapping"))),
    }
}

fn lenient_mapping<'a>(raw: &'a Mapping, key: &str) -> Option<&'a Mapping> {
    raw.get(key).and_then(Value::as_mapping)
}

fn repo_ref(value: &Value) -> String {
    match value.as_str().map(str::trim) {
        Some(text) if !text.is_empty() => text.to_string(),
        _ => "main".to_string(),
    }
}

/// Raw mapping to typed config with an explicit `FOLIOH_EXPERIMENTAL` value.
/// Warnings accumulate in emission order; the first invalid field wins.
pub fn parse_docs_config_with(
    raw: &Mapping,
    project_dir: &Path,
    experimental: &str,
    warnings: &mut Vec<String>,
) -> Result<DocsConfig, ConfigError> {
    parse_docs_config_with_keys(raw, project_dir, experimental, &[], warnings)
}

fn parse_docs_config_with_keys(
    raw: &Mapping,
    project_dir: &Path,
    experimental: &str,
    extension_keys: &[&str],
    warnings: &mut Vec<String>,
) -> Result<DocsConfig, ConfigError> {
    let mut allowed = crate::known_config_keys();
    allowed.extend_from_slice(extension_keys);
    warn_unknown_keys(raw, &allowed, "config", warnings);
    // Present at all, `null` included: an empty key is still a claim the
    // release does not honour.
    if raw.contains_key("plugins") {
        warnings.push(
            "project plugins are not available in this release; the plugins key is ignored"
                .to_string(),
        );
    }
    for (feature, key, what) in GATED_KEYS {
        if raw.contains_key(key) && !is_feature_enabled_in(feature, experimental) {
            warnings.push(format!(
                "{what} are not available in this release; the {key} key is ignored"
            ));
        }
    }

    let project = core_mapping(raw, "project")?;
    let source = core_mapping(raw, "source")?;
    let theme = core_mapping(raw, "theme")?;
    let llm = core_mapping(raw, "llm")?;
    let feedback = core_mapping(raw, "feedback")?;
    warn_unknown_keys(&feedback, &["enabled"], "feedback", warnings);
    warn_unknown_keys(&project, &PROJECT_KEYS, "project", warnings);
    warn_unknown_keys(&llm, &LLM_KEYS, "llm", warnings);
    for (key, allowed) in [
        ("deploy", &DEPLOY_KEYS[..]),
        ("sidebar", &SIDEBAR_KEYS[..]),
        ("search", &SEARCH_KEYS[..]),
    ] {
        if let Some(section) = lenient_mapping(raw, key) {
            warn_unknown_keys(section, allowed, key, warnings);
        }
    }

    let name = match get(&project, "name").as_str() {
        Some(name) if !name.trim().is_empty() => name.to_string(),
        _ => {
            warnings.push(
                "project.name must be a non-empty string in docs.yaml; defaulting to 'Untitled'"
                    .to_string(),
            );
            "Untitled".to_string()
        }
    };
    let source = parse_source(&source, warnings)?;
    let nav = string_list(raw.get("nav"), "nav")?;
    let public = string_list(raw.get("public"), "public")?;
    let components = parse_components(raw.get("components"), warnings)?;

    let project = ProjectConfig {
        name,
        version: string_field(&project, "version", "project.version", "0.0.0")?,
        repo: repo_url(get(&project, "repo"), "project.repo")?,
        repo_ref: repo_ref(get(&project, "repo_ref")),
        url: string_field(&project, "url", "project.url", "")?,
    };
    let deploy =
        lenient_mapping(raw, "deploy").map_or_else(DeployConfig::default, |deploy| DeployConfig {
            provider: get(deploy, "provider")
                .as_str()
                .unwrap_or_default()
                .to_string(),
            base_path: normalize_base_path(get(deploy, "base_path")),
        });
    let output_dir = match raw.get("output") {
        None => "_site".to_string(),
        Some(Value::String(output)) => output.clone(),
        Some(_) => {
            return Err(ConfigError::field(
                "output",
                "Output directory must be a non-empty relative path",
            ))
        }
    };
    let theme = parse_theme(&theme, warnings)?;
    let template = parse_template(raw.get("template"), warnings)?;
    let sidebar = SidebarConfig {
        default_collapsed: lenient_mapping(raw, "sidebar").map_or(Ok(true), |s| {
            bool_field(s, "default_collapsed", "sidebar.default_collapsed", true)
        })?,
    };
    let llm = LlmConfig {
        generate_llms_txt: bool_field(&llm, "generate_llms_txt", "llm.generate_llms_txt", true)?,
        generate_llms_full_txt: bool_field(
            &llm,
            "generate_llms_full_txt",
            "llm.generate_llms_full_txt",
            true,
        )?,
    };
    let search = match lenient_mapping(raw, "search") {
        Some(search) => SearchConfig {
            enabled: bool_field(search, "enabled", "search.enabled", true)?,
            placeholder: string_field(search, "placeholder", "search.placeholder", "")?,
        },
        None => SearchConfig {
            enabled: true,
            placeholder: String::new(),
        },
    };
    let feedback = FeedbackConfig {
        enabled: bool_field(&feedback, "enabled", "feedback.enabled", false)?,
    };
    let i18n = match lenient_mapping(raw, "i18n")
        .filter(|_| is_feature_enabled_in("i18n", experimental))
    {
        Some(i18n) => I18nConfig {
            default_locale: string_field(i18n, "default_locale", "i18n.default_locale", "")?,
            locales: json_list(i18n.get("locales"), "i18n.locales")?,
        },
        None => I18nConfig::default(),
    };
    let versions = if is_feature_enabled_in("versions", experimental) {
        json_list(raw.get("versions"), "versions")?
    } else {
        Vec::new()
    };

    Ok(DocsConfig {
        project,
        source,
        output_dir,
        theme,
        template,
        nav,
        public,
        sidebar,
        llm,
        search,
        feedback,
        deploy,
        components,
        i18n,
        versions,
        landing_enabled: raw.contains_key("landing"),
        extra: BTreeMap::new(),
        project_dir: project_dir.to_path_buf(),
    })
}

fn json_list(value: Option<&Value>, key: &str) -> Result<Vec<serde_json::Value>, ConfigError> {
    match value {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Sequence(items)) => items
            .iter()
            .map(|item| {
                serde_json::to_value(item)
                    .map_err(|_| ConfigError::field(key, format!("{key} must be a list")))
            })
            .collect(),
        Some(_) => Err(ConfigError::field(key, format!("{key} must be a list"))),
    }
}

impl DocsConfig {
    /// The configured roots of a language; unknown or unconfigured is empty.
    pub fn language_source(&self, language: &str) -> LanguageSource {
        self.source
            .languages
            .get(language)
            .cloned()
            .unwrap_or_default()
    }

    /// Every configured source root in `resolve_output_dir` check order:
    /// python, the other languages in map order, then docs.
    pub fn source_roots(&self) -> Vec<&str> {
        self.source
            .languages
            .values()
            .flat_map(|source| source.paths.iter())
            .chain(self.source.docs.iter())
            .map(String::as_str)
            .collect()
    }

    /// A copy with every path made absolute against `base` and `output_dir`
    /// validated; `project_dir` becomes the canonical `base`.
    pub fn resolve_paths(&self, base: &Path) -> Result<DocsConfig, ConfigError> {
        let output = resolve_output_dir(base, &self.output_dir, &self.source_roots())?;
        let mut out = self.clone();
        out.output_dir = output.to_string_lossy().into_owned();
        let join = |paths: &mut Vec<String>| {
            for path in paths {
                *path = join_string(base, path);
            }
        };
        for source in out.source.languages.values_mut() {
            join(&mut source.paths);
            join(&mut source.excludes);
        }
        join(&mut out.source.docs);
        if !out.theme.package_path.is_empty() {
            out.theme.package_path = resolve_contained_dir(
                Path::new(&self.theme.package_path),
                base,
                &output,
                "theme.package",
                false,
            )?
            .to_string_lossy()
            .into_owned();
        }
        for path in [&mut out.template.path, &mut out.template.overlay_path] {
            if !path.is_empty() {
                *path = join_string(base, path);
            }
        }
        out.components = self.components.resolved(base);
        out.components.validate_contained(base)?;
        out.project_dir = canonicalize_lenient(base);
        Ok(out)
    }
}

#[cfg(test)]
#[path = "docs_testing.rs"]
pub(crate) mod testing;

#[cfg(test)]
#[path = "docs_tests.rs"]
mod tests;

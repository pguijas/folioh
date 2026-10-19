//! Folioh Docs configuration, loading and validation: one error
//! message per field, warnings as data, path resolution, `slugify`, the
//! `FOLIOH_EXPERIMENTAL` gate and the `FOLIOH_TRACE` JSONL writer.

pub mod components;
pub mod docs;
pub mod error;
pub mod features;
pub mod paths;
pub mod slug;
pub mod source;
pub mod suggest;
pub mod template;
pub mod theme;
pub mod trace;
mod value;

pub use components::{ComponentSpec, ComponentsConfig};
pub use docs::{
    load_docs_config, load_docs_config_in, load_docs_config_with_keys, parse_docs_config,
    parse_docs_config_with, DeployConfig, DocsConfig, FeedbackConfig, I18nConfig, LlmConfig,
    ProjectConfig, SearchConfig, SidebarConfig,
};
pub use error::{ConfigError, Loaded};
pub use features::{
    disabled_feature_message, experimental_feature_state, experimental_feature_state_in,
    is_feature_enabled, is_feature_enabled_in, DISABLED_FEATURES, EXPERIMENTAL_ENV_VAR,
};
pub use paths::{canonicalize_lenient, join_lexical, resolve_contained_dir, resolve_output_dir};
pub use slug::{slug_label, slugify, title_case};
pub use source::{
    default_route, language_label, LanguageSource, SourceConfig, API_REFERENCE_SLUG,
    DEFAULT_DOCSTRING_STYLE, DOCSTRING_STYLES, LANGUAGE_IDS,
};
pub use suggest::{closest_match, did_you_mean};
pub use template::{normalize_base_path, normalize_docs_route_base, TemplateConfig};
pub use theme::{
    RemoteThemePackage, ThemeConfig, ThemeHeader, ThemeVariantControl, ThemeVariantOption,
    THEME_RADIUS_ALIASES, THEME_RADIUS_OPTIONS, THEME_TUNE_ALIASES, THEME_TUNE_KEYS,
    THEME_TUNE_OPTIONS, THEME_VARIANT_COMBINATION_LIMIT,
};
pub use trace::Tracer;

/// The core `docs.yaml` keys, in the order `/_folioh/contract.json` publishes them.
pub const CORE_CONFIG_KEYS: [&str; 16] = [
    "project",
    "source",
    "output",
    "theme",
    "nav",
    "public",
    "sidebar",
    "llm",
    "plugins",
    "components",
    "i18n",
    "search",
    "feedback",
    "versions",
    "deploy",
    "template",
];

/// The sections the compiled-in integrations claim; present = active.
pub const BUILTIN_SECTION_KEYS: [&str; 3] = ["landing", "roadmap", "openapi"];

/// Every known top-level key: `CORE_CONFIG_KEYS` then `BUILTIN_SECTION_KEYS`
/// (the `configKeys` array of the authoring contract).
pub fn known_config_keys() -> Vec<&'static str> {
    CORE_CONFIG_KEYS
        .iter()
        .chain(BUILTIN_SECTION_KEYS.iter())
        .copied()
        .collect()
}

#[cfg(test)]
mod tests;

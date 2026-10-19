//! `theme.*`: the project theme contract validators and `ThemeConfig`.

use indexmap::IndexMap;
use serde::Serialize;
use serde_yaml_ng::{Mapping, Value};

use crate::error::ConfigError;
use crate::slug::slug_label;
use crate::suggest::did_you_mean;
use crate::value::{
    bool_field, get, is_blank, quoted_value, string_field, warn_unknown_keys, NULL,
};

/// Canonical `theme.tune` ids.
pub const THEME_TUNE_KEYS: [&str; 8] = [
    "fontId",
    "colorId",
    "surfaceColorId",
    "shellPaddingId",
    "contentWidthId",
    "rhythmId",
    "borderId",
    "codeTreatmentId",
];

/// The option ids the bundled configurator offers for each `theme.tune` id,
/// in the order it lists them.
pub const THEME_TUNE_OPTIONS: [(&str, &[&str]); 8] = [
    (
        "fontId",
        &[
            "folioh",
            "sans",
            "geist",
            "serif",
            "mono",
            "terminal",
            "grotesque",
        ],
    ),
    ("colorId", &["ink", "laurel", "indigo", "copper"]),
    ("surfaceColorId", &["preset", "paper", "moss", "mist"]),
    ("shellPaddingId", &["preset", "flush", "frame", "gallery"]),
    ("contentWidthId", &["preset", "focus", "docs", "wide"]),
    ("rhythmId", &["preset", "compact", "balanced", "roomy"]),
    ("borderId", &["preset", "fine", "structured", "ruled"]),
    (
        "codeTreatmentId",
        &["preset", "soft", "framed", "plate", "terminal"],
    ),
];

/// User-facing `theme.tune` aliases and their canonical id.
pub const THEME_TUNE_ALIASES: [(&str, &str); 20] = [
    ("accent", "colorId"),
    ("accent_color", "colorId"),
    ("border", "borderId"),
    ("borders", "borderId"),
    ("code", "codeTreatmentId"),
    ("code_blocks", "codeTreatmentId"),
    ("code_treatment", "codeTreatmentId"),
    ("color", "colorId"),
    ("color_id", "colorId"),
    ("content_width", "contentWidthId"),
    ("font", "fontId"),
    ("font_id", "fontId"),
    ("reading", "rhythmId"),
    ("rhythm", "rhythmId"),
    ("shell", "shellPaddingId"),
    ("shell_padding", "shellPaddingId"),
    ("shell_spacing", "shellPaddingId"),
    ("surface", "surfaceColorId"),
    ("surface_color", "surfaceColorId"),
    ("width", "contentWidthId"),
];

/// The fixed radius scale the template understands.
pub const THEME_RADIUS_OPTIONS: [&str; 5] = ["0", "0.3rem", "0.5rem", "0.75rem", "1rem"];
/// Legacy named radii and the scale value they map onto.
pub const THEME_RADIUS_ALIASES: [(&str, &str); 5] = [
    ("none", "0"),
    ("sm", "0.3rem"),
    ("md", "0.5rem"),
    ("lg", "0.75rem"),
    ("full", "1rem"),
];
/// Cap on the cartesian product of `theme.variants` option counts.
pub const THEME_VARIANT_COMBINATION_LIMIT: usize = 256;

/// `theme:` as validated at load; `package_path` is relative until `resolve_paths`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ThemeConfig {
    pub preset: String,
    pub dark_mode: bool,
    pub extra_presets: bool,
    pub name: String,
    pub description: String,
    pub scene: String,
    pub preview: IndexMap<String, String>,
    pub radius: String,
    pub tune: IndexMap<String, String>,
    pub style: IndexMap<String, String>,
    pub tokens: IndexMap<String, IndexMap<String, String>>,
    pub header: ThemeHeader,
    pub variants: IndexMap<String, ThemeVariantControl>,
    pub package_path: String,
    /// Set instead of `package_path` when `theme.package` names a remote
    /// package; the directory only exists once the build materialises it.
    pub package_remote: Option<RemoteThemePackage>,
    pub logo: String,
    pub favicon: String,
}

/// `theme.package` as a remote reference: a git repository, the revision to
/// read it at, and the SHA-256 digest of the tree that revision must produce.
/// The digest is the pin. A fetch that produces any other tree fails the
/// build, so a theme nobody in the project wrote still cannot change under it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RemoteThemePackage {
    pub git: String,
    pub rev: String,
    pub digest: String,
    /// The subdirectory of the repository holding the package; empty is its root.
    pub path: String,
}

/// The keys a remote `theme.package` mapping may carry.
const REMOTE_PACKAGE_KEYS: [&str; 4] = ["git", "rev", "digest", "path"];

/// The URL forms a remote package may be fetched over. There is no plain
/// `http`: the digest would still catch a tampered tree, but nothing is gained
/// by inviting the round trip that produces it. `file://` is a mirror on disk,
/// and what the tests fetch from.
const REMOTE_PACKAGE_SCHEMES: [&str; 4] = ["https://", "ssh://", "git@", "file://"];

/// One required, non-empty, whitespace-free key of a remote package.
fn remote_field(map: &Mapping, key: &str) -> Result<String, ConfigError> {
    let path = format!("theme.package.{key}");
    let value = string_field(map, key, &path, "")?.trim().to_string();
    if value.is_empty() {
        return Err(ConfigError::field(
            &path,
            format!("{path} is required and must not be empty"),
        ));
    }
    if value.chars().any(char::is_whitespace) {
        return Err(ConfigError::field(
            &path,
            format!("{path} must not contain whitespace"),
        ));
    }
    // Both values reach git as arguments. A leading dash would reach it as an
    // option instead, so it is refused here rather than escaped there.
    if value.starts_with('-') {
        return Err(ConfigError::field(
            &path,
            format!("{path} must not start with '-'"),
        ));
    }
    Ok(value)
}

/// `theme.package`: a directory in the project as a string, or a mapping
/// pinning a package someone else published. The mapping is what makes a
/// theme installable by a project that did not write it.
fn theme_package(value: &Value) -> Result<(String, Option<RemoteThemePackage>), ConfigError> {
    if is_blank(Some(value)) {
        return Ok((String::new(), None));
    }
    if let Some(text) = value.as_str() {
        return Ok((text.trim().to_string(), None));
    }
    // A scalar that is not a string has always been ignored here; only a
    // mapping means the remote form.
    let Some(map) = value.as_mapping() else {
        return Ok((String::new(), None));
    };
    let unknown: Vec<&str> = map
        .keys()
        .filter_map(Value::as_str)
        .filter(|key| !REMOTE_PACKAGE_KEYS.contains(key))
        .collect();
    if !unknown.is_empty() {
        return Err(ConfigError::field(
            "theme.package",
            format!(
                "unknown keys in theme.package: {}. A remote package takes git, rev, digest and the optional path",
                unknown.join(", ")
            ),
        ));
    }
    let git = remote_field(map, "git")?;
    if !REMOTE_PACKAGE_SCHEMES
        .iter()
        .any(|scheme| git.starts_with(scheme))
    {
        return Err(ConfigError::field(
            "theme.package.git",
            "theme.package.git must start with https://, ssh://, git@ or file://",
        ));
    }
    let rev = remote_field(map, "rev")?;
    let digest = remote_field(map, "digest")?.to_ascii_lowercase();
    let hex = digest.strip_prefix("sha256:").unwrap_or_default();
    if hex.len() != 64 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ConfigError::field(
            "theme.package.digest",
            "theme.package.digest must read sha256: followed by 64 hex characters",
        ));
    }
    let raw = string_field(map, "path", "theme.package.path", "")?
        .trim()
        .to_string();
    if raw.contains('\\') || raw.split('/').any(|part| part == "..") {
        return Err(ConfigError::field(
            "theme.package.path",
            "theme.package.path must be a relative path inside the repository, without '..'",
        ));
    }
    // `themes/acme`, `/themes/acme/` and `./themes/acme` name the same place.
    let path = raw
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .collect::<Vec<_>>()
        .join("/");
    Ok((
        String::new(),
        Some(RemoteThemePackage {
            git,
            rev,
            digest,
            path,
        }),
    ))
}

/// `theme.header`: only the keys the project set are emitted.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct ThemeHeader {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brand: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub badge: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_href: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme_toggle: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<bool>,
}

/// One `theme.variants` control.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ThemeVariantControl {
    pub label: String,
    pub description: String,
    pub default: String,
    pub options: IndexMap<String, ThemeVariantOption>,
}

/// One option of a `theme.variants` control.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ThemeVariantOption {
    pub label: String,
    pub description: String,
    pub swatch: String,
    pub preview: IndexMap<String, String>,
    pub style: IndexMap<String, String>,
    pub tokens: IndexMap<String, IndexMap<String, String>>,
}

fn is_css_custom_property(key: &str) -> bool {
    let Some(rest) = key.strip_prefix("--") else {
        return false;
    };
    let mut chars = rest.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphanumeric())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn is_variant_id(id: &str) -> bool {
    let mut chars = id.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// `^[A-Za-z][A-Za-z0-9+.-]*:`
fn has_scheme(value: &str) -> bool {
    let Some(colon) = value.find(':') else {
        return false;
    };
    let scheme = &value[..colon];
    let mut chars = scheme.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'))
}

fn starts_with_any_scheme(value: &str, schemes: &[&str]) -> bool {
    let lower = value.to_ascii_lowercase();
    schemes.iter().any(|scheme| lower.starts_with(scheme))
}

/// A non-string is `""`; no strip.
fn theme_string(value: &Value) -> String {
    value.as_str().unwrap_or_default().to_string()
}

/// `_theme_text`: blank is `""`; a string is stripped and must not contain
/// `<`, `>` or line breaks.
pub fn theme_text(value: &Value, path: &str) -> Result<String, ConfigError> {
    if is_blank(Some(value)) {
        return Ok(String::new());
    }
    let Some(text) = value.as_str() else {
        return Err(ConfigError::field(path, format!("{path} must be a string")));
    };
    let stripped = text.trim();
    if stripped
        .chars()
        .any(|c| matches!(c, '<' | '>' | '\n' | '\r'))
    {
        return Err(ConfigError::field(
            path,
            format!("{path} contains unsafe text"),
        ));
    }
    Ok(stripped.to_string())
}

/// `_theme_href`: `theme_text`, then only `http(s)`/`mailto` or a scheme-less path.
pub fn theme_href(value: &Value, path: &str) -> Result<String, ConfigError> {
    let text = theme_text(value, path)?;
    if has_scheme(&text) && !starts_with_any_scheme(&text, &["http:", "https:", "mailto:"]) {
        return Err(ConfigError::field(
            path,
            format!("{path} must be an http(s) URL or a relative path"),
        ));
    }
    Ok(text)
}

/// `_repo_url`: any git URL form, minus the script/file schemes.
pub fn repo_url(value: &Value, path: &str) -> Result<String, ConfigError> {
    let text = theme_text(value, path)?;
    if starts_with_any_scheme(&text, &["javascript:", "data:", "vbscript:", "file:"]) {
        return Err(ConfigError::field(
            path,
            format!("{path} cannot use the javascript:, data:, vbscript:, or file: scheme"),
        ));
    }
    Ok(text)
}

/// `_theme_bool`: bools, the words `true/yes/1/on` and `false/no/0/off`; blank is unset.
pub fn theme_bool(value: &Value, path: &str) -> Result<Option<bool>, ConfigError> {
    if is_blank(Some(value)) {
        return Ok(None);
    }
    if let Some(b) = value.as_bool() {
        return Ok(Some(b));
    }
    if let Some(text) = value.as_str() {
        match text.trim().to_lowercase().as_str() {
            "true" | "yes" | "1" | "on" => return Ok(Some(true)),
            "false" | "no" | "0" | "off" => return Ok(Some(false)),
            _ => {}
        }
    }
    Err(ConfigError::field(
        path,
        format!("{path} must be a boolean"),
    ))
}

fn css_value(value: &Value, path: &str) -> Result<String, ConfigError> {
    let Some(text) = value.as_str() else {
        return Err(ConfigError::field(
            path,
            format!("{path} values must be strings"),
        ));
    };
    let stripped = text.trim();
    if stripped.is_empty()
        || stripped
            .chars()
            .any(|c| matches!(c, ';' | '{' | '}' | '\n' | '\r' | '<' | '>'))
    {
        return Err(ConfigError::field(
            path,
            format!("{path} contains an unsafe CSS value"),
        ));
    }
    Ok(stripped.to_string())
}

fn theme_swatch(value: &Value, path: &str) -> Result<String, ConfigError> {
    if is_blank(Some(value)) {
        return Ok(String::new());
    }
    css_value(value, path)
}

/// `_theme_css_vars`: a mapping of `--custom-property` to a safe CSS value.
pub fn theme_css_vars(value: &Value, path: &str) -> Result<IndexMap<String, String>, ConfigError> {
    if is_blank(Some(value)) {
        return Ok(IndexMap::new());
    }
    let Some(mapping) = value.as_mapping() else {
        return Err(ConfigError::field(
            path,
            format!("{path} must be a mapping of CSS custom properties"),
        ));
    };
    let mut vars = IndexMap::new();
    for (key, raw) in mapping {
        let Some(name) = key.as_str().filter(|k| is_css_custom_property(k)) else {
            return Err(ConfigError::field(
                path,
                format!(
                    "{path} contains invalid CSS custom property {}",
                    quoted_value(key)
                ),
            ));
        };
        vars.insert(name.to_string(), css_value(raw, path)?);
    }
    Ok(vars)
}

/// `_theme_tokens`: `light`/`dark` CSS variable maps; empty modes are omitted.
pub fn theme_tokens(
    value: &Value,
) -> Result<IndexMap<String, IndexMap<String, String>>, ConfigError> {
    if is_blank(Some(value)) {
        return Ok(IndexMap::new());
    }
    let Some(mapping) = value.as_mapping() else {
        return Err(ConfigError::field(
            "theme.tokens",
            "theme.tokens must be a mapping".to_string(),
        ));
    };
    let mut tokens = IndexMap::new();
    for mode in ["light", "dark"] {
        let vars = theme_css_vars(get(mapping, mode), &format!("theme.tokens.{mode}"))?;
        if !vars.is_empty() {
            tokens.insert(mode.to_string(), vars);
        }
    }
    Ok(tokens)
}

/// `_theme_preview`: `light`/`dark` non-blank strings, stripped; other keys warn.
pub fn theme_preview(value: &Value, warnings: &mut Vec<String>) -> IndexMap<String, String> {
    let Some(mapping) = value.as_mapping() else {
        return IndexMap::new();
    };
    for key in mapping.keys() {
        if !matches!(key.as_str(), Some("light") | Some("dark")) {
            warnings.push(format!(
                "Unknown theme.preview key {}; ignoring it",
                quoted_value(key)
            ));
        }
    }
    let mut preview = IndexMap::new();
    for mode in ["light", "dark"] {
        if let Some(text) = get(mapping, mode)
            .as_str()
            .map(str::trim)
            .filter(|t| !t.is_empty())
        {
            preview.insert(mode.to_string(), text.to_string());
        }
    }
    preview
}

/// `_theme_header`: text, href and bool keys; unknown keys warn.
pub fn theme_header(value: &Value, warnings: &mut Vec<String>) -> Result<ThemeHeader, ConfigError> {
    if is_blank(Some(value)) {
        return Ok(ThemeHeader::default());
    }
    let Some(mapping) = value.as_mapping() else {
        return Err(ConfigError::field(
            "theme.header",
            "theme.header must be a mapping".to_string(),
        ));
    };
    const KNOWN: [&str; 7] = [
        "brand",
        "badge",
        "action_label",
        "repo",
        "action_href",
        "theme_toggle",
        "search",
    ];
    for key in mapping.keys() {
        if !matches!(key.as_str(), Some(k) if KNOWN.contains(&k)) {
            warnings.push(format!(
                "Unknown theme.header key {}; ignoring it",
                quoted_value(key)
            ));
        }
    }
    let text = |key: &str| -> Result<Option<String>, ConfigError> {
        Ok(Some(theme_text(
            get(mapping, key),
            &format!("theme.header.{key}"),
        )?)
        .filter(|v| !v.is_empty()))
    };
    let href = |key: &str| -> Result<Option<String>, ConfigError> {
        Ok(Some(theme_href(
            get(mapping, key),
            &format!("theme.header.{key}"),
        )?)
        .filter(|v| !v.is_empty()))
    };
    Ok(ThemeHeader {
        brand: text("brand")?,
        badge: text("badge")?,
        action_label: text("action_label")?,
        repo: href("repo")?,
        action_href: href("action_href")?,
        theme_toggle: theme_bool(get(mapping, "theme_toggle"), "theme.header.theme_toggle")?,
        search: theme_bool(get(mapping, "search"), "theme.header.search")?,
    })
}

fn variant_id(raw: &Value, path: &str) -> Result<String, ConfigError> {
    match raw.as_str() {
        Some(id) if is_variant_id(id) => Ok(id.to_string()),
        _ => Err(ConfigError::field(
            path,
            format!(
                "{path} must start with a letter and contain only letters, numbers, \
                 underscores, or hyphens"
            ),
        )),
    }
}

fn titled(id: &str) -> String {
    slug_label(id)
}

fn variant_option(
    raw: &Value,
    path: &str,
    warnings: &mut Vec<String>,
) -> Result<ThemeVariantOption, ConfigError> {
    let Some(option) = raw.as_mapping() else {
        return Err(ConfigError::field(
            path,
            format!("{path} must be a mapping"),
        ));
    };
    // `preview` reuses the theme.preview rule, so its warnings keep that
    // generic wording rather than the option path.
    Ok(ThemeVariantOption {
        label: theme_text(get(option, "label"), &format!("{path}.label"))?,
        description: theme_text(get(option, "description"), &format!("{path}.description"))?,
        swatch: theme_swatch(get(option, "swatch"), &format!("{path}.swatch"))?,
        preview: theme_preview(get(option, "preview"), warnings),
        style: theme_css_vars(get(option, "style"), &format!("{path}.style"))?,
        tokens: theme_tokens(get(option, "tokens"))?,
    })
}

/// `_theme_variants`: controls with non-empty option maps, defaults resolved,
/// labels titled from ids, and the 256-combination cap.
pub fn theme_variants(
    value: &Value,
    warnings: &mut Vec<String>,
) -> Result<IndexMap<String, ThemeVariantControl>, ConfigError> {
    if is_blank(Some(value)) {
        return Ok(IndexMap::new());
    }
    let Some(mapping) = value.as_mapping() else {
        return Err(ConfigError::field(
            "theme.variants",
            "theme.variants must be a mapping".to_string(),
        ));
    };
    let mut variants = IndexMap::new();
    for (raw_control_id, raw_control) in mapping {
        let control_id = variant_id(raw_control_id, "theme.variants key")?;
        let path = format!("theme.variants.{control_id}");
        let Some(control) = raw_control.as_mapping() else {
            return Err(ConfigError::field(
                &path,
                format!("{path} must be a mapping"),
            ));
        };
        let options_path = format!("{path}.options");
        let raw_options = match get(control, "options").as_mapping() {
            Some(options) if !options.is_empty() => options,
            _ => {
                return Err(ConfigError::field(
                    &options_path,
                    format!("{options_path} must be a non-empty mapping"),
                ))
            }
        };
        let mut options = IndexMap::new();
        for (raw_option_id, raw_option) in raw_options {
            let option_id = variant_id(raw_option_id, &format!("{options_path} key"))?;
            let mut option =
                variant_option(raw_option, &format!("{options_path}.{option_id}"), warnings)?;
            if option.label.is_empty() {
                option.label = titled(&option_id);
            }
            options.insert(option_id, option);
        }
        let mut default = theme_text(get(control, "default"), &format!("{path}.default"))?;
        if default.is_empty() {
            default = options.keys().next().cloned().unwrap_or_default();
        }
        if !options.contains_key(&default) {
            return Err(ConfigError::field(
                &path,
                format!("{path}.default must match an option"),
            ));
        }
        let mut label = theme_text(get(control, "label"), &format!("{path}.label"))?;
        if label.is_empty() {
            label = titled(&control_id);
        }
        let description = theme_text(get(control, "description"), &format!("{path}.description"))?;
        variants.insert(
            control_id,
            ThemeVariantControl {
                label,
                description,
                default,
                options,
            },
        );
    }
    // Saturate rather than wrap: an overflowing product must still trip the cap.
    let combinations = variants
        .values()
        .try_fold(1usize, |acc, c| acc.checked_mul(c.options.len()))
        .unwrap_or(usize::MAX);
    if combinations > THEME_VARIANT_COMBINATION_LIMIT {
        return Err(ConfigError::field(
            "theme.variants",
            format!(
                "theme.variants define {combinations} option combinations across all \
                 controls, which exceeds the limit of {THEME_VARIANT_COMBINATION_LIMIT}. \
                 Every combination is resolved and embedded into each generated page's \
                 HTML, so large products bloat every page. Reduce the number of controls \
                 or options."
            ),
        ));
    }
    Ok(variants)
}

/// `_theme_tune`: aliases mapped onto canonical ids, `radius` skipped
/// (`theme_radius` owns it), unknown keys warn, values non-blank strings.
pub fn theme_tune(
    value: &Value,
    warnings: &mut Vec<String>,
) -> Result<IndexMap<String, String>, ConfigError> {
    if is_blank(Some(value)) {
        return Ok(IndexMap::new());
    }
    let Some(mapping) = value.as_mapping() else {
        return Err(ConfigError::field(
            "theme.tune",
            "theme.tune must be a mapping".to_string(),
        ));
    };
    let mut tune = IndexMap::new();
    for (raw_key, raw_value) in mapping {
        let Some(key) = raw_key.as_str() else {
            return Err(ConfigError::field(
                "theme.tune",
                "theme.tune keys must be strings".to_string(),
            ));
        };
        let canonical = THEME_TUNE_ALIASES
            .iter()
            .find(|(alias, _)| *alias == key)
            .map_or(key, |(_, canonical)| canonical);
        if canonical == "radius" {
            continue;
        }
        if !THEME_TUNE_KEYS.contains(&canonical) {
            let known = THEME_TUNE_KEYS
                .iter()
                .copied()
                .chain(THEME_TUNE_ALIASES.iter().map(|(alias, _)| *alias));
            warnings.push(format!(
                "Unknown theme.tune key {}{}; ignoring it",
                quoted_value(raw_key),
                did_you_mean(key, known)
            ));
            continue;
        }
        match raw_value.as_str().map(str::trim).filter(|v| !v.is_empty()) {
            Some(text) => {
                let options = THEME_TUNE_OPTIONS
                    .iter()
                    .find(|(id, _)| *id == canonical)
                    .map_or(&[][..], |(_, options)| options);
                if !options.contains(&text) {
                    let path = format!("theme.tune.{key}");
                    let listed: Vec<String> =
                        options.iter().map(|option| format!("'{option}'")).collect();
                    return Err(ConfigError::field(
                        &path,
                        format!(
                            "{path} must be one of {}; got '{text}'{}",
                            listed.join(", "),
                            did_you_mean(text, options.iter().copied())
                        ),
                    ));
                }
                tune.insert(canonical.to_string(), text.to_string());
            }
            None => {
                let path = format!("theme.tune.{key}");
                return Err(ConfigError::field(
                    &path,
                    format!("{path} must be a non-empty string"),
                ));
            }
        }
    }
    Ok(tune)
}

/// `_theme_radius`: `""`, a named alias, or one of the fixed scale values.
/// `path` is the key the value came from, `theme.radius` or
/// `theme.tune.radius`.
pub fn theme_radius(value: &Value, path: &str) -> Result<String, ConfigError> {
    let text = theme_string(value);
    let text = text.trim();
    if text.is_empty() {
        return Ok(String::new());
    }
    let lower = text.to_lowercase();
    if let Some((_, target)) = THEME_RADIUS_ALIASES
        .iter()
        .find(|(alias, _)| *alias == lower)
    {
        return Ok(target.to_string());
    }
    if !THEME_RADIUS_OPTIONS.contains(&text) {
        let quote = |items: &[&str]| {
            items
                .iter()
                .map(|item| format!("'{item}'"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let aliases: Vec<&str> = THEME_RADIUS_ALIASES
            .iter()
            .map(|(alias, _)| *alias)
            .collect();
        return Err(ConfigError::field(
            path,
            format!(
                "{path} must be one of {} (or a named alias: {}); got {}{}",
                quote(&THEME_RADIUS_OPTIONS),
                quote(&aliases),
                quoted_value(&Value::String(text.to_string())),
                did_you_mean(
                    text,
                    THEME_RADIUS_OPTIONS
                        .into_iter()
                        .chain(aliases.iter().copied())
                )
            ),
        ));
    }
    Ok(text.to_string())
}

/// Every `theme:` key; `path` is the legacy spelling of `package`.
const THEME_KEYS: [&str; 17] = [
    "dark_mode",
    "extra_presets",
    "preset",
    "name",
    "description",
    "scene",
    "preview",
    "radius",
    "tune",
    "style",
    "tokens",
    "header",
    "variants",
    "package",
    "path",
    "logo",
    "favicon",
];

/// Parse the `theme:` mapping in the documented field order.
pub(crate) fn parse_theme(
    theme: &Mapping,
    warnings: &mut Vec<String>,
) -> Result<ThemeConfig, ConfigError> {
    warn_unknown_keys(theme, &THEME_KEYS, "theme", warnings);
    let dark_mode = bool_field(theme, "dark_mode", "theme.dark_mode", true)?;
    let extra_presets = bool_field(theme, "extra_presets", "theme.extra_presets", false)?;
    let preset = string_field(theme, "preset", "theme.preset", "pastel")?;
    let name = theme_string(get(theme, "name"));
    let description = theme_string(get(theme, "description"));
    let scene = theme_string(get(theme, "scene"));
    let preview = theme_preview(get(theme, "preview"), warnings);
    // `theme.get("radius", tune.get("radius"))`: an explicit `radius: null`
    // wins over the tune alias.
    let (radius_value, radius_path) = if theme.contains_key("radius") {
        (get(theme, "radius"), "theme.radius")
    } else {
        (
            get(theme, "tune")
                .as_mapping()
                .map_or(&NULL, |tune| get(tune, "radius")),
            "theme.tune.radius",
        )
    };
    let radius = theme_radius(radius_value, radius_path)?;
    let tune = theme_tune(get(theme, "tune"), warnings)?;
    let style = theme_css_vars(get(theme, "style"), "theme.style")?;
    let tokens = theme_tokens(get(theme, "tokens"))?;
    let header = theme_header(get(theme, "header"), warnings)?;
    if !dark_mode && header.theme_toggle == Some(true) {
        warnings.push(
            "theme.header.theme_toggle is ignored while theme.dark_mode is false".to_string(),
        );
    }
    let variants = theme_variants(get(theme, "variants"), warnings)?;
    let package = if theme.contains_key("package") {
        get(theme, "package")
    } else {
        get(theme, "path")
    };
    let (package_path, package_remote) = theme_package(package)?;
    let logo = string_field(theme, "logo", "theme.logo", "")?;
    let favicon = string_field(theme, "favicon", "theme.favicon", "")?;
    Ok(ThemeConfig {
        preset,
        dark_mode,
        extra_presets,
        name,
        description,
        scene,
        preview,
        radius,
        tune,
        style,
        tokens,
        header,
        variants,
        package_path,
        package_remote,
        logo,
        favicon,
    })
}

#[cfg(test)]
#[path = "theme_tests.rs"]
mod tests;

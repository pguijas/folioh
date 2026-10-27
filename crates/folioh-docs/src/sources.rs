//! Python-first source orchestration: every configured `source.<language>`
//! root goes through its language's reader, missing roots and languages
//! without one warn, and a module route claimed twice fails before anything
//! is written.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use folioh_config::{language_label, DocsConfig, LANGUAGE_IDS};
use folioh_ir::docstring::{resolve_style, DocstringStyle};
use folioh_ir::ModuleIR;
use folioh_lang_python::{discover, parse_file};
use folioh_mdx::{parse_markdown_directory, MarkdownPage};

use crate::error::DocsError;
use crate::routes::module_route;

/// The modules of every parsed root plus what the sources step reports.
#[derive(Debug, Default, PartialEq)]
pub struct ParsedSources {
    pub modules: Vec<ModuleIR>,
    /// Existing roots, in config order (the verbose `Scanning` lines).
    pub scanned_paths: Vec<PathBuf>,
    /// Configured roots that do not exist, as written in the resolved config.
    pub missing_paths: Vec<String>,
    /// Missing-root warnings per language, then the no-parser warnings.
    pub warnings: Vec<String>,
}

/// The pages of every `source.docs` root plus what the sources step reports.
#[derive(Debug, Default)]
pub struct ParsedDocSources {
    pub docs: Vec<MarkdownPage>,
    pub scanned_paths: Vec<PathBuf>,
    pub missing_paths: Vec<String>,
    /// Missing-root warnings, then the reader's own (the `.rst` notice).
    pub warnings: Vec<String>,
}

/// One file a reader will parse: where it is, what module it publishes, the
/// root it was discovered under and the language that owns it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredFile {
    pub path: PathBuf,
    pub module_name: String,
    pub root: PathBuf,
    pub language: &'static str,
}

/// Whether a reader for `language` is compiled in.
pub fn has_parser(language: &str) -> bool {
    matches!(language, "python" | "javascript" | "rust")
}

/// The suffixes a language's reader reads, with the dot; a language nothing
/// reads has none, and the watcher then owns none of its files.
pub fn file_extensions(language: &str) -> &'static [&'static str] {
    match language {
        "python" => &[".py"],
        "javascript" => &[".js", ".mjs", ".cjs"],
        "rust" => &[".rs"],
        _ => &[],
    }
}

/// The files one language's roots publish, in the order its reader yields
/// them, and what the reader passed over on the way (the JSX notice).
pub fn discover_language(
    language: &'static str,
    roots: &[PathBuf],
    excludes: &[String],
) -> (Vec<DiscoveredFile>, Vec<String>) {
    let file = |path: PathBuf, module_name: String, root: PathBuf| DiscoveredFile {
        path,
        module_name,
        root,
        language,
    };
    match language {
        "javascript" => {
            let found = folioh_lang_javascript::discover(roots, excludes);
            let files = found
                .files
                .into_iter()
                .map(|f| file(f.path, f.module_name, f.root))
                .collect();
            (files, found.warnings)
        }
        "rust" => (
            folioh_lang_rust::discover(roots, excludes)
                .into_iter()
                .map(|f| file(f.path, f.module_name, f.root))
                .collect(),
            Vec::new(),
        ),
        _ => (
            discover(roots, excludes)
                .into_iter()
                .map(|f| file(f.path, f.module_name, f.root))
                .collect(),
            Vec::new(),
        ),
    }
}

/// A language's configured excludes plus the build's own output directory:
/// a root of `.` never reads what the last build wrote.
fn source_excludes(config: &DocsConfig, language: &str) -> Vec<String> {
    let mut excludes = config.language_source(language).excludes.clone();
    if !config.output_dir.trim().is_empty() {
        excludes.push(config.output_dir.clone());
    }
    excludes
}

/// Every enabled language's files, Python first, as the build parses them.
pub fn discover_sources(config: &DocsConfig) -> Vec<DiscoveredFile> {
    enabled_languages(config)
        .into_iter()
        .flat_map(|language| {
            let source = config.language_source(language);
            let roots: Vec<PathBuf> = source.paths.iter().map(PathBuf::from).collect();
            discover_language(language, &roots, &source_excludes(config, language)).0
        })
        .collect()
}

/// One discovered file through its language's reader. A docstring style is a
/// Python concern; Rust and JSDoc comments are Markdown as written.
pub fn parse_discovered(
    file: &DiscoveredFile,
    style: DocstringStyle,
) -> Result<ModuleIR, DocsError> {
    Ok(match file.language {
        "javascript" => folioh_lang_javascript::parse_file(&file.path, &file.root)?,
        "rust" => folioh_lang_rust::parse_file(&file.path, &file.root)?,
        _ => parse_file(&file.path, &file.root, style)?,
    })
}

fn configured<'a>(config: &'a DocsConfig) -> impl Iterator<Item = &'static str> + 'a {
    LANGUAGE_IDS
        .into_iter()
        .filter(|id| !config.language_source(id).paths.is_empty())
}

/// Configured languages that have a parser, Python first.
pub fn enabled_languages(config: &DocsConfig) -> Vec<&'static str> {
    let mut enabled: Vec<&str> = configured(config).filter(|id| has_parser(id)).collect();
    enabled.sort_by_key(|id| *id != "python");
    enabled
}

fn parse_language(config: &DocsConfig, language: &'static str) -> Result<ParsedSources, DocsError> {
    let source = config.language_source(language);
    let label = language_label(language);
    let mut result = ParsedSources::default();
    for root in &source.paths {
        if Path::new(root).exists() {
            result.scanned_paths.push(PathBuf::from(root));
        } else {
            result.missing_paths.push(root.clone());
            result
                .warnings
                .push(format!("{label} source path not found: {root}"));
        }
    }
    let roots: Vec<PathBuf> = source.paths.iter().map(PathBuf::from).collect();
    let style = resolve_style(&config.source.docstring_style);
    let (files, warnings) = discover_language(language, &roots, &source_excludes(config, language));
    for file in files {
        result.modules.push(parse_discovered(&file, style)?);
    }
    result.warnings.extend(warnings);
    Ok(result)
}

/// Two modules on one route (`foo.py` beside `foo/`) fail before any write.
fn reject_duplicate_routes(modules: &[ModuleIR]) -> Result<(), DocsError> {
    let mut owners: HashMap<String, &str> = HashMap::new();
    for module in modules {
        let route = module_route(module);
        if let Some(first) = owners.insert(route.clone(), &module.source_file) {
            return Err(DocsError::RouteCollision {
                route,
                first: first.to_string(),
                second: module.source_file.clone(),
            });
        }
    }
    Ok(())
}

/// The Python roots only (`folioh coverage`): nothing is written, so a
/// duplicate module route is not an error here.
pub fn parse_python_sources(config: &DocsConfig) -> Result<ParsedSources, DocsError> {
    parse_language(config, "python")
}

/// Every enabled language, Python first; every language Folioh recognises has
/// a reader, so a configured root is always read.
pub fn parse_language_sources(config: &DocsConfig) -> Result<ParsedSources, DocsError> {
    let mut result = ParsedSources::default();
    for language in enabled_languages(config) {
        let parsed = parse_language(config, language)?;
        result.modules.extend(parsed.modules);
        result.scanned_paths.extend(parsed.scanned_paths);
        result.missing_paths.extend(parsed.missing_paths);
        result.warnings.extend(parsed.warnings);
    }
    reject_duplicate_routes(&result.modules)?;
    Ok(result)
}

/// Every `source.docs` root through the Markdown reader; missing roots warn.
pub fn parse_doc_sources(config: &DocsConfig) -> Result<ParsedDocSources, DocsError> {
    let mut result = ParsedDocSources::default();
    let mut reader_warnings = Vec::new();
    for dir in &config.source.docs {
        let path = Path::new(dir);
        if !path.exists() {
            result.missing_paths.push(dir.clone());
            result
                .warnings
                .push(format!("Documentation source path not found: {dir}"));
            continue;
        }
        result.scanned_paths.push(path.to_path_buf());
        let scan = parse_markdown_directory(path, "")?;
        result.docs.extend(scan.pages);
        reader_warnings.extend(scan.warnings);
    }
    result.warnings.extend(reader_warnings);
    Ok(result)
}

#[cfg(test)]
#[path = "sources_tests.rs"]
mod tests;

/// Additional literal documentation inputs read by a source file. Missing
/// inputs remain listed so deletion and later creation invalidate the source.
pub fn source_dependencies(path: &Path) -> Vec<PathBuf> {
    if path.extension().is_some_and(|ext| ext == "rs") {
        folioh_lang_rust::doc_dependencies(path)
    } else {
        Vec::new()
    }
}

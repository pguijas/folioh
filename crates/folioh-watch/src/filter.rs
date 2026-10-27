//! Which paths enter a batch and as what: the roots, the ignore rules, the
//! classified `Batch`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use folioh_config::canonicalize_lenient;

use crate::events::{ChangeKind, RawEvent};

/// One enabled language: its parser's extensions (with the dot, `.py`) and
/// the configured roots, Python-first as the build orders them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageRoots {
    pub extensions: Vec<String>,
    pub roots: Vec<PathBuf>,
}

impl LanguageRoots {
    /// Suffix in the extension list and under one of the roots.
    pub fn owns(&self, path: &Path) -> bool {
        let suffix = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{e}"))
            .unwrap_or_default();
        self.extensions.contains(&suffix) && is_under(path, &self.roots)
    }
}

/// What the loop classifies against. Roots should be canonical
/// (`canonicalized`). An event counts when its path as delivered or its
/// canonical form is under a root (FSEvents reports real paths, inotify the
/// link path); the handler receives the path as delivered. Only the
/// generated-dirs test uses the canonical form.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WatchConfig {
    pub languages: Vec<LanguageRoots>,
    /// Exact extra inputs, including absent source documentation files.
    pub source_files: Vec<PathBuf>,
    /// Existing doc roots; any file under one counts (images beside Markdown).
    pub doc_roots: Vec<PathBuf>,
    /// Directories the built-ins asked to watch (a plugin's content dir), existing only.
    pub plugin_roots: Vec<PathBuf>,
    /// `project_dir/docs/examples` when it is a directory.
    pub preview_examples: Option<PathBuf>,
    /// The build dir and the output dir; nothing under them is an input.
    pub generated_dirs: Vec<PathBuf>,
}

/// One set of edits for the handler: the source-change paths (sources,
/// guides and their assets), the plugin-dir changes, the preview path.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Batch {
    /// Sorted, deduplicated; the handler classifies against its own roots.
    pub paths: BTreeSet<PathBuf>,
    /// Paths under a plugin root with their final kind, whatever the suffix.
    pub plugin_changes: BTreeMap<PathBuf, ChangeKind>,
    /// The last changed path under the preview examples dir.
    pub preview_changed: Option<PathBuf>,
}

impl WatchConfig {
    /// Every root resolved leniently (symlinks followed where the prefix exists).
    pub fn canonicalized(mut self) -> Self {
        let fix = |roots: &mut Vec<PathBuf>| {
            for root in roots {
                *root = canonicalize_lenient(root);
            }
        };
        for language in &mut self.languages {
            fix(&mut language.roots);
        }
        fix(&mut self.source_files);
        fix(&mut self.doc_roots);
        fix(&mut self.plugin_roots);
        fix(&mut self.generated_dirs);
        self.preview_examples = self.preview_examples.map(|p| canonicalize_lenient(&p));
        self
    }

    /// Classify one coalesced set. Plugin roots do not exclude the other
    /// classes (an overlapping plugin dir still lets sources through).
    pub fn batch(&self, coalesced: &[RawEvent]) -> Batch {
        let mut batch = Batch::default();
        for event in coalesced {
            let Some(class) = self.classify(&event.path) else {
                continue;
            };
            if class.plugin {
                batch.plugin_changes.insert(event.path.clone(), event.kind);
            }
            if class.preview {
                batch.preview_changed = Some(event.path.clone());
            } else if class.source {
                batch.paths.insert(event.path.clone());
            }
        }
        batch
    }

    /// The OS subscription filter: something watches this path.
    pub fn subscribed(&self, path: &Path) -> bool {
        self.classify(path).is_some()
    }

    fn classify(&self, raw: &Path) -> Option<Class> {
        let canonical = canonicalize_lenient(raw);
        if is_under(&canonical, &self.generated_dirs) || default_ignore(raw) {
            return None;
        }
        let under = |dirs: &[PathBuf]| is_under(raw, dirs) || is_under(&canonical, dirs);
        let class = Class {
            plugin: under(&self.plugin_roots),
            preview: self
                .preview_examples
                .as_ref()
                .is_some_and(|dir| raw.starts_with(dir) || canonical.starts_with(dir)),
            source: self
                .languages
                .iter()
                .any(|language| language.owns(raw) || language.owns(&canonical))
                || under(&self.doc_roots)
                || self
                    .source_files
                    .iter()
                    .any(|path| raw == path || &canonical == path),
        };
        (class.plugin || class.preview || class.source).then_some(class)
    }
}

/// Where one event path lands; a path may be a plugin change and a source
/// change at once.
struct Class {
    plugin: bool,
    preview: bool,
    source: bool,
}

impl Batch {
    /// Nothing to hand to the handler.
    pub fn is_empty(&self) -> bool {
        self.paths.is_empty() && self.plugin_changes.is_empty() && self.preview_changed.is_none()
    }

    /// Fold a later batch into this (pending) one: set unions, the later
    /// plugin kind and preview path win.
    pub fn merge(&mut self, later: Batch) {
        self.paths.extend(later.paths);
        self.plugin_changes.extend(later.plugin_changes);
        if later.preview_changed.is_some() {
            self.preview_changed = later.preview_changed;
        }
    }
}

/// `project_dir/docs/examples` appended when it is a directory and no watched
/// dir already resolves to it.
pub fn with_preview_examples(mut watch_dirs: Vec<PathBuf>, project_dir: &Path) -> Vec<PathBuf> {
    let examples = project_dir.join("docs").join("examples");
    if !examples.is_dir() {
        return watch_dirs;
    }
    let target = canonicalize_lenient(&examples);
    if watch_dirs
        .iter()
        .any(|d| d.exists() && canonicalize_lenient(d) == target)
    {
        return watch_dirs;
    }
    watch_dirs.push(examples);
    watch_dirs
}

/// Directory names `watchfiles.DefaultFilter` skips at any depth.
const IGNORED_DIRS: [&str; 12] = [
    "__pycache__",
    ".git",
    ".hg",
    ".svn",
    ".tox",
    ".venv",
    "site-packages",
    ".idea",
    "node_modules",
    ".mypy_cache",
    ".pytest_cache",
    ".hypothesis",
];

/// Lexical prefix test on path components, no I/O: `/src/mylib/core.py` is
/// under `/src/mylib`, `/p/docs/x` is not under `/p/d`.
pub fn is_under(path: &Path, dirs: &[PathBuf]) -> bool {
    dirs.iter().any(|dir| path.starts_with(dir))
}

/// The `watchfiles.DefaultFilter` rules: an ignored directory component, a
/// dotfile, an editor backup (`~`), `flycheck_` temp files, `.pyc`/`.pyo`.
pub fn default_ignore(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    name.starts_with('.')
        || name.starts_with("flycheck_")
        || name.ends_with('~')
        || matches!(ext, "pyc" | "pyo")
        || path
            .components()
            .any(|c| IGNORED_DIRS.contains(&c.as_os_str().to_str().unwrap_or("")))
}

#[cfg(test)]
#[path = "filter_tests.rs"]
mod tests;

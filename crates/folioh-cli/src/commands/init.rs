//! `folioh init`: project detection (`pyproject.toml`, `Cargo.toml`,
//! `package.json` and the layout on disk), the detected summary, the
//! prompts (docstring style for Python, then visual preset), the generated
//! `docs.yaml`, `docs/index.md`, the workflow files
//! and the ready output.

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anstyle::Style;

use crate::cli::InitArgs;
use crate::error::CliError;
use crate::signals::StopSignals;
use crate::ui::banner::{banner, current_news_item};
use crate::ui::panel::{center_block, Panel};
use crate::ui::select::{ask_choice, can_use_arrow_select, Cancelled, Choice, NewsTicker};
use crate::ui::text::{bold_rgb, rgb, DIM, DIM_YELLOW, YELLOW};
use crate::ui::Ui;
use crate::workflows::github_pages_workflows;

use super::{command_target_suffix, format_cli_path, resolve_path};

/// `docstring_style` when the user keeps the default.
pub const DEFAULT_DOCSTRING_STYLE: &str = "auto";
/// `theme.preset` when the user keeps the default.
pub const DEFAULT_THEME_PRESET: &str = "pastel";

/// The Docstring style prompt, in menu order.
pub const DOCSTRING_STYLES: [Choice; 3] = [
    Choice {
        value: "google",
        label: "Google",
        description: "Google-style Args, Returns, and Raises sections.",
    },
    Choice {
        value: "numpy",
        label: "NumPy",
        description: "NumPy/SciPy-style parameter tables.",
    },
    Choice {
        value: "auto",
        label: "Auto-detect",
        description: "Let Folioh choose the parser.",
    },
];

/// The Visual preset prompt, in menu order.
pub const THEME_PRESETS: [Choice; 5] = [
    Choice {
        value: "organic-editorial",
        label: "Roller",
        description: "Fine headings, generous spacing, and cobalt ink.",
    },
    Choice {
        value: "beacon",
        label: "Beacon",
        description: "Bright product docs with clear contrast.",
    },
    Choice {
        value: "atlas",
        label: "Letterpress",
        description: "Bold serif headings on a paper surface.",
    },
    Choice {
        value: "workshop",
        label: "Notebook",
        description: "Framed pages with warm surfaces and green accents.",
    },
    Choice {
        value: "pastel",
        label: "Folioh Pastel",
        description: "Pen-blue ink, soft colours, and organic shapes.",
    },
];

const DETECTED_BORDER: Style = rgb(0xbf, 0xdb, 0xfe);
const DETECTED_VALUE: Style = rgb(0xf8, 0xfa, 0xfc);
const DOCSTRING_TITLE: Style = bold_rgb(0xa7, 0x8b, 0xfa);
const THEME_TITLE: Style = bold_rgb(0xf4, 0x72, 0xb6);
const SUCCESS: Style = bold_rgb(0x22, 0xc5, 0x5e);
const FILES: Style = bold_rgb(0x60, 0xa5, 0xfa);
const COMMANDS: Style = bold_rgb(0xf5, 0x9e, 0x0b);
const COMMAND: Style = bold_rgb(0x38, 0xbd, 0xf8);
const READY_BORDER: Style = rgb(0xa7, 0x8b, 0xfa);

fn label_style(label: &str) -> Style {
    match label {
        "Target" => bold_rgb(0xfb, 0x71, 0x85),
        "Source" => bold_rgb(0xc0, 0x84, 0xfc),
        "Repo" => bold_rgb(0x38, 0xbd, 0xf8),
        "Python" => bold_rgb(0xbe, 0xf2, 0x64),
        "Framework" => bold_rgb(0xa7, 0x8b, 0xfa),
        "Status" => bold_rgb(0xfd, 0xe6, 0x8a),
        _ => bold_rgb(0xfb, 0xbf, 0x24),
    }
}

/// What init learned about the project and what the user chose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectInfo {
    pub name: String,
    pub version: String,
    pub python_path: Option<String>,
    /// `source.javascript.paths`: a directory holding `.js`/`.mjs`/`.cjs`.
    pub javascript_path: Option<String>,
    /// `source.rust.paths`: the crate's `src/`, or the workspace root.
    pub rust_path: Option<String>,
    /// `project.requires-python` when the manifest states it.
    pub python_version: String,
    pub framework: String,
    pub status: String,
    pub repo: String,
    pub docstring_style: String,
    pub theme_preset: String,
}

impl ProjectInfo {
    /// The values init starts from before reading anything.
    pub fn defaults(name: &str) -> ProjectInfo {
        ProjectInfo {
            name: name.to_string(),
            version: "0.1.0".to_string(),
            python_path: Some("src/".to_string()),
            javascript_path: None,
            rust_path: None,
            python_version: "not detected".to_string(),
            framework: "Python project".to_string(),
            status: String::new(),
            repo: String::new(),
            docstring_style: DEFAULT_DOCSTRING_STYLE.to_string(),
            theme_preset: DEFAULT_THEME_PRESET.to_string(),
        }
    }

    /// The detected source roots as `(language label, path)`, in
    /// `LANGUAGE_IDS` order.
    pub fn sources(&self) -> Vec<(&'static str, &str)> {
        [
            ("Python", &self.python_path),
            ("JavaScript", &self.javascript_path),
            ("Rust", &self.rust_path),
        ]
        .into_iter()
        .filter_map(|(label, path)| path.as_deref().map(|path| (label, path)))
        .collect()
    }
}

fn warn(ui: &Ui, message: &str) {
    ui.eprint_styled(YELLOW, &format!("Warning: {message}"));
}

/// `typer[all]>=0.9 ; python_version>"3.8"` -> `typer`.
pub fn dependency_name(spec: &str) -> String {
    let mut name = spec.split(';').next().unwrap_or("").trim().to_lowercase();
    for separator in ['[', ' ', '<', '>', '=', '!', '~'] {
        if let Some(at) = name.find(separator) {
            name.truncate(at);
        }
    }
    name.replace('_', "-")
}

fn string_items(value: Option<&toml::Value>) -> impl Iterator<Item = &str> {
    value
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(toml::Value::as_str)
}

fn grouped_items(value: Option<&toml::Value>) -> impl Iterator<Item = &str> {
    value
        .and_then(toml::Value::as_table)
        .into_iter()
        .flat_map(|groups| groups.values())
        .flat_map(|group| string_items(Some(group)))
}

/// Every dependency named under `project.dependencies`,
/// `project.optional-dependencies` and `dependency-groups`.
pub fn project_dependency_names(pyproject: &toml::Table) -> BTreeSet<String> {
    let project = pyproject.get("project").and_then(toml::Value::as_table);
    string_items(project.and_then(|p| p.get("dependencies")))
        .chain(grouped_items(
            project.and_then(|p| p.get("optional-dependencies")),
        ))
        .chain(grouped_items(pyproject.get("dependency-groups")))
        .map(dependency_name)
        .collect()
}

/// The framework label from the dependency names, else the package/project fallback.
pub fn detect_framework(pyproject: Option<&toml::Table>) -> &'static str {
    let Some(table) = pyproject else {
        return "Python project";
    };
    let deps = project_dependency_names(table);
    for (dep, framework) in [
        ("typer", "Typer package"),
        ("fastapi", "FastAPI package"),
        ("django", "Django package"),
        ("flask", "Flask package"),
        ("click", "Click package"),
    ] {
        if deps.contains(dep) {
            return framework;
        }
    }
    if table.is_empty() {
        "Python project"
    } else {
        "Python package"
    }
}

/// The Status row: whether `docs.yaml` or `docs/` already exist.
pub fn documentation_status(target: &Path) -> &'static str {
    if target.join("docs.yaml").exists() {
        "Documentation scaffold found"
    } else if target.join("docs").exists() {
        "docs/ found, docs.yaml missing"
    } else {
        "Documentation scaffold not found"
    }
}

/// Name and version read from a manifest; `None` where it states neither.
#[derive(Debug, Default)]
struct Metadata {
    name: Option<String>,
    version: Option<String>,
}

fn read_project_metadata(ui: &Ui, table: &toml::Table, info: &mut ProjectInfo) -> Metadata {
    let mut found = Metadata::default();
    let Some(project) = table.get("project") else {
        return found;
    };
    let Some(project) = project.as_table() else {
        warn(
            ui,
            "Could not read project metadata from pyproject.toml: [project] must be a table",
        );
        return found;
    };
    for (key, slot) in [("name", &mut found.name), ("version", &mut found.version)] {
        match project.get(key) {
            Some(toml::Value::String(value)) if !value.is_empty() => *slot = Some(value.clone()),
            Some(_) => warn(
                ui,
                &format!("Could not read project metadata from pyproject.toml: project.{key} must be a string"),
            ),
            None => {}
        }
    }
    if let Some(toml::Value::String(requires)) = project.get("requires-python") {
        if !requires.trim().is_empty() {
            info.python_version = requires.clone();
        }
    }
    found
}

/// A manifest file parsed, or `None` when it is absent; a file that cannot
/// be read or parsed warns and counts as absent.
fn read_manifest<T>(
    ui: &Ui,
    path: &Path,
    parse: impl FnOnce(&str) -> Result<T, String>,
) -> Option<T> {
    if !path.is_file() {
        return None;
    }
    let file = path.file_name().unwrap_or_default().to_string_lossy();
    match fs::read_to_string(path)
        .map_err(|err| err.to_string())
        .and_then(|text| parse(&text))
    {
        Ok(parsed) => Some(parsed),
        Err(err) => {
            warn(
                ui,
                &format!("Could not read project metadata from {file}: {err}"),
            );
            None
        }
    }
}

fn parse_toml(text: &str) -> Result<toml::Table, String> {
    text.parse::<toml::Table>()
        .map_err(|err| err.message().lines().collect::<Vec<_>>().join(" "))
}

fn toml_string<'a>(table: Option<&'a toml::Value>, key: &str) -> Option<&'a str> {
    table?
        .get(key)?
        .as_str()
        .filter(|value| !value.trim().is_empty())
}

/// Whether `dir` holds a file with one of `extensions`, looking at most a
/// few thousand entries deep and never into `node_modules`, `target` or dot
/// directories.
pub fn contains_source(dir: &Path, extensions: &[&str]) -> bool {
    let mut budget = 5000usize;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            if budget == 0 {
                return false;
            }
            budget -= 1;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                if !name.starts_with('.') && name != "node_modules" && name != "target" {
                    stack.push(path);
                }
            } else if path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| extensions.contains(&ext))
            {
                return true;
            }
        }
    }
    false
}

/// The extensions the JavaScript reader reads.
const JAVASCRIPT_EXTENSIONS: [&str; 3] = ["js", "mjs", "cjs"];

/// The Rust source root a `Cargo.toml` implies: the workspace root for a
/// workspace, else the crate's `src/`; the Framework label; name and version.
fn detect_rust(target: &Path, cargo: &toml::Table) -> (Option<String>, &'static str, Metadata) {
    let package = cargo.get("package");
    let workspace = cargo.get("workspace");
    let metadata = Metadata {
        name: toml_string(package, "name").map(str::to_string),
        version: toml_string(package, "version")
            .or_else(|| toml_string(workspace.and_then(|w| w.get("package")), "version"))
            .map(str::to_string),
    };
    if workspace.is_some() {
        return (Some("./".to_string()), "Rust workspace", metadata);
    }
    let path = if package.is_none() {
        None
    } else if target.join("src/lib.rs").is_file() || target.join("src/main.rs").is_file() {
        Some("src/".to_string())
    } else {
        Some("./".to_string())
    };
    (path, "Rust crate", metadata)
}

/// The JavaScript source root next to a `package.json`: `src/` or `lib/`
/// when it holds JavaScript, else the project root when JavaScript files
/// sit there; name and version from the manifest.
fn detect_javascript(target: &Path, package: &serde_json::Value) -> (Option<String>, Metadata) {
    let text = |key: &str| {
        package
            .get(key)
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string)
    };
    let metadata = Metadata {
        name: text("name"),
        version: text("version"),
    };
    let at_root = || {
        fs::read_dir(target).is_ok_and(|entries| {
            entries.flatten().any(|entry| {
                let path = entry.path();
                path.is_file()
                    && path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .is_some_and(|ext| JAVASCRIPT_EXTENSIONS.contains(&ext))
            })
        })
    };
    let path = ["src", "lib"]
        .into_iter()
        .find(|dir| contains_source(&target.join(dir), &JAVASCRIPT_EXTENSIONS))
        .map(|dir| format!("{dir}/"))
        .or_else(|| at_root().then(|| "./".to_string()));
    (path, metadata)
}

/// Name, version and framework from `pyproject.toml`, `Cargo.toml` and
/// `package.json` (in that order of precedence), the source roots from the
/// directories on disk. Nothing is executed.
pub fn detect_project(ui: &Ui, target: &Path) -> ProjectInfo {
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut info = ProjectInfo::defaults(&name);
    info.status = documentation_status(target).to_string();

    let pyproject = read_manifest(ui, &target.join("pyproject.toml"), parse_toml);
    let cargo = read_manifest(ui, &target.join("Cargo.toml"), parse_toml);
    let package_json = read_manifest(ui, &target.join("package.json"), |text| {
        serde_json::from_str::<serde_json::Value>(text).map_err(|err| err.to_string())
    });

    let mut metadata = Vec::new();
    if let Some(table) = &pyproject {
        metadata.push(read_project_metadata(ui, table, &mut info));
    }
    let mut labels = Vec::new();
    if let Some(cargo) = &cargo {
        let (path, label, found) = detect_rust(target, cargo);
        info.rust_path = path;
        labels.push(label);
        metadata.push(found);
    }
    if let Some(package) = &package_json {
        let (path, found) = detect_javascript(target, package);
        info.javascript_path = path;
        labels.push("JavaScript package");
        metadata.push(found);
    }
    if let Some(name) = metadata.iter().find_map(|m| m.name.clone()) {
        info.name = name;
    }
    if let Some(version) = metadata.iter().find_map(|m| m.version.clone()) {
        info.version = version;
    }

    let pkg = info.name.replace('-', "_");
    let candidates = [
        (target.join("src").join(&pkg), format!("src/{pkg}")),
        (target.join("src"), "src/".to_string()),
        (target.join(&pkg), format!("{pkg}/")),
        (target.join(&info.name), format!("{}/", info.name)),
    ];
    // Next to a Cargo.toml or package.json, `src/` is that language's until
    // a `.py` file says otherwise. When nothing on disk matches, the config
    // must not claim sources that do not exist, or every build warns about
    // init's own scaffolding.
    let other_manifest = cargo.is_some() || package_json.is_some();
    info.python_path = candidates
        .into_iter()
        .find(|(dir, _)| dir.is_dir() && (!other_manifest || contains_source(dir, &["py"])))
        .map(|(_, path)| path);
    if info.python_path.is_some() || pyproject.is_some() {
        labels.insert(0, detect_framework(pyproject.as_ref()));
    }
    info.framework = if labels.is_empty() {
        "not detected".to_string()
    } else {
        labels.join(" + ")
    };
    info
}

/// The GitHub URL of `origin`, or empty when there is none worth writing.
pub fn detect_git_remote(ui: &Ui, target: &Path) -> String {
    match Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(target)
        .output()
    {
        Ok(output) => {
            let mut url = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if url.starts_with("git@github.com:") {
                url = url.replace("git@github.com:", "https://github.com/");
            }
            if let Some(stripped) = url.strip_suffix(".git") {
                url = stripped.to_string();
            }
            if url.starts_with("https://") {
                url
            } else {
                String::new()
            }
        }
        Err(err) => {
            warn(ui, &format!("Could not detect git remote: {err}"));
            String::new()
        }
    }
}

/// A double-quoted YAML scalar (JSON string syntax): detected metadata is
/// repository content and must not be able to close the quote and append
/// keys to the config Folioh then trusts.
pub fn yaml_scalar(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
}

/// The `docs.yaml` text for the detected project and the chosen options.
pub fn generate_docs_yaml(info: &ProjectInfo) -> String {
    let mut lines = vec![
        "# Folioh configuration".to_string(),
        "# Full reference: https://folioh.site/docs/configuration".to_string(),
        String::new(),
        "project:".to_string(),
        format!("  name: {}", yaml_scalar(&info.name)),
        format!("  version: {}", yaml_scalar(&info.version)),
    ];
    if info.repo.is_empty() {
        lines.push("  # repo: \"https://github.com/owner/repo\"".to_string());
    } else {
        lines.push(format!("  repo: {}", yaml_scalar(&info.repo)));
    }
    lines.push(String::new());
    lines.push("source:".to_string());
    let no_sources = info.sources().is_empty();
    match &info.python_path {
        Some(python_path) => {
            lines.push("  python:".to_string());
            lines.push("    paths:".to_string());
            lines.push(format!("      - {}", yaml_scalar(python_path)));
            if info.docstring_style != DEFAULT_DOCSTRING_STYLE {
                lines.push(format!(
                    "    docstring_style: {}",
                    yaml_scalar(&info.docstring_style)
                ));
            }
            lines.push("    # exclude:".to_string());
            lines.push("    #   - \"**/test_*.py\"".to_string());
        }
        None if no_sources => lines.extend(
            [
                "  # No Python sources detected; uncomment to publish an API reference.",
                "  # python:",
                "  #   paths:",
                "  #     - \"src/\"",
            ]
            .map(str::to_string),
        ),
        None => {}
    }
    for (key, path) in [
        ("javascript", &info.javascript_path),
        ("rust", &info.rust_path),
    ] {
        if let Some(path) = path {
            lines.push(format!("  {key}:"));
            lines.push("    paths:".to_string());
            lines.push(format!("      - {}", yaml_scalar(path)));
        }
    }
    lines.extend(
        [
            "  docs:",
            "    - \"docs/\"",
            "",
            "output: \"_site\"",
            "",
            "theme:",
        ]
        .map(str::to_string),
    );
    lines.push(format!("  preset: {}", yaml_scalar(&info.theme_preset)));
    lines.extend(
        [
            "  dark_mode: true",
            "  # logo: \"docs/logo.png\"",
            "  # favicon: \"docs/favicon.ico\"",
        ]
        .map(str::to_string),
    );
    // Only sections the scaffold produces: `docs/index.md` is the docs root,
    // not a nav entry.
    if !no_sources {
        lines.push(String::new());
        lines.push("nav:".to_string());
        lines.push("  - \"API Reference\"".to_string());
    }
    lines.extend(
        [
            "",
            "llm:",
            "  generate_llms_txt: true",
            "  generate_llms_full_txt: true",
            "",
        ]
        .map(str::to_string),
    );
    lines.join("\n")
}

/// The Source row: `src/demo (Python), src/ (Rust)`, or that none was found.
pub fn source_summary(info: &ProjectInfo) -> String {
    let sources = info.sources();
    if sources.is_empty() {
        return "no sources detected".to_string();
    }
    sources
        .iter()
        .map(|(label, path)| format!("{path} ({label})"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn detected_panel(ui: &Ui, info: &ProjectInfo, target: &Path) -> Panel {
    let rows = [
        ("Target", format_cli_path(target)),
        ("Project", format!("{} {}", info.name, info.version)),
        ("Source", source_summary(info)),
        (
            "Repo",
            if info.repo.is_empty() {
                "not detected".to_string()
            } else {
                info.repo.clone()
            },
        ),
        ("Python", info.python_version.clone()),
        ("Framework", info.framework.clone()),
        ("Status", info.status.clone()),
    ];
    let label_width = rows.iter().map(|(label, _)| label.len()).max().unwrap_or(0);
    let body = rows
        .iter()
        .map(|(label, value)| {
            format!(
                "{}{}{}",
                ui.styled(label_style(label), label),
                " ".repeat(label_width - label.len() + 2),
                ui.styled(DETECTED_VALUE, value)
            )
        })
        .collect();
    Panel::new("Detected", DETECTED_BORDER, body)
}

/// Blank line, banner with the news line, blank line, the Detected panel
/// centred. Returns the rows from the news line to the last line printed,
/// which the ticker needs to find the news line again.
fn print_intro(ui: &Ui, info: &ProjectInfo, target: &Path, news_item: &str) -> usize {
    let version = format!("v{}", env!("CARGO_PKG_VERSION"));
    let mut lines = vec![String::new()];
    lines.extend(banner(&version, Some(ui.width), Some(news_item), ui.colors));
    lines.push(String::new());
    let panel = detected_panel(ui, info, target);
    let width = panel.outer_width(ui.width);
    lines.extend(center_block(
        panel.render(ui.width, ui.colors),
        width,
        ui.width,
    ));
    ui.print_lines(&lines);
    let news_index = lines
        .iter()
        .position(|line| line.contains('·'))
        .unwrap_or(lines.len() - 1);
    (lines.len() - news_index).max(1)
}

fn print_ready(ui: &Ui, target: &Path, created: &[String], compact: bool, python: bool) {
    let suffix = command_target_suffix(target);
    if compact {
        ui.blank();
        for path in created {
            ui.print(&format!(
                "  {}",
                ui.styled(SUCCESS, &format!("✔ Created {path}"))
            ));
        }
        ui.print(&format!(
            "  {} {}",
            ui.styled(DIM, "Next"),
            ui.styled(COMMAND, &format!("folioh serve{suffix}"))
        ));
        return;
    }

    let panel_width = ui.width.saturating_sub(2).clamp(44, 96);
    let line_width = panel_width - 4;
    let target_prefix = match format_cli_path(target) {
        path if path.is_empty() || path == "." => String::new(),
        path => format!("{path}/"),
    };
    let mut body = vec![
        ui.styled(SUCCESS, "✓ Documentation project ready."),
        String::new(),
        ui.styled(FILES, "Files"),
    ];
    for path in created {
        let mut display = path.as_str();
        if format!("  Created {display}").chars().count() > line_width
            && !target_prefix.is_empty()
            && path.starts_with(&target_prefix)
        {
            display = &path[target_prefix.len()..];
        }
        body.push(format!("  Created {display}"));
    }
    body.push(String::new());
    body.push(ui.styled(COMMANDS, "Next commands"));
    // `folioh coverage` reads Python only in this release.
    let commands: &[&str] = if python {
        &["serve", "build", "coverage"]
    } else {
        &["serve", "build"]
    };
    for command in commands {
        body.push(format!(
            "  {}",
            ui.styled(COMMAND, &format!("folioh {command}{suffix}"))
        ));
    }
    let panel = Panel {
        title: Some("Ready".to_string()),
        border: READY_BORDER,
        body,
        width: Some(panel_width),
        expand: false,
    };
    ui.blank();
    ui.print_lines(&panel.render(ui.width, ui.colors));
}

fn ask_choices(
    ui: &Ui,
    ticker: Option<&NewsTicker>,
    arrow: bool,
    interrupt: &StopSignals,
    has_python: bool,
) -> Result<(&'static str, &'static str), Cancelled> {
    let style = if has_python {
        ask_choice(
            ui,
            ticker,
            arrow,
            interrupt,
            "Docstring style",
            &DOCSTRING_STYLES,
            DEFAULT_DOCSTRING_STYLE,
            DOCSTRING_TITLE,
        )?
    } else {
        DEFAULT_DOCSTRING_STYLE
    };
    let preset = ask_choice(
        ui,
        ticker,
        arrow,
        interrupt,
        "Visual preset",
        &THEME_PRESETS,
        DEFAULT_THEME_PRESET,
        THEME_TITLE,
    )?;
    Ok((style, preset))
}

/// `folioh init` end to end: detect, show, ask, write, report.
pub fn run(ui: &Ui, args: InitArgs) -> Result<(), CliError> {
    // A missing or old Node/pnpm is a warning here: init writes files, the
    // build is what needs the toolchain.
    if !crate::pipeline::frontend_is_noop() {
        if let Err(err) = folioh_site::runtime::preflight_check() {
            ui.eprint_styled(YELLOW, &err.to_string());
        }
    }
    let requested: PathBuf = args
        .directory
        .clone()
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    if !requested.is_dir() {
        return Err(CliError::Message(format!(
            "Directory not found: {}",
            requested.display()
        )));
    }
    let target = resolve_path(&requested);
    let docs_yaml = target.join("docs.yaml");
    if docs_yaml.exists() {
        ui.print_styled(YELLOW, "docs.yaml already exists, skipping.");
        return Ok(());
    }

    let mut info = detect_project(ui, &target);
    info.repo = detect_git_remote(ui, &target);
    let interrupt = (!args.yes).then(StopSignals::install).transpose()?;
    let arrow = !args.yes && can_use_arrow_select();
    let news_item = current_news_item();
    let rows_below_news = print_intro(ui, &info, &target, news_item);
    let mut ticker =
        arrow.then(|| NewsTicker::start(ui.width, rows_below_news, news_item, ui.colors));

    if !args.yes {
        match &ticker {
            Some(ticker) => {
                let mut state = ticker.lock();
                ui.blank();
                state.add_static_lines(1);
            }
            None => ui.blank(),
        }
        let interrupt = interrupt
            .as_ref()
            .expect("interactive init installs signals");
        let choices = ask_choices(
            ui,
            ticker.as_ref(),
            arrow,
            interrupt,
            info.python_path.is_some(),
        );
        let choices = if interrupt.exit_code().is_some() {
            Err(Cancelled::ByUser)
        } else {
            choices
        };
        match choices {
            Ok((style, preset)) => {
                info.docstring_style = style.to_string();
                info.theme_preset = preset.to_string();
            }
            Err(reason) => {
                if let Some(ticker) = ticker.as_mut() {
                    ticker.stop();
                }
                ui.blank();
                ui.print_styled(
                    DIM_YELLOW,
                    match reason {
                        Cancelled::ByUser
                            if interrupt.exit_code().is_some_and(|code| code != 130) =>
                        {
                            "Init aborted. No files changed."
                        }
                        Cancelled::ByUser => "^C Init aborted. No files changed.",
                        Cancelled::InputClosed => {
                            "Init aborted: stdin ended before an answer. No files changed; \
                             rerun with --yes to take the defaults."
                        }
                    },
                );
                return Err(CliError::Exit(interrupt.exit_code().unwrap_or(1)));
            }
        }
    }
    if let Some(ticker) = ticker.as_mut() {
        ticker.stop();
    }

    let mut created = Vec::new();
    fs::write(&docs_yaml, generate_docs_yaml(&info))?;
    created.push(format_cli_path(&docs_yaml));

    // The docs root needs one page; an existing docs/ with Markdown in it
    // is never touched.
    let docs_dir = target.join("docs");
    let index = docs_dir.join("index.md");
    if !docs_dir.exists() || (docs_dir.is_dir() && !contains_source(&docs_dir, &["md"])) {
        fs::create_dir_all(&docs_dir)?;
        fs::write(
            &index,
            format!("# {}\n\nWelcome to the documentation.\n", info.name),
        )?;
        created.push(format_cli_path(&index));
    }

    for (relative, content) in github_pages_workflows() {
        let path = target.join(relative);
        if path.exists() {
            continue;
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, content)?;
        created.push(format_cli_path(&path));
    }

    print_ready(ui, &target, &created, !args.yes, info.python_path.is_some());
    Ok(())
}

#[cfg(test)]
#[path = "init_tests.rs"]
mod tests;

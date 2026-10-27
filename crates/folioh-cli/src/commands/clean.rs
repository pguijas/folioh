//! `folioh clean`: removes `.build/` and the output directory, read
//! best-effort from the raw `docs.yaml` so cleanup works when the rest of
//! the config is broken; the containment
//! rules are `folioh_config::resolve_output_dir`'s, fed the same source roots
//! the build refuses to delete.

use std::fs;
use std::path::{Path, PathBuf};

use serde_yaml_ng::Value;

use super::format_cli_path;
use crate::cli::CleanArgs;
use crate::error::CliError;
use crate::ui::text::{GREEN, YELLOW};
use crate::ui::Ui;
use crate::workspace_lock::WorkspaceLease;

const DEFAULT_OUTPUT: &str = "_site";

/// `resolve_output_dir` over the raw `output:` string and the raw source
/// roots.
fn resolve_output_dir(target: &Path, raw: &str, sources: &[String]) -> Result<PathBuf, String> {
    let sources: Vec<&str> = sources.iter().map(String::as_str).collect();
    folioh_config::resolve_output_dir(target, raw, &sources).map_err(|err| err.to_string())
}

fn strings(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Sequence(items)) => items
            .iter()
            .filter_map(|item| item.as_str().map(str::to_string))
            .collect(),
        _ => Vec::new(),
    }
}

/// Every source root the raw config names, read as leniently as the file
/// allows: `source.docs`, `source.python` (a list or `.paths`) and the other
/// languages' `.paths`.
fn raw_source_roots(raw: &Value) -> Vec<String> {
    let Some(source) = raw.get("source") else {
        return Vec::new();
    };
    let mut roots = strings(source.get("docs"));
    for language in folioh_config::LANGUAGE_IDS {
        match source.get(language) {
            Some(list @ Value::Sequence(_)) => roots.extend(strings(Some(list))),
            Some(section) => roots.extend(strings(section.get("paths"))),
            None => {}
        }
    }
    roots
}

/// The output directory to remove, or `None` when even the `_site` default
/// would delete a source root.
fn read_output_dir_best_effort(ui: &Ui, config_path: &Path, target: &Path) -> Option<PathBuf> {
    let shown = format_cli_path(config_path);
    let raw = if config_path.exists() {
        let parsed = fs::read_to_string(config_path)
            .map_err(|err| err.to_string())
            .and_then(|text| {
                serde_yaml_ng::from_str::<Value>(&text).map_err(|err| err.to_string())
            });
        match parsed {
            Ok(raw) => raw,
            Err(err) => {
                ui.print_styled(
                    YELLOW,
                    &format!("Warning: Could not read output from {shown}: {err}"),
                );
                Value::Null
            }
        }
    } else {
        Value::Null
    };
    let sources = raw_source_roots(&raw);
    let output = raw
        .get("output")
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_OUTPUT);
    match resolve_output_dir(target, output, &sources) {
        Ok(path) => return Some(path),
        Err(message) => ui.print_styled(
            YELLOW,
            &format!("Warning: Ignoring unsafe output in {shown}: {message}"),
        ),
    }
    if output == DEFAULT_OUTPUT {
        return None;
    }
    match resolve_output_dir(target, DEFAULT_OUTPUT, &sources) {
        Ok(path) => Some(path),
        Err(message) => {
            ui.print_styled(
                YELLOW,
                &format!("Warning: Skipping {DEFAULT_OUTPUT}: {message}"),
            );
            None
        }
    }
}

/// `folioh clean`: remove `.build/` and the output directory.
pub fn run(ui: &Ui, args: CleanArgs) -> Result<(), CliError> {
    let target = args.project.resolve()?;
    // A mistyped directory is an error, not "Nothing to clean."
    if !target.is_dir() {
        return Err(CliError::Message(format!(
            "Directory not found: {}",
            format_cli_path(&target)
        )));
    }
    let build_dir = target.join(".build");
    let lease = WorkspaceLease::acquire(&build_dir, "clean", None).map_err(CliError::message)?;
    if let Some(message) = &lease.recovered {
        ui.print_styled(YELLOW, &format!("warning: {message}"));
    }
    let output_dir = read_output_dir_best_effort(ui, &target.join(&args.config), &target);

    let dirs: Vec<PathBuf> = [Some(build_dir), output_dir]
        .into_iter()
        .flatten()
        .filter(|dir| dir.symlink_metadata().is_ok())
        .collect();
    let shown = |dir: &Path| {
        let relative = dir.strip_prefix(&target).unwrap_or(dir);
        relative.to_string_lossy().replace('\\', "/")
    };
    // Checked before anything is removed, so a refusal deletes nothing.
    if let Some(file) = dirs.iter().find(|dir| !dir.is_dir()) {
        return Err(CliError::Message(format!(
            "{} is not a directory; folioh clean only removes directories. Check output in {}.",
            shown(file),
            format_cli_path(&target.join(&args.config))
        )));
    }
    let mut removed = Vec::new();
    for dir in dirs {
        // An output inside `.build/` went with it.
        if dir.symlink_metadata().is_err() {
            continue;
        }
        fs::remove_dir_all(&dir)?;
        removed.push(shown(&dir));
    }
    if removed.is_empty() {
        ui.print_styled(YELLOW, "Nothing to clean.");
    } else {
        ui.print_styled(GREEN, &format!("Cleaned: {}", removed.join(", ")));
    }
    Ok(())
}

#[cfg(test)]
#[path = "clean_tests.rs"]
mod tests;

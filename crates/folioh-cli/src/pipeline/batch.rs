//! The source batch behind `folioh serve`: three hash passes over the known
//! set, re-parse of only the pending source files against the retained IR,
//! the guides re-read in full, then the build's own page generation,
//! finalize, LLM files and manifest save; the cache commits only after the
//! save.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use folioh_config::canonicalize_lenient;
use folioh_config::trace::Tracer;
use folioh_docs::{
    discover_sources, enabled_languages, file_extensions, parse_discovered, parse_doc_sources,
    DiscoveredFile,
};
use folioh_ir::docstring::resolve_style;
use folioh_ir::ModuleIR;
use folioh_plugins::check_route_collisions;
use folioh_site::builder::Manifest;
use folioh_site::fs::files_under;
use folioh_site::sidebar::directory_order_files;
use folioh_site::SiteError;
use folioh_watch::{BatchHandler, BatchSummary, ChangeKind, LanguageRoots, RetainedCache};

use super::finalize::{finalize_generated_files, write_llm_outputs};
use super::pages::{generate_content_pages, GenerateArgs};
use super::source_hash::source_hash as hash_file;
use super::{run_build, BuildOptions, Report, Site};

/// The batch handler `folioh serve` hands to the watcher loop.
pub struct SourceBatchHandler<'a, 'c> {
    pub site: &'a mut Site<'c>,
    /// Parsed modules keyed by source path with the hash they came from.
    pub cache: RetainedCache<ModuleIR>,
    pub report: Report<'a>,
    pub tracer: Option<Arc<Tracer>>,
}

fn text(err: impl ToString) -> String {
    err.to_string()
}

fn key(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// Every `*.md` under the doc roots, recursively.
fn markdown_files(doc_roots: &[String]) -> impl Iterator<Item = PathBuf> + '_ {
    doc_roots.iter().flat_map(|root| {
        files_under(Path::new(root))
            .into_iter()
            .filter(|p| p.extension().is_some_and(|e| e == "md"))
    })
}

/// The first key whose bytes changed since `hashes` recorded them.
pub fn changed_since<'k>(
    hashes: &HashMap<String, String>,
    keys: impl IntoIterator<Item = &'k String>,
) -> Option<&'k String> {
    keys.into_iter().find(|key| {
        hashes
            .get(*key)
            .is_some_and(|seen| *seen != hash_file(Path::new(key)))
    })
}

/// The retained cache of the initial build: every module with the hash
/// the manifest recorded for its source, not the file's hash now, so an edit
/// that landed between the Sources step and the watcher start is stale on
/// the first batch instead of being republished from old IR.
pub fn seed_cache(modules: &[ModuleIR], manifest: &Manifest) -> RetainedCache<ModuleIR> {
    let mut cache = RetainedCache::default();
    for module in modules {
        let recorded = manifest
            .sources
            .get(&module.source_file)
            .and_then(|entry| entry.get("hash"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        cache.upsert(PathBuf::from(&module.source_file), recorded, module.clone());
    }
    cache
}

impl<'a, 'c> SourceBatchHandler<'a, 'c> {
    /// Seed the retained cache from the manifest the build just saved.
    pub fn new(site: &'a mut Site<'c>, report: Report<'a>, tracer: Option<Arc<Tracer>>) -> Self {
        let manifest = site.builder.load_manifest().unwrap_or_default();
        let cache = seed_cache(&site.sources.modules, &manifest);
        SourceBatchHandler {
            site,
            cache,
            report,
            tracer,
        }
    }
}

impl BatchHandler for SourceBatchHandler<'_, '_> {
    fn plugin_dirs_changed(&mut self, changes: &[(PathBuf, ChangeKind)]) -> Result<bool, String> {
        let site = &mut *self.site;
        let mut handled = false;
        let mut diag = Vec::new();
        for (path, kind) in changes {
            let kind = match kind {
                ChangeKind::Added => folioh_plugins::ChangeKind::Added,
                ChangeKind::Modified => folioh_plugins::ChangeKind::Modified,
                ChangeKind::Deleted => folioh_plugins::ChangeKind::Deleted,
            };
            handled |=
                site.host
                    .dispatch_change(&mut site.builder, site.config, path, kind, &mut diag);
        }
        for warning in &diag {
            self.report.warning(warning);
        }
        Ok(handled)
    }

    fn refresh(
        &mut self,
        paths: &BTreeSet<PathBuf>,
        refresh_extensions: bool,
    ) -> Result<BatchSummary, String> {
        let tracer = self.tracer.clone();
        let trace = |event: &str| {
            if let Some(tracer) = &tracer {
                tracer.trace(event, &[]);
            }
        };
        let site = &mut *self.site;
        let config = site.config;
        let previous = site.builder.load_manifest().map_err(text)?;

        // The pending set is the owned event paths plus every retained
        // module whose bytes moved, whose file discovery no longer yields or
        // whose module name changed (an `__init__.py` appeared or vanished).
        let owners: Vec<LanguageRoots> = enabled_languages(config)
            .into_iter()
            .map(|language| LanguageRoots {
                extensions: file_extensions(language)
                    .iter()
                    .map(|suffix| suffix.to_string())
                    .collect(),
                roots: config
                    .language_source(language)
                    .paths
                    .iter()
                    .map(PathBuf::from)
                    .collect(),
            })
            .collect();
        let discovered: Vec<DiscoveredFile> = discover_sources(config);
        let order: HashMap<PathBuf, usize> = discovered
            .iter()
            .enumerate()
            .map(|(i, file)| (canonicalize_lenient(&file.path), i))
            .collect();
        let lookup = |path: &Path| {
            order
                .get(&canonicalize_lenient(path))
                .map(|i| &discovered[*i])
        };
        let mut pending: BTreeSet<PathBuf> = paths
            .iter()
            .filter(|path| owners.iter().any(|owner| owner.owns(path)))
            .cloned()
            .collect();
        for path in self.cache.paths() {
            let retained = &self.cache.get(path).expect("retained entry").payload.name;
            if lookup(path).is_none_or(|file| file.module_name != *retained) {
                pending.insert(path.to_path_buf());
            }
        }
        pending.extend(self.cache.stale(hash_file));

        let mut known: BTreeSet<PathBuf> = self.cache.paths().map(Path::to_path_buf).collect();
        known.extend(pending.iter().cloned());
        known.extend(markdown_files(&config.source.docs));
        known.extend(directory_order_files(&config.source.docs).map_err(text)?);
        known.extend(previous.sources.keys().map(PathBuf::from));
        known.extend(paths.iter().cloned());
        let mut hashes: HashMap<String, String> = known
            .iter()
            .map(|path| (key(path), hash_file(path)))
            .collect();

        // Re-parse only what is pending, into a copy of the cache.
        let style = resolve_style(&config.source.docstring_style);
        let mut staged = self.cache.clone();
        for path in &pending {
            match lookup(path) {
                Some(file) if path.is_file() => {
                    let module = parse_discovered(file, style).map_err(text)?;
                    let hash = hashes
                        .get(&key(&file.path))
                        .cloned()
                        .unwrap_or_else(|| hash_file(&file.path));
                    staged.upsert(file.path.clone(), hash, module);
                }
                _ => staged.remove(path),
            }
        }
        let mut modules: Vec<ModuleIR> = staged.payloads().cloned().collect();
        modules.sort_by_key(|m| {
            (
                order
                    .get(&canonicalize_lenient(Path::new(&m.source_file)))
                    .copied()
                    .unwrap_or(usize::MAX),
                m.source_file.clone(),
            )
        });

        // Guides and collected docs in full, then hash pass 2.
        let parsed_docs = parse_doc_sources(config).map_err(text)?;
        let mut docs = parsed_docs.docs;
        docs.extend(site.host.collect_docs(config).map_err(text)?);
        check_route_collisions(&docs).map_err(text)?;
        let warnings = parsed_docs.warnings;
        let inputs: Vec<String> = modules
            .iter()
            .map(|m| m.source_file.clone())
            .chain(docs.iter().map(|d| d.source_file.clone()))
            .chain(
                known
                    .iter()
                    .filter(|p| p.file_name().is_some_and(|n| n == "_meta.yaml"))
                    .map(|p| key(p)),
            )
            .filter(|k| !k.is_empty())
            .collect();
        if let Some(changed) = changed_since(&hashes, &inputs) {
            return Err(format!(
                "Source changed during parsing; retry on next save: {changed}"
            ));
        }
        for input in &inputs {
            hashes.insert(input.clone(), hash_file(Path::new(input)));
        }

        // The build's own generation with the previous context, so only
        // hash and symbol changes drive regeneration.
        let build_context = previous.build.clone().unwrap_or_default();
        let generation = generate_content_pages(
            GenerateArgs {
                builder: &mut site.builder,
                config,
                modules: &modules,
                docs: &docs,
                project_dir: site.project_dir,
                build_context: &build_context,
                clean: false,
                verbose: site.verbose,
                prev_manifest: Some(&previous),
                source_hashes: Some(&hashes),
            },
            &self.report,
        )
        .map_err(text)?;
        if refresh_extensions {
            trace("finalize_apply_extensions_start");
        }
        finalize_generated_files(
            &mut site.builder,
            site.host,
            config,
            site.project_dir,
            &generation.source_routes(),
            refresh_extensions.then_some(site.registry),
            false,
            &self.report,
            &mut |_| Ok(()),
        )
        .map_err(text)?;
        if refresh_extensions {
            trace("finalize_apply_extensions_end");
        }
        write_llm_outputs(&site.builder, config, site.project_dir, &modules, &docs)
            .map_err(text)?;
        if let Some(changed) = changed_since(&hashes, &inputs) {
            return Err(format!(
                "Source changed during generation; retry on next save: {changed}"
            ));
        }
        site.builder
            .save_manifest(&Manifest {
                build: Some(generation.build_context),
                sources: generation.sources,
            })
            .map_err(text)?;
        self.cache = staged;
        Ok(BatchSummary {
            pages: generation.total_pages,
            skipped: generation.skipped,
            warnings,
        })
    }

    fn refresh_previews(&mut self, _path: &Path) -> Result<(), String> {
        let site = &mut *self.site;
        let ui = self.report.ui();
        let tracer = self.tracer.clone();
        let examples = site.project_dir.join("docs").join("examples");
        site.builder
            .write_preview_examples(&examples, &mut |request| {
                let nested = BuildOptions {
                    plugins: site.plugins.to_vec(),
                    quiet: true,
                    output_override: Some(request.output_dir),
                    build_dir_override: Some(request.build_dir),
                    ..BuildOptions::new("docs.yaml")
                };
                run_build(&request.project_dir, &nested, ui, tracer.clone())
                    .map_err(|e| SiteError::Value(e.to_string()))
            })
            .map(|_| ())
            .map_err(text)
    }
}

#[cfg(test)]
#[path = "batch_tests.rs"]
mod tests;

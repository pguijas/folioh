//! The build pipeline behind `folioh build` and the build half of `folioh
//! serve`: preflight, config and the
//! plugin host, banner, sources, template and manifest context, incremental
//! pages, finalize, links, dependencies, export or LLM files, manifest,
//! `Done`. Every row goes through `ui`; the engine crates only return data.

pub mod batch;
pub mod finalize;
pub mod pages;
pub mod preview;
mod source_hash;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use anstyle::Style;
use folioh_config::{
    canonicalize_lenient, load_docs_config_with_keys, ConfigError, DocsConfig, Loaded, Tracer,
};
use folioh_docs::{parse_doc_sources, parse_language_sources, DocsError};
use folioh_ir::ModuleIR;
use folioh_mdx::MarkdownPage;
use folioh_plugins::{
    build_registry, check_route_collisions, ExtensionRegistry, PluginHost, PluginsError,
};
use folioh_site::builder::{Manifest, PublicFile, SiteBuilder};
use folioh_site::runtime::{preflight_check, FrontendRuntime, NextRuntime, NoopRuntime};
use folioh_site::template::{
    build_manifest_context, docs_route_base, generator_fingerprint, prune_stale_build_overlay,
    resolve_template_dir, theme_package_signature,
};
use folioh_site::SiteError;

use crate::error::CliError;
use crate::ui::banner::banner;
use crate::ui::panel::print_build_output;
use crate::ui::steps::{
    count_phrase, step, step_detail, Spinner, BOLD_GREEN, BOLD_MAGENTA, BOLD_YELLOW, MAGENTA,
};
use crate::ui::text::{BOLD, DIM, GREEN, YELLOW};
use crate::ui::Ui;
use crate::workspace_lock::WorkspaceLease;
use crate::PluginFactory;

use pages::GenerateArgs;

/// Test-only seam: `FOLIOH_FRONTEND_RUNTIME=noop` swaps the pnpm/next runtime
/// for one that installs, builds and serves nothing, so the pipeline runs
/// without Node. Not a user switch.
const RUNTIME_ENV: &str = "FOLIOH_FRONTEND_RUNTIME";

/// True when the test seam selects the no-op frontend runtime; `init` skips
/// the Node/pnpm preflight warning then, since nothing would run them.
pub fn frontend_is_noop() -> bool {
    std::env::var(RUNTIME_ENV).is_ok_and(|v| v == "noop")
}

/// The flags `folioh build`, `folioh serve`, `build-versions` and the nested
/// preview-example build pass in.
#[derive(Debug, Clone, Default)]
pub struct BuildOptions {
    /// Additional plugins supplied by the distribution.
    pub plugins: Vec<PluginFactory>,
    /// `folioh serve`: LLM files go to `public/`, no export, `ready in` row.
    pub serve: bool,
    /// The live server's port, included in workspace-occupancy diagnostics.
    pub serve_port: Option<u16>,
    pub verbose: bool,
    /// `--config`; `docs.yaml` by default.
    pub config_file: String,
    pub clean: bool,
    /// Build the example projects under `docs/examples/`, a nested site each:
    /// `folioh build` always, `folioh serve` only with `--previews`.
    pub previews: bool,
    /// Output dir override (relative to the project dir), as the nested example build passes it.
    pub output_override: Option<PathBuf>,
    /// `.build/` override for the nested example build.
    pub build_dir_override: Option<PathBuf>,
    /// `build-versions`: the version being built.
    pub current_version_path: String,
    /// `build-versions`: keep the configured version list.
    pub include_versions: bool,
    /// `build-versions`: the ref source links point at.
    pub source_ref_override: String,
    /// The nested example build: nothing reaches the terminal.
    pub quiet: bool,
}

impl BuildOptions {
    /// Defaults for `folioh build` with the given config file name.
    pub fn new(config_file: &str) -> BuildOptions {
        BuildOptions {
            config_file: config_file.to_string(),
            ..BuildOptions::default()
        }
    }
}

pub(crate) fn plugin_host(plugins: &[PluginFactory]) -> PluginHost {
    let mut host = PluginHost::builtin();
    for plugin in plugins {
        host.push(plugin());
    }
    host
}

pub(crate) fn load_config(
    path: &Path,
    project_dir: &Path,
    host: &PluginHost,
) -> Result<Loaded<DocsConfig>, ConfigError> {
    let keys = host.config_keys();
    let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
    load_docs_config_with_keys(path, project_dir, &keys)
}

/// What stops a build. `ConfigMissing` renders as `Error:`, everything else
/// as `Build failed:`.
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("{0}")]
    ConfigMissing(String),
    #[error("interrupted (exit {0})")]
    Interrupted(u8),
    #[error("{0}")]
    Failed(String),
}

impl From<BuildError> for CliError {
    fn from(err: BuildError) -> CliError {
        match err {
            BuildError::ConfigMissing(text) => CliError::Message(text),
            BuildError::Interrupted(code) => CliError::Exit(code),
            BuildError::Failed(text) => CliError::Build(text),
        }
    }
}

macro_rules! failed_from {
    ($($ty:ty),*) => {$(
        impl From<$ty> for BuildError {
            fn from(err: $ty) -> BuildError {
                BuildError::Failed(err.to_string())
            }
        }
    )*};
}
failed_from!(
    SiteError,
    ConfigError,
    PluginsError,
    DocsError,
    std::io::Error
);

/// The step printer: every row of the transcript, silent under `quiet`.
pub struct Report<'a> {
    ui: &'a Ui,
    quiet: bool,
    /// Transient spinners; off for a watcher batch, which shares the
    /// terminal with the dev server's output.
    spinners: bool,
}

impl<'a> Report<'a> {
    /// A reporter over the terminal; `quiet` swallows everything.
    pub fn new(ui: &'a Ui, quiet: bool) -> Report<'a> {
        Report {
            ui,
            quiet,
            spinners: true,
        }
    }

    /// The reporter of a watcher batch: rows and warnings, no spinners.
    pub fn for_batches(ui: &'a Ui) -> Report<'a> {
        Report {
            ui,
            quiet: false,
            spinners: false,
        }
    }

    /// The terminal behind this reporter.
    pub fn ui(&self) -> &'a Ui {
        self.ui
    }

    /// `✓ Label › detail`.
    pub fn step_ok(&self, label: &str, detail: &str) {
        self.step(label, detail, "✓", GREEN, BOLD);
    }

    /// A step row with explicit marker and label styles.
    pub fn step(
        &self,
        label: &str,
        detail: &str,
        marker: &str,
        marker_style: Style,
        label_style: Style,
    ) {
        if !self.quiet {
            step(self.ui, label, detail, marker, marker_style, label_style);
        }
    }

    /// `  detail`, dim or yellow.
    pub fn detail(&self, text: &str, style: Style) {
        if !self.quiet {
            step_detail(self.ui, text, style);
        }
    }

    /// A free line.
    pub fn line(&self, text: &str) {
        if !self.quiet {
            self.ui.print(text);
        }
    }

    /// A styled free line.
    pub fn styled(&self, style: Style, text: &str) {
        if !self.quiet {
            self.ui.print_styled(style, text);
        }
    }

    /// `warning: {text}` in yellow, the shape of every engine warning.
    pub fn warning(&self, text: &str) {
        self.styled(YELLOW, &format!("warning: {text}"));
    }

    /// The transient spinner row; `None` under `quiet` or off a terminal.
    pub fn spinner(&self, label: &str, detail: &str, total: Option<usize>) -> Option<Spinner> {
        (!self.quiet && self.spinners).then(|| Spinner::start(self.ui, label, detail, total))
    }
}

/// Everything parsed for one build.
pub struct BuildSources {
    pub modules: Vec<ModuleIR>,
    pub docs: Vec<MarkdownPage>,
    /// Language warnings, missing-docs warnings, then reader warnings.
    pub warnings: Vec<String>,
    /// The `--verbose` `Scanning` lines: source roots, then docs roots.
    pub scanned: Vec<PathBuf>,
}

/// Parse every enabled language, the Markdown guides and the documents the
/// built-ins contribute; reject duplicate public routes.
pub fn parse_project_sources(
    config: &DocsConfig,
    host: &PluginHost,
) -> Result<BuildSources, BuildError> {
    let parsed = parse_language_sources(config)?;
    let parsed_docs = parse_doc_sources(config)?;
    let mut docs = parsed_docs.docs;
    docs.extend(host.collect_docs(config)?);
    check_route_collisions(&docs)?;
    let mut warnings = parsed.warnings;
    warnings.extend(parsed_docs.warnings);
    let mut scanned = parsed.scanned_paths;
    scanned.extend(parsed_docs.scanned_paths);
    Ok(BuildSources {
        modules: parsed.modules,
        docs,
        warnings,
        scanned,
    })
}

/// The prepared site a batch or the dev server works on after `Done`.
pub struct Site<'a> {
    pub builder: SiteBuilder<'a>,
    /// The resolved config.
    pub config: &'a DocsConfig,
    pub host: &'a PluginHost,
    pub plugins: &'a [PluginFactory],
    pub registry: &'a ExtensionRegistry,
    pub project_dir: &'a Path,
    pub sources: BuildSources,
    pub verbose: bool,
}

fn frontend_runtime() -> (Box<dyn FrontendRuntime>, bool) {
    if std::env::var(RUNTIME_ENV).is_ok_and(|v| v == "noop") {
        (Box::new(NoopRuntime), true)
    } else {
        (Box::new(NextRuntime::default()), false)
    }
}

/// The empty-project gate, naming the config file the source paths are in.
fn empty_project_message(config_file: &str) -> String {
    format!("No source modules or documentation found. Check the source paths in {config_file}.")
}

/// The `FOLIOH_TRACE` tracer, created once per command and shared with the
/// builder and the watcher; a path that cannot be opened prints its warning
/// and switches tracing off.
pub fn tracer_from_env(ui: &Ui) -> Option<Arc<Tracer>> {
    match Tracer::from_env() {
        Ok(tracer) => tracer.map(Arc::new),
        Err(warning) => {
            ui.eprint_styled(YELLOW, &warning);
            None
        }
    }
}

/// `folioh build`: the whole pipeline through `Site ready`.
pub fn run_build(
    project_dir: &Path,
    opts: &BuildOptions,
    ui: &Ui,
    tracer: Option<Arc<Tracer>>,
) -> Result<(), BuildError> {
    build_site(project_dir, opts, ui, tracer, &mut |_, _| Ok(()))
}

/// The pipeline through `Done`; with `opts.serve` the built site is then
/// handed to `on_ready` (the dev server and the watcher) instead of exported.
pub fn build_site(
    project_dir: &Path,
    opts: &BuildOptions,
    ui: &Ui,
    tracer: Option<Arc<Tracer>>,
    on_ready: &mut dyn FnMut(&mut Site<'_>, &Report) -> Result<(), BuildError>,
) -> Result<(), BuildError> {
    let (runtime, noop) = frontend_runtime();
    let report = Report::new(ui, opts.quiet);
    let config_path = project_dir.join(&opts.config_file);
    // The missing config comes first: on a machine without Node, a project
    // that is not a Folioh project should say so, not report the toolchain.
    if !config_path.exists() {
        return Err(BuildError::ConfigMissing(format!(
            "Config file not found: {}",
            config_path.display()
        )));
    }
    let build_dir = opts
        .build_dir_override
        .clone()
        .unwrap_or_else(|| project_dir.join(".build"));
    // Keep the lease alive through export or the entire on_ready serve loop.
    // Claim before any plugin hook or template preparation can mutate it.
    let lease = WorkspaceLease::acquire(
        &build_dir,
        if opts.serve { "serve" } else { "build" },
        opts.serve_port,
    )
    .map_err(BuildError::Failed)?;
    if let Some(message) = &lease.recovered {
        report.warning(message);
    }
    if noop {
        // The seam writes no site; a green transcript over an empty output
        // directory must never read as a real build.
        ui.eprint_styled(
            YELLOW,
            &format!("warning: {RUNTIME_ENV}=noop, the frontend never runs and no site is written"),
        );
    } else {
        preflight_check()?;
    }
    let host = plugin_host(&opts.plugins);
    let loaded = load_config(&config_path, project_dir, &host)?;
    let mut config = loaded.config;
    let mut warnings = loaded.warnings;
    host.configure(&mut config, &loaded.raw, &mut warnings)?;
    let source_ref = opts.source_ref_override.trim();
    if !source_ref.is_empty() {
        config.project.repo_ref = source_ref.to_string();
    }
    let mut version_note = "";
    if !opts.include_versions {
        if !config.versions.is_empty() && !opts.serve {
            version_note = "Current version only; use 'folioh build-versions' for all versions.";
        }
        config.versions.clear();
    }
    let mut resolved = config.resolve_paths(project_dir)?;
    let output_display = match &opts.output_override {
        Some(override_dir) => {
            resolved.output_dir = canonicalize_lenient(&project_dir.join(override_dir))
                .to_string_lossy()
                .into_owned();
            override_dir.to_string_lossy().into_owned()
        }
        None => config.output_dir.clone(),
    };
    let t0 = Instant::now();

    if !opts.quiet {
        ui.blank();
        ui.print_lines(&banner(
            &format!("v{}", env!("CARGO_PKG_VERSION")),
            Some(ui.width),
            None,
            ui.colors,
        ));
        ui.blank();
    }
    let registry = build_registry(&host, &resolved, &mut warnings)?;
    for warning in &warnings {
        report.warning(warning);
    }

    let spinner = report.spinner("Sources", "scanning sources and docs", None);
    let sources = parse_project_sources(&resolved, &host)?;
    drop(spinner);
    if opts.verbose {
        for root in &sources.scanned {
            report.line(&format!("  Scanning {}", root.display()));
        }
    }
    if sources.modules.is_empty() && sources.docs.is_empty() && registry.views.is_empty() {
        for warning in &sources.warnings {
            report.detail(warning, YELLOW);
        }
        // Said once, by the `Build failed:` line the error becomes.
        return Err(BuildError::Failed(empty_project_message(&opts.config_file)));
    }
    report.step_ok(
        "Sources",
        &format!(
            "{}, {}",
            count_phrase(sources.modules.len(), "module", None),
            count_phrase(sources.docs.len(), "doc page", None)
        ),
    );
    for warning in &sources.warnings {
        report.detail(warning, YELLOW);
    }
    if !version_note.is_empty() {
        report.detail(version_note, DIM);
    }

    let template = resolve_template_dir(&resolved, &build_dir)?;
    let build_context = build_manifest_context(
        &config_path,
        &template.dir,
        &resolved.project.repo_ref,
        &theme_package_signature(&resolved)?,
        &docs_route_base(&resolved),
        &generator_fingerprint(),
    );
    let mut builder = SiteBuilder::new(&resolved, &template.dir, &build_dir, runtime);
    builder.tracer = tracer.clone();
    builder.serve = opts.serve;
    builder.current_version_path = opts.current_version_path.clone();
    let prev_manifest = if opts.clean {
        Manifest::default()
    } else {
        builder.load_manifest()?
    };

    let spinner = report.spinner("Template", "preparing workspace", None);
    if !opts.clean {
        prune_stale_build_overlay(prev_manifest.build.as_ref(), &build_dir, &build_context)
            .map_err(|e| SiteError::io(&build_dir, e))?;
    }
    let injection = builder.prepare(opts.clean)?;
    let public: Vec<PublicFile> = resolved
        .public
        .iter()
        .map(|source| PublicFile {
            source: PathBuf::from(source),
            dest: Path::new(source)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
        })
        .collect();
    builder.copy_public_files(&public)?;
    drop(spinner);
    for warning in &injection.warnings {
        report.warning(warning);
    }
    report.step_ok("Template", ".build/ workspace ready");
    if opts.verbose {
        for (target, reason) in &injection.skipped {
            report.detail(
                &format!("Skipped template injection: {target} ({reason})"),
                DIM,
            );
        }
    }

    let generation = pages::generate_content_pages(
        GenerateArgs {
            builder: &mut builder,
            config: &resolved,
            modules: &sources.modules,
            docs: &sources.docs,
            project_dir,
            build_context: &build_context,
            clean: opts.clean,
            verbose: opts.verbose,
            prev_manifest: Some(&prev_manifest),
            source_hashes: None,
        },
        &report,
    )?;
    let mut page_detail = count_phrase(generation.total_pages, "page", None);
    if generation.skipped > 0 {
        page_detail = format!(
            "{page_detail}, {}",
            count_phrase(generation.skipped, "skipped page", None)
        );
    }
    report.step_ok("Pages", &page_detail);

    let previews = finalize::finalize_generated_files(
        &mut builder,
        &host,
        &resolved,
        project_dir,
        &generation.source_routes(),
        Some(&registry),
        opts.previews,
        &report,
        &mut |request| {
            let nested = BuildOptions {
                plugins: opts.plugins.clone(),
                quiet: true,
                output_override: Some(request.output_dir),
                build_dir_override: Some(request.build_dir),
                ..BuildOptions::new("docs.yaml")
            };
            run_build(&request.project_dir, &nested, ui, tracer.clone())
        },
    )?;
    if previews.built + previews.reused + previews.swept > 0 {
        let mut parts = Vec::new();
        if previews.built > 0 {
            parts.push(count_phrase(
                previews.built,
                "example rebuilt",
                Some("examples rebuilt"),
            ));
        }
        if previews.reused > 0 {
            parts.push(count_phrase(
                previews.reused,
                "unchanged",
                Some("unchanged"),
            ));
        }
        if previews.swept > 0 {
            parts.push(count_phrase(previews.swept, "removed", Some("removed")));
        }
        report.step_ok("Previews", &parts.join(", "));
    }
    if !opts.previews {
        // The examples are left as they are: whatever an earlier build
        // published still serves, and a page that embeds an unbuilt one says so.
        let examples =
            SiteBuilder::preview_example_dirs(&project_dir.join("docs").join("examples")).len();
        if examples > 0 {
            report.step(
                "Previews",
                &format!(
                    "{} not built; `folioh serve --previews` builds them",
                    count_phrase(examples, "example project", None)
                ),
                "-",
                DIM,
                BOLD,
            );
        }
    }
    if opts.verbose && generation.skipped > 0 {
        report.detail(
            &format!("Skipped {} unchanged page(s)", generation.skipped),
            DIM,
        );
    }

    let report_links = |broken: &[folioh_site::links::BrokenLink]| {
        if broken.is_empty() {
            report.step_ok("Links", "valid");
        } else {
            report.step(
                "Links",
                &count_phrase(broken.len(), "broken internal link", None),
                "!",
                YELLOW,
                BOLD_YELLOW,
            );
            for link in broken {
                report.detail(
                    &format!(
                        "{}:{} → {}",
                        link.source_page, link.line_number, link.target
                    ),
                    YELLOW,
                );
            }
        }
    };
    if opts.serve || noop {
        let spinner = report.spinner("Links", "checking internal links", None);
        let broken = finalize::check_generated_links(&builder, &sources.modules, &sources.docs);
        drop(spinner);
        report_links(&broken);
    }

    // The row names the phase it is in, because the check takes an instant and
    // the install can take minutes, and a reader waiting on one deserves to
    // know which.
    let mut spinner = Some(report.spinner("Dependencies", "checking node and pnpm", None));
    let install = builder.install_deps(&mut |phase| {
        spinner = None;
        spinner = Some(report.spinner("Dependencies", phase, None));
    })?;
    spinner = None;
    let _ = spinner;
    report.step_ok(
        "Dependencies",
        if install.installed {
            "installed with pnpm"
        } else {
            "up to date, the lockfile has not moved"
        },
    );
    if opts.verbose {
        for line in &install.output {
            report.detail(line.trim_end(), DIM);
        }
    }

    let (llms_txt, llms_full) =
        finalize::llm_texts(&resolved, project_dir, &sources.modules, &sources.docs);
    if opts.serve {
        builder.write_llm_files(llms_txt.as_deref(), llms_full.as_deref(), true)?;
    } else {
        let exported = {
            let _spinner = report.spinner("Export", "running the frontend build", None);
            builder.export_static_site(llms_txt.as_deref(), llms_full.as_deref())
        };
        let export = match exported {
            Ok(export) => export,
            Err(SiteError::Build { output, log_path }) => {
                if !opts.quiet {
                    print_build_output(ui, &output);
                }
                return Err(SiteError::Build { output, log_path }.into());
            }
            Err(err) => return Err(err.into()),
        };
        for warning in &export.warnings {
            report.warning(warning);
        }
        let mut diag = Vec::new();
        host.post_build(Path::new(&resolved.output_dir), &mut diag);
        for warning in &diag {
            report.warning(warning);
        }
        if !noop {
            let spinner = report.spinner("Links", "checking routes and rendered fragments", None);
            let broken = finalize::check_exported_links(&builder, &sources.modules, &sources.docs)?;
            drop(spinner);
            report_links(&broken);
        }
        report.step_ok("Export", &export.detail());
        if !opts.quiet {
            print_build_output(ui, &export.output_lines);
        }
    }

    // Commit the cache only after every output succeeded.
    builder.save_manifest(&Manifest {
        build: Some(generation.build_context.clone()),
        sources: generation.sources.clone(),
    })?;
    let elapsed = t0.elapsed().as_secs_f64();
    let pages_phrase = count_phrase(generation.total_pages, "page", None);
    if opts.serve {
        report.step(
            "Done",
            &format!("{pages_phrase}, ready in {elapsed:.1}s"),
            "✓",
            GREEN,
            BOLD_GREEN,
        );
        let mut site = Site {
            builder,
            config: &resolved,
            host: &host,
            plugins: &opts.plugins,
            registry: &registry,
            project_dir,
            sources,
            verbose: opts.verbose,
        };
        on_ready(&mut site, &report)?;
    } else {
        report.step(
            "Done",
            &format!("{pages_phrase} in {elapsed:.1}s"),
            "✓",
            GREEN,
            BOLD_GREEN,
        );
        report.step(
            "Site ready",
            &format!("{output_display}/"),
            "✓",
            MAGENTA,
            BOLD_MAGENTA,
        );
    }
    Ok(())
}

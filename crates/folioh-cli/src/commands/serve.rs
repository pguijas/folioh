//! `folioh serve`: the pipeline in serve mode, the Next dev server, the file
//! watcher over the source and docs roots, and the `--versions` static
//! preview of the version matrix.

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use folioh_config::trace::Tracer;
use folioh_config::{
    canonicalize_lenient, disabled_feature_message, is_feature_enabled, resolve_output_dir,
};
use folioh_docs::{enabled_languages, file_extensions};
use folioh_site::runtime::is_port_in_use;
use folioh_watch::{
    run_loop, watch, with_preview_examples, Debounce, Event, LanguageRoots, WatchConfig,
};
use serde_json::json;

use crate::cli::ServeArgs;
use crate::error::CliError;
use crate::pipeline::batch::SourceBatchHandler;
use crate::pipeline::{
    build_site, load_config, plugin_host, preview, tracer_from_env, BuildError, BuildOptions,
    Report, Site,
};
use crate::signals::{stop_child, StopSignals};
use crate::ui::text::{BOLD, DIM, GREEN, RED, YELLOW};
use crate::ui::Ui;
use crate::PluginFactory;

use super::versions::build_configured_versions;

/// `folioh serve`: build, then the dev server and the watcher until the
/// server exits; `--versions` builds the matrix and serves it statically.
pub fn run(ui: &Ui, args: ServeArgs, plugins: &[PluginFactory]) -> Result<(), CliError> {
    let target = args.project.resolve()?;
    if args.versions {
        if !is_feature_enabled("versions") {
            ui.print_styled(YELLOW, &disabled_feature_message("versions"));
            return Err(CliError::Exit(1));
        }
        let config = load_config(&target.join(&args.config), &target, &plugin_host(plugins))
            .map_err(CliError::message)?
            .config;
        ui.print_styled(BOLD, "Building configured versions for static preview...");
        build_configured_versions(ui, &target, args.verbose, &args.config, args.clean, plugins)?;
        let site_dir = resolve_output_dir(&target, &config.output_dir, &config.source_roots())
            .map_err(CliError::message)?;
        preview::serve_static_site(ui, &site_dir, args.port, args.open, args.kill_existing)?;
        return Ok(());
    }
    // The config is read before any build so a missing file is `Error:`.
    load_config(&target.join(&args.config), &target, &plugin_host(plugins))
        .map_err(CliError::message)?;
    // A busy port is refused now, not after a build of minutes; the dev
    // server repeats the check (and `--kill-existing` acts) when it starts.
    if !args.kill_existing && is_port_in_use(args.port) {
        return Err(CliError::Message(format!(
            "Port {} is already in use. Stop the existing process or rerun with --kill-existing.",
            args.port
        )));
    }
    let tracer = tracer_from_env(ui);
    let opts = BuildOptions {
        serve: true,
        serve_port: Some(args.port),
        verbose: args.verbose,
        clean: args.clean,
        previews: args.previews,
        plugins: plugins.to_vec(),
        ..BuildOptions::new(&args.config)
    };
    build_site(&target, &opts, ui, tracer.clone(), &mut |site, report| {
        serve_site(site, report, &args, tracer.clone())
    })?;
    Ok(())
}

/// The roots the OS watcher subscribes to: language roots, doc roots, the
/// preview examples and the built-ins' directories, existing and deduplicated.
fn watched_roots(cfg: &WatchConfig, project_dir: &std::path::Path) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = cfg
        .languages
        .iter()
        .flat_map(|l| l.roots.iter().cloned())
        .chain(cfg.doc_roots.iter().cloned())
        .collect();
    for input in &cfg.source_files {
        // Watch a directory, so deleting/recreating an input stays observable.
        if let Some(parent) = input.parent().filter(|path| path.is_dir()) {
            if !roots.iter().any(|root| parent.starts_with(root)) {
                roots.push(parent.to_path_buf());
            }
        }
    }
    roots = with_preview_examples(roots, project_dir);
    for plugin_root in &cfg.plugin_roots {
        if !roots
            .iter()
            .any(|r| canonicalize_lenient(r) == canonicalize_lenient(plugin_root))
        {
            roots.push(plugin_root.clone());
        }
    }
    roots
}

fn watch_config(site: &Site<'_>) -> WatchConfig {
    let config = site.config;
    let existing = |paths: &[String]| -> Vec<PathBuf> {
        paths
            .iter()
            .map(PathBuf::from)
            .filter(|p| p.exists())
            .collect()
    };
    // A configured language without a reader is watched by nothing and
    // contributes no modules.
    let languages = enabled_languages(config)
        .into_iter()
        .map(|language| LanguageRoots {
            extensions: file_extensions(language)
                .iter()
                .map(|suffix| suffix.to_string())
                .collect(),
            roots: existing(&config.language_source(language).paths),
        })
        .collect();
    let examples = site.project_dir.join("docs").join("examples");
    WatchConfig {
        languages,
        source_files: site
            .sources
            .modules
            .iter()
            .flat_map(|module| {
                folioh_docs::source_dependencies(std::path::Path::new(&module.source_file))
            })
            .collect(),
        doc_roots: existing(&config.source.docs),
        plugin_roots: site.host.watch_dirs(config),
        preview_examples: examples.is_dir().then_some(examples),
        generated_dirs: vec![
            site.builder.build_dir.clone(),
            site.builder.output_dir.clone(),
        ],
    }
    .canonicalized()
}

/// Start the dev server, open the browser, watch, block until the server
/// exits. The child never outlives the loop.
fn serve_site(
    site: &mut Site<'_>,
    report: &Report,
    args: &ServeArgs,
    tracer: Option<Arc<Tracer>>,
) -> Result<(), BuildError> {
    let ui = report.ui();
    report.line("");
    report.styled(BOLD, "  Starting dev server...");
    report.line("");
    let relay_tracer = tracer.clone();
    // The dev server announces itself ("✓ Ready in 1.2s"); --open waits for
    // that line instead of guessing how long the first compile takes.
    let (ready_tx, ready_rx) = mpsc::channel::<()>();
    let mut ready_tx = args.open.then_some(ready_tx);
    let relay: Box<dyn FnMut(&str) + Send> = Box::new(move |line: &str| {
        let text = line.trim_end_matches(['\n', '\r']);
        if let Some(tracer) = &relay_tracer {
            tracer.trace("next_stdout", &[("line", json!(text))]);
        }
        if text.to_ascii_lowercase().contains("ready") {
            if let Some(tx) = ready_tx.take() {
                let _ = tx.send(());
            }
        }
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "{text}");
    });
    let interrupt = StopSignals::install()?;
    let child = site.builder.serve(args.port, args.kill_existing, relay)?;
    if args.open {
        let url = format!("http://localhost:{}", args.port);
        thread::spawn(move || {
            // A server that never says "ready" still gets the browser, late.
            let _ = ready_rx.recv_timeout(Duration::from_secs(60));
            preview::open_in_browser(&url);
        });
    }

    let cfg = watch_config(site);
    let roots = watched_roots(&cfg, site.project_dir);
    let (watcher, rx) = match watch(&roots) {
        Ok(watching) => watching,
        Err(err) => {
            if let Some(mut child) = child {
                let _ = stop_child(&mut child);
            }
            return Err(BuildError::Failed(err.to_string()));
        }
    };
    report.styled(DIM, "  Watching for file changes...");
    report.line("");

    let verbose = args.verbose;
    let mut handler = SourceBatchHandler::new(site, Report::for_batches(ui), tracer.clone());
    let mut on_event = |event: Event| match event {
        Event::Error(_) => ui.print_styled(RED, &format!("  {event}")),
        Event::Warning(_) => ui.print_styled(YELLOW, &format!("  {event}")),
        Event::BatchDone { .. } | Event::PreviewsRefreshed(_) => {
            if verbose {
                ui.print_styled(GREEN, &format!("  {event}"));
            }
        }
    };
    let server_exit: Arc<Mutex<Option<Result<ExitStatus, String>>>> = Arc::new(Mutex::new(None));
    let watched_exit = Arc::clone(&server_exit);
    thread::scope(|scope| {
        // The watcher lives with the server: when the child exits the
        // watcher drops, the channel closes and the loop below returns.
        let interrupt = &interrupt;
        scope.spawn(move || {
            let mut child = child;
            loop {
                if interrupt.exit_code().is_some() {
                    if let Some(child) = child.as_mut() {
                        *watched_exit.lock().unwrap() =
                            Some(stop_child(child).map_err(|err| err.to_string()));
                    }
                    break;
                }
                if let Some(child) = child.as_mut() {
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            *watched_exit.lock().unwrap() = Some(Ok(status));
                            break;
                        }
                        Ok(None) => {}
                        Err(err) => {
                            // A wait error cannot prove that the child is
                            // still unreaped; do not signal a recycled group.
                            *watched_exit.lock().unwrap() = Some(Err(err.to_string()));
                            break;
                        }
                    }
                }
                // This also makes the no-op runtime's watcher interruptible.
                thread::sleep(Duration::from_millis(25));
            }
            drop(watcher);
        });
        run_loop(
            &rx,
            &cfg,
            Debounce::default(),
            &mut handler,
            tracer.as_deref(),
            &mut on_event,
        );
    });
    // The dev server dying mid-session is a failed run, not a clean exit.
    let exit = server_exit
        .lock()
        .unwrap()
        .take()
        .transpose()
        .map_err(BuildError::Failed)?;
    if let Some(code) = interrupt.exit_code() {
        ui.print("Server stopped.");
        return Err(BuildError::Interrupted(code));
    }
    match exit {
        Some(status) if !status.success() => {
            Err(BuildError::Failed(format!("dev server {status}")))
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
#[path = "serve_tests.rs"]
mod tests;

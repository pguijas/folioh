//! `folioh serve` as a black-box run with the frontend runtime stubbed: the
//! serve transcript, then live batches over a copy of the example site: a
//! module edit republishes its page and the aggregates, a broken edit is one
//! error line that commits nothing, the fix recovers, a deleted guide leaves
//! the routes, mirrors and LLM files, a new guide enters them, a no-change
//! save rewrites nothing, and `FOLIOH_TRACE` records the batch events.

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use serde_json::Value;

mod common;

use common::{copy_tree, read};

/// The running server: killed on drop so a failed assertion leaves no orphan.
struct Server {
    child: Child,
    stdout: Arc<Mutex<String>>,
}

impl Server {
    fn spawn(project: &Path, trace: &Path) -> Server {
        Self::spawn_with(project, trace, &[])
    }

    fn spawn_with(project: &Path, trace: &Path, extra: &[&str]) -> Server {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_folioh"));
        let port = common::free_port().to_string();
        let mut child = common::env::scrub(&mut cmd)
            .current_dir(project)
            .args(["serve", "-v", "--port", &port])
            .args(extra)
            .env("FOLIOH_TRACE", trace)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("folioh binary");
        let stdout = Arc::new(Mutex::new(String::new()));
        let sink = Arc::clone(&stdout);
        let pipe = child.stdout.take().unwrap();
        thread::spawn(move || {
            for line in BufReader::new(pipe).lines().map_while(Result::ok) {
                sink.lock().unwrap().push_str(&line);
                sink.lock().unwrap().push('\n');
            }
        });
        Server { child, stdout }
    }

    fn output(&self) -> String {
        self.stdout.lock().unwrap().clone()
    }

    fn wait_for(&self, needle: &str) {
        let started = Instant::now();
        while !self.output().contains(needle) {
            assert!(
                started.elapsed() < Duration::from_secs(40),
                "timed out waiting for {needle:?} in:\n{}",
                self.output()
            );
            thread::sleep(Duration::from_millis(100));
        }
    }

    fn count(&self, needle: &str) -> usize {
        self.output().matches(needle).count()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn wait_until(what: &str, mut condition: impl FnMut() -> bool) {
    let started = Instant::now();
    while !condition() {
        assert!(
            started.elapsed() < Duration::from_secs(40),
            "timed out waiting until {what}"
        );
        thread::sleep(Duration::from_millis(100));
    }
}

/// An editor's atomic save: the bytes land in one rename, so the watcher
/// never observes the truncated file a plain `fs::write` leaves behind for
/// a moment. The temp file is a dotfile outside the docs root, never an
/// event of its own.
fn save_atomically(project: &Path, path: &Path, bytes: &str) {
    let tmp = project.join(format!(
        ".{}.tmp",
        path.file_name().unwrap().to_string_lossy()
    ));
    fs::write(&tmp, bytes).unwrap();
    fs::rename(&tmp, path).unwrap();
}

fn mtime(path: &Path) -> SystemTime {
    fs::metadata(path).unwrap().modified().unwrap()
}

fn routes(build: &Path) -> Vec<String> {
    let contract: Value =
        serde_json::from_str(&read(&build.join("public/_folioh/contract.json"))).unwrap();
    contract["routes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect()
}

const ADD_SUBTRACT: &str = "\n\ndef subtract(left: int, right: int) -> int:\n    \"\"\"Subtract right from left.\"\"\"\n    return left - right\n";

#[test]
fn serve_builds_then_republishes_on_every_save() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let project = root.join("generated-site");
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/examples/generated-site"),
        &project,
    );
    let build = project.join(".build");
    let trace = root.join("trace.jsonl");
    let server = Server::spawn(&project, &trace);
    server.wait_for("  Watching for file changes...");

    // The serve transcript: no Export, `ready in`, the dev-server lines.
    let output = server.output();
    assert!(output.contains("✓ Sources      › 2 modules, 3 doc pages"));
    assert!(
        output.contains("✓ Done         › 5 pages, ready in "),
        "{output}"
    );
    assert!(!output.contains("✓ Export") && !output.contains("Site ready"));
    assert!(
        output.contains("\n  Starting dev server...\n\n"),
        "{output}"
    );
    // Serve writes the LLM files into the workspace public root.
    assert!(
        !build.join("public/llms.txt").exists(),
        "llm is off in the example"
    );
    let arithmetic_page = build.join("content/api-reference/example_package/arithmetic.mdx");
    let cli_page = build.join("content/cli.mdx");
    let search_index = build.join("lib/search-index.ts");
    let cli_before = mtime(&cli_page);
    let manifest_path = build.join(".folioh-manifest.json");

    // Directory metadata refreshes without editing a page or restarting serve.
    let sidebar = build.join("content/_meta.ts");
    let baseline_sidebar = read(&sidebar);
    let metadata = project.join("docs/_meta.yaml");
    let precedes = |first: &str, second: &str| {
        let text = read(&sidebar);
        text.find(&format!("\"{first}\":")).unwrap() < text.find(&format!("\"{second}\":")).unwrap()
    };
    let batches = server.count("  Source batch: ");
    fs::write(&metadata, "order: [components, cli, index]\n").unwrap();
    wait_until("metadata creation orders components first", || {
        precedes("components", "cli")
    });
    wait_until("metadata creation commits", || {
        server.count("  Source batch: ") > batches
    });
    let batches = server.count("  Source batch: ");
    fs::write(&metadata, "order: [cli, components, index]\n").unwrap();
    wait_until("metadata edit orders cli first", || {
        precedes("cli", "components")
    });
    wait_until("metadata edit commits", || {
        server.count("  Source batch: ") > batches
    });
    let valid_sidebar = read(&sidebar);
    fs::write(&metadata, "order: wrong\n").unwrap();
    server.wait_for("_meta.yaml:");
    assert_eq!(read(&sidebar), valid_sidebar);
    let batches = server.count("  Source batch: ");
    fs::remove_file(&metadata).unwrap();
    wait_until("metadata removal restores the default order", || {
        read(&sidebar) == baseline_sidebar
    });
    wait_until("metadata removal commits", || {
        server.count("  Source batch: ") > batches
    });
    // The intentional invalid metadata event is already accounted for here.
    let metadata_errors = server.count("Watcher error");

    // A module edit republishes its page, its mirror and the API index.
    let arithmetic = project.join("src/example_package/arithmetic.py");
    let original = read(&arithmetic);
    fs::write(&arithmetic, format!("{original}{ADD_SUBTRACT}")).unwrap();
    wait_until("the page carries subtract", || {
        read(&arithmetic_page).contains("### `subtract`")
    });
    server.wait_for("  Source batch: 5 pages, 3 reused");
    assert!(read(
        &build.join("public/_folioh/markdown/api-reference/example_package/arithmetic.md")
    )
    .contains("subtract"));
    assert_eq!(
        mtime(&cli_page),
        cli_before,
        "an untouched guide is not rewritten"
    );
    let manifest_after_edit = read(&manifest_path);
    assert!(!manifest_after_edit.is_empty() && server.count("Watcher error") == metadata_errors);

    // A broken edit is one error line naming the file; nothing is committed.
    let page_before_break = read(&arithmetic_page);
    fs::write(&arithmetic, "class Broken(\n").unwrap();
    wait_until("broken Python reports its own error", || {
        server.count("Watcher error") > metadata_errors
    });
    let output = server.output();
    assert!(
        output.contains("arithmetic.py")
            && output.contains("Batch not committed; retry on next save."),
        "{output}"
    );
    thread::sleep(Duration::from_millis(300));
    assert_eq!(read(&arithmetic_page), page_before_break);
    assert_eq!(read(&manifest_path), manifest_after_edit);

    // The fix recovers and republishes what the failed batch held.
    fs::write(
        &arithmetic,
        format!("{original}{ADD_SUBTRACT}\n\ndef multiply(left: int, right: int) -> int:\n    \"\"\"Multiply.\"\"\"\n    return left * right\n"),
    )
    .unwrap();
    wait_until("the page carries multiply", || {
        read(&arithmetic_page).contains("### `multiply`")
    });
    assert!(read(&arithmetic_page).contains("### `subtract`"));

    // A deleted guide leaves the content tree, the mirror and the routes.
    assert!(routes(&build).contains(&"/docs/cli/".to_string()));
    fs::remove_file(project.join("docs/cli.md")).unwrap();
    wait_until("cli.mdx is gone", || !cli_page.exists());
    server.wait_for("  Source batch: 4 pages, ");
    wait_until("the cli route left the contract", || {
        !routes(&build).contains(&"/docs/cli/".to_string())
    });
    assert!(!build.join("public/_folioh/markdown/cli.md").exists());
    assert!(!read(&search_index).contains("/docs/cli/"));

    // A new guide enters the tree and the routes.
    let batches_before = server.count("  Source batch: ");
    fs::write(project.join("docs/guide.md"), "# Guide\n\nA new page.\n").unwrap();
    wait_until("guide.mdx exists", || {
        build.join("content/guide.mdx").exists()
    });
    wait_until("the guide route entered the contract", || {
        routes(&build).contains(&"/docs/guide/".to_string())
    });
    assert!(build.join("public/_folioh/markdown/guide.md").exists());
    // The batch row follows the manifest save: only then is the batch over.
    wait_until("the guide batch committed", || {
        server.count("  Source batch: ") > batches_before
    });

    // A save with identical bytes is a batch that rewrites nothing.
    let batches_before = server.count("  Source batch: ");
    let components_page = build.join("content/components.mdx");
    let components_before = mtime(&components_page);
    let search_before = mtime(&search_index);
    let manifest_before = read(&manifest_path);
    let components = project.join("docs/components.md");
    save_atomically(&project, &components, &read(&components));
    wait_until("a no-change batch ran", || {
        server.count("  Source batch: ") > batches_before
    });
    assert_eq!(mtime(&components_page), components_before);
    assert_eq!(mtime(&search_index), search_before);
    assert_eq!(read(&manifest_path), manifest_before);

    // The trace records the batch events with their envelope.
    let lines: Vec<Value> = read(&trace)
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    for event in [
        "watcher_event",
        "batch_start",
        "batch_end",
        "manifest_saved",
    ] {
        assert!(lines.iter().any(|l| l["event"] == event), "{event} missing");
    }
    for line in &lines {
        assert!(
            line["t_ns"].is_number() && line["wall_ms"].is_number() && line["batch"].is_number(),
            "{line}"
        );
    }
    assert!(lines
        .iter()
        .filter(|l| l["event"] == "batch_end")
        .all(|l| l["batch"].as_u64().unwrap() >= 1));
    drop(server);
}

#[test]
fn serve_builds_the_examples_only_with_previews() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let project = root.join("site");
    fs::create_dir_all(project.join("docs/examples/mini/docs")).unwrap();
    fs::write(
        project.join("docs.yaml"),
        "project:\n  name: Site\nsource:\n  docs: [docs]\n",
    )
    .unwrap();
    fs::write(project.join("docs/index.md"), "# Site\n").unwrap();
    fs::write(
        project.join("docs/examples/mini/docs.yaml"),
        "project:\n  name: Mini\nsource:\n  docs: [docs]\n",
    )
    .unwrap();
    fs::write(
        project.join("docs/examples/mini/docs/index.md"),
        "# Mini\n\nA preview.\n",
    )
    .unwrap();
    let published = project.join(".build/public/_folioh/examples/mini");

    // Cold: the row says what was skipped and how to build it; nothing is published.
    let server = Server::spawn(&project, &root.join("cold.jsonl"));
    server.wait_for("  Watching for file changes...");
    assert!(
        server.output().contains(
            "- Previews     › 1 example project not built; `folioh serve --previews` builds them"
        ),
        "{}",
        server.output()
    );
    assert!(!published.exists());
    drop(server);

    let server = Server::spawn_with(&project, &root.join("previews.jsonl"), &["--previews"]);
    server.wait_for("  Watching for file changes...");
    assert!(
        server
            .output()
            .contains("✓ Previews     › 1 example rebuilt"),
        "{}",
        server.output()
    );
    assert!(published.join("manifest.json").is_file());
}

#[test]
fn serve_errors_before_the_build() {
    let dir = tempfile::tempdir().unwrap();
    let root: PathBuf = dir.path().canonicalize().unwrap();
    let missing = Command::new(env!("CARGO_BIN_EXE_folioh"))
        .current_dir(&root)
        .args(["serve"])
        .env("NO_COLOR", "1")
        .env("FOLIOH_FRONTEND_RUNTIME", "noop")
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(missing.stderr).unwrap(),
        format!(
            "Error: Config file not found: {}/docs.yaml\n",
            root.display()
        )
    );
    assert!(missing.stdout.is_empty());

    // A busy port is refused before the build starts, not after it.
    fs::write(root.join("docs.yaml"), "project:\n  name: Demo\n").unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port().to_string();
    let busy = Command::new(env!("CARGO_BIN_EXE_folioh"))
        .current_dir(&root)
        .args(["serve", "--port", &port])
        .env("NO_COLOR", "1")
        .env("FOLIOH_FRONTEND_RUNTIME", "noop")
        .output()
        .unwrap();
    drop(listener);
    assert_eq!(busy.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(busy.stderr).unwrap(),
        format!(
            "Error: Port {port} is already in use. Stop the existing process or rerun with --kill-existing.\n"
        )
    );
    assert!(busy.stdout.is_empty());
    assert!(!root.join(".build").exists());
}

#[test]
fn included_rust_docs_are_watched_outside_the_source_directory() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path();
    fs::create_dir(project.join("src")).unwrap();
    fs::write(
        project.join("Cargo.toml"),
        "[package]\nname = \"included-docs\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    fs::write(
        project.join("docs.yaml"),
        "project: {name: Included}\nsource:\n  rust:\n    paths: [src]\n",
    )
    .unwrap();
    fs::write(
        project.join("src/lib.rs"),
        "#![doc = include_str!(\"../README.md\")]\npub fn example() {}\n",
    )
    .unwrap();
    let input = project.join("README.md");
    fs::write(&input, "Original included prose.\n").unwrap();
    let server = Server::spawn(project, &project.join("trace.jsonl"));
    server.wait_for("Watching for file changes");
    let page = project.join(".build/content/api-reference/rust/included_docs.mdx");
    save_atomically(project, &input, "Changed included prose.\n");
    wait_until("included docs edit published", || {
        read(&page).contains("Changed included prose.")
    });
    fs::remove_file(&input).unwrap();
    wait_until("included docs deletion published", || {
        !read(&page).contains("Changed included prose.")
    });
    save_atomically(project, &input, "Recreated included prose.\n");
    wait_until("included docs recreation published", || {
        read(&page).contains("Recreated included prose.")
    });
    drop(server);
}

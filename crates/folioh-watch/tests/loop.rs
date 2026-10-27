//! The batch loop fed by an in-memory channel and a recording handler:
//! one republish per save, filter, plugin and preview dispatch, failure
//! recovery, warnings and the trace events.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use folioh_config::trace::Tracer;
use folioh_watch::{
    run_loop, BatchHandler, BatchSummary, ChangeKind, Debounce, Event, LanguageRoots, RawEvent,
    WatchConfig,
};

const FAST: Debounce = Debounce {
    quiet: Duration::from_millis(40),
    cap: Duration::from_millis(400),
};

/// How long a feeder waits for the loop to handle a set before the test fails.
const HANDLED: Duration = Duration::from_secs(10);

/// Records every handler call as `name:file,file`, can fail one call once,
/// and acknowledges each call on `ack` so a feeder can wait for it.
#[derive(Default)]
struct Recorder {
    calls: Vec<String>,
    fail_once: Option<&'static str>,
    handled: bool,
    warn: bool,
    ack: Option<Sender<()>>,
}

fn names<'a>(paths: impl IntoIterator<Item = &'a PathBuf>) -> String {
    paths
        .into_iter()
        .map(|p| p.file_name().unwrap().to_str().unwrap())
        .collect::<Vec<_>>()
        .join(",")
}

impl Recorder {
    fn call(&mut self, name: &'static str, detail: String) -> Result<(), String> {
        self.calls.push(format!("{name}:{detail}"));
        if let Some(ack) = &self.ack {
            let _ = ack.send(());
        }
        if self.fail_once.take_if(|f| *f == name).is_some() {
            return Err("boom".into());
        }
        Ok(())
    }
}

impl BatchHandler for Recorder {
    fn plugin_dirs_changed(&mut self, changes: &[(PathBuf, ChangeKind)]) -> Result<bool, String> {
        let detail: Vec<String> = changes
            .iter()
            .map(|(p, k)| format!("{}={}", p.file_name().unwrap().to_str().unwrap(), k.name()))
            .collect();
        self.call("plugin_dirs_changed", detail.join(","))?;
        Ok(self.handled)
    }
    fn refresh(
        &mut self,
        paths: &BTreeSet<PathBuf>,
        refresh_extensions: bool,
    ) -> Result<BatchSummary, String> {
        let ext = if refresh_extensions { ";ext" } else { "" };
        self.call("refresh", format!("{}{ext}", names(paths)))?;
        Ok(BatchSummary {
            pages: 3,
            skipped: 2,
            warnings: if self.warn {
                vec!["careful with refresh".into()]
            } else {
                vec![]
            },
        })
    }
    fn refresh_previews(&mut self, path: &Path) -> Result<(), String> {
        self.call("refresh_previews", names([&path.to_path_buf()]))
    }
}

struct Project {
    _dir: tempfile::TempDir,
    root: PathBuf,
    cfg: WatchConfig,
}

fn project() -> Project {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    for sub in [
        "demo",
        "docs/guide",
        "gallery/items",
        "docs/examples/landing/docs",
        "d/content",
        "do",
    ] {
        std::fs::create_dir_all(root.join(sub)).unwrap();
    }
    let cfg = WatchConfig {
        languages: vec![LanguageRoots {
            extensions: vec![".py".into()],
            roots: vec![root.join("demo")],
        }],
        source_files: Vec::new(),
        doc_roots: vec![root.join("docs")],
        plugin_roots: vec![root.join("gallery")],
        preview_examples: Some(root.join("docs/examples")),
        generated_dirs: vec![root.join("d"), root.join("do")],
    };
    Project {
        _dir: dir,
        root,
        cfg,
    }
}

fn ev(kind: ChangeKind, path: PathBuf) -> RawEvent {
    RawEvent { kind, path }
}

/// Play `sets` into the channel, each one after the handler has taken the one
/// before it, then close it. Waiting on the handler rather than on a quiet gap
/// keeps two sets apart on a runner slow enough to read both in one window.
fn play(tx: Sender<RawEvent>, sets: Vec<Vec<RawEvent>>, acks: Receiver<()>) -> JoinHandle<()> {
    thread::spawn(move || {
        for (index, set) in sets.into_iter().enumerate() {
            if index > 0 {
                acks.recv_timeout(HANDLED)
                    .expect("the loop handled the set before");
            }
            for event in set {
                tx.send(event).unwrap();
            }
        }
    })
}

/// Run the loop over `sets` and return the handler and the events it emitted.
fn run(
    p: &Project,
    mut handler: Recorder,
    tracer: Option<&Tracer>,
    sets: Vec<Vec<RawEvent>>,
) -> (Recorder, Vec<String>) {
    let (tx, rx) = mpsc::channel();
    let (ack, acks) = mpsc::channel();
    handler.ack = Some(ack);
    let feeder = play(tx, sets, acks);
    let mut events = Vec::new();
    run_loop(&rx, &p.cfg, FAST, &mut handler, tracer, &mut |e: Event| {
        events.push(e.to_string())
    });
    feeder.join().unwrap();
    (handler, events)
}

#[test]
fn one_event_set_is_one_batch_with_the_final_file_state() {
    let p = project();
    let readme = p.root.join("docs/README.md");
    let added = p.root.join("docs/added.md");
    let module = p.root.join("demo/added.py");
    for f in [&readme, &added, &module] {
        std::fs::write(f, "x").unwrap();
    }
    let set = vec![
        ev(ChangeKind::Deleted, readme.clone()),
        ev(ChangeKind::Added, readme.clone()),
        ev(ChangeKind::Added, added.clone()),
        ev(ChangeKind::Added, module.clone()),
    ];
    let (handler, events) = run(&p, Recorder::default(), None, vec![set.clone()]);
    assert_eq!(handler.calls, ["refresh:added.py,README.md,added.md"]);
    assert_eq!(events, ["Source batch: 3 pages, 2 reused"]);

    // An overlapping plugin dir (the project root) still lets sources through
    // and asks for the extensions refresh.
    let mut overlapping = Project {
        cfg: p.cfg.clone(),
        ..project()
    };
    overlapping.cfg.plugin_roots = vec![p.root.clone()];
    let handler = Recorder {
        handled: true,
        ..Recorder::default()
    };
    let (handler, _) = run(&overlapping, handler, None, vec![set]);
    assert_eq!(
        handler.calls,
        [
            "plugin_dirs_changed:added.py=added,README.md=added,added.md=added",
            "refresh:added.py,README.md,added.md;ext",
        ]
    );
}

#[test]
fn ignored_paths_run_no_batch_and_an_asset_lifecycle_runs_three() {
    let p = project();
    let asset = p.root.join("docs/shot.png");
    std::fs::write(&asset, "first").unwrap();
    let ignored = vec![
        ev(ChangeKind::Modified, p.root.join("docs/shot.png~")),
        ev(ChangeKind::Modified, p.root.join("docs/.shot.swp")),
        ev(ChangeKind::Modified, p.root.join("docs/.git/index")),
        ev(ChangeKind::Modified, p.root.join("d/content/index.mdx")),
        ev(ChangeKind::Modified, p.root.join("do/image.png")),
    ];
    let (tx, rx) = mpsc::channel();
    let (ack, acks) = mpsc::channel();
    let asset_thread = asset.clone();
    let feeder = thread::spawn(move || {
        for e in ignored {
            tx.send(e).unwrap();
        }
        thread::sleep(FAST.quiet * 3);
        for (kind, content) in [
            (ChangeKind::Modified, Some("changed")),
            (ChangeKind::Deleted, None),
            (ChangeKind::Added, Some("recreated")),
        ] {
            match content {
                Some(c) => std::fs::write(&asset_thread, c).unwrap(),
                None => std::fs::remove_file(&asset_thread).unwrap(),
            }
            tx.send(ev(kind, asset_thread.clone())).unwrap();
            acks.recv_timeout(HANDLED)
                .expect("the loop handled the change");
        }
    });
    let mut handler = Recorder {
        ack: Some(ack),
        ..Recorder::default()
    };
    run_loop(&rx, &p.cfg, FAST, &mut handler, None, &mut |_| {});
    feeder.join().unwrap();
    assert_eq!(
        handler.calls,
        ["refresh:shot.png", "refresh:shot.png", "refresh:shot.png"]
    );
}

#[test]
fn a_plugin_dir_change_is_offered_to_the_handler_and_republishes_when_handled() {
    let p = project();
    let manifest = p.root.join("gallery/gallery.yaml");
    let item = p.root.join("gallery/items/x.md");
    let outside = p.root.join("elsewhere.md");
    for f in [&manifest, &item, &outside] {
        std::fs::write(f, "x").unwrap();
    }
    let set = vec![
        ev(ChangeKind::Modified, manifest),
        ev(ChangeKind::Modified, item),
        ev(ChangeKind::Modified, outside),
    ];
    let handled = Recorder {
        handled: true,
        ..Recorder::default()
    };
    let (handler, events) = run(&p, handled, None, vec![set.clone()]);
    assert_eq!(
        handler.calls,
        [
            "plugin_dirs_changed:gallery.yaml=modified,x.md=modified",
            "refresh:;ext",
        ]
    );
    assert_eq!(events.len(), 1);

    let (handler, events) = run(&p, Recorder::default(), None, vec![set]);
    assert_eq!(
        handler.calls,
        ["plugin_dirs_changed:gallery.yaml=modified,x.md=modified"]
    );
    assert!(
        events.is_empty(),
        "an unhandled plugin change runs no batch"
    );
}

#[test]
fn a_preview_example_change_refreshes_previews_once_after_the_batch() {
    let p = project();
    let yaml = p.root.join("docs/examples/landing/docs.yaml");
    let page = p.root.join("docs/examples/landing/docs/index.md");
    let guide = p.root.join("docs/guide/intro.md");
    for f in [&yaml, &page, &guide] {
        std::fs::write(f, "x").unwrap();
    }
    let set = vec![
        ev(ChangeKind::Modified, guide),
        ev(ChangeKind::Modified, yaml),
        ev(ChangeKind::Modified, page),
    ];
    let (handler, events) = run(&p, Recorder::default(), None, vec![set]);
    assert_eq!(
        handler.calls,
        ["refresh:intro.md", "refresh_previews:docs.yaml"]
    );
    // Coalesced order is by path components: `docs/index.md` sorts before
    // `docs.yaml`, so the last preview path in the set is `docs.yaml`.
    assert_eq!(
        events,
        [
            "Source batch: 3 pages, 2 reused",
            "Updated preview examples: examples/landing/docs.yaml",
        ]
    );
}

#[test]
fn a_failed_batch_keeps_its_edits_for_the_next_save() {
    let p = project();
    let a = p.root.join("demo/a.py");
    let b = p.root.join("demo/b.py");
    std::fs::write(&a, "x").unwrap();
    std::fs::write(&b, "x").unwrap();
    let handler = Recorder {
        fail_once: Some("refresh"),
        ..Recorder::default()
    };
    let sets = vec![
        vec![ev(ChangeKind::Modified, a)],
        vec![ev(ChangeKind::Modified, b)],
    ];
    let (handler, events) = run(&p, handler, None, sets);
    assert_eq!(handler.calls, ["refresh:a.py", "refresh:a.py,b.py"]);
    assert_eq!(
        events,
        [
            "Watcher error: boom. Batch not committed; retry on next save.",
            "Source batch: 3 pages, 2 reused",
        ]
    );
}

#[test]
fn handler_warnings_reach_the_callback() {
    let p = project();
    let a = p.root.join("demo/a.py");
    std::fs::write(&a, "x").unwrap();
    let handler = Recorder {
        warn: true,
        ..Recorder::default()
    };
    let (_, events) = run(&p, handler, None, vec![vec![ev(ChangeKind::Modified, a)]]);
    assert_eq!(
        events,
        ["careful with refresh", "Source batch: 3 pages, 2 reused"]
    );
}

#[test]
fn trace_emits_jsonl_events_for_a_batch() {
    let p = project();
    let a = p.root.join("demo/a.py");
    std::fs::write(&a, "x").unwrap();
    let trace_file = p.root.join("trace.jsonl");
    let tracer = Tracer::open(&trace_file).unwrap();
    // A set nothing subscribes to leaves no line; an ignored event in a real
    // set is not counted in `n_raw`.
    run(
        &p,
        Recorder::default(),
        Some(&tracer),
        vec![vec![ev(ChangeKind::Modified, p.root.join("docs/.a.swp"))]],
    );
    run(
        &p,
        Recorder::default(),
        Some(&tracer),
        vec![vec![
            ev(ChangeKind::Modified, a.clone()),
            ev(ChangeKind::Modified, p.root.join("docs/.b.swp")),
            ev(ChangeKind::Modified, a),
        ]],
    );
    drop(tracer);
    let lines: Vec<serde_json::Value> = std::fs::read_to_string(&trace_file)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let events: Vec<&str> = lines.iter().map(|l| l["event"].as_str().unwrap()).collect();
    assert_eq!(
        events,
        [
            "watcher_event",
            "batch_start",
            "manifest_saved",
            "batch_end"
        ]
    );
    for line in &lines {
        for key in ["t_ns", "wall_ms", "batch"] {
            assert!(line.get(key).is_some(), "{key} missing in {line}");
        }
    }
    assert_eq!(lines[0]["n_raw"], 2);
    assert_eq!(lines[0]["batch"], 0, "outside any batch");
    assert_eq!(lines[1]["n_paths"], 1);
    assert!(
        lines[1..].iter().all(|l| l["batch"] == 1),
        "one batch id >= 1"
    );
    assert_eq!(lines[3]["pages"], 3);
    assert_eq!(lines[3]["skipped"], 2);
}

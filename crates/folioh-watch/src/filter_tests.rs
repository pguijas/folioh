use super::*;
use crate::events::{ChangeKind, RawEvent};
use std::path::{Path, PathBuf};

fn ev(kind: ChangeKind, path: PathBuf) -> RawEvent {
    RawEvent { kind, path }
}

/// A project with a python root, a rust root, a docs root, a gallery dir,
/// preview examples and generated dirs whose names prefix `docs`.
fn project() -> (tempfile::TempDir, WatchConfig) {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().canonicalize().unwrap();
    for sub in [
        "demo",
        "crate/src",
        "docs/guide",
        "gallery/items",
        "docs/examples/landing",
        "d/content",
        "do",
    ] {
        std::fs::create_dir_all(p.join(sub)).unwrap();
    }
    let cfg = WatchConfig {
        languages: vec![
            LanguageRoots {
                extensions: vec![".py".into()],
                roots: vec![p.join("demo")],
            },
            LanguageRoots {
                extensions: vec![".rs".into()],
                roots: vec![p.join("crate/src")],
            },
        ],
        source_files: Vec::new(),
        doc_roots: vec![p.join("docs")],
        plugin_roots: vec![p.join("gallery")],
        preview_examples: Some(p.join("docs/examples")),
        generated_dirs: vec![p.join("d"), p.join("do")],
    };
    (dir, cfg)
}

#[test]
fn generated_dirs_and_ignored_names_never_enter_a_batch() {
    let (dir, cfg) = project();
    let p = dir.path().canonicalize().unwrap();
    for rejected in [
        "docs/shot.png~",
        "docs/.shot.swp",
        "docs/.git/index",
        "d/content/index.mdx",
        "do/image.png",
        "demo/notes.txt",
        "elsewhere/core.py",
    ] {
        let raw = [ev(ChangeKind::Modified, p.join(rejected))];
        assert!(cfg.batch(&raw).is_empty(), "{rejected} should be ignored");
    }
}

#[test]
fn sources_docs_assets_plugins_and_previews_are_told_apart() {
    let (dir, cfg) = project();
    let p = dir.path().canonicalize().unwrap();
    let raw = [
        ev(ChangeKind::Added, p.join("demo/added.py")),
        ev(ChangeKind::Deleted, p.join("demo/gone.py")),
        ev(ChangeKind::Modified, p.join("crate/src/lib.rs")),
        ev(ChangeKind::Modified, p.join("docs/README.md")),
        ev(ChangeKind::Deleted, p.join("docs/guide/old.md")),
        ev(ChangeKind::Modified, p.join("docs/shot.png")),
        ev(ChangeKind::Deleted, p.join("docs/guide/old.png")),
        ev(ChangeKind::Modified, p.join("gallery/gallery.yaml")),
        ev(ChangeKind::Added, p.join("gallery/items/x.md")),
        ev(
            ChangeKind::Modified,
            p.join("docs/examples/landing/docs.yaml"),
        ),
        ev(
            ChangeKind::Modified,
            p.join("docs/examples/landing/docs/index.md"),
        ),
    ];
    let batch = cfg.batch(&raw);
    let paths: Vec<PathBuf> = batch.paths.iter().cloned().collect();
    assert_eq!(
        paths,
        [
            "crate/src/lib.rs",
            "demo/added.py",
            "demo/gone.py",
            "docs/README.md",
            "docs/guide/old.md",
            "docs/guide/old.png",
            "docs/shot.png",
        ]
        .map(|rel| p.join(rel))
    );
    assert_eq!(
        batch.plugin_changes,
        BTreeMap::from([
            (p.join("gallery/gallery.yaml"), ChangeKind::Modified),
            (p.join("gallery/items/x.md"), ChangeKind::Added),
        ])
    );
    // The preview dir sits under the docs root and still wins; last wins.
    assert_eq!(
        batch.preview_changed,
        Some(p.join("docs/examples/landing/docs/index.md"))
    );
}

#[test]
fn an_overlapping_plugin_root_still_lets_sources_through() {
    let (dir, mut cfg) = project();
    let p = dir.path().canonicalize().unwrap();
    cfg.plugin_roots = vec![p.clone()];
    let raw = [
        ev(ChangeKind::Added, p.join("docs/README.md")),
        ev(ChangeKind::Added, p.join("demo/added.py")),
    ];
    let batch = cfg.batch(&raw);
    assert_eq!(
        batch.paths,
        BTreeSet::from([p.join("demo/added.py"), p.join("docs/README.md")])
    );
    assert_eq!(batch.plugin_changes.len(), 2);
}

#[test]
fn an_event_at_a_link_path_lands_under_the_real_root_as_delivered() {
    let (dir, cfg) = project();
    let p = dir.path().canonicalize().unwrap();
    let link = dir.path().join("link");
    std::os::unix::fs::symlink(p.join("docs"), &link).unwrap();
    // A root given through the symlink resolves to the real docs dir.
    let via_link = WatchConfig {
        doc_roots: vec![link.join("guide")],
        ..cfg.clone()
    }
    .canonicalized();
    assert_eq!(via_link.doc_roots, vec![p.join("docs/guide")]);
    // inotify reports the link path: it lands under the real root and the
    // handler gets the path as delivered.
    let batch = cfg.batch(&[ev(ChangeKind::Modified, link.join("guide/intro.md"))]);
    assert_eq!(batch.paths, BTreeSet::from([link.join("guide/intro.md")]));
    // The generated-dirs test alone resolves symlinks.
    let out_link = dir.path().join("out-link");
    std::os::unix::fs::symlink(p.join("do"), &out_link).unwrap();
    assert!(!cfg.subscribed(&out_link.join("image.png")));
}

#[test]
fn merging_a_failed_batch_is_a_set_union_where_the_later_kind_wins() {
    let a = PathBuf::from("/p/demo/a.py");
    let b = PathBuf::from("/p/demo/b.py");
    let card = PathBuf::from("/p/gallery/x.md");
    let mut pending = Batch {
        paths: BTreeSet::from([a.clone()]),
        plugin_changes: BTreeMap::from([(card.clone(), ChangeKind::Added)]),
        preview_changed: Some(PathBuf::from("/p/docs/examples/one")),
    };
    pending.merge(Batch {
        paths: BTreeSet::from([a.clone(), b.clone()]),
        plugin_changes: BTreeMap::from([(card.clone(), ChangeKind::Deleted)]),
        preview_changed: None,
    });
    assert_eq!(pending.paths, BTreeSet::from([a, b]));
    assert_eq!(
        pending.plugin_changes,
        BTreeMap::from([(card, ChangeKind::Deleted)])
    );
    assert_eq!(
        pending.preview_changed,
        Some(PathBuf::from("/p/docs/examples/one"))
    );
    pending.merge(Batch {
        preview_changed: Some(PathBuf::from("/p/docs/examples/two")),
        ..Batch::default()
    });
    assert_eq!(
        pending.preview_changed,
        Some(PathBuf::from("/p/docs/examples/two"))
    );
}

#[test]
fn preview_examples_dir_is_appended_once() {
    let dir = tempfile::tempdir().unwrap();
    let doc_dir = dir.path().join("docs/guide");
    let examples = dir.path().join("docs/examples");
    std::fs::create_dir_all(&doc_dir).unwrap();
    std::fs::create_dir_all(&examples).unwrap();
    assert_eq!(
        with_preview_examples(vec![doc_dir.clone()], dir.path()),
        vec![doc_dir, examples.clone()]
    );
    assert_eq!(
        with_preview_examples(vec![examples.clone()], dir.path()),
        vec![examples]
    );
    let other = tempfile::tempdir().unwrap();
    assert_eq!(
        with_preview_examples(vec![], other.path()),
        Vec::<PathBuf>::new()
    );
}

#[test]
fn is_under_is_a_component_prefix_test() {
    let dirs = [PathBuf::from("/src/mylib"), PathBuf::from("/docs")];
    assert!(is_under(Path::new("/src/mylib/core.py"), &dirs));
    assert!(is_under(Path::new("/docs/guide.md"), &dirs));
    assert!(!is_under(
        Path::new("/other/file.py"),
        &[PathBuf::from("/src/mylib")]
    ));
    // `d` is a string prefix of `docs` but not a path prefix.
    assert!(!is_under(
        Path::new("/p/docs/shot.png"),
        &[PathBuf::from("/p/d")]
    ));
    assert!(!is_under(
        Path::new("/p/docs/shot.png"),
        &[PathBuf::from("/p/do")]
    ));
}

#[test]
fn default_ignore_mirrors_watchfiles_rules() {
    for rejected in [
        "/p/docs/shot.png~",
        "/p/docs/.shot.swp",
        "/p/docs/.git/index",
        "/p/src/__pycache__/core.cpython-312.pyc",
        "/p/src/core.pyc",
        "/p/src/core.pyo",
        "/p/src/flycheck_core.py",
        "/p/node_modules/x/index.js",
        "/p/.venv/lib/x.py",
        "/p/.mypy_cache/x",
    ] {
        assert!(
            default_ignore(Path::new(rejected)),
            "{rejected} should be ignored"
        );
    }
    for kept in [
        "/p/docs/shot.png",
        "/p/src/core.py",
        "/p/docs/guide/intro.md",
        "/p/gallery/gallery.yaml",
    ] {
        assert!(!default_ignore(Path::new(kept)), "{kept} should pass");
    }
}

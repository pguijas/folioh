use super::*;

#[test]
fn missing_doc_directories_do_not_expand_watches_to_an_ancestor() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("project");
    std::fs::create_dir_all(project.join("src")).unwrap();
    let cfg = WatchConfig {
        languages: vec![LanguageRoots {
            extensions: vec![".rs".into()],
            roots: vec![project.join("src")],
        }],
        source_files: vec![
            project.join("README.md"),
            dir.path().join("missing/deep/doc.md"),
        ],
        ..WatchConfig::default()
    };
    let roots = watched_roots(&cfg, &project);
    assert!(roots.contains(&project));
    assert!(!roots.contains(&dir.path().to_path_buf()));
    assert!(!roots.iter().any(|root| root == std::path::Path::new("/")));
}

//! Builder behaviours that read `FOLIOH_BASE_PATH`; isolated in their own process.

mod common;

use folioh_site::builder::PreviewBuildRequest;

#[test]
fn base_path_env_feeds_robots_pointers_nested_example_builds_and_their_digest() {
    std::env::remove_var("FOLIOH_PREVIEW_URL");
    std::env::remove_var("FOLIOH_EXAMPLE_SITE_URL");
    let dir = tempfile::tempdir().unwrap();
    std::env::set_var("FOLIOH_BASE_PATH", "/mylib");
    let config = common::config(dir.path(), "project:\n  name: MyLib\n");
    let output = std::path::Path::new(&config.output_dir).to_path_buf();
    common::write(&output, "robots.txt", "User-Agent: *\nAllow: /\n");
    let build = dir.path().join("build");
    let builder = common::builder(&config, &dir.path().join("template"), &build);
    assert_eq!(builder.base_path(), "/mylib");
    builder
        .write_llm_files(Some("# MyLib\n"), None, false)
        .unwrap();
    let robots = common::read(&output.join("robots.txt"));
    assert!(robots.contains("# llms.txt: /mylib/llms.txt\n") && !robots.contains("llms-full.txt"));

    std::env::set_var("FOLIOH_BASE_PATH", "/folioh");
    let builder = common::builder(&config, &dir.path().join("template"), &build);
    let example = dir.path().join("docs/examples/sample-preview");
    std::fs::create_dir_all(&example).unwrap();
    let target = build.join("public/_folioh/examples/sample-preview");
    let mut seen = Vec::new();
    builder
        .build_preview_example_project(&example, &target, &mut |request: PreviewBuildRequest| {
            assert_eq!(std::env::var("FOLIOH_EXAMPLE_SITE_URL").unwrap(), "");
            seen.push((request, std::env::var("FOLIOH_BASE_PATH").unwrap()));
            Ok(())
        })
        .unwrap();
    assert_eq!(
        seen,
        vec![(
            PreviewBuildRequest {
                project_dir: example.clone(),
                output_dir: target,
                build_dir: build.join(".preview-examples/sample-preview"),
                base_path: "/folioh/_folioh/examples/sample-preview".to_string(),
            },
            "/folioh/_folioh/examples/sample-preview".to_string()
        )]
    );
    assert_eq!(std::env::var("FOLIOH_BASE_PATH").unwrap(), "/folioh");
    assert!(std::env::var_os("FOLIOH_EXAMPLE_SITE_URL").is_none());

    // The nested build bakes the base path in, so a new one rebuilds it.
    let examples = dir.path().join("docs/examples");
    std::fs::write(example.join("docs.yaml"), "project:\n  name: Sample\n").unwrap();
    let mut run = |request: PreviewBuildRequest| {
        std::fs::create_dir_all(&request.output_dir).unwrap();
        std::fs::write(request.output_dir.join("index.html"), "<html></html>").unwrap();
        Ok(())
    };
    let mut counts = Vec::new();
    for base in ["/one", "/one", "/two"] {
        std::env::set_var("FOLIOH_BASE_PATH", base);
        let builder = common::builder(&config, &dir.path().join("template"), &build);
        let summary = builder.write_preview_examples(&examples, &mut run).unwrap();
        counts.push((summary.built, summary.reused));
    }
    assert_eq!(counts, [(1, 0), (0, 1), (1, 0)]);

    // A different public origin must not reuse social metadata from the old one.
    let mut counts = Vec::new();
    for url in [
        "https://one.example",
        "https://one.example",
        "https://two.example",
    ] {
        let mut configured = config.clone();
        configured.project.url = url.to_string();
        let builder = common::builder(&configured, &dir.path().join("template"), &build);
        let summary = builder
            .write_preview_examples(&examples, &mut |request| {
                assert_eq!(std::env::var("FOLIOH_EXAMPLE_SITE_URL").unwrap(), url);
                run(request)
            })
            .unwrap();
        counts.push((summary.built, summary.reused));
        assert!(std::env::var_os("FOLIOH_EXAMPLE_SITE_URL").is_none());
    }
    assert_eq!(counts, [(1, 0), (0, 1), (1, 0)]);

    // An originless example removes only the template-owned image conventions.
    let template = common::make_template(dir.path());
    for file in ["app/opengraph-image.tsx", "app/docs/opengraph-image.tsx"] {
        common::write(
            &template,
            file,
            "export default function Image() { return '__PROJECT_NAME__' }",
        );
    }
    std::env::set_var("FOLIOH_EXAMPLE_SITE_URL", "");
    let prepared = common::prepared(dir.path(), &config, &template, "originless");
    assert!(!prepared.join("app/opengraph-image.tsx").exists());
    assert!(!prepared.join("app/docs/opengraph-image.tsx").exists());
    common::write(
        &template,
        "app/opengraph-image.tsx",
        "export default function CustomImage() {}",
    );
    let custom = common::prepared(dir.path(), &config, &template, "custom-image");
    assert!(custom.join("app/opengraph-image.tsx").exists());
    std::env::remove_var("FOLIOH_EXAMPLE_SITE_URL");
}

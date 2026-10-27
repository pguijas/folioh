//! The `SiteBuilder` write surface over the no-op runtime.

mod common;

use std::collections::BTreeSet;
use std::path::Path;

use folioh_site::builder::{Manifest, PreviewBuildRequest, PublicFile, SiteBuilder};
use serde_json::json;

fn read(path: &Path) -> String {
    common::read(path)
}

#[test]
fn a_plugin_page_stays_owned_across_watch_passes_and_is_swept_when_disabled() {
    let dir = tempfile::tempdir().unwrap();
    let config = common::config(dir.path(), "");
    let build = dir.path().join("build");
    let template = dir.path().join("template");
    let source_routes = BTreeSet::from(["index".to_string()]);
    {
        let mut builder = common::builder(&config, &template, &build);
        builder.write_page("index", "# Guide").unwrap();
        builder
            .write_page("roadmap", "# Generated roadmap")
            .unwrap();
        builder.sweep_plugin_pages(&source_routes).unwrap();
        builder.write_page("index", "# Edited guide").unwrap();
        builder.sweep_plugin_pages(&source_routes).unwrap();
    }
    let mut builder = common::builder(&config, &template, &build);
    builder.register_route("index");
    builder.sweep_plugin_pages(&source_routes).unwrap();
    assert!(!build.join("content/roadmap.mdx").exists());
    assert!(!build.join("public/_folioh/markdown/roadmap.md").exists());
    assert_eq!(read(&build.join("content/index.mdx")), "# Edited guide");
}

#[test]
fn write_page_resolves_relative_links_in_the_page_and_its_mirror() {
    let dir = tempfile::tempdir().unwrap();
    let config = common::config(dir.path(), "template:\n  docs_route_base: /reference\n");
    let build = dir.path().join("build");
    let mut builder = common::builder(&config, &dir.path().join("template"), &build);
    builder
        .write_page(
            "guide/setup",
            "# Setup\n\nSee [install](./install#linux).\n",
        )
        .unwrap();
    assert!(read(&build.join("content/guide/setup.mdx"))
        .contains("See [install](/reference/guide/install#linux)."));
    assert!(read(&build.join("public/_folioh/markdown/guide/setup.md"))
        .contains("See [install](/reference/guide/install#linux)."));
}

#[test]
fn write_page_writes_the_page_and_its_markdown_mirror() {
    let dir = tempfile::tempdir().unwrap();
    let config = common::config(dir.path(), "");
    let build = dir.path().join("build");
    let mut builder = common::builder(&config, &dir.path().join("template"), &build);
    builder
        .write_page("getting-started", "# Getting Started\n\nHello world.")
        .unwrap();
    assert!(read(&build.join("content/getting-started.mdx")).contains("# Getting Started"));
    assert_eq!(
        read(&build.join("public/_folioh/markdown/getting-started.md")),
        "# Getting Started\n\nHello world.\n"
    );
    builder
        .write_page("api-reference/mylib/core", "# mylib.core")
        .unwrap();
    assert!(
        build.join("content/api-reference/mylib/core.mdx").exists()
            && build
                .join("public/_folioh/markdown/api-reference/mylib/core.md")
                .exists()
    );
    builder.write_page("index", "# Home").unwrap();
    assert!(
        build.join("content/index.mdx").exists()
            && build.join("public/_folioh/markdown/index.md").exists()
    );
    builder.write_page("guide/index", "# Guide").unwrap();
    assert!(build
        .join("public/_folioh/markdown/guide/index.md")
        .exists());
    builder.write_page("guide", "---\ntitle: Guide\n---\nimport { Callout } from \"@/components/callout\"\n\n# Guide\n\n<Callout type=\"info\">\nUse this content.\n</Callout>\n\n<ParamTable args={[]} />\n").unwrap();
    let markdown = read(&build.join("public/_folioh/markdown/guide.md"));
    assert!(
        !markdown.contains("title: Guide")
            && !markdown.contains("import {")
            && !markdown.contains("<Callout")
    );
    assert!(markdown.contains("# Guide") && markdown.contains("Use this content."));
    assert_eq!(builder.read_page("guide/index").unwrap(), "# Guide");
    assert!(
        builder.page_exists("guide/index").unwrap()
            && builder.page_markdown_exists("guide/index").unwrap()
    );
    assert_eq!(
        builder.read_page("../secrets").unwrap_err().to_string(),
        "Route would access outside content directory: ../secrets"
    );
    assert_eq!(
        builder
            .write_page("../escape", "x")
            .unwrap_err()
            .to_string(),
        "Route would write outside content directory: ../escape"
    );
    assert_eq!(
        builder.content_route_to_docs_url("api-reference/index"),
        "/docs/api-reference/"
    );
    assert_eq!(
        builder.content_route_to_docs_url("guide/plugins/authoring"),
        "/docs/guide/plugins/authoring/"
    );
    assert_eq!(builder.content_route_to_docs_url(""), "/docs/");
}

#[test]
fn list_and_remove_pages() {
    let dir = tempfile::tempdir().unwrap();
    let config = common::config(dir.path(), "");
    let build = dir.path().join("build");
    let mut builder = common::builder(&config, &dir.path().join("template"), &build);
    for route in [
        "gallery/index",
        "gallery/one-item/index",
        "gallery/one-item/compared",
        "guide/intro",
    ] {
        builder.write_page(route, "# Page\n").unwrap();
    }
    assert_eq!(
        builder.list_pages("gallery").unwrap(),
        [
            "gallery/index",
            "gallery/one-item/compared",
            "gallery/one-item/index"
        ]
    );
    assert_eq!(
        builder.list_pages("gallery/one-item").unwrap(),
        ["gallery/one-item/compared", "gallery/one-item/index"]
    );
    assert!(builder.list_pages("gallery/missing").unwrap().is_empty());
    assert_eq!(builder.list_pages("").unwrap().len(), 4);
    assert_eq!(
        builder.list_pages("../secrets").unwrap_err().to_string(),
        "Prefix would list outside content directory: ../secrets"
    );

    assert!(builder
        .emitted_routes()
        .contains("gallery/one-item/compared"));
    builder.remove_page("gallery/one-item/compared").unwrap();
    assert!(
        !build.join("content/gallery/one-item/compared.mdx").exists()
            && !build
                .join("public/_folioh/markdown/gallery/one-item/compared.md")
                .exists()
    );
    assert!(!builder
        .emitted_routes()
        .contains("gallery/one-item/compared"));
    builder.remove_page("nonexistent").unwrap();
}

#[test]
fn frontend_routes_share_the_authoring_and_link_checker_contract() {
    let dir = tempfile::tempdir().unwrap();
    let config = common::config(
        dir.path(),
        "template:\n  docs_route_base: /reference/docs\n",
    );
    let build = dir.path().join("build");
    common::write(&build, "lib/folioh-routes.json", "{\"foliohRoutes\":true}");
    let mut builder = common::builder(&config, &dir.path().join("template"), &build);
    let routes = [
        "index",
        "components/callout",
        "api-reference/index",
        "api-reference/rust/demo_module",
    ];
    for route in routes {
        builder.write_page(route, "# Page\n").unwrap();
    }
    let contract = builder
        .write_authoring_contract(&BTreeSet::new(), "now")
        .unwrap();
    let data_path = build.join("lib/folioh-routes.json");
    let data: serde_json::Value = serde_json::from_str(&read(&data_path)).unwrap();
    let public: serde_json::Value = serde_json::from_str(&read(&contract)).unwrap();
    assert_eq!(data["routes"], public["routes"]);
    assert_eq!(data["apiReference"], true);
    let helper = common::bundled_template().join("lib/docs-routes.js");
    if std::process::Command::new("node")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let script = "const {pathToFileURL}=await import('node:url'); const {docsRoute,defaultDocsRoutes}=await import(pathToFileURL(process.argv[1]).href); const d=JSON.parse(process.argv[2]); console.log(JSON.stringify({urls:JSON.parse(process.argv[3]).map(r=>docsRoute(r,d)),defaults:defaultDocsRoutes(d),withoutApi:defaultDocsRoutes({...d,apiReference:false})}))";
    let output = std::process::Command::new("node")
        .args(["--input-type=module", "-e", script])
        .arg(&helper)
        .arg(data.to_string())
        .arg(serde_json::to_string(&routes).unwrap())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    for (route, url) in routes.iter().zip(result["urls"].as_array().unwrap()) {
        assert_eq!(url, &builder.content_route_to_docs_url(route));
    }
    assert_eq!(result["defaults"].as_array().unwrap().len(), 3);
    assert_eq!(result["withoutApi"].as_array().unwrap().len(), 2);
    let links = result["urls"]
        .as_array()
        .unwrap()
        .iter()
        .map(|url| {
            format!(
                "[Page]({})\n<a href=\"{}\">Page</a>\n",
                url.as_str().unwrap(),
                url.as_str().unwrap()
            )
        })
        .collect::<String>();
    builder.write_page("index", &links).unwrap();
    let pages: Vec<String> = routes.iter().map(|r| r.to_string()).collect();
    assert!(
        folioh_site::links::check_links(&folioh_site::links::LinkCheckInput {
            content_dir: &build.join("content"),
            pages: &pages,
            docs_route_base: "/reference/docs",
            site_routes: &std::collections::HashSet::new(),
            static_root: None
        })
        .is_empty()
    );
    // Custom unmarked data remains under template ownership.
    std::fs::write(&data_path, "{\"custom\":true}").unwrap();
    builder
        .write_authoring_contract(&BTreeSet::new(), "later")
        .unwrap();
    assert_eq!(read(&data_path), "{\"custom\":true}");
}

#[test]
fn generated_text_files_keep_mtimes_when_bytes_are_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let config = common::config(dir.path(), "");
    let build = dir.path().join("build");
    let mut builder = common::builder(&config, &dir.path().join("template"), &build);
    let keys = BTreeSet::from(["project".to_string()]);
    let write_outputs = |builder: &mut SiteBuilder, generated_at: &str| {
        builder.write_page("guide", "# Guide\n").unwrap();
        builder
            .write_meta("", "export default { guide: \"Guide\" }\n")
            .unwrap();
        builder.write_search_index().unwrap();
        let mut manifest = Manifest::default();
        manifest.sources.insert("guide.md".into(), json!("abc"));
        builder.save_manifest(&manifest).unwrap();
        builder
            .write_llm_files(Some("# Test\n"), Some("# Test full\n"), false)
            .unwrap();
        builder
            .write_authoring_contract(&keys, generated_at)
            .unwrap();
    };
    write_outputs(&mut builder, "2026-09-10T10:00:00Z");
    let output = Path::new(&config.output_dir).to_path_buf();
    let paths = [
        build.join("content/guide.mdx"),
        build.join("content/_meta.ts"),
        build.join("lib/search-index.ts"),
        build.join(".folioh-manifest.json"),
        output.join("llms.txt"),
        output.join("llms-full.txt"),
        build.join("public/_folioh/contract.json"),
    ];
    let pinned: Vec<u128> = paths.iter().map(|p| common::pin_mtime(p)).collect();
    let mirror = build.join("public/_folioh/markdown/guide.md");
    std::fs::remove_file(&mirror).unwrap();
    write_outputs(&mut builder, "2026-09-10T10:00:01Z");
    assert_eq!(read(&mirror), "# Guide\n");
    for (path, before) in paths.iter().zip(pinned) {
        assert_eq!(
            common::mtime_ns(path),
            before,
            "{} was rewritten",
            path.display()
        );
    }
    let contract_path = build.join("public/_folioh/contract.json");
    let contract: serde_json::Value = serde_json::from_str(&read(&contract_path)).unwrap();
    assert_eq!(contract["generatedAt"], "2026-09-10T10:00:00Z");
    assert_eq!(contract["routes"], json!(["/docs/guide/"]));
    assert_eq!(contract["configKeys"], json!(["project"]));
    assert_eq!(contract["foliohVersion"], env!("CARGO_PKG_VERSION"));
    assert_eq!(contract["components"][0]["name"], "ParamTable");
    assert_eq!(
        read(&build.join(".folioh-manifest.json")),
        "{\n  \"sources\": {\n    \"guide.md\": \"abc\"\n  }\n}"
    );

    builder.register_route("new-page");
    builder
        .write_authoring_contract(&keys, "2026-09-10T10:00:02Z")
        .unwrap();
    let changed: serde_json::Value = serde_json::from_str(&read(&contract_path)).unwrap();
    assert_eq!(changed["generatedAt"], "2026-09-10T10:00:02Z");
    assert_eq!(
        changed["routes"],
        json!(["/docs/guide/", "/docs/new-page/"])
    );

    std::fs::write(&contract_path, "not json\n").unwrap();
    builder
        .write_authoring_contract(&keys, "2026-09-10T10:00:03Z")
        .unwrap();
    let replaced: serde_json::Value = serde_json::from_str(&read(&contract_path)).unwrap();
    assert_eq!(replaced["generatedAt"], "2026-09-10T10:00:03Z");
}

#[test]
fn manifest_round_trips_with_and_without_build_context() {
    let dir = tempfile::tempdir().unwrap();
    let config = common::config(dir.path(), "");
    let build = dir.path().join("build");
    let builder = common::builder(&config, &dir.path().join("template"), &build);
    assert_eq!(builder.load_manifest().unwrap(), Manifest::default());
    let ctx = folioh_site::template::build_manifest_context(
        &dir.path().join("docs.yaml"),
        dir.path(),
        "main",
        "",
        "/docs",
        "gen",
    );
    let mut manifest = Manifest {
        build: Some(ctx.clone()),
        sources: Default::default(),
    };
    manifest.sources.insert("/abs/src/pkg/mod.py".into(), json!({"hash": "h", "route": "api-reference/pkg/mod", "symbols": "s", "language": "python"}));
    builder.save_manifest(&manifest).unwrap();
    let text = read(&builder.manifest_path());
    assert!(
        text.starts_with("{\n  \"build\": {\n    \"config\": ")
            && !text.ends_with('\n')
            && !text.contains("backend")
    );
    assert_eq!(builder.load_manifest().unwrap(), manifest);
    std::fs::write(builder.manifest_path(), "{\"build\": {\"config\": \"c\", \"template\": \"t\", \"theme_package\": \"\", \"docs_route_base\": \"/docs\", \"generator\": \"g\", \"source_ref\": \"main\", \"experimental_features\": \"disabled\", \"backend\": \"python\"}, \"sources\": {}}").unwrap();
    assert_eq!(
        builder.load_manifest().unwrap().build.unwrap().generator,
        "g"
    );
}

#[test]
fn meta_files_round_trip_and_prune() {
    let dir = tempfile::tempdir().unwrap();
    let config = common::config(dir.path(), "");
    let build = dir.path().join("build");
    let builder = common::builder(&config, &dir.path().join("template"), &build);
    assert_eq!(builder.read_meta("api").unwrap(), "");
    builder
        .write_meta("", "{\"introduction\": \"Introduction\"}")
        .unwrap();
    builder
        .write_meta("api-reference", "{\"module\": \"module\"}")
        .unwrap();
    builder
        .write_meta("api", "export default { index: \"Overview\" }\n")
        .unwrap();
    assert!(
        build.join("content/_meta.ts").exists()
            && build.join("content/api-reference/_meta.ts").exists()
    );
    assert_eq!(
        builder.read_meta("api").unwrap(),
        "export default { index: \"Overview\" }\n"
    );
    assert_eq!(
        builder.write_meta("../x", "").unwrap_err().to_string(),
        "Directory would write outside content directory: ../x"
    );
    assert_eq!(
        builder.read_meta("../x").unwrap_err().to_string(),
        "Directory would access outside content directory: ../x"
    );

    builder
        .write_meta("plugins/gallery", "export default {}")
        .unwrap();
    common::write(&build, "content/plugins/gallery/card.png", "image");
    builder
        .prune_meta_files(&BTreeSet::from([
            "_meta.ts".to_string(),
            "api/_meta.ts".to_string(),
        ]))
        .unwrap();
    assert!(
        !build.join("content/plugins/gallery/_meta.ts").exists()
            && !build.join("content/api-reference/_meta.ts").exists()
    );
    assert!(
        build.join("content/plugins/gallery/card.png").exists()
            && build.join("content/api/_meta.ts").exists()
    );
    builder.remove_meta_tree("").unwrap();
    assert!(
        !build.join("content/_meta.ts").exists() && !build.join("content/api/_meta.ts").exists()
    );
    builder.remove_meta_tree("missing").unwrap();
}

#[test]
fn llm_files_and_robots_pointers() {
    let dir = tempfile::tempdir().unwrap();
    let config = common::config(
        dir.path(),
        "project:\n  name: MyLib\n  url: https://example.com/base\n",
    );
    let output = Path::new(&config.output_dir).to_path_buf();
    common::write(
        &output,
        "robots.txt",
        "User-Agent: *\nAllow: /\n\nSitemap: https://example.com/base/sitemap.xml\n",
    );
    let builder = common::builder(
        &config,
        &dir.path().join("template"),
        &dir.path().join("build"),
    );
    builder
        .write_llm_files(Some("# MyLib\n"), Some("# MyLib full\n"), false)
        .unwrap();
    assert!(output.join("llms.txt").exists() && output.join("llms-full.txt").exists());
    let robots = read(&output.join("robots.txt"));
    assert!(
        robots.starts_with("User-Agent: *\nAllow: /\n")
            && robots.contains("Sitemap: https://example.com/base/sitemap.xml\n")
    );
    assert!(
        robots.contains("# llms.txt: https://example.com/base/llms.txt\n")
            && robots.contains("# llms-full.txt: https://example.com/base/llms-full.txt\n")
    );
    builder
        .write_llm_files(Some("# MyLib\n"), Some("# MyLib full\n"), false)
        .unwrap();
    let robots = read(&output.join("robots.txt"));
    assert_eq!(robots.matches("# llms.txt:").count(), 1);
    assert_eq!(robots.matches("# llms-full.txt:").count(), 1);

    builder
        .write_llm_files(Some("# MyLib\n"), None, false)
        .unwrap();
    assert!(output.join("llms.txt").exists() && !output.join("llms-full.txt").exists());

    let plain = common::config(dir.path(), "project:\n  name: MyLib\noutput: out2\n");
    let builder = common::builder(
        &plain,
        &dir.path().join("template"),
        &dir.path().join("build2"),
    );
    builder
        .write_llm_files(Some("# MyLib\n"), None, false)
        .unwrap();
    assert!(
        dir.path().join("out2/llms.txt").exists() && !dir.path().join("out2/robots.txt").exists()
    );
    builder
        .write_llm_files(Some("# MyLib\n"), Some("full"), true)
        .unwrap();
    assert!(
        dir.path().join("build2/public/llms.txt").exists()
            && dir.path().join("build2/public/llms-full.txt").exists()
    );
}

#[test]
fn routes_are_a_copied_set_reset_by_prepare() {
    let dir = tempfile::tempdir().unwrap();
    let template = common::make_template(dir.path());
    let config = common::config(dir.path(), "");
    let mut builder = common::builder(&config, &template, &dir.path().join("build"));
    assert!(builder.emitted_routes().is_empty());
    builder.register_route("api-reference/http");
    let mut copy = builder.emitted_routes();
    copy.insert("mutated".into());
    assert_eq!(
        builder.emitted_routes(),
        BTreeSet::from(["api-reference/http".to_string()])
    );
    builder.write_page("roadmap", "# Roadmap\n").unwrap();
    assert_eq!(
        builder.emitted_routes(),
        BTreeSet::from(["api-reference/http".to_string(), "roadmap".to_string()])
    );
    let snapshot = BTreeSet::from(["only".to_string()]);
    builder.restore_emitted_routes(snapshot.clone());
    assert_eq!(builder.emitted_routes(), snapshot);
    let injection = builder.prepare(true).unwrap();
    assert!(builder.emitted_routes().is_empty());
    assert!(
        injection
            .injected_files
            .contains(&"app/layout.tsx".to_string())
            && injection
                .injected_files
                .contains(&"next.config.mjs".to_string())
    );
    assert!(dir.path().join("build/content").is_dir());
}

#[test]
fn page_assets_and_static_assets() {
    let dir = tempfile::tempdir().unwrap();
    let template = common::make_template(dir.path());
    let config = common::config(dir.path(), "");
    let build = dir.path().join("build");
    let mut builder = common::builder(&config, &template, &build);
    builder.prepare(false).unwrap();
    let source = dir.path().join("docs/guide");
    let shot = common::write(&source, "shot.png", "\u{89}PNG");
    builder
        .copy_page_asset("guide/page", "shot.png", &shot)
        .unwrap();
    let copied = build.join("content/guide/shot.png");
    assert!(copied.is_file());
    let pinned = common::pin_mtime(&copied);
    builder
        .copy_page_asset("guide/page", "shot.png", &shot)
        .unwrap();
    assert_eq!(common::mtime_ns(&copied), pinned);
    std::fs::write(&shot, b"changed image").unwrap();
    builder
        .copy_page_asset("guide/page", "shot.png", &shot)
        .unwrap();
    assert_eq!(std::fs::read(&copied).unwrap(), b"changed image");
    assert_eq!(
        builder.copy_page_asset("guide/page", "../../escape.png", &shot).unwrap_err().to_string(),
        "Asset would be written outside the content directory: ../../escape.png (from route guide/page)"
    );

    let page_dir = build.join("content/guide");
    common::write(&page_dir, "page.mdx", "# Page\n");
    common::write(&page_dir, "index.mdx", "# Index\n");
    common::write(&page_dir, "_meta.ts", "export default {}\n");
    let outside = dir.path().join("outside");
    common::write(&outside, "image.png", "outside");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, page_dir.join("linked")).unwrap();
    for relative in [
        "../escape.png",
        "page.mdx",
        "_meta.ts",
        "INDEX.MDX",
        "_META.TS",
    ] {
        assert!(
            builder.remove_page_asset("guide/owner", relative).is_err(),
            "{relative}"
        );
    }
    #[cfg(unix)]
    assert!(builder
        .remove_page_asset("guide/owner", "linked/image.png")
        .is_err());
    assert!(
        outside.join("image.png").exists()
            && page_dir.join("page.mdx").exists()
            && page_dir.join("_meta.ts").exists()
    );
    assert_eq!(
        builder
            .page_asset_path("guide/owner", "a.mdx")
            .unwrap_err()
            .to_string(),
        "Asset path is reserved for generated content: a.mdx"
    );

    let card = common::write(
        &dir.path().join("cards/one-item"),
        "prototype.html",
        "<p>x</p>",
    );
    builder
        .copy_static_asset("_folioh/gallery/one-item/prototype.html", &card)
        .unwrap();
    let published = build.join("public/_folioh/gallery/one-item");
    assert_eq!(read(&published.join("prototype.html")), "<p>x</p>");
    builder.remove_static_tree("_folioh/gallery").unwrap();
    assert!(!published.exists());
    assert_eq!(
        builder
            .copy_static_asset("../escape.html", &card)
            .unwrap_err()
            .to_string(),
        "Static asset would be written outside the public directory: ../escape.html"
    );
    assert_eq!(
        builder.remove_static_tree(".").unwrap_err().to_string(),
        "Static tree would be removed outside the public directory: ."
    );
    assert!(build.join("public").is_dir());

    common::write(dir.path(), "install.sh", "#!/bin/sh\necho hi\n");
    builder
        .copy_public_files(&[PublicFile {
            source: "install.sh".into(),
            dest: "install.sh".into(),
        }])
        .unwrap();
    assert_eq!(
        read(&build.join("public/install.sh")),
        "#!/bin/sh\necho hi\n"
    );
    let err = builder
        .copy_public_files(&[PublicFile {
            source: "missing.md".into(),
            dest: "x.md".into(),
        }])
        .unwrap_err()
        .to_string();
    assert!(err.starts_with("public file not found: "), "{err}");
}

#[test]
fn search_index_documents_urls_and_cache() {
    let dir = tempfile::tempdir().unwrap();
    let template = common::make_template(dir.path());
    let config = common::config(dir.path(), "");
    let build = dir.path().join("build");
    let mut builder = common::builder(&config, &template, &build);
    builder.prepare(false).unwrap();
    builder
        .write_page(
            "configuration",
            "---\ntitle: Configuration\n---\n\n# Configuration\n\nSearch settings.",
        )
        .unwrap();
    builder
        .write_page(
            "api-reference/mylib/core",
            "# mylib.core\n\nCore API reference.",
        )
        .unwrap();
    builder
        .write_page(
            "guide/index",
            "Just `code` and <Tag prop={x} /> {expr}\n\n```py\nhidden\n```\n",
        )
        .unwrap();
    builder.write_page("_draft", "# Draft").unwrap();
    builder.write_search_index().unwrap();
    let index = build.join("lib/search-index.ts");
    let content = read(&index);
    assert!(content.starts_with("export interface FoliohSearchDocument {\n  url: string\n  title: string\n  content: string\n}\n"));
    assert!(content.contains("export const foliohSearchDocuments: FoliohSearchDocument[] = [\n"));
    assert!(
        content.contains("\"/docs/configuration/\"")
            && content.contains("\"title\": \"Configuration\"")
            && content.contains("\"content\": \"Configuration Search settings.\"")
    );
    assert!(
        content.contains("\"/docs/api-reference/mylib/core/\"")
            && content.contains("\"mylib.core\"")
    );
    assert!(
        content.contains("\"url\": \"/docs/guide/\"")
            && content.contains("\"title\": \"guide\"")
            && content.contains("\"content\": \"Just code and\"")
    );
    assert!(!content.contains("Draft") && !content.contains("hidden"));

    let route_config = common::config(
        dir.path(),
        "template:\n  docs_route_base: /reference/docs\nsearch:\n  placeholder: 'Find \"API\"'\n",
    );
    let mut routed = common::builder(&route_config, &template, &build);
    routed.write_search_index().unwrap();
    let content = read(&index);
    assert!(content.contains(&format!(
        "export const foliohSearchPlaceholder = {}",
        serde_json::to_string("Find \"API\"").unwrap()
    )));
    assert!(
        content.contains("\"/reference/docs/configuration/\"")
            && !content.contains("\"/docs/configuration/\"")
    );

    let disabled = common::config(dir.path(), "search:\n  enabled: false\n");
    let mut off = common::builder(&disabled, &template, &build);
    off.write_search_index().unwrap();
    assert!(read(&index).ends_with("= []\n"));
}

#[test]
fn search_index_detects_replacement_route_base_and_toggling() {
    let dir = tempfile::tempdir().unwrap();
    let config = common::config(dir.path(), "project:\n  name: Search\n");
    let build = dir.path().join("build");
    let mut builder = common::builder(&config, &dir.path().join("template"), &build);
    builder.write_page("guide", "# Guide\n\nBefore.").unwrap();
    builder
        .write_page("stable", "# Stable\n\nStable body.")
        .unwrap();
    builder.write_search_index().unwrap();
    let page = build.join("content/guide.mdx");
    let index = build.join("lib/search-index.ts");
    let before = common::mtime_ns(&index);
    builder.write_search_index().unwrap();
    assert_eq!(common::mtime_ns(&index), before);

    // A plugin may replace a page behind the builder's back: same size and
    // mtime, new inode and ctime.
    let old_mtime = std::fs::metadata(&page).unwrap().modified().unwrap();
    let replacement = build.join("content/guide.replacement");
    std::fs::write(&replacement, "# Guide\n\nAfter!.").unwrap();
    std::fs::File::options()
        .write(true)
        .open(&replacement)
        .unwrap()
        .set_modified(old_mtime)
        .unwrap();
    std::fs::rename(&replacement, &page).unwrap();
    builder.write_search_index().unwrap();
    let current = read(&index);
    assert!(current.contains("After!.") && !current.contains("Before."));

    std::fs::remove_file(&page).unwrap();
    builder.write_page("added", "# Added\n\nNew body.").unwrap();
    let routed_config = common::config(
        dir.path(),
        "project:\n  name: Search\ntemplate:\n  docs_route_base: /reference\n",
    );
    let mut routed = common::builder(&routed_config, &dir.path().join("template"), &build);
    routed.write_search_index().unwrap();
    let current = read(&index);
    assert!(
        current.contains("\"/reference/added/\"")
            && current.contains("\"/reference/stable/\"")
            && !current.contains("After!.")
    );
    let mut fresh = common::builder(&routed_config, &dir.path().join("template"), &build);
    fresh.write_search_index().unwrap();
    assert_eq!(read(&index), current);

    let disabled = common::config(dir.path(), "project:\n  name: Search\nsearch:\n  enabled: false\ntemplate:\n  docs_route_base: /reference\n");
    let mut off = common::builder(&disabled, &dir.path().join("template"), &build);
    off.write_search_index().unwrap();
    assert!(!read(&index).contains("Stable body."));
    routed.write_search_index().unwrap();
    assert_eq!(read(&index), current);
}

#[test]
fn preview_examples_are_built_from_folioh_projects_only() {
    let dir = tempfile::tempdir().unwrap();
    let config = common::config(dir.path(), "");
    let build = dir.path().join("build");
    let builder = common::builder(&config, &dir.path().join("template"), &build);
    let examples = dir.path().join("docs/examples");
    let handwritten = examples.join("hand-preview");
    common::write(&handwritten, "preview.html", "<!doctype html>");
    common::write(&handwritten, "files/docs.yaml", "project:\n  name: Tiny\n");
    let sample = examples.join("sample-preview");
    common::write(&sample, "docs.yaml", "project:\n  name: Demo\nsource:\n  python:\n    paths:\n      - src/demo\n  docs:\n    - docs\n");
    common::write(&sample, "docs/index.md", "# Demo docs\n");
    common::write(
        &sample,
        "src/demo/core.py",
        "def add(a, b):\n    return a + b\n",
    );
    common::write(
        &sample,
        "preview.html",
        "<!doctype html><title>Old design reference</title>",
    );
    common::write(&sample, ".build/stale.txt", "x");
    common::write(&examples, "bad name/docs.yaml", "project:\n  name: Bad\n");
    common::write(&build, "public/_folioh/examples/stale/index.html", "old");

    let mut calls = Vec::new();
    let env = common::env_lock();
    builder
        .write_preview_examples(&examples, &mut |request: PreviewBuildRequest| {
            std::fs::create_dir_all(&request.output_dir).unwrap();
            std::fs::write(
                request.output_dir.join("index.html"),
                "<!doctype html><main>Generated by Folioh</main>",
            )
            .unwrap();
            calls.push(request);
            Ok(())
        })
        .unwrap();
    drop(env);
    let output = build.join("public/_folioh/examples/sample-preview");
    assert_eq!(
        calls,
        vec![PreviewBuildRequest {
            project_dir: sample.clone(),
            output_dir: output.clone(),
            build_dir: build.join(".preview-examples/sample-preview"),
            base_path: "/_folioh/examples/sample-preview".to_string(),
        }]
    );
    assert!(
        !build.join("public/_folioh/examples/hand-preview").exists()
            && !build.join("public/_folioh/examples/stale").exists()
    );
    assert!(
        read(&output.join("index.html")).contains("Generated by Folioh")
            && !output.join("preview.html").exists()
    );
    let manifest: serde_json::Value =
        serde_json::from_str(&read(&output.join("manifest.json"))).unwrap();
    assert_eq!(
        manifest["files"],
        json!([
            {"path": "docs/index.md", "url": "/_folioh/examples/sample-preview/files/docs/index.md", "language": "markdown"},
            {"path": "docs.yaml", "url": "/_folioh/examples/sample-preview/files/docs.yaml", "language": "yaml"},
            {"path": "src/demo/core.py", "url": "/_folioh/examples/sample-preview/files/src/demo/core.py", "language": "python"},
        ])
    );
    // The digest is what lets the next build skip this example; its value is
    // the tree's, so the shape is what a test can pin.
    let digest = manifest["digest"].as_str().unwrap();
    assert!(
        digest.len() == 64 && digest.chars().all(|c| c.is_ascii_hexdigit()),
        "{digest}"
    );
    assert!(read(&output.join("manifest.json")).ends_with("}\n"));
    assert_eq!(read(&output.join("files/docs/index.md")), "# Demo docs\n");

    let example_build = build.join(".preview-examples/sample-preview");
    for name in [".next", "content", "out", "public"] {
        common::write(&example_build, &format!("{name}/stale.txt"), "stale");
    }
    common::write(&example_build, ".folioh-manifest.json", "{}");
    common::write(&example_build, ".folioh-build.log", "old log");
    common::write(&example_build, ".folioh-deps.hash", "deps");
    std::fs::create_dir_all(example_build.join("node_modules")).unwrap();
    SiteBuilder::reset_preview_example_workspace(&example_build).unwrap();
    for name in [
        ".next",
        "content",
        "out",
        "public",
        ".folioh-manifest.json",
        ".folioh-build.log",
    ] {
        assert!(!example_build.join(name).exists(), "{name}");
    }
    assert_eq!(read(&example_build.join(".folioh-deps.hash")), "deps");
    assert!(example_build.join("node_modules").is_dir());
}

#[test]
fn bundled_generated_site_example_is_a_folioh_project() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let guides = folioh_site::fs::files_under(&repo.join("docs/guide"));
    let docs_text: String = guides
        .iter()
        .filter(|p| p.extension().map(|e| e == "md").unwrap_or(false))
        .map(|p| read(p))
        .collect::<Vec<_>>()
        .join("\n");
    let mut names: Vec<String> = regex::Regex::new(r#"example="([^"]+)""#)
        .unwrap()
        .captures_iter(&docs_text)
        .map(|c| c[1].to_string())
        .collect();
    names.sort();
    names.dedup();
    assert_eq!(
        names,
        ["generated-site", "javascript", "landing-page", "rust"]
    );
    for name in &names {
        assert!(
            repo.join("docs/examples")
                .join(name)
                .join("docs.yaml")
                .is_file(),
            "missing example: {name}"
        );
    }
    let example = repo.join("docs/examples/generated-site");
    assert!(example.join("docs.yaml").is_file());
    let mut files: Vec<String> = SiteBuilder::preview_example_source_paths(&example)
        .iter()
        .map(|p| folioh_site::fs::posix(p.strip_prefix(&example).unwrap()))
        .collect();
    files.sort();
    assert_eq!(
        files,
        [
            "docs.yaml",
            "docs/cli.md",
            "docs/components.md",
            "docs/index.md",
            "src/example_package/__init__.py",
            "src/example_package/arithmetic.py"
        ]
    );
    let text: String = SiteBuilder::preview_example_source_paths(&example)
        .iter()
        .map(|p| read(p))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !text.contains("TinyMath")
            && !text.contains("tinymath")
            && text.contains("Example docs")
            && !text.contains("Compiled example")
    );
    for needle in ["Guide", "CLI", "API reference", "Components"] {
        assert!(text.contains(needle), "{needle}");
    }
    let raw: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&read(&example.join("docs.yaml"))).unwrap();
    assert_eq!(raw["landing"]["enabled"], serde_yaml_ng::Value::Bool(false));
    assert_eq!(
        raw["landing"]["comparison"],
        serde_yaml_ng::Value::Bool(false)
    );
}

/// An example is a whole nested build. Rebuilding one nobody touched is the
/// most expensive thing a warm build can do, so the digest decides.
#[test]
fn an_unchanged_preview_example_is_not_rebuilt() {
    let dir = tempfile::tempdir().unwrap();
    let config = common::config(dir.path(), "project:\n  name: MyLib\n");
    let build = dir.path().join("build");
    let builder = common::builder(&config, &dir.path().join("template"), &build);

    let examples = dir.path().join("docs/examples");
    let example = examples.join("sample");
    std::fs::create_dir_all(&example).unwrap();
    std::fs::write(example.join("docs.yaml"), "project:\n  name: Sample\n").unwrap();
    std::fs::create_dir_all(example.join("docs")).unwrap();
    std::fs::write(example.join("docs/index.md"), "# Sample\n").unwrap();

    // The nested build is the expensive half; stand in for it and count calls.
    let built = std::cell::RefCell::new(Vec::<String>::new());
    let mut run = |request: folioh_site::builder::PreviewBuildRequest| {
        let name = request
            .project_dir
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        built.borrow_mut().push(name);
        std::fs::create_dir_all(&request.output_dir).unwrap();
        std::fs::write(request.output_dir.join("index.html"), "<html></html>").unwrap();
        Ok(())
    };

    let _env = common::env_lock();
    let first = builder.write_preview_examples(&examples, &mut run).unwrap();
    assert_eq!((first.built, first.reused, first.swept), (1, 0, 0));

    let second = builder.write_preview_examples(&examples, &mut run).unwrap();
    assert_eq!(
        (second.built, second.reused, second.swept),
        (0, 1, 0),
        "nothing moved, so nothing is built"
    );

    // A source edit moves the digest and the example comes back.
    std::fs::write(example.join("docs/index.md"), "# Sample, edited\n").unwrap();
    let third = builder.write_preview_examples(&examples, &mut run).unwrap();
    assert_eq!((third.built, third.reused, third.swept), (1, 0, 0));
    assert_eq!(
        built.borrow().as_slice(),
        ["sample".to_string(), "sample".to_string()],
        "two builds, not three"
    );

    // A deleted example takes its published output with it.
    std::fs::remove_dir_all(&example).unwrap();
    let fourth = builder.write_preview_examples(&examples, &mut run).unwrap();
    assert_eq!((fourth.built, fourth.reused, fourth.swept), (0, 0, 1));
    assert!(!build.join("public/_folioh/examples/sample").exists());
}

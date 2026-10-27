//! The real toolchain end to end on `docs/examples/generated-site`:
//! bundled template, pnpm install, next build, static export, rewriter, LLM
//! files. Ignored by default (network + minutes); run locally with
//! `cargo test -p folioh-site --test e2e_export -- --ignored --nocapture`.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::time::Instant;

use folioh_site::builder::SiteBuilder;
use folioh_site::runtime::NextRuntime;
use folioh_site::sidebar::{generate_meta_files, read_directory_orders, SidebarInput, SidebarPage};
use folioh_site::template::resolve_template_dir;

#[test]
#[ignore = "runs pnpm install and next build on the generated-site example; run locally with --ignored"]
fn generated_site_example_exports_end_to_end() {
    let started = Instant::now();
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("generated-site");
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/examples/generated-site");
    folioh_site::fs::copy_tree_all(&source, &project).unwrap();
    let guides = [
        (
            "i18n.md",
            "Project Languages",
            "This authored language guide is not an experimental feature.",
            &["i18n"][..],
        ),
        (
            "versioning.md",
            "Project Versions",
            "This authored version guide is not an experimental feature.",
            &["versioning"][..],
        ),
        (
            "common_errors.md",
            "Common Errors",
            "An underscore guide renders its own content.",
            &["common-errors", "common_errors"][..],
        ),
        (
            "nested_errors/index.md",
            "Nested Errors",
            "An underscore directory index renders its own content.",
            &["nested-errors", "nested_errors"][..],
        ),
        (
            "mixed_folder/quick-start.md",
            "Mixed Path",
            "Mixed underscores and hyphens keep their source route.",
            &[
                "mixed-folder/quick-start",
                "mixed_folder/quick-start",
                "mixed_folder/quick_start",
            ][..],
        ),
        (
            "api-reference/example_package/module_name.md",
            "API Names",
            "API module names retain their underscores.",
            &["api-reference/example_package/module_name"][..],
        ),
    ];
    for (path, title, body, _) in &guides {
        common::write(
            &project.join("docs"),
            path,
            &format!("# {title}\n\n{body}\n"),
        );
    }
    // Give the index a visible sibling: Nextra's independent empty-folder
    // breadcrumb bug must not mask the route-lookup regression.
    common::write(
        &project.join("docs"),
        "nested_errors/recovery.md",
        "# Recovery\n\nRecovery steps.\n",
    );
    let intro = project.join("docs/index.md");
    std::fs::write(
        &intro,
        format!(
            "{}\n<a data-navigation-probe=\"first\" href=\"/docs/cli/\">CLI navigation probe</a>\n",
            common::read(&intro)
        ),
    )
    .unwrap();
    let loaded = folioh_config::load_docs_config(&project.join("docs.yaml")).unwrap();
    let mut config = loaded.config;
    let logo = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"40\" height=\"40\"><circle cx=\"20\" cy=\"20\" r=\"18\" fill=\"#6750a4\"/></svg>";
    common::write(&project, "assets/logo mark.svg", logo);
    config.theme.logo = "assets/logo mark.svg".into();
    folioh_plugins::Plugin::configure(
        &folioh_plugins::LandingPlugin,
        &mut config,
        &loaded.raw,
        &mut Vec::new(),
    )
    .unwrap();
    let config = config.resolve_paths(&project).unwrap();
    let build_dir = project.join(".build");
    let template = resolve_template_dir(&config, &build_dir).unwrap();
    let mut builder = SiteBuilder::new(
        &config,
        &template.dir,
        &build_dir,
        Box::new(NextRuntime::default()),
    );
    let injection = builder.prepare(true).unwrap();
    assert!(injection.warnings.is_empty(), "{:?}", injection.warnings);
    // Render the public component through Next, including an arbitrary key.
    common::write(
        &build_dir,
        "app/roadmap-icon-probe/page.tsx",
        r#"
import { ProjectIcon } from "@/components/roadmap-header"
export default function IconProbe() {
  return <main>{["docs", "orion", "agents"].map(key => <ProjectIcon key={key} projectKey={key} className={`probe-${key}`} />)}</main>
}
"#,
    );

    let scan = folioh_mdx::parse_markdown_directory(&project.join("docs"), "").unwrap();
    for page in &scan.pages {
        builder
            .write_page(&page.route, &folioh_mdx::markdown_to_mdx(page))
            .unwrap();
    }
    let sidebar_pages: Vec<SidebarPage> = scan
        .pages
        .iter()
        .map(|p| SidebarPage {
            route: &p.route,
            title: p.frontmatter.get("title").and_then(|t| t.as_str()),
            unlisted: p.unlisted,
        })
        .collect();
    let orders = read_directory_orders(&config.source.docs).unwrap();
    let meta = generate_meta_files(&SidebarInput {
        nav: &config.nav,
        modules: &[],
        pages: &sidebar_pages,
        default_collapsed: config.sidebar.default_collapsed,
        directory_orders: &orders,
    });
    for (path, text) in &meta {
        let directory = path.strip_suffix("/_meta.ts").unwrap_or("");
        builder.write_meta(directory, text).unwrap();
    }
    builder.write_search_index().unwrap();
    let keys: BTreeSet<String> = folioh_plugins::contract_config_keys()
        .into_iter()
        .map(str::to_string)
        .collect();
    builder
        .write_authoring_contract(&keys, "2026-09-18T00:00:00Z")
        .unwrap();
    let prepared = started.elapsed();

    let installed = builder.install_deps(&mut |_| {}).unwrap().installed;
    let after_install = started.elapsed();
    let export = builder
        .export_static_site(
            Some("# Example Package\n"),
            Some("# Example Package full\n"),
        )
        .unwrap();
    let total = started.elapsed();
    eprintln!(
        "prepare {prepared:?}; install (ran: {installed}) {:?}; export {:?}; total {total:?}",
        after_install - prepared,
        total - after_install
    );
    eprintln!("warnings: {:?}", export.warnings);
    eprintln!(
        "last build lines: {:?}",
        export.output_lines.iter().rev().take(5).collect::<Vec<_>>()
    );

    let out = Path::new(&config.output_dir);
    let probe = common::read(&out.join("roadmap-icon-probe/index.html"));
    let icons = regex::Regex::new(
        r#"(?s)<svg\b[^>]*class="[^"]*probe-(docs|orion|agents)[^"]*"[^>]*>(.*?)</svg>"#,
    )
    .unwrap();
    let rendered: std::collections::BTreeMap<_, _> = icons
        .captures_iter(&probe)
        .map(|c| (c[1].to_string(), c[2].to_string()))
        .collect();
    assert_eq!(rendered.len(), 3, "all project marks must render");
    assert!(rendered["docs"].contains("M4 2h5l3 3v9H4z") && rendered["docs"].contains("M9 2v3h3"));
    assert_eq!(rendered["orion"], rendered["agents"]);
    assert!(
        rendered["orion"].contains(r#"<circle cx="8" cy="8" r="3.25""#)
            && !rendered["orion"].contains("<rect")
    );
    assert_eq!(common::read(&out.join("logo mark.svg")), logo);
    let logo_tag = regex::Regex::new(r#"<img\b[^>]*src="[^"]*logo%20mark\.svg"[^>]*>"#).unwrap();
    for page in ["docs/index.html", "previews/index.html"] {
        let html = common::read(&out.join(page));
        assert!(
            logo_tag.is_match(&html),
            "project logo is absent from {page}"
        );
    }
    for rel in [
        "index.html",
        "docs/index.html",
        "docs/cli/index.html",
        "docs/components/index.html",
        "_folioh/markdown/index.md",
        "_folioh/markdown/cli.md",
        "_folioh/contract.json",
        "llms.txt",
        "llms-full.txt",
        "robots.txt",
        "sitemap.xml",
        "_pagefind/pagefind.js",
        "_folioh-search.js",
    ] {
        assert!(out.join(rel).exists(), "missing {rel}");
    }
    // Every page refers to the same physical, versioned script, including
    // file:// targets after the export rewriter has rebased their URLs.
    let boot_tag = regex::Regex::new(
        r#"(?s)<script\b([^>]*id="theme-configurator-boot"[^>]*)>(.*?)</script>"#,
    )
    .unwrap();
    let boot_src = regex::Regex::new(r#"src="([^"]+)""#).unwrap();
    let mut shared = None;
    for page in ["index.html", "docs/index.html", "docs/cli/index.html"] {
        let html = common::read(&out.join(page));
        let tag = boot_tag.captures(&html).expect("theme script");
        assert!(tag[2].is_empty(), "theme payload repeated in {page}");
        let attrs = boot_src.captures(&tag[1]).unwrap();
        let src = &attrs[1];
        assert!(
            !src.starts_with('/'),
            "file export needs a relative script: {src}"
        );
        let asset = out
            .join(page)
            .parent()
            .unwrap()
            .join(src)
            .canonicalize()
            .unwrap();
        assert!(asset.is_file() && asset.starts_with(out.canonicalize().unwrap()));
        assert!(asset
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("bootstrap-"));
        assert_eq!(asset.extension().unwrap(), "js");
        assert!(common::read(&asset).contains("const presets ="));
        if let Some(previous) = &shared {
            assert_eq!(previous, &asset);
        }
        shared = Some(asset);
    }
    let robots = common::read(&out.join("robots.txt"));
    assert!(
        robots.contains("# llms.txt: /llms.txt\n")
            && robots.contains("# llms-full.txt: /llms-full.txt\n"),
        "{robots}"
    );
    assert_eq!(export.llm_files, ["llms.txt", "llms-full.txt"]);
    let main = regex::Regex::new(r"(?s)<main\b[^>]*>(.*?)</main>").unwrap();
    let scripts = regex::Regex::new(r"(?s)<script\b[^>]*>.*?</script>").unwrap();
    for (mirror, title, body, routes) in guides {
        for route in routes {
            let html = common::read(&out.join(format!("docs/{route}/index.html")));
            assert!(
                html.contains(&format!("<title>{title} - Example Package</title>")),
                "wrong rendered title at {route}"
            );
            let visible = main
                .captures(&html)
                .unwrap_or_else(|| panic!("missing main at {route}"));
            assert!(
                scripts.replace_all(&visible[1], "").contains(body),
                "missing rendered body at {route}"
            );
            assert!(
                html.contains(&format!("_folioh/markdown/{mirror}")),
                "missing Markdown alternate at {route}"
            );
            assert!(out.join(format!("_folioh/markdown/{mirror}")).is_file());
        }
    }
    assert!(!out
        .join("docs/api-reference/example-package/module-name/index.html")
        .exists());
    let home = common::read(&out.join("docs/cli/index.html"));
    assert!(home.contains("Example Package") && !home.contains("__PROJECT_NAME__"));
    assert!(
        home.contains("src=\"../../_next/") || home.contains("src=\"./_next/"),
        "relative chunk srcs expected"
    );
    // The Nextra classes `template/lib/nextra-dom.ts` finds its hooks by. A
    // Nextra upgrade that renames one fails here, not silently in a browser.
    for class in ["nextra-code", "nextra-hamburger", "nextra-mobile-nav"] {
        let renders = home
            .split(|c: char| c == '"' || c.is_whitespace())
            .any(|token| token == class);
        assert!(renders, "{class} no longer renders");
    }
    // The search results reach the page only once a search opens, and the
    // template's own chunks name the class, so it is looked for in Nextra.
    // The same holds for headless-ui's portal root, which holds them.
    let ships = |dist: &Path, needle: &str| {
        folioh_site::fs::files_under(dist)
            .iter()
            .filter(|path| {
                path.extension()
                    .is_some_and(|ext| ext == "js" || ext == "cjs" || ext == "css")
            })
            .any(|path| {
                std::fs::read_to_string(path)
                    .unwrap_or_default()
                    .contains(needle)
            })
    };
    let nextra = build_dir.join("node_modules/nextra/dist");
    assert!(
        ships(&nextra, "\"nextra-search-results\""),
        "nextra-search-results no longer renders"
    );
    let theme_package = build_dir.join("node_modules/nextra-theme-docs");
    // headless-ui is nextra-theme-docs' dependency, so it resolves from there.
    let headless = std::fs::canonicalize(&theme_package)
        .expect("nextra-theme-docs installed")
        .parent()
        .expect("inside node_modules")
        .join("@headlessui/react/dist");
    assert!(
        ships(&headless, "\"headlessui-portal-root\""),
        "headlessui-portal-root no longer renders"
    );
    let theme = theme_package.join("dist");
    let holds = |file: &str, needle: &str| {
        std::fs::read_to_string(theme.join(file))
            .unwrap_or_default()
            .contains(needle)
    };
    // The drawer closes at Nextra's `md`: the mobile menu hides through
    // `x:md:hidden`, and that utility is a `(width >= 48rem)` query.
    let style: String = std::fs::read_to_string(theme.join("style.css"))
        .unwrap_or_default()
        .split_whitespace()
        .collect();
    assert!(
        holds("components/sidebar.js", "\"nextra-mobile-nav\"")
            && holds("components/sidebar.js", "\"x:md:hidden\"")
            && style.contains(".x\\:md\\:hidden{@media(width>=48rem){display:none"),
        "Nextra's mobile menu no longer hides at 48rem"
    );
    // The Nextra utility classes the docs CSS selects on, where Nextra gives
    // the element no class or attribute of its own: the Collapse that opens a
    // folder (shell.css, and the drawer in globals.css), the current table of
    // contents entry (shell.css) and the Details behind an MDX `<details>`
    // (disclosure.css). Renamed, those rules would stop matching silently.
    for class in ["x:transform-gpu", "x:not-first:mt-4"] {
        assert!(ships(&nextra, class), "{class} no longer renders");
    }
    assert!(
        holds("components/toc.js", "x:text-primary-600"),
        "x:text-primary-600 no longer renders"
    );
    assert!(build_dir.join(".folioh-build.log").is_file() && !export.output_lines.is_empty());
    if let Ok(module) = std::env::var("FOLIOH_PLAYWRIGHT_MODULE") {
        let browser = std::process::Command::new("node")
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/browser_navigation.cjs"))
            .arg(out)
            .arg("fixture")
            .env("FOLIOH_PLAYWRIGHT_MODULE", module)
            .output()
            .expect("Playwright driver starts");
        assert!(
            browser.status.success(),
            "browser regression failed:\n{}\n{}",
            String::from_utf8_lossy(&browser.stdout),
            String::from_utf8_lossy(&browser.stderr)
        );
        eprintln!(
            "Browser navigation: {}",
            String::from_utf8_lossy(&browser.stdout)
        );
    } else {
        eprintln!("Playwright navigation not run: set FOLIOH_PLAYWRIGHT_MODULE to its installed module path.");
    }
}

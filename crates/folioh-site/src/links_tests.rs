use super::*;

#[test]
fn checked_and_rendered_relative_routes_are_the_same() {
    for (href, source, expected) in [
        (
            "steps/../intro.md?tab=x#heading",
            "guide/setup",
            "/manual/guide/intro?tab=x#heading",
        ),
        ("../index.mdx", "guide/setup", "/manual"),
        (
            "./api-reference/pkg_common/index.mdx",
            "index",
            "/manual/api-reference/pkg_common",
        ),
    ] {
        let rendered = resolve_href(href, source, "/manual").unwrap();
        assert_eq!(rendered, expected);
        assert_eq!(
            normalize_target(href, source, "/manual"),
            normalize_target(&rendered, source, "/manual")
        );
    }
}

#[test]
fn normalize_target_cases() {
    let cases: [(&str, &str, &str, Option<&str>); 21] = [
        ("https://example.com", "index", "/docs", None),
        ("http://example.com", "index", "/docs", None),
        ("mailto:user@example.com", "index", "/docs", None),
        ("#section", "index", "/docs", None),
        ("?tab=x", "index", "/docs", None),
        ("/docs/installation", "index", "/docs", Some("installation")),
        (
            "/docs/api-reference/folioh/config",
            "index",
            "/docs",
            Some("api-reference/folioh/config"),
        ),
        ("/docs", "index", "/docs", Some("index")),
        ("/docs/", "index", "/docs", Some("index")),
        (
            "/reference/docs/api-reference/folioh/config",
            "index",
            "/reference/docs",
            Some("api-reference/folioh/config"),
        ),
        ("./installation", "index", "/docs", Some("installation")),
        ("installation", "index", "/docs", Some("installation")),
        ("config", "guide/setup", "/docs", Some("guide/config")),
        ("./config", "guide/setup", "/docs", Some("guide/config")),
        (
            "../installation",
            "guide/setup",
            "/docs",
            Some("installation"),
        ),
        ("./installation.md", "index", "/docs", Some("installation")),
        ("./installation.mdx", "index", "/docs", Some("installation")),
        (
            "installation#quickstart",
            "index",
            "/docs",
            Some("installation"),
        ),
        (
            "installation?tab=linux",
            "index",
            "/docs",
            Some("installation"),
        ),
        ("page#section", "index", "/docs", Some("page")),
        ("/roadmap/", "index", "", Some("/roadmap")),
    ];
    for (href, source, base, expected) in cases {
        assert_eq!(
            normalize_target(href, source, base).as_deref(),
            expected,
            "{href} from {source}"
        );
    }
    assert_eq!(
        normalize_target("/", "index", "/docs").as_deref(),
        Some("/")
    );
    assert_eq!(
        normalize_target("/../secret.txt", "index", "/docs").as_deref(),
        Some("/../secret.txt")
    );
    // A `..` that climbs above the docs root stays as written, which matches
    // no page, so the checker reports it instead of clamping it to the root.
    for (href, source) in [
        ("../", "index"),
        ("../api-reference/javascript/utils#engine", "index"),
        ("../../installation", "guide/setup"),
    ] {
        let raw = href.split('#').next().unwrap();
        assert_eq!(
            normalize_target(href, source, "/docs").as_deref(),
            Some(raw),
            "{href} from {source}"
        );
    }
    assert_eq!(
        normalize_target("../../x", "a/b/c", "/docs").as_deref(),
        Some("x")
    );
    assert_eq!(
        normalize_target("a.md.mdx", "index", "/docs").as_deref(),
        Some("a")
    );
    assert_eq!(
        normalize_target("//host/path", "index", "/docs").as_deref(),
        Some("/host/path")
    );
}

#[test]
fn a_relative_link_above_the_docs_root_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("guide")).unwrap();
    std::fs::write(
        dir.path().join("index.mdx"),
        "# Home\n[Up](../api-reference/javascript/utils#engine)\n[Guide](./guide/setup)\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("guide/setup.mdx"),
        "# Setup\n[Home](../index)\n[Out](../../index)\n",
    )
    .unwrap();
    let pages = [
        "index".to_string(),
        "guide/setup".to_string(),
        "api-reference/javascript/utils".to_string(),
    ];
    let site_routes = HashSet::new();
    let broken = check_links(&LinkCheckInput {
        content_dir: dir.path(),
        pages: &pages,
        docs_route_base: "/docs",
        site_routes: &site_routes,
        static_root: None,
    });
    let mut found: Vec<(&str, &str)> = broken
        .iter()
        .map(|b| (b.source_page.as_str(), b.target.as_str()))
        .collect();
    found.sort_unstable();
    assert_eq!(
        found,
        [
            ("guide/setup", "../../index"),
            ("index", "../api-reference/javascript/utils#engine"),
        ]
    );
}

#[test]
fn links_in_line_extracts_markdown_then_attributes_and_skips_code_and_images() {
    assert_eq!(
        links_in_line("See [a](./a) and [b](./b) <X href=\"/x\" /> <Y href='/y'/>"),
        vec!["./a", "./b", "/x", "/y"]
    );
    assert!(links_in_line("Example: `![path](path)` and `[x](./nope)`").is_empty());
    assert!(
        links_in_line("![The diagram](./diagram.png) ![Remote](https://x.test/a.png)").is_empty()
    );
    assert_eq!(links_in_line("[a](./x \"title\")"), vec!["./x \"title\""]);
    assert!(links_in_line("<A href={dynamic} />").is_empty());
}

#[test]
fn splitlines_matches_python() {
    assert_eq!(splitlines("a\nb\r\nc\rd"), vec!["a", "b", "c", "d"]);
    assert_eq!(splitlines("a\n"), vec!["a"]);
    assert_eq!(splitlines("a\u{2028}b"), vec!["a", "b"]);
    assert_eq!(percent_decode("a%20b%zz%C3%A9"), "a b%zzé");
}

#[test]
fn links_in_line_reads_href_object_props() {
    assert_eq!(
        links_in_line("<Hero items={[{ href: \"/docs\" }, { href:'./llms.txt' }]} />"),
        vec!["/docs", "./llms.txt"]
    );
    assert!(links_in_line("`{ href: \"/nope\" }`").is_empty());
}

#[test]
fn resolve_href_reads_the_href_against_the_page_file() {
    let cases: [(&str, &str, Option<&str>); 16] = [
        ("./installation", "quickstart", Some("/docs/installation")),
        ("installation", "quickstart", Some("/docs/installation")),
        (
            "./ci-cd",
            "deployment/github-pages",
            Some("/docs/deployment/ci-cd"),
        ),
        (
            "personalization",
            "theming/index",
            Some("/docs/theming/personalization"),
        ),
        (
            "../configuration#public",
            "plugins/catalog",
            Some("/docs/configuration#public"),
        ),
        (
            "./plugins/authoring",
            "configuration",
            Some("/docs/plugins/authoring"),
        ),
        ("./components/index", "index", Some("/docs/components")),
        ("./index.md", "index", Some("/docs")),
        ("../", "guide/setup", Some("/docs")),
        (
            "./setup.mdx?tab=a#b",
            "guide/index",
            Some("/docs/guide/setup?tab=a#b"),
        ),
        ("../x", "index", None),
        ("/docs/installation", "quickstart", None),
        ("https://example.com", "quickstart", None),
        ("mailto:a@b.c", "quickstart", None),
        ("#section", "quickstart", None),
        ("//host/path", "quickstart", None),
    ];
    for (href, route, expected) in cases {
        assert_eq!(
            resolve_href(href, route, "/docs").as_deref(),
            expected,
            "{href} from {route}"
        );
    }
    assert_eq!(
        resolve_href("./a", "guide/setup", "/reference/docs/").as_deref(),
        Some("/reference/docs/guide/a")
    );
    assert_eq!(resolve_href("./a", "index", "").as_deref(), Some("/docs/a"));
}

#[test]
fn resolve_relative_links_rewrites_links_and_leaves_code_as_written() {
    let page = "---\ntitle: \"[x](./front)\"\n---\n\
See [Installation](./installation \"Install\") and [CLI](../cli#flags).\n\
![Diagram](./diagram.png) `[inline](./inline)`\n\
<FeatureCard href=\"setup\" /> <Hero items={[{ href: './setup' }]} />\n\
[Site](/roadmap) [Out](https://example.com)\n\
```md\n[fenced](./fenced)\n```\n";
    assert_eq!(
        resolve_relative_links(page, "guide/intro", "/docs"),
        "---\ntitle: \"[x](./front)\"\n---\n\
See [Installation](/docs/guide/installation \"Install\") and [CLI](/docs/cli#flags).\n\
![Diagram](./diagram.png) `[inline](./inline)`\n\
<FeatureCard href=\"/docs/guide/setup\" /> <Hero items={[{ href: '/docs/guide/setup' }]} />\n\
[Site](/roadmap) [Out](https://example.com)\n\
```md\n[fenced](./fenced)\n```\n"
    );
    let untouched = "# Title\r\n\r\nNo links here.";
    assert_eq!(
        resolve_relative_links(untouched, "index", "/docs"),
        untouched
    );
}

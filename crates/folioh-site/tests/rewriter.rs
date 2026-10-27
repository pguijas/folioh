//! The static export rewriter over a fake `out/` tree.

mod common;

use std::path::Path;

use folioh_site::rewriter::StaticAssetRewriter;

fn read(path: &Path) -> String {
    common::read(path)
}

#[test]
fn directory_routes_and_assets_are_relativised_for_file_urls() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("output");
    common::write(&out, "index.html", "<h1>Home</h1>");
    common::write(&out, "docs/installation/index.html", "<h1>Install</h1>");
    common::write(&out, "docs/components/index.html", "<h1>Components</h1>");
    common::write(&out, "docs/api-reference/folioh/index.html", "<h1>API</h1>");
    common::write(&out, "icon.svg", "<svg />");
    common::write(&out, "_next/static/chunks/app.js", "console.log('ok')");
    common::write(&out, "_next/static/chunks/app.css", "body{}");
    let flight =
        r#"1:["$","link",null,{"href":"/_next/static/chunks/app.css?build=1","rel":"stylesheet"}]"#;
    common::write(&out, "docs/index.txt", flight);
    common::write(&out, "media/folioh-commercial-v2-poster.jpeg", "poster");
    common::write(&out, "media/folioh-commercial-v2.mp4", "video");
    common::write(
        &out,
        "docs/index.html",
        concat!(
            "<a href=\"/\">Home</a>",
            "<a href=\"/docs/\">Docs</a>",
            "<a href=\"/docs/installation/\">Install</a>",
            "<a href=\"/docs/components\">Components</a>",
            "<a href=\"/docs/api-reference/folioh#config\">API</a>",
            "<a href=\"./installation/\">Install relative</a>",
            "<button data-href=\"/docs/components\">Components tree</button>",
            "<button data-href=\"/docs/components/index.html?panel=open#top\">Components index</button>",
            "<a href=\"#local\">Local anchor</a>",
            "<a href=\"https://example.com/docs/\">External</a>",
            "<link rel=\"icon\" href=\"/icon.svg?icon.hash.svg\">",
            "<script src=\"/_next/static/chunks/app.js\"></script>",
            "<video poster=\"/media/folioh-commercial-v2-poster.jpeg\">",
            "<source src=\"/media/folioh-commercial-v2.mp4\" type=\"video/mp4\">",
            "</video>",
            "<script>self.__next_f.push([1,\"I[1,[\\\"/_next/static/chunks/app.js\\\"],\\\"Comp\\\"]\"])</script>",
            "<script>self.__next_f.push([1,\"{\\\"poster\\\":\\\"/media/folioh-commercial-v2-poster.jpeg\\\",\\\"src\\\":\\\"/media/folioh-commercial-v2.mp4\\\"}\"])</script>",
        ),
    );
    assert!(StaticAssetRewriter::new(&out)
        .fix_asset_paths()
        .unwrap()
        .is_empty());
    let content = read(&out.join("docs/index.html"));
    for needle in [
        "href=\"../index.html\"",
        "href=\"index.html\"",
        "href=\"installation/index.html\"",
        "href=\"components/index.html\"",
        "href=\"api-reference/folioh/index.html#config\"",
        "data-href=\"components/\"",
        "data-href=\"components/?panel=open#top\"",
        "href=\"#local\"",
        "href=\"https://example.com/docs/\"",
        "href=\"../icon.svg?icon.hash.svg\"",
        "src=\"../_next/static/chunks/app.js\"",
        "poster=\"../media/folioh-commercial-v2-poster.jpeg\"",
        "src=\"../media/folioh-commercial-v2.mp4\"",
        "\\\"/_next/static/chunks/app.js\\\"",
        "\\\"poster\\\":\\\"/media/folioh-commercial-v2-poster.jpeg\\\"",
        "\\\"src\\\":\\\"/media/folioh-commercial-v2.mp4\\\"",
    ] {
        assert!(content.contains(needle), "missing {needle} in\n{content}");
    }
    assert!(!content.contains("data-href=\"components/index.html\""));
    assert_eq!(read(&out.join("docs/index.txt")), flight);
}

#[test]
fn root_relative_and_source_directory_fallbacks() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("output");
    common::write(&out, "docs/index.html", "<h1>Docs</h1>");
    common::write(
        &out,
        "index.html",
        "<a href=\"./\">Home</a><a href=\"./docs/\">Docs</a><a href=\"./docs\">Docs no slash</a>",
    );
    StaticAssetRewriter::new(&out).fix_asset_paths().unwrap();
    let content = read(&out.join("index.html"));
    assert!(
        content.contains("href=\"index.html\"") && content.contains("href=\"docs/index.html\"")
    );

    let out = dir.path().join("leaf");
    common::write(&out, "docs/a/c/index.html", "<h1>C</h1>");
    common::write(&out, "docs/x/index.html", "<h1>X</h1>");
    common::write(&out, "docs/a/b/index.html", "<a href=\"./c#frag\">Dot form</a><a href=\"c#frag\">Bare form</a><button data-href=\"./c#frag\">Tree</button><a href=\"../x#frag\">Up one</a><a href=\"./nowhere#frag\">Nowhere</a>");
    StaticAssetRewriter::new(&out).fix_asset_paths().unwrap();
    let content = read(&out.join("docs/a/b/index.html"));
    assert_eq!(content.matches("href=\"../c/index.html#frag\"").count(), 2);
    assert!(
        content.contains("data-href=\"../c/#frag\"")
            && content.contains("href=\"../../x/index.html#frag\"")
            && content.contains("href=\"./nowhere#frag\"")
    );

    let out = dir.path().join("decoy");
    common::write(&out, "docs/a/b/index.html", "<h1>B</h1>");
    common::write(&out, "docs/b/index.html", "<h1>Decoy</h1>");
    common::write(&out, "docs/a/index.html", "<a href=\"b\">B</a>");
    StaticAssetRewriter::new(&out).fix_asset_paths().unwrap();
    assert!(read(&out.join("docs/a/index.html")).contains("href=\"b/index.html\""));
}

#[test]
fn the_root_404_page_keeps_root_absolute_urls() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("output");
    let page = "<link rel=\"stylesheet\" href=\"/_next/static/css/app.css\"><a href=\"/docs/\">Read the docs</a>";
    common::write(&out, "docs/index.html", "<h1>Docs</h1>");
    common::write(&out, "_next/static/css/app.css", "body{}");
    common::write(&out, "404.html", page);
    common::write(&out, "404/index.html", page);
    StaticAssetRewriter::new(&out).fix_asset_paths().unwrap();
    assert_eq!(read(&out.join("404.html")), page);
    assert!(read(&out.join("404/index.html")).contains("href=\"../docs/index.html\""));
}

#[test]
fn opengraph_images_get_png_copies_and_urls() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("output");
    std::fs::create_dir_all(out.join("docs/quickstart")).unwrap();
    std::fs::write(out.join("docs/opengraph-image"), b"png").unwrap();
    common::write(
        &out,
        "docs/quickstart/index.html",
        "<meta property=\"og:image\" content=\"https://example.com/docs/opengraph-image?abc123\"><meta name=\"twitter:image\" content=\"/docs/opengraph-image?abc123\"><meta name=\"twitter:image\" content=\"https://example.com/opengraph-image\"><script>self.__next_f.push([\"https://example.com/opengraph-image\\\"])</script>",
    );
    StaticAssetRewriter::new(&out).fix_asset_paths().unwrap();
    assert_eq!(
        std::fs::read(out.join("docs/opengraph-image.png")).unwrap(),
        b"png"
    );
    let content = read(&out.join("docs/quickstart/index.html"));
    for needle in [
        "https://example.com/docs/opengraph-image.png?abc123",
        "content=\"/docs/opengraph-image.png?abc123\"",
        "https://example.com/opengraph-image.png",
        "https://example.com/opengraph-image.png\\",
    ] {
        assert!(content.contains(needle), "{needle}");
    }
    assert!(!content.contains("opengraph-image?abc123"));
}

#[test]
fn next_chunks_are_untouched_and_the_turbopack_runtime_is_patched_by_shape() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("output");
    let chunk = "let t=\"/\";async function load(){window.pagefind=await import(addBasePath(\"/_pagefind/pagefind.js\"))}const image={path:\"/_next/image\"};";
    common::write(&out, "_next/static/chunks/search.js", chunk);
    common::write(&out, "_pagefind/pagefind.js", "export {}");
    let runtime = "(globalThis.TURBOPACK=[]).push([]),(()=>{let e;let t=\"/_next/\",r=function(){let e=document?.currentScript?.getAttribute?.(\"src\")??\"\";let t=e.indexOf(\"?\");return t>=0?e.slice(t):\"\"}();function F(e){if(e)return{src:e.getAttribute(\"src\")}}function q(e){return`${t}${e.split(\"/\").map(e=>encodeURIComponent(e)).join(\"/\")}${r}`}})();";
    common::write(&out, "_next/static/chunks/turbopack-runtime.js", runtime);
    assert!(StaticAssetRewriter::new(&out)
        .fix_asset_paths()
        .unwrap()
        .is_empty());
    assert_eq!(read(&out.join("_next/static/chunks/search.js")), chunk);
    let content = read(&out.join("_next/static/chunks/turbopack-runtime.js"));
    assert!(
        !content.contains("let t=\"/_next/\",")
            && content.contains("document.currentScript.getAttribute(\"src\")")
            && content.contains("return t?t[1]:\"/_next/\"")
    );
    assert!(content.contains("return{src:/^https?:$/.test(location.protocol)?(()=>{let u=new URL(e.src);return u.pathname+u.search+u.hash})():e.getAttribute(\"src\")}"));
    assert!(content.contains(
        "if(e&&/^https?:$/.test(location.protocol))e=new URL(e,document.baseURI).pathname"
    ));
    assert!(!content
        .replace("document.currentScript.getAttribute(\"src\")", "")
        .contains("currentScript.src"));
    assert!(content.contains("function q(e){let _fp=e.indexOf(\"/_next/\")"));

    let out = dir.path().join("simple");
    common::write(&out, "_next/static/chunks/app.js", "console.log('ok')");
    common::write(
        &out,
        "_next/static/chunks/turbopack-runtime.js",
        "(()=>{let t=\"/_next/\",r=\"\";function N(e){return`${t}${e}${r}`}})();",
    );
    common::write(
        &out,
        "index.html",
        "<script src=\"/_next/static/chunks/app.js\"></script>",
    );
    common::write(
        &out,
        "docs/index.html",
        "<script src=\"/_next/static/chunks/app.js\"></script>",
    );
    assert!(StaticAssetRewriter::new(&out)
        .fix_asset_paths()
        .unwrap()
        .is_empty());
    assert!(read(&out.join("index.html")).contains("src=\"./_next/static/chunks/app.js\""));
    assert!(read(&out.join("docs/index.html")).contains("src=\"../_next/static/chunks/app.js\""));
    let runtime_js = read(&out.join("_next/static/chunks/turbopack-runtime.js"));
    assert!(runtime_js.contains("e.match(/^((?:.*\\/)?_next\\/)static\\/chunks\\//)"));
    assert!(runtime_js.contains("function N(e){let _fp=e.indexOf(\"/_next/\")"));
    let prefix_re = regex::Regex::new(r"^((?:.*/)?_next/)static/chunks/").unwrap();
    for (src, expected) in [
        ("./_next/static/chunks/app.js", "./_next/"),
        ("../_next/static/chunks/app.js", "../_next/"),
        ("../../_next/static/chunks/app.js", "../../_next/"),
        ("_next/static/chunks/app.js", "_next/"),
        ("/_next/static/chunks/app.js", "/_next/"),
        (
            "/preview/probe/_next/static/chunks/app.js",
            "/preview/probe/_next/",
        ),
    ] {
        assert_eq!(&prefix_re.captures(src).unwrap()[1], expected, "{src}");
    }

    let out = dir.path().join("next163");
    common::write(
        &out,
        "_next/static/chunks/turbopack-runtime.js",
        "(()=>{let P=\"string\"==typeof TURBOPACK_CHUNK_BASE_PATH?TURBOPACK_CHUNK_BASE_PATH:\"/_next/\",r=\"\";let K=/[^A-Za-z0-9]/;function q(e,t=P){let n=K.test(e)?e.split(\"/\").map(encodeURIComponent).join(\"/\"):e;return`${t}${n}${r}`}})();",
    );
    assert!(StaticAssetRewriter::new(&out)
        .fix_asset_paths()
        .unwrap()
        .is_empty());
    let content = read(&out.join("_next/static/chunks/turbopack-runtime.js"));
    assert!(content.contains("function q(e,t=P){let _fp=e.indexOf(\"/_next/\")"));
    assert!(
        content.contains("_fp>=0&&(e=e.slice(_fp+7));let n=K.test(e)?")
            && content.contains(".map(encodeURIComponent).join(\"/\"):e;return`${t}${n}${r}`")
    );

    let out = dir.path().join("drift");
    common::write(
        &out,
        "_next/static/chunks/turbopack-runtime.js",
        "(()=>{let t=\"/_next/\";loadChunkUnrecognizedShape(t)})();",
    );
    let warnings = StaticAssetRewriter::new(&out).fix_asset_paths().unwrap();
    assert_eq!(warnings.len(), 1);
    assert!(
        warnings[0].starts_with(
            "static_rewriter: Turbopack chunk-path function not found in turbopack-runtime.js;"
        ),
        "{}",
        warnings[0]
    );
}

#[test]
fn pagefind_fragments_produce_the_file_search_fallback() {
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("output");
    common::write(
        &out,
        "docs/index.html",
        "<html><head></head><body>Docs mention _folioh-navigation.js</body></html>",
    );
    common::write(
        &out,
        "404.html",
        "<html><head></head><body>Missing</body></html>",
    );
    let fragment = serde_json::json!({"url": "/docs/components/", "content": "Components Built-in UI components and live previews.", "meta": {"title": "Components"}, "anchors": []});
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder
        .write_all(format!("pagefind_dcd{fragment}").as_bytes())
        .unwrap();
    std::fs::create_dir_all(out.join("_pagefind/fragment")).unwrap();
    std::fs::write(
        out.join("_pagefind/fragment/en_components.pf_fragment"),
        encoder.finish().unwrap(),
    )
    .unwrap();
    std::fs::write(
        out.join("_pagefind/fragment/broken.pf_fragment"),
        b"not gzip",
    )
    .unwrap();
    StaticAssetRewriter::new(&out)
        .fix_asset_paths_with_base_path("/preview/probe")
        .unwrap();
    let html = read(&out.join("docs/index.html"));
    let fallback = read(&out.join("_folioh-search.js"));
    assert!(
        html.contains("<script defer src=\"../_folioh-search.js\"></script>"),
        "{html}"
    );
    assert!(
        fallback.contains("window.__foliohStaticSearch")
            && fallback.contains("docs/components/index.html")
    );
    assert!(fallback.starts_with("(function(){\nconst documents=[{\"url\":\"docs/components/index.html\",\"title\":\"Components\",\"content\":\"Components Built-in UI components and live previews.\"}];\nlet options={};\n"));
    assert!(fallback.ends_with(
        "if(location.protocol==='file:'&&!window.pagefind){window.pagefind=api;}\n})();\n"
    ));
    let fallback_404 = read(&out.join("404.html"));
    assert!(
        fallback_404.contains("src=\"/preview/probe/_folioh-search.js\""),
        "{fallback_404}"
    );
    assert!(
        fallback_404.contains("src=\"/preview/probe/_folioh-navigation.js\""),
        "{fallback_404}"
    );
    assert!(html.contains("src=\"../_folioh-navigation.js\""));
}

#[test]
fn flight_navigation_uses_directory_routes_while_html_keeps_file_targets() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("output");
    common::write(
        &out,
        "docs/a/index.html",
        "<a href=\"/docs/b/?view=full#example\">B</a>",
    );
    common::write(&out, "docs/b/index.html", "<h1>B</h1>");
    common::write(&out, "icon.svg", "<svg />");
    common::write(&out, "preview/probe/_next/static/app.css", "body{}");
    common::write(
        &out,
        "docs/a/__next._full.txt",
        r#"{"href":"/docs/b/?view=full#example","src":"/icon.svg","styles":{"href":"/preview/probe/_next/static/app.css?build=1"}}"#,
    );
    StaticAssetRewriter::new(&out).fix_asset_paths().unwrap();
    assert!(
        read(&out.join("docs/a/index.html")).contains("href=\"../b/index.html?view=full#example\"")
    );
    let flight = read(&out.join("docs/a/__next._full.txt"));
    assert!(
        flight.contains(r#""href":"../b/?view=full#example""#),
        "{flight}"
    );
    assert!(flight.contains(r#""src":"../../icon.svg""#), "{flight}");
    assert!(
        flight.contains(r#""href":"/preview/probe/_next/static/app.css?build=1""#),
        "{flight}"
    );
}

use super::*;

#[test]
fn rendered_ids_match_sources_and_refresh_after_a_heading_changes() {
    let tmp = tempfile::tempdir().unwrap();
    let content = tmp.path().join("content");
    let output = tmp.path().join("output");
    std::fs::create_dir_all(content.join("guide")).unwrap();
    std::fs::create_dir_all(output.join("reference/guide/start")).unwrap();
    std::fs::create_dir_all(output.join("reference/api")).unwrap();
    std::fs::write(content.join("guide/start.mdx"), "generated text").unwrap();
    std::fs::write(content.join("api.mdx"), "[API](#explicit)").unwrap();
    let raw = tmp.path().join("start.md");
    let source = "---\ntitle: '[ignored](#frontmatter)'\n---\n# Guide\n[Same](#stable) [Encoded](#caf%C3%A9) [Entity](#a&amp;b)\n[API](../api.md#explicit \"title\")\n<a href='/reference/api/#explicit'>API</a>\n<Cards items={[{href: '/reference/api/#explicit'}]} />\n[Missing](#absent)\n```md\n[Code](#code)\n```\n`[inline](#inline)`\n<!-- [comment](#comment) -->\n<script> '[script](#script)' </script>\n[Text](#:~:text=example) [External](https://example.com/#other)\n[File](/guide.pdf#page=2)\n[Only data attribute](#fake)\n[Only script](#script-only)\n";
    std::fs::write(&raw, source).unwrap();
    let html = output.join("reference/guide/start/index.html");
    std::fs::write(&html, "<h1 id=\"stable\">Guide</h1><h2 id='café'>Café</h2><h2 id='a&amp;b'>A&amp;B</h2><code> id='fake' </code><div data-x=\" id='fake'\" data-id='fake'></div><script>\" id='script-only'\"</script>").unwrap();
    std::fs::write(
        output.join("reference/api/index.html"),
        "<a id='explicit'></a>",
    )
    .unwrap();
    let docs = [MarkdownPage {
        route: "guide/start".into(),
        source_file: raw.to_string_lossy().into_owned(),
        ..Default::default()
    }];
    let broken = check_fragments(&content, &output, "/reference", &docs).unwrap();
    assert_eq!(
        broken
            .iter()
            .map(|b| (b.target.as_str(), b.line_number))
            .collect::<Vec<_>>(),
        [("#absent", 9), ("#fake", 18), ("#script-only", 19)]
    );
    assert!(broken
        .iter()
        .all(|b| b.source_page == raw.to_string_lossy()));
    std::fs::write(
        &html,
        "<h1 id='renamed'>Guide</h1><h2 id='café'></h2><h2 id='a&amp;b'></h2>",
    )
    .unwrap();
    let changed = check_fragments(&content, &output, "/reference", &docs).unwrap();
    assert!(changed
        .iter()
        .any(|b| b.target == "#stable" && b.line_number == 5));
    // Generated sources use their own file and line for diagnostics.
    std::fs::write(content.join("api.mdx"), "# API\n[Missing](#removed)").unwrap();
    let changed = check_fragments(&content, &output, "/reference", &docs).unwrap();
    assert!(changed.iter().any(|b| b.source_page.ends_with("api.mdx")
        && b.line_number == 2
        && b.target == "#removed"));
}

#[test]
fn root_views_index_files_and_html_entities_use_browser_fragment_semantics() {
    let tmp = tempfile::tempdir().unwrap();
    let content = tmp.path().join("content");
    let output = tmp.path().join("output");
    std::fs::create_dir_all(&content).unwrap();
    for route in ["reference", "roadmap", "reference/v0.3"] {
        std::fs::create_dir_all(output.join(route)).unwrap();
        std::fs::write(
            output.join(route).join("index.html"),
            "<h1 id='Case'></h1><h2 id='a&amp;b'></h2><h2 id='repeat-1'></h2>",
        )
        .unwrap();
    }
    std::fs::write(output.join("index.html"), "<a id=home></a>").unwrap();
    std::fs::write(output.join("guide.html"), "<a id='file'></a>").unwrap();
    std::fs::write(content.join("index.mdx"), "[root](/#home) [view](/roadmap#Case) [file](/guide.html#file) [index](/reference/index.html#Case) [dotted](./v0.3#Case) [query](?next=https://example.com#repeat-1)\n<a href='#a&amp;b'>entity</a>\n<Cards items={[{href: '#a&b'}]} />\n[case](?next=https://example.com#case)\n<Cards items={[{href: '#a&amp;b'}]} />\n[asset](/guide.pdf#file)\n").unwrap();
    let broken = check_fragments(&content, &output, "/reference", &[]).unwrap();
    assert_eq!(
        broken
            .iter()
            .map(|b| (b.target.as_str(), b.line_number))
            .collect::<Vec<_>>(),
        [("?next=https://example.com#case", 4), ("#a&amp;b", 5)]
    );
}

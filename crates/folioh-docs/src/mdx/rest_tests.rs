use super::*;

#[test]
fn directives_spare_code_and_escape_untrusted_prose() {
    for code in [
        "```rst\n.. note:: literal\n```",
        "    .. note:: literal",
        "Use `.. note:: literal`.",
    ] {
        assert_eq!(render(code, 3), None, "{code}");
    }
    let text = "Before.\n\n.. note:: Inline text.\n\n    Continuation with {value} and <script>.\n\nAfter.";
    let rendered = render(text, 3).unwrap();
    assert!(rendered.starts_with("Before.\n\n<Callout type=\"note\" title=\"Note\">"));
    assert!(rendered.contains("Inline text.\n\nContinuation with \\{value\\} and &lt;script&gt;."));
    assert!(rendered.ends_with("</Callout>\n\nAfter."));
}

#[test]
fn nested_admonitions_preserve_lists_code_and_heading_floor() {
    let text = ".. warning::\n\n    ## Details\n\n    - First\n    - Second\n\n    .. tip:: Try this.\n\n        ```rst\n        .. note:: literal code\n        ```\n\nOutside.";
    let rendered = render(text, 4).unwrap();
    assert!(rendered.contains("#### Details\n\n- First\n- Second"));
    assert!(rendered.contains("<Callout type=\"tip\" title=\"Tip\">"));
    assert!(rendered.contains("```rst\n.. note:: literal code\n```"));
    assert!(rendered.ends_with("</Callout>\n\nOutside."));
}

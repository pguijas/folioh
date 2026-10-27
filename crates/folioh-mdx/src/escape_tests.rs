use super::*;

#[test]
fn dunder_identifiers_stay_literal_without_rewriting_code_or_urls() {
    for (input, expected) in [
        (
            "Call __init__ and __next__.",
            r"Call \_\_init\_\_ and \_\_next\_\_.",
        ),
        (
            "`__init__` **bold** __two words__",
            "`__init__` **bold** __two words__",
        ),
        ("```python\n__init__()\n```", "```python\n__init__()\n```"),
        (
            r"Already \_\_init\_\_ or \__call__.",
            r"Already \_\_init\_\_ or \__call__.",
        ),
        (
            "https://example.com/__init__/ [link](../__next__/)",
            "https://example.com/__init__/ [link](../__next__/)",
        ),
    ] {
        assert_eq!(escape_mdx(input), expected, "{input}");
    }
}

#[test]
fn escape_mdx_spares_inline_code_spans() {
    let cases = [
        ("Route is `/{repo}` here.", "Route is `/{repo}` here."),
        ("Use `<a href>` tags.", "Use `<a href>` tags."),
        ("A {brace} here.", "A \\{brace\\} here."),
        ("A <Tag> here.", "A &lt;Tag&gt; here."),
        ("`{kept}` then {escaped}", "`{kept}` then \\{escaped\\}"),
        ("``a `{b}` c``", "``a `{b}` c``"),
        ("An ` unclosed {brace}", "An ` unclosed \\{brace\\}"),
        ("``a` {x}", "``a` \\{x\\}"),
        (
            "caf\u{e9} `{\u{e9}}` {\u{e9}}",
            "caf\u{e9} `{\u{e9}}` \\{\u{e9}\\}",
        ),
    ];
    for (input, expected) in cases {
        assert_eq!(escape_mdx(input), expected, "{input:?}");
    }
}

#[test]
fn escape_mdx_spares_fenced_blocks_and_resumes_after_them() {
    let result = escape_mdx("Example:\n\n```python\nd = {\"k\": 1}\n```\n\nAfter {brace}.");
    assert!(result.contains("d = {\"k\": 1}"));
    assert!(result.contains("After \\{brace\\}."));
    let nested = escape_mdx("````markdown\n```python\nx = {1}\n```\n````\n\nProse {brace}.");
    assert!(nested.contains("x = {1}"));
    assert!(nested.contains("Prose \\{brace\\}."));
}

#[test]
fn fence_flags_follow_the_opener_character_and_length() {
    let lines = [
        "a", "  ```py", "{x}", "```", "b", "~~~", "```", "~~", "~~~~", "c", "````", "```", "````",
    ];
    assert_eq!(
        code_fence_flags(&lines),
        [false, true, true, true, false, true, true, true, true, false, true, true, true]
    );
}

#[test]
fn jsx_attr_escapes_quotes_before_anything_else() {
    let attr = escape_jsx_attr("Foo\" onclick=\"alert(1)");
    assert_eq!(attr, "Foo&quot; onclick=&quot;alert(1)");
    assert_eq!(escape_jsx_attr("a&b<{c}>"), "a&amp;b&lt;&#123;c&#125;&gt;");
}

#[test]
fn curly_escaping_spares_code_math_and_mdx_comments() {
    let cases = [
        ("Use {config}", "Use \\{config\\}"),
        ("Route is `/{repo}` here.", "Route is `/{repo}` here."),
        (
            "Math $x_{i}$ and code `{y}` and {z}.",
            "Math $x_{i}$ and code `{y}` and \\{z\\}.",
        ),
        ("$$\\frac{a}{b}$$", "$$\\frac{a}{b}$$"),
        (
            "Both $\\frac{x}{y}$ and $\\sum_{i=1}^{n}$ work",
            "Both $\\frac{x}{y}$ and $\\sum_{i=1}^{n}$ work",
        ),
        ("{/* a note */}", "{/* a note */}"),
        ("    second line */}", "    second line */}"),
        ("\\{literal} and {x\\}", "\\{literal\\} and \\{x\\\\}"),
        // Two dollars on one line read as inline math, braces included.
        ("$5 and {x} cost $", "$5 and {x} cost $"),
        ("$$x$$$y$ {z}", "$$x$$$y$ \\{z\\}"),
    ];
    for (input, expected) in cases {
        assert_eq!(escape_curly_outside_math(input), expected, "{input:?}");
    }
}

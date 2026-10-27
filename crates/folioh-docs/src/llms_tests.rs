use super::*;
use crate::fixtures::{arg, class, doc, function, module, ret, type_item, var};
use folioh_ir::{DocstringIR, FunctionKind, Language, ModuleIR, RaiseIR, TypeIR, TypeKind};
use serde_yaml_ng::Value;

const CTX: LlmsContext<'static> = LlmsContext {
    project_name: "TestProject",
    landing_hero_description: "",
    site_url: "",
    docs_route_base: "/docs",
    project_dir: "",
    api_gates: &DISABLED_API_MODULES,
};

fn page(content: &str, title: Option<&str>, route: &str) -> MarkdownPage {
    let mut frontmatter = Frontmatter::new();
    if let Some(title) = title {
        frontmatter.insert("title".to_string(), Value::String(title.to_string()));
    }
    MarkdownPage {
        content: content.to_string(),
        frontmatter,
        route: route.to_string(),
        ..MarkdownPage::default()
    }
}

fn intro() -> MarkdownPage {
    page(
        "# Introduction\n\nWelcome to the docs.",
        Some("Introduction"),
        "introduction",
    )
}

fn utils() -> ModuleIR {
    let mut greet = function(
        "greet",
        vec![arg("name", "str", None, "The name.")],
        Some(ret("str", "A greeting.")),
    );
    greet.docstring = doc("Greet someone.");
    let mut m = module("mylib.utils", vec![], vec![greet]);
    m.docstring = doc("Utility module.");
    m.source_file = "utils.py".to_string();
    m
}

fn rich() -> ModuleIR {
    let mut greet = function(
        "greet",
        vec![
            arg("name", "str", None, "The name."),
            arg("times", "int", Some("1"), "Repeat | count."),
        ],
        Some(ret("str", "A greeting.")),
    );
    greet.raises = vec![RaiseIR {
        exception: "ValueError".to_string(),
        description: "If name is empty.".to_string(),
    }];
    greet.docstring = DocstringIR {
        short_description: "Greet someone.".to_string(),
        long_description: "Builds the greeting string.".to_string(),
        examples: vec!["greet('ada')".to_string()],
        notes: vec!["Not thread\nsafe.".to_string()],
    };
    greet.source_file = "/proj/mylib/utils.py".to_string();
    greet.line_number = 12;
    let mut run = function("run", vec![], None);
    run.docstring = doc("Run the greeter.");
    run.source_file = "/proj/mylib/utils.py".to_string();
    run.line_number = 40;
    let mut greeter = class("Greeter", &["object"], vec![run]);
    greeter.docstring = doc("Greets people.");
    greeter.source_file = "/proj/mylib/utils.py".to_string();
    greeter.line_number = 30;
    greeter.inner_classes = vec![class("Hidden", &[], vec![])];
    let mut m = module("mylib.utils", vec![greeter], vec![greet]);
    m.docstring = doc("Utility module.");
    m.source_file = "/proj/mylib/utils.py".to_string();
    m.constants = vec![var("MAX", "int", "Limit.")];
    m
}

fn rust_module() -> ModuleIR {
    let mut norm = function("norm", vec![], Some(ret("f64", "Length.")));
    norm.docstring = doc("Vector length.");
    norm.source_file = "/proj/src/geo.rs".to_string();
    norm.line_number = 12;
    norm.kind = FunctionKind::Method;
    norm.signature = "pub fn norm(&self) -> f64".to_string();
    let point = TypeIR {
        docstring: doc("A 2D point."),
        fields: vec![var("x", "f64", "Horizontal.")],
        methods: vec![norm],
        signature: "pub struct Point".to_string(),
        source_file: "/proj/src/geo.rs".to_string(),
        line_number: 3,
        ..type_item("Point", TypeKind::Struct)
    };
    let shape = TypeIR {
        docstring: doc("A shape."),
        variants: vec![var("Circle", "f64", "Round.")],
        signature: "pub enum Shape".to_string(),
        source_file: "/proj/src/geo.rs".to_string(),
        line_number: 20,
        ..type_item("Shape", TypeKind::Enum)
    };
    let mut area = function(
        "area",
        vec![arg("shape", "Shape", None, "")],
        Some(ret("f64", "")),
    );
    area.docstring = doc("Area of a shape.");
    area.source_file = "/proj/src/geo.rs".to_string();
    area.line_number = 30;
    area.signature = "pub fn area(shape: &Shape) -> f64".to_string();
    let mut m = module("geo", vec![], vec![area]);
    m.docstring = doc("Geometry.");
    m.source_file = "/proj/src/geo.rs".to_string();
    m.language = Language::Rust;
    m.types = vec![point, shape];
    m
}

#[test]
fn llms_txt_is_byte_exact() {
    let ctx = LlmsContext {
        landing_hero_description: "One config\nfile builds the site.",
        ..CTX
    };
    let mut described = intro();
    described.frontmatter.insert(
        "description".to_string(),
        Value::String("Line one\nline two".to_string()),
    );
    let docs = [
        described,
        page("# Overview\n\nWelcome.", Some("Overview"), "index"),
        page("Catalog.", None, "components/index"),
    ];
    let text = generate_llms_txt(&ctx, &[utils(), rust_module()], &docs);
    assert_eq!(
            text,
            "# TestProject\n\n> One config file builds the site.\n\n## Docs\n- [Introduction](/docs/introduction/): Line one line two\n- [Overview](/docs/)\n- [components/index](/docs/components/)\n\n## API Reference\n- [mylib.utils](/docs/api-reference/mylib/utils/): Utility module.\n- [geo](/docs/api-reference/rust/geo/): Geometry.\n"
        );
    assert_eq!(generate_llms_txt(&CTX, &[], &[]), "# TestProject\n");
    let plain = generate_llms_txt(&CTX, &[], &[intro()]);
    assert!(!plain.contains('>'));
    assert!(plain.contains("- [Introduction](/docs/introduction/)\n"));
    assert!(!plain.contains("- [Introduction](/docs/introduction/):"));
}

#[test]
fn gated_api_modules_are_skipped_by_both_writers() {
    let mut roadmap = module("folioh_docs.docs.integrations.roadmap", vec![], vec![]);
    roadmap.docstring = doc("Roadmap plugin internals.");
    let ctx = LlmsContext {
        api_gates: &[("folioh_docs.docs.integrations.roadmap", "roadmap")],
        ..CTX
    };
    let text = generate_llms_txt(&ctx, &[roadmap.clone(), utils()], &[]);
    assert!(!text.contains("folioh_docs.docs.integrations.roadmap"));
    assert!(!text.contains("/docs/api-reference/folioh_docs/docs/integrations/roadmap/"));
    assert!(text.contains("- [mylib.utils](/docs/api-reference/mylib/utils/): Utility module."));
    let full = generate_llms_full_txt(std::slice::from_ref(&roadmap), &[], Some(&ctx));
    assert_eq!(full, "");
    // Without a table the module publishes; `None` uses the release table (empty).
    assert!(generate_llms_txt(&CTX, &[roadmap.clone()], &[]).contains("integrations.roadmap"));
    assert!(generate_llms_full_txt(&[roadmap], &[], None).contains("Roadmap plugin internals."));
    assert!(LlmsContext::default().api_gates.is_empty());
}

#[test]
fn llms_txt_links_follow_site_url_and_route_base() {
    let ctx = LlmsContext {
        site_url: "https://example.com/folioh/",
        ..CTX
    };
    let text = generate_llms_txt(&ctx, &[utils()], &[intro()]);
    assert!(text.contains("[Introduction](https://example.com/folioh/docs/introduction/)"));
    assert!(
        text.contains("[mylib.utils](https://example.com/folioh/docs/api-reference/mylib/utils/)")
    );
    let ctx = LlmsContext {
        site_url: "https://example.com",
        docs_route_base: "/reference/docs",
        ..CTX
    };
    let text = generate_llms_txt(&ctx, &[utils()], &[intro()]);
    assert!(text.contains("[Introduction](https://example.com/reference/docs/introduction/)"));
    assert!(text
        .contains("[mylib.utils](https://example.com/reference/docs/api-reference/mylib/utils/)"));

    assert_eq!(doc_link("index", "", "/docs"), "/docs/");
    assert_eq!(doc_link("", "", ""), "/docs/");
    assert_eq!(
        doc_link("/components/index/", "", "/docs/"),
        "/docs/components/"
    );
    let m = ModuleIR {
        name: "folioh_docs.agent_output.llm_output".to_string(),
        ..utils()
    };
    assert_eq!(
        api_link(&m, "", "/docs"),
        "/docs/api-reference/folioh_docs/agent_output/llm_output/"
    );
}

#[test]
fn llms_txt_includes_authored_pages_named_after_disabled_features() {
    for route in ["versioning", "i18n"] {
        let guide = page("# Guide\n\nProject guide.", Some("Guide"), route);
        let text = generate_llms_txt(&CTX, &[], &[guide]);
        assert!(
            text.contains(&format!("- [Guide](/docs/{route}/)")),
            "{text}"
        );
    }
}

#[test]
fn llms_full_txt_shape_headings_and_gates() {
    let text = generate_llms_full_txt(&[utils()], &[intro()], None);
    assert_eq!(text.matches("# Introduction").count(), 1);
    assert!(text.contains("\n\n---\n\n"));
    assert!(text.contains("```python\ndef greet(name: str) -> str\n```"));
    assert!(text.contains("Greet someone."));
    assert!(!text.contains("URL:"));
    assert!(!text.ends_with('\n'));

    assert_eq!(generate_llms_full_txt(&[], &[], None), "");
    for route in ["versioning", "i18n"] {
        let gated = page("# Guide\n\nProject guide content.", Some("Guide"), route);
        assert!(
            generate_llms_full_txt(&[], &[gated], None).contains("Project guide content."),
            "{route}"
        );
    }
    let landing = page(
        "# Landing Page\n\nConfigure the optional homepage.",
        Some("Landing Page"),
        "landing",
    );
    assert_eq!(
        generate_llms_full_txt(&[], &[landing], None),
        "# Landing Page\n\nConfigure the optional homepage."
    );

    let untitled = page("Just a paragraph.", Some("Untitled Page"), "untitled");
    assert_eq!(
        generate_llms_full_txt(&[], &[untitled], None),
        "# Untitled Page\n\nJust a paragraph."
    );
    let orphan = page("Body only.", None, "orphan");
    assert!(generate_llms_full_txt(&[], &[orphan], None).starts_with("# orphan\n"));
    let mdx = page(
            "import { Callout } from 'nextra/components'\nexport const meta = {}\n\n# Guide\n\n<Callout type='info'>Read this.</Callout>\n\nPlain body.\n",
            Some("Guide"),
            "guide",
        );
    let text = generate_llms_full_txt(&[], &[mdx], None);
    assert_eq!(text, "# Guide\n\nRead this.\n\nPlain body.");
    let below = page(
        "import { Tabs } from 'x'\n\n# Real Heading\n\nBody.",
        Some("Frontmatter Title"),
        "page",
    );
    let text = generate_llms_full_txt(&[], &[below], None);
    assert!(text.starts_with("# Real Heading\n"));
    assert!(!text.contains("Frontmatter Title"));
    let two = [
        intro(),
        page("# Second\n\nMore text.", Some("Second"), "second"),
    ];
    let text = generate_llms_full_txt(&[], &two, None);
    assert!(text.contains("Welcome to the docs.\n\n---\n\n# Second"));
}

#[test]
fn llms_full_txt_urls_follow_the_context() {
    let ctx = LlmsContext {
        site_url: "https://example.com",
        ..CTX
    };
    let text = generate_llms_full_txt(&[utils()], &[intro()], Some(&ctx));
    assert!(text.starts_with("# Introduction\nURL: https://example.com/docs/introduction/\n\nWelcome to the docs.\n\n---\n\n# mylib.utils\nURL: https://example.com/docs/api-reference/mylib/utils/\nSource: utils.py\n\nUtility module.\n\n## greet\nSource: test.py:1\n\n```python\n"));
    let ctx = LlmsContext {
        docs_route_base: "/reference/docs",
        ..CTX
    };
    let text = generate_llms_full_txt(&[utils()], &[intro()], Some(&ctx));
    assert!(text.contains("URL: /reference/docs/introduction/"));
    assert!(text.contains("URL: /reference/docs/api-reference/mylib/utils/"));
    assert!(generate_llms_full_txt(&[rust_module()], &[], Some(&CTX))
        .contains("# geo\nURL: /docs/api-reference/rust/geo/"));
}

#[test]
fn llms_full_txt_module_blocks_are_byte_exact() {
    let ctx = LlmsContext {
        project_dir: "/proj",
        ..CTX
    };
    let text = generate_llms_full_txt(&[rich()], &[], Some(&ctx));
    assert_eq!(
            text,
            "# mylib.utils\nURL: /docs/api-reference/mylib/utils/\nSource: mylib/utils.py\n\nUtility module.\n\n## Constants\n\n| Constant | Type | Description |\n| --- | --- | --- |\n| `MAX` | `int` | Limit. |\n\n## Greeter\nSource: mylib/utils.py:30\n\n```python\nclass Greeter(object)\n```\n\nGreets people.\n\n### run\nSource: mylib/utils.py:40\n\n```python\ndef run()\n```\n\nRun the greeter.\n\n### Hidden\nSource: test.py:1\n\n```python\nclass Hidden\n```\n\n## greet\nSource: mylib/utils.py:12\n\n```python\ndef greet(name: str, times: int = 1) -> str\n```\n\nGreet someone.\n\nBuilds the greeting string.\n\n| Parameter | Type | Default | Description |\n| --- | --- | --- | --- |\n| `name` | `str` |  | The name. |\n| `times` | `int` | `1` | Repeat \\| count. |\n\n**Returns:** `str` - A greeting.\n\n**Raises:**\n- `ValueError` - If name is empty.\n\n**Examples:**\n\n```python\ngreet('ada')\n```\n\n**Notes:**\n- Not thread safe."
        );
    let unrooted = generate_llms_full_txt(&[rich()], &[], None);
    assert!(unrooted.contains("Source: utils.py:12"));
    assert!(!unrooted.contains("/proj/"));
    assert_eq!(source_citation("", 3, "/proj"), "");
    assert_eq!(source_citation("rel/a.py", 0, "/proj/"), "rel/a.py");
    assert_eq!(source_citation("/proj/a.py", 0, "/proj/"), "a.py");
}

#[test]
fn llms_full_txt_a_return_or_raise_without_a_type_reads_as_its_description() {
    let mut fetch = function("fetch", vec![], Some(ret("", "The thing.")));
    fetch.raises = vec![
        RaiseIR {
            exception: String::new(),
            description: "When it fails.".to_string(),
        },
        RaiseIR {
            exception: String::new(),
            description: String::new(),
        },
    ];
    let empty = function("empty", vec![], Some(ret("", "")));
    let text = generate_llms_full_txt(&[module("app", vec![], vec![fetch, empty])], &[], None);
    assert!(text.contains("**Returns:** The thing.\n"), "{text}");
    assert!(text.contains("**Raises:**\n- When it fails.\n\n"), "{text}");
    assert!(!text.contains(":** ``") && !text.contains("- ``"), "{text}");
    assert!(!text.contains("\n- \n"), "{text}");
    assert_eq!(text.matches("**Returns:**").count(), 1, "{text}");
}

#[test]
fn llms_full_txt_keeps_yielded_items_separate_from_return_annotations() {
    let mut value = ret("int", "Each value.");
    value.is_generator = true;
    value.return_annotation = Some("Iterator[int]".into());
    let yielded = function("values", vec![], Some(value));
    let ordinary = function("name", vec![], Some(ret("str", "The name.")));
    let text = generate_llms_full_txt(
        &[module("streams", vec![], vec![yielded, ordinary])],
        &[],
        None,
    );
    assert!(text.contains("def values() -> Iterator[int]"));
    assert!(text.contains("**Yields:** `int` - Each value."));
    assert!(!text.contains("**Returns:** `int`") && !text.contains("**Yields:** `Iterator[int]`"));
    assert!(text.contains("**Returns:** `str` - The name."));
}

#[test]
fn llms_full_txt_argument_table_escapes_pipes_and_property_uses_type() {
    let mut lookup = function(
        "lookup",
        vec![
            arg("key", "str | None", Some("None"), "The key."),
            arg(
                "mode",
                "Literal['a'] | Literal['b']",
                Some("'a' | 'b'"),
                "The mode.",
            ),
            folioh_ir::ArgIR {
                kind: folioh_ir::ArgKind::VarKeyword,
                ..arg("extra", "", None, "")
            },
        ],
        None,
    );
    lookup.docstring = doc("Look something up.");
    lookup.raises = vec![RaiseIR {
        exception: "KeyError".to_string(),
        description: String::new(),
    }];
    let mut value = function(
        "value",
        vec![arg("self", "", None, "")],
        Some(ret("int", "")),
    );
    value.kind = FunctionKind::Property;
    let mut m = module("lookups", vec![], vec![lookup, value]);
    m.docstring = doc("Lookups.");
    let text = generate_llms_full_txt(&[m], &[], None);
    assert!(text.contains("| `key` | `str \\| None` | `None` | The key. |"));
    assert!(
        text.contains("| `mode` | `Literal['a'] \\| Literal['b']` | `'a' \\| 'b'` | The mode. |")
    );
    assert!(text.contains("| `**extra` | `Any` |  |  |"));
    assert!(text.contains("**Raises:**\n- `KeyError`"));
    assert!(text.contains(
        "## value\nSource: test.py:1\n\n```python\n@property\nvalue\n```\n\n**Type:** `int`"
    ));
    assert!(!text.contains("| `self` |"));
    for line in text
        .lines()
        .filter(|l| l.starts_with("| `key`") || l.starts_with("| `mode`"))
    {
        assert_eq!(line.replace("\\|", "").matches('|').count(), 5, "{line}");
    }
}

#[test]
fn llms_full_txt_writes_class_decorators_bases_and_method_kinds() {
    let mut clamp = function(
        "clamp",
        vec![arg("value", "float", None, "")],
        Some(ret("float", "")),
    );
    clamp.kind = FunctionKind::Staticmethod;
    clamp.decorators = vec!["staticmethod".to_string(), "cache".to_string()];
    let mut from_file = function("from_file", vec![arg("path", "str", None, "")], None);
    from_file.kind = FunctionKind::Classmethod;
    from_file.decorators = vec!["classmethod".to_string()];
    let mut full_name = function("full_name", vec![], Some(ret("str", "")));
    full_name.kind = FunctionKind::Property;
    full_name.decorators = vec!["property".to_string()];
    let mut point = class(
        "Point",
        &["Base", "Generic[T]"],
        vec![clamp, from_file, full_name],
    );
    point.decorators = vec![
        "dataclasses.dataclass(frozen=True)".to_string(),
        "total_ordering".to_string(),
    ];
    let text = generate_llms_full_txt(&[module("geo", vec![point], vec![])], &[], None);
    for block in [
        "## Point\nSource: test.py:1\n\n```python\n@dataclasses.dataclass(frozen=True)\n@total_ordering\nclass Point(Base, Generic[T])\n```\n\n### clamp",
        "### clamp\nSource: test.py:1\n\n```python\n@staticmethod\n@cache\ndef clamp(value: float) -> float\n```",
        "### from_file\nSource: test.py:1\n\n```python\n@classmethod\ndef from_file(path: str)\n```",
        "### full_name\nSource: test.py:1\n\n```python\n@property\nfull_name\n```\n\n**Type:** `str`",
    ] {
        assert!(text.contains(block), "{block}\n---\n{text}");
    }

    let mut widget = class("Widget", &["Base"], vec![]);
    widget.decorators = vec!["@sealed".to_string()];
    let javascript = ModuleIR {
        language: Language::Javascript,
        ..module("lib.ui", vec![widget, class("Plain", &[], vec![])], vec![])
    };
    let text = generate_llms_full_txt(&[javascript], &[], None);
    assert!(
        text.contains("```javascript\n@sealed\nclass Widget extends Base\n```"),
        "{text}"
    );
    assert!(text.contains("```javascript\nclass Plain\n```"), "{text}");
}

#[test]
fn llms_full_txt_renders_types_after_functions() {
    let ctx = LlmsContext {
        project_name: "Geo",
        project_dir: "/proj",
        ..CTX
    };
    let text = generate_llms_full_txt(&[rust_module()], &[], Some(&ctx));
    assert!(text.find("## area").unwrap() < text.find("## Struct Point").unwrap());
    assert!(text.contains("```rust\npub fn area(shape: &Shape) -> f64\n```\n\nArea of a shape.\n\n| Parameter | Type | Default | Description |\n| --- | --- | --- | --- |\n| `shape` | `Shape` |  |  |\n\n**Returns:** `f64`\n\n## Struct Point\nSource: src/geo.rs:3\n\n```rust\npub struct Point\n```\n\nA 2D point.\n\n| Field | Type | Description |\n| --- | --- | --- |\n| `x` | `f64` | Horizontal. |\n\n### norm\nSource: src/geo.rs:12\n\n```rust\npub fn norm(&self) -> f64\n```\n\nVector length.\n\n**Returns:** `f64` - Length.\n\n## Enum Shape\nSource: src/geo.rs:20\n\n```rust\npub enum Shape\n```\n\nA shape.\n\n| Variant | Type | Description |\n| --- | --- | --- |\n| `Circle` | `f64` | Round. |"));
    assert!(!text.contains("def area"));
    let mut implementation = type_item("Point", TypeKind::Impl);
    implementation.bases = vec!["fmt::Display".to_string()];
    implementation.signature = "impl fmt::Display for Point".to_string();
    let mut rust = rust_module();
    rust.types.push(implementation);
    let text = generate_llms_full_txt(&[rust], &[], Some(&ctx));
    assert!(text.contains("## impl fmt::Display for Point\n"));
    assert!(!text.contains("## Impl Point\n"));
}

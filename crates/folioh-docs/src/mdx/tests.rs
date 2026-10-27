use super::*;
use crate::fixtures::{arg, class, doc, function, module, ret, type_item, var};
use crate::xref::build_symbol_index;
use folioh_ir::{ArgIR, ClassIR, DocstringIR, FunctionKind, Language, RaiseIR, TypeIR, TypeKind};

fn greet(name: &str) -> FunctionIR {
    let mut f = function(
        name,
        vec![
            arg("name", "str", None, "The person's name."),
            arg("excited", "bool", Some("False"), "Add exclamation."),
        ],
        Some(ret("str", "A greeting string.")),
    );
    f.docstring = doc("Greet a person by name.");
    f
}

fn utils_module() -> ModuleIR {
    let mut m = module("mylib.utils", vec![], vec![greet("greet")]);
    m.docstring = doc("Utility functions.");
    m.source_file = "src/mylib/utils.py".to_string();
    m
}

fn calculator() -> ClassIR {
    let mut c = class("Calculator", &["BaseCalc"], vec![greet("add")]);
    c.decorators = vec!["dataclass".to_string()];
    c.docstring = doc("A simple calculator.");
    c.line_number = 10;
    c
}

fn point(kind: TypeKind, name: &str) -> TypeIR {
    let mut norm = function("norm", vec![], Some(ret("f64", "Length.")));
    norm.docstring = doc("Vector length.");
    norm.source_file = "src/geo.rs".to_string();
    norm.line_number = 12;
    norm.kind = FunctionKind::Method;
    norm.signature = "pub fn norm(&self) -> f64".to_string();
    TypeIR {
        docstring: DocstringIR {
            short_description: "A 2D point.".to_string(),
            long_description: "Has {braces} and <angles>.".to_string(),
            ..DocstringIR::default()
        },
        fields: vec![
            var("x", "f64", "Horizontal."),
            var("tag", "A | B", "Union | typed."),
        ],
        methods: vec![norm],
        signature: "pub struct Point".to_string(),
        source_file: "src/geo.rs".to_string(),
        line_number: 3,
        ..type_item(name, kind)
    }
}

fn geo(types: Vec<TypeIR>, functions: Vec<FunctionIR>) -> ModuleIR {
    let mut m = module("geo", vec![], functions);
    m.docstring = doc("Geometry.");
    m.source_file = "src/geo.rs".to_string();
    m.language = Language::Rust;
    m.types = types;
    m
}

fn with_repo(repo_url: &str) -> RenderOptions<'_> {
    RenderOptions {
        repo_url,
        ..RenderOptions::default()
    }
}

#[test]
fn yields_render_the_item_type_without_replacing_the_signature_or_returns() {
    let mut yielded = ret("int", "Each value.");
    yielded.is_generator = true;
    yielded.return_annotation = Some("Iterator[int]".into());
    let f = function("values", vec![], Some(yielded));
    let ordinary = function("name", vec![], Some(ret("str", "The name.")));
    let m = module("streams", vec![], vec![f, ordinary]);
    let mdx = module_to_mdx(&m, &RenderOptions::default());
    assert!(mdx.contains("def values() -> Iterator[int]"));
    assert!(mdx.contains("**Yields:** `int` - Each value."));
    assert!(!mdx.contains("**Yields:** `Iterator[int]`") && !mdx.contains("**Returns:** `int`"));
    assert!(mdx.contains("**Returns:** `str` - The name."));
}

#[test]
fn module_page_matches_the_spec_example_byte_for_byte() {
    let mdx = module_to_mdx(&utils_module(), &with_repo("https://github.com/acme/mylib"));
    let expected = r#"---
description: Utility functions.
title: mylib.utils
---

# mylib.utils <SourceLink href="https://github.com/acme/mylib/blob/main/src/mylib/utils.py#L1" />

Utility functions.

## Functions

### `greet` <SourceLink href="https://github.com/acme/mylib/blob/main/test.py#L1" /> [#greet]

```python
def greet(name: str, excited: bool = False) -> str
```

Greet a person by name.

<ParamTable args={[
  {
    "name": "name",
    "type": "str",
    "default": "",
    "description": "The person's name."
  },
  {
    "name": "excited",
    "type": "bool",
    "default": "False",
    "description": "Add exclamation."
  }
]} />

**Returns:** `str` - A greeting string.
"#;
    assert_eq!(mdx, expected);
}

#[test]
fn source_links_follow_the_configured_ref_and_root() {
    let mdx = module_to_mdx(
        &utils_module(),
        &RenderOptions {
            repo_url: "https://github.com/acme/mylib/",
            source_ref: Some("release/2.x"),
            ..RenderOptions::default()
        },
    );
    assert!(mdx
        .contains("href=\"https://github.com/acme/mylib/blob/release/2.x/src/mylib/utils.py#L1\""));
    assert!(mdx.contains("href=\"https://github.com/acme/mylib/blob/release/2.x/test.py#L1\""));
    assert!(!mdx.contains("/blob/main/"));

    assert_eq!(source_link("", "a.py", 1, "", None), "");
    assert_eq!(
        source_link("https://x/r", "/proj/src/a.py", 7, "/proj/", Some("  ")),
        " <SourceLink href=\"https://x/r/blob/main/src/a.py#L7\" />"
    );
    assert_eq!(
        source_link("https://x/r", "/other/a.py", 7, "/proj/", None),
        " <SourceLink href=\"https://x/r/blob/main//other/a.py#L7\" />"
    );
    let plain = module_to_mdx(&utils_module(), &RenderOptions::default());
    assert!(!plain.contains("SourceLink"));
    assert!(plain.starts_with(
        "---\ndescription: Utility functions.\ntitle: mylib.utils\n---\n\n# mylib.utils\n\n"
    ));
}

#[test]
fn class_renders_an_overview_card_methods_and_inner_classes() {
    let mut inner = class("InnerHelper", &[], vec![greet("nested")]);
    inner.docstring = doc("An inner class.");
    let mut outer = calculator();
    outer.inner_classes = vec![inner];
    let mut m = module("mylib.calc", vec![outer], vec![]);
    m.docstring = doc("Calculator module.");
    let mdx = module_to_mdx(&m, &with_repo("https://github.com/acme/mylib"));
    assert!(mdx.contains(
            "## Classes\n\n<span id=\"calculator\" />\n\n<ClassOverview name=\"Calculator\" bases={[\"BaseCalc\"]} decorators={[\"dataclass\"]} /> <SourceLink href=\"https://github.com/acme/mylib/blob/main/test.py#L10\" />\n\nA simple calculator.\n\n### `add` <SourceLink href=\"https://github.com/acme/mylib/blob/main/test.py#L1\" /> [#calculator-add]"
        ));
    assert!(mdx
        .contains("<span id=\"calculator-innerhelper\" />\n\n<ClassOverview name=\"InnerHelper\" bases={[]} decorators={[]} /> <SourceLink"));
    assert!(mdx.contains("An inner class.\n\n#### `nested` <SourceLink"));
    assert!(mdx.contains(" [#calculator-innerhelper-nested]\n"));
    assert!(!mdx.contains("## Functions"));

    let mut nameless_source = calculator();
    nameless_source.source_file.clear();
    let m = module("t", vec![nameless_source], vec![]);
    let mdx = module_to_mdx(&m, &with_repo("https://github.com/acme/mylib"));
    assert!(mdx.contains("decorators={[\"dataclass\"]} />\n\nA simple calculator."));
}

#[test]
fn a_static_member_sharing_an_instance_name_has_its_own_id() {
    let mut on_class = function("v", vec![], None);
    on_class.visibility = "static".to_string();
    let only_static = {
        let mut f = function("make", vec![], None);
        f.visibility = "static".to_string();
        f
    };
    let m = module(
        "t",
        vec![class(
            "Box",
            &[],
            vec![function("v", vec![], None), on_class, only_static],
        )],
        vec![],
    );
    let mdx = module_to_mdx(&m, &RenderOptions::default());
    assert!(mdx.contains(" [#box-v]\n"));
    assert!(mdx.contains(" [#box-static-v]\n"));
    assert!(mdx.contains(" [#box-make]\n"));
}

#[test]
fn class_name_is_attribute_escaped() {
    let m = module(
        "t",
        vec![class("Foo\" onclick=\"alert(1)", &[], vec![])],
        vec![],
    );
    let mdx = module_to_mdx(&m, &RenderOptions::default());
    assert!(!mdx.contains("name=\"Foo\" onclick=\"alert(1)\""));
    assert!(mdx.contains("name=\"Foo&quot; onclick=&quot;alert(1)\""));
}

#[test]
fn function_kinds_badges_signatures_and_tails() {
    let mut fetch = greet("fetch_data");
    fetch.is_async = true;
    let mut value = function("value", vec![], Some(ret("int", "The value.")));
    value.kind = FunctionKind::Property;
    value.docstring = doc("Get the value.");
    let mut create = function("create", vec![], None);
    create.kind = FunctionKind::Staticmethod;
    let mut from_dict = function("from_dict", vec![arg("cls", "", None, "")], None);
    from_dict.kind = FunctionKind::Classmethod;
    let mut raises = function("boom", vec![], Some(ret("str", "")));
    raises.raises = vec![RaiseIR {
        exception: "ValueError".to_string(),
        description: "If {bad} or <odd>.".to_string(),
    }];
    raises.docstring.examples = vec!["boom()\n{1}".to_string()];
    raises.docstring.long_description = "Long {text}.".to_string();
    let m = module(
        "test",
        vec![],
        vec![fetch, value, create, from_dict, raises],
    );
    let mdx = module_to_mdx(&m, &RenderOptions::default());

    assert!(mdx
        .contains("```python\nasync def fetch_data(name: str, excited: bool = False) -> str\n```"));
    assert!(mdx.contains("`@property`\n\n### `value` [#value]\n\n```python\nvalue\n```\n\nGet the value.\n\n**Type:** `int` - The value.\n"));
    assert!(!mdx.contains("def value("));
    assert!(!mdx.contains("**Returns:** `int`"));
    assert!(
        mdx.contains("`@staticmethod`\n\n### `create` [#create]\n\n```python\ndef create()\n```\n")
    );
    assert!(mdx.contains("`@classmethod`\n\n### `from_dict` [#from_dict]\n\n```python\ndef from_dict(cls)\n```\n\n<ParamTable args={[\n  {\n    \"name\": \"cls\",\n    \"type\": \"Any\",\n    \"default\": \"\",\n    \"description\": \"\"\n  }\n]} />\n"));
    assert!(mdx.contains("Long \\{text\\}.\n\n**Returns:** `str`\n\n**Raises:** `ValueError` - If \\{bad\\} or &lt;odd&gt;.\n\n```python\nboom()\n{1}\n```\n"));
    assert!(mdx.ends_with("```\n"));
}

#[test]
fn param_table_names_var_args_and_keeps_utf8() {
    let mut process = function("process", vec![], None);
    process.args = vec![
        arg("x", "int", None, "First."),
        ArgIR {
            kind: folioh_ir::ArgKind::VarPositional,
            ..arg("args", "Any", None, "Positional.")
        },
        ArgIR {
            kind: folioh_ir::ArgKind::VarKeyword,
            ..arg("kwargs", "Any", None, "Café \"quoted\" é.")
        },
    ];
    let table = render_param_table(&process, None, "");
    assert!(table.starts_with("<ParamTable args={[\n"));
    assert!(table.ends_with("\n]} />"));
    assert!(table.contains("\"name\": \"*args\""));
    assert!(table.contains("\"name\": \"**kwargs\""));
    assert!(table.contains("\"description\": \"Café \\\"quoted\\\" é.\""));
    assert!(!table.contains("\\u00e9"));
    assert_eq!(
        render_param_table(&function("f", vec![], None), None, ""),
        ""
    );
    let mdx = module_to_mdx(
        &module("test", vec![], vec![process]),
        &RenderOptions::default(),
    );
    assert!(mdx.contains("def process(x: int, *args: Any, **kwargs: Any)"));
}

#[test]
fn cross_references_link_params_returns_and_bases() {
    let config_mod = module("mylib.config", vec![class("Config", &[], vec![])], vec![]);
    let mut process = function(
        "process",
        vec![
            arg("cfg", "Config", None, "The config."),
            arg("name", "str", None, "A name."),
        ],
        Some(ret("Config", "Updated config.")),
    );
    process.docstring = doc("Process something.");
    let core_mod = module("mylib.core", vec![], vec![process]);
    let mut special = class("SpecialConfig", &["Config"], vec![]);
    special.docstring = doc("A special config.");
    let special_mod = module("mylib.special", vec![special], vec![]);
    let index = build_symbol_index(
        &[config_mod, core_mod.clone(), special_mod.clone()],
        "/docs",
    );
    let opts = RenderOptions {
        symbol_index: Some(&index),
        ..RenderOptions::default()
    };

    let core = module_to_mdx(&core_mod, &opts);
    let start = core.find("<ParamTable args={").unwrap() + "<ParamTable args={".len();
    let end = core[start..].find("} />").unwrap() + start;
    let args: serde_json::Value = serde_json::from_str(&core[start..end]).unwrap();
    assert_eq!(args[0]["href"], "/docs/api-reference/mylib/config#config");
    assert_eq!(args[1]["type"], "str");
    assert!(args[1].get("href").is_none());
    assert!(core.contains(
        "**Returns:** [`Config`](/docs/api-reference/mylib/config#config) - Updated config."
    ));

    let special = module_to_mdx(&special_mod, &opts);
    assert!(special.contains(
            "<span id=\"specialconfig\" />\n\n<ClassOverview name=\"SpecialConfig\" bases={[{\"name\": \"Config\", \"href\": \"/docs/api-reference/mylib/config#config\"}]} decorators={[]} />"
        ));

    let plain = module_to_mdx(&core_mod, &RenderOptions::default());
    assert!(!plain.contains("href"));
    assert!(plain.contains("**Returns:** `Config` - Updated config."));
    assert!(module_to_mdx(&special_mod, &RenderOptions::default()).contains("bases={[\"Config\"]}"));
}

#[test]
fn non_python_modules_link_locally_and_across_languages() {
    let mut greet = function(
        "greet",
        vec![arg("widget", "Widget", None, "")],
        Some(ret("Options", "")),
    );
    greet.signature = "export function greet(widget)".to_string();
    greet.source_file = "lib/util.js".to_string();
    let js = ModuleIR {
        language: Language::Javascript,
        types: vec![type_item("Options", TypeKind::Interface)],
        ..module("lib.util", vec![class("Widget", &[], vec![])], vec![greet])
    };
    let index = build_symbol_index(std::slice::from_ref(&js), "/docs");
    let mdx = module_to_mdx(
        &js,
        &RenderOptions {
            symbol_index: Some(&index),
            ..RenderOptions::default()
        },
    );
    assert!(mdx.contains("\"href\": \"/docs/api-reference/javascript/lib/util#widget\""));
    assert!(mdx.contains(
        "**Returns:** [`Options`](/docs/api-reference/javascript/lib/util#interface-options)\n"
    ));
    assert!(mdx.contains("```javascript\nexport function greet(widget)\n```"));
    assert!(!mdx.contains("def greet"));
    assert!(mdx.contains("<ParamTable args={["));
    assert!(mdx.contains("## Types\n\n### Interface `Options` [#interface-options]\n"));
}

#[test]
fn types_render_after_functions_with_signature_tables_and_methods() {
    let mut area = function("area", vec![], None);
    area.docstring = doc("Area.");
    area.source_file = "src/geo.rs".to_string();
    area.signature = "pub fn area() -> f64".to_string();
    let mdx = module_to_mdx(
        &geo(vec![point(TypeKind::Struct, "Point")], vec![area]),
        &with_repo("https://github.com/acme/geo"),
    );
    assert!(mdx.find("## Functions").unwrap() < mdx.find("## Types").unwrap());
    let mirror = folioh_mdx::mdx_to_markdown(&mdx);
    for kept in [
        "### Struct `Point`",
        "pub struct Point",
        "| `x` | `f64` | Horizontal. |",
        "#### `norm`",
    ] {
        assert!(mirror.contains(kept), "mirror lost {kept:?}:\n{mirror}");
    }
    assert!(mdx.contains(
            "## Types\n\n### Struct `Point` <SourceLink href=\"https://github.com/acme/geo/blob/main/src/geo.rs#L3\" /> [#struct-point]\n\n```rust\npub struct Point\n```\n\nA 2D point.\n\nHas \\{braces\\} and &lt;angles&gt;.\n\n| Field | Type | Description |\n| --- | --- | --- |\n| `x` | `f64` | Horizontal. |\n| `tag` | `A \\| B` | Union \\| typed. |\n\n#### `norm` <SourceLink href=\"https://github.com/acme/geo/blob/main/src/geo.rs#L12\" /> [#struct-point-norm]\n\n```rust\npub fn norm(&self) -> f64\n```\n\nVector length.\n\n**Returns:** `f64` - Length.\n"
        ));
}

#[test]
fn type_headings_use_the_kind_label() {
    for (kind, label) in [
        (TypeKind::Struct, "Struct"),
        (TypeKind::Enum, "Enum"),
        (TypeKind::Trait, "Trait"),
        (TypeKind::Interface, "Interface"),
        (TypeKind::TypeAlias, "Type"),
        (TypeKind::Union, "Union"),
        (TypeKind::Impl, "Impl"),
    ] {
        let mdx = module_to_mdx(
            &geo(vec![point(kind, "T")], vec![]),
            &RenderOptions::default(),
        );
        assert!(mdx.contains(&format!("### {label} `T` [#")), "{label}");
    }
}

#[test]
fn enum_variants_render_as_a_table_and_examples_close_the_type() {
    let mut shape = type_item("Shape", TypeKind::Enum);
    shape.docstring = DocstringIR {
        short_description: "A shape.".to_string(),
        examples: vec!["Shape::Circle(1.0)".to_string()],
        ..DocstringIR::default()
    };
    shape.variants = vec![var("Circle", "f64", "Round."), var("Square", "", "")];
    let mdx = module_to_mdx(&geo(vec![shape], vec![]), &RenderOptions::default());
    assert!(mdx.contains(
            "### Enum `Shape` [#enum-shape]\n\nA shape.\n\n| Variant | Type | Description |\n| --- | --- | --- |\n| `Circle` | `f64` | Round. |\n| `Square` |  |  |\n\n```rust\nShape::Circle(1.0)\n```\n"
        ));
    assert!(!mdx.contains("| Field |"));
    assert!(!mdx.contains("SourceLink"));
}

#[test]
fn constants_render_as_a_table_and_python_modules_have_no_types_section() {
    let mut m = utils_module();
    let mut version = var("VERSION", "", "");
    version.value = "\"1.0.0\"".to_string();
    let mut table = var("TABLE", "dict", "Lookup | table.");
    table.value = format!("{{{}}}", "'k': 1, ".repeat(20));
    m.constants = vec![var("MAX", "int", "Limit."), version, table];
    let mdx = module_to_mdx(&m, &RenderOptions::default());
    assert!(!mdx.contains("## Types"));
    assert!(mdx.contains(
        "## Constants\n\n| Constant | Type | Value | Description |\n| --- | --- | --- | --- |\n| `MAX` | `int` |  | Limit. |\n| `VERSION` |  | `\"1.0.0\"` |  |\n| `TABLE` | `dict` | `{'k': 1, 'k': 1, 'k': 1, 'k': 1, 'k': 1, 'k': 1, 'k': 1, 'k'…` | Lookup \\| table. |\n\n## Functions"
    ));
    assert!(mdx.contains("```python\ndef greet("));
}

#[test]
fn a_rust_pub_use_is_a_re_export_and_attributes_sit_above_the_signature() {
    let mut reexport = var(
        "pub use std::collections::HashMap;",
        folioh_ir::REEXPORT,
        "",
    );
    reexport.value.clear();
    let mut conditional = function("conditional", vec![], None);
    conditional.signature = "pub fn conditional()".to_string();
    conditional.decorators = vec!["#[cfg(feature = \"extra\")]".to_string()];
    let mut point = type_item("Point", TypeKind::Struct);
    point.signature = "pub struct Point".to_string();
    point.bases = vec!["#[derive(Debug, Clone)]".to_string()];
    let mut display = type_item("Point", TypeKind::Impl);
    display.signature = "impl fmt::Display for Point".to_string();
    display.bases = vec!["fmt::Display".to_string()];
    let mut m = geo(vec![point, display], vec![conditional]);
    m.constants = vec![reexport];
    let mdx = module_to_mdx(&m, &RenderOptions::default());
    assert!(mdx.contains("## Re-exports\n\n```rust\npub use std::collections::HashMap;\n```\n"));
    assert!(!mdx.contains("## Constants"));
    assert!(mdx.contains("```rust\n#[cfg(feature = \"extra\")]\npub fn conditional()\n```\n"));
    assert!(mdx.contains("```rust\n#[derive(Debug, Clone)]\npub struct Point\n```\n"));
    assert!(mdx.contains("```rust\nimpl fmt::Display for Point\n```\n"));
    assert!(mdx.contains("### impl `fmt::Display` for `Point`"));
}

#[test]
fn class_attributes_notes_and_custom_decorators_render() {
    let mut handle = function("handle", vec![], None);
    handle.kind = FunctionKind::Method;
    handle.decorators = vec!["retry(max_attempts=3)".to_string()];
    let mut size = function("size", vec![], Some(ret("int", "")));
    size.kind = FunctionKind::Property;
    size.decorators = vec!["property".to_string(), "functools.cache".to_string()];
    let mut server = class("Server", &[], vec![handle, size]);
    server.docstring = DocstringIR {
        short_description: "An HTTP server.".to_string(),
        examples: vec![">>> Server()".to_string()],
        notes: vec!["Not {thread} safe.".to_string()],
        ..DocstringIR::default()
    };
    let mut host = var("host", "str", "Where it listens.");
    host.value = "\"localhost\"".to_string();
    server.class_vars = vec![host];
    let mut m = module("net", vec![server], vec![]);
    m.docstring = DocstringIR {
        short_description: "Networking.".to_string(),
        examples: vec![">>> import net".to_string()],
        notes: vec!["Experimental.".to_string()],
        ..DocstringIR::default()
    };
    let mdx = module_to_mdx(&m, &RenderOptions::default());
    assert!(mdx.contains(
        "Networking.\n\n```python\n>>> import net\n```\n\n**Notes:**\n\nExperimental.\n\n## Classes"
    ));
    assert!(mdx.contains(
        "An HTTP server.\n\n```python\n>>> Server()\n```\n\n**Notes:**\n\nNot \\{thread\\} safe.\n\n| Attribute | Type | Value | Description |\n| --- | --- | --- | --- |\n| `host` | `str` | `\"localhost\"` | Where it listens. |\n\n### `handle`"
    ));
    assert!(mdx.contains("```python\n@retry(max_attempts=3)\ndef handle()\n```"));
    assert!(mdx.contains(
        "`@property`\n\n### `size` [#server-size]\n\n```python\n@functools.cache\nsize\n```"
    ));
}

#[test]
fn docstring_headings_sit_below_the_item_they_document() {
    let mut run = function("run", vec![], None);
    run.docstring = DocstringIR {
        short_description: "Runs.".to_string(),
        long_description: "# Panics\n\nNever.\n\n```\n# fenced\n```".to_string(),
        ..DocstringIR::default()
    };
    let mut m = module("app", vec![], vec![run]);
    m.docstring = doc("App.");
    m.docstring.long_description = "# Overview".to_string();
    let mdx = module_to_mdx(&m, &RenderOptions::default());
    assert!(mdx.contains("\n## Overview\n"), "{mdx}");
    assert!(
        mdx.contains("#### Panics\n\nNever.\n\n```\n# fenced\n```"),
        "{mdx}"
    );
}

#[test]
fn a_return_or_raise_without_a_type_reads_as_its_description() {
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
    let mdx = module_to_mdx(
        &module("app", vec![], vec![fetch, empty]),
        &RenderOptions::default(),
    );
    assert!(mdx.contains("**Returns:** The thing.\n"), "{mdx}");
    assert!(mdx.contains("**Raises:** When it fails.\n"), "{mdx}");
    assert!(!mdx.contains(":** ``"), "{mdx}");
    assert_eq!(mdx.matches("**Raises:**").count(), 1, "{mdx}");
    assert_eq!(mdx.matches("**Returns:**").count(), 1, "{mdx}");
}

#[test]
fn api_reference_index_lists_modules_sorted_by_name() {
    let mut core = module("mylib.core", vec![calculator()], vec![greet("normalize")]);
    core.docstring = doc("Core helpers");
    let mut root = module("mylib", vec![], vec![]);
    root.docstring = doc("Core package");
    let rust = geo(vec![point(TypeKind::Struct, "Point")], vec![]);
    let mdx = api_reference_index_to_mdx(&[rust, core, root]);
    let expected = r#"---
asIndexPage: true
description: Generated source code documentation for project modules.
title: Source Code
---

<ApiReferenceIndex modules={[
  {
    "name": "geo",
    "description": "Geometry.",
    "href": "./rust/geo/",
    "classCount": 0,
    "functionCount": 0,
    "typeCount": 1
  },
  {
    "name": "mylib",
    "description": "Core package",
    "href": "./mylib/",
    "classCount": 0,
    "functionCount": 0,
    "typeCount": 0
  },
  {
    "name": "mylib.core",
    "description": "Core helpers",
    "href": "./mylib/core/",
    "classCount": 1,
    "functionCount": 1,
    "typeCount": 0
  }
]} />
"#;
    assert_eq!(mdx, expected);
    let undocumented = api_reference_index_to_mdx(&[module("m", vec![], vec![])]);
    assert!(undocumented.contains("\"description\": \"\""));
    assert!(!undocumented.contains("Module documentation."));
    assert_eq!(
            api_reference_index_to_mdx(&[]),
            "---\nasIndexPage: true\ndescription: Generated source code documentation for project modules.\ntitle: Source Code\n---\n\n# Source Code\n\nNo source modules were found.\n"
        );
}

#[test]
fn dunder_names_in_api_prose_are_literal() {
    let mut m = utils_module();
    m.docstring = doc("Call __init__ before __call__. Keep `__new__` as code.");
    let mdx = module_to_mdx(&m, &RenderOptions::default());
    assert!(mdx.contains(r"Call \_\_init\_\_ before \_\_call\_\_. Keep `__new__` as code."));
}

/// Every `#id` the symbol index points at, written on the page it names:
/// as a heading's `[#id]` or on the element before a class card.
#[test]
fn every_index_anchor_is_an_id_on_its_page() {
    let mut inner = class("Inner", &[], vec![greet("nested")]);
    inner.docstring = doc("Inner.");
    let mut outer = calculator();
    outer.inner_classes = vec![inner];
    let python = module("mylib.calc", vec![outer], vec![greet("add")]);
    let mut impl_block = point(TypeKind::Impl, "Point");
    impl_block.bases = vec!["fmt::Display".to_string()];
    let rust = geo(
        vec![point(TypeKind::Struct, "Point"), impl_block],
        vec![function("area", vec![], None)],
    );
    let javascript = ModuleIR {
        language: Language::Javascript,
        types: vec![type_item("Options", TypeKind::Interface)],
        ..module(
            "lib.util",
            vec![class("Widget", &[], vec![])],
            vec![greet("greet")],
        )
    };
    let modules = [python, rust, javascript];
    let index = build_symbol_index(&modules, "/docs");
    let opts = RenderOptions {
        repo_url: "https://github.com/acme/x",
        symbol_index: Some(&index),
        ..RenderOptions::default()
    };
    let pages: Vec<(String, String)> = modules
        .iter()
        .map(|m| {
            (
                format!("/docs/{}", module_route(m)),
                module_to_mdx(m, &opts),
            )
        })
        .collect();
    let mut checked = 0;
    for (fqn, url) in &index {
        let Some((route, anchor)) = url.split_once('#') else {
            continue;
        };
        let (_, page) = pages.iter().find(|(r, _)| r == route).expect("a page");
        let as_heading = format!(" [#{anchor}]\n");
        let as_card = format!("<span id=\"{anchor}\" />\n\n<ClassOverview ");
        assert!(
            page.contains(&as_heading) || page.contains(&as_card),
            "{fqn} -> #{anchor} is on no heading or card of {route}"
        );
        assert_eq!(
            page.matches(&as_heading).count() + page.matches(&as_card).count(),
            1
        );
        checked += 1;
    }
    assert_eq!(checked, 8, "{index:#?}");
    // Two impls of one type keep apart; the struct keeps its own id.
    let geo_page = &pages[1].1;
    assert!(geo_page.contains("#L3\" /> [#impl-fmt-display-for-point]\n"));
    assert!(geo_page.contains(" [#impl-fmt-display-for-point-norm]\n"));
    assert!(geo_page.contains(" [#struct-point-norm]\n"));
}

#[test]
fn member_types_are_code_as_written() {
    let mut shape = type_item("Shape", TypeKind::Enum);
    shape.variants = vec![
        var("Failed", "{ code: u32 }", "Carries <the> {code}."),
        var("Many", "(Vec<Item>, `raw`)", ""),
    ];
    let mdx = module_to_mdx(&geo(vec![shape], vec![]), &RenderOptions::default());
    assert!(mdx.contains("| `Failed` | `{ code: u32 }` | Carries &lt;the&gt; \\{code\\}. |\n"));
    assert!(mdx.contains("| `Many` | `` (Vec<Item>, `raw`) `` |  |"));
}

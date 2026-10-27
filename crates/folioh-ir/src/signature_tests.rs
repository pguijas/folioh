use super::*;
use crate::ir::{ArgIR, DocstringIR, FunctionKind, ReturnIR};

fn arg(name: &str, ty: &str, default: Option<&str>, kind: ArgKind) -> ArgIR {
    ArgIR {
        name: name.to_string(),
        ty: ty.to_string(),
        default: default.map(str::to_string),
        description: String::new(),
        kind,
    }
}

fn func(name: &str, args: Vec<ArgIR>, returns: Option<&str>) -> FunctionIR {
    FunctionIR {
        name: name.to_string(),
        args,
        returns: returns.map(|ty| ReturnIR {
            ty: ty.to_string(),
            description: String::new(),
            ..ReturnIR::default()
        }),
        raises: vec![],
        decorators: vec![],
        docstring: DocstringIR::default(),
        is_async: false,
        source_file: String::new(),
        line_number: 0,
        kind: FunctionKind::Function,
        signature: String::new(),
        visibility: String::new(),
    }
}

#[test]
fn yielding_signatures_and_old_serialized_returns_stay_distinct() {
    let old = serde_json::json!({"type": "int", "description": "A value."});
    let plain: ReturnIR = serde_json::from_value(old.clone()).unwrap();
    assert!(!plain.is_generator && plain.return_annotation.is_none());
    assert_eq!(serde_json::to_value(&plain).unwrap(), old);
    for annotation in [Some("Iterator[int]"), None] {
        let mut f = func("values", vec![], Some("int"));
        let value = f.returns.as_mut().unwrap();
        value.is_generator = true;
        value.return_annotation = annotation.map(str::to_string);
        let roundtrip: ReturnIR =
            serde_json::from_value(serde_json::to_value(&*value).unwrap()).unwrap();
        assert_eq!(&roundtrip, value);
        assert_eq!(
            render_signature(&f, true),
            annotation.map_or_else(
                || "def values()".to_string(),
                |ty| format!("def values() -> {ty}")
            )
        );
    }
}

#[test]
fn render_signature_table() {
    use ArgKind::*;
    let cases: Vec<(FunctionIR, &str)> = vec![
        (
            func(
                "log",
                vec![
                    arg("message", "str", None, Regular),
                    arg("args", "Any", None, VarPositional),
                    arg("kwargs", "Any", None, VarKeyword),
                ],
                Some("None"),
            ),
            "def log(message: str, *args: Any, **kwargs: Any) -> None",
        ),
        (
            func(
                "g",
                vec![
                    arg("x", "int", None, PositionalOnly),
                    arg("y", "str", Some("None"), KeywordOnly),
                ],
                Some("str"),
            ),
            "def g(x: int, /, *, y: str = None) -> str",
        ),
        (
            func("h", vec![arg("a", "", None, PositionalOnly)], None),
            "def h(a)",
        ),
        (
            func(
                "f",
                vec![
                    arg("x", "int", None, PositionalOnly),
                    arg("y", "int", None, PositionalOnly),
                    arg("z", "int", Some("0"), Regular),
                ],
                None,
            ),
            "def f(x: int, y: int, /, z: int = 0)",
        ),
        (
            func(
                "f",
                vec![
                    arg("a", "int", None, Regular),
                    arg("args", "", None, VarPositional),
                    arg("key", "bool", Some("False"), KeywordOnly),
                    arg("kwargs", "", None, VarKeyword),
                ],
                None,
            ),
            "def f(a: int, *args, key: bool = False, **kwargs)",
        ),
        (
            func(
                "key",
                vec![arg("key", "str", Some("'default'"), Regular)],
                None,
            ),
            "def key(key: str = 'default')",
        ),
        // Docstring-only `Returns:` without a type: no trailing arrow.
        (func("r", vec![], Some("")), "def r()"),
    ];
    for (function, expected) in cases {
        assert_eq!(render_signature(&function, true), expected);
    }

    let mut fetch = func(
        "fetch",
        vec![arg("url", "str", None, Regular)],
        Some("bytes"),
    );
    fetch.is_async = true;
    assert_eq!(
        render_signature(&fetch, true),
        "async def fetch(url: str) -> bytes"
    );
}

#[test]
fn show_parens_false_is_just_the_name() {
    let p = func("p", vec![], Some("int"));
    assert_eq!(render_signature(&p, false), "p");
    assert_eq!(display_signature(&p, false), "p");
}

#[test]
fn display_signature_prefers_the_raw_signature() {
    let mut init = func("init", vec![], None);
    init.signature = "pub fn init(name: &str) -> bool".to_string();
    assert_eq!(
        display_signature(&init, true),
        "pub fn init(name: &str) -> bool"
    );
    assert_eq!(
        display_signature(&init, false),
        "pub fn init(name: &str) -> bool"
    );
    init.signature.clear();
    assert_eq!(display_signature(&init, true), "def init()");
}

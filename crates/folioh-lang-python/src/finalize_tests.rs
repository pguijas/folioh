use super::*;
use crate::extract::parse_source_raw;
use folioh_ir::FunctionKind;

fn module(source: &str, style: DocstringStyle) -> ModuleIR {
    finalize_module(parse_source_raw(source, "m", "m.py").unwrap(), style)
}

#[test]
fn arguments_take_the_annotation_then_the_doc_type_and_the_doc_description() {
    let m = module(
            "def g(x, /, *, y: str = None) -> str:\n    \"\"\"G.\n\n    Args:\n        x (int): the x\n        y (str, optional): the y. Defaults to None.\n\n    Returns:\n        str: out\n\n    Yields:\n        int: ignored\n    \"\"\"\n",
            DocstringStyle::Auto,
        );
    let g = &m.functions[0];
    assert_eq!(g.args[0].ty, "int");
    assert_eq!(g.args[0].description, "the x");
    assert_eq!(g.args[0].kind, ArgKind::PositionalOnly);
    assert_eq!(g.args[1].ty, "str");
    assert_eq!(g.args[1].default.as_deref(), Some("None"));
    assert_eq!(g.args[1].description, "the y. Defaults to None.");
    let returns = g.returns.as_ref().unwrap();
    assert!(!returns.is_generator);
    assert_eq!(returns.return_annotation, None);
    assert_eq!(
        (returns.ty.as_str(), returns.description.as_str()),
        ("str", "out")
    );
    assert_eq!(g.signature, "");
    assert_eq!(g.visibility, "");
    assert_eq!(m.language, Language::Python);
    assert!(m.types.is_empty());
}

#[test]
fn yields_keep_the_item_type_separate_from_the_return_annotation() {
    for (style, text) in [
        (DocstringStyle::Google, "Yields:\n    int: Each value."),
        (
            DocstringStyle::Numpy,
            "Yields\n------\nint\n    Each value.",
        ),
        (DocstringStyle::Auto, ":yields int: Each value."),
    ] {
        let body = text
            .lines()
            .map(|line| format!("    {line}"))
            .collect::<Vec<_>>()
            .join("\n");
        for annotation in [" -> Iterator[int]", ""] {
            let source = format!(
                "def values(){annotation}:\n    \"\"\"Values.\n\n{body}\n    \"\"\"\n    yield 1\n"
            );
            let m = module(&source, style);
            let f = &m.functions[0];
            let value = f.returns.as_ref().unwrap();
            assert!(value.is_generator, "{style:?}");
            assert_eq!(value.ty, "int", "{style:?}");
            assert_eq!(value.description, "Each value.");
            assert_eq!(
                value.return_annotation.as_deref(),
                (!annotation.is_empty()).then_some("Iterator[int]")
            );
            assert_eq!(
                folioh_ir::signature::display_signature(f, true),
                format!("def values(){annotation}")
            );
        }
    }
    let m = module("def values() -> Iterator[int]:\n    \"\"\"Values.\n\n    Yields:\n        Each value.\n    \"\"\"\n    yield 1\n", DocstringStyle::Google);
    let value = m.functions[0].returns.as_ref().unwrap();
    assert!(value.is_generator);
    assert!(
        value.ty.is_empty(),
        "an iterator annotation is not the yielded item type"
    );
    assert_eq!(value.return_annotation.as_deref(), Some("Iterator[int]"));
}

#[test]
fn starred_doc_names_match_var_args() {
    let m = module(
            "def log(message: str, *args: Any, **kwargs: Any) -> None:\n    \"\"\"Log.\n\n    Args:\n        message: The template.\n        *args: Positional.\n        **kwargs: Keywords.\n\n    Returns:\n        Nothing useful.\n    \"\"\"\n",
            DocstringStyle::Google,
        );
    let log = &m.functions[0];
    assert_eq!(log.args[1].description, "Positional.");
    assert_eq!(log.args[2].description, "Keywords.");
    let returns = log.returns.as_ref().unwrap();
    assert_eq!(returns.ty, "None");
    assert_eq!(returns.description, "Nothing useful.");
}

#[test]
fn returns_raises_and_docstring_only_returns() {
    let m = module(
            "def f(a):\n    \"\"\"F.\n\n    Returns:\n        the thing\n\n    Raises:\n        ValueError: If empty.\n        KeyError: Missing.\n    \"\"\"\n\ndef g():\n    \"\"\"No returns at all.\"\"\"\n",
            DocstringStyle::Google,
        );
    let f = &m.functions[0];
    let returns = f.returns.as_ref().unwrap();
    assert_eq!(returns.ty, "");
    assert_eq!(returns.description, "the thing");
    assert_eq!(
        f.raises
            .iter()
            .map(|r| (r.exception.as_str(), r.description.as_str()))
            .collect::<Vec<_>>(),
        [("ValueError", "If empty."), ("KeyError", "Missing.")]
    );
    assert_eq!(m.functions[1].returns, None);
}

#[test]
fn the_attributes_section_describes_the_class_and_module_variables() {
    let m = module(
            "\"\"\"Mod.\n\nAttributes:\n    X: The limit.\n\"\"\"\n\nX: int = 1\nY = 2\n\nclass C:\n    \"\"\"Cls.\n\n    Attributes:\n        v (str): Declared and documented.\n        RED: An enum member.\n        w (float): Set in __init__.\n    \"\"\"\n\n    v: int = 1\n    RED = 1\n    plain = 2\n    _hidden = 3\n",
            DocstringStyle::Google,
        );
    assert_eq!(m.constants[0].description, "The limit.");
    assert_eq!(m.constants[1].description, "");
    let vars: Vec<(&str, &str, &str, &str)> = m.classes[0]
        .class_vars
        .iter()
        .map(|v| {
            (
                v.name.as_str(),
                v.ty.as_str(),
                v.value.as_str(),
                v.description.as_str(),
            )
        })
        .collect();
    assert_eq!(
        vars,
        [
            ("v", "int", "1", "Declared and documented."),
            ("RED", "", "1", "An enum member."),
            ("plain", "", "2", ""),
            ("w", "float", "", "Set in __init__."),
        ]
    );
    assert_eq!(m.classes[0].docstring.short_description, "Cls.");
}

#[test]
fn every_style_value_parses_and_numpy_merges() {
    let source = "def add(a, b):\n    \"\"\"Add values.\n\n    Parameters\n    ----------\n    a : int\n        First value.\n    b : int\n        Second value.\n    Returns\n    -------\n    int\n        Sum.\n    \"\"\"\n    return a + b\n";
    for style in [
        DocstringStyle::Google,
        DocstringStyle::Numpy,
        DocstringStyle::Auto,
    ] {
        let m = module(source, style);
        assert_eq!(m.functions[0].name, "add");
    }
    let add = &module(source, DocstringStyle::Numpy).functions[0];
    assert_eq!(add.args[0].description, "First value.");
    assert_eq!(add.args[0].ty, "int");
    assert_eq!(add.returns.as_ref().unwrap().description, "Sum.");
    // Forced Google sees no sections: the whole body is the long description.
    let google = &module(source, DocstringStyle::Google).functions[0];
    assert_eq!(google.args[0].description, "");
    assert_eq!(google.returns, None);
}

const SAMPLE: &str = r#"
"""Sample module for testing."""


def greet(name: str, excited: bool = False) -> str:
    """Greet a person by name.

    Args:
        name: The person's name.
        excited: Whether to add an exclamation mark.

    Returns:
        A greeting string.

    Raises:
        ValueError: If name is empty.

    Example:
        >>> greet("World")
        'Hello, World.'
    """
    if not name:
        raise ValueError("name cannot be empty")
    end = "!" if excited else "."
    return f"Hello, {name}{end}"


class Calculator:
    """A simple calculator.

    Args:
        precision: Number of decimal places.

    Example:
        >>> calc = Calculator(precision=2)
        >>> calc.add(1.1, 2.2)
        3.3
    """

    def __init__(self, precision: int = 2) -> None:
        self.precision = precision

    def add(self, a: float, b: float) -> float:
        """Add two numbers.

        Args:
            a: First number.
            b: Second number.

        Returns:
            The sum rounded to precision.
        """
        return round(a + b, self.precision)

    async def add_async(self, a: float, b: float) -> float:
        """Async version of add.

        Args:
            a: First number.
            b: Second number.
        """
        return self.add(a, b)
"#;

fn google(source: &str) -> ModuleIR {
    finalize_module(
        parse_source_raw(source, "mod", "mod.py").expect("valid source"),
        DocstringStyle::Google,
    )
}

#[test]
fn parse_function_merges_google_sections() {
    let module = google(SAMPLE);
    assert_eq!(module.name, "mod");
    assert_eq!(
        module.docstring.short_description,
        "Sample module for testing."
    );
    assert_eq!(module.functions.len(), 1);
    let greet = &module.functions[0];
    assert_eq!(greet.name, "greet");
    assert_eq!(greet.kind, FunctionKind::Function);
    assert_eq!(
        (greet.args[0].name.as_str(), greet.args[0].ty.as_str()),
        ("name", "str")
    );
    assert_eq!(greet.args[0].description, "The person's name.");
    assert_eq!(
        (greet.args[1].name.as_str(), greet.args[1].ty.as_str()),
        ("excited", "bool")
    );
    assert_eq!(greet.args[1].default.as_deref(), Some("False"));
    let returns = greet.returns.as_ref().unwrap();
    assert_eq!(returns.ty, "str");
    assert_eq!(returns.description, "A greeting string.");
    assert_eq!(greet.raises.len(), 1);
    assert_eq!(greet.raises[0].exception, "ValueError");
    assert_eq!(greet.raises[0].description, "If name is empty.");
    assert!(!greet.is_async);
    assert_eq!(
        greet.docstring.examples,
        [">>> greet(\"World\")\n'Hello, World.'"]
    );
    assert_eq!(greet.line_number, 5);
    assert_eq!(greet.source_file, "mod.py");
}

#[test]
fn parse_class_keeps_dunder_methods_and_async() {
    let module = google(SAMPLE);
    assert_eq!(module.classes.len(), 1);
    let cls = &module.classes[0];
    assert_eq!(cls.name, "Calculator");
    assert_eq!(cls.docstring.short_description, "A simple calculator.");
    assert_eq!(cls.methods.len(), 3);
    let init = cls.methods.iter().find(|m| m.name == "__init__").unwrap();
    assert_eq!(
        (init.args[0].name.as_str(), init.args[0].ty.as_str()),
        ("precision", "int")
    );
    let add = cls.methods.iter().find(|m| m.name == "add").unwrap();
    assert_eq!(add.returns.as_ref().unwrap().ty, "float");
    assert!(!add.is_async);
    assert_eq!(add.kind, FunctionKind::Method);
    let add_async = cls.methods.iter().find(|m| m.name == "add_async").unwrap();
    assert!(add_async.is_async);
}

#[test]
fn private_members_undocumented_dunders_and_overload_stubs_stay_out() {
    let module = google(
            "\nfrom typing import overload\n\n@overload\ndef parse(x: int) -> int: ...\n@overload\ndef parse(x: str) -> str: ...\ndef parse(x):\n    \"\"\"Parse.\"\"\"\n\n@overload\ndef stub(x: int) -> int: ...\n@overload\ndef stub(x: str) -> str: ...\n\nclass C:\n    _count: int = 0\n    __slots__ = ()\n\n    class _Inner:\n        pass\n\n    def __init__(self):\n        pass\n\n    def __repr__(self):\n        return ''\n\n    def __eq__(self, other):\n        \"\"\"Equal by value.\"\"\"\n\n    def _helper(self):\n        pass\n\n    def __mangled(self):\n        pass\n",
        );
    let functions: Vec<(&str, usize)> = module
        .functions
        .iter()
        .map(|f| (f.name.as_str(), f.decorators.len()))
        .collect();
    assert_eq!(functions, [("parse", 0), ("stub", 1)]);
    let cls = &module.classes[0];
    let methods: Vec<&str> = cls.methods.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(methods, ["__init__", "__eq__"]);
    assert!(cls.class_vars.is_empty());
    assert!(cls.inner_classes.is_empty());
}

#[test]
fn a_name_defined_twice_is_its_last_definition() {
    let module = google(
        "\ndef h():\n    \"\"\"First.\"\"\"\n\ndef h(x):\n    \"\"\"Second.\"\"\"\n\nclass C:\n    def m(self):\n        \"\"\"First.\"\"\"\n\n    def m(self, y):\n        \"\"\"Second.\"\"\"\n",
    );
    let functions: Vec<(&str, usize)> = module
        .functions
        .iter()
        .map(|f| (f.name.as_str(), f.args.len()))
        .collect();
    assert_eq!(functions, [("h", 1)]);
    let methods: Vec<(&str, usize)> = module.classes[0]
        .methods
        .iter()
        .map(|m| (m.name.as_str(), m.args.len()))
        .collect();
    assert_eq!(methods, [("m", 1)]);
}

#[test]
fn argument_kinds_and_default_reprs() {
    let module = google(
            "\ndef func(a: int, *args: str, **kwargs: float) -> None:\n    \"\"\"A function with *args and **kwargs.\"\"\"\n    pass\n",
        );
    let f = &module.functions[0];
    let shape: Vec<(&str, ArgKind, &str)> = f
        .args
        .iter()
        .map(|a| (a.name.as_str(), a.kind, a.ty.as_str()))
        .collect();
    assert_eq!(
        shape,
        [
            ("a", ArgKind::Regular, "int"),
            ("args", ArgKind::VarPositional, "str"),
            ("kwargs", ArgKind::VarKeyword, "float"),
        ]
    );

    let module = google(
        "\ndef func(a: int, *, key: str = \"default\", flag: bool = True) -> None:\n    pass\n",
    );
    let f = &module.functions[0];
    assert_eq!(f.args[1].kind, ArgKind::KeywordOnly);
    assert_eq!(f.args[1].default.as_deref(), Some("'default'"));
    assert_eq!(f.args[2].kind, ArgKind::KeywordOnly);
    assert_eq!(f.args[2].default.as_deref(), Some("True"));

    let module = google("\ndef func(x: int, y: int, /, z: int = 0) -> None:\n    pass\n");
    let f = &module.functions[0];
    assert_eq!(f.args[0].kind, ArgKind::PositionalOnly);
    assert_eq!(f.args[1].kind, ArgKind::PositionalOnly);
    assert_eq!(f.args[2].kind, ArgKind::Regular);
    assert_eq!(f.args[2].default.as_deref(), Some("0"));

    let module = google(
            "\ndef func(a: int, b: str, /, c: float = 1.0, *args, key: bool = False, **kwargs) -> None:\n    pass\n",
        );
    let f = &module.functions[0];
    let kinds: Vec<(&str, ArgKind, Option<&str>)> = f
        .args
        .iter()
        .map(|a| (a.name.as_str(), a.kind, a.default.as_deref()))
        .collect();
    assert_eq!(
        kinds,
        [
            ("a", ArgKind::PositionalOnly, None),
            ("b", ArgKind::PositionalOnly, None),
            ("c", ArgKind::Regular, Some("1.0")),
            ("args", ArgKind::VarPositional, None),
            ("key", ArgKind::KeywordOnly, Some("False")),
            ("kwargs", ArgKind::VarKeyword, None),
        ]
    );
}

#[test]
fn method_kinds_and_self_cls_filtering() {
    let module = google(
            "\nclass MyClass:\n    \"\"\"A class.\"\"\"\n\n    @property\n    def value(self) -> int:\n        \"\"\"The value.\"\"\"\n        return self._value\n\n    @value.setter\n    def value(self, val: int) -> None:\n        self._value = val\n\n    @staticmethod\n    def create(name: str) -> \"MyClass\":\n        \"\"\"Create an instance.\"\"\"\n        return MyClass()\n\n    @classmethod\n    def from_string(cls, data: str) -> \"MyClass\":\n        \"\"\"Create from string.\"\"\"\n        return cls()\n\n    def method(self, x: int) -> None:\n        pass\n",
        );
    let cls = &module.classes[0];
    let getter = cls
        .methods
        .iter()
        .find(|m| m.name == "value" && m.kind == FunctionKind::Property)
        .unwrap();
    assert_eq!(getter.returns.as_ref().unwrap().ty, "int");
    assert_eq!(
        cls.methods.iter().filter(|m| m.name == "value").count(),
        1,
        "the setter belongs to the property"
    );
    let create = cls.methods.iter().find(|m| m.name == "create").unwrap();
    assert_eq!(create.kind, FunctionKind::Staticmethod);
    assert_eq!(create.args.len(), 1);
    assert_eq!(create.returns.as_ref().unwrap().ty, "'MyClass'");
    let from_string = cls
        .methods
        .iter()
        .find(|m| m.name == "from_string")
        .unwrap();
    assert_eq!(from_string.kind, FunctionKind::Classmethod);
    assert_eq!(from_string.args.len(), 1);
    assert_eq!(from_string.args[0].name, "data");
    let method = cls.methods.iter().find(|m| m.name == "method").unwrap();
    assert_eq!(method.kind, FunctionKind::Method);
    assert_eq!(method.args.len(), 1);
    assert_eq!(method.args[0].name, "x");
}

#[test]
fn dunder_all_filters_everything_top_level() {
    let module = google(
            "\n\"\"\"Module with __all__.\"\"\"\n\n__all__ = [\"public_func\", \"PublicClass\", \"MAX_SIZE\"]\n\ndef public_func() -> None:\n    \"\"\"Public function.\"\"\"\n    pass\n\ndef _private_func() -> None:\n    pass\n\ndef excluded_func() -> None:\n    pass\n\nclass PublicClass:\n    \"\"\"Public class.\"\"\"\n    pass\n\nclass ExcludedClass:\n    pass\n\nMAX_SIZE: int = 100\nEXCLUDED_CONST: int = 50\n",
        );
    let functions: Vec<&str> = module.functions.iter().map(|f| f.name.as_str()).collect();
    let classes: Vec<&str> = module.classes.iter().map(|c| c.name.as_str()).collect();
    let constants: Vec<&str> = module.constants.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(functions, ["public_func"]);
    assert_eq!(classes, ["PublicClass"]);
    assert_eq!(constants, ["MAX_SIZE"]);

    let tuple = google("\n__all__ = (\"func_a\",)\n\ndef func_a() -> None:\n    pass\n\ndef func_b() -> None:\n    pass\n");
    assert_eq!(tuple.functions.len(), 1);
    assert_eq!(tuple.functions[0].name, "func_a");

    // Without `__all__`, a leading underscore keeps a name out.
    let everything =
        google("\ndef func_a() -> None:\n    pass\n\ndef _func_b() -> None:\n    pass\n");
    assert_eq!(everything.functions.len(), 1);
    assert_eq!(everything.functions[0].name, "func_a");
    // Listed in `__all__`, it is public however it is spelled.
    let listed = google("\n__all__ = [\"_func_b\"]\n\ndef _func_b() -> None:\n    pass\n");
    assert_eq!(listed.functions[0].name, "_func_b");

    // ALL_FILTERED: constants are filtered too, the private class stays out.
    let filtered = google(
            "\"\"\"Filtered module.\"\"\"\n\n__all__ = [\"public_func\", \"PublicClass\"]\n\ndef public_func() -> None:\n    pass\n\ndef _private_func() -> None:\n    pass\n\nclass PublicClass:\n    pass\n\nclass _PrivateClass:\n    pass\n\nPUBLIC_CONST = 42\n_PRIVATE_CONST = 99\n",
        );
    assert_eq!(filtered.functions.len(), 1);
    assert_eq!(filtered.classes.len(), 1);
    assert_eq!(filtered.classes[0].name, "PublicClass");
    assert!(filtered.constants.is_empty());
}

#[test]
fn nested_classes_carry_their_methods() {
    let module = google(
            "\nclass Outer:\n    \"\"\"Outer class.\"\"\"\n\n    class Inner:\n        \"\"\"Inner class.\"\"\"\n\n        def inner_method(self) -> None:\n            \"\"\"An inner method.\"\"\"\n            pass\n\n    class AnotherInner:\n        \"\"\"Another inner class.\"\"\"\n        pass\n\n    def outer_method(self) -> None:\n        pass\n",
        );
    let cls = &module.classes[0];
    assert_eq!(cls.name, "Outer");
    assert_eq!(cls.inner_classes.len(), 2);
    assert_eq!(cls.inner_classes[0].name, "Inner");
    assert_eq!(cls.inner_classes[0].methods.len(), 1);
    assert_eq!(cls.inner_classes[0].methods[0].name, "inner_method");
    assert_eq!(cls.inner_classes[1].name, "AnotherInner");
    assert_eq!(cls.methods.len(), 1);
}

#[test]
fn unannotated_uppercase_constants() {
    let module = google(
            "\nMAX_RETRIES = 3\nDEFAULT_TIMEOUT = 30.0\nAPI_URL = \"https://api.example.com\"\nlowercase_var = \"not a constant\"\n_PRIVATE = \"private\"\nANNOTATED: int = 42\n",
        );
    let shape: Vec<(&str, &str, &str)> = module
        .constants
        .iter()
        .map(|c| (c.name.as_str(), c.ty.as_str(), c.value.as_str()))
        .collect();
    assert_eq!(
        shape,
        [
            ("MAX_RETRIES", "", "3"),
            ("DEFAULT_TIMEOUT", "", "30.0"),
            ("API_URL", "", "'https://api.example.com'"),
            ("ANNOTATED", "int", "42"),
        ]
    );
}

#[test]
fn simple_module_extracts_every_shape() {
    let module = google(
            "\"\"\"Module docstring.\"\"\"\n\nMAX_RETRIES: int = 3\nTIMEOUT = 30\n\ndef greet(name: str, greeting: str = \"Hello\") -> str:\n    \"\"\"Say hello.\"\"\"\n    return f\"{greeting}, {name}!\"\n\nasync def fetch(url: str) -> bytes:\n    \"\"\"Fetch a URL.\"\"\"\n    ...\n\nclass Animal:\n    \"\"\"An animal.\"\"\"\n    sound: str = \"...\"\n\n    def __init__(self, name: str) -> None:\n        self.name = name\n\n    def speak(self) -> str:\n        \"\"\"Make a sound.\"\"\"\n        return self.sound\n\n    class Inner:\n        \"\"\"Inner class.\"\"\"\n        pass\n",
        );
    assert_eq!(module.docstring.short_description, "Module docstring.");
    assert_eq!(module.constants.len(), 2);
    assert_eq!(module.constants[1].value, "30");
    assert_eq!(module.functions.len(), 2);
    assert_eq!(
        module.functions[0].args[1].default.as_deref(),
        Some("'Hello'")
    );
    assert!(module.functions[1].is_async);
    let animal = &module.classes[0];
    assert_eq!(animal.class_vars[0].value, "'...'");
    assert_eq!(animal.methods.len(), 2);
    assert_eq!(
        animal.inner_classes[0].docstring.short_description,
        "Inner class."
    );
    assert_eq!(animal.line_number, 14);

    let empty = google("");
    assert_eq!(empty.name, "mod");
    assert!(empty.functions.is_empty() && empty.classes.is_empty());
    assert_eq!(empty.docstring.short_description, "");
}

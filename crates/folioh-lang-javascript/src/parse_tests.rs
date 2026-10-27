use super::*;

fn module(source: &str) -> ModuleIR {
    parse_source(source, "demo", "demo.js").expect("the source parses")
}

#[test]
fn yields_tags_describe_items_while_returns_still_describe_returns() {
    for (tag, ty, description, is_generator) in [
        ("yields {number} Each value.", "number", "Each value.", true),
        ("yield {number} Each value.", "number", "Each value.", true),
        (
            "returns {number} Final value.",
            "number",
            "Final value.",
            false,
        ),
        ("yields Each value.", "", "Each value.", true),
        ("yields", "", "", true),
    ] {
        let m = module(&format!(
            "/** @{tag} */\nexport function* values() {{ yield 1; }}"
        ));
        let f = &m.functions[0];
        let value = f.returns.as_ref().unwrap();
        assert_eq!(value.is_generator, is_generator, "{tag}");
        assert_eq!(value.ty, ty);
        assert_eq!(value.description, description);
        assert_eq!(value.return_annotation, None);
        assert_eq!(f.signature, "export function* values()");
    }
}

#[test]
fn an_exported_function_carries_its_signature_and_its_jsdoc() {
    let module = module(
        r#"
/**
 * Greet someone.
 *
 * @param {string} name - Who to greet.
 * @param {boolean} [excited=false] - Whether to shout.
 * @returns {string} The greeting.
 * @throws {TypeError} When the name is not a string.
 */
export async function greet(name, excited = false) {
  return name;
}
"#,
    );
    let func = &module.functions[0];
    assert_eq!(func.name, "greet");
    assert_eq!(
        func.signature,
        "export async function greet(name, excited = false)"
    );
    assert!(func.is_async);
    assert_eq!(func.visibility, "export");
    assert_eq!(func.docstring.short_description, "Greet someone.");
    assert_eq!(
        func.args
            .iter()
            .map(|a| (a.name.as_str(), a.ty.as_str(), a.default.as_deref()))
            .collect::<Vec<_>>(),
        [
            ("name", "string", None),
            ("excited", "boolean", Some("false"))
        ]
    );
    assert_eq!(func.returns.as_ref().expect("a return").ty, "string");
    assert_eq!(func.raises[0].exception, "TypeError");
}

#[test]
fn a_class_publishes_its_members_as_methods_whatever_flavour_they_are() {
    let module = module(
        "/** A rounder. */\nexport class Rounder extends Base {\n  static MAX = 10;\n  #secret = 1;\n  constructor(precision = 2) { this.precision = precision; }\n  get value() { return 1; }\n  static create() { return new Rounder(); }\n  #hide() {}\n}\n",
    );
    let class = &module.classes[0];
    assert_eq!(class.bases, ["Base"]);
    assert_eq!(class.docstring.short_description, "A rounder.");
    assert_eq!(
        class
            .methods
            .iter()
            .map(|m| (m.signature.as_str(), m.kind, m.visibility.as_str()))
            .collect::<Vec<_>>(),
        [
            ("constructor(precision = 2)", FunctionKind::Method, ""),
            ("get value()", FunctionKind::Method, ""),
            ("static create()", FunctionKind::Method, "static"),
        ]
    );
    assert_eq!(class.class_vars[0].name, "static MAX");
    assert_eq!(class.class_vars.len(), 1);
}

#[test]
fn a_setter_beside_its_getter_is_one_member_and_a_lone_setter_stays() {
    let module = module(
        "export class Box {\n  set size(v) {}\n  get size() { return 1; }\n  static get size() { return 2; }\n  set label(v) {}\n  get() {}\n}\n",
    );
    assert_eq!(
        module.classes[0]
            .methods
            .iter()
            .map(|m| m.signature.as_str())
            .collect::<Vec<_>>(),
        ["get size()", "static get size()", "set label(v)", "get()"]
    );
}

#[test]
fn a_paired_setter_lends_its_jsdoc_to_an_undocumented_getter() {
    let module = module(
        "export class Box {\n  /** Width getter. */\n  get width() { return 1; }\n  /** Width setter. */\n  set width(v) {}\n  get height() { return 1; }\n  /**\n   * Height, in pixels.\n   *\n   * Rounded down.\n   */\n  set height(v) {}\n  static get depth() { return 1; }\n  /** Instance depth. */\n  set depth(v) {}\n}\n",
    );
    let docs: Vec<(&str, &str, &str)> = module.classes[0]
        .methods
        .iter()
        .map(|m| {
            (
                m.signature.as_str(),
                m.docstring.short_description.as_str(),
                m.docstring.long_description.as_str(),
            )
        })
        .collect();
    assert_eq!(
        docs,
        [
            // A documented getter keeps its own JSDoc.
            ("get width()", "Width getter.", ""),
            ("get height()", "Height, in pixels.", "Rounded down."),
            // A static getter and an instance setter are not a pair.
            ("static get depth()", "", ""),
            ("set depth(v)", "Instance depth.", ""),
        ]
    );
}

#[test]
fn constants_arrows_and_re_exports_each_land_where_they_belong() {
    let module = module(
        "/** The module. */\n\n/** How many. */\nexport const LIMIT = 10;\nexport const twice = (n) => n * 2;\nexport { helper } from './helpers';\nexport * from './utils';\n",
    );
    assert_eq!(module.docstring.short_description, "The module.");
    assert_eq!(
        module.docstring.long_description,
        "Re-exports: helper (from ./helpers), * (from ./utils)"
    );
    assert_eq!(module.constants[0].name, "LIMIT");
    assert_eq!(module.constants[0].value, "10");
    assert_eq!(module.constants[0].description, "How many.");
    assert_eq!(module.functions[0].name, "twice");
    assert_eq!(module.functions[0].signature, "export const twice = (n) =>");
}

#[test]
fn common_js_exports_are_read_too() {
    let module = module(
        "/** A helper. */\nexports.helper = function (a) { return a; };\nexports.LIMIT = 4;\nmodule.exports = { other };\n",
    );
    assert_eq!(module.functions[0].name, "helper");
    assert_eq!(
        module.functions[0].signature,
        "exports.helper = function (a)"
    );
    assert_eq!(module.constants[0].name, "LIMIT");
    assert_eq!(module.docstring.long_description, "Re-exports: other");
}

#[test]
fn module_exports_of_a_function_documents_the_function_not_its_body() {
    let module = module(
        "/** Adds. */\nmodule.exports = function single(a, b) {\n  const secret = 1;\n  return a + b;\n};\nmodule.exports.bye = function () {};\n",
    );
    let functions: Vec<(&str, &str)> = module
        .functions
        .iter()
        .map(|f| (f.name.as_str(), f.signature.as_str()))
        .collect();
    assert_eq!(
        functions,
        [
            ("single", "module.exports = function single(a, b)"),
            ("bye", "module.exports.bye = function ()"),
        ]
    );
    assert_eq!(module.functions[0].docstring.short_description, "Adds.");
    assert_eq!(module.docstring.long_description, "");
}

#[test]
fn a_local_declaration_exported_by_name_is_documented() {
    let module = module(
        "/** Greets. */\nfunction greet() {}\n/** The size. */\nconst SIZE = 3;\nclass Box {}\nexport { greet, SIZE as LIMIT };\nexport default Box;\n",
    );
    assert_eq!(module.functions[0].name, "greet");
    assert_eq!(module.functions[0].docstring.short_description, "Greets.");
    assert_eq!(module.constants[0].name, "LIMIT");
    assert_eq!(module.constants[0].description, "The size.");
    assert_eq!(module.classes[0].name, "Box");
    assert_eq!(module.docstring.long_description, "");

    let common =
        self::module("function greet() {}\nmodule.exports = { greet, other: require('./x') };\n");
    assert_eq!(common.functions[0].name, "greet");
    assert_eq!(common.docstring.long_description, "Re-exports: other");
}

#[test]
fn an_anonymous_default_export_is_named_default() {
    let module = module("export default function () { return 1; }\n");
    assert_eq!(module.functions[0].name, "default");
    assert_eq!(module.functions[0].signature, "export default function ()");
    let value = self::module("export default 42;\n");
    assert_eq!(value.constants[0].name, "default");
    assert_eq!(value.constants[0].value, "42");
}

#[test]
fn a_documented_property_of_a_parameter_gets_its_own_row() {
    let module = module(
        "/**\n * Make.\n * @param {Object} opts - Options.\n * @param {string} opts.name - The name.\n * @param {number} [opts.size=1] - The size.\n * @param {Object[]} rows - Rows.\n * @param {string} rows[].id - An id.\n */\nexport function make(opts, rows) {}\n",
    );
    let rows: Vec<(&str, &str, Option<&str>)> = module.functions[0]
        .args
        .iter()
        .map(|a| (a.name.as_str(), a.ty.as_str(), a.default.as_deref()))
        .collect();
    assert_eq!(
        rows,
        [
            ("opts", "Object", None),
            ("opts.name", "string", None),
            ("opts.size", "number", Some("1")),
            ("rows", "Object[]", None),
            ("rows[].id", "string", None),
        ]
    );
}

#[test]
fn a_broken_file_names_its_line_and_column() {
    let err = parse_source("export function () {\n", "demo", "demo.js").expect_err("it is broken");
    assert_eq!(err.line, 1);
    assert!(err.message.starts_with("invalid syntax"), "{}", err.message);
}

//! tree-sitter nodes to `ModuleIR`: what a file exports, nothing else. A
//! signature is the source from the `export` keyword to the closing
//! parenthesis, collapsed to one line; the prose, parameter types and
//! documented defaults come from the JSDoc comment above the declaration.

use folioh_ir::{
    ArgIR, ArgKind, ClassIR, DocstringIR, FunctionIR, FunctionKind, Language, ModuleIR, RaiseIR,
    ReturnIR, VarIR,
};
use std::collections::HashMap;

use tree_sitter::{Node, Parser};

use crate::error::JavaScriptSyntaxError;
use crate::jsdoc::{self, JsDoc};

/// One file's exported surface. `name` is the module the path publishes,
/// `source_file` what the IR reports as its file.
pub fn parse_source(
    source: &str,
    name: &str,
    source_file: &str,
) -> Result<ModuleIR, JavaScriptSyntaxError> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_javascript::LANGUAGE.into())
        .expect("the grammar is built against this tree-sitter");
    let tree = parser.parse(source, None).expect("no timeout, no cancel");
    let root = tree.root_node();
    if let Some(bad) = first_error(root) {
        let at = bad.start_position();
        return Err(JavaScriptSyntaxError {
            path: source_file.into(),
            message: message(source, bad),
            line: at.row as u32 + 1,
            column: at.column as u32 + 1,
        });
    }

    let mut module = ModuleIR {
        name: name.to_string(),
        docstring: DocstringIR::default(),
        classes: Vec::new(),
        functions: Vec::new(),
        constants: Vec::new(),
        source_file: source_file.to_string(),
        language: Language::Javascript,
        types: Vec::new(),
    };
    let mut reexports = Vec::new();
    let mut cursor = root.walk();
    let children: Vec<Node> = root.named_children(&mut cursor).collect();
    if let Some(doc) = module_doc(source, &children) {
        module.docstring = doc.docstring();
    }
    let file = File {
        src: source,
        path: source_file,
        locals: locals(source, &children),
    };
    for child in &children {
        match child.kind() {
            "export_statement" => export(&file, *child, &mut module, &mut reexports),
            "expression_statement" => common_js(&file, *child, &mut module, &mut reexports),
            _ => {}
        }
    }
    if !reexports.is_empty() {
        let line = format!("Re-exports: {}", reexports.join(", "));
        let long = &mut module.docstring.long_description;
        if long.is_empty() {
            *long = line;
        } else {
            long.push_str("\n\n");
            long.push_str(&line);
        }
    }
    Ok(module)
}

/// The file's own comment: a leading `/** */` with a blank line under it. A
/// comment written against the first declaration documents that declaration.
fn module_doc(src: &str, children: &[Node]) -> Option<JsDoc> {
    let first = children.first()?;
    let comment = block_comment(src, *first)?;
    let detached = children
        .get(1)
        .is_none_or(|next| next.start_position().row > first.end_position().row + 1);
    detached.then(|| jsdoc::parse(comment))
}

/// The file being read, with the top-level declarations it makes without
/// exporting them: `export { x }`, `export default x` and
/// `module.exports = { x }` publish those by name further down.
struct File<'a, 't> {
    src: &'a str,
    path: &'a str,
    locals: HashMap<String, (Node<'t>, Node<'t>)>,
}

/// Every unexported top-level declaration by name: the statement (where its
/// JSDoc sits) and the function, class or declarator it holds.
fn locals<'t>(src: &str, children: &[Node<'t>]) -> HashMap<String, (Node<'t>, Node<'t>)> {
    let mut locals = HashMap::new();
    for statement in children {
        let declared: Vec<Node<'t>> = match statement.kind() {
            "function_declaration" | "generator_function_declaration" | "class_declaration" => {
                vec![*statement]
            }
            "lexical_declaration" | "variable_declaration" => {
                let mut cursor = statement.walk();
                let declarators = statement
                    .named_children(&mut cursor)
                    .filter(|child| child.kind() == "variable_declarator")
                    .collect();
                declarators
            }
            _ => continue,
        };
        for node in declared {
            if let Some(name) = node.child_by_field_name("name") {
                locals.insert(text(src, name).to_string(), (*statement, node));
            }
        }
    }
    locals
}

/// Documents the local declaration `local` under the name `published`.
/// Whether there was one to document.
fn publish(file: &File, local: &str, published: &str, module: &mut ModuleIR) -> bool {
    let Some((statement, node)) = file.locals.get(local) else {
        return false;
    };
    let doc = doc_above(file.src, *statement);
    let before = (
        module.functions.len(),
        module.classes.len(),
        module.constants.len(),
    );
    if node.kind() == "variable_declarator" {
        bind(file.src, node.start_byte(), *node, &doc, file.path, module);
    } else {
        declare(file.src, node.start_byte(), *node, &doc, file.path, module);
    }
    let name = published.to_string();
    if module.functions.len() > before.0 {
        module.functions.last_mut().expect("just pushed").name = name;
    } else if module.classes.len() > before.1 {
        module.classes.last_mut().expect("just pushed").name = name;
    } else if module.constants.len() > before.2 {
        module.constants.last_mut().expect("just pushed").name = name;
    }
    true
}

/// A value published under `name`: a function or class expression, or a
/// local declaration named by an identifier. Whether it was one of those.
fn publish_value(
    file: &File,
    start: usize,
    value: Node,
    name: &str,
    doc: &JsDoc,
    module: &mut ModuleIR,
) -> bool {
    match value.kind() {
        "function_expression" | "arrow_function" | "generator_function" => {
            let mut func = function(file.src, start, value, doc, file.path);
            if func.name.is_empty() || name != "default" {
                func.name = name.to_string();
            }
            if value.kind() == "arrow_function" {
                func.signature.push_str(" =>");
            }
            module.functions.push(func);
            true
        }
        "class" => {
            let mut class = class(file.src, value, doc, file.path);
            if class.name.is_empty() || name != "default" {
                class.name = name.to_string();
            }
            module.classes.push(class);
            true
        }
        "identifier" => {
            let local = text(file.src, value);
            publish(
                file,
                local,
                if name == "default" { local } else { name },
                module,
            )
        }
        _ => false,
    }
}

/// One `export …` statement into the module.
fn export(file: &File, node: Node, module: &mut ModuleIR, reexports: &mut Vec<String>) {
    let src = file.src;
    let doc = doc_above(src, node);
    if let Some(declaration) = node.child_by_field_name("declaration") {
        declare(src, node.start_byte(), declaration, &doc, file.path, module);
    } else if let Some(value) = node.child_by_field_name("value") {
        // `export default <expression>`.
        if !publish_value(file, node.start_byte(), value, "default", &doc, module) {
            module.constants.push(VarIR {
                name: String::from("default"),
                ty: String::new(),
                value: collapse(text(src, value)),
                description: collapse(&doc.description),
            });
        }
    } else if node.child_by_field_name("source").is_none() {
        // `export { a, b as c }` of the file's own declarations.
        for (local, published) in specifiers(src, node) {
            if !publish(file, &local, &published, module) {
                reexports.push(published);
            }
        }
    } else {
        reexports.extend(clause(src, node));
    }
}

/// The `(local, published)` names of an `export { … }` clause.
fn specifiers(src: &str, node: Node) -> Vec<(String, String)> {
    let mut names = Vec::new();
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() != "export_clause" {
            continue;
        }
        let mut inner = child.walk();
        for spec in child.named_children(&mut inner) {
            let name = spec
                .child_by_field_name("name")
                .map_or(String::new(), |n| text(src, n).to_string());
            let alias = spec
                .child_by_field_name("alias")
                .map_or(name.clone(), |n| text(src, n).to_string());
            names.push((name, alias));
        }
    }
    names
}

/// A declaration that `export` (or `export default`) fronts. `start` is where
/// its signature begins, so the `export` keyword is part of it.
fn declare(src: &str, start: usize, node: Node, doc: &JsDoc, file: &str, module: &mut ModuleIR) {
    match node.kind() {
        "function_declaration" | "generator_function_declaration" => {
            module.functions.push(function(src, start, node, doc, file));
        }
        "class_declaration" => module.classes.push(class(src, node, doc, file)),
        "lexical_declaration" | "variable_declaration" => {
            let mut cursor = node.walk();
            for (index, declarator) in node
                .named_children(&mut cursor)
                .filter(|child| child.kind() == "variable_declarator")
                .enumerate()
            {
                // Only the first name of `export const a = 1, b = 2` can carry
                // the `export const` prefix in its signature.
                let start = if index == 0 {
                    start
                } else {
                    declarator.start_byte()
                };
                bind(src, start, declarator, doc, file, module);
            }
        }
        _ => {}
    }
}

/// An exported binding: a function when it holds one, a constant otherwise.
fn bind(src: &str, start: usize, node: Node, doc: &JsDoc, file: &str, module: &mut ModuleIR) {
    let name = node
        .child_by_field_name("name")
        .map_or(String::new(), |n| text(src, n).to_string());
    let value = node.child_by_field_name("value");
    match value.map(|v| v.kind()) {
        Some("arrow_function" | "function_expression") => {
            let value = value.expect("matched on its kind");
            let mut func = function(src, start, value, doc, file);
            func.name = name;
            if value.kind() == "arrow_function" {
                func.signature.push_str(" =>");
            }
            module.functions.push(func);
        }
        _ => module.constants.push(VarIR {
            name,
            ty: String::new(),
            value: value.map_or(String::new(), |v| collapse(text(src, v))),
            description: collapse(&doc.description),
        }),
    }
}

/// A function, method or arrow: the JSDoc merged onto what the source says.
fn function(src: &str, start: usize, node: Node, doc: &JsDoc, file: &str) -> FunctionIR {
    let params = node
        .child_by_field_name("parameters")
        .or_else(|| node.child_by_field_name("parameter"));
    let end = params.map_or(node.start_byte(), |p| p.end_byte());
    FunctionIR {
        name: node
            .child_by_field_name("name")
            .map_or(String::new(), |n| text(src, n).to_string()),
        args: args(src, params, doc),
        returns: (doc.is_generator
            || !doc.returns_type.is_empty()
            || !doc.returns_description.is_empty())
        .then(|| ReturnIR {
            ty: doc.returns_type.clone(),
            description: doc.returns_description.clone(),
            is_generator: doc.is_generator,
            return_annotation: None,
        }),
        raises: doc
            .throws
            .iter()
            .map(|(exception, description)| RaiseIR {
                exception: exception.clone(),
                description: description.clone(),
            })
            .collect(),
        decorators: Vec::new(),
        docstring: doc.docstring(),
        is_async: has(node, "async"),
        source_file: file.to_string(),
        line_number: node.start_position().row as u32 + 1,
        kind: FunctionKind::Function,
        signature: collapse(&src[start..end]),
        visibility: String::from("export"),
    }
}

/// What a class `extends`, as written.
fn heritage(node: Node) -> Option<Node> {
    let mut cursor = node.walk();
    let extends = node
        .named_children(&mut cursor)
        .find(|child| child.kind() == "class_heritage")
        .and_then(|clause| clause.named_child(0));
    extends
}

/// An exported class with its members; `#private` ones are not its surface.
fn class(src: &str, node: Node, doc: &JsDoc, file: &str) -> ClassIR {
    let mut class = ClassIR {
        name: node
            .child_by_field_name("name")
            .map_or(String::new(), |n| text(src, n).to_string()),
        bases: heritage(node).map_or(Vec::new(), |base| vec![text(src, base).to_string()]),
        decorators: Vec::new(),
        docstring: doc.docstring(),
        methods: Vec::new(),
        class_vars: Vec::new(),
        inner_classes: Vec::new(),
        source_file: file.to_string(),
        line_number: node.start_position().row as u32 + 1,
    };
    let Some(body) = node.child_by_field_name("body") else {
        return class;
    };
    let mut getters = Vec::new();
    let mut setters = Vec::new();
    let mut cursor = body.walk();
    for member in body.named_children(&mut cursor) {
        let name = member
            .child_by_field_name("name")
            .or_else(|| member.child_by_field_name("property"))
            .map_or(String::new(), |n| text(src, n).to_string());
        if name.starts_with('#') {
            continue;
        }
        let doc = doc_above(src, member);
        let is_static = has(member, "static");
        match member.kind() {
            "method_definition" => {
                if has(member, "get") {
                    getters.push((name.clone(), is_static, class.methods.len()));
                } else if has(member, "set") {
                    setters.push(class.methods.len());
                }
                // Every JavaScript method is a `Method`: the property and
                // staticmethod kinds are Python decorators the renderer prints.
                let mut method = function(src, member.start_byte(), member, &doc, file);
                method.name = name;
                method.kind = FunctionKind::Method;
                method.visibility = if is_static { "static" } else { "" }.to_string();
                class.methods.push(method);
            }
            "field_definition" => class.class_vars.push(VarIR {
                name: if is_static {
                    format!("static {name}")
                } else {
                    name
                },
                ty: String::new(),
                value: member
                    .child_by_field_name("value")
                    .map_or(String::new(), |v| collapse(text(src, v))),
                description: collapse(&doc.description),
            }),
            _ => {}
        }
    }
    // A setter beside its getter is one property on the page, as a Python
    // property's setter is, so the two never share a heading id. The getter
    // keeps its own JSDoc; when it has none, the setter's is the property's.
    let paired: Vec<(usize, usize)> = setters
        .into_iter()
        .filter_map(|setter| {
            let method = &class.methods[setter];
            let is_static = method.visibility == "static";
            getters
                .iter()
                .find(|(name, getter_static, _)| {
                    *name == method.name && *getter_static == is_static
                })
                .map(|(_, _, getter)| (setter, *getter))
        })
        .collect();
    for &(setter, getter) in &paired {
        if class.methods[getter].docstring == DocstringIR::default() {
            class.methods[getter].docstring = class.methods[setter].docstring.clone();
        }
    }
    let mut index = 0;
    class.methods.retain(|_| {
        let keep = !paired.iter().any(|(setter, _)| *setter == index);
        index += 1;
        keep
    });
    class
}

/// `exports.name = …` is a constant or function; `module.exports = …` is
/// the function or class it assigns, the declarations an object names, or
/// else what the file re-exports.
fn common_js(file: &File, node: Node, module: &mut ModuleIR, reexports: &mut Vec<String>) {
    let src = file.src;
    let Some(assignment) = node
        .named_child(0)
        .filter(|n| n.kind() == "assignment_expression")
    else {
        return;
    };
    let (Some(left), Some(right)) = (
        assignment.child_by_field_name("left"),
        assignment.child_by_field_name("right"),
    ) else {
        return;
    };
    if left.kind() != "member_expression" {
        return;
    }
    let object = left
        .child_by_field_name("object")
        .map_or(String::new(), |n| text(src, n).to_string());
    let property = left
        .child_by_field_name("property")
        .map_or(String::new(), |n| text(src, n).to_string());
    let doc = doc_above(src, node);
    let start = node.start_byte();
    match (object.as_str(), property.as_str()) {
        ("module", "exports") => match right.kind() {
            "object" => {
                let mut cursor = right.walk();
                for entry in right.named_children(&mut cursor) {
                    let key = entry
                        .child_by_field_name("key")
                        .map(|key| text(src, key).to_string())
                        .or_else(|| {
                            (entry.kind() == "shorthand_property_identifier")
                                .then(|| text(src, entry).to_string())
                        });
                    let Some(key) = key else { continue };
                    let published = match entry.child_by_field_name("value") {
                        Some(value) => {
                            let doc = doc_above(src, entry);
                            publish_value(file, entry.start_byte(), value, &key, &doc, module)
                        }
                        None => publish(file, &key, &key, module),
                    };
                    if !published {
                        reexports.push(key);
                    }
                }
            }
            _ if publish_value(file, start, right, "default", &doc, module) => {}
            // A single line names what it re-exports; a longer expression
            // is code, not a name.
            _ if !text(src, right).contains('\n') => reexports.push(collapse(text(src, right))),
            _ => {}
        },
        ("exports" | "module.exports", name)
            if !name.is_empty() && !publish_value(file, start, right, name, &doc, module) =>
        {
            module.constants.push(VarIR {
                name: name.to_string(),
                ty: String::new(),
                value: collapse(text(src, right)),
                description: collapse(&doc.description),
            });
        }
        _ => {}
    }
}

/// The names an `export { … } from '…'` or `export * from '…'` publishes.
fn clause(src: &str, node: Node) -> Vec<String> {
    let source = node
        .child_by_field_name("source")
        .map(|s| text(src, s).trim_matches(['\'', '"']).to_string());
    let mut names = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "*" => names.push(String::from("*")),
            "export_clause" => {
                let mut inner = child.walk();
                names.extend(child.named_children(&mut inner).map(|spec| {
                    let alias = spec.child_by_field_name("alias");
                    let name = alias.or_else(|| spec.child_by_field_name("name"));
                    name.map_or(String::new(), |n| text(src, n).to_string())
                }));
            }
            _ => {}
        }
    }
    match source {
        Some(from) => names
            .into_iter()
            .map(|name| format!("{name} (from {from})"))
            .collect(),
        None => names,
    }
}

/// Every parameter, with the JSDoc entry of the same name merged in.
fn args(src: &str, params: Option<Node>, doc: &JsDoc) -> Vec<ArgIR> {
    let Some(params) = params else {
        return Vec::new();
    };
    if params.kind() != "formal_parameters" {
        // `x => x * 2` writes its one parameter without parentheses.
        return vec![arg(text(src, params).to_string(), None, doc)];
    }
    let mut cursor = params.walk();
    let mut args = Vec::new();
    for child in params
        .named_children(&mut cursor)
        .filter(|child| child.kind() != "comment")
    {
        let arg = match child.kind() {
            "assignment_pattern" => arg(
                child
                    .child_by_field_name("left")
                    .map_or(String::new(), |n| text(src, n).to_string()),
                child
                    .child_by_field_name("right")
                    .map(|n| collapse(text(src, n))),
                doc,
            ),
            _ => arg(collapse(text(src, child)), None, doc),
        };
        // `@param opts.name` documents a property of `opts`: its own row,
        // under the parameter it belongs to.
        let prefix = format!("{}.", arg.name.trim_start_matches('.'));
        let properties: Vec<ArgIR> = doc
            .params
            .iter()
            .filter(|p| {
                p.name.starts_with(&prefix)
                    || p.name
                        .starts_with(&format!("{}[].", &prefix[..prefix.len() - 1]))
            })
            .map(|p| ArgIR {
                name: p.name.clone(),
                ty: p.ty.clone(),
                default: Some(p.default.clone()).filter(|d| !d.is_empty()),
                description: p.description.clone(),
                kind: ArgKind::Regular,
            })
            .collect();
        args.push(arg);
        args.extend(properties);
    }
    args
}

/// One parameter: the source name and default, the JSDoc type and prose.
fn arg(name: String, default: Option<String>, doc: &JsDoc) -> ArgIR {
    let documented = doc.param(name.trim_start_matches('.'));
    ArgIR {
        ty: documented.map_or(String::new(), |p| p.ty.clone()),
        default: default.or_else(|| {
            documented
                .map(|p| p.default.clone())
                .filter(|d| !d.is_empty())
        }),
        description: documented.map_or(String::new(), |p| p.description.clone()),
        name,
        kind: ArgKind::Regular,
    }
}

/// The JSDoc comment written directly above a node: a blank line between
/// them detaches it, as it does the file's own comment.
fn doc_above(src: &str, node: Node) -> JsDoc {
    node.prev_sibling()
        .filter(|prev| prev.end_position().row + 1 >= node.start_position().row)
        .and_then(|prev| block_comment(src, prev))
        .map_or(JsDoc::default(), jsdoc::parse)
}

/// The text of a node when it is a `/** */` comment.
fn block_comment<'a>(src: &'a str, node: Node) -> Option<&'a str> {
    (node.kind() == "comment")
        .then(|| text(src, node))
        .filter(|comment| comment.starts_with("/**"))
}

/// The first node the grammar could not read.
fn first_error(node: Node) -> Option<Node> {
    if node.is_error() || node.is_missing() {
        return Some(node);
    }
    let mut cursor = node.walk();
    let found = node
        .children(&mut cursor)
        .filter(|child| child.has_error() || child.is_missing())
        .find_map(first_error);
    found
}

/// What to tell the reader about an unparsable node.
fn message(src: &str, node: Node) -> String {
    if node.is_missing() {
        return format!("missing {}", node.kind());
    }
    let near: String = text(src, node)
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .chars()
        .take(40)
        .collect();
    match near.is_empty() {
        true => String::from("invalid syntax"),
        false => format!("invalid syntax near `{near}`"),
    }
}

/// Whether a node is written with the given keyword, `static` or `async`.
fn has(node: Node, keyword: &str) -> bool {
    let mut cursor = node.walk();
    let written = node.children(&mut cursor).any(|c| c.kind() == keyword);
    written
}

fn text<'a>(src: &'a str, node: Node) -> &'a str {
    node.utf8_text(src.as_bytes()).unwrap_or_default()
}

/// Source as one line: the shape a signature takes on a page.
fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
#[path = "parse_tests.rs"]
mod tests;

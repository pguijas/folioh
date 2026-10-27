use std::path::Path;
#[test]
fn file_and_conditional_attributes_are_read_without_evaluating_cfg() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/doc-attributes");
    let path = root.join("src/lib.rs");
    let module = folioh_lang_rust::parse_file(&path, &root).unwrap();
    assert_eq!(
        module.docstring.short_description,
        "Crate documentation from a file."
    );
    assert_eq!(module.docstring.long_description, "A second paragraph.");
    assert_eq!(
        module.functions[0].docstring.short_description,
        "Conditional function documentation."
    );
    assert!(module.functions[1].docstring.short_description.is_empty());
    let item = module
        .types
        .iter()
        .find(|item| item.name == "Example")
        .unwrap();
    assert_eq!(
        item.docstring.short_description,
        "Nested conditional documentation."
    );
    assert_eq!(
        item.fields[0].description,
        "Conditional field documentation.\n"
    );
    let implementation = module
        .types
        .iter()
        .find(|item| !item.methods.is_empty())
        .unwrap();
    assert_eq!(
        implementation.methods[0].docstring.short_description,
        "Method documentation from a file."
    );
    let dependencies = folioh_lang_rust::doc_dependencies(&path);
    assert_eq!(dependencies.len(), 3);
    for file in ["README.md", "method.md", "missing.md"] {
        assert!(dependencies.iter().any(|path| path.ends_with(file)));
    }
}

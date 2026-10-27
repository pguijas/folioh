//! Python source through docstring parsing and both reference renderers.

use std::{fs, path::Path};

use folioh_docs::{generate_llms_full_txt, module_to_mdx, RenderOptions};
use folioh_lang_python::{parse_module, DocstringStyle};

#[test]
fn rest_notices_match_the_page_and_full_text_goldens() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rest_directives");
    let source = fs::read_to_string(root.join("source.py")).unwrap();
    for style in [
        DocstringStyle::Auto,
        DocstringStyle::Google,
        DocstringStyle::Numpy,
    ] {
        let module = parse_module(
            Path::new("/fixture/directives.py"),
            &source,
            Path::new("/fixture"),
            style,
        )
        .unwrap();
        let page = module_to_mdx(&module, &RenderOptions::default());
        let full = generate_llms_full_txt(&[module], &[], None);
        for (name, actual) in [("page.mdx", page), ("full.md", full)] {
            if style == DocstringStyle::Auto && std::env::var_os("FOLIOH_UPDATE_GOLDEN").is_some() {
                fs::write(root.join(name), &actual).unwrap();
            }
            assert_eq!(
                actual,
                fs::read_to_string(root.join(name)).unwrap(),
                "{style:?}: {name}"
            );
            for label in [
                "Note",
                "Warning",
                "Tip",
                "See also",
                "Deprecated",
                "Directive: custom-notice",
            ] {
                assert!(actual.contains(label), "{style:?}: {label}");
            }
            assert!(!actual.contains(".. note::") && !actual.contains(".. custom-notice::"));
            assert!(actual.contains("After the notices."));
        }
    }
}

use super::*;
use crate::builtins::BUILTIN_COMPONENTS;
use crate::registry::ComponentDefinition;
use indexmap::IndexMap;
use serde_json::json;

fn definition(name: &str) -> ComponentDefinition {
    ComponentDefinition::new(name, &format!("@/components/{}", name.to_lowercase()))
}

fn props(pairs: &[(&str, &str)]) -> IndexMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[test]
fn mdx_contract_is_versioned_and_documents_required_props() {
    assert_eq!(FOLIOH_MDX_CONTRACT_VERSION, "1.1");
    assert_eq!(FOLIOH_AUTHORING_CONTRACT_PATH, "_folioh/contract.json");
    let contract = build_contract(&BUILTIN_COMPONENTS);
    assert_eq!(contract.len(), 25);
    let by_name = |name: &str| contract.iter().find(|c| c.name == name).unwrap();
    assert_eq!(
            by_name("ParamTable").props,
            props(&[("args", "Array<{ name: string; type: string; default?: string; description?: string | null; href?: string }>")])
        );
    assert_eq!(
        by_name("Tabs").props,
        props(&[
            ("children", "React.ReactNode"),
            ("aria-label", "string | undefined")
        ])
    );
    assert_eq!(by_name("Mermaid").props, props(&[("chart", "string")]));
    assert_eq!(
        by_name("Callout").props["type"],
        "\"note\" | \"warning\" | \"info\" | \"tip\" | \"check\" | \"danger\" | undefined"
    );
    assert!(by_name("ParamTable").required);
    assert_eq!(by_name("ParamTable").source, "api-reference");
}

#[test]
fn required_component_names_come_in_manifest_order() {
    assert_eq!(
        required_component_names(&BUILTIN_COMPONENTS),
        [
            "ParamTable",
            "ClassOverview",
            "Callout",
            "SourceLink",
            "Mermaid",
            "Tabs",
            "TabItem",
            "ApiReferenceIndex"
        ]
    );
}

#[test]
fn build_contract_membership_follows_contract_flag() {
    let included = ComponentDefinition {
        props: props(&[("items", "string[]")]),
        contract: true,
        source_label: "plugin:glossary".to_string(),
        ..definition("GlossaryList")
    };
    let excluded = ComponentDefinition {
        props: props(&[("value", "string")]),
        ..definition("Internal")
    };
    let propless = ComponentDefinition {
        contract: true,
        source_label: "plugin:glossary".to_string(),
        category: "component-catalog".to_string(),
        ..definition("GlossaryBadge")
    };
    let contract = build_contract(&[included, excluded, propless]);
    let names: Vec<&str> = contract.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["GlossaryList", "GlossaryBadge"]);
    assert_eq!(contract[0].source, "plugin:glossary");
    assert_eq!(contract[0].props, props(&[("items", "string[]")]));
    // `source` comes from `source_label`, never `category`; empty props serialise as `{}`.
    assert_eq!(
        serde_json::to_value(&contract[1]).unwrap(),
        json!({"name": "GlossaryBadge", "required": false, "source": "plugin:glossary", "props": {}})
    );
}

#[test]
fn rendered_mdx_contract_module_is_importable_typescript() {
    let module = render_mdx_contract_module(&BUILTIN_COMPONENTS);
    assert!(module.starts_with("export const foliohMdxContractVersion = \"1.1\" as const\n\nexport const foliohMdxComponents = [\n  {\n    \"name\": \"ParamTable\",\n    \"required\": true,\n    \"source\": \"api-reference\",\n    \"props\": {\n      \"args\": "));
    assert!(module.ends_with("\n] as const\n\nexport type FoliohMdxComponentName = (typeof foliohMdxComponents)[number][\"name\"]\n"));
    let plugin = ComponentDefinition {
        props: props(&[("items", "string[]")]),
        contract: true,
        source_label: "plugin:glossary".to_string(),
        ..definition("GlossaryList")
    };
    let mut components: Vec<ComponentDefinition> = BUILTIN_COMPONENTS.to_vec();
    components.push(plugin);
    let module = render_mdx_contract_module(&components);
    assert!(module.contains("\"name\": \"GlossaryList\""));
    assert!(module.contains("\"source\": \"plugin:glossary\""));
}

#[test]
fn authoring_contract_envelope_sorts_and_deduplicates_keys_and_routes() {
    let plugin = ComponentDefinition {
        props: props(&[("items", "string[]")]),
        contract: true,
        source_label: "plugin:glossary".to_string(),
        ..definition("GlossaryList")
    };
    let mut components: Vec<ComponentDefinition> = BUILTIN_COMPONENTS.to_vec();
    components.push(plugin);
    let contract = build_authoring_contract(
        "9.9.9",
        "2026-07-28T09:12:04Z",
        &components,
        ["roadmap", "project", "roadmap"].map(String::from),
        ["/docs/guide/", "/docs/", "/docs/guide/"].map(String::from),
    );
    assert_eq!(contract.folioh_version, "9.9.9");
    assert_eq!(contract.mdx_contract_version, "1.1");
    assert_eq!(contract.generated_at, "2026-07-28T09:12:04Z");
    assert_eq!(contract.instructions, AUTHORING_CONTRACT_INSTRUCTIONS);
    assert!(contract
        .instructions
        .starts_with("Ignore fields you do not recognise"));
    assert_eq!(contract.config_keys, ["project", "roadmap"]);
    assert_eq!(contract.routes, ["/docs/", "/docs/guide/"]);
    assert_eq!(contract.components.len(), 26);
    assert_eq!(contract.components[25].name, "GlossaryList");
    let value = serde_json::to_value(&contract).unwrap();
    let keys: Vec<&str> = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        [
            "foliohVersion",
            "mdxContractVersion",
            "generatedAt",
            "instructions",
            "components",
            "configKeys",
            "routes"
        ]
    );
}

#[test]
fn ignored_keys_are_never_published_config_keys() {
    let contract = build_authoring_contract(
        "0.3.0-a1",
        "2026-07-28T09:12:04Z",
        &[],
        ["plugins", "i18n", "project", "versions"].map(String::from),
        [],
    );
    assert_eq!(contract.config_keys, ["project"]);
    assert_eq!(
        contract_config_keys().len(),
        folioh_config::known_config_keys().len() - UNPUBLISHED_CONFIG_KEYS.len()
    );
    for key in UNPUBLISHED_CONFIG_KEYS {
        assert!(folioh_config::known_config_keys().contains(&key), "{key}");
        assert!(!contract_config_keys().contains(&key), "{key}");
    }
}

#[test]
fn rendered_authoring_contract_is_json_with_a_trailing_newline_and_utf8() {
    let text = render_authoring_contract(
        "0.3.0-a1",
        "2026-07-28T09:12:04Z",
        &BUILTIN_COMPONENTS,
        folioh_config::known_config_keys()
            .into_iter()
            .map(String::from),
        ["/docs/ünïcode/".to_string()],
    );
    assert!(text.ends_with("}\n"));
    assert!(text.contains("/docs/ünïcode/"));
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    let mut expected: Vec<&str> = contract_config_keys();
    expected.sort_unstable();
    assert!(!expected.contains(&"plugins"));
    assert!(!expected.contains(&"i18n") && !expected.contains(&"versions"));
    assert_eq!(expected.len(), 16);
    assert!(expected.contains(&"feedback"));
    assert!(folioh_config::known_config_keys().contains(&"plugins"));
    assert_eq!(
        parsed["configKeys"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect::<Vec<_>>(),
        expected
    );
    assert_eq!(parsed["components"].as_array().unwrap().len(), 25);
}

fn validate(body: &str) -> Vec<String> {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("mdx-components.tsx"), body).unwrap();
    validate_template_mdx_contract(dir.path(), &required_component_names(&BUILTIN_COMPONENTS))
}

fn entries_without(skip: &str) -> String {
    required_component_names(&BUILTIN_COMPONENTS)
        .into_iter()
        .filter(|n| n != skip)
        .map(|n| format!("  {n},"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn template_validation_reports_missing_required_components() {
    let required = required_component_names(&BUILTIN_COMPONENTS);
    // No file at all: every required name is missing.
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        validate_template_mdx_contract(dir.path(), &required),
        required
    );
    // A name only in a comment is missing.
    let body = format!(
        "export const components = {{\n{}\n  // ParamTable, (temporarily disabled)\n}}\n",
        entries_without("ParamTable")
    );
    assert_eq!(validate(&body), ["ParamTable"]);
    // Slashes inside a string literal do not start a comment.
    let entries = required
        .iter()
        .map(|n| format!("{n},"))
        .collect::<Vec<_>>()
        .join(" ");
    let body = format!(
        "const DOCS = \"https://example.com\"; export const components = {{ {entries} }}\n"
    );
    assert_eq!(validate(&body), Vec::<String>::new());
    // A merely imported name is missing.
    let body = format!("import {{ ParamTable }} from \"@/components/param-table\"\nexport const components = {{\n{}\n}}\n", entries_without("ParamTable"));
    assert_eq!(validate(&body), ["ParamTable"]);
    // An `as Name` re-export counts.
    let body = format!(
        "export {{ Foo as ParamTable }} from './foo'\nexport const components = {{\n{}\n}}\n",
        entries_without("ParamTable")
    );
    assert_eq!(validate(&body), Vec::<String>::new());
    // The object-property form passes.
    let body = format!(
        "export const components = {{\n{}\n}}\n",
        entries_without("")
    );
    assert_eq!(validate(&body), Vec::<String>::new());
}

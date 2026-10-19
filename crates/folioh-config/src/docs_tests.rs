use std::path::Path;

use serde_json::json;

use indexmap::IndexMap;

use super::testing::{mapping, parse_err, parse_ok, parse_yaml, parse_yaml_with};
use super::*;
use crate::source::LanguageSource;

#[test]
fn minimal_config_takes_every_default() {
    let config =
        parse_ok("project:\n  name: \"Minimal\"\nsource:\n  python:\n    paths: [\"src/\"]\n");
    assert_eq!(config.project.name, "Minimal");
    assert_eq!(config.project.version, "0.0.0");
    assert_eq!(config.project.repo, "");
    assert_eq!(config.project.repo_ref, "main");
    assert_eq!(config.project.url, "");
    assert_eq!(config.output_dir, "_site");
    assert!(config.theme.dark_mode);
    assert_eq!(config.theme.preset, "pastel");
    assert_eq!(
        parse_ok("project:\n  name: Roller\ntheme:\n  preset: organic-editorial\n")
            .theme
            .preset,
        "organic-editorial"
    );
    assert_eq!(config.theme.radius, "");
    assert_eq!(config.nav, Vec::<String>::new());
    assert_eq!(config.public, Vec::<String>::new());
    assert!(config.llm.generate_llms_txt && config.llm.generate_llms_full_txt);
    assert!(!config.landing_enabled);
    assert_eq!(config.source.docstring_style, "auto");
    assert_eq!(config.source.docs, Vec::<String>::new());
    assert_eq!(
        config.source.languages,
        IndexMap::from([(
            "python".to_string(),
            LanguageSource {
                paths: vec!["src/".to_string()],
                excludes: vec![]
            }
        )])
    );
    assert!(config.sidebar.default_collapsed);
    assert!(config.search.enabled);
    assert!(!config.feedback.enabled);
    assert_eq!(config.search.placeholder, "");
    assert_eq!(
        config.deploy,
        DeployConfig {
            provider: String::new(),
            base_path: String::new()
        }
    );
    assert_eq!(config.template.docs_route_base, "/docs");
    assert_eq!(config.template.path, "");
    assert!(config.extra.is_empty());
    assert!(config.versions.is_empty());
    assert_eq!(config.i18n, I18nConfig::default());
    assert_eq!(config.project_dir, Path::new("/proj"));
}

#[test]
fn feedback_requires_an_explicit_boolean_opt_in() {
    for (section, enabled) in [
        ("", false),
        ("feedback: {}\n", false),
        ("feedback:\n  enabled: false\n", false),
        ("feedback:\n  enabled: true\n", true),
    ] {
        let yaml = format!("project:\n  name: T\n  repo: https://github.com/acme/docs\n{section}");
        let (config, warnings) = parse_yaml(&yaml).unwrap();
        assert_eq!(config.feedback.enabled, enabled, "{yaml}");
        assert!(warnings.is_empty(), "{warnings:?}");
    }
    for value in ["null", "\"true\"", "1", "[]"] {
        assert_eq!(
            parse_err(&format!("feedback:\n  enabled: {value}\n")),
            "feedback.enabled must be a boolean"
        );
    }
}

#[test]
fn empty_document_is_an_untitled_project() {
    let (config, warnings) = parse_yaml("").unwrap();
    assert_eq!(config.project.name, "Untitled");
    assert_eq!(
        warnings,
        ["project.name must be a non-empty string in docs.yaml; defaulting to 'Untitled'"]
    );
}

#[test]
fn core_sections_must_be_mappings() {
    for section in ["project", "source", "theme", "llm", "feedback"] {
        let yaml = format!("{section}: not-a-mapping\n");
        assert_eq!(parse_err(&yaml), format!("{section} must be a mapping"));
        // A bare `key:` line is an explicit null, not a missing section.
        let yaml = format!("{section}:\n");
        assert_eq!(parse_err(&yaml), format!("{section} must be a mapping"));
    }
}

#[test]
fn invalid_project_name_warns_and_defaults() {
    for name in ["\"\"", "42", "null", "\"   \""] {
        let (config, warnings) = parse_yaml(&format!("project:\n  name: {name}\n")).unwrap();
        assert_eq!(config.project.name, "Untitled", "name {name}");
        assert_eq!(
            warnings,
            ["project.name must be a non-empty string in docs.yaml; defaulting to 'Untitled'"]
        );
    }
    assert_eq!(
        parse_ok("project:\n  name: \"  Spaced  \"\n").project.name,
        "  Spaced  "
    );
}

#[test]
fn list_fields_reject_scalars_and_non_string_items() {
    for (yaml, field) in [
        ("source:\n  docs: docs\n", "source.docs"),
        (
            "source:\n  python:\n    paths: src\n",
            "source.python.paths",
        ),
        (
            "source:\n  python:\n    exclude: tests\n",
            "source.python.exclude",
        ),
        ("nav: Guide\n", "nav"),
        ("nav: [Guide, 3]\n", "nav"),
        ("public: install.sh\n", "public"),
        ("public: [install.sh, 3]\n", "public"),
        ("source:\n  docs: [docs, null]\n", "source.docs"),
    ] {
        let yaml = format!("project:\n  name: Test\n{yaml}");
        assert_eq!(
            parse_err(&yaml),
            format!("{field} must be a list of strings")
        );
    }
}

#[test]
fn source_python_accepts_a_mapping_a_list_or_null() {
    let config = parse_ok("project: {name: T}\nsource:\n  python: [\"a\", \"b\"]\n");
    assert_eq!(
        config.language_source("python"),
        LanguageSource {
            paths: vec!["a".into(), "b".into()],
            excludes: vec![]
        }
    );
    let config = parse_ok("project: {name: T}\nsource:\n  python:\n");
    assert_eq!(config.language_source("python"), LanguageSource::default());
    let (config, warnings) = parse_yaml(
            "project: {name: T}\nsource:\n  python:\n    paths: [\"src\"]\n    exclude: [\"src/tests\", \"\"]\n    docstring_style: numpy\n    other: ignored\n",
        ).unwrap();
    assert_eq!(warnings, ["Unknown source.python keys in docs.yaml: other"]);
    assert_eq!(
        config.source.languages["python"].excludes,
        vec!["src/tests", ""]
    );
    assert_eq!(config.source.docstring_style, "numpy");
    assert_eq!(
        parse_err("project: {name: T}\nsource:\n  python: [\"a\", 1]\n"),
        "source.python must be a list of strings"
    );
    assert_eq!(
        parse_err("project: {name: T}\nsource:\n  python: src\n"),
        "source.python must be a mapping or a list of strings"
    );
}

#[test]
fn docstring_style_warns_on_unknown_values_and_keeps_them() {
    let (config, warnings) = parse_yaml(
        "project: {name: T}\nsource:\n  python:\n    paths: [src]\n    docstring_style: sphinx\n",
    )
    .unwrap();
    assert_eq!(config.source.docstring_style, "sphinx");
    assert_eq!(
            warnings,
            ["source.python.docstring_style must be one of 'auto', 'google', 'numpy'; got 'sphinx'; using 'google'"]
        );
    assert_eq!(
        parse_err("project: {name: T}\nsource:\n  python:\n    docstring_style: 3\n"),
        "source.python.docstring_style must be a string"
    );
}

#[test]
fn language_sources_are_parsed_in_language_id_order() {
    let config = parse_ok(
            "project: {name: Polyglot}\nsource:\n  rust:\n    paths: [\"crates/core\"]\n  python:\n    paths: [\"src/\"]\n  javascript:\n    paths: [\"web/src\"]\n    exclude: [\"web/src/vendor\"]\n",
        );
    let keys: Vec<&String> = config.source.languages.keys().collect();
    assert_eq!(keys, ["python", "javascript", "rust"]);
    assert_eq!(
        config.language_source("javascript"),
        LanguageSource {
            paths: vec!["web/src".into()],
            excludes: vec!["web/src/vendor".into()]
        }
    );
    assert_eq!(
        config.language_source("rust"),
        LanguageSource {
            paths: vec!["crates/core".into()],
            excludes: vec![]
        }
    );
    assert_eq!(config.language_source("cobol"), LanguageSource::default());
    assert_eq!(config.source_roots(), ["src/", "web/src", "crates/core"]);
}

#[test]
fn malformed_language_sources_fail_with_field_messages() {
    for (source, message) in [
        ("rust: [crates]", "source.rust must be a mapping"),
        ("javascript: web", "source.javascript must be a mapping"),
        ("rust:\n    exclude: [x]", "source.rust.paths is required"),
        (
            "rust:\n    paths: crates",
            "source.rust.paths must be a list of strings",
        ),
        (
            "rust:\n    paths: null",
            "source.rust.paths must be a list of strings",
        ),
        (
            "javascript:\n    paths: [1]",
            "source.javascript.paths must be a list of strings",
        ),
        (
            "javascript:\n    paths: [web]\n    exclude: vendor",
            "source.javascript.exclude must be a list of strings",
        ),
    ] {
        let yaml = format!("project: {{name: Test}}\nsource:\n  {source}\n");
        assert_eq!(parse_err(&yaml), message, "{source}");
    }
}

#[test]
fn unknown_source_keys_warn_sorted() {
    let (_, warnings) = parse_yaml(
        "project: {name: T}\nsource:\n  pyhton: {paths: [x]}\n  go: {paths: [y]}\n  docs: []\n",
    )
    .unwrap();
    assert_eq!(
        warnings,
        ["Unknown source keys in docs.yaml: go, pyhton (did you mean 'python'?)"]
    );
}

#[test]
fn unknown_top_level_keys_warn_before_validation() {
    let (_, warnings) =
        parse_yaml("project: {name: T}\noverrides: docs/overrides\nzeta: 1\nalpha: 2\n").unwrap();
    assert_eq!(
        warnings,
        ["Unknown config keys in docs.yaml: alpha, overrides, zeta"]
    );
    // Every released key and the three Docs built-in sections are silent.
    let config = parse_ok(
            "project: {name: T}\nsource: {}\noutput: out\ntheme: {}\nnav: []\npublic: []\nsidebar: {}\nllm: {}\ncomponents: []\nsearch: {}\ndeploy: {}\ntemplate: {}\nlanding: {}\nroadmap: {}\nopenapi: {}\n",
        );
    assert!(config.landing_enabled);
    // The warning is emitted even when a later field fails.
    let mut warnings = Vec::new();
    let err = parse_docs_config_with(
        &mapping("project: {name: T}\nbogus: 1\nnav: Guide\n"),
        Path::new("/proj"),
        "",
        &mut warnings,
    )
    .unwrap_err();
    assert_eq!(err.to_string(), "nav must be a list of strings");
    assert_eq!(warnings, ["Unknown config keys in docs.yaml: bogus"]);
}

#[test]
fn plugins_key_warns_once_even_when_null() {
    let message = "project plugins are not available in this release; the plugins key is ignored";
    let (_, warnings) =
        parse_yaml("project: {name: T}\nplugins:\n  - \"./plugins/x.py\"\n").unwrap();
    assert_eq!(warnings, [message]);
    assert_eq!(
        parse_yaml("project: {name: T}\nplugins:\n").unwrap().1,
        [message]
    );
    assert!(parse_yaml("project: {name: T}\n").unwrap().1.is_empty());
}

#[test]
fn typos_in_nested_sections_warn_with_the_near_key() {
    let (config, warnings) = parse_yaml(
        "project: {name: T, vesion: \"1.0\"}\nsearch: {placeholdr: Find}\nllm: {generate_llm_txt: false}\nsidebar: {collapsed: false}\ndeploy: {base-path: /x}\ntemplate: {docs_route: /guide}\ntheme: {darkMode: false}\nsource:\n  rust: {paths: [crates], exlude: [x]}\ncomponents:\n  - {name: Hero, from: hero.tsx, expose: {mdx: true, landing: true}}\n  - {from: other.tsx, exprt: Other}\n",
    )
    .unwrap();
    assert_eq!(
        warnings,
        [
            "Unknown project keys in docs.yaml: vesion (did you mean 'version'?)",
            "Unknown llm keys in docs.yaml: generate_llm_txt (did you mean 'generate_llms_txt'?)",
            "Unknown deploy keys in docs.yaml: base-path (did you mean 'base_path'?)",
            "Unknown sidebar keys in docs.yaml: collapsed",
            "Unknown search keys in docs.yaml: placeholdr (did you mean 'placeholder'?)",
            "Unknown source.rust keys in docs.yaml: exlude (did you mean 'exclude'?)",
            "Unknown components.Hero.expose keys in docs.yaml: landing",
            "Unknown components[1] keys in docs.yaml: exprt (did you mean 'export'?)",
            "Unknown theme keys in docs.yaml: darkMode (did you mean 'dark_mode'?)",
            "Unknown template keys in docs.yaml: docs_route",
        ]
    );
    // The typo is ignored, not guessed at.
    assert!(config.theme.dark_mode);
    assert_eq!(config.project.version, "0.0.0");
}

#[test]
fn project_scalars_are_typed() {
    let config = parse_ok("project:\n  name: T\n  version: \"2.0.0\"\n  url: \"https://acme.dev\"\n  repo_ref: \"release/2.x\"\n");
    assert_eq!(config.project.version, "2.0.0");
    assert_eq!(config.project.url, "https://acme.dev");
    assert_eq!(config.project.repo_ref, "release/2.x");
    assert_eq!(
        parse_err("project:\n  name: T\n  version: 1.0\n"),
        "project.version must be a string"
    );
    assert_eq!(
        parse_err("project:\n  name: T\n  version:\n"),
        "project.version must be a string"
    );
    assert_eq!(
        parse_err("project:\n  name: T\n  url: 3\n"),
        "project.url must be a string"
    );
    for repo_ref in ["\"  \"", "3", "null", "[a]"] {
        assert_eq!(
            parse_ok(&format!("project:\n  name: T\n  repo_ref: {repo_ref}\n"))
                .project
                .repo_ref,
            "main"
        );
    }
    assert_eq!(
        parse_ok("project:\n  name: T\n  repo_ref: \" dev \"\n")
            .project
            .repo_ref,
        "dev"
    );
}

#[test]
fn project_repo_rejects_dangerous_schemes_and_unsafe_text() {
    for repo in [
        "javascript:alert(1)",
        "data:text/html,<script>alert(1)</script>",
        "vbscript:msgbox(1)",
        "file:///etc/passwd",
        "JAVASCRIPT:alert(1)",
    ] {
        let err = parse_err(&format!(
            "project:\n  name: T\n  repo: {}\n",
            serde_json::to_string(repo).unwrap()
        ));
        assert!(err.starts_with("project.repo "), "{repo}: {err}");
        if repo.contains('<') {
            assert_eq!(err, "project.repo contains unsafe text");
        } else {
            assert_eq!(
                err,
                "project.repo cannot use the javascript:, data:, vbscript:, or file: scheme"
            );
        }
    }
    assert_eq!(
        parse_err("project:\n  name: T\n  repo: 42\n"),
        "project.repo must be a string"
    );
    for repo in [
        "https://github.com/acme/docs",
        "ssh://git@github.com/acme/docs.git",
        "git://github.com/acme/docs.git",
        "git+https://github.com/acme/docs.git",
        "git@github.com:acme/docs.git",
        "acme/docs",
    ] {
        assert_eq!(
            parse_ok(&format!("project:\n  name: T\n  repo: \"{repo}\"\n"))
                .project
                .repo,
            repo
        );
    }
    assert_eq!(
        parse_ok("project:\n  name: T\n  repo: \" x \"\n")
            .project
            .repo,
        "x"
    );
    assert_eq!(
        parse_ok("project:\n  name: T\n  repo: \"\"\n").project.repo,
        ""
    );
}

#[test]
fn deploy_settings_are_lenient() {
    let config =
        parse_ok("project: {name: T}\ndeploy:\n  provider: github-pages\n  base_path: docs\n");
    assert_eq!(
        config.deploy,
        DeployConfig {
            provider: "github-pages".into(),
            base_path: "/docs".into()
        }
    );
    assert_eq!(
        parse_ok("project: {name: T}\ndeploy:\n  provider: 3\n  base_path: 3\n").deploy,
        DeployConfig::default()
    );
    assert_eq!(
        parse_ok("project: {name: T}\ndeploy: nope\n").deploy,
        DeployConfig::default()
    );
}

#[test]
fn sidebar_llm_and_search_scalars() {
    assert!(
        parse_ok("project: {name: T}\nsidebar:\n  default_collapsed: true\n")
            .sidebar
            .default_collapsed
    );
    assert!(parse_ok("project: {name: T}\n").sidebar.default_collapsed);
    assert!(
        parse_ok("project: {name: T}\nsidebar: nope\n")
            .sidebar
            .default_collapsed
    );
    assert!(
        !parse_ok("project: {name: T}\nsidebar:\n  default_collapsed: false\n")
            .sidebar
            .default_collapsed
    );
    assert_eq!(
        parse_err("project: {name: T}\nsidebar:\n  default_collapsed: yes please\n"),
        "sidebar.default_collapsed must be a boolean"
    );
    // An explicit null is not "unset": Python stored None (feature off).
    for (yaml, field) in [
        (
            "sidebar:\n  default_collapsed:\n",
            "sidebar.default_collapsed",
        ),
        ("llm:\n  generate_llms_txt:\n", "llm.generate_llms_txt"),
        (
            "llm:\n  generate_llms_full_txt: null\n",
            "llm.generate_llms_full_txt",
        ),
        ("search:\n  enabled: ~\n", "search.enabled"),
        ("theme:\n  dark_mode:\n", "theme.dark_mode"),
    ] {
        let err = parse_yaml(&format!("project: {{name: T}}\n{yaml}")).expect_err(yaml);
        assert_eq!(err.to_string(), format!("{field} must be a boolean"));
    }

    let config = parse_ok("project: {name: T}\nllm:\n  generate_llms_txt: false\n");
    assert!(!config.llm.generate_llms_txt && config.llm.generate_llms_full_txt);
    assert_eq!(
        parse_err("project: {name: T}\nllm:\n  generate_llms_txt: 1\n"),
        "llm.generate_llms_txt must be a boolean"
    );
    assert_eq!(
        parse_err("project: {name: T}\nllm:\n  generate_llms_full_txt: \"no\"\n"),
        "llm.generate_llms_full_txt must be a boolean"
    );

    let config = parse_ok("project: {name: T}\nsearch:\n  enabled: false\n");
    assert!(!config.search.enabled);
    assert_eq!(config.search.placeholder, "");
    let config = parse_ok(
        "project: {name: T}\nsearch:\n  enabled: true\n  placeholder: \"Search the docs...\"\n",
    );
    assert!(config.search.enabled);
    assert_eq!(config.search.placeholder, "Search the docs...");
    assert_eq!(
        parse_ok("project: {name: T}\nsearch: []\n").search,
        SearchConfig {
            enabled: true,
            placeholder: String::new()
        }
    );
    assert_eq!(
        parse_err("project: {name: T}\nsearch:\n  enabled: on\n"),
        "search.enabled must be a boolean"
    );
    assert_eq!(
        parse_err("project: {name: T}\nsearch:\n  placeholder: 1\n"),
        "search.placeholder must be a string"
    );
}

#[test]
fn output_is_stored_as_written() {
    assert_eq!(
        parse_ok("project: {name: T}\noutput: out\n").output_dir,
        "out"
    );
    assert_eq!(
        parse_err("project: {name: T}\noutput:\n"),
        "Output directory must be a non-empty relative path"
    );
    assert_eq!(
        parse_err("project: {name: T}\noutput: 42\n"),
        "Output directory must be a non-empty relative path"
    );
}

#[test]
fn gated_keys_warn_and_are_ignored_until_enabled() {
    let yaml = "project: {name: T}\ni18n:\n  default_locale: en\n  locales:\n    - {code: en, name: English}\n    - {code: es, name: Espanol}\nversions:\n  - {label: latest, path: latest}\n";
    let (config, warnings) = parse_yaml(yaml).unwrap();
    assert_eq!(
        warnings,
        [
            "translated docs are not available in this release; the i18n key is ignored",
            "versioned docs are not available in this release; the versions key is ignored",
        ]
    );
    assert_eq!(config.i18n, I18nConfig::default());
    assert!(config.versions.is_empty());
    // An empty key is still a claim the release does not honour.
    assert_eq!(
        parse_yaml("project: {name: T}\nversions:\n").unwrap().1,
        ["versioned docs are not available in this release; the versions key is ignored"]
    );

    let (config, warnings) = parse_yaml_with(yaml, "i18n,versions").unwrap();
    assert!(warnings.is_empty());
    assert_eq!(config.i18n.default_locale, "en");
    assert_eq!(config.i18n.locales.len(), 2);
    assert_eq!(
        config.i18n.locales[1],
        json!({"code": "es", "name": "Espanol"})
    );
    assert_eq!(
        config.versions,
        vec![json!({"label": "latest", "path": "latest"})]
    );
    assert_eq!(
        parse_yaml_with("project: {name: T}\ni18n: nope\n", "i18n")
            .unwrap()
            .0
            .i18n,
        I18nConfig::default()
    );
    assert_eq!(
        parse_yaml_with("project: {name: T}\nversions: nope\n", "versions")
            .unwrap_err()
            .to_string(),
        "versions must be a list"
    );
}

#[test]
fn released_keys_load_together_without_warnings() {
    let config = parse_ok(
            "project: {name: Demo}\ncomponents:\n  - docs/components\nlanding:\n  enabled: true\nroadmap:\n  phases:\n    - {id: foundation, title: Foundation}\n",
        );
    assert_eq!(config.components.dirs, ["docs/components"]);
    assert!(config.components.specs.is_empty());
    assert!(config.landing_enabled);
    // The built-ins have not run: nothing in `extra` yet.
    assert!(config.extra.is_empty());
}

#[test]
fn landing_enabled_means_the_key_is_present() {
    assert!(!parse_ok("project: {name: T}\n").landing_enabled);
    assert!(parse_ok("project: {name: T}\nlanding:\n").landing_enabled);
    assert!(parse_ok("project: {name: T}\nlanding: false\n").landing_enabled);
}

#[test]
fn serialized_shape_follows_struct_order_and_omits_unset_header_keys() {
    let config = parse_ok("project: {name: T}\ntheme:\n  header:\n    brand: Acme\n");
    let json = serde_json::to_value(&config).unwrap();
    let keys: Vec<&String> = json.as_object().unwrap().keys().collect();
    assert_eq!(
        keys,
        [
            "project",
            "source",
            "output_dir",
            "theme",
            "template",
            "nav",
            "public",
            "sidebar",
            "llm",
            "search",
            "feedback",
            "deploy",
            "components",
            "i18n",
            "versions",
            "landing_enabled",
            "extra",
            "project_dir"
        ]
    );
    assert_eq!(json["theme"]["header"], json!({"brand": "Acme"}));
    assert_eq!(
        json["source"]["languages"],
        json!({"python": {"paths": [], "excludes": []}})
    );
}

#[test]
fn resolve_paths_anchors_every_path_on_base() {
    let config = parse_ok(
            "project: {name: T}\nsource:\n  python:\n    paths: [\"src/\"]\n    exclude: [\"**/test_*.py\"]\n  rust:\n    paths: [crates/core]\n    exclude: [crates/core/tests]\n  docs: [\"docs/\"]\noutput: out\ntheme:\n  package: docs/theme\ntemplate:\n  overlay_path: overlay\ncomponents:\n  - docs/components\n  - {name: Hero, from: docs/hero.tsx}\n  - {name: Old, path: docs/old.tsx}\n",
        );
    let resolved = config.resolve_paths(Path::new("/proj")).unwrap();
    assert_eq!(resolved.output_dir, "/proj/out");
    assert_eq!(resolved.language_source("python").paths, ["/proj/src"]);
    assert_eq!(
        resolved.language_source("python").excludes,
        ["/proj/**/test_*.py"]
    );
    assert_eq!(
        resolved.language_source("rust").paths,
        ["/proj/crates/core"]
    );
    assert_eq!(
        resolved.language_source("rust").excludes,
        ["/proj/crates/core/tests"]
    );
    assert_eq!(resolved.source.docs, ["/proj/docs"]);
    assert_eq!(resolved.theme.package_path, "/proj/docs/theme");
    assert_eq!(resolved.template.overlay_path, "/proj/overlay");
    assert_eq!(resolved.template.path, "");
    assert_eq!(resolved.components.dirs, ["/proj/docs/components"]);
    assert_eq!(
        resolved.components.specs[0].from.as_deref(),
        Some("/proj/docs/hero.tsx")
    );
    assert_eq!(
        resolved.components.specs[1].rest["path"],
        json!("/proj/docs/old.tsx")
    );
    assert_eq!(resolved.project_dir, Path::new("/proj"));
    // Everything else is copied.
    assert_eq!(resolved.project, config.project);
    assert_eq!(resolved.theme.tune, config.theme.tune);
    assert_eq!(resolved.template.params, config.template.params);
}

#[test]
fn resolve_paths_refuses_an_output_dir_over_a_language_root() {
    let config = parse_ok(
        "project: {name: Demo}\noutput: crates\nsource:\n  rust:\n    paths: [crates/core]\n",
    );
    let err = config
        .resolve_paths(Path::new("/proj"))
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("would delete the source directory 'crates/core'"),
        "{err}"
    );
}

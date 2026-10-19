use serde_json::json;
use serde_yaml_ng::Value;

use super::*;
use crate::docs::testing::{parse_err, parse_ok, parse_yaml};

fn yaml(text: &str) -> Value {
    serde_yaml_ng::from_str(text).unwrap()
}

#[test]
fn theme_preset_and_scalars_are_typed() {
    let config = parse_ok("project: {name: T}\ntheme:\n  preset: beacon\n  dark_mode: false\n  logo: l.svg\n  favicon: f.ico\n  name: 3\n  description: [x]\n  scene: {}\n");
    assert_eq!(config.theme.preset, "beacon");
    assert!(!config.theme.dark_mode);
    assert_eq!(config.theme.logo, "l.svg");
    assert_eq!(config.theme.favicon, "f.ico");
    assert_eq!(
        (
            config.theme.name.as_str(),
            config.theme.description.as_str(),
            config.theme.scene.as_str()
        ),
        ("", "", "")
    );
    assert_eq!(
        parse_err("project: {name: T}\ntheme:\n  preset: 3\n"),
        "theme.preset must be a string"
    );
    assert_eq!(
        parse_err("project: {name: T}\ntheme:\n  preset:\n"),
        "theme.preset must be a string"
    );
    assert_eq!(
        parse_err("project: {name: T}\ntheme:\n  dark_mode: yes\n"),
        "theme.dark_mode must be a boolean"
    );
    assert_eq!(
        parse_err("project: {name: T}\ntheme:\n  logo: 3\n"),
        "theme.logo must be a string"
    );
    assert_eq!(
        parse_err("project: {name: T}\ntheme:\n  favicon: 3\n"),
        "theme.favicon must be a string"
    );
}

#[test]
fn extra_presets_require_an_explicit_boolean_opt_in() {
    assert!(!parse_ok("project: {name: T}\n").theme.extra_presets);
    for value in ["true", "false"] {
        let config = parse_ok(&format!(
            "project: {{name: T}}\ntheme: {{extra_presets: {value}}}\n"
        ));
        assert_eq!(config.theme.extra_presets, value == "true");
    }
    for value in ["null", "yes", "1"] {
        assert_eq!(
            parse_err(&format!(
                "project: {{name: T}}\ntheme: {{extra_presets: {value}}}\n"
            )),
            "theme.extra_presets must be a boolean"
        );
    }
}

#[test]
fn project_theme_contract_parses_to_canonical_ids() {
    let config = parse_ok(
        r#"
project:
  name: "ThemeDocs"
theme:
  preset: "p2pfl"
  name: "P2PFL"
  description: "Operational docs theme"
  scene: "Maintainers inspect experiments, nodes, and APIs in one compact surface."
  preview:
    light: "oklch(0.490 0.130 285)"
    dark: "oklch(0.720 0.100 285)"
  radius: "0.5rem"
  tune:
    font: "sans"
    accent: "ink"
    surface: "preset"
    shell: "flush"
    width: "wide"
    rhythm: "compact"
    borders: "fine"
    code: "terminal"
  style:
    "--content-max-width": "74rem"
    "--body-line-height": "1.58"
  tokens:
    light:
      "--background": "oklch(0.985 0.008 80)"
      "--foreground": "oklch(0.175 0.008 75)"
      "--brand-accent": "oklch(0.680 0.110 160)"
    dark:
      "--background": "oklch(0.155 0.010 75)"
      "--foreground": "oklch(0.950 0.008 80)"
"#,
    );
    let theme = &config.theme;
    assert_eq!(theme.preset, "p2pfl");
    assert_eq!(theme.name, "P2PFL");
    assert_eq!(theme.description, "Operational docs theme");
    assert!(theme.scene.starts_with("Maintainers inspect"));
    assert_eq!(
        serde_json::to_value(&theme.preview).unwrap(),
        json!({"light": "oklch(0.490 0.130 285)", "dark": "oklch(0.720 0.100 285)"})
    );
    assert_eq!(theme.radius, "0.5rem");
    assert_eq!(
        serde_json::to_value(&theme.tune).unwrap(),
        json!({
            "fontId": "sans", "colorId": "ink", "surfaceColorId": "preset", "shellPaddingId": "flush",
            "contentWidthId": "wide", "rhythmId": "compact", "borderId": "fine", "codeTreatmentId": "terminal"
        })
    );
    assert_eq!(
        serde_json::to_value(&theme.style).unwrap(),
        json!({"--content-max-width": "74rem", "--body-line-height": "1.58"})
    );
    assert_eq!(
        theme.tokens["light"]["--brand-accent"],
        "oklch(0.680 0.110 160)"
    );
    assert_eq!(theme.tokens["dark"].len(), 2);
    let resolved = config.resolve_paths(std::path::Path::new("/proj")).unwrap();
    assert_eq!(resolved.theme.tokens, theme.tokens);
    assert_eq!(resolved.theme.tune, theme.tune);
}

#[test]
fn variants_and_header_contract_have_the_exact_shape() {
    let config = parse_ok(
        r#"
project:
  name: "ThemeDocs"
theme:
  header:
    brand: "p2pfl"
    badge: "Web Services"
    repo: "https://github.com/pguijas/p2pfl"
    theme_toggle: true
    action_label: "Dashboard"
    action_href: "/dashboard"
    search: false
  variants:
    palette:
      label: "Palette"
      default: "default"
      options:
        default:
          label: "Default"
          swatch: "oklch(0.490 0.130 285)"
          preview:
            light: "oklch(0.490 0.130 285)"
            dark: "oklch(0.720 0.100 285)"
        midnight:
          label: "Midnight"
          swatch: "oklch(0.680 0.180 200)"
          preview:
            light: "oklch(0.480 0.160 200)"
            dark: "oklch(0.680 0.180 200)"
          tokens:
            light:
              "--background": "oklch(0.985 0.008 250)"
              "--primary": "oklch(0.480 0.160 200)"
            dark:
              "--background": "oklch(0.095 0.020 250)"
              "--primary": "oklch(0.680 0.180 200)"
"#,
    );
    assert_eq!(
        serde_json::to_value(&config.theme.header).unwrap(),
        json!({
            "brand": "p2pfl", "badge": "Web Services", "action_label": "Dashboard",
            "repo": "https://github.com/pguijas/p2pfl", "action_href": "/dashboard",
            "theme_toggle": true, "search": false
        })
    );
    assert_eq!(
        serde_json::to_value(&config.theme.variants).unwrap(),
        json!({
            "palette": {
                "label": "Palette",
                "description": "",
                "default": "default",
                "options": {
                    "default": {
                        "label": "Default", "description": "", "swatch": "oklch(0.490 0.130 285)",
                        "preview": {"light": "oklch(0.490 0.130 285)", "dark": "oklch(0.720 0.100 285)"},
                        "style": {}, "tokens": {}
                    },
                    "midnight": {
                        "label": "Midnight", "description": "", "swatch": "oklch(0.680 0.180 200)",
                        "preview": {"light": "oklch(0.480 0.160 200)", "dark": "oklch(0.680 0.180 200)"},
                        "style": {},
                        "tokens": {
                            "light": {"--background": "oklch(0.985 0.008 250)", "--primary": "oklch(0.480 0.160 200)"},
                            "dark": {"--background": "oklch(0.095 0.020 250)", "--primary": "oklch(0.680 0.180 200)"}
                        }
                    }
                }
            }
        })
    );
    let resolved = config.resolve_paths(std::path::Path::new("/proj")).unwrap();
    assert_eq!(resolved.theme.header, config.theme.header);
    assert_eq!(resolved.theme.variants, config.theme.variants);
}

#[test]
fn unsafe_tokens_name_the_mode() {
    assert_eq!(
            parse_err("project: {name: T}\ntheme:\n  tokens:\n    light:\n      \"background\": \"oklch(1 0 0)\"\n"),
            "theme.tokens.light contains invalid CSS custom property 'background'"
        );
    assert_eq!(
        parse_err("project: {name: T}\ntheme:\n  tokens:\n    dark:\n      \"--x\": \"a; b\"\n"),
        "theme.tokens.dark contains an unsafe CSS value"
    );
    assert_eq!(
        parse_err("project: {name: T}\ntheme:\n  tokens: [x]\n"),
        "theme.tokens must be a mapping"
    );
    assert_eq!(
        parse_err("project: {name: T}\ntheme:\n  tokens:\n    light: 3\n"),
        "theme.tokens.light must be a mapping of CSS custom properties"
    );
    // Other modes are ignored; empty modes are omitted; null/"" mean none.
    let config =
        parse_ok("project: {name: T}\ntheme:\n  tokens:\n    light: {}\n    sepia: {\"--x\": y}\n");
    assert!(config.theme.tokens.is_empty());
    assert!(parse_ok("project: {name: T}\ntheme:\n  tokens: \"\"\n")
        .theme
        .tokens
        .is_empty());
}

#[test]
fn style_follows_the_css_var_rule() {
    assert_eq!(
        parse_err("project: {name: T}\ntheme:\n  style: x\n"),
        "theme.style must be a mapping of CSS custom properties"
    );
    assert_eq!(
        parse_err("project: {name: T}\ntheme:\n  style:\n    \"--x\": 3\n"),
        "theme.style values must be strings"
    );
    assert_eq!(
        parse_err("project: {name: T}\ntheme:\n  style:\n    \"--x\": \"\"\n"),
        "theme.style contains an unsafe CSS value"
    );
    assert_eq!(
        parse_err("project: {name: T}\ntheme:\n  style:\n    \"--x\": \"url(<a>)\"\n"),
        "theme.style contains an unsafe CSS value"
    );
    assert_eq!(
        parse_err("project: {name: T}\ntheme:\n  style:\n    \"-x\": \"a\"\n"),
        "theme.style contains invalid CSS custom property '-x'"
    );
    assert_eq!(
        parse_err("project: {name: T}\ntheme:\n  style:\n    1: \"a\"\n"),
        "theme.style contains invalid CSS custom property 1"
    );
    let config = parse_ok("project: {name: T}\ntheme:\n  style:\n    \"--folioh-content-max-width\": \" 82rem \"\n    \"--content-max-width\": \"74rem\"\n");
    assert_eq!(
        serde_json::to_value(&config.theme.style).unwrap(),
        json!({"--folioh-content-max-width": "82rem", "--content-max-width": "74rem"})
    );
}

#[test]
fn header_hrefs_reject_unsafe_schemes_and_accept_relative_paths() {
    for key in ["repo", "action_href"] {
        assert_eq!(
            parse_err(&format!(
                "project: {{name: T}}\ntheme:\n  header:\n    {key}: \"javascript:alert(1)\"\n"
            )),
            format!("theme.header.{key} must be an http(s) URL or a relative path")
        );
    }
    let config = parse_ok("project: {name: T}\ntheme:\n  header:\n    repo: \"https://github.com/acme/docs\"\n    action_href: \"/dashboard\"\n    badge: \"mailto:x@y\"\n");
    assert_eq!(
        config.theme.header.repo.as_deref(),
        Some("https://github.com/acme/docs")
    );
    assert_eq!(
        config.theme.header.action_href.as_deref(),
        Some("/dashboard")
    );
    for href in ["docs/index", "mailto:x@y", "HTTPS://x", "MAILTO:x@y"] {
        let yaml =
            format!("project: {{name: T}}\ntheme:\n  header:\n    action_href: \"{href}\"\n");
        assert_eq!(
            parse_ok(&yaml).theme.header.action_href.as_deref(),
            Some(href)
        );
    }
}

#[test]
fn header_text_and_bool_rules() {
    assert_eq!(
        parse_err("project: {name: T}\ntheme:\n  header: x\n"),
        "theme.header must be a mapping"
    );
    assert_eq!(
        parse_err("project: {name: T}\ntheme:\n  header:\n    brand: 3\n"),
        "theme.header.brand must be a string"
    );
    assert_eq!(
        parse_err("project: {name: T}\ntheme:\n  header:\n    badge: \"a<b\"\n"),
        "theme.header.badge contains unsafe text"
    );
    assert_eq!(
        parse_err("project: {name: T}\ntheme:\n  header:\n    theme_toggle: maybe\n"),
        "theme.header.theme_toggle must be a boolean"
    );
    assert_eq!(
        parse_err("project: {name: T}\ntheme:\n  header:\n    search: 1\n"),
        "theme.header.search must be a boolean"
    );
    let config = parse_ok("project: {name: T}\ntheme:\n  header:\n    brand: \" Acme \"\n    theme_toggle: \" YES \"\n    search: \"off\"\n    badge: \"\"\n    repo: null\n");
    assert_eq!(
        config.theme.header,
        ThemeHeader {
            brand: Some("Acme".into()),
            theme_toggle: Some(true),
            search: Some(false),
            ..ThemeHeader::default()
        }
    );
    assert!(
        parse_ok("project: {name: T}\ntheme:\n  header: \"\"\n")
            .theme
            .header
            == ThemeHeader::default()
    );
}

#[test]
fn header_and_preview_unknown_keys_warn() {
    let mut warnings = Vec::new();
    let header = theme_header(&yaml("{brand: Acme, bogus: x}"), &mut warnings).unwrap();
    assert_eq!(warnings, ["Unknown theme.header key 'bogus'; ignoring it"]);
    assert_eq!(
        header,
        ThemeHeader {
            brand: Some("Acme".into()),
            ..ThemeHeader::default()
        }
    );

    let mut warnings = Vec::new();
    let header = theme_header(
        &yaml("{brand: Acme, theme_toggle: true, action_href: /x}"),
        &mut warnings,
    )
    .unwrap();
    assert!(warnings.is_empty());
    assert_eq!(
        header,
        ThemeHeader {
            brand: Some("Acme".into()),
            theme_toggle: Some(true),
            action_href: Some("/x".into()),
            ..ThemeHeader::default()
        }
    );

    let mut warnings = Vec::new();
    let preview = theme_preview(&yaml("{light: 'oklch(0.5 0 0)', bogus: x}"), &mut warnings);
    assert_eq!(warnings, ["Unknown theme.preview key 'bogus'; ignoring it"]);
    assert_eq!(
        serde_json::to_value(preview).unwrap(),
        json!({"light": "oklch(0.5 0 0)"})
    );

    let mut warnings = Vec::new();
    let preview = theme_preview(&yaml("{dark: ' d ', light: l, sepia: 3}"), &mut warnings);
    assert_eq!(warnings, ["Unknown theme.preview key 'sepia'; ignoring it"]);
    assert_eq!(
        serde_json::to_string(&preview).unwrap(),
        r#"{"light":"l","dark":"d"}"#
    );
    assert!(theme_preview(&yaml("[x]"), &mut warnings).is_empty());
    assert!(theme_preview(&yaml("{light: 3, dark: ''}"), &mut warnings).is_empty());
}

fn variants_with_option_counts(counts: &[usize]) -> Value {
    let mut controls = serde_yaml_ng::Mapping::new();
    for (index, count) in counts.iter().enumerate() {
        let mut options = serde_yaml_ng::Mapping::new();
        for option in 0..*count {
            options.insert(
                Value::String(format!("opt{option}")),
                Value::Mapping(Default::default()),
            );
        }
        let mut control = serde_yaml_ng::Mapping::new();
        control.insert(Value::String("options".into()), Value::Mapping(options));
        controls.insert(
            Value::String(format!("control{index}")),
            Value::Mapping(control),
        );
    }
    Value::Mapping(controls)
}

#[test]
fn variants_combination_limit() {
    let variants =
        theme_variants(&variants_with_option_counts(&[4, 4, 4, 4]), &mut Vec::new()).unwrap();
    assert_eq!(variants.len(), 4);
    assert_eq!(variants["control0"].label, "Control0");
    assert_eq!(variants["control0"].default, "opt0");
    assert_eq!(variants["control0"].options["opt1"].label, "Opt1");
    let tail = "option combinations across all controls, which exceeds the limit of 256. Every combination is resolved and embedded into each generated page's HTML, so large products bloat every page. Reduce the number of controls or options.";
    assert_eq!(
        theme_variants(&variants_with_option_counts(&[257]), &mut Vec::new())
            .unwrap_err()
            .to_string(),
        format!("theme.variants define 257 {tail}")
    );
    assert_eq!(
        theme_variants(
            &variants_with_option_counts(&[5, 5, 5, 5, 5, 5]),
            &mut Vec::new()
        )
        .unwrap_err()
        .to_string(),
        format!("theme.variants define 15625 {tail}")
    );
    assert!(theme_variants(&Value::Null, &mut Vec::new())
        .unwrap()
        .is_empty());
    // A product that overflows usize still trips the cap instead of wrapping past it.
    let err = theme_variants(&variants_with_option_counts(&[1000; 7]), &mut Vec::new())
        .unwrap_err()
        .to_string();
    assert!(err.contains("exceeds the limit of 256"), "{err}");
    // An option preview warns with the generic theme.preview wording.
    let mut warnings = Vec::new();
    theme_variants(
        &yaml("{color: {options: {a: {preview: {sepia: x}}}}}"),
        &mut warnings,
    )
    .unwrap();
    assert_eq!(warnings, ["Unknown theme.preview key 'sepia'; ignoring it"]);
}

#[test]
fn variants_shape_errors() {
    let id_tail =
        "must start with a letter and contain only letters, numbers, underscores, or hyphens";
    for (variants, message) in [
        ("x", "theme.variants must be a mapping".to_string()),
        (
            "{1a: {options: {a: {}}}}",
            format!("theme.variants key {id_tail}"),
        ),
        (
            "{color: x}",
            "theme.variants.color must be a mapping".to_string(),
        ),
        (
            "{color: {}}",
            "theme.variants.color.options must be a non-empty mapping".to_string(),
        ),
        (
            "{color: {options: {}}}",
            "theme.variants.color.options must be a non-empty mapping".to_string(),
        ),
        (
            "{color: {options: {'-x': {}}}}",
            format!("theme.variants.color.options key {id_tail}"),
        ),
        (
            "{color: {options: {a: 3}}}",
            "theme.variants.color.options.a must be a mapping".to_string(),
        ),
        (
            "{color: {options: {a: {label: 3}}}}",
            "theme.variants.color.options.a.label must be a string".to_string(),
        ),
        (
            "{color: {options: {a: {swatch: 'a;b'}}}}",
            "theme.variants.color.options.a.swatch contains an unsafe CSS value".to_string(),
        ),
        (
            "{color: {options: {a: {swatch: 3}}}}",
            "theme.variants.color.options.a.swatch values must be strings".to_string(),
        ),
        (
            "{color: {options: {a: {style: {bad: x}}}}}",
            "theme.variants.color.options.a.style contains invalid CSS custom property 'bad'"
                .to_string(),
        ),
        (
            "{color: {options: {a: {tokens: 3}}}}",
            "theme.tokens must be a mapping".to_string(),
        ),
        (
            "{color: {options: {a: {tokens: {light: {bad: x}}}}}}",
            "theme.tokens.light contains invalid CSS custom property 'bad'".to_string(),
        ),
        (
            "{color: {options: {a: {}}, default: b}}",
            "theme.variants.color.default must match an option".to_string(),
        ),
        (
            "{color: {options: {a: {}}, default: 3}}",
            "theme.variants.color.default must be a string".to_string(),
        ),
        (
            "{color: {options: {a: {}}, label: 'a<b'}}",
            "theme.variants.color.label contains unsafe text".to_string(),
        ),
    ] {
        assert_eq!(
            theme_variants(&yaml(variants), &mut Vec::new())
                .unwrap_err()
                .to_string(),
            message,
            "{variants}"
        );
    }
    let variants = theme_variants(
        &yaml("{ocean-blue_2: {options: {deep_sea-x: {}}}}"),
        &mut Vec::new(),
    )
    .unwrap();
    assert_eq!(variants["ocean-blue_2"].label, "Ocean Blue 2");
    assert_eq!(
        variants["ocean-blue_2"].options["deep_sea-x"].label,
        "Deep Sea X"
    );
}

#[test]
fn radius_scale_and_aliases() {
    for radius in ["0", "0.3rem", "0.5rem", "0.75rem", "1rem"] {
        assert_eq!(
            parse_ok(&format!(
                "project: {{name: T}}\ntheme:\n  radius: \"{radius}\"\n"
            ))
            .theme
            .radius,
            radius
        );
    }
    for (alias, expected) in [
        ("none", "0"),
        ("sm", "0.3rem"),
        ("md", "0.5rem"),
        ("lg", "0.75rem"),
        ("full", "1rem"),
        ("SM", "0.3rem"),
        (" md ", "0.5rem"),
    ] {
        assert_eq!(
            parse_ok(&format!(
                "project: {{name: T}}\ntheme:\n  radius: \"{alias}\"\n"
            ))
            .theme
            .radius,
            expected,
            "{alias}"
        );
    }
    assert_eq!(
        parse_ok("project: {name: T}\ntheme:\n  tune:\n    radius: sm\n")
            .theme
            .radius,
        "0.3rem"
    );
    for (radius, hint) in [
        ("0.4rem", " (did you mean '0.3rem'?)"),
        ("2rem", " (did you mean '1rem'?)"),
        ("lgg", " (did you mean 'lg'?)"),
        ("8px", ""),
        ("medium", ""),
    ] {
        assert_eq!(
                parse_err(&format!("project: {{name: T}}\ntheme:\n  radius: \"{radius}\"\n")),
                format!("theme.radius must be one of '0', '0.3rem', '0.5rem', '0.75rem', '1rem' (or a named alias: 'none', 'sm', 'md', 'lg', 'full'); got '{radius}'{hint}")
            );
    }
    // A value from `tune` is named by the key it came from.
    assert_eq!(
        parse_err("project: {name: T}\ntheme:\n  tune:\n    radius: lgg\n"),
        "theme.tune.radius must be one of '0', '0.3rem', '0.5rem', '0.75rem', '1rem' (or a named alias: 'none', 'sm', 'md', 'lg', 'full'); got 'lgg' (did you mean 'lg'?)"
    );
    assert_eq!(parse_ok("project: {name: T}\n").theme.radius, "");
    assert_eq!(
        parse_ok("project: {name: T}\ntheme:\n  radius: 3\n")
            .theme
            .radius,
        ""
    );
    // An explicit `radius: null` wins over the tune alias.
    assert_eq!(
        parse_ok("project: {name: T}\ntheme:\n  radius:\n  tune:\n    radius: sm\n")
            .theme
            .radius,
        ""
    );
}

#[test]
fn tune_aliases_warnings_and_errors() {
    let (config, warnings) = parse_yaml("project: {name: T}\ntheme:\n  tune:\n    accent: laurel\n    bogus: x\n    widht: wide\n    color: \" indigo \"\n    fontId: geist\n    radius: sm\n").unwrap();
    assert_eq!(
        warnings,
        [
            "Unknown theme.tune key 'bogus'; ignoring it",
            "Unknown theme.tune key 'widht' (did you mean 'width'?); ignoring it"
        ]
    );
    assert_eq!(
        serde_json::to_value(&config.theme.tune).unwrap(),
        json!({"colorId": "indigo", "fontId": "geist"})
    );
    assert_eq!(
        parse_err("project: {name: T}\ntheme:\n  tune:\n    rhythm: dense\n"),
        "theme.tune.rhythm must be one of 'preset', 'compact', 'balanced', 'roomy'; got 'dense'"
    );
    assert_eq!(
        parse_err("project: {name: T}\ntheme:\n  tune:\n    codeTreatmentId: termnal\n"),
        "theme.tune.codeTreatmentId must be one of 'preset', 'soft', 'framed', 'plate', 'terminal'; got 'termnal' (did you mean 'terminal'?)"
    );
    assert_eq!(
        parse_err("project: {name: T}\ntheme:\n  tune: x\n"),
        "theme.tune must be a mapping"
    );
    assert_eq!(
        parse_err("project: {name: T}\ntheme:\n  tune:\n    1: x\n"),
        "theme.tune keys must be strings"
    );
    assert_eq!(
        parse_err("project: {name: T}\ntheme:\n  tune:\n    font: 3\n"),
        "theme.tune.font must be a non-empty string"
    );
    assert_eq!(
        parse_err("project: {name: T}\ntheme:\n  tune:\n    accent: \" \"\n"),
        "theme.tune.accent must be a non-empty string"
    );
    assert!(parse_ok("project: {name: T}\ntheme:\n  tune: \"\"\n")
        .theme
        .tune
        .is_empty());
    assert_eq!(THEME_TUNE_KEYS.len(), 8);
    assert_eq!(
        THEME_TUNE_OPTIONS.map(|(id, _)| id),
        THEME_TUNE_KEYS,
        "every tune id lists its options"
    );
    assert_eq!(THEME_TUNE_ALIASES.len(), 20);
    assert_eq!(
        THEME_RADIUS_OPTIONS,
        ["0", "0.3rem", "0.5rem", "0.75rem", "1rem"]
    );
    assert_eq!(THEME_VARIANT_COMBINATION_LIMIT, 256);
}

#[test]
fn theme_package_falls_back_to_legacy_path() {
    assert_eq!(
        parse_ok("project: {name: T}\ntheme:\n  package: docs/theme/p2pfl\n")
            .theme
            .package_path,
        "docs/theme/p2pfl"
    );
    assert_eq!(
        parse_ok("project: {name: T}\ntheme:\n  path: legacy\n")
            .theme
            .package_path,
        "legacy"
    );
    assert_eq!(
        parse_ok("project: {name: T}\ntheme:\n  package: 3\n  path: legacy\n")
            .theme
            .package_path,
        ""
    );
    assert_eq!(parse_ok("project: {name: T}\n").theme.package_path, "");
}

#[test]
fn a_remote_theme_package_is_parsed_and_pinned() {
    const GIT: &str = "https://github.com/acme/folioh-theme";
    let digest = format!("sha256:{}", "a".repeat(64));
    let yaml = |body: &str| format!("project: {{name: T}}\ntheme:\n  package:\n{body}");

    let theme = parse_ok(&yaml(&format!(
        "    git: \"{GIT}\"\n    rev: v1.2.0\n    digest: \"{digest}\"\n"
    )))
    .theme;
    let remote = theme.package_remote.expect("a mapping is a remote package");
    assert_eq!(remote.git, GIT);
    assert_eq!(remote.rev, "v1.2.0");
    assert_eq!(remote.digest, digest);
    assert_eq!(remote.path, "");
    assert_eq!(
        theme.package_path, "",
        "a remote package has no directory until the build fetches it"
    );

    let nested = parse_ok(&yaml(&format!(
        "    git: \"{GIT}\"\n    rev: v1.2.0\n    digest: \"{digest}\"\n    path: /themes/acme/\n"
    )))
    .theme
    .package_remote
    .unwrap();
    assert_eq!(
        nested.path, "themes/acme",
        "surrounding slashes are trimmed"
    );

    for written in [
        "themes/acme",
        "/themes/acme/",
        "./themes/acme",
        "themes//acme",
    ] {
        let normalised = parse_ok(&yaml(&format!(
            "    git: \"{GIT}\"\n    rev: v1.2.0\n    digest: \"{digest}\"\n    path: \"{written}\"\n"
        )))
        .theme
        .package_remote
        .unwrap()
        .path;
        assert_eq!(normalised, "themes/acme", "{written}");
    }

    // The digest compares case-insensitively and is stored lowered.
    let upper = parse_ok(&yaml(&format!(
        "    git: \"{GIT}\"\n    rev: v1.2.0\n    digest: \"SHA256:{}\"\n",
        "A".repeat(64)
    )))
    .theme
    .package_remote
    .unwrap();
    assert_eq!(upper.digest, format!("sha256:{}", "a".repeat(64)));

    // The string form keeps its meaning, and nothing else changes.
    let local = parse_ok("project: {name: T}\ntheme:\n  package: docs/theme/acme\n").theme;
    assert_eq!(local.package_path, "docs/theme/acme");
    assert_eq!(local.package_remote, None);
}

#[test]
fn a_malformed_remote_theme_package_names_the_field_that_is_wrong() {
    let digest = format!("sha256:{}", "a".repeat(64));
    let base = format!("    git: https://example.test/t\n    rev: v1\n    digest: \"{digest}\"\n");
    let yaml = |body: &str| format!("project: {{name: T}}\ntheme:\n  package:\n{body}");
    let err = |body: &str| parse_err(&yaml(body));

    for (body, needle) in [
        ("    rev: v1\n", "theme.package.git is required"),
        (
            &format!("    git: ftp://example.test/t\n    rev: v1\n    digest: \"{digest}\"\n"),
            "must start with https://, ssh://, git@ or file://",
        ),
        (
            &format!("    git: https://example.test/t\n    digest: \"{digest}\"\n"),
            "theme.package.rev is required",
        ),
        (
            "    git: https://example.test/t\n    rev: v1\n",
            "theme.package.digest is required",
        ),
        (
            "    git: https://example.test/t\n    rev: v1\n    digest: deadbeef\n",
            "sha256: followed by 64 hex characters",
        ),
        (
            &format!("{base}    path: ../outside\n"),
            "must be a relative path inside the repository",
        ),
        (
            &format!("{base}    branch: main\n"),
            "unknown keys in theme.package: branch",
        ),
        (
            &format!("    git: http://example.test/t\n    rev: v1\n    digest: \"{digest}\"\n"),
            "must start with https://, ssh://, git@ or file://",
        ),
        (
            &format!("    git: \"--upload-pack=x\"\n    rev: v1\n    digest: \"{digest}\"\n"),
            "theme.package.git must not start with '-'",
        ),
        (
            &format!(
                "    git: https://example.test/t\n    rev: \"--exec=x\"\n    digest: \"{digest}\"\n"
            ),
            "theme.package.rev must not start with '-'",
        ),
        (
            "    git: https://example.test/t\n    rev: \"v1 v2\"\n    digest: deadbeef\n",
            "theme.package.rev must not contain whitespace",
        ),
        (
            &format!(
                "    git: https://example.test/t\n    rev: v1\n    digest: \"sha256:{}\"\n",
                "z".repeat(64)
            ),
            "sha256: followed by 64 hex characters",
        ),
    ] {
        let message = err(body);
        assert!(message.contains(needle), "{needle:?} in {message:?}");
    }
}

#[test]
fn a_theme_toggle_is_ignored_without_dark_mode() {
    let (config, warnings) = parse_yaml(
        "project: {name: T}\ntheme:\n  dark_mode: false\n  header:\n    theme_toggle: true\n",
    )
    .unwrap();
    assert!(!config.theme.dark_mode);
    assert_eq!(
        warnings,
        ["theme.header.theme_toggle is ignored while theme.dark_mode is false"]
    );
    let (_, warnings) =
        parse_yaml("project: {name: T}\ntheme:\n  header:\n    theme_toggle: true\n").unwrap();
    assert!(warnings.is_empty());
}

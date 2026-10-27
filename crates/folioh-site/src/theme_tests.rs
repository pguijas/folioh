use super::*;

#[test]
fn radius_index_falls_back_to_the_default_with_a_warning() {
    let mut warnings = Vec::new();
    assert_eq!(theme_radius_index("13px", &mut warnings), 2);
    assert_eq!(
        warnings,
        vec!["theme radius '13px' is not on the shared radius scale; falling back to 0.5rem"]
    );
    assert_eq!(theme_radius_index("0.75rem", &mut warnings), 3);
    assert_eq!(warnings.len(), 1);
}

#[test]
fn legacy_style_keys_are_namespaced_only_when_they_name_a_style_property() {
    assert_eq!(
        namespace_style_key("--content-max-width"),
        "--folioh-content-max-width"
    );
    assert_eq!(
        namespace_style_key("--folioh-content-max-width"),
        "--folioh-content-max-width"
    );
    assert_eq!(namespace_style_key("--font-sans"), "--font-sans");
}

#[test]
fn contract_constants_match_the_schema() {
    assert_eq!(THEME_TUNE_KEYS.len(), 8);
    assert!(PROJECT_THEME_BASE_STYLE
        .iter()
        .all(|(k, _)| k.starts_with("--folioh-")));
    assert!(PROJECT_THEME_BASE_STYLE
        .iter()
        .any(|(k, _)| *k == "--folioh-card-shadow"));
    assert_eq!(
        PROJECT_THEME_BASE_STYLE.last().unwrap().0,
        "--folioh-workspace-shell-topbar-border"
    );
    assert_eq!(
        THEME_RADIUS_OPTIONS,
        ["0", "0.3rem", "0.5rem", "0.75rem", "1rem"]
    );
    assert_eq!(
        PROJECT_THEME_BASE_LIGHT
            .iter()
            .map(|(k, _)| *k)
            .collect::<Vec<_>>(),
        PROJECT_THEME_BASE_DARK
            .iter()
            .map(|(k, _)| *k)
            .collect::<Vec<_>>()
    );
    for (alias, target) in folioh_config::THEME_TUNE_ALIASES {
        assert!(THEME_TUNE_KEYS.contains(&target), "{alias} -> {target}");
    }
}

#[test]
fn codegen_emits_style_props_tune_keys_and_the_radius_scale() {
    let out = generate_typescript_contract();
    assert!(out.starts_with("// GENERATED FILE - DO NOT EDIT\n"));
    assert!(out.contains("  \"--folioh-card-shadow\"?: string\n"));
    assert!(out.contains("export type ThemeTuneKey =\n  | \"borderId\"\n  | \"codeTreatmentId\"\n"));
    assert!(out.contains("export const themeRadiusScale = [\"0\", \"0.3rem\", \"0.5rem\", \"0.75rem\", \"1rem\"] as const\n"));
    assert!(out.ends_with("as const\n") && !out.ends_with("\n\n"));
}

#[test]
fn codegen_matches_the_committed_template_file() {
    let committed = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../template/theme/theme-contract.generated.ts"
    ))
    .unwrap();
    assert_eq!(generate_typescript_contract(), committed);
}

fn package(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (rel, text) in files {
        let path = dir.path().join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        if rel.ends_with('/') {
            std::fs::create_dir_all(&path).unwrap();
        } else {
            std::fs::write(&path, text).unwrap();
        }
    }
    dir
}

#[test]
fn theme_package_validator_cases() {
    let missing = tempfile::tempdir().unwrap();
    let errors = validate_theme_package(&missing.path().join("nope"), false);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("not found"));
    let file = missing.path().join("file.txt");
    std::fs::write(&file, "x").unwrap();
    assert!(validate_theme_package(&file, false)[0].contains("is not a directory"));

    let reserved = package(&[
        ("content/", ""),
        (
            "theme/theme-contract.generated.ts",
            "export const stale = 1\n",
        ),
    ]);
    let errors = validate_theme_package(reserved.path(), false);
    assert_eq!(
            errors,
            vec![
                "Theme package must not contain reserved path 'content/'. Folioh generates this at build time.",
                "Theme package must not contain reserved file 'theme/theme-contract.generated.ts'. This is a Folioh internal file.",
            ]
        );
    assert_eq!(
        validate_and_raise(reserved.path(), false)
            .unwrap_err()
            .to_string(),
        format!(
            "Theme package validation failed:\n  - {}\n  - {}",
            errors[0], errors[1]
        )
    );

    let ok = package(&[("app/", "")]);
    assert!(validate_theme_package(ok.path(), false).is_empty());

    let cases: [(&str, bool); 5] = [
        (
            "export const projectThemePreset = 1\nexport const projectThemeDefaultConfig = {}\n",
            true,
        ),
        (
            "export { foo as projectThemePreset }\nexport { bar as projectThemeDefaultConfig }\n",
            true,
        ),
        ("export const nope = 1\n", false),
        (
            "export const projectThemePresetX = 1\nexport const projectThemeDefaultConfig = {}\n",
            false,
        ),
        (
            "export { projectThemePreset as foo }\nexport const projectThemeDefaultConfig = {}\n",
            false,
        ),
    ];
    for (text, valid) in cases {
        let dir = package(&[("theme/project-theme.ts", text)]);
        let errors = validate_theme_package(dir.path(), false);
        if valid {
            assert!(errors.is_empty(), "{text:?}: {errors:?}");
        } else {
            assert!(
                errors
                    .iter()
                    .any(|e| e.contains("must export 'projectThemePreset'")),
                "{text:?}: {errors:?}"
            );
        }
    }
}

#[test]
fn provenance_decides_who_may_own_the_next_config() {
    // The three manifest files are reserved for everyone: the frontend build
    // installs against Folioh's own pinned lockfile.
    for name in ["package.json", "pnpm-lock.yaml", "pnpm-workspace.yaml"] {
        let dir = package(&[(name, "{}\n")]);
        let errors = validate_theme_package(dir.path(), false);
        assert_eq!(errors.len(), 1, "{name}: {errors:?}");
        assert!(errors[0].contains(name), "{errors:?}");
    }

    // next.config.mjs is the project's to own, and not a stranger's.
    let dir = package(&[("next.config.mjs", "export default {}\n")]);
    assert!(
        validate_theme_package(dir.path(), false).is_empty(),
        "a local package may own the Next config"
    );
    let errors = validate_theme_package(dir.path(), true);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("next.config.mjs"), "{errors:?}");
}

fn template_text(rel: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../template")
            .join(rel),
    )
    .unwrap()
}

/// The `id: "…"` values between `start` and the next `end` in `text`.
fn ids_between(text: &str, start: &str, end: &str) -> Vec<String> {
    let from = text
        .find(start)
        .unwrap_or_else(|| panic!("{start} missing"));
    let block = &text[from..];
    let block = &block[..block.find(end).unwrap()];
    crate::re(r#"\bid: "([^"]+)""#)
        .captures_iter(block)
        .map(|caps| caps[1].to_string())
        .collect()
}

#[test]
fn tune_options_match_the_bundled_configurator() {
    let configurator = template_text("theme/theme-config.ts");
    for (key, block) in [
        ("fontId", "fontOptions"),
        ("colorId", "colorOptions"),
        ("surfaceColorId", "surfaceColorOptions"),
        ("shellPaddingId", "shellPaddingOptions"),
        ("contentWidthId", "contentWidthOptions"),
        ("rhythmId", "rhythmOptions"),
        ("borderId", "borderOptions"),
        ("codeTreatmentId", "codeTreatmentOptions"),
    ] {
        let options = folioh_config::THEME_TUNE_OPTIONS
            .iter()
            .find(|(id, _)| *id == key)
            .unwrap()
            .1;
        assert_eq!(
            ids_between(&configurator, &format!("const {block} = ["), "] satisfies"),
            options,
            "{key}"
        );
    }
}

#[test]
fn preset_tables_match_the_bundled_template() {
    let presets = template_text("theme/presets.ts");
    let mut builtins: Vec<String> = crate::re(r#"(?m)^  id: "([^"]+)""#)
        .captures_iter(&presets)
        .map(|caps| caps[1].to_string())
        .collect();
    builtins.sort();
    assert_eq!(builtins, BUILTIN_THEME_PRESETS);
    let configurator = template_text("theme/theme-config.ts");
    let from = configurator.find("const LEGACY_PRESET_IDS").unwrap();
    let block = &configurator[from..];
    let block = &block[..block.find("\n}\n").unwrap()];
    let mut legacy: Vec<String> = crate::re(r#"(?m)^  "([^"]+)":"#)
        .captures_iter(block)
        .map(|caps| caps[1].to_string())
        .collect();
    legacy.sort();
    assert_eq!(legacy, LEGACY_THEME_PRESETS);
}

#[test]
fn a_preset_must_be_one_the_configurator_shows() {
    let config = |yaml: &str| {
        let mapping: serde_yaml_ng::Mapping = serde_yaml_ng::from_str(yaml).unwrap();
        folioh_config::parse_docs_config_with(&mapping, Path::new("/proj"), "", &mut Vec::new())
            .unwrap()
    };
    for ok in [
        "project: {name: T}\n",
        "project: {name: T}\ntheme: {preset: beacon}\n",
        "project: {name: T}\ntheme: {preset: folioh}\n",
        "project: {name: T}\ntheme: {preset: acme, name: Acme}\n",
    ] {
        assert!(check_theme_preset(&config(ok), &[]).is_ok(), "{ok}");
    }
    let typo = config("project: {name: T}\ntheme: {preset: atlass}\n");
    let err = check_theme_preset(&typo, &[]).unwrap_err().to_string();
    assert!(
        err.starts_with("theme.preset must be one of 'aperture', 'atlas',")
            && err.contains("; got 'atlass' (did you mean 'atlas'?). A new preset id"),
        "{err}"
    );
    let pack = config("project: {name: T}\ntheme: {preset: acme}\n");
    assert!(check_theme_preset(&pack, &[]).is_err());
    assert!(check_theme_preset(&pack, &["acme".to_string()]).is_ok());

    let dir = package(&[
        (
            "theme/project-theme.ts",
            "export const projectThemePreset = { id: 'acme', name: \"Acme\" }\n",
        ),
        ("theme/extra.tsx", "registerPreset({ id: \"acme-dark\" })\n"),
        (
            "theme/controls.ts",
            "controls: [{\n  id: \"palette\",\n  label: \"Palette\",\n}]\n",
        ),
        ("components/other.tsx", "{ id: \"not-a-preset\" }\n"),
    ]);
    assert_eq!(declared_preset_ids(dir.path()), ["acme", "acme-dark"]);
    // The bundled presets.ts, copied into an overlay, adds no control ids and
    // lists no builtin twice.
    let overlay = package(&[("theme/presets.ts", &template_text("theme/presets.ts"))]);
    let declared = declared_preset_ids(overlay.path());
    let mut builtins = BUILTIN_THEME_PRESETS.to_vec();
    builtins.sort_unstable();
    assert_eq!(declared, builtins);
    let err = check_theme_preset(
        &config("project: {name: T}\ntheme: {preset: x}\n"),
        &declared,
    )
    .unwrap_err()
    .to_string();
    assert_eq!(err.matches("'atlas'").count(), 1, "{err}");
}

#[test]
fn a_variant_control_without_style_is_a_colour_control() {
    let yaml = r#"
project: {name: T}
theme:
  preset: acme
  name: Acme
  variants:
    palette:
      label: Palette
      default: day
      options:
        day: {label: Day, swatch: "oklch(0.98 0 0)"}
        night:
          label: Night
          tokens: {dark: {--background: "oklch(0.16 0 0)"}}
    density:
      label: Density
      default: cozy
      options:
        cozy: {label: Cozy}
        tight:
          label: Tight
          style: {--folioh-body-line-height: "1.4"}
"#;
    let mapping: serde_yaml_ng::Mapping = serde_yaml_ng::from_str(yaml).unwrap();
    let config =
        folioh_config::parse_docs_config_with(&mapping, Path::new("/proj"), "", &mut Vec::new())
            .unwrap();
    let controls = controls(&config);
    assert_eq!(controls[0]["id"], "palette");
    assert_eq!(controls[0]["kind"], "color");
    assert_eq!(controls[1]["id"], "density");
    assert!(controls[1].get("kind").is_none(), "{}", controls[1]);
}

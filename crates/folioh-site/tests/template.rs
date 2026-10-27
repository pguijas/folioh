//! Template resolution, validation, overlay staging, prune and the workspace copy.

mod common;

use std::path::Path;

use folioh_site::template::{
    check_bundled_template_drift, materialize_overlay_template, prune_stale_build_overlay,
    remove_relocated_docs_route, resolve_template_dir, validate_template_contract, BuildContext,
    TemplateKind, TemplateWorkspace,
};

fn ctx(template: &str, theme: &str, base: &str) -> BuildContext {
    BuildContext {
        config: "c".into(),
        template: template.into(),
        theme_package: theme.into(),
        docs_route_base: base.into(),
        generator: "g".into(),
        source_ref: "main".into(),
        experimental_features: "disabled".into(),
    }
}

#[test]
fn contract_validation_reports_files_then_components_then_markers() {
    let dir = tempfile::tempdir().unwrap();
    let template = dir.path().join("custom");
    std::fs::create_dir_all(&template).unwrap();
    assert_eq!(
        validate_template_contract(&template, "template.path").unwrap_err().to_string(),
        "template.path is missing required Next/Nextra files: package.json, pnpm-lock.yaml, next.config.mjs, mdx-components.tsx, app, app/docs/layout.tsx, app/docs/[[...mdxPath]]/page.jsx"
    );
    for rel in [
        "package.json",
        "pnpm-lock.yaml",
        "next.config.mjs",
        "app/docs/layout.tsx",
        "app/docs/[[...mdxPath]]/page.jsx",
    ] {
        common::write(&template, rel, "");
    }
    common::write(&template, "mdx-components.tsx", "export function useMDXComponents() {\n  return {\n    ParamTable,\n    ...components,\n  }\n}\n");
    assert_eq!(
        validate_template_contract(&template, "template.overlay_path").unwrap_err().to_string(),
        "template.overlay_path mdx-components.tsx is missing Folioh MDX contract components: ClassOverview, Callout, SourceLink, Mermaid, Tabs, TabItem, ApiReferenceIndex"
    );
    common::write(&template, "mdx-components.tsx", "export function useMDXComponents() {\n  return {\n    ParamTable,\n    ClassOverview,\n    Callout,\n    SourceLink,\n    Mermaid,\n    Tabs,\n    TabItem,\n    ApiReferenceIndex,\n  }\n}\n");
    let err = validate_template_contract(&template, "template.path")
        .unwrap_err()
        .to_string();
    assert!(err.starts_with("template.path is missing required Folioh injection markers: __PROJECT_NAME__ in app/layout.tsx, __PROJECT_DESCRIPTION__ in app/layout.tsx, __SITE_URL__ in app/layout.tsx, __PROJECT_NAME__ in app/docs/layout.tsx, "), "{err}");
    assert!(
        err.ends_with("const configuredBasePath = '' // __FOLIOH_BASE_PATH__ in next.config.mjs"),
        "{err}"
    );
    assert!(validate_template_contract(&common::bundled_template(), "template.path").is_ok());
}

#[test]
fn drift_gate_reports_both_directions() {
    let bundled = common::bundled_template();
    assert!(check_bundled_template_drift(&bundled).is_ok());
    let dir = tempfile::tempdir().unwrap();
    let text = common::read(&bundled.join("mdx-components.tsx"));
    common::write(
        dir.path(),
        "mdx-components.tsx",
        &text
            .replace("    ...components,", "    RogueWidget,\n    ...components,")
            .replace("    ParamTable,\n", ""),
    );
    let err = check_bundled_template_drift(dir.path())
        .unwrap_err()
        .to_string();
    assert!(err.starts_with("Bundled template drifted from the builtin component manifest:\n  - builtin component 'ParamTable' is declared in the manifest (folioh-plugins/src/builtins.rs) but has no entry in mdx-components.tsx\n"), "{err}");
    assert!(err.contains("  - component entry 'RogueWidget' in mdx-components.tsx is not declared in the builtin manifest (folioh-plugins/src/builtins.rs)"));
}

#[test]
fn resolve_template_dir_guards_custom_paths_and_returns_the_bundled_copy() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path();
    let build_dir = project.join(".build");
    std::fs::create_dir_all(project.join(".build/docs-template")).unwrap();
    std::fs::create_dir_all(project.join("_site/docs-template")).unwrap();
    let cases = [
        (
            ".build/docs-template",
            "template.path cannot point inside the .build directory",
        ),
        (
            "_site/docs-template",
            "template.path cannot point inside the output directory",
        ),
    ];
    for (path, expected) in cases {
        let config = common::config(
            project,
            &format!("template:\n  path: {path}\noutput: _site\n"),
        );
        assert_eq!(
            resolve_template_dir(&config, &build_dir)
                .unwrap_err()
                .to_string(),
            expected
        );
    }
    let config = common::config(
        project,
        "template:\n  path: does-not-exist\noutput: _site\n",
    );
    let err = resolve_template_dir(&config, &build_dir)
        .unwrap_err()
        .to_string();
    assert!(
        err.starts_with("template.path does not exist: ") && err.ends_with("does-not-exist"),
        "{err}"
    );

    let custom = project.join("docs-template");
    folioh_site::fs::copy_tree(&common::bundled_template(), &custom, &[]).unwrap();
    std::fs::copy(
        common::bundled_template().join("next.config.mjs"),
        custom.join("next.config.mjs"),
    )
    .unwrap();
    let config = common::config(project, "template:\n  path: docs-template\noutput: _site\n");
    let resolved = resolve_template_dir(&config, &build_dir).unwrap();
    assert_eq!(resolved.kind, TemplateKind::Custom);
    assert_eq!(resolved.dir, custom.canonicalize().unwrap());

    let config = common::config(project, "output: _site\n");
    let resolved = resolve_template_dir(&config, &build_dir).unwrap();
    assert_eq!(resolved.kind, TemplateKind::Bundled);
    assert!(resolved.dir.join("mdx-components.tsx").is_file());
}

#[test]
fn overlay_staging_merges_user_files_over_the_bundled_template() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path();
    let build_dir = project.join(".build");
    let staging = project.join(".build-template");
    common::write(
        project,
        "overlay/components/callout.tsx",
        "// FOLIOH_OVERLAY_SENTINEL custom Callout\nexport function Callout() { return null }\n",
    );
    let config = common::config(
        project,
        "template:\n  overlay_path: overlay\noutput: _site\n",
    );
    let resolved = resolve_template_dir(&config, &build_dir).unwrap();
    assert_eq!(
        resolved,
        folioh_site::template::ResolvedTemplate {
            dir: staging.clone(),
            kind: TemplateKind::Overlay
        }
    );
    assert!(
        common::read(&staging.join("components/callout.tsx")).contains("FOLIOH_OVERLAY_SENTINEL")
    );
    assert_eq!(
        common::read(&staging.join("components/param-table.tsx")),
        common::read(&common::bundled_template().join("components/param-table.tsx"))
    );
    assert!(staging.join(".folioh-staging").is_file());
    assert!(
        staging.join("next.config.mjs").is_file(),
        "the overlay staging keeps next.config.mjs"
    );

    // A marked staging dir is recreated; an unmarked one is user data.
    std::fs::write(staging.join("leftover.txt"), "x").unwrap();
    resolve_template_dir(&config, &build_dir).unwrap();
    assert!(!staging.join("leftover.txt").exists());
    std::fs::remove_file(staging.join(".folioh-staging")).unwrap();
    std::fs::write(staging.join("precious.txt"), "user data").unwrap();
    let err = resolve_template_dir(&config, &build_dir)
        .unwrap_err()
        .to_string();
    assert!(err.starts_with(&format!("Refusing to delete existing directory {}: it was not created by Folioh (missing .folioh-staging marker).", staging.display())), "{err}");
    assert_eq!(common::read(&staging.join("precious.txt")), "user data");
    std::fs::remove_dir_all(&staging).unwrap();

    let outside = project.parent().unwrap().join(format!(
        "{}-overlay",
        project.file_name().unwrap().to_string_lossy()
    ));
    common::write(&outside, "components/callout.tsx", "x");
    let config = common::config(
        project,
        &format!(
            "template:\n  overlay_path: {}\noutput: _site\n",
            outside.display()
        ),
    );
    let err = resolve_template_dir(&config, &build_dir)
        .unwrap_err()
        .to_string();
    std::fs::remove_dir_all(&outside).unwrap();
    assert_eq!(
        err,
        "template.overlay_path must stay within the project directory"
    );

    common::write(
        project,
        ".build-template/overlay/components/callout.tsx",
        "x",
    );
    let config = common::config(
        project,
        "template:\n  overlay_path: .build-template/overlay\noutput: _site\n",
    );
    assert_eq!(
        resolve_template_dir(&config, &build_dir).unwrap_err().to_string(),
        "template.overlay_path cannot point inside the template staging directory (.build-template/): it is recreated on every build"
    );
    assert!(project
        .join(".build-template/overlay/components/callout.tsx")
        .exists());
}

#[cfg(unix)]
#[test]
fn overlay_symlinks_are_rejected_except_under_ignored_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path();
    std::fs::write(project.join("secret.txt"), "top secret").unwrap();
    common::write(
        project,
        "overlay/components/callout.tsx",
        "export function Callout() { return null }\n",
    );
    std::fs::create_dir_all(project.join("overlay/content")).unwrap();
    std::os::unix::fs::symlink(
        project.join("secret.txt"),
        project.join("overlay/content/leak.txt"),
    )
    .unwrap();
    let bundled = project.join("bundled");
    common::write(&bundled, "bundled.txt", "bundled");
    let staging = project.join(".build-template");
    let merged =
        materialize_overlay_template(&bundled, &project.join("overlay"), &staging).unwrap();
    assert_eq!(merged, staging);
    assert!(!staging.join("content/leak.txt").exists());
    assert_eq!(common::read(&staging.join("bundled.txt")), "bundled");
    std::os::unix::fs::symlink(
        project.join("secret.txt"),
        project.join("overlay/components/leak.txt"),
    )
    .unwrap();
    assert_eq!(
        materialize_overlay_template(&bundled, &project.join("overlay"), &staging)
            .unwrap_err()
            .to_string(),
        "template.overlay_path must not contain symlinks: components/leak.txt"
    );
}

#[test]
fn scoped_prune_and_relocated_route_removal() {
    let dir = tempfile::tempdir().unwrap();
    let build = dir.path().join(".build");
    for rel in [
        "node_modules/pkg/index.js",
        ".next/cache/data.json",
        "content/index.mdx",
        "lib/folioh-template.ts",
        "app/reference/docs/layout.tsx",
        "stray.txt",
    ] {
        common::write(&build, rel, "x");
    }
    prune_stale_build_overlay(None, &build, &ctx("t2", "", "/docs")).unwrap();
    assert!(build.join("lib/folioh-template.ts").exists());
    prune_stale_build_overlay(
        Some(&ctx("t1", "", "/docs")),
        &build,
        &ctx("t2", "", "/docs"),
    )
    .unwrap();
    assert!(
        build.join("node_modules/pkg/index.js").exists()
            && build.join(".next/cache/data.json").exists()
            && build.join("content/index.mdx").exists()
    );
    assert!(
        !build.join("lib").exists()
            && !build.join("app").exists()
            && !build.join("stray.txt").exists()
    );

    common::write(&build, "app/reference/docs/layout.tsx", "x");
    common::write(&build, "app/docs/layout.tsx", "x");
    prune_stale_build_overlay(
        Some(&ctx("t2", "", "/reference/docs")),
        &build,
        &ctx("t2", "", "/docs"),
    )
    .unwrap();
    assert!(!build.join("app/reference").exists());
    assert!(build.join("app/docs/layout.tsx").exists());
    remove_relocated_docs_route(&build, "/docs").unwrap();
    assert!(build.join("app/docs/layout.tsx").exists());
    common::write(&build, "app/a/b/layout.tsx", "x");
    common::write(&build, "app/a/keep.txt", "x");
    remove_relocated_docs_route(&build, "/a/b").unwrap();
    assert!(!build.join("app/a/b").exists() && build.join("app/a/keep.txt").exists());
    prune_stale_build_overlay(Some(&ctx("t2", "", "")), &build, &ctx("t2", "", "/docs")).unwrap();
}

#[test]
fn a_changed_executable_removes_previous_plugins_generated_files() {
    let dir = tempfile::tempdir().unwrap();
    let build = dir.path().join(".build");
    for rel in [
        "content/gallery/index.mdx",
        "public/_folioh/gallery/item.md",
        "components/gallery-grid.tsx",
        "node_modules/pkg/index.js",
    ] {
        common::write(&build, rel, "x");
    }
    let previous = ctx("same-template", "", "/docs");
    let mut current = previous.clone();
    current.generator = "docs-next".into();
    prune_stale_build_overlay(Some(&previous), &build, &current).unwrap();
    assert!(!build.join("content").exists());
    assert!(!build.join("public/_folioh/gallery").exists());
    assert!(!build.join("components/gallery-grid.tsx").exists());
    assert!(build.join("node_modules/pkg/index.js").exists());
}

#[test]
fn workspace_prepare_copies_keeps_and_cleans() {
    let dir = tempfile::tempdir().unwrap();
    let template = common::make_template(dir.path());
    common::write(&template, "content/_meta.json", "{\"guide\": \"Guide\"}");
    common::write(&template, "content/guide/index.mdx", "# Getting Started");
    let build = dir.path().join("build");
    let content = build.join("content");
    common::write(&content, "_meta.json", "{\"guide\": \"Guide\"}");
    common::write(&content, "guide/index.mdx", "# Getting Started");
    common::write(&content, "index.mdx", "# Generated Overview");
    let workspace = TemplateWorkspace::new(&template, &build, None);
    workspace.prepare(false).unwrap();
    assert!(
        build.join("package.json").exists()
            && build.join("app/layout.tsx").exists()
            && content.is_dir()
    );
    assert!(
        !build.join("next.config.mjs").exists(),
        "raw next.config.mjs never lands in .build"
    );
    assert!(content.join("index.mdx").exists());
    assert!(!content.join("_meta.json").exists() && !content.join("guide/index.mdx").exists());

    common::write(
        &build,
        "node_modules/some-pkg/index.js",
        "module.exports = {}",
    );
    common::write(&build, ".next/cache/data.json", "{}");
    common::write(&content, "test.mdx", "old content");
    workspace.prepare(false).unwrap();
    assert!(
        build.join("node_modules/some-pkg/index.js").exists()
            && build.join(".next/cache/data.json").exists()
            && content.join("test.mdx").exists()
    );

    // A clean keeps the installed dependencies: `folioh clean` is the command
    // that removes them.
    workspace.prepare(true).unwrap();
    assert!(
        build.exists()
            && build.join("node_modules/some-pkg/index.js").exists()
            && !build.join(".next").exists()
            && content.is_dir()
    );
}

#[cfg(unix)]
#[test]
fn workspace_prepare_rejects_template_symlinks() {
    let dir = tempfile::tempdir().unwrap();
    let template = common::make_template(dir.path());
    std::os::unix::fs::symlink(Path::new("/etc/hosts"), template.join("app/hosts")).unwrap();
    let err = TemplateWorkspace::new(&template, &dir.path().join("build"), None)
        .prepare(false)
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        "template.path must not contain symlinks: app/hosts"
    );
}

#[test]
fn bundled_template_carries_injection_anchors() {
    let bundled = common::bundled_template();
    let text = |rel: &str| common::read(&bundled.join(rel));
    let next_config = text("next.config.mjs");
    for anchor in [
        "const configuredBasePath = '' // __FOLIOH_BASE_PATH__",
        "output: 'export'",
        "trailingSlash: true",
        "const isDevServer = process.env.NODE_ENV === 'development'",
        "const rawBasePath = isDevServer",
        "process.env.FOLIOH_BASE_PATH?.trim() ?? ''",
        ": process.env.FOLIOH_BASE_PATH?.trim() || configuredBasePath",
        "contentDirBasePath: '/docs'",
        "NEXT_PUBLIC_FOLIOH_BASE_PATH: basePath ?? \"\",",
        "images: { unoptimized: true }",
        "'next-mdx-import-source-file': './mdx-components.tsx'",
        "  __I18N_CONFIG__\n",
    ] {
        assert!(
            next_config.contains(anchor),
            "next.config.mjs lacks {anchor:?}"
        );
    }
    let docs_layout = text("app/docs/layout.tsx");
    for anchor in [
        "url: \"/docs/opengraph-image\"",
        "\"/docs/opengraph-image\"",
        "getPageMap(\"/docs\")",
        "footer={<Footer />}",
        "<ThemeGallery />",
        "darkMode={false}",
        "// __PROJECT_HEADER_ACTION_IMPORTS_START__",
        "{/* __PROJECT_HEADER_LOGO_START__ */}",
        "{/* __PROJECT_HEADER_ACTIONS_START__ */}",
        "{/* __PROJECT_REPO_LINK_START__ */}",
        "href=\"__PROJECT_REPO__\"",
        "<VersionSelector />",
    ] {
        assert!(
            docs_layout.contains(anchor),
            "app/docs/layout.tsx lacks {anchor:?}"
        );
    }
    assert!(!docs_layout.contains("<ThemeConfigurator />"));
    let page = text("app/docs/[[...mdxPath]]/page.jsx");
    for anchor in [
        "`${siteUrl}/docs/opengraph-image`",
        "\"/docs/opengraph-image\"",
        "docsIndexCanonicalPath === \"/\" ? \"/\" : \"/docs/\"",
        "`/docs/${mdxPath.join(\"/\")}/`",
        "\"__DOCS_INDEX_CANONICAL_PATH__\"",
        "[`${route}.md`, `${route}/index.md`]",
        "process.env.NEXT_PUBLIC_FOLIOH_BASE_PATH",
        "expandStaticParams(params)",
        "data-pagefind-ignore=\"all\"",
        "{ ...metadata, searchable: false }",
        "permanentRedirect(docsRouteForMdxPath([\"api-reference\"]))",
    ] {
        assert!(page.contains(anchor), "page.jsx lacks {anchor:?}");
    }
    let presets = text("theme/presets.ts");
    let builtins_at = presets
        .find("builtinPresets.forEach((preset) => registerPreset(preset))")
        .expect("builtins registered");
    let project_at = presets
        .find("registerPreset(projectThemePreset, \"project\")")
        .expect("project preset registered");
    assert!(
        builtins_at < project_at,
        "builtins register before the project preset (registry is last-wins)"
    );
    assert!(text("components/landing-navbar.tsx").contains("__PROJECT_NAME_JSON__"));
    assert!(text("theme/theme-config.ts")
        .contains("const configuredDefaultPresetId = \"pastel\" // __FOLIOH_THEME_PRESET__"));
    let selector = text("components/version-selector.tsx");
    assert!(selector.contains("__VERSIONS__") && selector.contains("__CURRENT_VERSION_PATH__"));
    let sitemap = text("app/sitemap.ts");
    for anchor in [
        "\"__SITE_URL__\"",
        "\"__INCLUDE_DOCS_INDEX__\"",
        "\"__DOCS_ROUTE_BASE__\"",
        "/_folioh/markdown/",
    ] {
        assert!(sitemap.contains(anchor), "sitemap.ts lacks {anchor:?}");
    }
    assert!(text("app/robots.ts").contains("\"__SITE_URL__\""));
    assert!(text("app/icon.svg").contains("__PROJECT_MONOGRAM__"));
    let package: serde_json::Value = serde_json::from_str(&text("package.json")).unwrap();
    assert_eq!(
        package["scripts"]["postbuild"],
        "pagefind --site out --output-subdir _pagefind --glob \"{index.html,docs/**/*.html}\""
    );
    // Authored paths are never reserved by experimental configuration gates.
    let params = text("lib/docs-route-params.js");
    let literal = regex::Regex::new(r"const DISABLED_DOC_STATIC_PATHS = (\[.*?\])\n")
        .unwrap()
        .captures(&params)
        .expect("gate literal")[1]
        .to_string();
    let js_paths: serde_json::Value = serde_json::from_str(&literal).unwrap();
    assert_eq!(js_paths, serde_json::json!([]));
    assert_eq!(folioh_config::DISABLED_FEATURES, ["i18n", "versions"]);
}

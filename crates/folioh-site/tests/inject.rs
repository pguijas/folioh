//! `docs.yaml` values injected into the copied template.

mod common;

use std::path::Path;

use folioh_plugins::{LandingPlugin, Plugin};
use folioh_site::inject::TemplateConfigInjector;
use folioh_site::template::TemplateWorkspace;
use serde_json::json;

/// Run the landing built-in over a raw `landing:` value, as the host does.
fn configure_landing(config: &mut folioh_config::DocsConfig, raw: serde_json::Value) {
    let mut map = serde_json::Map::new();
    map.insert("landing".into(), raw);
    LandingPlugin
        .configure(config, &map, &mut Vec::new())
        .unwrap();
}

fn read(build: &Path, rel: &str) -> String {
    common::read(&build.join(rel))
}

#[test]
fn root_docs_and_previews_layouts_take_project_identity_and_repo() {
    let dir = tempfile::tempdir().unwrap();
    let template = common::make_template(dir.path());

    let plain = common::config(dir.path(), "landing: true\n");
    let build = common::prepared(dir.path(), &plain, &template, "plain");
    let layout = read(&build, "app/layout.tsx");
    assert!(
        layout.contains("\"TestProject\"") && layout.contains("\"Documentation for TestProject\"")
    );
    assert!(!layout.contains("__PROJECT_NAME__") && !layout.contains("__PROJECT_DESCRIPTION__"));
    let previews = read(&build, "app/previews/layout.tsx");
    assert!(
        !previews.contains("aria-label=\"GitHub repository\"")
            && !previews.contains("__PROJECT_REPO__")
    );
    let docs = read(&build, "app/docs/layout.tsx");
    assert!(
        !docs.contains("aria-label=\"GitHub repository\"")
            && !docs.contains("__PROJECT_REPO")
            && !docs.contains("https://github.com")
    );
    assert!(
        docs.contains("import { SearchCommand } from \"@/components/search-command\"")
            && docs.contains("search={<SearchCommand />}")
            && !docs.contains("search={null}")
    );
    let page = read(&build, "app/page.tsx");
    assert!(
        page.contains("Built with {\"TestProject\"}") && !page.contains("__PROJECT_NAME__"),
        "{page}"
    );
    let navbar = read(&build, "components/landing-navbar.tsx");
    assert!(
        navbar.contains("TestProject") && navbar.contains("te") && !navbar.contains("__PROJECT_")
    );

    let repo = common::config(
        dir.path(),
        "project:\n  name: MyLib\n  repo: https://github.com/org/mylib\n",
    );
    let build = common::prepared(dir.path(), &repo, &template, "repo");
    let previews = read(&build, "app/previews/layout.tsx");
    assert!(
        previews.contains("MyLib")
            && previews.contains("my")
            && previews.contains("href={\"https://github.com/org/mylib\"}")
            && previews.contains("search={<SearchCommand")
    );
    assert!(
        !previews.contains("__PROJECT_NAME__")
            && !previews.contains("__PROJECT_MONOGRAM__")
            && !previews.contains("__PROJECT_REPO")
    );
    let docs = read(&build, "app/docs/layout.tsx");
    assert!(
        docs.contains("href={\"https://github.com/org/mylib\"}")
            && docs.contains("aria-label=\"GitHub repository\"")
    );
    assert!(!docs.contains("__PROJECT_REPO"));
    assert!(docs.contains("docsRepositoryBase={\"https://github.com/org/mylib\"}\n            editLink={null}\n            feedback={{ content: null }}\n            footer={<Footer />}"));
}

#[test]
fn feedback_is_opt_in_even_when_the_project_has_a_repository() {
    let dir = tempfile::tempdir().unwrap();
    let template = common::make_template(dir.path());
    for (label, yaml, enabled) in [
        ("default", "project:\n  name: T\n  repo: https://github.com/acme/docs\n", false),
        ("disabled", "project:\n  name: T\n  repo: https://github.com/acme/docs\nfeedback:\n  enabled: false\n", false),
        ("enabled", "project:\n  name: T\n  repo: https://github.com/acme/docs\nfeedback:\n  enabled: true\n", true),
        ("no-repo", "feedback:\n  enabled: true\n", false),
    ] {
        let config = common::config(dir.path(), yaml);
        let build = common::prepared(dir.path(), &config, &template, label);
        let docs = read(&build, "app/docs/layout.tsx");
        let expected = if enabled {
            "feedback={{ content: \"Feedback\", labels: \"feedback\" }}"
        } else {
            "feedback={{ content: null }}"
        };
        assert!(docs.contains(expected), "{label}: {docs}");
    }
    let config = common::config(dir.path(), "");
    let build = common::prepared(
        dir.path(),
        &config,
        &common::bundled_template(),
        "bundled-preview-index",
    );
    assert!(read(&build, "app/previews/layout.tsx").contains("feedback={{ content: null }}"));
}

#[test]
fn docs_layout_without_a_repo_hides_the_edit_and_feedback_links() {
    let dir = tempfile::tempdir().unwrap();
    let template = common::make_template(dir.path());
    let config = common::config(dir.path(), "project:\n  name: MyLib\n");
    let build = common::prepared(dir.path(), &config, &template, "no-repo");
    let docs = read(&build, "app/docs/layout.tsx");
    assert!(
        docs.contains("editLink={null}\n            feedback={{ content: null }}\n            footer={<Footer />}")
            && !docs.contains("docsRepositoryBase"),
        "{docs}"
    );
}

#[test]
fn search_slot_variants() {
    let dir = tempfile::tempdir().unwrap();
    let template = common::make_template(dir.path());
    let disabled = common::config(dir.path(), "search:\n  enabled: false\n");
    let build = common::prepared(dir.path(), &disabled, &template, "disabled");
    let docs = read(&build, "app/docs/layout.tsx");
    assert!(
        docs.contains("search={null}\n      pageMap={await getPageMap")
            && !docs.contains("SearchCommand")
    );
    let placeholder = common::config(dir.path(), "search:\n  placeholder: Find <something>...\n");
    let build = common::prepared(dir.path(), &placeholder, &template, "placeholder");
    let docs = read(&build, "app/docs/layout.tsx");
    assert!(docs.contains("import { SearchCommand } from \"@/components/search-command\""));
    assert!(
        docs.contains("<SearchCommand placeholder=\"Find &lt;something&gt;...\" />"),
        "{docs}"
    );
}

#[test]
fn optional_theme_route_pages_and_navbar_without_landing() {
    let dir = tempfile::tempdir().unwrap();
    let template = common::make_template(dir.path());
    common::write(
        &template,
        "app/product/page.tsx",
        "const monogram = __PROJECT_MONOGRAM_JSON__\nconst headline = __LANDING_HEADLINE_JSON__\nconst description = __LANDING_DESCRIPTION_JSON__\nconst primaryText = __LANDING_CTA_PRIMARY_TEXT_JSON__\nconst primaryLink = __LANDING_CTA_PRIMARY_LINK_JSON__\nconst secondaryText = __LANDING_CTA_SECONDARY_TEXT_JSON__\nconst secondaryLink = __LANDING_CTA_SECONDARY_LINK_JSON__\nconst commands = __LANDING_INSTALL_COMMANDS__\nconst sections = __LANDING_SECTIONS__\n",
    );
    let mut config = common::config(dir.path(), "landing: true\n");
    configure_landing(
        &mut config,
        json!({"hero": {"headline": "Docs from source"}, "sections": [{"type": "features", "title": "Complete"}]}),
    );
    let build = common::prepared(dir.path(), &config, &template, "theme-route");
    let page = read(&build, "app/product/page.tsx");
    assert!(
        page.contains("const monogram = \"te\"")
            && page.contains("const headline = \"Docs from source\"")
    );
    assert!(
        page.contains("const sections = [{\"type\": \"features\", \"title\": \"Complete\"}]"),
        "{page}"
    );
    assert!(
        page.contains("const primaryLink = \"/docs\"")
            && page.contains("const secondaryLink = null")
            && page.contains("const primaryText = \"Get Started\"")
    );
    assert!(!page.contains("__LANDING_") && !page.contains("__PROJECT_MONOGRAM_JSON__"));

    common::write(
        &template,
        "components/landing-navbar.tsx",
        "const name = __PROJECT_NAME_JSON__\nconst monogram = __PROJECT_MONOGRAM_JSON__\n",
    );
    let off = common::config(dir.path(), "");
    let build = common::prepared(dir.path(), &off, &template, "no-landing");
    let navbar = read(&build, "components/landing-navbar.tsx");
    assert!(
        !navbar.contains("__PROJECT_NAME_JSON__")
            && !navbar.contains("__PROJECT_MONOGRAM_JSON__")
            && navbar.contains("\"TestProject\"")
    );
}

#[test]
fn docs_route_base_relocates_the_route_and_rewrites_every_reference() {
    let dir = tempfile::tempdir().unwrap();
    let template = common::make_template(dir.path());
    common::write(
        &template,
        "app/docs/[[...mdxPath]]/page.jsx",
        "import { useMDXComponents as getMDXComponents } from \"../../../mdx-components\"\nconst docsOgImageUrl = siteUrl ? `${siteUrl}/docs/opengraph-image` : \"/docs/opengraph-image\"\nconst docsIndexCanonicalPath = \"__DOCS_INDEX_CANONICAL_PATH__\"\nfunction docsRouteForMdxPath(mdxPath) {\n  if (!mdxPath.length) {\n    return docsIndexCanonicalPath === \"/\" ? \"/\" : \"/docs/\"\n  }\n  return `/docs/${mdxPath.join(\"/\")}/`\n}\n",
    );
    common::write(
        &template,
        "app/docs/opengraph-image.tsx",
        "export const alt = '__PROJECT_NAME__ documentation'\n",
    );
    common::write(
        &template,
        "next.config.mjs",
        "const configuredBasePath = '' // __FOLIOH_BASE_PATH__\nconst withNextra = nextra({\n  contentDirBasePath: '/docs',\n})\nconst nextConfig = {\n  env: {\n    NEXT_PUBLIC_FOLIOH_BASE_PATH: basePath ?? \"\",\n  },\n  __I18N_CONFIG__\n}\n",
    );
    let config = common::config(dir.path(), "project:\n  name: RouteDocs\n  url: https://example.com\ntemplate:\n  docs_route_base: /reference/docs\n");
    let build = common::prepared(dir.path(), &config, &template, "route");
    assert!(!build.join("app/docs").exists());
    assert!(build.join("app/reference/docs/layout.tsx").exists());
    let page = read(&build, "app/reference/docs/[[...mdxPath]]/page.jsx");
    assert!(read(&build, "app/previews/layout.tsx").contains("getPageMap(\"/reference/docs\")"));
    assert!(
        read(&build, "app/reference/docs/layout.tsx").contains("getPageMap(\"/reference/docs\")")
    );
    let next_config = read(&build, "next.config.mjs");
    assert!(next_config.contains("contentDirBasePath: \"/reference/docs\""));
    assert!(next_config.contains("NEXT_PUBLIC_FOLIOH_BASE_PATH: basePath ?? \"\",\n    NEXT_PUBLIC_FOLIOH_DOCS_ROUTE_BASE: \"/reference/docs\","));
    assert!(page.contains("/reference/docs/opengraph-image"));
    assert!(page.contains("return docsIndexCanonicalPath === \"/\" ? \"/\" : \"/reference/docs/\""));
    assert!(
        page.contains("return `/reference/docs/${mdxPath.join(\"/\")}/`")
            && page.contains("from \"@/mdx-components\"")
    );
    assert!(page.contains("const docsIndexCanonicalPath = \"/\""));
    assert!(
        read(&build, "lib/folioh-template.ts").contains("\"docsRouteBase\": \"/reference/docs\"")
    );

    // A nested base relocates inside app/docs and a warm rebuild does not nest again.
    let nested = common::config(dir.path(), "template:\n  docs_route_base: /docs/v2\n");
    let build = dir.path().join("build-nested");
    common::prepare(&nested, &template, &build);
    common::prepare(&nested, &template, &build);
    assert!(build.join("app/docs/v2/[[...mdxPath]]/page.jsx").exists());
    assert!(!build.join("app/docs/v2/v2").exists());
    assert!(!build.join("app/docs/layout.tsx").exists());
}

#[test]
fn og_images_favicon_metadata_sitemap_and_postbuild() {
    let dir = tempfile::tempdir().unwrap();
    let template = common::make_template(dir.path());
    common::write(&template, "app/opengraph-image.tsx", "export default function OGImage() {\n  return <div>__PROJECT_NAME__ __PROJECT_MONOGRAM__ __PROJECT_DESCRIPTION__</div>\n}\n");
    common::write(&template, "app/docs/opengraph-image.tsx", "export default function OGImage() {\n  return <div>__PROJECT_NAME__ __PROJECT_MONOGRAM__</div>\n}\n");
    common::write(
        &template,
        "app/icon.svg",
        "<svg><text>__PROJECT_MONOGRAM__</text></svg>",
    );
    common::write(&template, "app/docs/[[...mdxPath]]/page.jsx", "const configuredSiteUrl = \"__SITE_URL__\"\nconst projectName = \"__PROJECT_NAME__\"\nconst projectDescription = \"__PROJECT_DESCRIPTION__\"\n");
    common::write(
        &template,
        "app/sitemap.ts",
        "const SITE_URL = \"__SITE_URL__\"\n",
    );
    common::write(
        &template,
        "app/robots.ts",
        "const SITE_URL = \"__SITE_URL__\"\n",
    );
    common::write(&template, "package.json", "{\"name\": \"test\", \"scripts\": {\"build\": \"next build\", \"postbuild\": \"pagefind --site out\"}}");

    let config = common::config(
        dir.path(),
        "project:\n  name: MyLib\n  url: https://example.com/docs\nsearch:\n  enabled: false\n",
    );
    let build = common::prepared(dir.path(), &config, &template, "og");
    let root_og = read(&build, "app/opengraph-image.tsx");
    let docs_og = read(&build, "app/docs/opengraph-image.tsx");
    for content in [&root_og, &docs_og] {
        assert!(
            content.contains("MyLib") && content.contains("my") && !content.contains("__PROJECT_")
        );
    }
    assert!(root_og.contains("Documentation for MyLib"));
    assert_eq!(read(&build, "app/icon.svg"), "<svg><text>my</text></svg>");
    assert!(read(&build, "app/layout.tsx")
        .contains("metadataBase: new URL(\"https://example.com/docs\")"));
    let page = read(&build, "app/docs/[[...mdxPath]]/page.jsx");
    assert!(
        page.contains("const configuredSiteUrl = \"https://example.com/docs\"")
            && page.contains("const projectName = \"MyLib\"")
            && page.contains("const projectDescription = \"Documentation for MyLib\"")
    );
    assert!(
        read(&build, "app/sitemap.ts").contains("const SITE_URL = \"https://example.com/docs\"")
    );
    assert!(read(&build, "app/robots.ts").contains("const SITE_URL = \"https://example.com/docs\""));
    let package: serde_json::Value = serde_json::from_str(&read(&build, "package.json")).unwrap();
    assert_eq!(package["scripts"], json!({"build": "next build"}));
    assert!(read(&build, "package.json").ends_with("}\n"));

    std::fs::write(dir.path().join("favicon.ico"), b"icon-bytes").unwrap();
    let favicon = common::config(
        dir.path(),
        "project:\n  name: MyLib\ntheme:\n  favicon: favicon.ico\n",
    );
    let build = common::prepared(dir.path(), &favicon, &template, "favicon");
    assert_eq!(
        std::fs::read(build.join("app/icon.ico")).unwrap(),
        b"icon-bytes"
    );
    assert!(!build.join("app/icon.svg").exists());
}

#[test]
fn generated_favicon_follows_the_preset_without_owning_custom_icons() {
    let dir = tempfile::tempdir().unwrap();
    let template = common::make_template(dir.path());
    common::write(
        &template,
        "app/icon.svg",
        &common::read(&common::bundled_template().join("app/icon.svg")),
    );
    common::write(
        &template,
        "theme/project-icon.ts",
        "export const projectIconSvg: string | null = null\n",
    );
    let project_icon = |build: &Path| -> Option<String> {
        serde_json::from_str(
            read(build, "theme/project-icon.ts")
                .trim()
                .strip_prefix("export const projectIconSvg: string | null = ")
                .unwrap(),
        )
        .unwrap()
    };

    for (preset, shape) in [("pastel", "pastel"), ("atlas", "default")] {
        let config = common::config(
            dir.path(),
            &format!("project:\n  name: Folioh\ntheme:\n  preset: {preset}\n"),
        );
        let build = common::prepared(dir.path(), &config, &template, preset);
        let svg = read(&build, "app/icon.svg");
        assert!(svg.contains(&format!("data-folioh-icon=\"{shape}\"")));
        assert!(svg.contains(">fo</text>"));
        assert_eq!(project_icon(&build), Some(svg));
    }

    for (ext, bytes) in [
        ("ico", "custom icon bytes"),
        (
            "svg",
            "<svg data-folioh-icon=\"default\"><text>custom</text></svg>",
        ),
    ] {
        common::write(dir.path(), &format!("custom.{ext}"), bytes);
        let config = common::config(dir.path(), &format!("theme:\n  favicon: custom.{ext}\n"));
        let build = common::prepared(dir.path(), &config, &template, ext);
        assert_eq!(read(&build, &format!("app/icon.{ext}")), bytes);
        assert_eq!(project_icon(&build), None);
    }

    let custom_svg = "<svg><style>[data-folioh-icon=\"pastel\"] text { fill: blue }</style><path d=\"M0 0h32v32H0Z\"/></svg>";
    common::write(&dir.path().join("theme"), "app/icon.svg", custom_svg);
    let config = common::config(dir.path(), "theme:\n  package: theme\n");
    let build = common::prepared(dir.path(), &config, &template, "package-icon");
    assert_eq!(read(&build, "app/icon.svg"), custom_svg);
    assert_eq!(project_icon(&build), None);

    common::write(&template, "app/icon.svg", custom_svg);
    let config = common::config(dir.path(), "");
    let build = common::prepared(dir.path(), &config, &template, "template-icon");
    assert_eq!(read(&build, "app/icon.svg"), custom_svg);
    assert_eq!(project_icon(&build), None);
}

#[test]
fn next_config_i18n_base_path_and_versions() {
    let dir = tempfile::tempdir().unwrap();
    let template = common::make_template(dir.path());
    common::write(
        &template,
        "components/version-selector.tsx",
        "const versions = __VERSIONS__\nconst current = __CURRENT_VERSION_PATH__\n",
    );
    common::write(&template, "components/theme-configurator.tsx", "const configuredDefaultPresetId = \"organic-editorial\" // __FOLIOH_THEME_PRESET__\nconst DEFAULT_CONFIG = { presetId: configuredDefaultPresetId }\n");

    let plain = common::config(
        dir.path(),
        "project:\n  name: T\n  url: https://example.com/docs/v1/\ntheme:\n  preset: beacon\n",
    );
    let build = common::prepared(dir.path(), &plain, &template, "plain");
    let next_config = read(&build, "next.config.mjs");
    assert!(
        !next_config.contains("__I18N_CONFIG__")
            && !next_config.contains("__FOLIOH_BASE_PATH__")
            && !next_config.contains("i18n:")
    );
    assert!(next_config.contains("const configuredBasePath = \"\""));
    assert_eq!(
        read(&build, "components/version-selector.tsx"),
        "const versions = []\nconst current = \"\"\n"
    );
    let configurator = read(&build, "components/theme-configurator.tsx");
    assert!(
        configurator.contains("const configuredDefaultPresetId = \"beacon\"")
            && !configurator.contains("__FOLIOH_THEME_PRESET__")
    );

    // New bundled defaults and older overlays both retain the injection contract.
    common::write(
        &template,
        "components/theme-configurator.tsx",
        "const configuredDefaultPresetId = \"pastel\" // __FOLIOH_THEME_PRESET__\n",
    );
    let current = common::prepared(dir.path(), &plain, &template, "current-preset-marker");
    assert_eq!(
        read(&current, "components/theme-configurator.tsx"),
        "const configuredDefaultPresetId = \"beacon\"\n"
    );

    // The shared model receives the same project preset while the client
    // wrapper keeps its stable import surface.
    common::write(
        &template,
        "components/theme-configurator.tsx",
        "export * from '@/theme/theme-config'\n",
    );
    common::write(
        &template,
        "theme/theme-config.ts",
        "const configuredDefaultPresetId = \"pastel\" // __FOLIOH_THEME_PRESET__\n",
    );
    let shared = common::prepared(dir.path(), &plain, &template, "shared-preset-marker");
    assert_eq!(
        read(&shared, "theme/theme-config.ts"),
        "const configuredDefaultPresetId = \"beacon\"\n"
    );

    let deploy = common::config(dir.path(), "deploy:\n  base_path: /published/docs\n");
    let build = common::prepared(dir.path(), &deploy, &template, "deploy");
    assert!(
        read(&build, "next.config.mjs").contains("const configuredBasePath = \"/published/docs\"")
    );

    let mapping: serde_yaml_ng::Mapping = serde_yaml_ng::from_str("project:\n  name: T\ni18n:\n  default_locale: en\n  locales:\n    - {code: en, name: English}\n    - {code: es, name: Espanol}\nversions:\n  - {label: \"v0.2.1 (latest)\", path: latest}\n  - {label: v0.1.0, path: v0.1, default_path: docs/}\n").unwrap();
    let experimental = folioh_config::parse_docs_config_with(
        &mapping,
        dir.path(),
        "i18n,versions",
        &mut Vec::new(),
    )
    .unwrap()
    .resolve_paths(dir.path())
    .unwrap();
    let build = dir.path().join("build-i18n");
    TemplateWorkspace::new(&template, &build, None)
        .prepare(false)
        .unwrap();
    let mut injector = TemplateConfigInjector::new(&experimental, &build, &template);
    injector.current_version_path = "v0.1".to_string();
    injector.inject().unwrap();
    let next_config = read(&build, "next.config.mjs");
    assert!(
        next_config
            .contains("  i18n: {\n    locales: ['en', 'es'],\n    defaultLocale: 'en',\n  },\n"),
        "{next_config}"
    );
    let selector = read(&build, "components/version-selector.tsx");
    assert!(
        selector.contains("\"label\": \"v0.2.1 (latest)\", \"path\": \"latest\"")
            && selector
                .contains("\"label\": \"v0.1.0\", \"path\": \"v0.1\", \"defaultPath\": \"docs/\"")
    );
    assert!(selector.contains("const current = \"v0.1\"") && !selector.contains("__VERSIONS__"));
}

#[test]
fn warm_prepare_writes_next_config_once_and_never_raw() {
    let dir = tempfile::tempdir().unwrap();
    let template = common::make_template(dir.path());
    let build = dir.path().join("build");
    let plain = common::config(dir.path(), "");
    common::prepare(&plain, &template, &build);
    let config_path = build.join("next.config.mjs");
    let before = std::fs::metadata(&config_path).unwrap();
    let again = common::prepare(&plain, &template, &build);
    assert!(
        !again
            .injected_files
            .contains(&"next.config.mjs".to_string()),
        "{:?}",
        again.injected_files
    );
    let after = std::fs::metadata(&config_path).unwrap();
    assert_eq!(before.modified().unwrap(), after.modified().unwrap());
    assert!(!read(&build, "next.config.mjs").contains("__I18N_CONFIG__"));

    let mapping: serde_yaml_ng::Mapping = serde_yaml_ng::from_str("project:\n  name: TestProject\ni18n:\n  default_locale: en\n  locales:\n    - {code: en, name: English}\n").unwrap();
    let localized =
        folioh_config::parse_docs_config_with(&mapping, dir.path(), "i18n", &mut Vec::new())
            .unwrap()
            .resolve_paths(dir.path())
            .unwrap();
    let changed = common::prepare(&localized, &template, &build);
    assert!(changed
        .injected_files
        .contains(&"next.config.mjs".to_string()));
    let content = read(&build, "next.config.mjs");
    assert!(
        !content.contains("__I18N_CONFIG__")
            && !content.contains("// __FOLIOH_BASE_PATH__")
            && content.contains("defaultLocale: 'en'")
    );

    let package = dir.path().join("theme-package");
    common::write(&package, "next.config.mjs", "const configuredBasePath = '' // __FOLIOH_BASE_PATH__\nconst nextConfig = {\n  images: { unoptimized: true },\n  __I18N_CONFIG__\n}\n");
    let packaged = common::config(dir.path(), "theme:\n  package: theme-package\n");
    let build = dir.path().join("build-package");
    common::prepare(&packaged, &template, &build);
    common::prepare(&packaged, &template, &build);
    let content = read(&build, "next.config.mjs");
    assert!(!content.contains("__I18N_CONFIG__") && !content.contains("// __FOLIOH_BASE_PATH__"));
}

#[test]
fn theme_header_writes_logo_and_actions() {
    let dir = tempfile::tempdir().unwrap();
    let template = common::make_template(dir.path());
    common::write(
        &template,
        "theme/project-theme.ts",
        "export const projectThemePreset = null\nexport const projectThemeDefaultConfig = {}\n",
    );
    common::write(
        &template,
        "app/docs/layout.tsx",
        "import { VersionSelector } from \"@/components/version-selector\"\nimport { getPageMap } from \"nextra/page-map\"\n// __PROJECT_HEADER_ACTION_IMPORTS_START__\n// __PROJECT_HEADER_ACTION_IMPORTS_END__\nlogo={\n  <span>\n    {/* __PROJECT_HEADER_LOGO_START__ */}\n    <span>__PROJECT_MONOGRAM__</span>\n    <span>__PROJECT_NAME__</span>\n    {/* __PROJECT_HEADER_LOGO_END__ */}\n  </span>\n}\n{/* __PROJECT_HEADER_ACTIONS_START__ */}\n<VersionSelector />\n{/* __PROJECT_HEADER_ACTIONS_END__ */}\npageMap={await getPageMap(\"/docs\")}\nfooter={<Footer />}\n",
    );
    common::write(
        &template,
        "components/theme-configurator.tsx",
        "const configuredDefaultPresetId = \"organic-editorial\" // __FOLIOH_THEME_PRESET__\n",
    );
    let config = common::config(
        dir.path(),
        "theme:\n  preset: p2pfl\n  name: P2PFL\n  header:\n    brand: p2pfl\n    badge: Web Services\n    repo: https://github.com/pguijas/p2pfl\n    theme_toggle: true\n    action_label: Dashboard\n    action_href: /dashboard\n    search: false\n",
    );
    let build = common::prepared(dir.path(), &config, &template, "header");
    let docs = read(&build, "app/docs/layout.tsx");
    assert!(docs
        .contains("import { ProjectHeaderActions } from \"@/components/project-header-actions\""));
    assert!(docs.contains("<ProjectHeaderActions\n  repoHref={\"https://github.com/pguijas/p2pfl\"}\n  themeToggle\n  actionHref={\"/dashboard\"}\n  actionLabel={\"Dashboard\"}\n/>\n<VersionSelector />"), "{docs}");
    assert!(docs.contains("    <span className=\"text-sm font-semibold tracking-tight\">{\"p2pfl\"}</span>\n    <span className=\"rounded-full bg-primary/10 px-2 py-0.5 text-[10px] font-medium text-primary\">{\"Web Services\"}</span>\n"), "{docs}");
    assert!(docs.contains("search={null}") && !docs.contains("__PROJECT_HEADER_"));
    assert!(read(&build, "components/theme-configurator.tsx")
        .contains("const configuredDefaultPresetId = \"p2pfl\""));
    assert!(read(&build, "theme/project-theme.ts").contains("\"id\": \"p2pfl\""));

    let plain = common::config(dir.path(), "");
    let build = common::prepared(dir.path(), &plain, &template, "no-header");
    let docs = read(&build, "app/docs/layout.tsx");
    assert!(
        !docs.contains("    <span>__PROJECT_MONOGRAM__</span>\n")
            && docs.contains("    <span>{\"te\"}</span>\n    <span>{\"TestProject\"}</span>\n"),
        "{docs}"
    );
    assert!(
        docs.contains("<VersionSelector />")
            && !docs.contains("__PROJECT_HEADER_")
            && !docs.contains("ProjectHeaderActions")
    );
}

#[test]
fn theme_package_overlays_and_reserved_paths() {
    let dir = tempfile::tempdir().unwrap();
    let template = common::make_template(dir.path());
    common::write(
        &template,
        "theme/project-theme.ts",
        "export const projectThemePreset = null\nexport const projectThemeDefaultConfig = {}\n",
    );
    common::write(
        &template,
        "theme/theme-contract.generated.ts",
        "// STALE BUNDLED CONTRACT\n",
    );
    common::write(
        &template,
        "components/theme-configurator.tsx",
        "const configuredDefaultPresetId = \"organic-editorial\" // __FOLIOH_THEME_PRESET__\n",
    );
    common::write(
        &template,
        "components/project-header-actions.tsx",
        "export function ProjectHeaderActions() { return null }\n",
    );
    common::write(
        &template,
        "app/docs/layout.tsx",
        "import { VersionSelector } from \"@/components/version-selector\"\nimport { getPageMap } from \"nextra/page-map\"\n// __PROJECT_HEADER_ACTION_IMPORTS_START__\n// __PROJECT_HEADER_ACTION_IMPORTS_END__\n{/* __PROJECT_HEADER_ACTIONS_START__ */}\n<VersionSelector />\n{/* __PROJECT_HEADER_ACTIONS_END__ */}\npageMap={await getPageMap(\"/docs\")}\n",
    );
    let package = dir.path().join("docs/theme/p2pfl");
    common::write(&package, "theme/project-theme.ts", "export const projectThemePreset = { id: 'p2pfl-pack' }\nexport const projectThemeDefaultConfig = { presetId: 'p2pfl-pack' }\n");
    let package_configurator = "const configuredDefaultPresetId = \"organic-editorial\" // __FOLIOH_THEME_PRESET__\nexport function ThemeConfigurator() { return <div data-p2pfl-theme /> }\n";
    common::write(
        &package,
        "components/theme-configurator.tsx",
        package_configurator,
    );
    common::write(
        &package,
        "components/project-header-actions.tsx",
        "export function ProjectHeaderActions() { return <a>Pack action</a> }\n",
    );
    common::write(
        &package,
        "app/layout.tsx",
        "export default function RootLayout({ children }) { return children }\n",
    );
    let config = common::config(dir.path(), "theme:\n  preset: p2pfl-pack\n  package: docs/theme/p2pfl\n  header:\n    theme_toggle: true\n");
    let build = common::prepared(dir.path(), &config, &template, "pack");
    let project_theme = read(&build, "theme/project-theme.ts");
    assert!(project_theme.contains("p2pfl-pack") && !project_theme.contains("projectBaseStyle"));
    assert_eq!(
        read(&build, "components/theme-configurator.tsx"),
        package_configurator
    );
    assert!(read(&build, "components/project-header-actions.tsx").contains("Pack action"));
    assert!(read(&build, "app/layout.tsx").contains("return children"));
    let docs = read(&build, "app/docs/layout.tsx");
    assert!(
        docs.contains(
            "import { ProjectHeaderActions } from \"@/components/project-header-actions\""
        ) && docs.contains("<ProjectHeaderActions")
    );
    let contract = read(&build, "theme/theme-contract.generated.ts");
    assert!(!contract.contains("STALE BUNDLED CONTRACT"));
    assert_eq!(contract, folioh_site::theme::generate_typescript_contract());

    let stale = dir.path().join("theme_package_contract");
    common::write(
        &stale,
        "theme/theme-contract.generated.ts",
        "export const stale = 1\n",
    );
    let config = common::config(dir.path(), "theme:\n  package: theme_package_contract\n");
    let build = dir.path().join("build-stale");
    TemplateWorkspace::new(&template, &build, None)
        .prepare(false)
        .unwrap();
    let err = TemplateConfigInjector::new(&config, &build, &template)
        .inject()
        .unwrap_err()
        .to_string();
    assert!(err.contains("theme/theme-contract.generated.ts"), "{err}");

    let reserved = dir.path().join("theme_package_reserved");
    common::write(&reserved, "content/index.mdx", "# Reserved");
    let config = common::config(dir.path(), "theme:\n  package: theme_package_reserved\n");
    let build = dir.path().join("build-reserved");
    TemplateWorkspace::new(&template, &build, None)
        .prepare(false)
        .unwrap();
    let err = TemplateConfigInjector::new(&config, &build, &template)
        .inject()
        .unwrap_err()
        .to_string();
    assert!(err.contains("content/"), "{err}");
    assert!(!build.join("content/index.mdx").exists());

    let missing = common::config(dir.path(), "theme:\n  package: nowhere\n");
    let err = TemplateConfigInjector::new(&missing, &build, &template)
        .inject()
        .unwrap_err()
        .to_string();
    assert!(err.starts_with("Theme package not found: "), "{err}");
}

#[test]
fn plugin_view_owning_root_skips_the_docs_wrapper() {
    let dir = tempfile::tempdir().unwrap();
    let template = common::make_template(dir.path());
    common::write(&template, "app/docs/[[...mdxPath]]/page.jsx", "const docsIndexCanonicalPath = \"__DOCS_INDEX_CANONICAL_PATH__\"\nexport default function DocsPage() { return null }\n");
    common::write(&template, "app/sitemap.ts", "const INCLUDE_DOCS_INDEX: string = \"__INCLUDE_DOCS_INDEX__\"\nexport default function sitemap() { return [] }\n");
    let cases = [
        (json!("/"), false, "/docs/", "true"),
        (json!(true), true, "/", "false"),
        (json!("/gallery"), true, "/", "false"),
    ];
    for (index, (public, wrapper, canonical, include)) in cases.iter().enumerate() {
        let mut config = common::config(dir.path(), "");
        config
            .extra
            .insert("surface".into(), json!({"routes": {"public": public}}));
        let build = common::prepared(dir.path(), &config, &template, &format!("root-{index}"));
        let page = read(&build, "app/page.tsx");
        assert_eq!(
            page.contains("DocsLayout") && page.contains("DocsPage"),
            *wrapper,
            "{public}"
        );
        assert!(
            read(&build, "app/docs/[[...mdxPath]]/page.jsx")
                .contains(&format!("const docsIndexCanonicalPath = \"{canonical}\"")),
            "{public}"
        );
        assert!(
            read(&build, "app/sitemap.ts")
                .contains(&format!("const INCLUDE_DOCS_INDEX: string = \"{include}\"")),
            "{public}"
        );
    }
}

fn landing(dir: &Path, name: &str, section: serde_json::Value) -> folioh_config::DocsConfig {
    let mut config = common::config(
        dir,
        &format!("project:\n  name: \"{name}\"\nlanding: true\n"),
    );
    configure_landing(&mut config, section);
    config
}

#[test]
fn bundled_landing_page_serialises_values_safely() {
    let dir = tempfile::tempdir().unwrap();
    let template = common::bundled_template();
    let quoted = landing(
        dir.path(),
        "Quote \\\"Docs\\\"",
        json!({"hero": {"tagline": "Ship \"docs\"", "headline": "Docs for \"quoted\" APIs", "description": "Line one\nLine \"two\""}, "cta": {"primary": {"text": "Start \"now\"", "link": "/docs?query=\"quoted\""}}}),
    );
    let build = common::prepared(dir.path(), &quoted, &template, "quoted");
    let page = read(&build, "app/page.tsx");
    let navbar = read(&build, "components/landing-navbar.tsx");
    assert!(
        page.contains("const landingHeadline = \"Docs for \\\"quoted\\\" APIs\""),
        "{page}"
    );
    assert!(page.contains("const landingDescription = \"Line one\\nLine \\\"two\\\"\""));
    assert!(page.contains("const primaryCtaLink = \"/docs?query=\\\"quoted\\\"\""));
    assert!(
        page.contains("const secondaryCtaLink: string | null = null")
            && !page.contains("https://github.com")
            && page.contains("{projectMonogram}")
    );
    assert!(
        navbar.contains("const secondaryCtaLink: string | null = null")
            && !navbar.contains("https://github.com")
    );
    assert!(page.contains(
        "Landing page, roadmap, and OpenAPI activate from docs.yaml sections; nothing else to install or run."
    ));
    assert!(
        page.contains("const landingSections = [{\"type\": \"features\"")
            && page.contains("\"type\": \"routes\"")
            && page.contains("\"type\": \"output\"")
            && page.contains("\"type\": \"cta\"")
    );
    assert!(page.contains("const landingHeroVariant = \"docs-map\""));

    let empty_tagline = landing(dir.path(), "No Kicker", json!({"hero": {"tagline": ""}}));
    let build = common::prepared(dir.path(), &empty_tagline, &template, "tagline");
    let page = read(&build, "app/page.tsx");
    assert!(page.contains("const landingTagline = \"\"") && !page.contains(".py → no kicker"));
    let default_tagline = landing(dir.path(), "Kicker", json!({"hero": {"tagline": null}}));
    let build = common::prepared(dir.path(), &default_tagline, &template, "default-tagline");
    assert!(
        read(&build, "app/page.tsx").contains("const landingTagline = \"\""),
        "an omitted tagline is no kicker"
    );
}

#[test]
fn a_hostile_project_name_reaches_every_bundled_file_escaped() {
    let dir = tempfile::tempdir().unwrap();
    let template = common::bundled_template();
    let config = common::config(
        dir.path(),
        "project:\n  name: '<\"q\" {b} \\ `t` ${x}>'\n  repo: 'https://github.com/o/r\"x'\n  url: 'https://example.com/\"u'\nlanding: true\n",
    );
    assert_eq!(config.project.name, "<\"q\" {b} \\ `t` ${x}>");
    let build = common::prepared(dir.path(), &config, &template, "hostile");
    let name = r#""<\"q\" {b} \\ `t` ${x}>""#;
    let expected = [
        ("app/layout.tsx", format!("const projectName = {name}")),
        (
            "app/layout.tsx",
            "const configuredSiteUrl = \"https://example.com/\\\"u\"".to_string(),
        ),
        (
            "app/docs/layout.tsx",
            r#"alt: "<\"q\" {b} \\ `t` ${x}> documentation""#.to_string(),
        ),
        ("app/docs/layout.tsx", format!("{{{name}}}")),
        (
            "app/docs/layout.tsx",
            "href={\"https://github.com/o/r\\\"x\"}".to_string(),
        ),
        (
            "app/docs/layout.tsx",
            "docsRepositoryBase={\"https://github.com/o/r\\\"x\"}".to_string(),
        ),
        (
            "app/docs/[[...mdxPath]]/page.jsx",
            format!("const projectName = {name}"),
        ),
        ("app/opengraph-image.tsx", format!("{{{name}}}")),
        ("app/docs/opengraph-image.tsx", format!("{{{name}}}")),
        ("app/previews/layout.tsx", format!("{{{name}}}")),
        (
            "app/sitemap.ts",
            "const SITE_URL: string = \"https://example.com/\\\"u\"".to_string(),
        ),
        ("app/icon.svg", ">&lt;&quot;</text>".to_string()),
    ];
    for (file, text) in &expected {
        let content = read(&build, file);
        assert!(
            content.contains(text.as_str()),
            "{file} lacks {text}:\n{content}"
        );
    }
    for file in [
        "app/layout.tsx",
        "app/docs/layout.tsx",
        "app/docs/[[...mdxPath]]/page.jsx",
        "app/opengraph-image.tsx",
        "app/docs/opengraph-image.tsx",
        "app/previews/layout.tsx",
        "app/page.tsx",
        "components/landing-navbar.tsx",
    ] {
        let content = read(&build, file);
        assert!(
            !content.contains("<\"q\"") && !content.contains("__PROJECT_"),
            "{file} holds the raw name or a marker:\n{content}"
        );
    }
}

#[test]
fn bundled_landing_secondary_cta_hero_variant_and_sections() {
    let dir = tempfile::tempdir().unwrap();
    let template = common::bundled_template();
    let repo = common::config(
        dir.path(),
        "project:\n  name: RepoDocs\n  repo: https://github.com/acme/repo\nlanding: true\n",
    );
    let build = common::prepared(dir.path(), &repo, &template, "repo");
    for file in ["app/page.tsx", "components/landing-navbar.tsx"] {
        let text = read(&build, file);
        assert!(
            text.contains(
                "const secondaryCtaLink: string | null = \"https://github.com/acme/repo\""
            ) && text.contains("const secondaryCtaText = \"GitHub\""),
            "{file}"
        );
    }
    let navbar = read(&build, "components/landing-navbar.tsx");
    assert!(
        navbar.contains("normalizeLandingHref,\n} from \"@/components/landing/actions\"")
            && navbar.contains("? normalizeLandingHref(secondaryCtaLink, pathToRoot)")
    );

    let custom = landing(
        dir.path(),
        "ExampleDocs",
        json!({"cta": {"secondary": {"text": "View API", "link": "/docs/api-reference/example_package/arithmetic/"}}}),
    );
    let build = common::prepared(dir.path(), &custom, &template, "custom");
    let navbar = read(&build, "components/landing-navbar.tsx");
    assert!(
        read(&build, "app/page.tsx").contains("const secondaryCtaText = \"View API\"")
            && navbar.contains("const secondaryCtaText = \"View API\"")
    );
    assert!(navbar.contains("const secondaryCtaLink: string | null = \"/docs/api-reference/example_package/arithmetic/\""));

    let pipeline = landing(
        dir.path(),
        "ExampleDocs",
        json!({"hero": {"variant": "source-pipeline"}, "comparison": false}),
    );
    let build = common::prepared(dir.path(), &pipeline, &template, "pipeline");
    let page = read(&build, "app/page.tsx");
    assert!(
        page.contains("const landingHeroVariant = \"source-pipeline\"")
            && !page.contains("\"type\": \"comparison\"")
            && !page.contains("__LANDING_")
    );
    let compared = landing(
        dir.path(),
        "ExampleDocs",
        json!({"hero": {"variant": "source-pipeline"}, "comparison": {"caption": "Cap", "tools": ["A"], "rows": [{"feature": "Static export", "values": [true]}]}}),
    );
    let build = common::prepared(dir.path(), &compared, &template, "compared");
    assert!(read(&build, "app/page.tsx").contains(
        "{\"type\": \"comparison\", \"caption\": \"Cap\", \"tools\": [\"A\"], \"rows\": [{\"feature\": \"Static export\", \"values\": [true]"
    ));

    let catalog = landing(
        dir.path(),
        "CatalogDocs",
        json!({"sections": [{"type": "stats", "eyebrow": "Adoption", "title": "Used by \"teams\"", "items": [{"value": "3", "label": "commands"}]}, {"type": "cta", "title": "Read the generated docs", "actions": [{"title": "Open docs", "href": "/docs/"}]}]}),
    );
    let build = common::prepared(dir.path(), &catalog, &template, "catalog");
    let page = read(&build, "app/page.tsx");
    assert!(
        page.contains("const landingSections = [{\"type\": \"stats\"")
            && page.contains("\"title\": \"Used by \\\"teams\\\"\"")
            && page.contains("\"type\": \"cta\"")
    );
}

#[test]
fn bundled_template_without_landing_or_repo() {
    let dir = tempfile::tempdir().unwrap();
    let template = common::bundled_template();
    let docs_only = common::config(dir.path(), "project:\n  name: DocsOnly\n");
    let build = common::prepared(dir.path(), &docs_only, &template, "docs-only");
    let page = read(&build, "app/page.tsx");
    assert!(
        !page.contains("LandingNavbar")
            && page.contains("import DocsLayout from \"./docs/layout\"")
    );
    assert!(page.contains("import DocsPage, { generateMetadata as generateDocsMetadata } from \"./docs/[[...mdxPath]]/page\""));
    assert!(
        page.contains("mdxPath: []")
            && page.contains("<DocsLayout>")
            && page.contains("<DocsPage {...rootDocsProps()} />")
            && !page.contains("__LANDING_")
    );
    let docs = read(&build, "app/docs/layout.tsx");
    assert!(
        !docs.contains("GithubIcon")
            && !docs.contains("HugeiconsIcon")
            && !docs.contains("aria-label=\"GitHub repository\"")
            && !docs.contains("__PROJECT_REPO")
    );
    assert!(
        docs.contains("<ThemeGallery />")
            && !docs.contains("<ThemeConfigurator />")
            && docs.contains("darkMode={false}")
    );
    assert!(
        docs.contains("getPageMap(\"/docs\")") && docs.contains("url: \"/docs/opengraph-image\"")
    );
    assert!(!read(&build, "components/landing-navbar.tsx").contains("__PROJECT_NAME_JSON__"));
    assert!(read(&build, "app/docs/[[...mdxPath]]/page.jsx")
        .contains("const docsIndexCanonicalPath = \"/\""));
    assert!(read(&build, "app/sitemap.ts").contains("const INCLUDE_DOCS_INDEX: string = \"false\""));
    assert!(
        !build.join("next.config.mjs").exists()
            || !read(&build, "next.config.mjs").contains("__FOLIOH_BASE_PATH__")
    );
    let context = read(&build, "lib/folioh-template.ts");
    assert!(context.starts_with("export const foliohProject = {\n  \"name\": \"DocsOnly\",\n  \"version\": \"0.0.0\",\n  \"repo\": \"\",\n  \"repoRef\": \"main\",\n  \"url\": \"\"\n} as const\n\nexport const foliohTemplateParams = {} as const\n\nexport const foliohDocs = {\n  \"routeBase\": \"/docs\",\n  \"mdxContractVersion\": \"1.1\"\n} as const\n\nexport const foliohTemplateContext = {\n"), "{context}");
    assert!(context.ends_with("} as const\n"));
    assert!(read(&build, "lib/folioh-mdx-contract.ts").contains("\"name\": \"ParamTable\""));
    assert!(!read(&build, "app/icon.svg").contains("__PROJECT_MONOGRAM__"));
    assert!(read(&build, "app/icon.svg").contains(">do</text>"));
}

/// A git repository on disk holding `files`, fetched over `file://`.
fn origin(dir: &Path, files: &[(&str, &str)]) -> String {
    for (rel, body) in files {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, body).unwrap();
    }
    for args in [
        vec!["init", "--quiet", "-b", "main"],
        vec!["add", "-A"],
        vec!["commit", "--quiet", "-m", "theme"],
    ] {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(dir)
            .args([
                "-c",
                "user.email=theme@test",
                "-c",
                "user.name=Theme Test",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "core.hooksPath=",
            ])
            .args(&args)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}");
    }
    format!("file://{}", dir.display())
}

/// A theme nobody in the project wrote reaches `.build/` through the overlay,
/// and the rule a fetched package is held to is enforced where it is used.
#[test]
#[cfg(unix)]
fn a_fetched_theme_package_is_overlaid_like_a_local_one() {
    let dir = tempfile::tempdir().unwrap();
    let template = common::make_template(dir.path());

    let repo = dir.path().join("origin");
    std::fs::create_dir_all(&repo).unwrap();
    let url = origin(
        &repo,
        &[("app/globals.css", ":root { --brand: hotpink; }\n")],
    );

    // The digest of the tree the fetch will produce: the same files, no .git.
    let clean = dir.path().join("clean");
    std::fs::create_dir_all(clean.join("app")).unwrap();
    std::fs::write(
        clean.join("app/globals.css"),
        ":root { --brand: hotpink; }\n",
    )
    .unwrap();
    let digest = folioh_site::template::package_tree_digest(&clean).unwrap();

    let cache = dir.path().join("theme-cache");
    std::env::set_var(folioh_site::theme_package::THEME_CACHE_ENV, &cache);
    let config = common::config(
        dir.path(),
        &format!(
            "theme:\n  package:\n    git: \"{url}\"\n    rev: main\n    digest: \"sha256:{digest}\"\n"
        ),
    );
    let build = common::prepared(dir.path(), &config, &template, "fetched");
    std::env::remove_var(folioh_site::theme_package::THEME_CACHE_ENV);

    assert_eq!(
        read(&build, "app/globals.css"),
        ":root { --brand: hotpink; }\n",
        "the fetched package overlaid the template"
    );
    assert!(
        cache.is_dir(),
        "and it came from the cache the environment named"
    );
}

#[test]
fn a_misspelt_preset_stops_the_build_with_the_near_name() {
    let dir = tempfile::tempdir().unwrap();
    let template = common::make_template(dir.path());
    common::write(
        &template,
        "components/theme-configurator.tsx",
        "const configuredDefaultPresetId = \"organic-editorial\" // __FOLIOH_THEME_PRESET__\n",
    );
    let config = common::config(dir.path(), "theme:\n  preset: beakon\n");
    let build = dir.path().join("build-typo");
    TemplateWorkspace::new(&template, &build, None)
        .prepare(false)
        .unwrap();
    let err = TemplateConfigInjector::new(&config, &build, &template)
        .inject()
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("got 'beakon' (did you mean 'beacon'?)"),
        "{err}"
    );
}

#[test]
fn an_overlay_preset_is_one_the_configurator_shows() {
    let dir = tempfile::tempdir().unwrap();
    let template = common::make_template(dir.path());
    let configurator =
        "const configuredDefaultPresetId = \"organic-editorial\" // __FOLIOH_THEME_PRESET__\n";
    common::write(&template, "components/theme-configurator.tsx", configurator);
    let inject = |overlay_files: &[(&str, &str)], preset: &str| {
        let overlay = tempfile::tempdir_in(dir.path()).unwrap();
        for (rel, text) in overlay_files {
            common::write(overlay.path(), rel, text);
        }
        let yaml = format!(
            "template:\n  overlay_path: {}\ntheme:\n  preset: {preset}\n",
            overlay.path().file_name().unwrap().to_string_lossy()
        );
        let config = common::config(dir.path(), &yaml);
        let build = dir.path().join(format!("build-{preset}"));
        TemplateWorkspace::new(&template, &build, None)
            .prepare(false)
            .unwrap();
        TemplateConfigInjector::new(&config, &build, &template)
            .inject()
            .map(|_| ())
            .map_err(|e| e.to_string())
    };
    let presets = (
        "theme/presets.ts",
        "export const notebook = {\n  id: \"notebook\",\n}\n",
    );
    assert_eq!(inject(&[presets], "notebook"), Ok(()));
    let err = inject(&[presets], "notebok").unwrap_err();
    assert!(err.contains("(did you mean 'notebook'?)"), "{err}");
    // An overlay that replaces the configurator owns its preset list.
    let own = ("components/theme-configurator.tsx", configurator);
    assert_eq!(inject(&[own], "anything"), Ok(()));
}

#[test]
fn dark_mode_false_forces_light_and_drops_every_toggle() {
    let dir = tempfile::tempdir().unwrap();
    let template = common::bundled_template();
    let light = common::config(
        dir.path(),
        "theme:\n  dark_mode: false\n  header:\n    repo: https://github.com/o/r\n    theme_toggle: true\nlanding: true\n",
    );
    let build = dir.path().join("build-light");
    TemplateWorkspace::new(&template, &build, None)
        .prepare(false)
        .unwrap();
    let injection = TemplateConfigInjector::new(&light, &build, &template)
        .inject()
        .unwrap();
    assert!(injection.warnings.is_empty(), "{:?}", injection.warnings);
    let provider = read(&build, "components/theme-provider.tsx");
    assert!(
        provider.contains("const darkModeEnabled: boolean = false\n")
            && !provider.contains("__FOLIOH_DARK_MODE__"),
        "{provider}"
    );
    let navbar = read(&build, "components/landing-navbar.tsx");
    assert!(!navbar.contains("<ThemeToggle />"), "{navbar}");
    let docs = read(&build, "app/docs/layout.tsx");
    assert!(
        docs.contains("<ProjectHeaderActions") && !docs.contains("themeToggle"),
        "{docs}"
    );

    let default = common::config(dir.path(), "landing: true\n");
    let build = common::prepared(dir.path(), &default, &template, "default");
    assert!(read(&build, "components/theme-provider.tsx")
        .contains("const darkModeEnabled: boolean = true // __FOLIOH_DARK_MODE__"));
    let navbar = read(&build, "components/landing-navbar.tsx");
    assert!(
        !navbar.contains("<ThemeToggle />") && navbar.contains("<ThemeGallery />"),
        "{navbar}"
    );
}

#[test]
fn the_landing_toggle_stays_only_when_asked_for_with_dark_mode_on() {
    let dir = tempfile::tempdir().unwrap();
    let template = common::bundled_template();
    let asked = common::config(
        dir.path(),
        "theme:\n  header:\n    theme_toggle: true\nlanding: true\n",
    );
    let build = common::prepared(dir.path(), &asked, &template, "asked");
    assert!(read(&build, "components/landing-navbar.tsx").contains("<ThemeToggle />"));

    let declined = common::config(
        dir.path(),
        "theme:\n  header:\n    theme_toggle: false\nlanding: true\n",
    );
    let build = common::prepared(dir.path(), &declined, &template, "declined");
    assert!(!read(&build, "components/landing-navbar.tsx").contains("<ThemeToggle />"));
}

#[test]
fn dark_mode_false_warns_when_the_provider_cannot_take_it() {
    let dir = tempfile::tempdir().unwrap();
    let template = common::make_template(dir.path());
    let config = common::config(dir.path(), "theme:\n  dark_mode: false\n");
    let injection = common::prepare(&config, &template, &dir.path().join("build"));
    assert_eq!(
        injection.warnings,
        ["theme.dark_mode: false has no effect: components/theme-provider.tsx does not carry the // __FOLIOH_DARK_MODE__ marker, so dark mode stays available"]
    );
}

#[test]
fn theme_logo_is_an_image_in_the_logo_slot_or_a_clear_error() {
    let dir = tempfile::tempdir().unwrap();
    let template = common::bundled_template();
    common::write(dir.path(), "assets/logo mark.svg", "<svg/>");
    let config = common::config(dir.path(), "theme:\n  logo: assets/logo mark.svg\n");
    let build = common::prepared(dir.path(), &config, &template, "logo");
    assert_eq!(
        std::fs::read_to_string(build.join("public/logo mark.svg")).unwrap(),
        "<svg/>"
    );
    let docs = read(&build, "app/docs/layout.tsx");
    let image = "<img src={(process.env.NEXT_PUBLIC_FOLIOH_BASE_PATH ?? \"\") + \"/logo%20mark.svg\"} alt=\"\" className=\"h-7 w-auto\" />";
    let at = docs.find(image).unwrap_or_else(|| panic!("{docs}"));
    let slot = &docs[..at];
    assert!(slot.contains("logo={") && !slot[slot.rfind("logo={").unwrap()..].contains("<a"));
    assert!(docs[at..].contains("tracking-tight\">{\"TestProject\"}</span>"));
    assert!(!docs.contains("font-mono text-[11px] font-bold") && !docs.contains("HEADER_LOGO"));

    let branded = common::config(
        dir.path(),
        "theme:\n  logo: assets/logo mark.svg\n  header:\n    brand: Acme\n",
    );
    let build = common::prepared(dir.path(), &branded, &template, "branded");
    let docs = read(&build, "app/docs/layout.tsx");
    let at = docs.find(image).unwrap();
    assert!(docs[at..].contains("{\"Acme\"}</span>"), "{docs}");
    let previews = read(&build, "app/previews/layout.tsx");
    let at = previews.find(image).unwrap();
    assert!(previews[at..].contains("{\"Acme\"}</span>"), "{previews}");

    let missing = common::config(dir.path(), "theme:\n  logo: assets/nope.svg\n");
    let build = dir.path().join("build-missing");
    TemplateWorkspace::new(&template, &build, None)
        .prepare(false)
        .unwrap();
    let err = TemplateConfigInjector::new(&missing, &build, &template)
        .inject()
        .unwrap_err()
        .to_string();
    assert!(
        err.starts_with("theme.logo does not exist: ") && err.ends_with("assets/nope.svg"),
        "{err}"
    );
}

#[test]
fn theme_logo_replaces_monograms_in_landing_docs_and_previews() {
    let dir = tempfile::tempdir().unwrap();
    let template = common::bundled_template();
    common::write(dir.path(), "assets/logo mark.svg", "<svg/>");
    let config = common::config(dir.path(), "theme:\n  logo: assets/logo mark.svg\n");
    let build = common::prepared(dir.path(), &config, &template, "logo");
    let navbar = read(&build, "components/landing-navbar.tsx");
    assert!(
        navbar.contains("const projectLogo: string | null = \"/logo%20mark.svg\"\n")
            && navbar.contains("src={normalizeLandingHref(projectLogo, pathToRoot)}"),
        "{navbar}"
    );
    // The monogram is the fallback branch of the logo, never drawn beside it.
    let at = navbar.find("{projectLogo ? (").unwrap();
    let (logo, fallback) = navbar[at..].split_once(") : (").unwrap();
    assert!(logo.contains("<img") && !logo.contains("projectMonogram"));
    assert!(fallback[..fallback.find(")}").unwrap()].contains("{projectMonogram}"));
    assert_eq!(navbar.matches("{projectMonogram}").count(), 1, "{navbar}");
    let docs = read(&build, "app/docs/layout.tsx");
    assert_eq!(docs.matches("<img ").count(), 1, "{docs}");
    assert_eq!(docs.matches("\"/logo%20mark.svg\"").count(), 1, "{docs}");
    let previews = read(&build, "app/previews/layout.tsx");
    assert_eq!(previews.matches("<img ").count(), 1, "{previews}");
    assert!(previews
        .contains("(process.env.NEXT_PUBLIC_FOLIOH_BASE_PATH ?? \"\") + \"/logo%20mark.svg\""));
    assert!(!previews.contains("folioh-monogram") && !previews.contains("HEADER_LOGO"));

    // Without a logo the landing navbar keeps the monogram.
    let plain = common::config(dir.path(), "");
    let build = common::prepared(dir.path(), &plain, &template, "no-logo");
    let navbar = read(&build, "components/landing-navbar.tsx");
    assert!(
        navbar.contains("const projectLogo: string | null = null\n")
            && navbar.contains("const projectMonogram = \"te\"\n"),
        "{navbar}"
    );
    assert!(!read(&build, "app/docs/layout.tsx").contains("<img "));
    let previews = read(&build, "app/previews/layout.tsx");
    assert!(!previews.contains("<img ") && previews.contains("folioh-monogram"));
    assert!(!previews.contains("HEADER_LOGO"));
}

#[test]
fn missing_favicons_fail_with_the_requested_path() {
    let dir = tempfile::tempdir().unwrap();
    let template = common::bundled_template();
    std::fs::create_dir(dir.path().join("assets")).unwrap();
    // A directory cannot silently select the bundled icon either.
    for (index, path) in ["assets/missing.ico", "assets"].into_iter().enumerate() {
        let config = common::config(dir.path(), &format!("theme:\n  favicon: {path}\n"));
        let build = dir.path().join(format!("missing-favicon-{index}"));
        TemplateWorkspace::new(&template, &build, None)
            .prepare(false)
            .unwrap();
        let error = TemplateConfigInjector::new(&config, &build, &template)
            .inject()
            .unwrap_err()
            .to_string();
        assert_eq!(
            error,
            format!(
                "theme.favicon does not exist: {}",
                dir.path().canonicalize().unwrap().join(path).display()
            )
        );
    }
}

#[test]
fn the_extra_preset_catalog_is_explicitly_enabled() {
    let dir = tempfile::tempdir().unwrap();
    let template = common::make_template(dir.path());
    common::write(
        &template,
        "theme/preset-catalog.ts",
        "export const extraPresetsEnabled: boolean = false // __FOLIOH_EXTRA_PRESETS__\n",
    );
    for enabled in [false, true] {
        let config = common::config(
            dir.path(),
            &format!("theme: {{extra_presets: {enabled}}}\n"),
        );
        let build = common::prepared(
            dir.path(),
            &config,
            &template,
            &format!("catalog-{enabled}"),
        );
        assert_eq!(
            read(&build, "theme/preset-catalog.ts"),
            format!("export const extraPresetsEnabled: boolean = {enabled}\n")
        );
    }
}

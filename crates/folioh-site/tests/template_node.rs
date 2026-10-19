//! The bundled template's JavaScript and TypeScript helpers driven by Node
//! (skipped when `node` is absent or cannot import TypeScript directly).

mod common;

use std::path::Path;
use std::process::Command;

fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Run `node args...`; `Ok(stdout)`, `Err(None)` when Node cannot load TypeScript, `Err(Some(stderr))` otherwise.
fn run_node(args: &[&str], cwd: &Path) -> Result<String, Option<String>> {
    let output = Command::new("node")
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("node runs");
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    if stderr.contains("ERR_UNKNOWN_FILE_EXTENSION") {
        Err(None)
    } else {
        Err(Some(stderr))
    }
}

fn driver_json(dir: &Path, script: &str, module: &Path) -> Option<serde_json::Value> {
    let driver = common::write(dir, "driver.mjs", script);
    match run_node(
        &[
            "--no-warnings",
            driver.to_str().unwrap(),
            module.to_str().unwrap(),
        ],
        dir,
    ) {
        Ok(stdout) => Some(serde_json::from_str(stdout.trim()).expect("driver prints json")),
        Err(None) => None,
        Err(Some(stderr)) => panic!("node driver failed:\n{stderr}"),
    }
}

#[test]
fn preview_metadata_uses_each_sites_deploy_path() {
    let dir = tempfile::tempdir().unwrap();
    let helper = common::bundled_template().join("lib/site-metadata-url.js");
    let Some(value) = driver_json(
        dir.path(),
        r#"
const { siteMetadataUrl } = await import(process.argv[2])
delete process.env.FOLIOH_PREVIEW_URL
delete process.env.NEXT_PUBLIC_FOLIOH_BASE_PATH
const results = [siteMetadataUrl("https://site.example/"), siteMetadataUrl("")]
process.env.FOLIOH_PREVIEW_URL = "https://dev.example/previews/pr-1/"
results.push(siteMetadataUrl("https://site.example"))
for (const path of ["/previews/pr-1", "/previews/pr-1/_previews/example", ""]) {
  process.env.NEXT_PUBLIC_FOLIOH_BASE_PATH = path
  results.push(siteMetadataUrl("https://site.example"))
}
console.log(JSON.stringify(results))
"#,
        &helper,
    ) else {
        return;
    };
    assert_eq!(
        value,
        serde_json::json!([
            "https://site.example",
            "",
            "https://dev.example/previews/pr-1",
            "https://dev.example/previews/pr-1",
            "https://dev.example/previews/pr-1/_previews/example",
            "https://dev.example"
        ])
    );
}

#[test]
fn docs_route_params_expand_index_html_and_underscore_aliases() {
    if !node_available() {
        return;
    }
    let helper = common::bundled_template().join("lib/docs-route-params.js");
    let helper_url = format!("file://{}", helper.display());
    let script = format!(
        r##"
import {{ expandStaticParams, isDisabledMdxPath, isUnderscoreAlias, normalizeMdxPath }} from {helper_url:?}
const assert = (condition, message) => {{ if (!condition) throw new Error(message) }}
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b)
assert(same(normalizeMdxPath(["index.html"]), []), "index.html should resolve to the docs root")
assert(same(normalizeMdxPath(["components", "index.html"]), ["components"]), "nested index.html should resolve to its directory route")
assert(same(normalizeMdxPath(["source", "common_errors"]), ["source", "common-errors"]), "hand-written docs paths should accept underscore aliases")
assert(same(normalizeMdxPath(["source", "common_errors", "index.html"]), ["source", "common-errors"]), "underscore aliases should compose with index.html aliases")
assert(same(normalizeMdxPath(["api-reference", "example_package", "module_name"]), ["api-reference", "example_package", "module_name"]), "API reference names keep underscores")
assert(isUnderscoreAlias(["source", "common_errors"]) && isUnderscoreAlias(["source", "common_errors", "index.html"]), "underscore paths are aliases")
assert(!isUnderscoreAlias(["source", "common-errors"]) && !isUnderscoreAlias([]) && !isUnderscoreAlias(undefined), "hyphenated paths and the root are not aliases")
assert(!isUnderscoreAlias(["api-reference", "example_package"]), "API reference names are not aliases")
assert(isDisabledMdxPath(["i18n"]) && isDisabledMdxPath(["i18n", "index.html"]) && isDisabledMdxPath(["versioning"]), "gated routes are recognized")
for (const path of [["configuration"], ["roadmap"], ["landing"], ["plugins"], ["api-reference", "folioh", "plugins", "roadmap"]]) {{
  assert(!isDisabledMdxPath(path), `released route must not be gated: ${{path.join("/")}}`)
}}
const params = expandStaticParams(
  [{{ mdxPath: [""] }}, {{ mdxPath: ["components"] }}, {{ mdxPath: ["source", "common-errors"] }}, {{ mdxPath: ["api-reference", "example_package"] }}, {{ lang: "en", mdxPath: ["guide"] }}],
  {{ includeIndexHtmlAliases: true, includeDisabledParams: true }},
)
const has = (path, extra = () => true) => params.some((p) => same(p.mdxPath, path) && extra(p))
assert(has([]), "root param should be normalized")
assert(has(["index.html"]), "root index.html alias should be included")
assert(has(["components", "index.html"]), "nested index.html alias should be included")
assert(has(["source", "common_errors"]), "underscore aliases should be included for hand-written docs routes")
assert(has(["source", "common_errors", "index.html"]), "underscore index.html aliases should be included")
assert(!has(["api-reference", "example-package"]), "API reference static params should not get hyphenated aliases")
assert(has(["guide", "index.html"], (p) => p.lang === "en"), "locale params should keep their non-route fields on aliases")
assert(has(["i18n"]) && has(["i18n", "index.html"]) && has(["versioning"]), "disabled routes are included for dev export requests")
const production = expandStaticParams(
  [{{ mdxPath: [""] }}, {{ mdxPath: ["components"] }}, {{ mdxPath: ["source", "common-errors"] }}],
  {{ includeIndexHtmlAliases: false, includeDisabledParams: false }},
)
assert(!production.some((p) => p.mdxPath.includes("index.html")), "index.html aliases stay out of production exports")
assert(!production.some((p) => p.mdxPath.includes("i18n")), "disabled params stay out of production exports")
assert(production.some((p) => same(p.mdxPath, ["source", "common_errors"])), "underscore aliases are generated for static exports")
"##
    );
    if let Err(Some(stderr)) = run_node(
        &["--input-type=module", "--eval", &script],
        &common::bundled_template(),
    ) {
        panic!("{stderr}");
    }
}

const REGISTRY_DRIVER: &str = r##"
import { pathToFileURL } from "node:url"
const [registryPath] = process.argv.slice(2)
const { registerPreset, registerGroup, getPresets, getGroups, groupPresetsForDisplay } = await import(pathToFileURL(registryPath).href)
const theme = { preview: { light: "#fff", dark: "#000" }, radius: "0", style: {}, light: {}, dark: {} }
const makePreset = (id, name = id) => ({ id, name, description: "", scene: "", preview: { light: "#fff", dark: "#000" }, defaultOptions: {}, controls: [], resolve: () => theme })
registerPreset(makePreset("atlas", "Atlas"))
registerPreset(makePreset("beacon", "Beacon"))
registerPreset(makePreset("atlas", "Customized Atlas"), "project")
registerPreset(makePreset("ext-preset", "Extension"), "someGroup")
registerPreset(makePreset("loose", "Loose"))
registerGroup("project", "Project", ["atlas"])
registerGroup("someGroup", "Some Group", [])
registerGroup("reference", "Reference", ["atlas", "beacon"])
registerGroup("empty", "Empty", [])
const presets = getPresets()
const groups = getGroups()
const display = groupPresetsForDisplay(groups, presets)
console.log(JSON.stringify({
  atlasEntries: presets.filter((p) => p.id === "atlas").map((p) => p.name),
  someGroup: groups.find((g) => g.id === "someGroup"),
  display: display.map((g) => ({ id: g.id, label: g.label, presetNames: g.presets.map((p) => p.name) })),
}))
"##;

#[test]
fn preset_registry_is_last_wins_and_display_dedups_with_a_fallback_group() {
    if !node_available() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let Some(data) = driver_json(
        dir.path(),
        REGISTRY_DRIVER,
        &common::bundled_template().join("theme/preset-registry.ts"),
    ) else {
        return;
    };
    assert_eq!(
        data["atlasEntries"],
        serde_json::json!(["Customized Atlas"])
    );
    assert_eq!(data["someGroup"]["label"], "Some Group");
    assert_eq!(
        data["someGroup"]["presetIds"],
        serde_json::json!(["ext-preset"])
    );
    let display = data["display"].as_array().unwrap();
    let by_id = |id: &str| {
        display
            .iter()
            .find(|g| g["id"] == id)
            .map(|g| g["presetNames"].clone())
    };
    let atlas: Vec<(String, String)> = display
        .iter()
        .flat_map(|g| {
            g["presetNames"].as_array().unwrap().iter().map(move |n| {
                (
                    g["id"].as_str().unwrap().to_string(),
                    n.as_str().unwrap().to_string(),
                )
            })
        })
        .filter(|(_, name)| name.ends_with("Atlas"))
        .collect();
    assert_eq!(
        atlas,
        [("project".to_string(), "Customized Atlas".to_string())]
    );
    assert_eq!(by_id("reference"), Some(serde_json::json!(["Beacon"])));
    assert_eq!(by_id("someGroup"), Some(serde_json::json!(["Extension"])));
    assert_eq!(by_id("other"), Some(serde_json::json!(["Loose"])));
    assert_eq!(by_id("empty"), None);
}

const BOOTSTRAP_DRIVER: &str = r##"
import { pathToFileURL } from "node:url"
const [typesPath] = process.argv.slice(2)
const { buildBootstrapPresets, countPresetOptionCombinations, MAX_BOOTSTRAP_COMBINATIONS } = await import(pathToFileURL(typesPath).href)
const theme = { preview: { light: "", dark: "" }, radius: "0", style: {}, light: {}, dark: {} }
const makeControl = (id, count) => ({ id, label: id, options: Array.from({ length: count }, (_, i) => ({ label: `${id}${i}`, value: `${id}${i}` })) })
const makePreset = (id, controls, counter) => ({
  id, name: id, description: "", scene: "", preview: { light: "", dark: "" },
  defaultOptions: Object.fromEntries(controls.map((control) => [control.id, control.options[0].value])),
  controls,
  resolve: () => { counter.count += 1; return theme },
})
const smallCounter = { count: 0 }
const small = makePreset("small", [makeControl("a", 2), makeControl("b", 2)], smallCounter)
const boundaryCounter = { count: 0 }
const boundary = makePreset("boundary", Array.from({ length: 9 }, (_, i) => makeControl(`b${i}`, 2)), boundaryCounter)
const overCounter = { count: 0 }
const over = makePreset("over", [makeControl("a", 8), makeControl("b", 8), makeControl("c", 9)], overCounter)
const hugeCounter = { count: 0 }
const huge = makePreset("huge", Array.from({ length: 10 }, (_, i) => makeControl(`h${i}`, 10)), hugeCounter)
const bootstrap = buildBootstrapPresets([small, boundary, over, huge])
const byId = Object.fromEntries(bootstrap.map((preset) => [preset.id, preset]))
console.log(JSON.stringify({
  max: MAX_BOOTSTRAP_COMBINATIONS,
  smallThemeKeyCount: Object.keys(byId.small.themes).length,
  boundaryThemeKeyCount: Object.keys(byId.boundary.themes).length,
  overThemeKeys: Object.keys(byId.over.themes),
  overDefaultKey: byId.over.defaultKey,
  hugeThemeKeys: Object.keys(byId.huge.themes),
  hugeDefaultKey: byId.huge.defaultKey,
  hugeResolveCalls: hugeCounter.count,
  overCombinationCount: countPresetOptionCombinations(over),
  hugeCombinationCount: countPresetOptionCombinations(huge),
}))
"##;

#[test]
fn bootstrap_presets_cap_combinations_without_enumerating_them() {
    if !node_available() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let Some(data) = driver_json(
        dir.path(),
        BOOTSTRAP_DRIVER,
        &common::bundled_template().join("theme/preset-types.ts"),
    ) else {
        return;
    };
    assert_eq!(data["max"], 512);
    assert_eq!(data["smallThemeKeyCount"], 4);
    assert_eq!(data["boundaryThemeKeyCount"], 512);
    assert_eq!(data["overCombinationCount"], 576);
    assert_eq!(
        data["overThemeKeys"],
        serde_json::json!([data["overDefaultKey"]])
    );
    assert_eq!(data["hugeCombinationCount"], 10_000_000_000u64);
    assert_eq!(
        data["hugeThemeKeys"],
        serde_json::json!([data["hugeDefaultKey"]])
    );
    assert_eq!(data["hugeResolveCalls"], 1);
}

/// A Node loader for the template's own modules: `@/` resolves to the template
/// root, extensionless imports find their `.ts` or `.tsx` file, and both are
/// compiled with the template's TypeScript. `projectTheme`, when set, stands in
/// for `theme/project-theme.ts`, as a project's `docs.yaml` would.
const TEMPLATE_LOADER: &str = r##"
import { existsSync, readFileSync } from "node:fs"
import { createRequire } from "node:module"
import { dirname, join, resolve as resolvePath } from "node:path"
import { fileURLToPath, pathToFileURL } from "node:url"
let root
let projectTheme
let extraPresets
let dependencyUrl
let ts
export function initialize(data) {
  root = data.root
  projectTheme = data.projectTheme
  extraPresets = data.extraPresets
  dependencyUrl = pathToFileURL(join(data.dependencies, "../package.json")).href
  ts = createRequire(dependencyUrl)("typescript")
}
const findFile = (base) => {
  for (const candidate of [base, `${base}.ts`, `${base}.tsx`]) {
    if (/\.(tsx?|m?js)$/.test(candidate) && existsSync(candidate)) return candidate
  }
  return undefined
}
export async function resolve(specifier, context, next) {
  // The feedback URL is pure; Nextra's hook contexts require the Next runtime.
  if (specifier === "nextra-theme-docs" && context.parentURL === pathToFileURL(join(root, "components/floating-feedback.tsx")).href) {
    return { url: "data:text/javascript," + encodeURIComponent('export const useConfig = () => { throw new Error("Nextra context is not mounted") }; export const useThemeConfig = useConfig'), shortCircuit: true }
  }
  const parent = context.parentURL?.startsWith("file:") ? dirname(fileURLToPath(context.parentURL)) : undefined
  const base = specifier.startsWith("@/")
    ? join(root, specifier.slice(2))
    : parent && /^\.\.?\//.test(specifier) ? resolvePath(parent, specifier) : undefined
  const file = base && findFile(base)
  // pnpm dependencies resolve their own imports from their package, not the template root.
  if (!file) return next(specifier, specifier.startsWith(".") || parent?.includes("/node_modules/") ? context : { ...context, parentURL: dependencyUrl })
  const own = projectTheme && file === join(root, "theme/project-theme.ts") ? projectTheme : file
  return { url: pathToFileURL(own).href, shortCircuit: true }
}
export async function load(url, context, next) {
  if (!/\.tsx?$/.test(url)) return next(url, context)
  const fileName = fileURLToPath(url)
  let source = readFileSync(fileName, "utf8")
  if (fileName === join(root, "theme/preset-catalog.ts") && extraPresets !== undefined) {
    source = source.replace("export const extraPresetsEnabled: boolean = false // __FOLIOH_EXTRA_PRESETS__", `export const extraPresetsEnabled: boolean = ${extraPresets}`)
  }
  const { outputText } = ts.transpileModule(source, {
    fileName,
    compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022, jsx: ts.JsxEmit.ReactJSX },
  })
  return { format: "module", source: outputText, shortCircuit: true }
}
"##;

/// Facts about the theme model, from the template's presets and
/// `components/theme-configurator.tsx`, with a stub `localStorage`.
const THEME_MODEL_DRIVER: &str = r##"
import { register } from "node:module"
import { join } from "node:path"
import { pathToFileURL } from "node:url"
import { runInNewContext } from "node:vm"
const [loader, root, dependencies, projectTheme] = process.argv.slice(2)
register(pathToFileURL(loader).href, { data: { root, dependencies, projectTheme, extraPresets: true } })
const load = (path) => import(pathToFileURL(join(root, path)).href)
const { presets } = await load("theme/presets.ts")
const { getPresetOptionCombinations, normalizePresetOptions, resolvePresetTheme } = await load("theme/preset-types.ts")
const model = await load("components/theme-configurator.tsx")
const { themeVariants } = await load("components/theme-adjustments.tsx")
const { gallerySlides, listThemes } = await load("components/theme-gallery.tsx")
const { DEFAULT_CONFIG, configForPreset, fontOptions, getPresetDefaults, radiusOptions, readThemeConfig } = model
const { feedbackHref } = await load("components/floating-feedback.tsx")
const feedbackLinks = [
  feedbackHref({ content: "Feedback", labels: "feedback,docs" }, "https://github.com/acme/docs/tree/main/docs", "Install & run"),
  feedbackHref({ content: "Feedback", labels: "documentation" }, "https://gitlab.example.com/acme/docs/-/tree/main", "API"),
  feedbackHref({ content: "Feedback", labels: "feedback", link: "https://support.example.com/new?from=docs" }, "", "API"),
  feedbackHref({ content: null, labels: "feedback", link: "https://support.example.com/new" }, "", "API"),
  feedbackHref({ content: "Feedback", labels: "feedback" }, "https://example.com/acme/docs", "API"),
]
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b)
const colourControls = (preset) => preset.controls.filter((control) => control.kind === "color")
const gallery = themeVariants(presets).map(({ preset, option }) => `${preset.id}:${option?.value ?? ""}`)
const projectBeacon = { ...presets.find((preset) => preset.id === "beacon"), name: "Project Beacon" }
const projectVariants = themeVariants([projectBeacon]).map(({ option }) => option?.value)
const { builtinGalleryVariants } = await load("theme/preset-registry.ts")
const catalog = new Map(builtinGalleryVariants)
builtinGalleryVariants.clear()
const overlayVariants = themeVariants(presets).length
catalog.forEach((values, preset) => builtinGalleryVariants.set(preset, values))
const galleryFamilies = listThemes()
const galleryDrafts = {
  folioh: configForPreset(DEFAULT_CONFIG, "organic-editorial", { image: "cobalt" }),
  "preset:pastel": configForPreset(DEFAULT_CONFIG, "pastel"),
  "preset:omarchy": configForPreset(DEFAULT_CONFIG, "omarchy", { palette: "tokyo-night" }),
}
const slides = gallerySlides(galleryFamilies, galleryDrafts)
const galleryFailures = []
const galleryCheck = (ok, message) => { if (!ok) galleryFailures.push(message) }
galleryCheck(same(galleryFamilies.map(({ name, presets }) => [name, themeVariants(presets).length]), [["Folioh", 6], ["Folioh Pastel", 5], ["Omarchy", 4]]), "Folioh Pastel is independent of Folioh, before Omarchy")
const slideIds = (slides) => slides.map(({ preset, option }) => `${preset.id}:${option?.value ?? ""}`)
galleryCheck(same(slideIds(slides), gallery), "slides follow every swatch in family order")
for (const slide of slides) {
  const options = normalizePresetOptions(slide.preset, slide.draft.optionsByPreset[slide.preset.id])
  galleryCheck(slide.draft.presetId === slide.preset.id && (!slide.control || options[slide.control.id] === slide.option?.value), `${slide.key}: neighbor renders its own preset and color`)
}
const custom = { ...galleryDrafts.folioh, colorOverrides: { light: { "--primary": "#123456" } } }
const customSlides = gallerySlides(galleryFamilies, { ...galleryDrafts, folioh: custom })
galleryCheck(customSlides[0].draft === custom, "front keeps manual changes")
galleryCheck(!customSlides[1].draft.colorOverrides, "manual changes cannot leak to a neighbor")
for (const [family, preset, options] of [["folioh", "beacon", { surface: "console" }], ["preset:omarchy", "omarchy", { palette: "white" }]]) {
  const active = galleryFamilies.findIndex(({ id }) => id === family)
  const legacy = configForPreset(DEFAULT_CONFIG, preset, options)
  const drafts = { ...galleryDrafts, [family]: legacy }
  const before = gallerySlides(galleryFamilies, drafts)
  galleryCheck(before.some((slide) => slide.draft === legacy), `${preset}: saved recipe remains reachable from any family`)
  const next = before.find((slide) => slide.familyIndex === active && !slide.selected)
  galleryCheck(same(slideIds(gallerySlides(galleryFamilies, { ...drafts, [family]: next.draft })), gallery), `${preset}: choosing another variant removes the saved recipe from the slides`)
}
const externalFamilies = [
  { id: "project", name: "Project", presets: [projectBeacon] },
  { id: "plain", name: "Plain", presets: [{ ...presets.find((preset) => preset.id === "workshop") }] },
]
galleryCheck(same(slideIds(gallerySlides(externalFamilies, {
  project: configForPreset(DEFAULT_CONFIG, "beacon"), plain: configForPreset(DEFAULT_CONFIG, "workshop"),
})), ["beacon:studio", "beacon:console", "workshop:"]), "project replacements and presets without colors remain selectable")

// Every combination of a preset's other controls, so a colour is checked
// against each of them.
const combinations = (controls) =>
  controls.reduce((list, control) => list.flatMap((values) => control.options.map((option) => ({ ...values, [control.id]: option.value }))), [{}])

// The style properties that hold a colour; a colour option may set these
// and nothing else outside the light and dark tokens.
const COLOUR_STYLE_KEYS = ["--folioh-code-bg"]
const leaks = []
for (const preset of presets) {
  for (const control of colourControls(preset)) {
    const others = preset.controls.filter((other) => other !== control)
    for (const rest of combinations(others)) {
      const themes = control.options.map((option) =>
        resolvePresetTheme(preset, normalizePresetOptions(preset, { ...rest, [control.id]: option.value }))
      )
      themes.forEach((theme, at) => {
        const where = `${preset.id} ${control.id}=${control.options[at].value} ${JSON.stringify(rest)}`
        const keys = new Set([...Object.keys(theme.style), ...Object.keys(themes[0].style)])
        for (const key of keys) {
          if (theme.style[key] !== themes[0].style[key] && !COLOUR_STYLE_KEYS.includes(key)) leaks.push(`${where}: ${key}`)
        }
        if (theme.radius !== themes[0].radius) leaks.push(`${where}: radius`)
      })
    }
  }
}

const radiusLost = []
for (const preset of presets) {
  const control = colourControls(preset)[0]
  if (!control) continue
  const start = configForPreset(DEFAULT_CONFIG, preset.id)
  const radiusIndex = (start.radiusIndex + 1) % radiusOptions.length
  const tuned = { ...start, radiusIndex }
  for (const option of control.options) {
    const next = configForPreset(tuned, preset.id, { ...tuned.optionsByPreset[preset.id], [control.id]: option.value })
    if (next.radiusIndex !== radiusIndex || !same(next.customization, tuned.customization)) {
      radiusLost.push(`${preset.id} ${control.id}=${option.value}`)
    }
  }
}

const site = presets.find((preset) => preset.id === DEFAULT_CONFIG.presetId)
const away = presets.find((preset) => preset.id !== site.id)
const back = configForPreset(configForPreset(DEFAULT_CONFIG, away.id), site.id)
const siteDefaults = getPresetDefaults(site)

const pastel = presets.find((preset) => preset.id === "pastel")
const pastelTheme = resolvePresetTheme(pastel, normalizePresetOptions(pastel))
const pastelConfig = configForPreset(DEFAULT_CONFIG, pastel.id)
const pastelFont = fontOptions.find((option) => option.id === pastelConfig.customization.fontId)

const store = new Map()
globalThis.localStorage = {
  getItem: (key) => (store.has(key) ? store.get(key) : null),
  setItem: (key, value) => store.set(key, String(value)),
  removeItem: (key) => store.delete(key),
}
globalThis.window = globalThis
const stored = Object.fromEntries(
  presets.flatMap((preset) => {
    const control = colourControls(preset)[0]
    return control ? [[preset.id, { [control.id]: control.options.at(-1).value }]] : []
  })
)
store.set(`folioh-theme:${DEFAULT_CONFIG.presetId}`, JSON.stringify({ ...DEFAULT_CONFIG, optionsByPreset: stored }))
const read = readThemeConfig()
const notReadBack = Object.entries(stored)
  .filter(([id, options]) => {
    const [[controlId, value]] = Object.entries(options)
    return read.optionsByPreset[id]?.[controlId] !== value
      || configForPreset(read, id).optionsByPreset[id][controlId] !== value
  })
  .map(([id]) => id)

// Run the public bootstrap component and its script, then compare the CSS
// it writes before hydration with the immediate save/apply path.
const { createElement } = await import("react")
const { renderToStaticMarkup } = await import("react-dom/server")
const { default: postcss } = await import("postcss")
const { ThemePixelField } = await load("components/theme-artwork.tsx")
const { FOLIOH_BANNER, ThemeWordmark, wordmarkFrame } = await load("components/theme-wordmark.tsx")
const artworkFailures = []
const artworkCheck = (ok, message) => { if (!ok) artworkFailures.push(message) }
const wordmark = (name, interactive = false) => renderToStaticMarkup(createElement(ThemeWordmark, { name, interactive }))
const field = renderToStaticMarkup(createElement(ThemePixelField))
artworkCheck(wordmark("Folioh").includes(FOLIOH_BANNER) && wordmark("Folioh") === wordmark("fOLIoh"), "Folioh uses its banner regardless of case")
artworkCheck(wordmark("<Atlas & Co>").endsWith('>&lt;Atlas &amp; Co&gt;</span>'), "another project keeps its escaped name")
artworkCheck(wordmark("Folioh").startsWith('<span aria-hidden="true"'), "a preview wordmark stays decorative without a nested button")
artworkCheck(wordmark("Atlas", true).startsWith('<button') && wordmark("Atlas", true).includes('aria-label="Replay Atlas animation"'), "the page wordmark exposes a named native replay button")
for (const text of [FOLIOH_BANNER, "<Atlas & Co>", "Docs 🧭\nAPI v2"]) {
  const source = Array.from(text)
  for (let effect = 0; effect < 4; effect++) {
    for (const progress of [0, 0.25, 0.5, 0.9, 1]) {
      const frame = Array.from(wordmarkFrame(text, effect, progress))
      artworkCheck(frame.length === source.length && source.every((character, i) => !/\s/.test(character) || frame[i] === character), "animation preserves every cell, space and line break")
    }
    artworkCheck(wordmarkFrame(text, effect, 1) === text && wordmarkFrame(text, effect, 2) === text, "each effect finishes with the exact project text")
  }
}
artworkCheck(new Set([0, 1, 2, 3].map((effect) => wordmarkFrame(FOLIOH_BANNER, effect, 0.5))).size === 4, "the four effects produce different intermediate frames")
artworkCheck(field === renderToStaticMarkup(createElement(ThemePixelField)), "the pixel field renders deterministically")
artworkCheck(field.includes('aria-hidden="true"') && field.includes('focusable="false"'), "the pixel field is decorative and unfocusable")
artworkCheck((field.match(/<path\b/g) ?? []).length === 3 && !/\sid=/.test(field), "the pixel field has three paths and no colliding IDs")
const { PastelArtwork, pastelShapes } = await load("components/pastel-artwork.tsx")
const pastelArtwork = renderToStaticMarkup(createElement(PastelArtwork, { seed: "docs" }))
artworkCheck(pastelArtwork === renderToStaticMarkup(createElement(PastelArtwork, { seed: "docs" })), "Pastel artwork is deterministic across hydration")
artworkCheck(pastelArtwork.includes('aria-hidden="true"') && !/<button|\sid=/.test(pastelArtwork), "Pastel artwork stays decorative without duplicate IDs")
for (const seed of ["docs", "", "api/café🧭", "a", "b", "c", "d", "e", "f"]) {
  const shapes = pastelShapes(seed)
  artworkCheck(shapes.every(({ from, to }) => same(from.match(/[A-Z]/g), to.match(/[A-Z]/g))), "organic morph preserves compatible path commands")
  const paths = shapes.flatMap(shape => [shape.from, shape.to])
  artworkCheck(paths.length === 6 && paths.every(path => /^M[\d.-]+ [\d.-]+C/.test(path) && path.endsWith("Z") && (path.match(/C/g) ?? []).length >= 5 && !/NaN|Infinity|undefined/.test(path)), "Pastel organic outlines are finite and closed")
}
artworkCheck(!same(pastelShapes("a"), pastelShapes("b")), "different routes vary the Pastel composition")
const markup = renderToStaticMarkup(createElement(model.ThemeStyleBootstrap))
const bootstrap = markup.match(/<script[^>]*>([\s\S]*?)<\/script>/)[1]
const element = { textContent: "" }
globalThis.document = {
  documentElement: { dataset: {}, style: {}, classList: { add() {}, remove() {} } },
  querySelectorAll: () => [element],
}
window.dispatchEvent = () => {}
const failures = []
const check = (ok, name) => { if (!ok) failures.push(name) }
const luminance = hex => hex.slice(1).match(/../g).map(channel => {
  const value = parseInt(channel, 16) / 255
  return value <= .04045 ? value / 12.92 : ((value + .055) / 1.055) ** 2.4
}).reduce((sum, value, i) => sum + value * [.2126, .7152, .0722][i], 0)
const pastelOptions = getPresetOptionCombinations(pastel)
for (const options of [{}, { palette: "retired" }]) {
  check(same(resolvePresetTheme(pastel, options), pastelTheme), "missing or invalid new Pastel palette uses Ink")
}
for (const options of pastelOptions) {
  const resolved = resolvePresetTheme(pastel, options)
  check(same(resolved.style, pastelTheme.style) && resolved.radius === pastelTheme.radius, "Pastel colors keep the same typography and geometry")
  for (const scheme of ["light", "dark"]) {
    const vars = resolved[scheme]
    for (const [foreground, background, minimum] of [
      ["--foreground", "--background", 4.5], ["--muted-foreground", "--muted", 4.5],
      ["--primary-foreground", "--primary", 4.5], ["--accent-foreground", "--accent", 4.5],
      ["--ring", "--background", 3], ["--input", "--background", 3],
      ["--warning", "--background", 4.5], ["--destructive", "--background", 4.5],
    ]) {
      const values = [luminance(vars[foreground]), luminance(vars[background])].sort((a, b) => a - b)
      check((values[1] + .05) / (values[0] + .05) >= minimum, `Pastel ${options.palette} ${scheme} ${foreground} contrast`)
    }
  }
}
check(new Set(pastelOptions.map(options => resolvePresetTheme(pastel, options).light["--background"])).size === pastelOptions.length, "Pastel palettes change the paper, not just the accent")

const atlas = configForPreset(DEFAULT_CONFIG, "atlas")
const overrides = {
  light: { "--background": "#f0e2cc", "--foreground": "#182530", "--primary": "#bb3377" },
  dark: { "--background": "#182530", "--foreground": "#f0e2cc", "--primary": "#aa55cc" },
}
const unsafe = { ...atlas, colorOverrides: {
  light: { "--background": "#F0E2CC", "--foreground": "#123", "--primary": "#123456; } body { color:red", "--unknown": "#112233" },
  dark: { "--primary": "#aa55cc", "--foreground": 123 }, system: { "--primary": "#112233" },
} }
const tuned = { ...atlas, colorOverrides: overrides }
check(same(configForPreset(unsafe, "atlas").colorOverrides, { light: { "--background": "#F0E2CC" }, dark: { "--primary": "#aa55cc" } }), "only known modes/tokens and six-digit hex survive normalization")
check(configForPreset({ ...atlas, colorOverrides: null }, "atlas").colorOverrides === undefined, "legacy/missing colors normalize")
check(same(configForPreset(tuned, "atlas").colorOverrides, overrides), "same preset retains colors")
check(configForPreset(tuned, "beacon").colorOverrides === undefined, "another preset clears colors")
const lightOnly = { ...atlas, colorOverrides: { light: overrides.light } }
const cases = [atlas, tuned, lightOnly, unsafe, ...["tokyo-night", "catppuccin-latte", "white", "nord"].map((palette) => ({
  ...configForPreset(DEFAULT_CONFIG, "omarchy", { palette }), colorOverrides: overrides,
})), configForPreset(DEFAULT_CONFIG, "beacon"),
  ...pastelOptions.map(options => configForPreset(DEFAULT_CONFIG, pastel.id, options)),
  { ...pastelConfig, optionsByPreset: { pastel: {} } },
]
for (const config of cases) {
  store.set(`folioh-theme:${DEFAULT_CONFIG.presetId}`, JSON.stringify(config))
  element.textContent = ""
  runInNewContext(bootstrap, { document, window, localStorage, Event })
  const bootCss = element.textContent
  model.saveThemeConfig(config)
  check(readThemeConfig().presetId === config.presetId, `${config.presetId}: selection is persisted immediately`)
  check(Boolean(bootCss) && bootCss === element.textContent, `${config.presetId}: bootstrap matches client CSS`)
  for (const dark of [false, true]) {
    const tokens = {}
    postcss.parse(bootCss).each((rule) => {
      if (rule.type === "rule" && [":root", dark ? ".dark" : ":root:not(.dark)"].includes(rule.selector)) {
        rule.each((decl) => { if (decl.type === "decl") tokens[decl.prop] = decl.value })
      }
    })
    const vars = model.configVars(readThemeConfig(), dark)
    for (const token of Object.keys(overrides.light)) check(tokens[token] === vars[token], `${config.presetId}/${dark}: CSS and stored preview agree on ${token}`)
  }
}
check(model.configVars(tuned, false)["--background"] === "#f0e2cc", "light preview uses light override")
check(model.configVars(tuned, true)["--primary"] === "#aa55cc", "dark preview uses dark override")
check(model.configVars(lightOnly, true)["--background"] === model.configVars(atlas, true)["--background"], "light override cannot leak into dark")
for (const [config, mode] of [[cases[4], "dark"], [cases[5], "light"]]) {
  for (const dark of [false, true]) check(model.configVars(config, dark)["--primary"] === overrides[mode]["--primary"], "fixed-scheme palette uses its own mode")
}

// Changing the default must preserve choices written under the former key,
// before hydration as well as when the picker reads its initial selection.
const readerDefaultFailures = []
const readerCheck = (ok, message) => { if (!ok) readerDefaultFailures.push(message) }
const savedReader = { ...configForPreset(DEFAULT_CONFIG, "omarchy", { palette: "gruvbox" }), colorOverrides: overrides }
const currentReader = configForPreset(DEFAULT_CONFIG, "pastel", { palette: "sky" })
const oldPastel = { ...currentReader, optionsByPreset: { pastel: {} } }
const readerCases = [
  [[], DEFAULT_CONFIG.presetId, DEFAULT_CONFIG.optionsByPreset[DEFAULT_CONFIG.presetId]?.palette],
  [[["folioh-theme:organic-editorial", savedReader]], "omarchy", "gruvbox"],
  [[["folioh-theme:organic-editorial", savedReader], ["folioh-theme:pastel", currentReader]], "pastel", "sky"],
  [[["folioh-theme:pastel", oldPastel]], "pastel", "jade"],
  [[["folioh-theme", savedReader]], "omarchy", "gruvbox"],
]
for (const [entries, preset, palette] of readerCases) {
  const reset = () => { store.clear(); entries.forEach(([key, value]) => store.set(key, JSON.stringify(value))) }
  reset()
  const client = readThemeConfig()
  readerCheck(client.presetId === preset && client.optionsByPreset[preset]?.palette === palette, `${preset}/${palette}: client retains the right selection`)
  if (entries.length === 0) readerCheck(same(client, DEFAULT_CONFIG), "fresh default retains project tuning")
  if (preset === "omarchy") readerCheck(same(client.colorOverrides, overrides), "migration retains manual colors")
  reset()
  element.textContent = ""
  runInNewContext(bootstrap, { document, window, localStorage, Event })
  const bootCss = element.textContent
  readerCheck(document.documentElement.dataset.foliohPreset === preset, `${preset}/${palette}: bootstrap retains the selection`)
  readerCheck(same(readThemeConfig(), client), `${preset}/${palette}: bootstrap and client read the same config`)
  model.saveThemeConfig(client)
  readerCheck(bootCss === element.textContent, `${preset}/${palette}: no appearance change on hydration`)
}

const transitionFailures = []
const transitionCheck = (ok, message) => { if (!ok) transitionFailures.push(message) }
const originalDocument = document
const originalMatchMedia = window.matchMedia
const originalViewport = { innerWidth: window.innerWidth, innerHeight: window.innerHeight }
const pending = []
globalThis.document = { ...document, hidden: false, startViewTransition: (apply) => {
  let finish
  const finished = new Promise((resolve) => { finish = resolve })
  pending.push(() => { apply(); finish() })
  // Skipped snapshots still run their queued updates.
  return { ready: Promise.reject(new Error("transition skipped")), finished, skipTransition() {} }
} }
const queueTransition = document.startViewTransition
window.matchMedia = () => ({ matches: false })
try {
  const night = configForPreset(DEFAULT_CONFIG, "omarchy", { palette: "tokyo-night" })
  const warm = configForPreset(DEFAULT_CONFIG, "omarchy", { palette: "gruvbox" })
  model.saveThemeConfig(atlas)
  const foliohCss = element.textContent
  model.saveThemeConfig(night)
  model.saveThemeConfig(warm)
  transitionCheck(pending.length === 2 && readThemeConfig().optionsByPreset.omarchy.palette === "gruvbox", "two palette changes queue while the latest choice is saved immediately")
  model.saveThemeConfig(atlas)
  transitionCheck(document.documentElement.dataset.foliohPreset === "atlas" && element.textContent === foliohCss, "leaving Omarchy applies Folioh immediately")
  for (const apply of pending) {
    apply()
    transitionCheck(document.documentElement.dataset.foliohPreset === "atlas" && element.textContent === foliohCss && readThemeConfig().presetId === "atlas", "an older transition cannot restore Omarchy after leaving it")
  }
  delete document.startViewTransition
  model.saveThemeConfig(night)
  transitionCheck(document.documentElement.dataset.foliohPreset === "omarchy" && element.textContent !== foliohCss, "without View Transitions the new palette applies synchronously")
  const nightCss = element.textContent
  document.startViewTransition = () => { transitionFailures.push("reduced motion started a transition"); throw new Error("unexpected transition") }
  window.matchMedia = () => ({ matches: true })
  model.saveThemeConfig(warm)
  transitionCheck(element.textContent !== nightCss && document.documentElement.dataset.foliohPreset === "omarchy", "reduced motion applies a different palette synchronously")

  const { switchScheme } = await load("lib/scheme-transition.ts")
  document.documentElement = { ...document.documentElement, style: { setProperty() {} } }
  window.innerWidth = 1000
  window.innerHeight = 800
  window.matchMedia = () => ({ matches: false })
  document.startViewTransition = queueTransition
  pending.length = 0
  let mode = "system"
  const setTheme = (value) => { mode = value }
  model.saveThemeConfig(pastelConfig)
  switchScheme(setTheme, "dark")
  model.saveThemeConfig(night)
  model.saveThemeConfig(atlas)
  switchScheme(setTheme, "light")
  transitionCheck(mode === "light", "leaving Pastel changes the mode synchronously")
  pending.forEach((apply) => apply())
  transitionCheck(mode === "light" && document.documentElement.dataset.foliohPreset === "atlas", "an older Pastel callback cannot replace a newer Folioh mode")

  model.saveThemeConfig(pastelConfig)
  delete document.startViewTransition
  switchScheme(setTheme, "dark")
  transitionCheck(mode === "dark", "Pastel without View Transitions changes mode synchronously")
  let starts = 0
  document.startViewTransition = () => { starts++; throw new Error("transition unavailable") }
  for (const [nextMode, reduced, hidden] of [["light", true, false], ["dark", false, true], ["system", false, false]]) {
    window.matchMedia = () => ({ matches: reduced })
    document.hidden = hidden
    try { switchScheme(setTheme, nextMode) }
    catch { transitionFailures.push("a failed scheme transition escaped instead of applying its fallback") }
    transitionCheck(mode === nextMode, "reduced, hidden and failed scheme transitions apply synchronously")
  }
  transitionCheck(starts === 1, "only visible unreduced Pastel attempts a native scheme transition")
  await Promise.resolve()
} finally {
  globalThis.document = originalDocument
  if (originalMatchMedia) window.matchMedia = originalMatchMedia
  else delete window.matchMedia
  for (const [key, value] of Object.entries(originalViewport)) {
    if (value === undefined) delete window[key]
    else window[key] = value
  }
}

console.log(JSON.stringify({
  feedbackLinks,
  gallery,
  galleryFailures,
  projectVariants,
  overlayVariants,
  colourControls: Object.fromEntries(presets.map((preset) => [preset.id, colourControls(preset).map((control) => control.id)])),
  leaks,
  radiusLost,
  siteTune: { radiusIndex: DEFAULT_CONFIG.radiusIndex, customization: DEFAULT_CONFIG.customization },
  siteDefaults,
  back: { radiusIndex: back.radiusIndex, customization: back.customization },
  stored: Object.keys(stored).length,
  notReadBack,
  colorOverrideFailures: failures,
  transitionFailures,
  readerDefaultFailures,
  artworkFailures,
  pastel: {
    combinations: getPresetOptionCombinations(pastel).length,
    ring: [pastelTheme.light["--ring"], pastelTheme.dark["--ring"]],
    input: [pastelTheme.light["--input"], pastelTheme.dark["--input"]],
    radius: pastelTheme.radius,
    radiusIndex: pastelConfig.radiusIndex,
    heading: pastelFont.style["--folioh-heading-font-family"],
    body: pastelFont.style["--folioh-body-font-family"],
    siteDefault: DEFAULT_CONFIG.presetId === pastel.id,
  },
}))
"##;

const THEME_CATALOG_DRIVER: &str = r##"
import { register } from "node:module"
import { join } from "node:path"
import { pathToFileURL } from "node:url"
import { runInNewContext } from "node:vm"
const [loader, root, dependencies, projectTheme, extra] = process.argv.slice(2)
register(pathToFileURL(loader).href, { data: { root, dependencies, projectTheme, extraPresets: extra === "true" } })
const load = (path) => import(pathToFileURL(join(root, path)).href)
const model = await load("components/theme-configurator.tsx")
const { listThemes } = await load("components/theme-gallery.tsx")
const { themeVariants } = await load("components/theme-adjustments.tsx")
const { createElement } = await import("react")
const { renderToStaticMarkup } = await import("react-dom/server")
const { DEFAULT_CONFIG, SELECTABLE_PRESETS, configForPreset, readThemeConfig, saveThemeConfig } = model
const store = new Map()
const localStorage = { getItem: (key) => store.get(key) ?? null, setItem: (key, value) => store.set(key, String(value)), removeItem: (key) => store.delete(key) }
globalThis.localStorage = localStorage
globalThis.window = globalThis
window.dispatchEvent = () => {}
const element = { textContent: "" }
globalThis.document = { documentElement: { dataset: {}, style: {}, classList: { add() {}, remove() {} } }, querySelectorAll: () => [element] }
const markup = renderToStaticMarkup(createElement(model.ThemeStyleBootstrap))
const bootstrap = markup.match(/<script[^>]*>([\s\S]*?)<\/script>/)[1]
const restored = Object.fromEntries(["omarchy", "pastel", DEFAULT_CONFIG.presetId, "beacon"].map((id) => {
  const config = configForPreset(DEFAULT_CONFIG, id, id === "pastel" ? { palette: "sky" } : undefined)
  config.customization.fontId = "mono"
  const raw = JSON.stringify(config)
  store.clear()
  store.set(`folioh-theme:${DEFAULT_CONFIG.presetId}`, raw)
  const read = readThemeConfig()
  runInNewContext(bootstrap, { document, window, localStorage, Event })
  const bootPreset = document.documentElement.dataset.foliohPreset
  const bootCss = element.textContent
  const untouched = store.get(`folioh-theme:${DEFAULT_CONFIG.presetId}`) === raw
  saveThemeConfig(config)
  return [id, { read: read.presetId, boot: bootPreset, saved: readThemeConfig().presetId, font: read.customization.fontId, palette: read.optionsByPreset[read.presetId]?.palette, untouched, cssMatches: bootCss === element.textContent }]
}))
console.log(JSON.stringify({
  ids: SELECTABLE_PRESETS.map((preset) => preset.id).sort(),
  families: listThemes().map((family) => family.name),
  variants: themeVariants(SELECTABLE_PRESETS).map(({ preset, option }) => `${preset.id}:${option?.value ?? ""}`),
  defaultId: DEFAULT_CONFIG.presetId,
  defaultFont: DEFAULT_CONFIG.customization.fontId,
  restored,
}))
"##;

/// The theme model's facts, or `None` when Node or the template's
/// frontend dependencies (under `template/` or `.build/`) are absent.
fn theme_model(project_theme: Option<&str>) -> Option<serde_json::Value> {
    theme_driver(project_theme, THEME_MODEL_DRIVER, true)
}

fn theme_driver(
    project_theme: Option<&str>,
    script: &str,
    extra_presets: bool,
) -> Option<serde_json::Value> {
    let template = common::bundled_template();
    if !node_available() {
        return None;
    }
    let dependencies = [
        template.join("node_modules"),
        template.parent()?.join(".build/node_modules"),
    ]
    .into_iter()
    .find(|path| {
        ["typescript", "react", "react-dom", "next-themes", "postcss"]
            .iter()
            .all(|name| path.join(name).join("package.json").exists())
    })?;
    let dir = tempfile::tempdir().unwrap();
    let loader = common::write(dir.path(), "loader.mjs", TEMPLATE_LOADER);
    let driver = common::write(dir.path(), "driver.mjs", script);
    let mut args = vec![
        "--no-warnings".to_string(),
        driver.display().to_string(),
        loader.display().to_string(),
        template.display().to_string(),
        dependencies.display().to_string(),
    ];
    args.push(
        project_theme
            .map(|source| {
                common::write(dir.path(), "project-theme.ts", source)
                    .display()
                    .to_string()
            })
            .unwrap_or_default(),
    );
    args.push(extra_presets.to_string());
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match run_node(&args, dir.path()) {
        Ok(stdout) => Some(serde_json::from_str(stdout.trim()).expect("driver prints json")),
        Err(None) => None,
        Err(Some(stderr)) => panic!("node driver failed:\n{stderr}"),
    }
}

#[test]
fn a_colour_option_changes_only_colour_tokens_and_the_preview() {
    let Some(data) = theme_model(None) else {
        return;
    };
    assert_eq!(data["leaks"], serde_json::json!([]), "{:#}", data["leaks"]);
}

#[test]
fn floating_feedback_preserves_the_destination_and_page_context() {
    let Some(data) = theme_model(None) else {
        return;
    };
    assert_eq!(
        data["feedbackLinks"],
        serde_json::json!([
            "https://github.com/acme/docs/issues/new?title=Feedback%20for%20%E2%80%9CInstall%20%26%20run%E2%80%9D&labels=feedback,docs",
            "https://gitlab.example.com/acme/docs/-/issues/new?issue[title]=Feedback%20for%20%E2%80%9CAPI%E2%80%9D&issue[description]=/label%20~documentation%0A",
            "https://support.example.com/new?from=docs",
            null,
            null,
        ])
    );
}

#[test]
fn a_preset_has_at_most_one_colour_control() {
    let Some(data) = theme_model(None) else {
        return;
    };
    let controls = data["colourControls"].as_object().unwrap();
    let many: Vec<&String> = controls
        .iter()
        .filter(|(_, ids)| ids.as_array().unwrap().len() > 1)
        .map(|(id, _)| id)
        .collect();
    assert!(many.is_empty(), "{many:?}");
    let with_one = controls
        .values()
        .filter(|ids| ids.as_array().unwrap().len() == 1)
        .count();
    assert!(with_one >= 8, "{controls:?}");
}

#[test]
fn a_colour_change_keeps_the_radius_and_the_customization() {
    let Some(data) = theme_model(None) else {
        return;
    };
    assert_eq!(
        data["radiusLost"],
        serde_json::json!([]),
        "{:#}",
        data["radiusLost"]
    );
}

#[test]
fn the_site_preset_picked_again_keeps_the_project_tune() {
    let tune = "export const projectThemePreset = null\nexport const projectThemeDefaultConfig = { radiusIndex: 4, customization: { fontId: \"mono\", rhythmId: \"roomy\", borderId: \"ruled\" } }\n";
    let Some(data) = theme_model(Some(tune)) else {
        return;
    };
    assert_eq!(data["siteTune"]["radiusIndex"], 4);
    assert_eq!(data["siteTune"]["customization"]["fontId"], "mono");
    assert_ne!(data["siteTune"], data["siteDefaults"]);
    assert_eq!(data["back"], data["siteTune"]);
    assert_eq!(data["readerDefaultFailures"], serde_json::json!([]));
}

#[test]
fn stored_colour_options_are_read_back_for_every_theme() {
    let Some(data) = theme_model(None) else {
        return;
    };
    assert!(data["stored"].as_u64().unwrap() >= 8, "{data:#}");
    assert_eq!(data["notReadBack"], serde_json::json!([]));
}

#[test]
fn the_gallery_is_curated_without_filtering_project_replacements() {
    let Some(data) = theme_model(None) else {
        return;
    };
    assert_eq!(
        data["gallery"],
        serde_json::json!([
            "organic-editorial:cobalt",
            "aperture:canvas",
            "stacks:catalog",
            "atlas:wove",
            "workshop:",
            "carbon:tempered",
            "pastel:ink",
            "pastel:jade",
            "pastel:lavender",
            "pastel:peach",
            "pastel:sky",
            "omarchy:catppuccin-latte",
            "omarchy:tokyo-night",
            "omarchy:gruvbox",
            "omarchy:hackerman",
        ])
    );
    assert_eq!(
        data["projectVariants"],
        serde_json::json!(["studio", "console"])
    );
    assert_eq!(data["overlayVariants"], 52);
    assert_eq!(data["galleryFailures"], serde_json::json!([]));
}

#[test]
fn custom_colours_are_validated_per_mode_and_match_the_bootstrap() {
    let Some(data) = theme_model(None) else {
        return;
    };
    assert_eq!(data["colorOverrideFailures"], serde_json::json!([]));
    assert_eq!(data["transitionFailures"], serde_json::json!([]));
}

#[test]
fn theme_artwork_keeps_project_names_and_renders_a_fixed_decorative_field() {
    let Some(data) = theme_model(None) else {
        return;
    };
    assert_eq!(data["artworkFailures"], serde_json::json!([]));
}

#[test]
fn pastel_palettes_share_their_ring_edge_and_type_contract() {
    let Some(data) = theme_model(None) else {
        return;
    };
    let pastel = &data["pastel"];
    assert_eq!(pastel["combinations"], 5, "{pastel:#}");
    assert_eq!(pastel["ring"], serde_json::json!(["#244CB8", "#ACBFF5"]));
    assert_eq!(pastel["input"], serde_json::json!(["#7A839C", "#8797BC"]));
    assert_eq!(pastel["radius"], "0.75rem");
    assert_eq!(pastel["radiusIndex"], 3);
    assert!(pastel["heading"]
        .as_str()
        .unwrap()
        .starts_with("var(--font-bricolage)"));
    assert!(pastel["body"]
        .as_str()
        .unwrap()
        .starts_with("var(--font-dm-sans)"));
    assert_eq!(pastel["siteDefault"], true);
    assert_eq!(data["readerDefaultFailures"], serde_json::json!([]));
}

#[test]
fn the_default_catalog_limits_selection_and_saved_themes() {
    let Some(default) = theme_driver(None, THEME_CATALOG_DRIVER, false) else {
        return;
    };
    assert_eq!(default["ids"], serde_json::json!(["pastel"]));
    assert_eq!(default["families"], serde_json::json!(["Folioh Pastel"]));
    assert_eq!(
        default["variants"],
        serde_json::json!([
            "pastel:ink",
            "pastel:jade",
            "pastel:lavender",
            "pastel:peach",
            "pastel:sky"
        ])
    );
    for key in ["read", "boot", "saved"] {
        assert_eq!(default["restored"]["omarchy"][key], "pastel");
    }
    assert_eq!(
        default["restored"]["omarchy"]["font"],
        default["defaultFont"]
    );
    assert_eq!(default["restored"]["omarchy"]["untouched"], true);
    assert_eq!(default["restored"]["omarchy"]["cssMatches"], true);
    assert_eq!(default["restored"]["pastel"]["palette"], "sky");

    let all = theme_driver(None, THEME_CATALOG_DRIVER, true).unwrap();
    for key in ["read", "boot", "saved"] {
        assert_eq!(all["restored"]["omarchy"][key], "omarchy");
    }
    assert_eq!(all["restored"]["omarchy"]["cssMatches"], true);
    assert_eq!(all["variants"].as_array().unwrap().len(), 15);

    let explicit = theme_driver(Some("export const projectThemePreset = null\nexport const projectThemeDefaultConfig = { presetId: 'beacon' }\n"), THEME_CATALOG_DRIVER, false).unwrap();
    assert_eq!(explicit["ids"], serde_json::json!(["beacon", "pastel"]));
    assert_eq!(explicit["defaultId"], "beacon");
    assert!(explicit["variants"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value.as_str().unwrap().starts_with("beacon:")));
    for key in ["read", "boot", "saved"] {
        assert_eq!(explicit["restored"]["omarchy"][key], "beacon");
    }

    let project = theme_driver(Some(r#"export const projectThemeDefaultConfig = {}
export const projectThemePreset = { id: 'beacon', name: 'Project Beacon', description: '', scene: '', preview: { light: '#123456', dark: '#abcdef' }, defaultOptions: {}, controls: [], resolve: () => ({ preview: { light: '#123456', dark: '#abcdef' }, radius: '0.75rem', style: {}, light: { '--background': '#ffffff', '--foreground': '#123456' }, dark: { '--background': '#123456', '--foreground': '#ffffff' } }) }
"#), THEME_CATALOG_DRIVER, false).unwrap();
    assert_eq!(project["ids"], serde_json::json!(["beacon", "pastel"]));
    assert!(project["families"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "Project Beacon"));
    for key in ["read", "boot", "saved"] {
        assert_eq!(project["restored"]["beacon"][key], "beacon");
    }
}

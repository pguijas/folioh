# Custom Templates

> [!WARNING] Security
> A custom template is **executed as trusted code** during every build. Folioh
> runs `pnpm install --frozen-lockfile` (which executes dependency lifecycle
> scripts) and `next build` (which runs the template's `next.config.mjs` in
> Node), so pointing `template.path` at a directory is equivalent to running
> that code on your machine. Only point `template.path` at frontend code you
> have reviewed and trust. Building an untrusted repository or branch in CI
> inherits the same remote-code-execution surface — treat a docs build like any
> other untrusted-code execution. Never put secrets, API tokens, or credentials
> in `template.params`: they are baked into the static export and published.

Custom templates are the expert escape hatch for teams that need complete
control over their documentation site. A template is a full frontend workspace.
It is not a color-token override system: it owns layout, routes, CSS,
JavaScript, components, package dependencies, search UI, navigation chrome, and
brand behavior.

Use [personalization](/docs/theming/personalization) when the bundled Folioh template is close
enough. Use [theme packages](/docs/theming/theme-packages) when you only need to overlay
selected files on the bundled template. Use
[`template.overlay_path`](#overlay-partial-override) when you want to override
just one or a few template files while inheriting the rest of the bundled
template. Use `template.path` when the documentation must live inside a
product-specific frontend shell.

## Configure a Template

Point Folioh at a local template directory from `docs.yaml`:

```yaml
project:
  name: "Acme SDK"

source:
  python:
    paths:
      - "src/acme"
  docs:
    - "docs/"

template:
  path: "docs-template"
  docs_route_base: "/reference/docs"
  params:
    navbarVariant: "dense"
    productName: "Acme SDK"
    showBetaBadge: true
```

`template.path` is resolved relative to the project directory. It must point to a
trusted local directory inside the project, and it cannot point at `.build/` or
the configured output directory.

`template.params` is an arbitrary JSON-serializable mapping. Folioh does not
interpret these values; the selected template owns their meaning. See the
[`template.params` contract](#templateparams-contract) below for the full rules
and a worked example.

`template.docs_route_base` controls where documentation pages are served. It
defaults to `/docs`. Set it when the generated docs need to live at a product
route such as `/reference/docs`. Folioh uses this path for generated links,
search URLs, sitemap entries, canonical metadata, LLM output, and the copied
Next.js route folder.

## Overlay (partial override)

`template.path` is all-or-nothing: you must fork the **entire** bundled template
to change a single file. When you only need to override one or a few files,
use the opt-in `template.overlay_path` instead.

```yaml
template:
  overlay_path: "template-overlay"
```

In overlay mode Folioh starts from the bundled template, then copies your overlay
files on top — **your files win**, and any file you do not provide falls back to
the bundled template. Only place the files you want to change inside the overlay
directory, mirroring their path in the template tree:

```text
template-overlay/
  components/
    callout.tsx        # replaces the bundled Callout
  app/
    docs/
      layout.tsx       # replaces the bundled docs layout
```

After the overlay is merged, Folioh runs the normal injection pipeline on the
result, so project metadata, the docs route base, search, and theme
configuration are injected exactly as they would be for the bundled template.

> [!IMPORTANT]
> `template.overlay_path` and `template.path` are mutually exclusive.
> `template.path` is a full replacement; `template.overlay_path` is a partial
> override. If both are set, `template.path` wins and the overlay is ignored
> with a warning.

The overlay directory is held to the same guards as `template.path`: it must
live inside the project, may not point at `.build/` or the output directory, and
may not contain symlinks. Because the merged template still has to satisfy the
[injection-marker contract](#injection-markers), any marker-bearing file you
override must keep the required markers verbatim — the safest approach is to copy
the file from the bundled template and edit around the markers.

### Limitations

The overlay is a **file-level** merge, not a deep merge. Overriding a file
replaces it entirely; Folioh does not merge JSON (for example `package.json`) or
splice individual functions inside a file. Deleting a bundled file via the
overlay is not supported — an overlay can only add or replace files. For
deeper changes than a handful of file replacements, fork the whole template with
`template.path`.

## Ownership Model

Folioh owns generated documentation data. Your template owns presentation.

| Owner | Responsibilities |
|-------|------------------|
| Folioh | Parse Python, JavaScript/TypeScript, Rust and Markdown sources, generate MDX pages, write `_meta.ts`, emit API reference pages, write Markdown exports, build the search index, link-check generated routes, and export static files. |
| Template | Render the configured docs route, provide compatible MDX components, style the whole site, implement navigation/search/version UI, ship dependencies, and decide how `template.params` affect the frontend. |

Do not edit `.build/`. Folioh copies the template there, removes template demo
content, injects generated content and metadata, runs the frontend build, and may
overwrite reserved generated files on every build.

## Template Anatomy

Folioh currently supports local Next/Nextra-compatible templates. A custom
template must include a source docs route at `app/docs`; Folioh relocates that
folder to `template.docs_route_base` inside `.build/` when the route base is not
`/docs`.

```text
docs-template/
  package.json
  pnpm-lock.yaml
  next.config.mjs
  mdx-components.tsx
  app/
    layout.tsx
    docs/
      layout.tsx
      [[...mdxPath]]/
        page.jsx
  components/
  content/
  public/
```

The build runtime expects `pnpm install --frozen-lockfile`, `pnpm run build`, and
a static export in `out/`. Keep `app/docs` as the template source route even
when the public route is configured differently.

## Injection markers

Folioh does not template these files with a templating engine. It performs
string substitutions and regex block edits against specific source files after
copying the template into `.build/`. The marker strings below are a **load-bearing
contract**: a custom template must contain them verbatim, in the expected files,
or Folioh cannot inject project metadata, the docs route base, the repository
link, the header, search, the theme preset, dark mode, the base path, or
Folioh-managed MDX component wiring.

> [!IMPORTANT]
> Folioh validates the **load-bearing** markers before building. If a required
> marker (or the file that owns it) is missing, the build fails fast with
> `template.path is missing required Folioh injection markers: <marker> in <file>, ...`,
> one `<marker> in <file>` pair per missing entry (an overlay reports as
> `template.overlay_path`), alongside the existing missing-files and
> MDX-contract checks. The load-bearing set is the
> always-applied project metadata in `app/layout.tsx`,
> `app/docs/layout.tsx`, `app/docs/[[...mdxPath]]/page.jsx`, and the base-path
> marker in `next.config.mjs`.
>
> Optional targets — paired blocks that are stripped when a feature is off, the
> `mdx-components.tsx` component markers (which fall back to a `...components,`
> spread), and markers that only live in optional files such as the landing page
> or `components/theme-configurator.tsx` — are not hard failures: when one is
> absent, that injection does not happen and the build goes on. The safest way
> to author a custom template is to copy the marker-bearing files **verbatim**
> from the bundled template and edit around the markers, rather than recreating
> them from scratch.

### Text replacements

Each of these is a `string → value` replacement. The marker may appear more
than once in a file; all occurrences are replaced.

The project name, description, site URL, repository URL, monogram and the
landing text markers are escaped for where the marker sits, so a name such as
`Acme "Docs" <v2>` reaches the page as text and never as code:

- A marker that is a whole `"…"` or `'…'` string literal becomes a JSON string.
- Inside a longer `"…"` literal it is escaped for that string (HTML-escaped when
  the literal is a JSX attribute value, as in `title="… __PROJECT_NAME__"`).
- Inside a `` `…` `` template literal, backslashes, backticks and `${` are
  escaped.
- Anywhere else it is JSX text and becomes an expression such as
  `{"Acme SDK"}`, so `Built with __PROJECT_NAME__` renders as text.

The `_JSON` markers are pasted as JSON and belong where a JavaScript value is
expected.

| Marker string | Expected file(s) | What Folioh substitutes |
|---------------|------------------|------------------------|
| `__PROJECT_NAME__` | `app/layout.tsx`, `app/docs/layout.tsx`, `app/docs/[[...mdxPath]]/page.jsx`, `app/opengraph-image.tsx`, `app/docs/opengraph-image.tsx`, `app/previews/layout.tsx` | Project name from `docs.yaml`. |
| `__PROJECT_DESCRIPTION__` | `app/layout.tsx`, `app/docs/[[...mdxPath]]/page.jsx`, `app/opengraph-image.tsx` | `Documentation for <name>`. |
| `__SITE_URL__` | `app/layout.tsx`, `app/docs/[[...mdxPath]]/page.jsx`, `app/sitemap.ts`, `app/robots.ts` | Configured site URL (trailing slash trimmed). |
| `__PROJECT_MONOGRAM__` | `app/icon.svg`, `app/docs/layout.tsx`, `app/opengraph-image.tsx`, `app/docs/opengraph-image.tsx`, `app/previews/layout.tsx` | Two-letter lowercase monogram derived from the name. |
| `__PROJECT_REPO__` | `app/docs/layout.tsx`, `app/previews/layout.tsx` | Repository URL (emptied when no repo is set). |
| `__DOCS_INDEX_CANONICAL_PATH__` | `app/docs/[[...mdxPath]]/page.jsx` | Canonical docs index path (`/` when a landing page is enabled, otherwise the docs route base). |
| `__DOCS_ROUTE_BASE__` | `app/sitemap.ts`, `app/robots.ts` | Configured `template.docs_route_base`. |
| `__INCLUDE_DOCS_INDEX__` | `app/sitemap.ts`, `app/robots.ts` | `true`/`false` depending on whether the landing page is enabled. |
| `__FOLIOH_DOCS_ROUTE_BASE__` | `next.config.mjs` | Configured `template.docs_route_base` (the bundled template does not use it). |
| `__LANDING_*__`, `__LANDING_*_JSON__` | `app/page.tsx`, `components/landing-navbar.tsx` | Landing hero/CTA/feature/section data (raw and JSON-encoded forms). |
| `__PROJECT_NAME_JSON__`, `__PROJECT_MONOGRAM_JSON__` | `app/page.tsx`, `components/landing-navbar.tsx` | JSON-encoded name/monogram for the landing page. |
| `__PROJECT_LOGO_JSON__` | `components/landing-navbar.tsx` | JSON-encoded root-relative URL of the `theme.logo` copy (`"/logo.svg"`, percent-encoded, without `deploy.base_path`), or `null` without a logo; the bundled navbar makes it page-relative with `normalizeLandingHref`. |

### Paired block markers

These are `START`/`END` comment pairs that wrap a region. Folioh either replaces
the region's contents (preserving indentation) or strips the whole block — and
its marker lines — when the corresponding feature is not configured. Include both
the start and end marker, and the placeholder content in between, exactly as in
the bundled template.

| Marker pair | Expected file | What Folioh substitutes |
|-------------|---------------|------------------------|
| `// __PROJECT_REPO_IMPORTS_START__` … `// __PROJECT_REPO_IMPORTS_END__` | `app/docs/layout.tsx` | Repository-link imports (block removed when no repo is set). |
| `{/* __PROJECT_REPO_LINK_START__ */}` … `{/* __PROJECT_REPO_LINK_END__ */}` | `app/docs/layout.tsx` | Repository link markup (block removed when no repo is set). |
| `{/* __PROJECT_HEADER_LOGO_START__ */}` … `{/* __PROJECT_HEADER_LOGO_END__ */}` | `app/docs/layout.tsx` | The `theme.logo` image and the project header brand/badge markup (the monogram is dropped when a logo is set; with neither configured only the marker lines are removed and the placeholder between them stays). |
| `// __PROJECT_HEADER_ACTION_IMPORTS_START__` … `// __PROJECT_HEADER_ACTION_IMPORTS_END__` | `app/docs/layout.tsx` | `ProjectHeaderActions` import (removed when no header actions are configured). |
| `{/* __PROJECT_HEADER_ACTIONS_START__ */}` … `{/* __PROJECT_HEADER_ACTIONS_END__ */}` | `app/docs/layout.tsx` | Header action markup (removed when no header actions are configured). |
| `// __FOLIOH_COMPONENT_IMPORTS__` | `mdx-components.tsx` | Reserved for internal Folioh use: import lines Folioh may inject for additional MDX components. |
| `// __FOLIOH_COMPONENT_ENTRIES__` | `mdx-components.tsx` | Reserved for internal Folioh use: entries Folioh may add to the MDX components map. |

The two `mdx-components.tsx` markers are single comment lines (not pairs). They
are reserved for internal Folioh use. If they are absent but Folioh needs to
inject components, it falls back to prepending imports and inserting entries
next to an existing `...components,` spread — so keeping the markers gives you
deterministic placement.

### Troubleshooting

Custom-template failures surface as one of a few recognizable symptoms. Use this
table to map the symptom to its cause and fix.

| Symptom | Cause | Fix |
|---------|-------|-----|
| Build fails listing **missing required files** | The template directory is missing an anatomy file Folioh expects (for example `next.config.mjs`, `mdx-components.tsx`, or the `app/docs` route). | Add the missing files. Copy the [anatomy](#template-anatomy) skeleton from the bundled template rather than recreating it. |
| Build fails listing **missing MDX components** | `mdx-components.tsx` does not export a required Folioh component name, so generated MDX would import a component that does not exist. | Export the named components from `mdx-components.tsx`. See the [MDX Component Contract](#mdx-component-contract). |
| Build fails with **`template.path is missing required Folioh injection markers: X in Y, ...`** (one `marker in file` pair per missing entry) | A load-bearing injection marker — or the file that owns it — is absent, so Folioh cannot inject project metadata, the docs route base, or the base path. | Restore the marker verbatim in the named file. The load-bearing set lives in `app/layout.tsx`, `app/docs/layout.tsx`, `app/docs/[[...mdxPath]]/page.jsx`, and `next.config.mjs`; copy those files from the bundled template and edit around the markers. |
| Warning that the **overlay is ignored** | Both `template.path` and `template.overlay_path` are set. They are [mutually exclusive](#overlay-partial-override); `template.path` wins. | Remove whichever key you do not want. Use `template.path` for a full fork, `template.overlay_path` for a partial override. |
| Config error mentioning an unsafe URL scheme | A header URL uses a rejected scheme (`javascript:`, `data:`), or a repository URL uses a scheme that could execute script or read local files. | Use an `http(s)`, `mailto:`, or relative URL for header links; repository URLs may also use `ssh://`, `git://`, `git+https://`, or scp-style forms. See [header URL validation](/docs/theming/personalization#header-url-validation). |

Optional markers (paired feature blocks, the `mdx-components.tsx` markers, and
markers that only live in optional files such as the landing page or
`components/theme-configurator.tsx`) do **not** fail the build. When one is
absent Folioh skips that injection. Ordinary builds omit skipped-target details,
with one exception: `theme.dark_mode: false` warns when
`components/theme-provider.tsx` lacks its marker. `folioh build --verbose` lists
recorded skipped targets and their reasons, including absent optional files and
files owned by a theme package. If a template change did
not take effect and the build still succeeded, compare the file in `.build/`
with the bundled template: a marker that is still there verbatim, or missing
from both, was never injected.

### Config markers

These are inline `// __MARKER__` comments appended to a real source line. Folioh
replaces the **entire line** (the value plus the marker comment) with the
configured value, so both the placeholder assignment and the trailing marker must
be present verbatim.

| Marker string | Expected file | What Folioh substitutes |
|---------------|---------------|------------------------|
| `const configuredDefaultPresetId = "pastel" // __FOLIOH_THEME_PRESET__` | `components/theme-configurator.tsx` | Rewrites the line to the configured `theme.preset`. |
| `const configuredBasePath = '' // __FOLIOH_BASE_PATH__` | `next.config.mjs` | Rewrites the line to the resolved deploy base path. |
| `const darkModeEnabled: boolean = true // __FOLIOH_DARK_MODE__` | `components/theme-provider.tsx` | Rewrites the line to `false` when `theme.dark_mode` is `false`, so the provider forces the light theme. |

The earlier theme marker with `"organic-editorial"` is still accepted for existing overlays.

`next.config.mjs` is also edited by other substitutions: `contentDirBasePath` is
rewritten to the docs route base, `NEXT_PUBLIC_FOLIOH_DOCS_ROUTE_BASE` is appended
next to `NEXT_PUBLIC_FOLIOH_BASE_PATH`, and the `__I18N_CONFIG__` line is removed.
Localized docs are not available in this release.

## Generated Files

After Folioh copies the template into `.build/`, it writes generated data into
reserved paths:

| Path in `.build/` | Purpose |
|-------------------|---------|
| `content/` | Generated MDX pages and Nextra `_meta.ts` files. |
| `public/_folioh/markdown/` | Plain Markdown versions used by page actions and LLM workflows. |
| `lib/search-index.ts` | Static search document metadata for the navbar search component. |
| `lib/folioh-template.ts` | Build-time project metadata and template params for custom templates. |
| `lib/folioh-mdx-contract.ts` | Versioned list of Folioh MDX component names and prop contracts. |
| `.folioh-manifest.json` | Incremental rebuild cache. |
| `.folioh-build.log` | Last frontend build log. |

## `template.params` contract

`template.params` is the supported channel for passing project-specific
configuration from `docs.yaml` into a custom template. The contract is small and
deliberately strict so that what reaches the frontend is predictable:

- **JSON-serializable.** `template.params` must be a mapping whose values are
  JSON-serializable (strings, numbers, booleans, `null`, lists, and nested
  mappings). Folioh validates this at config-load time. A value JSON cannot
  carry (for example a mapping that uses a list as a key) fails the build with a
  clear `template.params must be JSON-serializable` error.
- **Normalized at load time.** An absent `params` key and an explicit `null`
  both become `{}`. A `params` value that is not a mapping (a list, string, or
  number) is ignored with a warning and also becomes `{}`, so the template still
  builds with empty params instead of crashing.
- **Emitted frozen as `as const`.** Folioh writes the validated mapping verbatim
  into `lib/folioh-template.ts` as `foliohTemplateParams`, serialized with
  `JSON.stringify` and suffixed with `as const`. The `as const` makes the object
  and its string/number/boolean literals read-only at the type level, so editors
  infer literal types (`"dense"`, not `string`).
- **NOT type-checked.** Folioh does not generate or validate a TypeScript type for
  your params. `foliohTemplateParams` is typed only by the literal values that
  happened to be present in the last build. The template owns the meaning of each
  field and is responsible for any narrowing, defaulting, or runtime validation
  it needs. Treat unknown fields as `unknown` and guard before use.

### Worked example

Pass a navbar variant and product name through `docs.yaml`:

```yaml
# docs.yaml
project:
  name: "Acme SDK"

template:
  path: "docs-template"
  params:
    navbarVariant: "dense"
    productName: "Acme SDK"
    showBetaBadge: true
```

Folioh emits the params into `lib/folioh-template.ts` (other exports omitted):

```ts
// .build/lib/folioh-template.ts (generated)
export const foliohTemplateParams = {
  "navbarVariant": "dense",
  "productName": "Acme SDK",
  "showBetaBadge": true
} as const
```

A template component then branches on a field, defaulting when it is absent:

```tsx
import {
  foliohDocs,
  foliohProject,
  foliohTemplateParams,
} from "@/lib/folioh-template"

export function ProductBadge() {
  // `navbarVariant` is inferred as the literal "dense" thanks to `as const`,
  // but Folioh does not guarantee its presence — branch and default explicitly.
  const variant = foliohTemplateParams.navbarVariant ?? "default"
  return (
    <span
      data-docs-route={foliohDocs.routeBase}
      data-variant={variant}
    >
      {foliohTemplateParams.productName ?? foliohProject.name}
      {foliohTemplateParams.showBetaBadge ? " (beta)" : ""}
    </span>
  )
}
```

`template.params` are build-time/static-site visible. Do not put secrets, API
tokens, private keys, or environment-specific credentials in them.

## MDX Component Contract

Generated API and guide pages can reference Folioh component names. A custom
template must expose compatible components from `mdx-components.tsx`, or the
Next.js build will fail when generated MDX imports a missing component.

Folioh writes the current contract to `lib/folioh-mdx-contract.ts` with
`foliohMdxContractVersion`. The build also checks custom templates for required
component names before running the frontend build, so missing required components
fail early.

At minimum, custom templates should provide these names when they render Folioh's
own generated API and guide output:

| Component | Used for |
|-----------|----------|
| `ParamTable` | Function and method parameter tables. |
| `ClassOverview` | Class summaries and inheritance information. |
| `SourceLink` | Links back to repository source files. |
| `ApiReferenceIndex` | Generated API reference landing page. |
| `Callout`, `Tabs`, `TabItem`, `Mermaid` | Converted Markdown/MDX guide content. |

You can re-export Folioh-compatible components, wrap them in product styling, or
replace them entirely. The important contract is the component name and props
that generated MDX expects.

## Sidebar Metadata Contract

Folioh regenerates Nextra `_meta.ts` files on every build. Custom templates may
rely on these stability guarantees:

- Root entries preserve Folioh's documented page order first, then append
  remaining discovered pages.
- Nested directory index pages are emitted as `{ "display": "hidden" }` so the
  folder opens to its index page without duplicating an `Index` item.
- Folders start collapsed: while `sidebar.default_collapsed` is `true` (the
  default) a folder entry is emitted as
  `{ "title": "…", "theme": { "collapsed": true } }`. Set it to `false` and the
  entry is the plain title string. Folioh emits no other collapse state.

## What Default Options Mean

When `template.path` is omitted, Folioh uses the bundled template and applies
Folioh-owned options such as `theme.preset`, `theme.tune`, `theme.dark_mode`,
logo, favicon, Pagefind search, and the default docs layout. A `theme.tune`
value the theme model does not offer always stops the build and names the
nearest valid value. `theme.preset` is checked the same way against the bundled
presets (and a theme package's) unless `template.path` is set.

When `template.path` is set, those options are not a guarantee that Folioh can
style your template. A custom template may choose to read the generated project
metadata, use `template.params`, or ignore Folioh's bundled theme system
entirely. That separation is intentional: expert templates get full control
instead of a partial override layer.

## Compatibility Checklist

Before committing a custom template, verify that it:

- Builds with `pnpm install --frozen-lockfile` and `pnpm run build`.
- Uses Next static export and writes `out/`.
- Renders generated content from `content/` under the configured docs route.
- Supports Nextra `_meta.ts` files if it uses Nextra navigation.
- Exposes Folioh-compatible MDX components from `mdx-components.tsx`.
- Handles `deploy.base_path` or `FOLIOH_BASE_PATH` if the site is deployed under
  a subpath.
- Implements search, dark mode, landing pages, or version and language
  switching itself when those experiences are needed.
- Keeps template source files under the configured template directory.
- Treats `.build/`, `content/`, `public/_folioh/`, `lib/search-index.ts`,
  `lib/folioh-template.ts`, and `lib/folioh-mdx-contract.ts` as generated build
  output.

Custom templates execute frontend install and build scripts. Use templates you
own or trust, review dependency changes, and keep the lockfile checked in with
the template.

---
title: Configuration
description: Configure Folioh source inputs, project metadata, theme options, custom templates, search, LLM files, and deployment settings in docs.yaml.
---

# Configuration

*Complete reference for every option in your `docs.yaml` file.*

## Quick Start

Start with the project name and at least one source input. Folioh needs source
paths or Markdown docs to generate pages:

<ConfigPanel
  title="Minimum docs.yaml"
  description="This is enough for Folioh to scan source, write _site/, and enable the default docs theme."
  fields={[
    { name: "project.name", type: "string", description: "The product name shown in navigation, metadata, and generated pages." },
    { name: "source.python.paths", type: "list[string]", description: "Python packages or modules to scan." },
    { name: "source.docs", type: "list[string]", description: "Markdown guide directories to include." },
  ]}
>
```yaml
project:
  name: "my-library"

source:
  python:
    paths:
      - "src/my_library"
  docs:
    - "docs"
```
</ConfigPanel>

This scans your Python package and Markdown guides, outputs to `_site/`, and enables dark mode.

## Full Reference

Here is every available section and field.

### project

Project metadata shown in the navbar, page titles, and generated output.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `string` | `"Untitled"` | Project name displayed in the navbar and page titles. If missing or empty, a warning is emitted and `"Untitled"` is used. |
| `version` | `string` | `"0.0.0"` | Version string shown in the docs header. Quote it (`version: "1.0"`): an unquoted `1.0` is a YAML number, not a string, and fails the load. |
| `repo` | `string` | `""` | URL to the source repository (e.g. GitHub). Used for source links, the navbar repository link and project metadata. Feedback requires a separate opt-in through `feedback.enabled`. Without a repository, these links are hidden. |
| `repo_ref` | `string` | `"main"` | Branch, tag, or commit used for generated source links. |
| `url` | `string` | `""` | Public site URL for sitemap, canonical metadata, and social previews. It does not control local routing or static asset paths. |

```yaml
project:
  name: "my-library"
  version: "2.0.0"
  repo: "https://github.com/org/my-library"
  repo_ref: "main"
```

### source

Controls where folioh looks for source code and documentation files.

#### source.python

The `python` key can be either a mapping (recommended) or a simple list of paths.

**Mapping form (recommended):**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `paths` | `list[string]` | `[]` | Python package directories, or a conventional `src/` import root. Each path is resolved relative to the project directory. |
| `exclude` | `list[string]` | `[]` | Glob patterns or directory paths to exclude from documentation. Useful for skipping tests, vendored code, or internal modules. |
| `docstring_style` | `string` | `"auto"` | Docstring format to parse. Options: `"auto"`, `"google"`, `"numpy"`. An unknown value warns and falls back to Google. |

<ConfigPanel
  title="source.python"
  description="Use the mapping form when you need exclusions or a specific docstring parser."
  fields={[
    { name: "paths", type: "list[string]", description: "Python source directories to scan." },
    { name: "exclude", type: "list[string]", default: "[]", description: "Glob patterns or directories to skip." },
    { name: "docstring_style", type: "string", default: "auto", description: "Docstring parser: auto, google, or numpy." },
  ]}
>
```yaml
source:
  python:
    paths:
      - "src/my_library"
    exclude:
      - "**/test_*.py"
      - "src/my_library/_internal/"
    docstring_style: "numpy"
```
</ConfigPanel>

When `src/` itself is listed and is not a Python package, Folioh treats it as an
import root. A file such as `src/tools.py` publishes as `tools`, and
`src/my_library/client.py` publishes as `my_library.client`; Folioh does not add a
synthetic `src.` prefix. A path that names a package directly, such as
`src/my_library`, continues to use that directory as the package name.

**List form (shorthand):**

You can also pass a plain list of paths. In this case, no exclude patterns are applied:

```yaml
source:
  python:
    - "src/my_library"
    - "src/my_library_utils"
```

#### source.javascript

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `paths` | `list[string]` | required | Directories with `.js`, `.mjs` or `.cjs` sources. Each path is resolved relative to the project directory. |
| `exclude` | `list[string]` | `[]` | Glob patterns or file/directory paths to skip. |

```yaml
source:
  javascript:
    paths:
      - "web/src"
    exclude:
      - "web/src/vendor"
```

Folioh names a module after its path (`utils/format.js` is `utils.format`, and
an `index.js` takes its directory's name), reads the JSDoc comment above each
export, and publishes what the file exports under `api-reference/javascript/`.
`node_modules` is never read, and a `.jsx` file is skipped with a warning.

#### source.rust

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `paths` | `list[string]` | required | Crate directories, a workspace holding them, or a crate's `src/`. Each path is resolved relative to the project directory. |
| `exclude` | `list[string]` | `[]` | Glob patterns or file/directory paths to skip. |

```yaml
source:
  rust:
    paths:
      - "crates/core/src"
```

Folioh reads each crate's `Cargo.toml` for its name, follows the `pub mod`
declarations from `lib.rs` or `main.rs`, and publishes the public items under
`api-reference/rust/`.

#### source.docs

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `docs` | `list[string]` | `[]` | Directories containing Markdown (`.md`) documentation pages. These are converted to MDX and included in the site alongside the API reference. |

```yaml
source:
  docs:
    - "docs/"
```

`.rst` files are migration inputs, not build inputs. Convert them to Markdown before placing them in `source.docs`; Folioh warns when `.rst` files are present in a docs source directory.

### deploy

Controls deployment-specific path handling. Most sites can omit this section. Use it when the static site is published under a subpath such as `/my-repo`.

Base path priority is:

1. `FOLIOH_BASE_PATH` environment variable.
2. `deploy.base_path` in `docs.yaml`.
3. GitHub Pages inference when `deploy.provider: "github-pages"` or `FOLIOH_DEPLOY_PROVIDER=github-pages` is active in GitHub Actions.
4. No base path.

`folioh serve` stays rooted at `/` unless `FOLIOH_BASE_PATH` is explicitly set.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `provider` | `"github-pages" \| string` | `""` | Enables provider-specific inference. For GitHub Pages project sites, Folioh infers `/{repo}` from `GITHUB_REPOSITORY`; user or organization pages like `owner.github.io` stay at `/`. |
| `base_path` | `string` | `""` | Explicit static asset base path such as `"/docs"` or `"/my-repo"`. Use `"/"` for a root deployment. |

```yaml
deploy:
  provider: "github-pages"
```

For custom domains or reverse proxies, set the base path explicitly:

```yaml
deploy:
  base_path: "/"
```

### versions and i18n

Not available in this release; ignored with a warning. A project can keep
either key in `docs.yaml`: the build warns and carries on without it.
These configuration gates do not reserve documentation routes: authored pages
named `i18n.md` or `versioning.md` publish normally.

### output

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `output` | `string` | `"_site"` | Directory where the generated documentation site is written. Resolved relative to the project directory. Removed by `folioh clean`. |

```yaml
output: "build/docs"
```

Folioh keeps an incremental manifest in `.build/`. The manifest includes source hashes plus config, template, and generator fingerprints, so changing `docs.yaml` or the generator invalidates stale generated pages automatically.

### public

Repository files served verbatim from the site root, such as an installer script or a file that needs a stable URL.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `public` | `list[string]` | `[]` | Project-relative files copied to the root of the built site under their own file name. `folioh serve` serves them from `/` too. |

```yaml
public:
  - "install.sh"
  - "docs/changelog.md"
```

A file that does not exist fails the build with `public file not found: <absolute path>`. A path outside the project directory fails with `public must stay within the project directory`; a path under `.build/` or the output directory fails with `public cannot point inside the .build directory` or `public cannot point inside the output directory`. Folioh's own site serves `install.sh` this way.

### theme

Controls the visual appearance of the generated documentation site.

Generated Pastel social images use the project-name monogram on root and docs
cards, independently of desktop fonts installed on the build host.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `preset` | `string` | `"pastel"` | Default visual preset for the generated site (Folioh Pastel with Ink). `folioh init` offers `organic-editorial`, `beacon`, `atlas`, `workshop`, and `pastel`; it may also be any built-in id, one a theme package or overlay declares, or a new id that `theme.name`, `theme.tokens` or `theme.style` defines. Anything else stops the build and names the nearest id. |
| `extra_presets` | `bool` | `false` | Enable the full bundled theme catalog. By default the picker offers Pastel, the configured site preset and project presets; saved disabled themes fall back to the site default. |
| `dark_mode` | `bool` | `true` | Enable dark mode: the theme picker's Light, Dark and System radios, the toggles `theme.header.theme_toggle` adds, and the `d` key that switches between light and dark themes; `false` keeps the site light, with no mode controls, toggles or `d` shortcut, and `theme.header.theme_toggle` then warns; a dark-only palette the reader applies, such as a dark Omarchy one, still turns the page dark. Must be a YAML boolean; an explicit `null` is an error. |
| `logo` | `string` | `""` | Path to a logo image shown beside the project name in the docs, landing and previews navbars, under `deploy.base_path`. Resolved relative to the project directory; a missing file stops the build. |
| `favicon` | `string` | `""` | Path to a favicon file. Resolved relative to the project directory; a missing file stops the build and reports the requested path. |
| `name` | `string` | `""` | Optional project preset display name. When set with project theme data, Folioh adds a Project group to the theme picker. |
| `description` | `string` | `""` | Optional project preset description. |
| `preview` | `mapping` | `{}` | Optional `light` and `dark` swatch colors for the project preset preview. |
| `radius` | `string` | `""` | Default radius choice for the theme picker. Must be one of `"0"`, `"0.3rem"`, `"0.5rem"`, `"0.75rem"`, or `"1rem"`, or a named alias (`"none"`, `"sm"`, `"md"`, `"lg"`, `"full"`); any other value fails config validation. |
| `tune` | `mapping` | `{}` | Defaults of the theme picker's Customize step, such as font, accent, surface, width, rhythm, borders, code blocks, and radius. |
| `style` | `mapping` | `{}` | Safe layout and typography CSS custom property overrides. |
| `tokens` | `mapping` | `{}` | Safe light/dark CSS variable overrides for shadcn tokens and project tokens. |
| `header` | `mapping` | `{}` | Docs header brand, badge, repository link, search visibility, theme toggle, and project action. |
| `variants` | `mapping` | `{}` | Project-owned theme picker controls with options, swatches, previews, style overrides, and token overrides. Capped at 256 option combinations across all controls (the product of each control's option count); larger products fail config validation. |
| `package` | `string` \| `mapping` | `""` | Theme package copied over the bundled template before generated content and metadata are injected. A string is a directory in the project. A mapping (`git`, `rev`, `digest`, optional `path`) fetches a published package and verifies its tree against the digest. See [Theme Packages](./theming/theme-packages#install-a-package-someone-else-published). |

```yaml
theme:
  preset: "pastel"
  dark_mode: true
  logo: "docs/assets/logo.svg"
  favicon: "docs/assets/favicon.ico"
  tune:
    font: "geist"
    width: "wide"
```

See [Theming](./theming/index) for the ownership model, safe personalization
options, theme packages, and custom templates.

### template

Expert escape hatch for replacing the bundled Folioh frontend with a project-owned
template. Omit this section to use the default Folioh template.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `path` | `string` | `""` | Local Next/Nextra-compatible template directory. Resolved relative to the project directory. Must stay inside the project and cannot point at `.build/` or the output directory. |
| `overlay_path` | `string` | `""` | Directory of files layered on top of the bundled template (overlay files win; missing files fall back to the bundled template). Mutually exclusive with `path`: if both are set, `path` wins and the overlay is ignored with a warning. Held to the same location guards as `path`. |
| `docs_route_base` | `string` | `"/docs"` | Public route where generated documentation pages are served. Folioh rewrites generated links, search URLs, sitemap entries, canonical metadata, LLM output, and the copied Next.js docs route to this path. |
| `params` | `mapping` | `{}` | Arbitrary JSON-serializable values exposed to the template as build-time data. Folioh does not interpret these values. |

```yaml
template:
  path: "docs-template"
  docs_route_base: "/reference/docs"
  params:
    navbarVariant: "dense"
    productName: "Acme SDK"
```

Custom templates own layout, CSS, JavaScript, package dependencies, routing
chrome, search UI, and the meaning of `template.params`. Folioh still owns the
generated MDX content, `_meta.ts` files, search index, Markdown exports, and
static export pipeline. See [Custom Templates](./theming/custom-templates) for
the full contract and required file structure.

### search

The 404 page offers the same search when its index is available. Setting
`search.enabled: false` also hides this control; missing pages remain noindex.

Controls the built-in full-text search powered by [Pagefind](https://pagefind.app). Search is enabled by default, appears in the docs navbar, and opens with `Cmd+K` on macOS or `Ctrl+K` on Windows/Linux.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | `true` | Enable or disable the navbar search. Set to `false` to hide search entirely. Must be a YAML boolean; an explicit `null` is an error. |
| `placeholder` | `string` | `""` | Custom placeholder text for the search input. When empty, the default "Search documentation..." is used. |

```yaml
search:
  enabled: true
  placeholder: "Search documentation..."
```

To disable search completely:

```yaml
search:
  enabled: false
```

### nav

An ordered list of strings defining the top-level sidebar sections. `Guide`
keeps all authored pages together in their normal order. `API Reference` and
`Source Code` both place the generated source tree, which keeps the stable
`/api-reference/` route. Other labels order a matching top-level page or folder;
unknown labels are ignored rather than creating dead routes.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `nav` | `list[string]` | `[]` | Top-level navigation sections. |

```yaml
nav:
  - "Introduction"
  - "Getting Started"
  - "Source Code"
  - "Changelog"
```

### feedback

Feedback is disabled by default. A repository link alone does not enable it.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | `false` | Show the Feedback button on documentation pages. Requires `project.repo` pointing to a GitHub or GitLab repository. |

```yaml
project:
  name: "my-library"
  repo: "https://github.com/org/my-library"

feedback:
  enabled: true
```

The floating message icon opens a new issue with the current page title. Its
Feedback tooltip appears on hover or keyboard focus. The circular control is
36px on desktop and keeps a 44px target on mobile and touch screens. It follows
the active theme; reduced-motion preferences disable its hover animation.
Omitting `feedback`, using an empty mapping, or setting `enabled: false` hides
both the floating control and the table-of-contents feedback action. A
configuration change requires restarting `folioh serve`.

### sidebar

Controls generated sidebar section behavior. Generated groups start collapsed by
default (`sidebar.default_collapsed: true`). Set `default_collapsed: false` if you
want generated groups to start expanded.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `default_collapsed` | `bool` | `true` | Render generated folder entries with Nextra's `open: false` state so sidebar sections start collapsed. Leaf pages are unchanged. Set to `false` to expand generated groups. Must be a YAML boolean; an explicit `null` is an error. |

```yaml
sidebar:
  default_collapsed: false # expand generated groups instead of collapsing them
```

By default, Folioh emits object entries in generated `_meta.ts` files for folders:

```ts
export default {
  "components": {
    "title": "Components",
    "theme": { "collapsed": true },
  },
}
```

This applies to generated guide folders and API reference folder groups. It does not
change the route for folder index pages, and it does not collapse individual leaf
pages.

#### Directory page order

Put `_meta.yaml` beside a directory’s pages to order its immediate children:

```yaml filename="docs/advanced/_meta.yaml"
order:
  - index
  - quickstart
  - configuration
  - examples
```

Use child route names without `.md`, such as `quickstart` or the folder name
`examples`. `README` refers to `index`. A nested folder owns its own order and
does not need an index page. Names listed here come first; other children keep
their deterministic discovery order. Missing future pages are skipped.

Titles still come from frontmatter or the first H1. Unlisted pages and nested
index pages stay hidden. Metadata can also order plugin pages and
generated API folders by placing the file at the corresponding route directory
under a `source.docs` root, for example `docs/api-reference/mylib/_meta.yaml`.
The root directory’s order applies before `docs.yaml` `nav`.

`build` rereads these files on warm builds. `serve` applies edits, additions and
removals without restarting. Invalid metadata reports its source filename;
unknown keys, non-string or duplicate entries, paths and Markdown file extensions are
errors. Two documentation roots declaring the same route directory’s order are
also an error.

### llm

Controls generation of [llms.txt](https://llmstxt.org/) files for AI consumption.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `generate_llms_txt` | `bool` | `true` | Generate an `llms.txt` summary file in the output directory. This is a condensed index of your documentation for LLMs. |
| `generate_llms_full_txt` | `bool` | `true` | Generate an `llms-full.txt` file containing the full text of all documentation pages. |

Both keys must be YAML booleans; an explicit `null` is an error.

```yaml
llm:
  generate_llms_txt: true
  generate_llms_full_txt: false
```

Every page is also written as plain Markdown under `_folioh/markdown/`, linked from
the page head as a `text/markdown` alternate and listed in the sitemap. These mirrors
are lossy on purpose: complex component props, such as an entire data table, do
not survive. Prose, headings, lists, code blocks, Mermaid source, component
children, and useful labels from simple cards do. Treat a mirror as the page's
portable text, not as a pixel-equivalent copy of the page.

### components

Every entry resolves relative to the project directory and must stay inside
it, directory or spec `from`/`path` alike: a component is trusted frontend code,
held to the rule `theme.package` already obeys.

Custom React components to include in the generated site. Registered components are copied into Folioh's generated component namespace and exposed to MDX without requiring manual imports in Markdown pages. This is how a site adds its own components.

Each list entry is either a **directory path** or a **named component spec**:

| Entry form | Type | Description |
|------------|------|-------------|
| Directory | `string` | Path to a directory, relative to the project directory. Every top-level `.tsx`/`.jsx` file in it is registered as a component named after its PascalCased file stem (`hero.tsx` → `Hero`, `my-chart.tsx` → `MyChart`). Each file must have a named export matching the derived name. |
| Named spec | `mapping` | A single component file with an explicit name, source path, and options. |

Named spec fields:

| Field | Type | Description |
|-------|------|-------------|
| `name` | `string` | JSX name exposed to MDX. |
| `from` | `string` | Path to the `.tsx` or `.jsx` component file, relative to the project directory. |
| `export` | `string` | Export name in the source file. Defaults to `name`. |
| `expose.mdx` | `bool` | Make the component available in docs MDX. Defaults to `true`. |

```yaml
components:
  - "docs/components"          # every top-level .tsx/.jsx in the directory
  - name: "Hero"
    from: "docs/widgets/hero.tsx"
    export: "Hero"
    expose:
      mdx: true
```

Validation is loud: a directory that does not exist fails the build, a directory without any `.tsx`/`.jsx` files produces a warning, and a `from:` file that does not exist fails the build when components are copied into the workspace. If two source files share the same filename stem, Folioh generates distinct component import paths automatically.

A malformed value — a non-list value (`components: "docs/components"` must be `components: ["docs/components"]`), or a list entry that is neither a path string nor a `{name, from}` mapping — fails config loading for every command (`folioh build`, `folioh serve`, ...).

A component whose name matches a Folioh builtin (for example `Callout`) replaces the builtin and emits a warning; two `components:` entries (or a config entry and a component of a built-in plugin) with the same name raise an error.

### Built-in integrations

The [landing page](./plugins/landing), the [roadmap](./plugins/roadmap) and OpenAPI ship in the `folioh` binary; [Plugins](./plugins/index) covers each. Each one activates through its own top-level section in `docs.yaml` and stays inert without it; there is no `plugins:` list.

```yaml
roadmap:
  phases:
    - id: "foundation"
      version: "0.1"
      title: "Foundation"
      status: "shipped"
      layer: "Source analysis"
      summary: "Source in, documented site out."

openapi:
  sources:
    - path: "openapi.yaml"
```

Project plugins loaded from your own repository: Not available in this release.

## Path Resolution

In CLI commands, relative paths in `docs.yaml` are resolved from the active project directory. When you run `folioh build /path/to/project` or `folioh build --project-dir /path/to/project`, that directory becomes the base for config paths.

For example, given this structure:

```
my-project/
  docs.yaml
  src/
    my_lib/
      __init__.py
  docs/
    index.md
```

The config `source.python.paths: ["src/my_lib"]` resolves to `/path/to/my-project/src/my_lib`.

## Config Validation

folioh validates your config file. A few problems stop the load; everything else is a warning, and the build proceeds with defaults where possible:

- **Duplicate keys** anywhere in the file fail the load.

- **Typed keys** are checked. String keys such as `project.version` must be quoted YAML strings (`version: "1.0"`); boolean keys (`theme.dark_mode`, `theme.extra_presets`, `llm.*`, `search.enabled`, `feedback.enabled`, `sidebar.default_collapsed`) must be YAML booleans. An explicit `null` on a typed key is an error.

- **Missing config file** stops the command with an error naming the path that was expected.

- **Unknown top-level keys** produce a warning listing the unrecognized keys. Core top-level keys are: `project`, `source`, `output`, `public`, `theme`, `template`, `nav`, `sidebar`, `llm`, `search`, `feedback`, `components`, and `deploy`, plus the sections of the built-in integrations: `landing`, `roadmap`, and `openapi`. `versions` and `i18n` are recognised and [ignored with a warning](#versions-and-i18n).

- **Unknown keys inside a core section** (`project`, `source`, `theme`, `template`, `llm`, `deploy`, `sidebar`, `search`, `feedback`, `components`) produce a warning too, naming the nearest valid key when one is close: `Unknown project keys in docs.yaml: vesion (did you mean 'version'?)`. The `landing`, `roadmap` and `openapi` sections are not checked in this release: a typo there is ignored without a warning. An unknown `source.python.docstring_style` warns and falls back to Google.

- **The `plugins:` key** warns with exactly `project plugins are not available in this release; the plugins key is ignored`. The `configKeys` list in the authoring contract (`/_folioh/contract.json`) never includes `plugins`.

- **Missing or empty `project.name`** produces a warning and defaults to `"Untitled"`.

## Common Patterns

### Monorepo Setup

When your Python package lives in a subdirectory of a larger repository:

```yaml
project:
  name: "my-service"
  repo: "https://github.com/org/monorepo"

source:
  python:
    paths:
      - "packages/my-service/src/my_service"
    exclude:
      - "**/test_*.py"
      - "**/conftest.py"
  docs:
    - "packages/my-service/docs/"

output: "packages/my-service/_site"
```

### Excluding Test Files

Keep test files out of your API reference:

```yaml
source:
  python:
    paths:
      - "src/"
    exclude:
      - "**/test_*.py"
      - "**/tests/"
      - "**/conftest.py"
      - "src/my_lib/_fixtures/"
```

### Custom Output Directory

Write the built site to a specific location for CI/CD deployment:

```yaml
output: "build/site"
```

### Multiple Python Source Directories

Document code spread across multiple directories:

```yaml
source:
  python:
    paths:
      - "src/core"
      - "src/plugins"
      - "src/utils"
```

### Minimal Docs-Only Site

If you just want to serve Markdown documentation without Python API reference:

```yaml
project:
  name: "My Docs"

source:
  docs:
    - "docs/"

nav:
  - "Guide"
  - "FAQ"
```

The component catalog and default landing links follow `template.docs_route_base`.
The default API entry appears when a generated reference index is published;
explicitly configured landing routes keep their destinations. Relative guide
links are checked against the same file-relative paths used in rendered pages.

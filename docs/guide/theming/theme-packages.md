---
title: Theme Packages
description: Use theme.package to overlay project-owned theme files on top of Folioh's bundled template.
---

# Theme Packages

> [!WARNING] Security
> A theme package is **executed as trusted code** during every build. The
> package files are overlaid onto the template that Folioh then builds with
> `pnpm install --frozen-lockfile` (which executes dependency lifecycle scripts)
> and `next build` (which runs `next.config.mjs` in Node), so pointing
> `theme.package` at a directory is equivalent to running that code on your
> machine. Only point `theme.package` at frontend code you have reviewed and
> trust. Building an untrusted repository or branch in CI inherits the same
> remote-code-execution surface — treat a docs build like any other
> untrusted-code execution. Never put secrets, API tokens, or credentials in
> theme code or `docs.yaml`: static docs are published.

Theme packages are the middle ownership level. They let a project ship a theme
overlay, local or fetched from git, that is copied over the bundled Folioh
template before generated content and metadata are injected.

Use a theme package when YAML personalization is too limited, but a full custom
template would duplicate too much of Folioh's default docs runtime.

Folioh’s own site uses this layer for its Docs/Board product selector. Its
landing passes a project-owned component into `LandingNavbar`’s
`productSwitcher` slot. The bundled navbar keeps its original monogram and
theme gallery, and other sites receive no product links unless their own
layout supplies the slot.

## Configure a Package

```yaml
theme:
  package: "docs/theme/acme"
  preset: "acme"
```

`theme.package` is resolved relative to the project directory. The directory is
trusted frontend code: it can override template files, import dependencies
already available to the template, and replace Folioh's default theme modules.

## Install a Package Someone Else Published

`theme.package` also takes a mapping, which is how a theme you did not write
reaches your project without copying a directory into it:

```yaml
theme:
  package:
    git: "https://github.com/acme/folioh-theme"
    rev: "v1.2.0"
    digest: "sha256:9f2c…"
```

| Key | Required | Purpose |
|-----|----------|---------|
| `git` | yes | Repository to fetch: `https://`, `ssh://`, `git@host:owner/repo` or `file://`. There is no plain `http`. |
| `rev` | yes | Tag, branch or commit to read it at. |
| `digest` | yes | `sha256:` and 64 hex characters: the digest of the package tree. |
| `path` | no | Subdirectory of the repository that holds the package; the root by default. |

Folioh fetches the revision into the theme cache, one directory per digest under
`<cache>/folioh/themes/`, hashes the tree it got, and refuses to use it unless
the hash is the one you pinned. `FOLIOH_THEME_CACHE_DIR` points that cache
somewhere else, which is how a CI job pre-seeds it and how an air-gapped
machine fills it by hand. The build
then overlays it exactly as it overlays a local package. The second build reads
the cache and makes no network call.

### The digest is the pin, not the revision

A tag can be moved and a branch always moves. The digest cannot: it is the
SHA-256 of every file in the package, so the same four lines of `docs.yaml`
produce the same theme on every machine and in CI. When the hash does not
match, the build fails and names both:

```text
theme.package https://github.com/acme/folioh-theme at v1.2.0 does not match its digest.
  expected sha256:9f2c…
  found    sha256:41ab…
Update theme.package.digest if the change is one you reviewed.
```

Take a new digest by reviewing the change and reading the value out of that
message. Folioh never updates the pin for you. Pinning a package for the first
time works the same way: put any 64 hex characters in `digest`, run the build,
and copy the `found` value once you have read what you are installing.

### What a fetched package may not own

A theme package is executed as trusted code, and a fetched one was written by
someone else. Two rules follow. Every package, local or fetched, is refused if
it ships `package.json`, `pnpm-lock.yaml` or `pnpm-workspace.yaml`, because the
frontend build installs against Folioh's own pinned lockfile. A fetched package
is refused if it ships `next.config.mjs`: owning the Next config is the
shortest path from a one-line change in `docs.yaml` to arbitrary code in your
build, and it stays a decision a project makes about itself.

Everything else the digest covers: you are trusting a tree you pinned, not a
repository that can change under you. Fetching needs `git` on the machine.

## Ownership Model

| Owner | Responsibilities |
|-------|------------------|
| Folioh | Copy the bundled template, apply the theme package overlay, generate content, write metadata, inject fallback theme config, run the frontend build, and export static files. |
| Theme package | Own selected frontend files such as layouts, global CSS, the theme model behind the picker, project header actions, or `theme/project-theme.ts`. |

Folioh still writes generated docs content and reserved build files into `.build/`.
Do not edit `.build/` directly; change the package source instead.

## Package Anatomy

A package can be small and only override the files it owns:

```text
docs/theme/acme/
  app/
    globals.css
    docs/
      layout.tsx
  components/
    theme-configurator.tsx
    project-header-actions.tsx
  theme/
    project-theme.ts
```

Common override points:

| File | Purpose |
|------|---------|
| `app/layout.tsx` | Own font loading, metadata shell, providers, and global layout behavior. |
| `app/docs/layout.tsx` | Own the docs navbar, sidebar placement, search slot, and ThemeGallery mount. |
| `components/theme-configurator.tsx` | Replace the bundled theme model, its defaults and its bootstrap with a project-specific implementation. |
| `components/project-header-actions.tsx` | Replace the default generated header action component. |
| `theme/project-theme.ts` | Own all presets, controls, variants, defaults, and resolved tokens in TypeScript. |
| `app/globals.css` | Override or replace template-level CSS when a project needs exact visual parity. |
| `app/styles/<family>.css` | Restyle one component family (`shell`, `callout`, `disclosure`, `code`, `shapes`, `surfaces` or `components`) without copying `globals.css`, which imports these files last. |

A package that replaces `components/theme-provider.tsx` has to keep the
`const darkModeEnabled: boolean = true // __FOLIOH_DARK_MODE__` line for
`theme.dark_mode: false` to take effect. Without it the build warns and dark
mode stays available.

## YAML and TypeScript Together

If the package supplies `theme/project-theme.ts`, Folioh does not overwrite it
with a YAML-generated module. The package owns the preset implementation.

If the package omits `theme/project-theme.ts`, Folioh still emits the safe
`docs.yaml`-driven project preset described in
[Personalization](./personalization). This lets a project start with YAML and
graduate individual surfaces to TypeScript only when needed.

## Register a Custom Preset

A theme package can register a custom preset without forking `presets.ts`. From
`theme/project-theme.ts`, import `registerPreset` and call it before exporting
the `projectThemePreset`.

```typescript
import type { ThemePreset } from "./preset-types"
import { registerPreset } from "./preset-registry"

const acmePreset: ThemePreset = {
  id: "acme",
  name: "Acme",
  description: "Custom preset for Acme's design system",
  scene: "A developer browses API documentation with Acme brand colors.",
  preview: { light: "oklch(0.50 0.10 210)", dark: "oklch(0.70 0.08 210)" },
  defaultOptions: {},
  controls: [],
  resolve: (options) => ({
    preview: { light: "oklch(0.50 0.10 210)", dark: "oklch(0.70 0.08 210)" },
    radius: "0.5rem",
    style: {
      /* ThemeStyle fields */
    },
    light: {
      /* ThemeVars for light mode */
    },
    dark: {
      /* ThemeVars for dark mode */
    },
  }),
}

registerPreset(acmePreset, "project")

export const projectThemePreset = acmePreset
export const projectThemeDefaultConfig = {
  /* Default config object */
}
```

The `registerPreset` function is the supported extension point. The optional
second argument (`"project"`) adds the preset to that display group in the
theme picker. If a preset with the same `id` already exists, it will be
replaced with a console warning.

`theme.preset` may name a preset the package declares. Folioh reads the
`id: "…"` values in the `.ts` and `.tsx` files under the package's `theme/`
directory and accepts those ids next to the bundled ones, so `preset: "acme"`
above builds. Any other id stops the build with the list of valid ids and the
nearest one. A package that ships its own `components/theme-configurator.tsx`
owns preset selection, and Folioh does not check `theme.preset` then.

The theme picker and the root layout import from
`components/theme-configurator.tsx`, so a package that replaces it keeps these
exports: `ThemeStyleBootstrap`, `DEFAULT_CONFIG`, the `ThemeConfig` type,
`configForPreset`, `configVars`, `getPresetDefaults`, `readThemeConfig`,
`saveThemeConfig`, `previewThemeConfig`, and the option lists `fontOptions`,
`colorOptions`, `surfaceColorOptions`, `shellPaddingOptions`, `rhythmOptions`,
`borderOptions`, `codeTreatmentOptions` and `radiusOptions`. `ThemeConfigurator`
renders nothing and stays exported for layouts that still mount it.

The root layout mounts `ThemeStyleBootstrap` once for every page, so a
package's page or layout needs no copy of its own; a copy it still carries
re-applies the stored theme and changes nothing else.

See [Theme picker](../components/theme-configurator) for the full
`ThemePreset` contract, including `controls`, `resolve`, and all theme fields.

## What Packages Should Not Do

- Do not assume files written under `.build/` are stable source files.
- Do not put secrets in theme code or `docs.yaml`; static docs are public.
- Do not use a package when the project needs a different app structure,
  dependency graph, route model, or product shell. Use
  [custom templates](./custom-templates) for that level.

## Validation Checklist

Folioh validates theme packages at build time before any overlay. If validation
fails, the build halts with one error listing all violations.

### Reserved Paths

A theme package must NOT contain any of these paths. Folioh generates them at
build time, and including them will cause a validation error:

- `content/` — Generated docs pages and metadata live here
- `lib/folioh-template.ts` — Folioh internal contract file
- `lib/folioh-mdx-contract.ts` — Folioh internal contract file
- `theme/theme-contract.generated.ts` — Folioh internal contract file
- `.next/` — Next.js build cache
- `node_modules/` — Dependency install directory
- `package.json` — The frontend installs against Folioh's own manifest
- `pnpm-lock.yaml` — The install is `--frozen-lockfile` against Folioh's lockfile
- `pnpm-workspace.yaml` — Same reason
- `next.config.mjs` — Fetched packages only; a local package may own it


### Required Exports

If a theme package includes `theme/project-theme.ts`, that file MUST export both:

- `projectThemePreset` — The preset definition (colors, spacing, variants)
- `projectThemeDefaultConfig` — The default configuration object

Folioh checks for `export const <name>` or `export { <name> }` forms. Missing
exports fail validation.

### Example Validation Error

```
Theme package validation failed:
  - Theme package must not contain reserved path 'content/'. Folioh generates this at build time.
  - theme/project-theme.ts must export 'projectThemePreset'. Expected: export const projectThemePreset = ...
```

### Why This Fails Early

Validation runs before any file overlay. You get one clear error with
actionable messages instead of a cryptic Next.js build failure or runtime
import breakage.

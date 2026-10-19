---
title: Personalization
description: Configure Folioh's bundled docs theme with presets, project tokens, variants, header branding, logo, favicon, and dark mode.
---

# Personalization

Personalization keeps the bundled Folioh template and changes the theme data it
receives. This is the right level when the docs should still feel like Folioh,
but with your project's brand, typography, accent, spacing, and header defaults.

## Basic Theme Options

```yaml
theme:
  preset: "pastel"
  extra_presets: false
  dark_mode: true
  logo: "docs/assets/logo.svg"
  favicon: "docs/assets/favicon.ico"
```

| Field | Purpose |
|-------|---------|
| `preset` | Default visual preset for the generated site; omitted, Folioh Pastel with Ink. |
| `extra_presets` | `false` by default: Pastel, the configured site preset and project presets remain selectable. Set `true` to enable the full bundled catalog. |
| `dark_mode` | `true` (the default) offers light, dark, and system modes through `next-themes`; `false` keeps the site light, unless the reader applies a dark-only palette such as a dark Omarchy one. |
| `logo` | Copies a project logo into the generated site and shows it beside the project name in the docs and landing navbars. |
| `favicon` | Copies a project favicon into the generated site. |

`preset` must name a
[built-in id](../components/theme-configurator#preset-library), one a
[theme package](./theme-packages#register-a-custom-preset) declares, or a new id
that `theme.name`, `theme.tokens` or `theme.style` defines (see
[Project Presets](#project-presets)). Any other value stops the build with the
list of valid ids and the nearest one, so `preset: "beakon"` asks whether you
meant `beacon`.

The default picker offers Pastel and its five palettes. Extra bundled themes require `theme.extra_presets: true`. A reader’s saved disabled theme falls back to the configured site preset before the page appears. Folioh’s own site keeps extra presets disabled.

Dark mode is enabled by default. When enabled, readers can use the mode radios in
the theme picker or press `d` outside form fields to switch between light and dark
modes. `theme.header.theme_toggle: true` adds a one-click light/dark toggle to the
landing and the docs navbars alike; without it neither navbar has one. With
`dark_mode: false` the site stays light whatever the reader's system or saved
preference: the picker shows no mode control, no toggle is rendered, and `d` does
nothing. `theme.header.theme_toggle: true` is then ignored with a warning. A
dark-only palette the reader applies still turns the page dark while it is
applied.

The logo path is resolved from the project directory and must exist; a missing
file stops the build. The image takes the monogram's place in the docs navbar
and the landing navbar, and is served under `deploy.base_path` when one is set.
The previews page keeps the monogram.

## Theme Picker

Every generated site includes the theme picker on the landing page, the docs and
the previews. The palette button in the navbar, or the `t` key, opens it: every
theme as a slide drawn in its own colors, the front theme's colors as swatches
under it, and light, dark and system mode in its header unless
`theme.dark_mode` is `false`. The carousel walks six Folioh styles, five Folioh Pastel palettes and
four Omarchy palettes. Arrows, swipes and swatches apply and save each choice
immediately. Done or Escape closes the picker and keeps the selected theme.

Customize, or the `c` key, opens the picker's second step
for the front theme: its own options, typography, reading rhythm, corners,
borders, code block frame, page frame and background, text and accent colors.
Each change updates the page and saves immediately.

Reader preferences are persisted in a project-scoped `localStorage` key derived
from the configured default preset. When that default changes from Roller to
Folioh Pastel, existing reader choices are reused unless the new key already
exists. Folioh also renders the default theme CSS into
the page before hydration so the site does not flash through unconfigured
typography or colors.

See [Theme picker](../components/theme-configurator) for the picker's keys, the
built-in preset catalog and the TypeScript preset contract.

## Tune Defaults

Use `theme.tune` to set the defaults of the picker's Customize step without
creating a new preset. They are defaults only and restrict nothing:

```yaml
theme:
  preset: "beacon"
  tune:
    font: "geist"
    accent: "laurel"
    surface: "paper"
    width: "wide"
    rhythm: "compact"
    borders: "fine"
    code: "terminal"
    radius: "0.3rem"
```

Common aliases map to the internal configurator ids:

| YAML key | Internal control |
|----------|------------------|
| `font` | `fontId` |
| `accent` or `color` | `colorId` |
| `surface` | `surfaceColorId` |
| `shell` | `shellPaddingId` |
| `width` or `content_width` | `contentWidthId` |
| `rhythm` or `reading` | `rhythmId` |
| `borders` or `border` | `borderId` |
| `code` or `code_blocks` | `codeTreatmentId` |

Each value must be one the configurator offers for that control:

| Control | Values |
|---------|--------|
| `fontId` | `folioh`, `sans`, `geist`, `serif`, `mono`, `terminal`, `grotesque` |
| `colorId` | `ink`, `laurel`, `indigo`, `copper` |
| `surfaceColorId` | `preset`, `paper`, `moss`, `mist` |
| `shellPaddingId` | `preset`, `flush`, `frame`, `gallery` |
| `contentWidthId` | `preset`, `focus`, `docs`, `wide` |
| `rhythmId` | `preset`, `compact`, `balanced`, `roomy` |
| `borderId` | `preset`, `fine`, `structured`, `ruled` |
| `codeTreatmentId` | `preset`, `soft`, `framed`, `plate`, `terminal` |

Any other value stops the build with the list and the nearest value, for example
`theme.tune.rhythm must be one of 'preset', 'compact', 'balanced', 'roomy'; got
'dense'`. A key that is none of the above warns, names the key it most likely
meant, and is ignored.

`font: "geist"` selects the bundled Geist and Geist Mono font pair and maps the
public `--font-sans` and `--font-mono` tokens used by Tailwind utilities.

`theme.radius` (and its `theme.tune.radius` alias) is validated against the
fixed radius scale: `"0"`, `"0.3rem"`, `"0.5rem"`, `"0.75rem"`, or `"1rem"`.
The named aliases `"none"`, `"sm"`, `"md"`, `"lg"`, and `"full"` map onto the
same scale (matching the picker's Corners labels). Any other value fails
config validation, because the configurator maps the configured radius onto
this fixed scale.

## Project Presets

When a project supplies `name`, `description`, `scene`, `preview`, `style`,
`tokens`, or `variants`, Folioh emits `theme/project-theme.ts` during template preparation and
adds the project preset before the built-in preset library.

<ConfigPanel
  title="docs.yaml"
  description="A project-owned theme preset generated from safe YAML data."
  fields={[
    { name: "theme.name", type: "string", description: "Label shown in the theme picker's Project group." },
    { name: "theme.preview", type: "object", description: "Light and dark swatches for the preset preview." },
    { name: "theme.style", type: "object", description: "Layout and typography CSS custom properties." },
    { name: "theme.tokens", type: "object", description: "Light and dark shadcn/project CSS variable overrides." },
    { name: "theme.header", type: "object", description: "Docs header brand, badge, repo, search, and action controls." },
    { name: "theme.variants", type: "object", description: "Project-owned preset controls with options, swatches, and token overrides." },
  ]}
>
```yaml
theme:
  preset: "acme"
  name: "Acme"
  description: "Operational docs theme"
  preview:
    light: "oklch(0.64 0.12 155)"
    dark: "oklch(0.78 0.14 155)"
  tune:
    font: "geist"
    width: "wide"
    rhythm: "compact"
    radius: "0.3rem"
  header:
    brand: "Acme"
    badge: "Docs"
    repo: "https://github.com/acme/sdk"
    search: true
    theme_toggle: true
    action_label: "Dashboard"
    action_href: "https://app.acme.dev"
  tokens:
    light:
      --background: "oklch(0.985 0.01 150)"
      --foreground: "oklch(0.13 0.02 150)"
      --primary: "oklch(0.47 0.14 155)"
      --ring: "oklch(0.47 0.14 155)"
    dark:
      --background: "oklch(0.10 0.01 150)"
      --foreground: "oklch(0.94 0.01 150)"
      --primary: "oklch(0.73 0.15 155)"
      --ring: "oklch(0.73 0.15 155)"
  style:
    --folioh-content-max-width: "82rem"
    --folioh-section-gap: "2.75rem"
    --folioh-card-padding: "1.1rem"
    --folioh-code-bg: "oklch(0.14 0.01 150)"
```
</ConfigPanel>

`tokens.light` and `tokens.dark` accept CSS custom properties such as shadcn
tokens (`--background`, `--card`, `--border`, `--chart-1`) and project tokens
(`--brand-accent`). `style` accepts layout variables such as
`--folioh-content-max-width`, `--folioh-section-gap`, `--folioh-card-padding`,
`--folioh-code-bg`, `--folioh-workspace-shell-topbar`,
`--folioh-workspace-shell-topbar-blur`, and
`--folioh-workspace-shell-topbar-border`. The un-prefixed legacy spellings
(for example `--content-max-width`) are still accepted for compatibility but
are deprecated; new configs should use the canonical `--folioh-*` names.

### Header URL Validation

The header can carry links: `theme.header.repo` and `theme.header.action_href`.
Folioh validates these URLs at config-load time against an allowlist of safe
schemes. Permitted values are:

- `http` and `https` URLs (for example `https://github.com/acme/sdk`);
- `mailto:` links; and
- relative URLs (for example `/dashboard` or `docs/index`).

Unsafe schemes — notably `javascript:` and `data:` — are rejected. A header
link using a rejected scheme raises a config error and fails the build, naming
the offending field, rather than being emitted into the generated site. This
keeps script-injection payloads out of the docs header even when the
`docs.yaml` comes from an untrusted source. Use an `http(s)`, `mailto:`, or
relative URL for any header link.

The top-level `project.repo` is validated less strictly because repository
URLs legitimately use non-web schemes: `ssh://`, `git://`, `git+https://`, and
scp-style `git@host:path` forms are accepted, and only schemes that could
execute script or read local files (`javascript:`, `data:`, `vbscript:`,
`file:`) are rejected.

## Variants

Variants let a project expose its own theme controls. Each option can set
a swatch, preview colors, style overrides, and light/dark token overrides while
inheriting the base project preset. The first control whose options set no
`style` only recolors the preset, and the picker shows it as the Colours row
under the preset's slide; every other control is a row of its Customize step.

```yaml
theme:
  preset: "acme"
  name: "Acme"
  variants:
    color:
      label: "Color"
      default: "default"
      options:
        default:
          label: "Default"
          swatch: "oklch(0.47 0.14 155)"
        ocean:
          label: "Ocean"
          swatch: "oklch(0.56 0.16 230)"
          tokens:
            light:
              --primary: "oklch(0.50 0.16 230)"
              --ring: "oklch(0.50 0.16 230)"
            dark:
              --primary: "oklch(0.74 0.15 230)"
              --ring: "oklch(0.74 0.15 230)"
```

If an option has `swatch` but no full light/dark `preview`, Folioh uses the
swatch for both preview modes.

Every option combination across all variant controls is resolved and embedded
into each generated page, so `theme.variants` is capped at 256 combinations
(the product of each control's option count). Exceeding the cap fails config
validation; reduce the number of controls or options.

## CSS Variables

Folioh themes use `oklch` colors through CSS custom properties:

```css
:root {
  --background: oklch(0.995 0 0);
  --foreground: oklch(0.145 0.005 285);
  --primary: oklch(0.51 0.14 170);
  --primary-foreground: oklch(0.99 0 0);
  --muted: oklch(0.96 0.003 264);
  --muted-foreground: oklch(0.50 0.015 264);
  --card: oklch(0.995 0 0);
  --card-foreground: oklch(0.145 0.005 285);
  --border: oklch(0.91 0.004 264);
  --input: oklch(0.91 0.004 264);
  --ring: oklch(0.51 0.14 170);
  --radius: 0.5rem;
}

.dark {
  --background: oklch(0.14 0.004 285);
  --foreground: oklch(0.92 0.004 264);
  --primary: oklch(0.72 0.17 170);
  --border: oklch(1 0 0 / 8%);
}
```

Sidebar-specific variables control the left navigation independently from the
main content area:

| Variable | Purpose |
|----------|---------|
| `--sidebar` | Sidebar background. |
| `--sidebar-foreground` | Sidebar text color. |
| `--sidebar-primary` | Sidebar active item color. |
| `--sidebar-accent` | Sidebar hover/focus background. |
| `--sidebar-border` | Sidebar border color. |

Motion variables time the mobile menu and the Term card on every preset, and
the transitions the Pastel preset adds. The rest of the docs shell keeps its own fixed timings,
Nextra's and Folioh's:

| Variable | Default | Purpose |
|----------|---------|---------|
| `--folioh-motion-micro` | `120ms` | The close button answering a hover. |
| `--folioh-motion-element` | `200ms` | The scrim fading in, and the close icon's turn. |
| `--folioh-motion-exit` | `180ms` | The scrim fading out. |
| `--folioh-motion-max` | `300ms` | The menu sliding out, and Pastel's light and dark reveal. |
| `--folioh-motion-glide` | `240ms` | Pastel's tab highlight gliding to the chosen tab, and the tab label's colour. |
| `--folioh-motion-fade` | `160ms` | Pastel's newly chosen tab panel fading in. |
| `--folioh-motion-drawer` | `560ms` | The menu springing in. |
| `--folioh-ease-enter` | `cubic-bezier(0.2, 0.8, 0.2, 1)` | The scrim fading in, the close button, and Pastel's light and dark reveal. |
| `--folioh-ease-exit` | `cubic-bezier(0.4, 0, 1, 1)` | The menu and the scrim leaving. |
| `--folioh-spring-drawer` | a spring as `linear()` | The menu springing in; a `cubic-bezier` where `linear()` is not supported. |

Below Nextra's `md` breakpoint (48rem, 768 px at the default font size) the
docs menu is a drawer, drawn from these variables:

| Variable | Default | Purpose |
|----------|---------|---------|
| `--folioh-drawer-inset` | `12px` | Gap between the drawer and the screen edges. |
| `--folioh-drawer-width` | `21.25rem` | Drawer width, 340 px at the default font size and never wider than the screen minus both insets. |
| `--folioh-drawer-radius` | `24px` | Corner radius. |
| `--folioh-drawer-shadow` | a hairline and a soft shadow | The drawer's edge. |
| `--folioh-drawer-scrim` | `--scrim` at 30 % | The layer that dims the page behind the drawer. |

A theme that wants a full-screen menu sets `--folioh-drawer-inset: 0`,
`--folioh-drawer-width: 100vw`, `--folioh-drawer-radius: 0` and
`--folioh-drawer-shadow: none`. Under reduced motion every transition and
animation on a docs page finishes at once; the landing and the roadmap keep
their own reduced-motion styles.

Callouts draw their six types from tone variables, one set per type: a fill
for the surface, an ink for the title and the text, and an accent for the icon
and the edge. `<type>` is `note`, `info`, `tip`, `check`, `warning` or
`danger`:

| Variable | Default | Purpose |
|----------|---------|---------|
| `--folioh-tone-<type>-accent` | a hue mixed 75 % into `--foreground`: blue for `info`, violet for `tip`, green for `check`, `--warning` and `--destructive` for the last two; `note` takes `--muted-foreground` | The icon and the edge. Marker's `ok` and `danger` labels take the `check` and `danger` accents. |
| `--folioh-tone-<type>-fill` | the accent at 12 % over `--background`; `note` mixes `--muted-foreground` at 8 % | The callout's surface. |
| `--folioh-tone-<type>-ink` | the accent at 25 % into `--foreground`; `note` takes `--foreground` | The title and the text. |

A theme sets one by name, for example `--folioh-tone-tip-accent`, and the fill
and ink mixed from it follow. The Pastel preset sets its own fills and inks
inside the callout, so a tone set at `:root` does not reach a Pastel callout.

| Variable | Default | Purpose |
|----------|---------|---------|
| `--folioh-surface-radius` | `var(--radius)`; Pastel sets `20px` | The corner of a raised surface: the cards, panels, empty states, tab panels, file trees and callouts the Pastel preset draws. |

### What Plugin Surfaces Rely On

The landing and the roadmap are drawn from the tokens on this page and nothing
else: no colour of their own, no per-page override. That is the contract a
plugin surface keeps, and it is what makes a preset switch or a theme package
restyle every page at once.

| Family | Tokens | Set by |
|--------|--------|--------|
| Surfaces and ink | `--background`, `--foreground`, `--card`, `--card-foreground`, `--popover`, `--popover-foreground`, `--muted`, `--muted-foreground`, `--accent`, `--accent-foreground`, `--secondary`, `--secondary-foreground`, `--border`, `--input`, `--ring`, `--radius` | the preset; `theme.tokens` overrides |
| Emphasis | `--primary`, `--primary-foreground`, `--destructive`, `--warning` | the preset |
| Data | `--chart-1` to `--chart-5`; the index pages, the preview cards and Mermaid draw from these | the preset |
| Type | `--font-sans`, `--font-mono`, `--folioh-heading-font-family`, `--folioh-body-font-family` | the preset; `theme.tune` |
| Layout | the `--folioh-*` variables listed under Project Presets | the preset; `theme.style` |

Mermaid diagrams read the computed values of these tokens when they render and
fall back to a neutral palette only when none resolve.

A plugin surface that needs a colour these families do not carry adds a named
token in the same shape rather than a literal, so a theme can reach it.

## When Personalization Is Not Enough

Use [theme packages](./theme-packages) when the project needs to override files
inside the bundled template while keeping Folioh's template as a base. Use
[custom templates](./custom-templates) when the project needs to own the entire
frontend workspace.

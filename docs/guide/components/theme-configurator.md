# Theme picker

The theme picker lets readers choose how the site looks. A preset owns color tokens, document rhythm, code block treatment, borders, radius defaults, typography defaults, and its own controls; the picker groups related presets into families, shows complete color-and-style variants as swatches, and offers manual controls in Customize.

Applied choices are persisted in `localStorage` under a project-scoped key derived from the configured default preset, and the saved theme is the same on the landing page, the docs and the previews. The default theme CSS is also rendered into the page so the generated site does not flash or fall back to Folioh's bundled typography before hydration. Saved choices are restored only when their preset is in the enabled catalog. A disabled preset falls back to the site default before hydration. When moving from the former Roller default to Folioh Pastel, an existing new storage key takes precedence.

## API

### ThemeGallery

This component takes no props. The bundled template renders it as the palette button in the landing, docs and previews navbars, and every copy opens the same dialog. The palette button, or the `t` key outside a text field, opens the picker.

`ThemeConfigurator` is still exported from `components/theme-configurator.tsx` and renders nothing, so a layout that mounts it still builds.

### Themes

The first step shows each color-and-style variant as a slide drawn as a small docs page in its own colors, type, and radius: the theme in front in the middle, two neighbours on each side, one on phones. The front slide has a ring, and its family and variant name sit under it. A check marks the slide that looks as the saved theme does.

Every slide shows the same short Getting started page with real text, code and cards. It uses the theme's heading and body fonts, reading rhythm, section rules, code treatment and outer frame, so those differences remain visible while comparing colors. Customize updates this same preview.

By default the picker offers Folioh Pastel with Ink, Jade, Lavender, Peach and Sky. Set `theme.extra_presets: true` to enable the full bundled catalog: six Folioh styles, five Pastel palettes and four Omarchy palettes. An explicitly configured `theme.preset`, project presets and independently registered presets remain selectable without that opt-in.

With extra presets enabled, the catalog stays ordered Folioh, Folioh Pastel and Omarchy, with project presets first when defined. Each opening centers your saved theme and palette, including its customizations. Opening it leaves the applied theme unchanged.

Each swatch applies its variant's colors and default style together. For example, Ballpoint includes its typography, code treatment and corners. A sun or moon marks a fixed light or dark scheme. The final circle, with the adjustments icon, opens Customize. The carousel follows the swatch order through the enabled families and wraps at both ends.

| Input | Action |
|-------|--------|
| The left and right arrow keys, the side buttons, a swipe on the slides | Apply the previous or next variant, continuing across families and wrapping at both ends. |
| A click on a side slide | Select that variant. |
| Home, End | Apply the first or last variant. |
| The up and down arrow keys, a swatch | Select a complete variant within the family. |
| Enter, Cmd/Ctrl+Enter, a click on the front slide | Select the front theme with its color, and close. |
| `c`, Customize | Open the Customize step for the front theme. |
| Reset | Apply the site's own theme with its defaults. |
| Done, `t`, Escape, the close button, a click outside the dialog | Close and keep the current theme. |

Navigation, swatches, manual adjustments and mode changes apply to the page and save immediately. The picker stays open while changing themes. Done only closes it; opening and closing without making a choice leaves the applied theme unchanged.

The front preview keeps its manual adjustments; each neighbouring preview shows the exact recipe it will select. Changing to another variant starts from its configured style. Selecting a swatch also clears manual adjustments. The site's preset uses `theme.tune` and `theme.radius`. A saved recipe removed from the catalog remains reachable and editable until another variant replaces its family's draft, without adding a swatch. Stored preset IDs remain compatible with earlier versions.

### Customize

The final circle in the variant row, or the `c` key outside a text field, opens Customize inside the same centered popup. Its size, backdrop and position stay the same. Its controls update both the preview and the page immediately. Back or Escape returns to the variants; every change is already saved.

| Section | Rows |
|---------|------|
| The theme's name and options, such as Roller options | The theme's own controls other than its colors, such as density, code, frame, binding or rules. Omarchy has none. |
| Type | Typography, Reading rhythm |
| Shape | Corners, Borders, Code block frame |
| Layout | Page frame |
| Colors | Free background, text and accent colors, edited independently for light and dark. Fixed-scheme variants edit their own scheme. |

A dot marks each row's default: the theme's own value, or on the site's own theme the value from `theme.tune` and `theme.radius`. The option that keeps the theme's own value reads Default. Reset returns every row to the theme's defaults and keeps its color. Reset also removes manual colors for both modes. Manual colors remain editable on Omarchy palettes.

| Input | Action |
|-------|--------|
| The arrow keys | Move within the row that has focus. |
| `c`, Escape, the back arrow | Return to the themes. |
| Cmd/Ctrl+Enter, Done, the close button | Close and keep the current theme. |

Closing the popup keeps all changes.

### Mode

Light, Dark and System sit in the picker's header and apply immediately to the front theme. The `d` shortcut changes the mode directly inside and outside the picker. While the front theme is light or dark only, the mode radios show its scheme, are disabled, and say why, for example "Tokyo Night is dark only"; the reader's own mode stays stored.

A theme drawn for one scheme sets that scheme itself. While one is applied, such as an Omarchy palette, the light/dark toggles are hidden and `d` does nothing; another theme brings the reader's own mode back.

With `theme.dark_mode: false` the theme provider forces the light theme: the picker shows no mode control and `d` does nothing. A dark-only palette the reader applies, such as a dark Omarchy one, still turns the page dark.

The navbars carry no separate light/dark toggle unless `theme.header.theme_toggle` is `true` and dark mode is on; then the landing and the docs navbars both show one.

### Preset Library

Presets keep their existing IDs for `theme.preset`, stored preferences and theme packages. The full catalog below requires `theme.extra_presets: true`; the configured site preset and project presets are always available:

| Family | Choices |
|--------|---------|
| Folioh | Roller, Ballpoint, Paperback, Letterpress, Notebook, Typewriter |
| Folioh Pastel | Ink, Jade, Lavender, Peach, Sky |
| Omarchy | Catppuccin Latte, Tokyo Night, Gruvbox, Hackerman |

Folioh starts with Roller and moves from light typography to stronger rules and frames. Omarchy starts with its light palette, followed by the blue, warm and neon dark palettes. Keyboard navigation follows this same order.

A project preset appears first when defined. Other registered presets remain selectable independently. A project override of a built-in ID appears once, under its project name, and keeps all its variants.

| Folioh style | Preset ID | Distinguishing treatment |
|-------------|-----------|-------------------------|
| Roller | `organic-editorial` | Thin headings, generous spacing and a cobalt accent. |
| Ballpoint | `aperture` | Compact sans-serif documentation with rounded code panels. |
| Paperback | `stacks` | Serif headings and body for long-form reading. |
| Letterpress | `atlas` | Bold serif headings, paper surface and square corners. |
| Notebook | `workshop` | A framed workspace with warm surfaces and green accents. |
| Typewriter | `carbon` | Monospaced headings, strong rules and square corners. |

Folioh Pastel is the default preset, with Ink selected: pen-blue ink, paper surfaces, Grotesque typography and 0.75rem corners. Roller remains available in the Folioh family and as the explicit `organic-editorial` preset.

Each of these styles can be selected from its family’s swatch row. A preset picked for the first time applies its default controls, typography, accent, radius, and layout defaults. Customize keeps the front preset while changing its options.

`theme.preset` names the preset readers see first. Use the preset ID from the table above (for example, `organic-editorial` for Roller), an ID a theme package or template overlay declares, or a new ID the project defines in `docs.yaml` (see [Project Theme Contract](#project-theme-contract)). Preset aliases, such as `folioh` or `openai`, select their corresponding current preset. Any other value stops the build with the list of valid ids and the nearest one.

Omarchy offers four palettes: Catppuccin Latte (cool light), Tokyo Night (blue dark), Gruvbox (warm dark), and Hackerman (neon green dark). Each fixes its light or dark scheme. The preset also gives the navbar and the chapter list flat, square rows, and defaults to the Terminal typography, a monospaced body under Geist headings, and a square radius. Its background is a field of pixels tinted by the palette. Folioh shows its CLI block-letter wordmark above the landing and documentation headings; other projects keep their own name. The picker previews include the same artwork.

The pixels flow continuously and brighten around the mouse. Click an empty part of the page to release a wave; holding first makes it stronger. The wordmark alternates between four reconstructions: a sweep, falling columns, scattered characters and a radial reveal. It rebuilds on entry and palette changes. Click the wordmark, or focus it and press Enter or Space, to replay it. The final text always keeps the project's name.

Omarchy palette changes use a brief diagonal transition in supporting browsers. Without support, or with reduced motion enabled, the palette applies directly. Reduced motion also keeps the artwork still. Animation pauses outside the viewport and in hidden tabs; neighbouring picker previews remain static. The active preview and Customize show the same motion as the page.

The Omarchy preset and the gallery are inspired by [Omarchy](https://omarchy.org): the palettes are read from the MIT-licensed [omacom/omarchy](https://github.com/omacom/omarchy) repository, and the gallery takes its idea from the theme picker in [its manual](https://omarchy.org/manual/). Thanks to David Heinemeier Hansson and the Omarchy contributors. The notice is in `THIRD-PARTY-NOTICES.md`.

Notebook includes a Borders control for switching between fine, structured, and ruled outlines.

Earlier saved styles and palettes still render unchanged and can be adjusted in Customize. Beacon, Ledger, Proof, Draftline, Canopy and the omitted color variants remain valid in project configurations for compatibility, but no longer add choices to the built-in picker.

Folioh Pastel is a separate family with preset ID `pastel` and five palettes: Ink (default), Jade, Lavender, Peach and Sky. Each supports light and dark and uses the same organic shapes, Grotesque typography and 0.75rem default radius. Choose a palette from its swatches in T. Jade keeps the original cool paper, petrol ink and pastel fills; older saved Pastel choices without a palette still resolve to Jade.

Pastel’s generated logo and favicon use the same irregular silhouette as its artwork. The mark appears in the navbars, landing footer, demo, picker preview and social images. Page logos follow the reader’s selected theme; the favicon and social images use the site’s configured default. Pastel social cards also use the configured light palette and the site’s freehand contours. The generated image is fixed at build time, independent of a reader’s saved theme. Other themes keep their normal mark. Explicit project logos and favicons retain their own artwork.

On desktop the 288 px floating sidebar contains the project branding, search and theme controls. Its header stays attached when scrolling or collapsing the sidebar; the navigation list scrolls independently below the search. Collapsing leaves an 84 px rail with the logo and 44 px controls aligned vertically. The magnifier opens a search panel beside the rail, with matching input and result widths. Escape closes it and returns focus to the magnifier; clicking outside or choosing a result closes it too. Cmd+K or Ctrl+K opens the same panel. The hidden navigation leaves the keyboard tab order until expanded. The desktop table of contents floats beside the article without a background and stays in view while scrolling; long outlines scroll within it. It marks the section in view and shows reading progress where supported. Callouts keep their functional icons on soft asymmetric holders. FeatureCard keeps its named Hugeicons centered over four original organic backgrounds, alternating with the card colors; text/emoji fallback and cards without icons stay unchanged. Shell icons remain unchanged. Decorative corner marks, step tabs and timeline markers use original organic outlines. Code blocks have a file header and language pill, and TerminalSession is a petrol window with three tabs. Pill tabs glide between selections, accordions are soft cards, and light/dark changes reveal the new scheme from the control where supported. Reduced motion removes these transitions. The MDX does not change, and other presets keep their own look.

The page title shares its space with three original, soft asymmetric silhouettes. The landing places the same shapes in solid, contrasting palette colors around the product demo, outside the title and text flow. The project's content and actions stay intact. Native animations gently morph and drift them, with a small response to the pointer. They remain still under reduced motion, while hidden, outside the viewport or in a neighbouring picker slide. The active gallery preview reproduces the rail, typography and artwork. The background wash uses CSS gradients.

Pastel's layout, measurements, motion timings and component CSS adapt [cojeev](https://github.com/luv-jeri/cojeev-ui), used under the MIT License. Its five palettes, decorative masks and irregular silhouettes are Folioh's own. The Bricolage Grotesque and DM Sans font pairing also comes from cojeev; the fonts are loaded from Google Fonts under the SIL Open Font License 1.1. Thanks to Sanjay Kumar. The retained attribution and licenses are in `THIRD-PARTY-NOTICES.md`.

### Shared Controls

Every preset can expose its own controls. The shared controls are rows of every theme's Customize step, and override the front preset when changed:

| Row | Purpose |
|-----|---------|
| Typography | Switch heading, body, and code font treatment. |
| Reading rhythm | Override base type size, line height, section gaps, and card padding. |
| Corners | Adjust shared UI and card radius. |
| Borders | Tune card and shell rule strength globally. |
| Code block frame | Switch source examples between soft, framed, plate, and terminal treatments. |
| Page frame | Expose the outer page padding used by framed workspace themes. |
| Background, Text, Accent | Pick any color for the current scheme. |

Content width is not a row of the picker. `theme.tune.width` still sets it, and a stored choice stays valid.

Earlier stored surface and accent options remain valid. Manual colors override the background, foreground and primary tokens after those layers, including fixed-scheme palettes. The saved colors are applied before hydration.

### Radius Options

| Option | Value |
|--------|-------|
| None | `0` |
| Sm | `0.3rem` |
| Md | `0.5rem` |
| Lg | `0.75rem` |
| Full | `1rem` |

Reset on the Themes step applies the configured default preset with its tuning. In Customize, Reset returns the front theme's rows to their defaults. Both save immediately.

### Project Theme Contract

Projects can define their own ThemeConfigurator preset in `docs.yaml` without forking the bundled template. During template preparation, Folioh writes `theme/project-theme.ts` with a typed `ThemePreset` that merges Folioh's base docs tokens with the project's overrides.

```yaml
theme:
  preset: "acme"
  name: "Acme"
  description: "Operational docs theme"
  scene: "Engineers scan APIs, examples, and release notes in a compact product surface."
  preview:
    light: "oklch(0.490 0.130 285)"
    dark: "oklch(0.720 0.100 285)"
  header:
    brand: "Acme"
    badge: "Platform"
    repo: "https://github.com/acme/project"
    theme_toggle: true
    action_label: "Dashboard"
    action_href: "/dashboard"
    search: false
  radius: "0.5rem"
  tune:
    font: "geist"
    accent: "ink"
    surface: "preset"
    shell: "flush"
    width: "wide"
    rhythm: "compact"
    borders: "fine"
    code: "terminal"
  style:
    "--folioh-content-max-width": "74rem"
    "--folioh-body-line-height": "1.58"
    "--folioh-workspace-shell-topbar": "color-mix(in oklch, var(--background) 80%, transparent)"
    "--folioh-workspace-shell-topbar-blur": "blur(12px)"
    "--folioh-workspace-shell-topbar-border": "1px solid color-mix(in oklch, var(--border) 50%, transparent)"
  tokens:
    light:
      "--background": "oklch(0.985 0.008 80)"
      "--foreground": "oklch(0.175 0.008 75)"
      "--primary": "oklch(0.490 0.130 285)"
    dark:
      "--background": "oklch(0.155 0.010 75)"
      "--foreground": "oklch(0.950 0.008 80)"
      "--primary": "oklch(0.720 0.100 285)"
  variants:
    palette:
      label: "Palette"
      default: "default"
      options:
        default:
          label: "Default"
          swatch: "oklch(0.490 0.130 285)"
        midnight:
          label: "Midnight"
          swatch: "oklch(0.680 0.180 200)"
          tokens:
            light:
              "--background": "oklch(0.985 0.008 250)"
              "--primary": "oklch(0.480 0.160 200)"
            dark:
              "--background": "oklch(0.095 0.020 250)"
              "--primary": "oklch(0.680 0.180 200)"
```

`tokens.light` and `tokens.dark` accept CSS custom properties such as shadcn tokens (`--background`, `--card`, `--border`, `--chart-1`) and project tokens (`--brand-accent`). `style` accepts ThemeConfigurator layout variables such as `--folioh-content-max-width`, `--folioh-section-gap`, `--folioh-card-padding`, `--folioh-code-bg`, `--folioh-workspace-shell-topbar`, `--folioh-workspace-shell-topbar-blur`, and `--folioh-workspace-shell-topbar-border`. Un-prefixed legacy names such as `--content-max-width` are still accepted for compatibility but are deprecated; use the `--folioh-*` names.

`header.brand` and `header.badge` replace the default docs navbar wordmark. `header.repo`, `header.theme_toggle`, `header.action_label`, and `header.action_href` replace the default docs navbar actions with project-owned actions; `header.search: false` hides the navbar search field while leaving the generated search index controlled by `search.enabled`. `variants` defines project-owned preset controls; each option can set a `swatch` for the control UI, override `preview`, `style`, and light/dark tokens while inheriting the base project theme. If an option has `swatch` but no full light/dark `preview`, Folioh uses the swatch for both preview modes.

For safety, token and style keys must be CSS custom properties beginning with `--`, values must be strings without CSS statement or block delimiters, and header and variant labels cannot include markup. Unknown `tune` keys are ignored with a warning that names the key they most likely meant, and a `tune` value the control does not offer stops the build; [Tune Defaults](../theming/personalization#tune-defaults) lists the values.

`theme.radius` must be one of the fixed radius scale values `"0"`, `"0.3rem"`, `"0.5rem"`, `"0.75rem"`, or `"1rem"`, or a named alias (`"none"`, `"sm"`, `"md"`, `"lg"`, `"full"`) that maps onto the same scale; any other value fails config validation. `theme.variants` is capped at 256 option combinations across all controls (the product of each control's option count) because every combination is resolved and embedded into each generated page; a larger product fails config validation.

Tune aliases map to the shared controls:

| YAML key | Runtime control |
|----------|-----------------|
| `font` | `fontId` |
| `accent` / `color` | `colorId` |
| `surface` | `surfaceColorId` |
| `shell` | `shellPaddingId` |
| `width` / `content_width` | `contentWidthId` |
| `rhythm` / `reading` | `rhythmId` |
| `borders` / `border` | `borderId` |
| `code` / `code_blocks` | `codeTreatmentId` |

`font: "geist"` selects the bundled Geist/Geist Mono pair and maps the public `--font-sans` / `--font-mono` tokens used by Tailwind utility classes. If `theme.preset` matches a built-in preset and the project only provides `tune`, Folioh keeps the built-in preset and applies the configured defaults. If the project supplies `name`, `description`, `scene`, `preview`, `style`, `tokens`, or `variants`, Folioh places the project preset before the built-in library. A `variants` control whose options set no `style` only recolors the theme, so the picker shows the first such control as the preset's Colours row and every other control in Customize.

### Theme Packages

Theme packages can replace the bundled configurator, project header actions, or
`theme/project-theme.ts` while Folioh still supplies generated content and
metadata. See [Theme Packages](../theming/theme-packages) for the ownership
model, file overlay rules, and validation checklist.

### Create a Custom Preset

The bundled presets live in the template's `theme/presets.ts` and use the interfaces from `theme/preset-types.ts`. A project adds one without forking the template: a [theme package](../theming/theme-packages#register-a-custom-preset) calls `registerPreset` from its `theme/project-theme.ts`, or a [template overlay](../theming/custom-templates#overlay-partial-override) ships its own copy of `theme/presets.ts`. Folioh reads the `id` from either, so `theme.preset` can select it.

1. Create a new object that satisfies `ThemePreset`.
2. Give it stable `defaultOptions`, optional `defaultRadiusIndex`, optional `defaultCustomization`, and matching `controls`.
3. Implement `resolve(options)` so every option combination returns `light`, `dark`, `style`, `radius`, and `preview`.
4. Register it: call `registerPreset(preset, groupId)` from a theme package, or, in an overlay copy of `presets.ts`, add it to `builtinPresets` and list its id in a `registerGroup` call. A preset in no group shows under Other.

The `style` object must use the namespaced `--folioh-*` keys defined by `ThemeStyle` in `template/theme/theme-contract.generated.ts` (its header line names the generator, `crates/folioh-site/src/theme.rs`). Un-prefixed keys such as `--card-shadow` fail the TypeScript check against `ThemeStyle` and are not read by the generated CSS.

Example:

```ts
import type { ThemePreset } from "./preset-types"

export const notebookPreset: ThemePreset = {
  id: "notebook",
  name: "Notebook",
  description: "Lab notes, ruled paper, compact examples",
  scene: "A maintainer reviews examples and release notes in a working notebook before publishing.",
  preview: {
    light: "oklch(0.34 0.035 236)",
    dark: "oklch(0.78 0.040 236)",
  },
  defaultOptions: {
    paper: "ruled",
    code: "margin",
  },
  defaultRadiusIndex: 1,
  defaultCustomization: {
    fontId: "sans",
    colorId: "indigo",
  },
  controls: [
    {
      id: "paper",
      label: "Paper",
      options: [
        { label: "Ruled", value: "ruled" },
        { label: "Plain", value: "plain" },
      ],
    },
    {
      id: "code",
      label: "Code",
      options: [
        { label: "Margin", value: "margin" },
        { label: "Block", value: "block" },
      ],
    },
  ],
  resolve(options) {
    const ruled = options.paper === "ruled"
    const blockCode = options.code === "block"

    return {
      preview: {
        light: "oklch(0.34 0.035 236)",
        dark: "oklch(0.78 0.040 236)",
      },
      radius: "0.25rem",
      style: {
        "--folioh-heading-font-family": "Georgia, \"Times New Roman\", ui-serif, serif",
        "--folioh-body-font-family": "var(--font-sans), ui-sans-serif, system-ui, sans-serif",
        "--folioh-code-font-family": "var(--font-mono), ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
        "--folioh-heading-letter-spacing": "0",
        "--folioh-heading-weight": "780",
        "--folioh-body-line-height": ruled ? "1.78" : "1.70",
        "--folioh-font-size-base": "1rem",
        "--folioh-card-shadow": "none",
        "--folioh-card-border-width": "1px",
        "--folioh-card-padding": "1.25rem",
        "--folioh-card-hover-shadow": "0 0 0 1px var(--foreground)",
        "--folioh-card-backdrop": "none",
        "--folioh-card-opacity": "1",
        "--folioh-code-border-radius": blockCode ? "0.2rem" : "0",
        "--folioh-code-border": blockCode ? "1px solid var(--border)" : "1px solid var(--foreground)",
        "--folioh-code-bg": blockCode ? "var(--muted)" : "var(--background)",
        "--folioh-code-foreground": "inherit",
        "--folioh-code-shadow": "none",
        "--folioh-h2-border": ruled ? "1px solid var(--border)" : "none",
        "--folioh-h2-transform": "none",
        "--folioh-h2-letter-spacing": "0",
        "--folioh-h2-weight": "760",
        "--folioh-h2-padding-left": "0",
        "--folioh-h2-border-left": "none",
        "--folioh-link-decoration": "underline",
        "--folioh-section-gap": ruled ? "2.5rem" : "2.75rem",
        "--folioh-content-max-width": "48rem",
        "--folioh-workspace-shell-padding": "0px",
        "--folioh-workspace-shell-border": "0 solid transparent",
        "--folioh-workspace-shell-shadow": "none",
        "--folioh-workspace-shell-background": "var(--background)",
        "--folioh-workspace-shell-surface": "transparent",
        "--folioh-workspace-shell-topbar": "var(--background)",
        "--folioh-workspace-shell-topbar-blur": "none",
        "--folioh-workspace-shell-topbar-border": "1px solid var(--border)",
      },
      light: {
        "--background": "oklch(0.976 0.006 236)",
        "--foreground": "oklch(0.180 0.012 236)",
        "--card": "oklch(0.988 0.005 236)",
        "--card-foreground": "oklch(0.180 0.012 236)",
        "--popover": "oklch(0.992 0.005 236)",
        "--popover-foreground": "oklch(0.180 0.012 236)",
        "--primary": "oklch(0.340 0.060 236)",
        "--primary-foreground": "oklch(0.976 0.006 236)",
        "--secondary": "oklch(0.935 0.007 236)",
        "--secondary-foreground": "oklch(0.180 0.012 236)",
        "--muted": "oklch(0.935 0.007 236)",
        "--muted-foreground": "oklch(0.460 0.012 236)",
        "--accent": "oklch(0.890 0.018 236)",
        "--accent-foreground": "oklch(0.180 0.012 236)",
        "--destructive": "oklch(0.550 0.200 28)",
        "--border": "oklch(0.760 0.010 236)",
        "--input": "oklch(0.760 0.010 236)",
        "--ring": "oklch(0.340 0.060 236)",
        "--chart-1": "oklch(0.340 0.060 236)",
        "--chart-2": "oklch(0.500 0.040 180)",
        "--chart-3": "oklch(0.560 0.070 120)",
        "--chart-4": "oklch(0.520 0.090 48)",
        "--chart-5": "oklch(0.420 0.040 290)",
        "--sidebar": "oklch(0.950 0.006 236)",
        "--sidebar-foreground": "oklch(0.180 0.012 236)",
        "--sidebar-primary": "oklch(0.340 0.060 236)",
        "--sidebar-primary-foreground": "oklch(0.976 0.006 236)",
        "--sidebar-accent": "oklch(0.915 0.008 236)",
        "--sidebar-accent-foreground": "oklch(0.180 0.012 236)",
        "--sidebar-border": "oklch(0.760 0.010 236)",
        "--sidebar-ring": "oklch(0.340 0.060 236)",
      },
      dark: {
        "--background": "oklch(0.120 0.012 236)",
        "--foreground": "oklch(0.910 0.007 236)",
        "--card": "oklch(0.155 0.012 236)",
        "--card-foreground": "oklch(0.910 0.007 236)",
        "--popover": "oklch(0.170 0.012 236)",
        "--popover-foreground": "oklch(0.910 0.007 236)",
        "--primary": "oklch(0.780 0.060 236)",
        "--primary-foreground": "oklch(0.120 0.012 236)",
        "--secondary": "oklch(0.205 0.012 236)",
        "--secondary-foreground": "oklch(0.910 0.007 236)",
        "--muted": "oklch(0.205 0.012 236)",
        "--muted-foreground": "oklch(0.620 0.008 236)",
        "--accent": "oklch(0.250 0.020 236)",
        "--accent-foreground": "oklch(0.910 0.007 236)",
        "--destructive": "oklch(0.650 0.180 28)",
        "--border": "oklch(0.315 0.012 236)",
        "--input": "oklch(0.315 0.012 236)",
        "--ring": "oklch(0.780 0.060 236)",
        "--chart-1": "oklch(0.780 0.060 236)",
        "--chart-2": "oklch(0.640 0.040 180)",
        "--chart-3": "oklch(0.680 0.070 120)",
        "--chart-4": "oklch(0.680 0.090 48)",
        "--chart-5": "oklch(0.620 0.050 290)",
        "--sidebar": "oklch(0.100 0.012 236)",
        "--sidebar-foreground": "oklch(0.910 0.007 236)",
        "--sidebar-primary": "oklch(0.780 0.060 236)",
        "--sidebar-primary-foreground": "oklch(0.100 0.012 236)",
        "--sidebar-accent": "oklch(0.190 0.012 236)",
        "--sidebar-accent-foreground": "oklch(0.910 0.007 236)",
        "--sidebar-border": "oklch(0.315 0.012 236)",
        "--sidebar-ring": "oklch(0.780 0.060 236)",
      },
    }
  },
}
```

Then register it. In an overlay copy of `theme/presets.ts`:

```ts
const builtinPresets: ThemePreset[] = [
  workshopPreset,
  canopyPreset,
  beaconPreset,
  atlasPreset,
  ledgerPreset,
  proofPreset,
  stacksPreset,
  draftlinePreset,
  aperturePreset,
  organicEditorialPreset,
  carbonPreset,
  notebookPreset,
]

// …

registerGroup("reference", "Reference", ["atlas", "stacks", "draftline", "proof", "notebook"])
```

and select it with `theme.preset: "notebook"`.

### Generate a Preset with ChatGPT

Paste this prompt into ChatGPT, then register the returned object as described in [Create a Custom Preset](#create-a-custom-preset).

```text
Create a documentation theme preset as a TypeScript object that satisfies ThemePreset from ./preset-types.

Rules:
- Return only TypeScript code.
- No gradients, no neon, no glow.
- Use OKLCH colors.
- Include id, name, description, scene, preview, defaultOptions, controls, defaultRadiusIndex, defaultCustomization, and resolve(options).
- The preset must feel like a documentation material system, not a decorative skin.
- Controls should change real aspects of the preset, such as paper tone, rule weight, density, code block treatment, or contrast.
- resolve(options) must return preview, radius, style, light, and dark.
- style must include every ThemeStyle key.
- light and dark must include every shadcn color token used by the existing built-in presets.

Preset concept:
[describe the material or publishing idea here]
```

After ChatGPT returns a preset:

1. Paste it into your theme package's `theme/project-theme.ts` or your overlay's `theme/presets.ts`.
2. Register it as in [Create a Custom Preset](#create-a-custom-preset).
3. Set `theme.preset` to its id and run `folioh build`.
4. Open the theme picker with the palette button or `t`.
5. Test each generated control in light and dark mode.

## Example

<PreviewCode>

```mdx
<ThemeGallery />
```

<Callout type="info" title="Layout Component">
  ThemeGallery is rendered in the landing, docs and previews navbars of every site built on the bundled template. A theme package or template overlay can replace `components/theme-gallery.tsx`, or `components/theme-configurator.tsx` if it keeps the exports the picker imports (see [Theme Packages](../theming/theme-packages)).
</Callout>

</PreviewCode>

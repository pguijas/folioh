# Landing Page

*Configure the optional homepage that appears before the documentation app.*

<DocPreview
  title="Landing page example"
  example="landing-page"
  description="Open a complete homepage generated from its example configuration."
  height={420}
/>

Folioh starts as a docs-first site. The landing page is a built-in integration:
it is compiled into `folioh` and stays inert until a `landing:` section appears
in `docs.yaml`. When you want a public entry point, add the section and keep
the docs under `/docs/`.

```yaml
landing:
  enabled: true
  hero:
    tagline: "Documentation"     # optional kicker above the headline
    headline: "Your project"
    notice:                       # optional announcement chip above the kicker
      text: "New — v1.2 released"
      link: "/docs/changelog"     # optional; same href policy as other links
    description: "Generated API reference and guides, straight from your source."
  cta:
    primary:
      text: "Read the docs"
      link: "/docs"
    secondary:
      text: "View source"
      link: "https://github.com/you/project"
```

Use the landing page for project positioning, install commands, and links to the
most important documentation routes. Keep detailed tutorials, configuration
reference, and API content in the docs section.

## Recommended Structure

- **Hero**: project name, one-sentence value proposition, and primary docs link.
- **Install**: the shortest command sequence that gets users to a local preview.
- **Proof**: links to generated API pages, guides, examples, or release notes.
- **CTA**: a final link to `/docs` or the quickstart guide.

The generated landing page uses the same theme tokens, navigation, search
metadata, and static export path as the rest of the site.

`hero.tagline` is the kicker above the headline. Omit it and the headline opens
the hero; there is no default kicker.

`landing.install` lists the commands the install block shows. Omit it and the
page renders no install block; there is no default command.

Without `landing.features` and `landing.sections`, the page renders six
default cards: "Automatic API Reference", "One Config File", "Built-in
Integrations", "Dark Mode, Search, Responsive", "LLM-Friendly Output", and
"Markdown + API in One Site".

`hero.variant` picks the hero layout: `docs-map` (the default),
`source-pipeline`, `build-pipeline`, or `heartbeat`. Any other value renders
`docs-map` and the build warns, naming the valid variants.

## Section Catalog

`landing.sections` composes the homepage below the hero from reusable, config-driven
sections. Each entry needs a `type`; everything else is optional and degrades to
sensible defaults. Available types: `features`, `comparison`, `output`, `routes`,
`pipeline`, `funnel`, `install`, `stats`, `use-cases`, `cta`, `link-grid`, `cells`,
`mechanism`, and `statement`.

A section with any other `type`, or with none, is passed through to the
template unchanged and the build warns, listing the valid types: the bundled
template renders nothing for it, and a theme package that overrides
`components/landing/sections.tsx` may render it.

Most sections share the same heading fields (`eyebrow`, `title`, `description`).
Any section may also set `stage: "The mechanism"` — sections with a stage get a
numbered rail above their heading: the zero-padded stage number, then the label
as written (`02  The mechanism`), numbered in page order across all staged
sections. Sections without a stage render unchanged.

The showcase sections below are documented in detail because they render
live plugin data or bespoke layouts.

### `funnel` — the build funnel plate

A technical-diagram centerpiece: source tiles on the left converge through a
single build node, and every output tile fans out on the right. Each side is
a two-column block of tiles with alternating margins, and a tile is a mark
and a plain label — `"Markdown"`, `"API reference"` — never a path or a glob.
Inputs marked `ghost: true` render dimmed and dashed, for roadmap items that
are not shipped yet, and an optional `chip` labels them (for example
`"roadmap"`). Each tile may carry an `icon`, one of: `config`, `python`,
`javascript`, `rust`, `markdown`, `language`, `guides`, `api`, `pages`,
`folder`, `search`, `agents` (the llms.txt mark), `hash`. An unknown value is dropped and
the tile renders without a mark. Setting `description: ""` suppresses the
heading paragraph. When `inputs` or `outputs` are omitted the template renders
Folioh's own defaults, so a bare `- type: "funnel"` works. The earlier plate's
`guarantees`, `caption` and `command_notes` keys are no longer rendered; a
config that still carries one gets a build warning.

```yaml
landing:
  sections:
    - type: "funnel"
      stage: "The mechanism"
      title: "One build. Every output generated from it."
      command: "folioh build"            # default; the label on the node
      inputs:
        - label: "Config file"
          icon: "config"                # tile mark; unknown values drop
        - label: "Go"
          icon: "language"
          ghost: true                   # dimmed + dashed: honest roadmap item
          chip: "roadmap"
      outputs:
        - label: "API reference"
          icon: "api"
```

### `features` — rows or bento

The default `features` layout is the editorial row list. Setting
`variant: "bento"` switches to a two-column bento grid where each card carries
a small token-only vignette above its copy via `visual`, one of: `components`,
`llms`, `receipt`, `deploy`, `plugins`, `theming`. An unknown `visual` is
dropped and the card renders without a vignette.

```yaml
landing:
  sections:
    - type: "features"
      variant: "bento"
      features:
        - title: "30 MDX components"
          visual: "components"
          description: "Callouts, tabs, steps, file trees — themed with the site."
```

### `cells` — small feature cells

A bento-style row of compact cards: a mono micro-label, a short claim, a
one-line description, and an optional footer link (the whole cell becomes the
link). The heading is optional — omit `eyebrow`/`title` to render the grid
alone. Cells lay out four per row (three per row when the count is divisible
by three).

```yaml
landing:
  sections:
    - type: "cells"
      items:
        - label: "Agents"                # mono micro-label
          title: "llms.txt output"       # required; cells without it are dropped
          description: "When enabled, the build emits llms.txt alongside the human site."
          href: "/llms.txt"              # optional; makes the cell a link
          link_text: "See this site's llms.txt"
```

### `mechanism` — YAML diff to live UI

Shows the edit-commit-rebuild loop: a code window with an optional `git log`
strip, followed by a pipeline pill rail. Inside `code`, lines starting with `+ `
are tinted as additions and lines starting with `- ` as removals — indented
YAML list items are never mistaken for diff markers.

```yaml
landing:
  sections:
    - type: "mechanism"
      eyebrow: "Live from this repo"
      title: "The diff is the UI."
      code_title: "docs.yaml"
      code: |-
        source:
        - docs: ["old-guides/"]
        + docs: ["docs/"]
      commits:
        - hash: "a3f92c1"
          message: "docs: move the guide source"
      pills: ["git push", "folioh build", "deploy"]   # defaults shown
      caption: "Change the YAML. Commit. The site updates."
```

### `statement` — typographic closer

A huge centered statement with an optional accent-highlighted substring and
optional CTA links (two work best). `accent` must be an exact substring of
`text`; the first action defaults to the primary button style unless a
`primary` flag says otherwise. `size: "md"` steps the headline and padding
down for a mid-page thesis block (the default suits closers), and
`description` renders a reading-size lead paragraph under the headline.

```yaml
landing:
  sections:
    - type: "statement"
      size: "md"                 # optional: mid-page scale (default is closer scale)
      eyebrow: "The premise"
      text: "No database. No accounts. Just your repo."
      accent: "your repo"
      description: "A few sentences of thesis prose, rendered at reading size."
    - type: "statement"
      eyebrow: "Built with Folioh, on Folioh"
      text: "If it breaks, our own docs break first."
      accent: "our own docs"
      actions:
        - title: "Read the docs"
          href: "/docs"
```

All hrefs in these sections pass through the same scheme policy as other
configured links; an unsafe value (for example `javascript:`) degrades to the
section default with a warning instead of failing the build.

## Comparison Section

`landing.comparison` adds a feature matrix to the landing page. The table is
yours: you name the tools across the columns and write every row, so the
section says what your project wants to say about its own field.

```yaml
landing:
  comparison:
    caption: "Capability"          # optional label for the top-left header cell
    tools: ["Your tool", "Alternative A", "Alternative B"]
    rows:
      - feature: "Generated API reference"
        values: [true, true, false]
      - feature: "Static export"
        values: [true, "~", false]
        note: "optional gloss under the feature name"
```

Each row needs a `feature` name and exactly one entry in `values` per name in
`tools`. A cell is `true` (yes), `false` (no), or `"~"` (partial); the strings
`yes`, `no`, `true`, and `false` are accepted too, and anything else reads as
partial so a typo never turns into a claim about a named tool. Quote the tilde:
a bare `~` is YAML's null, which also reads as partial.

Malformed config degrades instead of failing the build. A row whose value count
disagrees with `tools` is dropped with a warning, because its cells would
otherwise slide under the wrong column. Rows without a `feature` and rows whose
`values` is not a list are dropped quietly. A `comparison:` mapping left with no
usable row renders nothing and warns.

The same keys work on a `comparison` entry in `landing.sections`, next to the
usual `eyebrow`, `title`, and `description` heading fields:

```yaml
landing:
  sections:
    - type: "comparison"
      title: "Where this fits"
      tools: ["Your tool", "Alternative A"]
      rows:
        - feature: "Runs offline"
          values: [true, false]
```

The two forms place the table differently. A `comparison` entry in
`landing.sections` renders where you put it. The top-level `landing.comparison`
key feeds the default section list instead, which a site uses only when it sets
no `landing.sections` and its hero variant is `source-pipeline`.

### Deprecated: `comparison: true`

`landing.comparison: true` renders Folioh's own built-in matrix, which names the
documentation tools Folioh compares itself against. It still works and warns on
every build; it will be removed. A `comparison` section with no `tools` and
`rows` falls back to the same built-in matrix and warns the same way. Replace
both with your own `tools`/`rows` table.

## Live demo

This site uses the plugin itself: the [landing page at the site root](/) is rendered from the `landing:` section of this repository's `docs.yaml`.

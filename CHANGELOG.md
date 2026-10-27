# Changelog

All notable user-facing changes are recorded here. Folioh follows semantic
versioning while the public CLI, configuration, and plugin contracts stabilize.

## 0.3.5 — 2026-10-27

### Added

- Documentation directories can order sidebar children with `_meta.yaml`; builds and live sessions refresh metadata without a compiled-in page list.

- Verbose builds list skipped template injection targets and their reasons.

- Exported 404 pages offer documentation search when its index is available.

### Fixed

- Repository workflow intermediates expire after one day and successful deployments and releases remove their consumed artifacts.

- Preview workflow Rust caches use explicit prefixes distinct from main and release jobs, including fallback restores.

- Client navigation from a nested missing URL keeps Next.js stylesheets and later JavaScript chunks rooted at the deployment path.

- Pastel social-card monograms render with the image renderer’s font, avoiding missing glyphs on build hosts without desktop fonts.

- Rust documentation reads literal `include_str!` files and conditional `cfg_attr` doc values, warns on missing files, and refreshes included prose during warm builds and serve sessions.

- Catalog links respect custom docs routes. Default landing API links require a published reference index, and relative link checking shares the renderer’s canonical path calculation.

- Embedded examples inherit the parent social-image origin and deployment path, omit generated images without a public origin, and refresh when that origin changes. Language and landing guides link their examples.

- reStructuredText docstring notices render as callouts, including labelled unknown directives, while preserving deprecation versions and code.

- API pages name the trait and target type in impl headings, omit placeholder module summaries and render dunder names literally in prose.

- Interactive initialization asks for the docstring style only when Python sources are detected.

- Roadmap project keys other than `docs` use a neutral marker consistently.

- Generator documentation renders Yields with the yielded item type while preserving Python iterator return annotations in signatures. JavaScript reads `@yields` and `@yield` into the same IR distinction.

- The previews navbar shows the configured project logo. A missing custom favicon fails the build with its requested path instead of silently using the default icon.

- Theme bootstrap data is shared in a cacheable, content-versioned asset instead of being repeated in every HTML page. Saved themes still apply before hydration.

- Unix development servers stop their child process tree on interruption and print `Server stopped.`; init cancels terminal and piped prompts without leaving raw mode or writing files.

- Static builds report missing heading and API fragments with source filenames and lines, including anchors removed in warm builds.

- HTTP navigation keeps directory URLs after gallery and sidebar hops while
  exported file links retain their index targets. The fallback 404 loads search
  and navigation scripts from the deployment root.

- Removing an integration section removes its generated pages, Markdown mirrors
  and standalone routes on the next warm build while preserving authored pages.

- Authored guides named `i18n.md` or `versioning.md` publish normally while the
  corresponding experimental configuration features remain disabled.

- Guides with underscores in their filenames or directories render their content
  at canonical and alias URLs instead of a not-found page. Markdown download
  links work at both URLs; ambiguous underscore/hyphen routes report a conflict.
- Builds, development servers and cleanup commands refuse to rewrite a workspace
  held by another Folioh process, report its PID and serve port, and recover
  leftover owner records after the lock is released.

## 0.3.4 — 2026-10-19

### Added

- The Components guide includes live examples with links to each component guide.

### Changed

- The theme picker defaults to five Pastel palettes. Set `theme.extra_presets: true`
  to enable the other bundled presets; explicit project presets remain available.
- Feedback requires explicit activation and uses a compact icon with a hover and
  keyboard-focus label. The icon motion respects reduced-motion preferences.
- Pastel social cards use the configured light palette and shared artwork.
  Preview metadata points to the deployed site's own images.
- Header navigation uses the Docs label and landing roadmap summaries are shorter.

### Fixed

- Gallery examples scale and center within their cards, with contained decorative
  shapes and one background through the guide link.
- Gallery code examples use the normal syntax highlighter and copy controls.

## 0.3.3 — 2026-10-09

### Fixed

- The README wordmark uses the original Pastel silhouette beside Folioh in
  light and dark mode.

## 0.3.2 — 2026-10-09

### Changed

- The terminal and web ASCII wordmark now reads FOLIOH!, with six rows of
  51 columns. The terminal banner remains centered.

### Fixed

- Removed the extra separator between O and H in the ASCII wordmark.

## 0.3.1 — 2026-10-07

### Changed

- The product and public repository are Folioh. The website is
  [folioh.site](https://folioh.site).
- The command and release binary are `folioh`. Workspace crates use the
  `folioh-*` prefix, environment variables use `FOLIOH_*`, and generated paths
  and template namespaces use `folioh`.
- GitHub Pages previews use the configured Pages domain and base path.
- To upgrade an existing Folio installation, run the installer at
  `https://folioh.site/install.sh` and use `folioh`. Existing `folio`
  installations are retained. Use `FOLIOH_*`
  environment variables with the new command. Older binaries retain their
  previous command, settings and release archive names.

## 0.3.0 — 2026-10-04

### Added

- `theme.logo` shows in the landing navbar in place of the monogram.
- The navbar theme picker offers fifteen choices in three families: six Folioh
  styles, five Folioh Pastel palettes and four Omarchy palettes. Each swatch applies
  a complete color-and-style recipe. Project and registered presets keep all their
  choices; earlier saved styles and palettes continue to render unchanged.
- Customize opens from the final circle in the swatch row, inside the same
  popup with a live preview. It offers background, text and accent colors
  per mode, plus typography, rhythm, corners, borders and layout. Choices
  apply and save immediately; Done closes the picker and keeps them.
- Motion and drawer CSS variables: `--folio-motion-*`, `--folio-ease-enter`,
  `--folio-ease-exit`, `--folio-spring-drawer` and `--folio-drawer-*`. A
  theme sets `--folio-drawer-inset: 0`, `--folio-drawer-width: 100vw`,
  `--folio-drawer-radius: 0` and `--folio-drawer-shadow: none` for a
  full-screen menu.
- The Omarchy preset, inspired by [Omarchy](https://omarchy.org): one theme
  with four selected palettes, each light or dark only,
  with flat square navbar and chapter rows, a palette-tinted pixel background
  and a block-letter Folioh wordmark, also visible in picker previews.
  The pixels flow, follow the mouse with a soft halo and release a wave on
  click or hold. The wordmark cycles through four reconstructions on entry,
  palette change or replay by click, Enter or Space. Omarchy palette changes
  have a brief diagonal transition in supporting browsers. Reduced motion
  leaves the artwork and palette changes still; hidden or offscreen artwork
  pauses, and neighbouring previews stay static.
  Other projects keep their own name. The palettes come from the
  MIT-licensed omacom/omarchy repository; `THIRD-PARTY-NOTICES.md` carries
  its notice.
- The Terminal typography (`fontId: "terminal"`): a monospaced body under
  Geist headings.
- Folioh Pastel is the default for new sites and `folio init`, with Ink's
  pen-blue palette, Grotesque typography and 0.75rem corners. Ink, Jade,
  Lavender, Peach and Sky support light and dark with the same shapes.
  Existing reader choices survive the default change; older saved Pastel
  choices without a palette keep Jade. Project defaults still override
  preset defaults. Its palettes and irregular silhouettes are Folioh's own; layout,
  measurements, motion timings and component CSS adapt [cojeev](https://github.com/luv-jeri/cojeev-ui).
  `THIRD-PARTY-NOTICES.md` retains its MIT notice and the fonts' OFL notice.
- Pastel draws the docs shell after cojeev: a 288 px floating sidebar with
  integrated branding, search and controls, and a reading trail that marks
  the section in view. Three original, soft asymmetric silhouettes morph,
  drift and respond to the pointer above a soft color wash. The landing
  places them in solid, contrasting palette colors around the product
  demo, outside the title and text flow; content and actions stay intact.
  Reduced motion keeps them still; hidden and offscreen artwork pauses,
  and neighbouring previews stay static. The picker shares the page's
  layout, type and artwork.
- Pastel callouts keep their functional icons on soft asymmetric holders.
  FeatureCard keeps named Hugeicons centered over four original organic
  backgrounds, alternating with the card colors; text/emoji fallback and
  cards without icons stay unchanged. Decorative masks, number tabs and
  timeline markers use original organic outlines, with ruled connectors
  and FileTree elbows.
- Pastel tabs, code groups, example tabs and the Preview/Code switch are
  pills with a selected surface that glides between them, and accordions and
  Markdown details are soft cards that open in 300 ms and close in 180 ms.
- Pastel code blocks are paper sheets with a file header and language pill,
  with five syntax roles in each scheme. TerminalSession is a petrol window
  with three stationery tabs. Cards, pill badges, dot-grid previews, dashed
  empty states and flush tables retain the adapted component layout.
- Under Pastel, switching between light and dark reveals the new scheme as
  a circle growing from the control over 300 ms, where the browser supports
  view transitions. Under reduced motion the scheme changes at once.
- The Grotesque typography (`fontId: "grotesque"`): Bricolage Grotesque
  headings over a DM Sans body, from Google Fonts. The files are not
  preloaded, so a site on another typography downloads neither.
- One CSS file per component family under `app/styles/`, imported at the
  end of `globals.css`, so a theme package can restyle a family by
  overriding one file. The MDX components carry `data-slot` attributes, and
  `data-variant` where they come in kinds (a callout's type, a checklist
  item's state), for that CSS to target.
- Kbd, Marker and Term, in every theme: keycaps (a bare `<kbd>` in page
  content too), labelled dividers between two dashed rules with `ok` and
  `danger` tones, and inline definitions that open on hover, focus or tap.
  Under Pastel their adapted component styles follow the chosen palette.
- Tone variables for the six callout types, `--folio-tone-<type>-fill`,
  `-ink` and `-accent`, and `--folio-surface-radius` for the corner of a
  raised surface.

### Changed

- Folioh styles use stationery names: Letterpress, Ballpoint, Notebook,
  Paperback, Roller and Typewriter. Existing preset IDs and saved preferences
  remain compatible; Roller remains an explicit choice.
- Callouts draw six distinct tones, one per type, from the tone variables on
  every preset; `note`, `tip` and `check` no longer share the primary color.
- On a phone, the docs menu is a drawer that floats 12 px from the screen
  edges and springs in from the left over a dimmed page. It is a modal
  dialog with a title and a close button: Esc, the close button and a tap
  outside close it, focus stays inside it while it is open, and the rest of
  the page is inert. Under reduced motion, docs pages drop their transitions
  and animations at every width; otherwise desktop does not change.
- The theme drawer in the docs sidebar is removed; its controls are the
  picker's Customize step, on every page and on phones. `ThemeConfigurator`
  renders nothing and stays exported, so a layout that mounts it still
  builds.
- The landing no longer shows its own light/dark button;
  `theme.header.theme_toggle: true` adds one to the landing and the docs
  navbars alike.
- The carousel arrows and swipes browse each color-and-style variant in
  swatch order, continuing across families. Neighbouring previews show
  the recipes they select. Navigating applies the selected theme to the page
  immediately, without a separate confirmation.
- Choosing a variant restores its configured style. The site preset uses
  `theme.tune` and `theme.radius`; Reset in Customize clears manual changes.
- Manual colors also work on fixed-scheme Omarchy palettes and persist
  through the initial theme bootstrap without a color flash.
- A theme package that overrides `components/theme-configurator.tsx` keeps
  the exports the picker imports; the root layout mounts
  `ThemeStyleBootstrap`, and page overrides no longer need their own.
- A theme package whose `theme/project-theme.ts` exports
  `projectThemeDefaultConfig` as an untyped literal no longer fails the
  frontend type check. The bundled configurator reads it with every key
  optional, so the literal may leave keys out ("does not exist on type") and
  may set `customization` keys the configurator also defaults, such as
  `fontId` or `colorId` ("specified more than once").

### Fixed

- Theme previews show real text, section rules and workspace frames, so
  typography and spacing differences remain visible alongside colors.
- Going from the docs to the landing page by the logo keeps the reader's
  theme.
- The landing navbar takes the theme's navbar height, and landing headings
  and code boxes follow the theme's heading font and code background.
- Under `folio serve`, a docs page no longer turns into "Element type is
  invalid" when it is reloaded or opened from the landing page. Pages under
  `folio serve` carry no "Last updated" line; a build still shows it on
  authored pages.

## 0.3.0-a1 — 2026-09-25

### Added

- Folioh is one native Rust binary, `folio`, installed by `install.sh` from
  GitHub Releases. It replaces the `folio-docs` Python package and needs no
  Python; `folio build` and `folio serve` still render through the bundled
  Next.js template with Node.js 20.19+ and pnpm 10.
- The commands: `folio init`, `build`, `serve`, `coverage`, `clean` and
  `roadmap`, with the hidden `github-pages` group behind the preview
  workflows.
- Built-in integrations: landing page, roadmap and OpenAPI are compiled in and
  activate through their `docs.yaml` sections (`landing:`, `roadmap:`,
  `openapi:`).
- `public:` lists project-relative files served verbatim from the site root;
  a missing file, or a path outside the project or inside `.build/` or the
  output directory, fails the build.
- The Next.js template ships inside the binary and is materialised once under
  the user cache directory; `FOLIO_TEMPLATE_DIR` overrides it.
- `folio --update` replaces the binary with the latest GitHub release: it
  follows `releases/latest`, downloads the platform archive and `SHA256SUMS`,
  verifies the archive and swaps it in place with the installer's tools,
  `curl` and `tar`. `FOLIO_REPO` is shared with the installer;
  `FOLIO_RELEASES_URL` points it at a mirror.
- JavaScript sources: `source.javascript.paths` reads `.js`, `.mjs` and `.cjs`
  statically, without Node or a bundler. A module is named after its path
  (`utils/index.js` is `utils`), and the ESM and CommonJS exports of each file
  — functions, generators, classes with their getters, setters and static
  members, and constants — are published with their JSDoc under
  `api-reference/javascript/`. A setter beside its getter reads as the
  getter, which takes the setter's JSDoc when it has none of its own, and a
  static member that shares its name with an instance one has its own anchor.
  `node_modules` is never read and `.jsx` is skipped with a warning.
- Rust sources: `source.rust.paths` reads crates statically, without cargo or a
  Rust toolchain. Folioh takes the crate name from `Cargo.toml`, follows the
  `pub mod` declarations from `lib.rs` or `main.rs`, and publishes public
  functions, structs, enums, traits, impl blocks and type aliases with their
  signatures, attributes and `///` comments under `api-reference/rust/`.
  `folio serve` watches `.rs` files, and a syntax error fails the build naming
  the file.
- `theme.package` takes a mapping naming a git repository, a revision and a
  `sha256:` digest, so a theme someone else published installs in four lines
  instead of a copied directory. The package is fetched into the user cache
  once, keyed by the digest, and the tree is verified against it before any
  build reads it. A local package still works exactly as before.
- A Troubleshooting guide for the messages a build most often stops or
  warns on — the environment check, `Config file not found`, unknown config
  keys, a stale incremental build, the port, broken links and the export
  log — each with its cause and the page that fixes it. Architecture gained the agent-facing surface it always
  emitted: `llms.txt`, the Markdown mirrors and `_folio/contract.json`.
- Agent-readable output from the same build as the HTML: `llms.txt`,
  `llms-full.txt`, a Markdown mirror per page, declared as a `text/markdown`
  alternate and listed in the sitemap, and the authoring contract at
  `/_folio/contract.json`. `folio serve` publishes the same files as a static
  build.
- Custom components (`components:`), custom templates (`template.path`),
  template overlays (`template.overlay_path`), project themes, Pagefind search,
  GitHub Pages branch previews and the MDX component library.
- `folio init` detects Rust (`Cargo.toml`) and JavaScript (`package.json`)
  projects as well as Python ones and writes their source roots; an existing
  `docs/` without Markdown gets an `index.md`.
- A typo in a top-level `docs.yaml` key or inside a core section is reported
  with the key it most likely meant; the keys inside `landing:`, `roadmap:`
  and `openapi:` are not checked yet, apart from a roadmap phase's. An unknown
  landing section type or hero variant warns.
- `theme.logo` shows in the docs navbar under the base path; a logo file that
  does not exist stops the build.
- A missing page is a themed 404 with a way back, at any depth, and a docs
  root without an index page opens the API reference.

### Changed

- Folioh is now licensed under MIT (previously AGPL-3.0-only).
- Release archives ship THIRD-PARTY-NOTICES.md next to LICENSE, with the
  licenses of the bundled shadcn/ui components and of the code adapted from
  docstring_parser (MIT) and CPython (PSF License Version 2).
- Docs pages no longer show an "Edit this page" link, and the feedback link
  opens an issue in `project.repo`. Without a `repo`, neither link is shown;
  both used to point at Nextra's own repository.
- `folio serve` no longer builds the example projects under `docs/examples/`
  on its own: each one is a full nested site, and a cold `serve` of Folioh's own
  repository spent minutes on four of them before the dev server came up. The
  `Previews` row says what was skipped, `folio serve --previews` builds them,
  a `DocPreview` whose example is not built says so in its place, and
  `folio build` still builds every example. The nested builds share the
  workspace's `node_modules` through a link instead of installing their own.
- A full build no longer rebuilds every preview example. Each published
  example carries the digest of its sources, the Folioh version, the template
  and the base path it is built for; one whose digest still matches is left
  alone, and the `Previews` row reports what was rebuilt, what was unchanged
  and what was swept. An example is a
  whole nested build, so a warm build of Folioh's own site stops paying for
  four of them.
- The `Dependencies` row names the phase it is in while it runs, rather than
  reading `checking pnpm` through a `pnpm install` that can take minutes.
- `folio serve` coalesces source events into one batch per save, reuses
  unchanged search-index work, and writes a generated file only when its bytes
  change.
- The repository is flat: the crates, the guides, the template and the theme
  sit at the root, and every crate keeps the tests and the fixtures those tests
  read. The `folio` binary is built by `folio-cli`.
- `Plugin::name` and `Plugin::config_keys` return owned values, and the
  extension-registry types serialize, so an extension can be described as data.
  Nothing loads dynamically yet; this is groundwork for the plugin seam.
- A theme package may no longer ship `package.json`, `pnpm-lock.yaml` or
  `pnpm-workspace.yaml`; a fetched one may not ship `next.config.mjs` either.
  A `components:` entry may no longer point outside the project directory.
- The theme package digest that keys the incremental build context is computed
  with a length-prefixed encoding, so it identifies one tree and only one. The
  first build after upgrading recomputes it once.
- Config keys are typed: string and boolean keys are checked, an explicit
  `null` for a string or boolean key is an error (a `null` `project.name`
  warns and falls back to `Untitled`, and `nav`, `public`, `template.path` and
  `source.docs` read `null` as absent), and each field reports its own
  message. A `plugins:` key warns and is ignored; the authoring contract's
  `configKeys` never lists it.
- `output:` is rejected when it names, or contains, a configured source
  directory or the repository's `.git`, or sits inside `.git`: the build
  removes the output directory before writing to it.
- The `nav` list orders real top-level entries and ignores unknown labels.
- Landing defaults name no `pip install` command and no `.py` kicker; an
  omitted `install:` renders no install block and the third default card reads
  "Built-in Integrations".
- The roadmap groups phases by `project`; the generated `/docs/roadmap/` page
  is a `# Roadmap` heading over the timeline.
- `--version`, the banner and generated metadata read `0.3.0-a1`.
- The landing `funnel` plate draws its sources and outputs as compact tiles —
  an icon and a plain label, in two columns with alternating margins — instead
  of path cards; `guarantees`, `caption` and `command_notes` are retired and
  warn.
- Relative links between pages resolve to their docs route when the page is
  written, a relative link whose `..` climbs out of the docs is reported as
  broken, and the link check reads the `href` props of components too.
- Underscore route aliases name the hyphenated route as their canonical (and
  are `noindex` when no `project.url` is set); they stay out of the search
  index, and so do the page actions.
- A `theme.preset` or `tune` value the configurator does not offer stops the
  build naming the nearest one; `dark_mode: false` keeps the site light with
  no mode controls.
- A roadmap phase the renderer cannot read fails before the build, naming the
  entry: a missing or non-string field, a `status` other than `shipped`,
  `active`, `next` or `later`, a key the phase does not take, or a `features:`
  that is not a list of strings and `text`/`done` mappings. A release links
  nowhere outside the roadmap.
- `folio serve` refuses a busy port before the build starts, `folio build
  --port` without `--open` is a usage error, and `folio clean` on a directory
  that does not exist is an error instead of nothing to clean; an output
  inside `.build/` is cleaned with it.
- `CodeGroup` labels its tabs from each block's language.
- API pages carry more of what the source documents: constants, re-exports,
  class attributes, notes and decorators render on the page and in
  `llms-full.txt`; warning, see also, references, todo and deprecated sections
  read as labelled prose; Sphinx roles such as `:class:` read as code; and
  every cross-reference lands on an id its page writes. The Markdown mirror of
  an API page keeps its class cards and parameter tables.
- A type in a Python or JavaScript signature links in the other ways it is
  commonly written: quoted (`'Config'`), under a dotted generic
  (`typing.Optional[Config]`), and in JSDoc as `Config[]`, `?Config`,
  `Config|null`, `Array<Config>` or `Promise<Config>`. A name defined only in
  another language's modules no longer links.
- Python: private names, undocumented dunders, overload stubs and property
  setters stay out of the reference, and a name defined twice is documented
  once, as its last definition.
- A page's inferred description reads a link as its text, so the page meta and
  `llms.txt` carry no raw Markdown and a broken link in the first paragraph is
  reported once.
- JavaScript: `module.exports` of a function documents the function, local
  declarations exported by name, default or object are documented, `{@link}`
  reads as its text and `options.name` parameters get their own row. A root
  of `.` reads neither `dist/`, `build/` nor the build output, TypeScript files
  are reported instead of skipped in silence, and when two files publish one
  module the first is read and the other is named in a warning.
- Rust: only plain `pub` items are documented, never `#[doc(hidden)]` or an
  impl of a private type or trait, and doc comments read as rustdoc renders
  them: untagged code blocks are Rust, hidden lines are dropped and intra-doc
  links keep their text.

### Removed

- The `folio board` command and the `kanban:` integration are gone: `folio`
  no longer reads `agents.yaml`, a `kanban:` section in `docs.yaml` warns and
  is ignored, and `KanbanBoard` is no longer a page component.
- `PageFeedback`, which no page rendered.
- A roadmap phase no longer takes `milestone`; a phase that still carries it
  fails the build.
- The landing `boards` section, which now warns and renders nothing, and the
  funnel's `board` icon, which is dropped.

### Security

- `folio init` quotes the project metadata it detects, from `pyproject.toml`
  and the git remote, into the generated `docs.yaml`, so a cloned repository
  cannot close the quote and append keys to the config Folioh then trusts.
- `folio clean` never deletes a source root or anything that is not a
  directory, and `folio serve --kill-existing` stops only the process
  listening on the port.
- An `openapi.sources[].path` must stay inside the project, and each source's
  `route` is slugged segment by segment with no `.` or `..` segment; a route
  with no segment left takes the default.
- The project name, url and repository are escaped for each template file
  they land in.

### Not available in this release

- TypeScript sources, project plugins, i18n, and multi-version builds. The
  configuration guide and the pages that mention them carry the notice; the
  `i18n:` and `versions:` keys warn and are ignored, and the authoring
  contract does not offer them.
- `folio coverage` reads Python sources only, and refuses a config without
  them.
- MCP and a public IR export remain future work.

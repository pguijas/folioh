# Roadmap

The built-in roadmap integration renders source-defined phases as a release timeline. The view below reads the same data as the standalone `/roadmap/` route, which is optional: `routes.public: true` turns it on.

The roadmap integration is compiled into `folioh`: nothing to install, nothing to list. It stays inert until a `roadmap:` section appears in `docs.yaml` — the config key is the activation switch.

Activate it in `docs.yaml`:

```yaml
roadmap:
  routes:
    docs: true
    public: true
  phases:
    - id: "foundation"
      version: "0.1"
      project: "docs"
      title: "Foundation"
      status: "shipped"
      layer: "Source analysis"
      summary: "Parse Python source and docs into a static site."
      command: "folioh build"
      features:
        - "Parser"
        - "Search"
```

The timeline is rendered from source: change phases in `docs.yaml`, commit the file, and rebuild the site to update this view. Give phases a `project` to render independent release sequences from one roadmap.

Routes are opt-out: `routes.docs` (default `true`) publishes a generated `/docs/roadmap/` page when your sources do not already contain one, and `routes.public` (default `false`) adds the standalone `/roadmap/` page. The generated page is a `# Roadmap` heading over the timeline; write your own `roadmap.md` to replace it. The `folioh roadmap` CLI command prints the configured phases as a table.

`phases` must be a list of mappings. A phase takes `id`, `version`,
`project`, `title`, `status`, `layer`, `summary`, `command` and `features`,
and no other key. `id`, `version`, `title`, `status`, `layer` and `summary`
are required strings: quote a version (`"0.1"`), or YAML reads it as a
number. `status` is one of `shipped`, `active`, `next` or `later`, `project`
and `command` are optional strings, and `features`, when present, must be a
list. Any other shape stops the build with an error that names the entry, for
example `roadmap.phases[0].features must be a list (got string)`. A phase
without `features` renders with an empty feature list.

The optional top-level `description` is the page's own copy: the `/roadmap/`
page shows it under the title, above the releases. Releases are ordered by
version, never by status.

Each feature can be a string or an object with `text` and an optional `done`
boolean. Strings inherit completion from the phase: features in a `shipped`
phase are complete. Use `done` to mark individual work within an active phase,
or to override that inherited state:

```yaml
features:
  - "Inherits the phase status"
  - text: "Source batches"
    done: true
  - text: "Native performance comparison"
    done: false
```

## Naming the projects

Give a phase a `project` and the plugin groups the roadmap by it. On the
standalone `/roadmap/` page each project becomes a card, and every card holds
that project's whole plan: the release list on the left, the release you pick
opened beside it. Projects are numbered independently, so two projects that
ship separately each start at their own 0.1.

An optional `projects:` block names them. A `label` titles the card and a
`description` sits under it; without either, the project key is shown as
written.

The `docs` key uses a page mark; other project keys use a neutral dot.

The plugin also writes the `projects:` block to `lib/roadmap-projects.ts` as
`roadmapProjects`, keyed by project, whether or not `routes.public` is on. A
page that draws a project card outside the `/roadmap/` view (a landing, or a
theme page) reads the label and description from there, so they stay in
`docs.yaml`.

```yaml
roadmap:
  routes:
    public: true
  projects:
    engine:
      label: "Engine"
      description: "The library the CLI drives."
    cli:
      label: "CLI"
  phases:
    - id: "foundation"
      project: "engine"
      version: "0.1"
      # ...
```

Every card is expanded by default, and each one can be collapsed to its
heading. `?product=<key>` opens one card and collapses the rest, which is the
link a project's own landing page should use:
`/roadmap/?product=cli`. A release is addressable too, scoped by project:
`/roadmap/#cli-0.2`.

Collapsing hides a card's releases; it does not drop them. Every release is in
the served HTML whatever the page is showing, so a crawler or a reader with
JavaScript off still sees the whole roadmap. The page's Markdown mirror and
`llms-full.txt` do not carry the timeline yet.

Internally, the integration uses the same extension primitives as the other built-in plugins: it registers a `Roadmap` component for embedding and a `RoadmapPage` component for the standalone route, writes two typed data modules (`lib/roadmap-data.ts` and `lib/roadmap-projects.ts`), and, when `routes.public` is on, creates the `/roadmap/` view with the required `folioh.public` layout.

## Live demo

This is Folioh's own roadmap — the code tab shows the shape of the `roadmap:` section in this repository's `docs.yaml`, the preview tab is the component rendering the real data:

<PreviewCode title="Folioh's release line" defaultMode="preview">

```yaml
roadmap:
  routes:
    docs: false        # this guide page owns the docs demo
    public: false      # this site has no standalone /roadmap
  projects:
    docs:
      label: "Folioh Docs"
  phases:
    - id: "foundation"
      version: "0.1"
      project: "docs"
      title: "Foundation"
      status: "shipped"          # shipped | active | next | later
      layer: "Source analysis"
      summary: "Python first: source in, documented site out."
      features: ["Parser", "API reference pages"]
    - id: "extension"
      version: "0.3"
      project: "docs"
      title: "One Binary"
      status: "shipped"
      layer: "One platform"
      summary: "One native binary reads Python, JavaScript and Rust; the built-in plugins share one contract."
```

<Roadmap />

</PreviewCode>

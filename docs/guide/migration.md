---
title: Migrating from Sphinx
description: Move a Sphinx project's guides and API documentation to Folioh, configure sources, and verify the generated site.
---

# Migrating from Sphinx

*Step-by-step guide to moving a Sphinx project to Folioh by hand.*

A migration run by your agent, with a migration skill and a few scripts: Not available
in this release; the roadmap puts it in 0.4 Agent Ready. [Why Folioh](./why-folioh) covers
the reasons to switch; this page covers the move.

## Step-by-step migration

### 1. Install Folioh

Follow [Installation](./installation).

### 2. Initialize configuration

Run the init command in your project root. It reads the project name and version
from `pyproject.toml` and finds the source directory on disk:

```bash
folioh init
```

This creates a `docs.yaml` file. Edit it to point to your Python source directories:

```yaml
project:
  name: "my-project"
  version: "1.0.0"
  repo: "https://github.com/org/my-project"

source:
  python:
    paths:
      - "src/my_project/"
    exclude:
      - "src/my_project/_vendor/"
  docs:
    - "docs/"

nav:
  - "Guide"
  - "API Reference"
```

### 3. Convert existing documentation

Keep existing Markdown pages and review any directives or components specific to
your previous documentation tool.

If you have hand-written documentation in `.rst` format, convert it to Markdown before
adding it to `source.docs`. Folioh does not ship or invoke an RST converter; use your
migration tool of choice, review the generated Markdown, and then let Folioh build the
converted pages.

Folioh warns about `.rst` files only when those files are still present inside a
directory listed in `source.docs`. It does not emit a warning if the `.rst` files were
already removed before running `folioh build` or `folioh serve`.

Guide files and directories may contain underscores: `common_errors.md`
publishes at `/docs/common-errors/`, with `/docs/common_errors/` serving the same
content. The original mixed spelling used by source links also works. Avoid two guides
whose routes differ only by underscores and hyphens: they share a canonical URL,
so the build reports a conflict. Old Sphinx `.html` URLs are not redirected.
API reference routes keep Python package and module names unchanged, including
their underscores.

### 4. Leave your docstrings alone

Your docstrings need no conversion. With `docstring_style: "auto"`, the
default, Folioh detects the style per docstring and reads reStructuredText
(`:param name:`), Google, NumPy and epydoc. Set `source.python.docstring_style`
only to pin one style for a codebase that mixes them by accident.

### 5. Remove the previous documentation setup

Once migration is verified, remove configuration and dependencies used only by
your previous documentation tool. For a Sphinx project, these may include:

- `conf.py`
- `Makefile` (if only used for docs)
- `make.bat`
- `_build/` directory
- `_static/`, `_templates/` directories
- Sphinx from your dependencies

### 6. Build and verify

```bash
folioh serve
```

This starts a dev server at `http://localhost:4321` where you can verify your
documentation looks correct.

## Mapping Sphinx concepts to Folioh

### conf.py to docs.yaml

| Sphinx `conf.py` | Folioh `docs.yaml` |
|-------------------|-------------------------|
| `project = "Name"` | `project.name: "Name"` |
| `version = "1.0"` | `project.version: "1.0"` |
| `extensions = ["autodoc"]` | Not needed (automatic) |
| `html_theme = "furo"` | Built-in theme presets (see [Theming](./theming/index)) |
| `html_static_path` | `public:` for files served from the site root; images a page uses go beside the page |
| `html_logo` | `theme.logo` |
| `html_favicon` | `theme.favicon` |
| `exclude_patterns` | `source.python.exclude` |
| `autodoc_member_order` | Not configurable (source order) |

### autodoc to automatic generation

In Sphinx, you write explicit directives to pull in API documentation:

```rst
.. automodule:: my_project.core
   :members:
   :undoc-members:
   :show-inheritance:
```

In Folioh, you list your source directories and everything is documented
automatically:

```yaml
source:
  python:
    paths:
      - "src/my_project/"
```

No per-module directives needed. Use `__all__` in your Python modules to control which
symbols appear in the documentation.

### RST directives

Folioh does not translate RST directives during a build. Convert them before adding the
page to `source.docs`, then review common constructs against this mapping:

**Common manual rewrites:**

| RST Directive | Converted To |
|---------------|-------------|
| `.. code-block:: python` | Fenced code block (` ```python `) |
| `.. note::` | Callout (note) |
| `.. warning::` | Callout (warning) |
| `.. tip::` | Callout (tip) |
| `.. hint::` | Callout (tip) |
| `.. danger::` | Callout (danger) |
| `.. error::` | Callout (danger) |
| `.. important::` | Callout (warning) |
| `.. caution::` | Callout (warning) |
| `.. attention::` | Callout (warning) |
| `.. seealso::` | Callout (info) |
| `.. deprecated::` | Callout (danger) with version |
| `.. versionadded::` | Callout (note) with version |
| `.. versionchanged::` | Callout (note) with version |
| `.. image:: path` | `![path](path)` |
| RST headings (underlines) | Markdown headings (`#`, `##`, etc.) |
| `` ``inline code`` `` | `` `inline code` `` |
| `` :role:`text` `` | `` `text` `` |

**Features that need manual redesign:**

| RST Feature | Status |
|-------------|--------|
| `.. toctree::` | Not needed (auto-generated navigation) |
| `.. include::` | Not supported |
| `.. math::` | `$$` block, rendered with KaTeX (inline: `$…$`) |
| `.. table::` | Use Markdown tables instead |
| `.. raw::` | Not supported |
| `.. tab-set::` / `.. tab-item::` | Convert manually to `<Tabs>` and `<TabItem>` |
| `.. only::` | Not supported |
| Cross-references (`:ref:`, `:doc:`) | Not available in this release; use Markdown links |
| Substitutions (`\|name\|`) | Not supported |
| Field lists (`:field:`) | Not supported |
| Footnotes | Markdown footnotes (`[^1]`) |

For Sphinx tab sets, rewrite each block with Folioh's MDX tabs components:

````mdx
<Tabs>
  <TabItem label="Python">
    ```python
    import my_library
    ```
  </TabItem>
  <TabItem label="CLI">
    ```bash
    my-library run
    ```
  </TabItem>
</Tabs>
````

For callouts that were written as Markdown blockquotes during migration, use Folioh's
`Callout` component directly. Patterns such as `> **Warning:**` should become
`<Callout type="warning">`. For example, convert `> **Warning:** This changes state.`
to:

```mdx
<Callout type="warning">
  This changes state.
</Callout>
```

### Sphinx extensions to Folioh features

Many common Sphinx extension use cases are built into Folioh.
Custom extensions: Not available in this release.

| Sphinx Extension | Folioh Equivalent |
|-----------------|----------------------|
| `autodoc` | Built-in (automatic) |
| `napoleon` | Built-in (Google and NumPy styles) |
| `viewcode` | Built-in: each definition links to its source line when `project.repo` is set |
| `intersphinx` | Not available in this release |
| `todo` | Not supported |
| `coverage` | Built-in (`folioh coverage`) |
| `doctest` | Not supported |
| `sphinx-copybutton` | Built-in (code blocks have copy buttons) |
| Custom extensions | Not available in this release |

### make html to folioh build

| Sphinx Command | Folioh Command |
|---------------|-------------------|
| `make html` | `folioh build` |
| `make clean` | `folioh clean` |
| `sphinx-autobuild` | `folioh serve` |
| `sphinx-quickstart` | `folioh init` |

## What works differently

### Sphinx roles do not link

Sphinx has an extensive cross-reference system (`:class:`, `:func:`, `:meth:`, etc.) that
creates links between documented symbols. Folioh generates API links for parsed type
annotations; Sphinx role syntax is Not available in this release. In a docstring a role
reads as code, unlinked: `` :class:`MyClass` `` renders as `MyClass`. In a Markdown page
a role stays as written. Replace roles with Markdown links where you need the link.

### No intersphinx

Sphinx's `intersphinx` extension lets you link to other projects' documentation.
Intersphinx is Not available in this release. If your docs link heavily to external
API documentation (for example the Python standard library), replace those links
with plain URLs.

### Source order, not alphabetical

Folioh documents members in the order they appear in the source file. Sphinx's
`autodoc_member_order` option with alphabetical sorting has no equivalent.

### Only the field lists of reStructuredText docstrings

Folioh reads the Sphinx field lists in docstrings (`:param x:`, `:type x:`, `:returns:`,
`:rtype:`, `:raises:`). Other reStructuredText markup in a docstring, such as
substitutions, directives or grid tables, is not converted and shows as written.

## Common gotchas

**Docstring parsing errors.** If no style parses a docstring, Folioh falls back to
the first line as the summary and the rest as the description. The parameter table
still lists the signature's parameters, without descriptions. Indentation matters: in
a Google-style section, a line at column zero ends the section and the lines from
there to the next section header are dropped.

**Missing `__init__` docs.** Folioh documents `__init__` like any other method. If
you were relying on Sphinx's `autoclass_content = "both"` to merge class and `__init__`
docstrings, you may need to adjust your docstrings.

**Private members.** Without `__all__`, Folioh documents every top-level class,
function and annotated variable, and every unannotated assignment to an upper-case
name, when the name does not start with `_`. Define `__all__` in a module to choose
exactly what appears: it can list a class, function or annotated variable whose name
starts with `_`, while an unannotated assignment still needs an upper-case name
without a leading `_`.

**MyST directives disappear silently.** A fenced block whose info string is in
braces, such as MyST's `{note}` or `{eval-rst}`, is removed with its content, and the
build does not warn. Rewrite each one in Markdown or as a component before adding the
page to `source.docs`. An importer that rewrites these blocks, tab sets and
blockquote-style callouts: Not available in this release.

**Raw embeds disappear silently.** The Markdown-to-MDX step removes raw
`<iframe>`, `<script>`, `<style>`, `<video>`, `<audio>`, `<object>`, and `<embed>`
elements, inside code blocks too, and the build does not warn. Use a normal Markdown
link or a documented component instead of a raw YouTube or HTML embed.

**Images stay beside the page.** An image path relative to the page is copied with
it, as long as the file sits in the page's directory or below it. A path with `..`
is removed from the page and the build warns
`image not found: ../img.png (escapes the docs directory)`.

**Curly braces are escaped for you.** Folioh outputs MDX, where a bare `{` starts a
JSX expression, so it escapes bare braces in prose; code blocks, inline code and
`$…$` math pass through untouched. A line that starts with `<` is treated as JSX and
left alone, so a literal brace on such a line needs a backslash (`\{`).

**Markdown details that change.** Links to `.md` files drop the extension and point
at the published page; `class=` on an HTML tag becomes `className=`; a `mermaid`
fence renders as a diagram.

## Feature comparison

| Feature | Sphinx | Folioh |
|---------|--------|-------------|
| Python API docs | Via autodoc extension | Built-in, automatic |
| Docstring styles | Google, NumPy, reStructuredText | Google, NumPy, reStructuredText, epydoc, auto-detected per docstring |
| Configuration | Python (conf.py) | YAML (docs.yaml) |
| Output format | HTML, PDF, ePub, etc. | HTML (Next.js) |
| Theme system | Jinja2 templates | React + shadcn/ui |
| Dark mode | Theme-dependent | Built-in |
| Hot reload dev server | Via sphinx-autobuild | Built-in |
| Search | Built-in | Built-in (Pagefind) |
| Cross-references | Full support | Generated API type links; Sphinx roles: Not available in this release |
| Intersphinx | Full support | Not available in this release |
| RST support | Native | Convert to Markdown before build |
| Markdown support | Via MyST | Native |
| Custom extensions | Extensive plugin ecosystem | Not available in this release |
| PDF output | Built-in | Not supported |
| i18n | Built-in | Not available in this release |
| LLM-friendly output | Not built-in | llms.txt + llms-full.txt |
| Custom components | Jinja2 macros | React/MDX components |
| Setup complexity | High | Low |
| Build speed | Moderate | Fast (Turbopack) |

## Migration checklist

<Checklist
  items={[
    { label: "Install Folioh and run folioh init", state: "todo" },
    { label: "Edit docs.yaml with project details and source paths", state: "todo" },
    { label: "Convert .rst files to .md", description: "Folioh ships no converter: use the tool you prefer, then review the Markdown by hand.", state: "warn" },
    { label: "Replace :ref: and :doc: cross-references", description: "Sphinx roles do not become links: use Markdown links.", state: "todo" },
    { label: "Rewrite {eval-rst} and other MyST directive blocks", state: "todo" },
    { label: "Move html_static_path files to public: or beside the pages that use them", state: "todo" },
    { label: "Run folioh serve and verify every page", state: "todo" },
    { label: "Remove Sphinx configuration files and dependencies", state: "todo" },
    { label: "Update CI/CD scripts to use folioh build", state: "todo" },
  ]}
/>

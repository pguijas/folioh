---
title: CLI Reference
description: Reference for Folioh CLI commands, flags, and workflows including init, build, serve, coverage, and clean.
---

# CLI Reference

*Every command, flag, and option of the `folioh` CLI.*

```
folioh [OPTIONS] COMMAND [ARGS]
```

Running `folioh` with no arguments prints the help text and exits with code `2`, like any usage error: an unknown command or flag, a missing value, or a flag that needs another one (`build --port` without `--open`).

<CommandGrid>
  <CommandCard command="folioh init" title="Initialize" description="Create docs.yaml for an existing project." flags={["--yes"]} href="#folioh-init" />
  <CommandCard command="folioh build" title="Build" description="Generate MDX, search data, LLM files, and static output." flags={["--clean", "--open", "--verbose"]} href="#folioh-build" />
  <CommandCard command="folioh serve" title="Serve" description="Start a local authoring server with source watching." flags={["--port", "--open", "--kill-existing"]} href="#folioh-serve" />
  <CommandCard command="folioh coverage" title="Coverage" description="Measure Python docstring coverage and enforce a minimum threshold." flags={["--min", "--verbose"]} href="#folioh-coverage" />
  <CommandCard command="folioh clean" title="Clean" description="Remove the build cache and the generated output directory." flags={["--config"]} href="#folioh-clean" />
  <CommandCard command="folioh roadmap" title="Roadmap" description="Preview the roadmap phases defined in docs.yaml." flags={["--config"]} href="#folioh-roadmap" />
</CommandGrid>

## Global Options

| Flag | Short | Description |
|------|-------|-------------|
| `--version` | `-V` | Print the installed Folioh version and exit. |
| `--update` | | Replace this binary with the latest GitHub release and exit. |
| `--help` | `-h` | Show the help message and exit. Every command accepts it too, as in `folioh build -h`. |

```bash
folioh --version
```

It prints `folioh <version>`, for example `folioh 0.3.0-a1`. In the `folioh init` example below, `X.Y.Z` stands for the installed version; it is compiled into the binary.

```bash
folioh --update
```

`--update` follows `releases/latest` of `pguijas/folioh` to the newest release tag. When that release is newer than the installed version, the archive for this platform is downloaded together with the release's `SHA256SUMS`, checked against it, unpacked with `tar` and swapped into the binary's own path; the `Installed` row then shows the new `folioh --version`. A binary that is up to date, or ahead of the latest release, reports so and changes nothing. The flag runs alone: `folioh --update build` is a usage error. It needs `curl` and `tar`, the tools the installer uses, and write access to the directory that holds `folioh`. See [Update](./installation#update) for the environment variables it reads.

## Commands

### `folioh init`

Generate a `docs.yaml` configuration file for an existing project. This is the recommended way to get started.

```
folioh init [OPTIONS] [DIRECTORY]
```

| Argument / Option | Required | Default | Description |
|--------------------|----------|---------|-------------|
| `DIRECTORY` | No | Current working directory | Path to the project directory where `docs.yaml` will be created. |
| `--yes` / `-y` | No | `false` | Skip all prompts and use detected defaults (useful for CI/scripts). |

**Interactive wizard:**

When run without `--yes`, `folioh init` shows what it detected, pre-filled from the project scan, and asks for the visual preset. When Python sources are detected, it first asks for the docstring style; Rust-only, JavaScript-only and docs-only projects skip that question.

In a terminal each question is an inline menu. The arrow keys or `j`/`k` move, Return submits, a digit submits that option, and Esc, `q` or `Ctrl+C` abort. The open menu reads `? Docstring style › - Use arrow-keys. Return to submit.` above its options; once submitted, it is replaced by one line with the chosen value. While a menu is open, the tips line under the banner (`· tip ·`) shows the next Folioh tip every second. When stdin or stdout is not a terminal, or `CI` is set, the questions become numbered lists that read one line each; an empty line keeps the default.

The screen at 80 columns after Return on both defaults:

```
$ folioh init

                     ████████╗ ██████╗ ██╗     ██╗ ██████╗
                     ██╔═════╝██╔═══██╗██║     ██║██╔═══██╗
                     █████╗   ██║   ██║██║     ██║██║   ██║
                     ██╔══╝   ██║   ██║██║     ██║██║   ██║
                     ██║      ╚██████╔╝███████╗██║╚██████╔╝
                     ╚═╝       ╚═════╝ ╚══════╝╚═╝ ╚═════╝  vX.Y.Z

             · Theme presets ship polished docs shells by default ·

                ╭────────────────── Detected ──────────────────╮
                │ Target     .                                 │
                │ Project    my-library 1.2.0                  │
                │ Source     src/my_library (Python)           │
                │ Repo       https://github.com/org/my-library │
                │ Python     >=3.10                            │
                │ Framework  Typer package                     │
                │ Status     Documentation scaffold not found  │
                ╰──────────────────────────────────────────────╯

✔ Docstring style › auto
✔ Visual preset   › pastel

  ✔ Created docs.yaml
  ✔ Created docs/index.md
  ✔ Created .github/workflows/pages.yml
  ✔ Created .github/workflows/branch-previews.yml
  Next folioh serve
```

The default visual preset is Folioh Pastel with Ink. Roller remains an explicit choice.

With `--yes` there are no questions: the created files and the next commands are listed in a `Ready` panel, which suggests `folioh coverage` only when a Python source was detected.

**What it does:**

1. Runs `node --version` and `pnpm --version` for the tools found on `PATH`. A missing or too old Node.js or pnpm prints `Environment check failed:` with one line per problem, as a warning: init goes on, since the build is what needs them.
2. Checks if `docs.yaml` already exists. If it does, prints `docs.yaml already exists, skipping.` and exits without overwriting.
3. Reads the project name and version from `pyproject.toml`, `Cargo.toml` (`[package]`, or `[workspace.package]` for the version) and `package.json`, in that order of precedence. Without any, the directory name and `0.1.0` are used. A manifest that cannot be parsed is reported in a warning and skipped. Folioh never runs Python or Cargo: the `Python` row shows the declared `requires-python`, or `not detected`.
4. Detects the GitHub repo URL from `git remote origin` (if available).
5. Detects one source path per language:
   - **Python:** the first existing directory of `src/<package_name>/` (src layout), `src/`, `<package_name>/` (flat layout) and `<project_name>/`. Next to a `Cargo.toml` or `package.json`, the directory must also hold a `.py` file.
   - **Rust:** with a `Cargo.toml`, the root (`./`) of a `[workspace]`; for a single crate, `src/` when it holds `lib.rs` or `main.rs`, else `./`.
   - **JavaScript:** with a `package.json`, the first of `src/` and `lib/` that holds `.js`, `.mjs` or `.cjs` files, else `./` when such files sit at the root. TypeScript is not read in this release.

   The `Source` row lists each path with its language, or `no sources detected`. The `Framework` row names what was found: a Python framework such as `Typer package`, `Rust crate`, `Rust workspace` or `JavaScript package`, joined with ` + ` in a mixed project, or `not detected`.
6. Displays the detected project settings, then prompts for docstring style and visual preset (or uses defaults with `--yes`).
7. Generates `docs.yaml` with the chosen theme preset, a `source` entry for each detected language, and an `API Reference` navigation entry when a source was found. With no source detected, `source.python` is left as a commented hint.
8. Creates `docs/index.md` when `docs/` is missing or holds no Markdown file. A `docs/` with Markdown in it is left untouched.
9. Creates a GitHub Pages workflow at `.github/workflows/pages.yml` if it does not already exist.
10. Creates a GitHub Pages PR preview workflow at `.github/workflows/branch-previews.yml` if it does not already exist.
11. Prints created file paths relative to the current shell directory, so `folioh init tmp/project` reports `tmp/project/docs.yaml`.

**Examples:**

```bash
# Interactive setup
folioh init

# Non-interactive with defaults
folioh init --yes

# Initialize a specific project
folioh init /path/to/my-project -y
```

**Exit codes:**

| Code | Meaning |
|------|---------|
| `0` | Success, or `docs.yaml` already exists (skipped). |
| `1` | The directory does not exist, or the wizard was aborted. Nothing is written. Esc, `q` or `Ctrl+C` print `^C Init aborted. No files changed.`; stdin closing before an answer prints `Init aborted: stdin ended before an answer. No files changed; rerun with --yes to take the defaults.` |
| `130` / `143` | Unix SIGINT / SIGTERM while awaiting an init answer. The prompt aborts without writing files and restores terminal mode, including numbered prompts and unfinished piped answers. |
| `2` | Usage error, such as an unknown flag. |

### `folioh build`

Parse your source code and documentation files, then generate a static documentation site.

```
folioh build [OPTIONS] [DIRECTORY]
```

| Argument | Type | Default | Description |
|----------|------|---------|-------------|
| `DIRECTORY` | `PATH` | Current working directory | Project directory containing `docs.yaml`. |

| Option | Short | Type | Default | Description |
|--------|-------|------|---------|-------------|
| `--project-dir` | | `PATH` | Current working directory | Compatibility option for scripts that prefer named arguments. |
| `--verbose` | `-v` | `flag` | `false` | Show detailed output during the build process. |
| `--config` | `-c` | `TEXT` | `"docs.yaml"` | Path to the config file, relative to the project directory. |
| `--clean` | | `flag` | `false` | Force full rebuild, clearing the `.build/` cache directory. |
| `--open` | `-o` | `flag` | `false` | Serve the built static site locally, open it in your browser, and keep the preview server running until interrupted. |
| `--port` | `-p` | `PORT` | `8787` | Port for the `--open` preview. When it is taken, the next free port is used and its URL printed. Only valid with `--open`: `folioh build --port 9000` alone is a usage error (exit `2`). |

**What it does:**

1. Loads the config file from the project directory.
2. Parses the configured source files and extracts doc comments, class hierarchies, and module structure.
3. Converts Markdown documentation files to MDX.
4. Generates the Nextra site in a `.build/` directory.
5. Runs the Next.js build to produce the final output.

Build output uses the same compact terminal rhythm as `folioh init`: a centered Folioh banner followed by short status rows for sources, template prep, page generation, link checks, dependencies, export, completion, and the ready output path. Warnings stay attached to their step instead of appearing as raw warning output. During static export, Folioh prints the full export log once in a bordered build output panel and also saves it to `.build/.folioh-build.log`.

The incremental cache tracks source file hashes plus config, template, and generator inputs. Changing `docs.yaml`, Folioh's generator code, or the site template invalidates generated pages even when Python and Markdown sources are unchanged.

Generator inputs include the Markdown-mirror and LLM/contract generators, so
changes to those converters also refresh cached source pages.

**Examples:**

```bash
# Build docs for the current directory
folioh build

# Build with verbose output
folioh build -v

# Build a project in a different directory
folioh build /path/to/project

# Use a custom config file
folioh build --config my-docs.yaml

# Build and open a local static preview
folioh build --open
```

`folioh build --open` is intentionally blocking after the build: it starts a static preview server and waits until you press `Ctrl+C`, which ends the process with the interrupt status (`130` in most shells).

Only one build or development server can own a project's `.build/` workspace at a
time. A competing command fails before changing it and names the holder's PID,
command and serve port. `folioh clean` respects the same ownership. Stop that
process or use a different checkout. A claim
left by an interrupted process is recovered automatically with a message.

**Exit codes:**

| Code | Meaning |
|------|---------|
| `0` | Build succeeded. |
| `1` | Build failed. Error details are printed to the console. Common causes: missing config file, parse errors, no source modules or documentation found, or Next.js build failure. |
| `2` | Usage error, such as `--port` without `--open`. |

### `folioh serve`

Build the documentation and start a local development server with hot reloading. This is the primary command during development.

```
folioh serve [OPTIONS] [DIRECTORY]
```

| Argument | Type | Default | Description |
|----------|------|---------|-------------|
| `DIRECTORY` | `PATH` | Current working directory | Project directory containing `docs.yaml`. |

| Option | Short | Type | Default | Description |
|--------|-------|------|---------|-------------|
| `--project-dir` | | `PATH` | Current working directory | Compatibility option for scripts that prefer named arguments. |
| `--verbose` | `-v` | `flag` | `false` | Show detailed output. |
| `--config` | `-c` | `TEXT` | `"docs.yaml"` | Path to the config file. |
| `--port` | `-p` | `INT` | `4321` | Port for the development server. A port already in use is refused before the build starts: `Error: Port 4321 is already in use. Stop the existing process or rerun with --kill-existing.` |
| `--open` | `-o` | `flag` | `false` | Open the documentation in your default browser after the server starts. |
| `--clean` | | `flag` | `false` | Force full rebuild, clearing the `.build/` cache directory. |
| `--previews` | | `flag` | `false` | Build the example projects under `docs/examples/` before serving. Each is a full nested site, so a cold `serve` skips them and says so; a page that embeds an unbuilt example says so too. |
| `--kill-existing` | | `flag` | `false` | Stop the process listening on the selected port before serving. Processes that are only connected to that port, such as a browser tab, are left alone. |

**What it does:**

1. Performs the same build steps as `folioh build`, except the example projects under `docs/examples/`, which it builds only with `--previews`.
2. Prints `Starting dev server...` and starts a Next.js development server on the specified port.
3. **File watching:** after the dev server starts, Folioh prints `Watching for file changes...` and watches the configured source paths of every language and the documentation directories for changes. Edits are automatically detected, the affected pages and local images are regenerated, and the browser updates via hot reload.
4. **Incremental builds:** subsequent `serve` runs reuse cached dependencies (`.build/node_modules/`) and skip `pnpm install` when deps haven't changed.
5. Keeps the server running until interrupted. On Unix, `Ctrl+C`/SIGINT and SIGTERM stop the owned development-server process tree and print `Server stopped.`. A plain `kill <folioh-pid>` stops the server too. Terminal hangup and quit signals also clean up that tree.

The development server keeps ownership of `.build/` while watching. Starting a
second serve on another port, or running `build --clean`, refuses to change that
workspace. `--kill-existing` handles port occupants; it does not bypass workspace
ownership. Stop the owning Folioh process before restarting it.

The `--clean` flag forces a full rebuild from scratch, clearing all caches.

Markdown routes follow the same rule during builds and watched edits:
`README.md` publishes its folder's `index` page, including nested folders and
custom documentation URL bases. Editing or deleting the README updates or
removes that page and its Markdown mirror.

Watched saves are grouped into one source batch. Folioh retains parsed Python
modules, reparses changed modules, and uses the build generator to refresh pages,
navigation, search, Markdown mirrors, the authoring contract, and LLM indexes.
Changing the symbol index also regenerates API consumers so renamed or deleted
types do not leave stale links. Guides and plugin-collected documents are
reparsed on each batch; plugin callbacks remain serial.

Local Markdown images refresh when their files change, disappear, or return.
The source manifest records copied assets and removes obsolete copies once no
page owns them. Identical images keep their modification time. Generated output
directories and common editor temporary files are ignored by source watching.
If different images map to the same output path, generation reports both source
pages and stops before writing; shared images with identical bytes are allowed.
Search reuses extracted text for unchanged generated pages within the running
builder; page replacement, deletion, and documentation URL changes invalidate it.

Unchanged core generated text keeps its bytes and modification time. Changed
files are replaced atomically, one file at a time. The contract's `generatedAt`
records its last semantic change. This is not a transaction across the whole
site: a later write or plugin failure can leave some outputs refreshed. The
source manifest is saved only after source outputs finish successfully. A parse
or render failure occurs before core source pages are written, and pending Python
edits are retried on the next source save.

Preview examples rebuild when an example changes. Changes to configuration,
component implementations, themes, or undeclared plugin inputs still require
restarting `folioh serve`. Plugins own the refresh policy for their static bundles.

**Examples:**

```bash
# Start the dev server
folioh serve

# Serve on a custom port and open the browser
folioh serve --port 8080 --open

# Serve a different project with verbose output
folioh serve ../my-lib -v -o

```

**Exit codes:**

| Code | Meaning |
|------|---------|
| `1` | Failed to start: a missing config file, a port already in use, or a build error; or the dev server exited on its own with a failure. |
| `2` | Usage error, such as an unknown flag. |

On Unix, stopping the development-server session with SIGINT (`Ctrl+C`) returns `130`; SIGTERM (`kill <folioh-pid>`) returns `143`. Folioh stops its server group, reaps the launcher and releases the workspace before returning.

**Batch tracing:** Set [`FOLIOH_TRACE`](#folioh_trace) to record a JSONL
event stream for watch batches. See the environment variables section for
the full schema.

### `folioh coverage`

Analyze docstring coverage for configured Python source files. Coverage reads Python only in this release: a config without `source.python` paths is refused with `Error: folioh coverage reads Python sources only in this release, and docs.yaml lists no source.python paths.` JavaScript and Rust sources still build; they are just not counted.

```
folioh coverage [OPTIONS] [DIRECTORY]
```

| Argument | Type | Default | Description |
|----------|------|---------|-------------|
| `DIRECTORY` | `PATH` | Current working directory | Project directory containing `docs.yaml`. |

| Option | Short | Type | Default | Description |
|--------|-------|------|---------|-------------|
| `--project-dir` | | `PATH` | Current working directory | Compatibility option for scripts that prefer named arguments. |
| `--config` | `-c` | `TEXT` | `"docs.yaml"` | Path to the config file. |
| `--verbose` | `-v` | `flag` | `false` | List each undocumented symbol. |
| `--min` | | `FLOAT` | `0` | Minimum coverage percentage. Exits with code `1` if coverage is below this value. |

**Examples:**

```bash
folioh coverage
folioh coverage --min 80
folioh coverage --verbose
```

**Exit codes:**

| Code | Meaning |
|------|---------|
| `0` | Coverage check completed and met the configured threshold. |
| `1` | Config was missing, it lists no Python source paths, no modules were found, or coverage was below `--min`. |

### `folioh clean`

Remove generated build artifacts and output directories.

```
folioh clean [OPTIONS] [DIRECTORY]
```

| Argument | Type | Default | Description |
|----------|------|---------|-------------|
| `DIRECTORY` | `PATH` | Current working directory | Project directory. |

| Option | Short | Type | Default | Description |
|--------|-------|------|---------|-------------|
| `--project-dir` | | `PATH` | Current working directory | Compatibility option for scripts that prefer named arguments. |
| `--config` | `-c` | `TEXT` | `"docs.yaml"` | Config file to read `output` from, relative to the project directory. |

**What it does:**

Removes two directories:

1. **`.build/`** -- the intermediate Nextra project used during the build process.
2. **The output directory** -- by default `_site/`, or whatever `output` is set to in the config file. The clean command reads this value directly from YAML, so cleanup still works when the rest of the config is broken; a config it cannot read is reported in a warning and `_site/` is used.

The output directory is checked the same way `folioh build` checks it: an `output` that is absolute, outside the project, the project itself, or that contains `.git` or a configured source directory (such as `docs/` or `src/`) is ignored with a `Warning: Ignoring unsafe output in docs.yaml: ...` line, and `_site/` is cleaned instead. If either path exists but is not a directory, nothing is removed and the command fails.

If neither directory exists, it prints "Nothing to clean."

**Examples:**

```bash
# Clean the current project
folioh clean

# Clean a specific project
folioh clean /path/to/project

# Example output:
# Cleaned: .build, _site
```

**Exit codes:**

| Code | Meaning |
|------|---------|
| `0` | The directories were removed, or there was nothing to clean. |
| `1` | A path to remove is not a directory (`Error: docs.yaml is not a directory; folioh clean only removes directories. Check output in docs.yaml.`), the project directory does not exist, or a removal failed. |
| `2` | Usage error, such as an unknown flag. |

### `folioh roadmap`

Preview the source-defined roadmap phases configured under the `roadmap:` key in `docs.yaml` as a table. Provided by the built-in roadmap integration (see the [Roadmap](./plugins/roadmap) page).

```
folioh roadmap [OPTIONS] [DIRECTORY]
```

| Argument | Type | Default | Description |
|----------|------|---------|-------------|
| `DIRECTORY` | `PATH` | Current working directory | Project directory containing `docs.yaml`. |

| Option | Short | Type | Default | Description |
|--------|-------|------|---------|-------------|
| `--project-dir` | | `PATH` | Current working directory | Compatibility option for scripts that prefer named arguments. |
| `--config` | `-c` | `TEXT` | `"docs.yaml"` | Path to the config file. |

**Examples:**

```bash
folioh roadmap
folioh roadmap /path/to/project
```

**Exit codes:**

| Code | Meaning |
|------|---------|
| `0` | Phases were listed, or no phases are configured. |
| `1` | Config file was missing, or conflicting directory arguments were passed. |

## Environment Variables

Folioh reads these from the environment. Command-line options win over all of
them.

### FOLIOH_TRACE

Path to a JSONL file. When set, `folioh serve` appends one JSON object per
watch-batch event: the event name, a monotonic timestamp, and the event's own
fields. Unset, nothing is recorded.

```bash
FOLIOH_TRACE=trace.jsonl folioh serve
```

### FOLIOH_BASE_PATH

The static asset base path, such as `/my-repo`. It overrides `deploy.base_path`
and provider inference. `folioh serve` stays rooted at `/` unless this is set.

### FOLIOH_DEPLOY_PROVIDER

Set to `github-pages` to turn on GitHub Pages base-path inference without
writing `deploy.provider` into `docs.yaml`.

### NO_COLOR

Set to anything to drop ANSI colour from every command.

## Error Messages

Here are common errors you might encounter and what they mean:

**`Error: Config file not found: <path>/docs.yaml`**
No `docs.yaml` was found in the project directory; the message ends with the absolute path Folioh looked for. Run `folioh init` to create one, or specify the correct path with `--config`.

**`Build failed: ...`**
The build process encountered an error. Run with `--verbose` for detailed output. Common causes include syntax errors in Python source files, invalid YAML in the config, or missing dependencies.

**`docs.yaml already exists, skipping.`**
The `init` command found an existing config file and did not overwrite it. This is informational, not an error.

## Typical Workflow

A typical documentation workflow looks like this:

```bash
# 1. Initialize (one time)
folioh init

# 2. Edit docs.yaml to customize settings

# 3. Start the dev server while writing docs
folioh serve --open

# 4. Build for production
folioh build

# 5. Deploy the _site/ directory to your hosting provider

# 6. Clean up build artifacts when needed
folioh clean
```

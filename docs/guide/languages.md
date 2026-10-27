---
title: Languages
description: Source language support, configuration, and limits for Python, JavaScript, and Rust.
---

# Languages

Folioh documents **Python, JavaScript, and Rust**, reading source without
importing your package, installing your dependencies, building your crate, or
running your code. The native parsers extract modules, declarations,
signatures, types, attributes, and doc comments.

<Callout type="warning" title="Not available in this release">
  TypeScript is a separate parser, planned for 0.5 Every Language. `.ts`,
  `.tsx`, `.mts` and `.cts` files are not read; the build reports them in a
  warning instead.
</Callout>

Every supported language feeds the same API reference, search index, Markdown
mirrors, and LLM files. Reading source does not require that language's
interpreter, compiler, or build toolchain.

## Source coverage

| Language | Declarations | Documentation |
| --- | --- | --- |
| Python | Modules, functions and async functions, classes and nested classes, methods, properties, class attributes (enum members included), module constants, signatures, annotations, and decorators | Google-style and NumPy-style docstrings, detected per docstring; detection also reads reStructuredText `:param:` fields and epydoc `@param` fields, and Sphinx roles such as `` :class:`Foo` `` read as code |
| JavaScript | `.js`, `.mjs`, and `.cjs`; named and default ESM exports, including local declarations exported by `export { name }`; CommonJS `module.exports` (a function, a class, or an object of names) and `exports.*`; functions and generators, arrow functions bound to an exported name, classes with constructors, static methods, getters and setters, class fields, and constants | JSDoc descriptions, `@param` (with `[name=default]`, and `options.name` properties as rows of their own), `@returns`, `@yields` (or `@yield`), `@throws`, `@deprecated`, `@example`, and `{@link}` read as its text |
| Rust | Public functions, structs and fields, enums and variants, traits and associated items, inherent and trait implementations, type aliases, constants, and statics | `///`, `//!`, `#[doc = "..."]`, literal `include_str!` files, and `cfg_attr` doc values: untagged code blocks are Rust, `# ` hidden lines are dropped, and intra-doc links keep their text; signatures and attributes as written |

See [Writing Doc Comments](./docstrings) for Python examples, method kinds,
annotations, and control over what gets documented, and for what the JSDoc
and Rust readers take from a comment.

## Configuration

Point `source.python.paths` at your Python source roots and `source.docs` at
your Markdown guides:

```yaml
source:
  python:
    paths:
      - src
    exclude:
      - src/vendor
    docstring_style: auto
  docs:
    - docs/guide/
```

`exclude` accepts exact files, directories and glob patterns. Docstring style
can be `auto`, `google`, or `numpy`; `auto` also reads reStructuredText and
epydoc fields.

| Language | Source roots | Configuration key |
| --- | --- | --- |
| Python | Directories containing Python packages or modules | `source.python.paths` |
| JavaScript | Directories containing `.js`, `.mjs`, or `.cjs` files | `source.javascript.paths` |
| Rust | A crate directory containing `Cargo.toml`, a workspace holding several of them, or a crate's `src/` directory | `source.rust.paths` |

Each language block accepts its own `exclude` list, and every parser ships in
the binary: configuring a language is all it takes to read it. See
[Configuration](./configuration#source) for the complete `source` shape.

## Module names and reference pages

| Language | Module discovery | Example reference route |
| --- | --- | --- |
| Python | Package and module names relative to the source root | `api-reference/mypackage/utils` |
| JavaScript | File paths relative to the source root, with `/` as `.`; `utils/index.js` becomes `utils`. When two files publish one name (`utils/index.js` and `utils.js`, or `index.mjs` and `index.cjs`), the first in path order is read and the other is named in a warning | `api-reference/javascript/utils` |
| Rust | Crate names from `Cargo.toml`, with hyphens converted to underscores; `mod` declarations followed from `lib.rs` or `main.rs` | `api-reference/rust/demo_crate/models` |

## Rust documentation attributes

```rust
#![doc = include_str!("../README.md")]

#[cfg_attr(feature = "extra-docs", doc = "Additional documentation.")]
pub fn example() {}
```

Literal `include_str!` paths resolve relative to the Rust source file. Missing
or unreadable UTF-8 files produce a warning naming the input and its source;
the remaining API still builds. Conditional doc attributes contribute without
evaluating features or configuration predicates, including nested `cfg_attr`.
Folioh does not execute macros to compute paths.

Editing, deleting, or recreating an included file invalidates its API page on
warm builds and in `serve`. Restart `serve` after changing include paths so its
file subscriptions include the new inputs.

## Limits

Source is inspected statically. Folioh does not execute imports, evaluate
application code, or infer runtime behavior.

| Language | Limits |
| --- | --- |
| Python | Documentation comes from source declarations, annotations, and docstrings; the package does not need to be installed or importable. A module's `__all__` decides what it publishes; without one, a name with a leading underscore is skipped. A dunder method other than `__init__` appears only when it has a docstring, `@overload` stubs give way to the implementation, and a property's setter and deleter fold into its getter. Block reStructuredText notices render as callouts; fenced and indented code stays literal. |
| JavaScript | Only what a file exports is documented, and a `#private` class member is not. `node_modules`, `dist`, `build`, and dot directories are never read, `.jsx` is skipped with a warning, and TypeScript is reported in a warning rather than read. A setter beside its getter is shown once, as the getter. Types and defaults come from the source and from JSDoc, never from inference; re-exports are listed by name without resolving the module they come from. |
| Rust | Only plain `pub` items are documented: `pub(crate)`, `pub(super)`, `#[doc(hidden)]`, and impls of private types are skipped, and so are macro definitions and invocations. A module is read only if some `lib.rs`, `main.rs`, or parent module declares it `pub`. `pub use` is listed as written under Re-exports without resolving its target, so an item re-exported from a private module is not documented. `cfg` and `cfg_attr` predicates are not evaluated. Conditional doc values contribute as written; other macros, including computed include paths, are not expanded. Inline-module items appear on the page of the file that declares them. |

## Generated examples

<DocPreview
  title="JavaScript API example"
  example="javascript"
  description="Inspect a reference generated from JavaScript exports and JSDoc."
  height={360}
/>

<DocPreview
  title="Rust API example"
  example="rust"
  description="Inspect a reference generated from Rust source and doc comments."
  height={360}
/>

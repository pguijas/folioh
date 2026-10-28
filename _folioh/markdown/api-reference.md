# API Reference

**Not available in this release**

  Folioh reads Rust source, but this site does not publish a reference for
  Folioh's own code yet: it ships its landing and guides only.

## What the API reference is in your project

The API reference is the part of the site Folioh generates from source. List
your source roots under `source.python.paths`, `source.javascript.paths` or
`source.rust.paths` in `docs.yaml`; every module becomes one page, and each
page shows:

- the module's doc comment as prose;
- every public class, function and type with its signature, including type
  annotations, defaults, decorators and Rust attributes (a Python signature
  leaves out `self` and `cls` and takes an unannotated parameter's type from
  its docstring);
- a parameter table and return type per Python and JavaScript function, read
  from the signature and from Google or NumPy docstrings (with `auto` also
  reading reStructuredText and epydoc fields) or JSDoc tags; a Rust function
  shows its signature and doc comment, as rustdoc does;
- the raises, examples and notes of a docstring, and its deprecated, warning,
  see also, references and todo sections as labelled paragraphs;
- module constants, class attributes, struct fields and enum variants in
  tables, and JavaScript and Rust re-exports listed as written, without
  resolving what they point at.

Rust trait implementations are titled `impl Trait for Type`. Modules without a
doc comment have no summary in the overview. Dunder identifiers such as
`__init__` render literally in API prose; fenced and inline code keep their
original content. Use `**text**` for strong emphasis.

Block reStructuredText directives in docstrings render as callouts: `note`,
`warning`, `tip`, `seealso` and `deprecated`. Deprecation versions and body
content are preserved. Other directive names become labelled notices. Nested
notices, lists and code remain readable, and the Markdown/full-text reference
retains each notice label and body.

[Languages](/docs/languages) lists what each reader takes from its source and
where it stops.

Folioh reads the source and never executes it. Each generated page also gets an
agent-readable Markdown mirror, so agents read the same reference people do.

[Quick Start](/docs/quickstart) walks the first build end to end, and
[Configuration](/docs/configuration) documents the `source` block.

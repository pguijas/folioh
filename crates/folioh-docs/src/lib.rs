//! The Docs product: IR to MDX pages, the `api-reference/index` overview,
//! routes and symbol prefixes per language, cross-references, coverage data,
//! `llms.txt`/`llms-full.txt` text, the disabled-feature gates and the
//! Python-first source orchestration. Returns strings and `warnings`;
//! `folioh-site` writes files and the `folioh` binary renders tables.

pub mod coverage;
pub mod error;
pub mod features;
mod headings;
pub mod llms;
pub mod mdx;
pub mod routes;
pub mod sources;
pub mod xref;

pub use coverage::{
    aggregate, analyze_module, analyze_modules, below_minimum, coverage_level, CoverageLevel,
    CoverageResult, NO_MODULES_ERROR,
};
pub use error::DocsError;
pub use features::{disabled_api_feature_for_module, disabled_doc_feature_for_route};
pub use llms::{
    api_link, doc_link, generate_llms_full_txt, generate_llms_txt, source_citation, LlmsContext,
};
pub use mdx::{
    api_reference_index_to_mdx, module_to_mdx, render_param_table, source_link, RenderOptions,
};
pub use routes::{anchor_slug, member_anchor, module_route, symbol_prefix, type_anchor};
pub use sources::{
    discover_language, discover_sources, enabled_languages, file_extensions, has_parser,
    parse_discovered, parse_doc_sources, parse_language_sources, parse_python_sources,
    source_dependencies, DiscoveredFile, ParsedDocSources, ParsedSources,
};
pub use xref::{build_symbol_index, resolve_type_link, SymbolIndex, BUILTINS};

/// IR builders shared by the unit tests of every module.
#[cfg(test)]
pub(crate) mod fixtures;

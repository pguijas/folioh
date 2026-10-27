//! The steps after the pages: extensions and
//! plugin assets, the authoring contract, the search index, the preview
//! examples, the link check and the LLM texts.

use std::collections::{BTreeSet, HashSet};
use std::path::Path;

use folioh_config::DocsConfig;
use folioh_docs::{generate_llms_full_txt, generate_llms_txt, module_route, LlmsContext};
use folioh_ir::ModuleIR;
use folioh_mdx::MarkdownPage;
use folioh_plugins::{contract_config_keys, ExtensionRegistry, Landing, PluginHost};
use folioh_site::builder::{PreviewBuildRequest, PreviewExamples, SiteBuilder};
use folioh_site::links::{check_fragments, check_links, BrokenLink, LinkCheckInput};
use folioh_site::SiteError;

use super::pages::{published_docs, published_modules};
use super::{BuildError, Report};
use crate::ui::steps::count_phrase;

/// UTC now to the second, RFC 3339 with `Z`: the contract's `generatedAt`.
pub fn build_timestamp() -> String {
    jiff::Timestamp::now()
        .strftime("%Y-%m-%dT%H:%M:%SZ")
        .to_string()
}

/// Extensions (when `registry` is given), plugin assets, the contract, the
/// search index, then the preview examples when `refresh_previews`. Returns
/// whether previews were built (the `Previews` row).
#[allow(clippy::too_many_arguments)]
pub fn finalize_generated_files(
    builder: &mut SiteBuilder<'_>,
    host: &PluginHost,
    config: &DocsConfig,
    project_dir: &Path,
    source_routes: &BTreeSet<String>,
    registry: Option<&ExtensionRegistry>,
    refresh_previews: bool,
    report: &Report,
    nested_build: &mut dyn FnMut(PreviewBuildRequest) -> Result<(), BuildError>,
) -> Result<PreviewExamples, BuildError> {
    let spinner = report.spinner("Finalize", "preparing generated files", None);
    if let Some(registry) = registry {
        builder.apply_extensions(registry)?;
    }
    let mut diag = Vec::new();
    host.emit_assets(builder, config, &mut diag);
    builder.sweep_plugin_pages(source_routes)?;
    folioh_site::sidebar::apply_directory_orders(builder)?;
    let mut keys: std::collections::BTreeSet<String> = contract_config_keys()
        .into_iter()
        .map(str::to_string)
        .collect();
    keys.extend(host.config_keys());
    builder.write_authoring_contract(&keys, &build_timestamp())?;
    builder.write_search_index()?;
    drop(spinner);
    for warning in &diag {
        report.warning(warning);
    }
    if !refresh_previews {
        return Ok(PreviewExamples::default());
    }
    let examples_dir = project_dir.join("docs").join("examples");
    if !examples_dir.exists() {
        builder.write_preview_examples(&examples_dir, &mut |_| Ok(()))?;
        return Ok(PreviewExamples::default());
    }
    // Each stale example is a whole Folioh project under `docs/examples/<name>`
    // built as its own site (the pages `DocPreview` embeds), so the row names
    // the project by its path: a first build otherwise looks stuck.
    let names: Vec<String> = SiteBuilder::preview_example_dirs(&examples_dir)
        .iter()
        .filter_map(|dir| Some(dir.file_name()?.to_string_lossy().into_owned()))
        .collect();
    let mut spinner = report.spinner(
        "Previews",
        &format!(
            "{} under docs/examples",
            count_phrase(names.len(), "example project", None)
        ),
        None,
    );
    let mut run = |request: PreviewBuildRequest| {
        let name = request
            .project_dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let position = names.iter().position(|n| *n == name).map_or(0, |i| i + 1);
        spinner = report.spinner(
            "Previews",
            &format!(
                "{position} of {}: docs/examples/{name}, built as its own site, slow at first",
                names.len()
            ),
            None,
        );
        nested_build(request).map_err(|e| SiteError::Value(e.to_string()))
    };
    let summary = builder.write_preview_examples(&examples_dir, &mut run)?;
    drop(spinner);
    Ok(summary)
}

/// Every internal link that resolves to nothing: published docs and modules,
/// `api-reference/index`, plugin-emitted routes, registry views and static
/// files are valid targets; gated pages are not.
pub fn check_generated_links(
    builder: &SiteBuilder<'_>,
    modules: &[ModuleIR],
    docs: &[MarkdownPage],
) -> Vec<BrokenLink> {
    let published = published_modules(modules);
    let mut routes: Vec<String> = published_docs(docs)
        .iter()
        .map(|d| d.route.clone())
        .collect();
    if !published.is_empty() {
        routes.push("api-reference/index".to_string());
    }
    routes.extend(published.iter().map(|m| module_route(m)));
    routes.extend(builder.emitted_routes());
    let mut site_routes: HashSet<String> = builder.view_routes().into_iter().collect();
    site_routes.insert("/".to_string());
    check_links(&LinkCheckInput {
        content_dir: &builder.content_dir,
        pages: &routes,
        docs_route_base: &builder.docs_route_base(),
        site_routes: &site_routes,
        static_root: Some(&builder.build_dir.join("public")),
    })
}

/// Route checks plus exact anchors from the completed static export.
pub fn check_exported_links(
    builder: &SiteBuilder<'_>,
    modules: &[ModuleIR],
    docs: &[MarkdownPage],
) -> Result<Vec<BrokenLink>, SiteError> {
    let mut broken = check_generated_links(builder, modules, docs);
    broken.extend(check_fragments(
        &builder.content_dir,
        &builder.output_dir,
        &builder.docs_route_base(),
        docs,
    )?);
    Ok(broken)
}

/// The `llms.txt` and `llms-full.txt` bodies the config asks for
/// (`None` = switched off); published modules only, the docs filter themselves.
pub fn llm_texts(
    config: &DocsConfig,
    project_dir: &Path,
    modules: &[ModuleIR],
    docs: &[MarkdownPage],
) -> (Option<String>, Option<String>) {
    let published: Vec<ModuleIR> = published_modules(modules).into_iter().cloned().collect();
    let landing = Landing::from_config(config);
    let hero_description = landing
        .enabled
        .then(|| landing.hero.description.as_str())
        .flatten()
        .unwrap_or("");
    let project_dir = project_dir.to_string_lossy();
    let ctx = LlmsContext {
        project_name: &config.project.name,
        landing_hero_description: hero_description,
        site_url: &config.project.url,
        docs_route_base: &config.template.docs_route_base,
        project_dir: &project_dir,
        ..LlmsContext::default()
    };
    let llms_txt = config
        .llm
        .generate_llms_txt
        .then(|| generate_llms_txt(&ctx, &published, docs));
    let llms_full = config
        .llm
        .generate_llms_full_txt
        .then(|| generate_llms_full_txt(&published, docs, Some(&ctx)));
    (llms_txt, llms_full)
}

/// A watcher batch: rewrite the LLM files where the builder publishes them.
pub fn write_llm_outputs(
    builder: &SiteBuilder<'_>,
    config: &DocsConfig,
    project_dir: &Path,
    modules: &[ModuleIR],
    docs: &[MarkdownPage],
) -> Result<(), BuildError> {
    let (llms_txt, llms_full) = llm_texts(config, project_dir, modules, docs);
    builder.write_llm_files(llms_txt.as_deref(), llms_full.as_deref(), builder.serve)?;
    Ok(())
}

#[cfg(test)]
#[path = "finalize_tests.rs"]
mod tests;

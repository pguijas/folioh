//! Release gates on generated API modules. Authored documentation is never
//! hidden by its route name; experimental features are gated in configuration.

use folioh_config::EXPERIMENTAL_ENV_VAR;

/// Doc route -> the feature that gates it. Authored routes are not reserved.
pub const DISABLED_DOC_ROUTES: [(&str, &str); 0] = [];
/// Module prefix -> the feature that gates it (and its submodules). Empty this release.
pub const DISABLED_API_MODULES: [(&str, &str); 0] = [];

fn named_in(experimental: &str, feature: &str) -> bool {
    experimental.split(',').map(str::trim).any(|f| f == feature)
}

/// The current `FOLIOH_EXPERIMENTAL` value.
pub(crate) fn experimental() -> String {
    std::env::var(EXPERIMENTAL_ENV_VAR).unwrap_or_default()
}

/// `disabled_doc_feature_for_route` against an explicit `FOLIOH_EXPERIMENTAL` value.
pub fn disabled_doc_feature_for_route_in(route: &str, experimental: &str) -> Option<&'static str> {
    let route = route.trim_matches('/');
    DISABLED_DOC_ROUTES
        .iter()
        .find(|(gated, _)| *gated == route)
        .map(|(_, feature)| *feature)
        .filter(|feature| !named_in(experimental, feature))
}

/// The feature gating a doc route, or `None` when the page may be published.
pub fn disabled_doc_feature_for_route(route: &str) -> Option<&'static str> {
    disabled_doc_feature_for_route_in(route, &experimental())
}

/// `disabled_api_feature_for_module` against an explicit gate table and env
/// value. Membership in the table is the gate; a prefix covers its submodules.
pub fn disabled_api_feature_for_module_in<'a>(
    module_name: &str,
    gates: &[(&str, &'a str)],
    experimental: &str,
) -> Option<&'a str> {
    gates
        .iter()
        .find(|(prefix, _)| {
            module_name == *prefix || module_name.starts_with(&format!("{prefix}."))
        })
        .map(|(_, feature)| *feature)
        .filter(|feature| !named_in(experimental, feature))
}

/// The feature gating a module's API pages, or `None` when publishable.
pub fn disabled_api_feature_for_module(module_name: &str) -> Option<&'static str> {
    disabled_api_feature_for_module_in(module_name, &DISABLED_API_MODULES, &experimental())
}

#[cfg(test)]
#[path = "features_tests.rs"]
mod tests;

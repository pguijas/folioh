use super::*;

#[test]
fn authored_doc_routes_are_never_gated_by_their_name() {
    for route in [
        "versioning",
        "/versioning/",
        "i18n",
        "quickstart",
        "roadmap",
        "guide/versioning",
    ] {
        for experimental in ["", "versions", " i18n , versions ", " , "] {
            assert_eq!(
                disabled_doc_feature_for_route_in(route, experimental),
                None,
                "{route}: {experimental}"
            );
        }
    }
}

#[test]
fn api_module_gate_is_membership_in_the_table_and_covers_submodules() {
    let gates = [("folioh_docs.docs.integrations.roadmap", "plugins")];
    assert_eq!(
        disabled_api_feature_for_module_in("folioh_docs.docs.integrations.roadmap", &gates, ""),
        Some("plugins")
    );
    assert_eq!(
        disabled_api_feature_for_module_in(
            "folioh_docs.docs.integrations.roadmap.data",
            &gates,
            ""
        ),
        Some("plugins")
    );
    assert_eq!(
        disabled_api_feature_for_module_in("folioh_docs.docs.integrations.roadmapper", &gates, ""),
        None
    );
    assert_eq!(
        disabled_api_feature_for_module_in("folioh_docs.docs.integrations.landing", &gates, ""),
        None
    );
    assert_eq!(
        disabled_api_feature_for_module_in(
            "folioh_docs.docs.integrations.roadmap",
            &gates,
            "plugins"
        ),
        None
    );
    assert_eq!(
        disabled_api_feature_for_module_in(
            "folioh_docs.docs.integrations.roadmap",
            &gates,
            "versions"
        ),
        Some("plugins")
    );
    // This release gates no API module.
    assert!(DISABLED_API_MODULES.is_empty());
    assert_eq!(
        disabled_api_feature_for_module("folioh_docs.docs.integrations.roadmap"),
        None
    );
}

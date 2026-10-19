#[test]
fn known_keys_are_core_then_builtins() {
    let keys = super::known_config_keys();
    assert_eq!(keys.len(), 19);
    assert_eq!(keys[0], "project");
    assert_eq!(&keys[16..], ["landing", "roadmap", "openapi"]);
}

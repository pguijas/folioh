//! Source fingerprints include literal documentation files read by the parser.
use folioh_docs::source_dependencies;
use folioh_site::fs::{hash_file, sha256_hex};
use std::path::Path;

pub(super) fn source_hash(path: &Path) -> String {
    let hash = hash_file(path);
    let dependencies = source_dependencies(path);
    if dependencies.is_empty() || hash.is_empty() {
        return hash;
    }
    let mut inputs = vec![(path.to_string_lossy().into_owned(), hash)];
    inputs.extend(
        dependencies
            .into_iter()
            .map(|path| (path.to_string_lossy().into_owned(), hash_file(&path))),
    );
    sha256_hex(
        serde_json::to_string(&inputs)
            .expect("source fingerprints serialize")
            .as_bytes(),
    )
}

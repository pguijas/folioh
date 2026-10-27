//! Ownership records for files emitted by integrations. A sweep removes only
//! files whose bytes still match what the previous build generated.

use std::collections::BTreeMap;
use std::path::Path;

use crate::fs::sha256_hex;
use crate::{Result, SiteError};

pub(crate) fn load(path: &Path) -> Result<BTreeMap<String, String>> {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text)
            .map_err(|e| SiteError::Value(format!("{}: {e}", path.display()))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(BTreeMap::new()),
        Err(e) => Err(SiteError::io(path, e)),
    }
}

pub(crate) fn digest(path: &Path) -> Result<Option<String>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(sha256_hex(&bytes))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(SiteError::io(path, e)),
    }
}

//! Rust source to `ModuleIR`: the crates under the configured roots, the module
//! tree their `mod` declarations publish, and every public item of each module
//! with its signature as written. The reader holds no state and no
//! configuration beyond the excludes; docstring styles are a Python concern.

use std::path::Path;

use folioh_ir::ModuleIR;

pub mod discover;
mod doc_attributes;
pub mod error;
pub mod parse;
mod rustdoc;
pub use doc_attributes::doc_dependencies;

pub use discover::{crate_of, discover, module_name, SourceFile};
pub use error::{ParseError, RustSyntaxError};
pub use folioh_ir::excludes::is_excluded;
pub use parse::parse_source;

/// Read the file as UTF-8 and parse it; a missing or undecodable file names its
/// path. `root` is the configured source root the file was discovered under; the
/// IR reports the file as discovered, so source links, the incremental manifest
/// and the watcher all name the same path.
pub fn parse_file(path: &Path, _root: &Path) -> Result<ModuleIR, ParseError> {
    let bytes = std::fs::read(path).map_err(|source| ParseError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let source = String::from_utf8(bytes).map_err(|err| {
        let start = err.utf8_error().valid_up_to();
        let end = err
            .utf8_error()
            .error_len()
            .map_or(err.as_bytes().len(), |len| start + len);
        ParseError::Decode {
            path: path.to_path_buf(),
            start,
            end,
        }
    })?;
    let source_file = path.display().to_string();
    parse_source(&source, &module_name(path), &source_file).map_err(|err| {
        ParseError::Syntax(RustSyntaxError {
            path: path.to_path_buf(),
            ..err
        })
    })
}

//! The site workspace: the bundled template embedded in the binary, the
//! `.build/` lifecycle, the `docs.yaml` injection into the copied template,
//! the `SiteBuilder` write surface (pages, mirrors, `_meta.ts`, static and
//! agent artifacts), the sidebar, search data, the link checker, the Next
//! runtime behind `FrontendRuntime`, the static export and its rewriter.
//! Everything between rendered MDX strings and a `_site/` a host can serve.
//! Libraries never print: every function returns data and warnings.

pub mod builder;
pub mod export;
pub mod extensions;
pub mod fs;
mod generated;
pub mod inject;
pub mod json;
pub mod links;
pub mod rewriter;
pub mod runtime;
pub mod sidebar;
pub mod template;
pub mod theme;
pub mod theme_package;

use std::path::{Path, PathBuf};

/// One error enum for the crate; `Display` is the user-facing text.
#[derive(Debug, thiserror::Error)]
pub enum SiteError {
    /// A value or path the build refuses (Python `ValueError`).
    #[error("{0}")]
    Value(String),
    /// A configured file or directory that does not exist.
    #[error("{0}")]
    NotFound(String),
    /// The Node toolchain or a subprocess failed.
    #[error("{0}")]
    Runtime(String),
    /// `pnpm run build` exited non-zero; `output` is the streamed log.
    #[error("pnpm build failed:\n{}\nFull log: {}", output.concat(), log_path.display())]
    Build {
        /// Every line `pnpm run build` printed.
        output: Vec<String>,
        /// Where the same output was saved.
        log_path: PathBuf,
    },
    /// A filesystem operation failed at `path`.
    #[error("{}: {source}", path.display())]
    Io {
        /// The path the operation targeted.
        path: PathBuf,
        #[source]
        /// The OS error.
        source: std::io::Error,
    },
}

impl SiteError {
    /// An `Io` error for `path`.
    pub fn io(path: &Path, source: std::io::Error) -> Self {
        SiteError::Io {
            path: path.to_path_buf(),
            source,
        }
    }
}

/// `Result<T, SiteError>`.
pub type Result<T> = std::result::Result<T, SiteError>;

/// Compiles one of the crate's fixed patterns.
pub(crate) fn re(pattern: &str) -> regex::Regex {
    regex::Regex::new(pattern).expect("static pattern")
}

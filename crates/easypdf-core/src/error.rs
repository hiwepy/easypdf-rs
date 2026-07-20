//! Error types for `easypdf-rs`.
//!
//! Provides the central `PdfError` enum and a convenience `Result` type alias.

use std::io;

/// Central error type for `easypdf-rs`.
///
/// Covers I/O, parsing, encryption, and unsupported-feature errors.
#[derive(Debug, thiserror::Error)]
pub enum PdfError {
    /// Wraps a standard I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// A PDF could not be parsed or contains malformed data.
    #[error("PDF parse error: {0}")]
    Parse(String),

    /// A page index is out of bounds.
    #[error("Invalid page index: {0}")]
    InvalidPage(usize),

    /// The requested feature is not yet implemented or not supported by the engine.
    #[error("Unsupported feature: {0}")]
    UnsupportedFeature(String),

    /// The PDF is encrypted and either no password was supplied or the password was wrong.
    #[error("Encryption error: {0}")]
    Encryption(String),

    /// Catch-all for other errors.
    #[error("{0}")]
    Other(String),
}

/// Convenience `Result` type that uses [`PdfError`] as the error variant.
pub type Result<T, E = PdfError> = std::result::Result<T, E>;

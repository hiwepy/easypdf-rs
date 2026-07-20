#![doc = "Core types, traits, enums, converters, and errors for `easypdf-rs`."]
#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![deny(unsafe_code)]

// --- Modules ---
pub mod content;
pub mod enums;
pub mod error;
pub mod event;
pub mod metadata;
pub mod style;
pub mod traits;

// --- Convenience re-exports ---
pub use content::*;
pub use enums::*;
pub use error::{PdfError, Result};
pub use event::*;
pub use metadata::*;
pub use style::*;
pub use traits::*;

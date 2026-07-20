//! Proc-macro derive for the `PdfModel` trait.
//!
//! Provides `#[derive(PdfModel)]` which generates compile-time
//! reflection code mapping Rust struct fields to PDF content elements.
//!
//! ## Usage
//!
//! ```ignore
//! use easypdf_derive::PdfModel;
//!
//! #[derive(PdfModel)]
//! #[pdf(page = A4, orientation = Portrait)]
//! struct Invoice {
//!     #[pdf(text, position = (100, 700))]
//!     title: String,
//! }
//! ```

use proc_macro::TokenStream;

mod implementation;

/// Derive macro that generates a [`PdfModel`] trait implementation.
///
/// # Attributes
///
/// - `#[pdf(page = ..., orientation = ..., margins = ...)]` on the struct
/// - `#[pdf(text, position = (x, y), font = ...)]` on text fields
/// - `#[pdf(table, position = (x, y), headers = [...])]` on collection fields
/// - `#[pdf(field = "field_name")]` on form/template fields
/// - `#[pdf(ignore)]` to skip a field
#[proc_macro_derive(PdfModel, attributes(pdf))]
pub fn derive_pdf_model(input: TokenStream) -> TokenStream {
    implementation::expand_pdf_model(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

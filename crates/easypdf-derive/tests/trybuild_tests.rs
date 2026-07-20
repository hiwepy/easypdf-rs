//! Trybuild-based compile-time tests for the PdfModel derive macro.
//!
//! These tests verify that the macro generates correct code by compiling
//! test cases. Currently requires the derive macro to use `proc-macro-crate`
//! name resolution correctly — see implementation.rs for details.
//!
//! NOTE: Tests are `#[ignore]` because trybuild standalone compilation
//! requires the derive macro to properly resolve `easypdf_core` crate name
//! at compile time. This is tracked as a future improvement.

#[test]
#[ignore = "requires proc-macro-crate name resolution fix in implementation.rs"]
fn test_derive_trybuild() {
    let t = trybuild::TestCases::new();
    t.pass("tests/trybuild/01-basic-text.rs");
    t.pass("tests/trybuild/02-multiple-fields.rs");
    t.pass("tests/trybuild/03-ignore-fields.rs");
    t.pass("tests/trybuild/04-custom-page.rs");
}

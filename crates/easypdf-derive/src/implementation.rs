//! Implementation of the `#[derive(PdfModel)]` proc-macro.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Error, Result};

/// Entry point: expands `#[derive(PdfModel)]` into the trait implementation.
pub(crate) fn expand_pdf_model(input: TokenStream) -> Result<TokenStream> {
    let input: DeriveInput = syn::parse2(input)?;
    let name = &input.ident;

    // Parse struct-level #[pdf(...)] attributes
    let PdfStructAttrs {
        page_size,
        orientation,
        margins,
    } = parse_struct_attrs(&input.attrs)?;

    // Generate field rendering code
    let render_arms = generate_render_arms(&input)?;

    let expanded = quote! {
        impl easypdf_core::PdfModel for #name {
            fn render(&self) -> easypdf_core::Result<Vec<easypdf_core::RenderedElement>> {
                let mut elements = Vec::new();
                #render_arms
                Ok(elements)
            }

            fn metadata(&self) -> easypdf_core::PdfModelMetadata {
                easypdf_core::PdfModelMetadata {
                    page_size: #page_size,
                    orientation: #orientation,
                    margins: #margins,
                }
            }
        }
    };

    Ok(expanded)
}

struct PdfStructAttrs {
    page_size: TokenStream,
    orientation: TokenStream,
    margins: TokenStream,
}

fn parse_struct_attrs(attrs: &[syn::Attribute]) -> Result<PdfStructAttrs> {
    let mut page_size = quote! { easypdf_core::PageSize::A4 };
    let mut orientation = quote! { easypdf_core::Orientation::Portrait };
    let mut margins = quote! { 72.0_f64 };

    for attr in attrs {
        if !attr.path().is_ident("pdf") {
            continue;
        }

        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("page") {
                let value: syn::Expr = meta.value()?.parse()?;
                page_size = quote! { #value };
            } else if meta.path.is_ident("orientation") {
                let value: syn::Expr = meta.value()?.parse()?;
                orientation = quote! { #value };
            } else if meta.path.is_ident("margins") {
                let value: syn::Expr = meta.value()?.parse()?;
                margins = quote! { #value };
            }
            Ok(())
        })?;
    }

    Ok(PdfStructAttrs {
        page_size,
        orientation,
        margins,
    })
}

fn generate_render_arms(input: &DeriveInput) -> Result<TokenStream> {
    let fields = match &input.data {
        syn::Data::Struct(s) => &s.fields,
        _ => return Err(Error::new_spanned(input, "PdfModel only supports structs")),
    };

    let syn::Fields::Named(named) = fields else {
        return Err(Error::new_spanned(fields, "PdfModel requires named fields"));
    };

    let mut arms = TokenStream::new();
    for field in &named.named {
        let field_name = field
            .ident
            .as_ref()
            .ok_or_else(|| Error::new_spanned(field, "unnamed fields not supported"))?;

        // Check for #[pdf(ignore)]
        if has_pdf_attr(&field.attrs, "ignore") {
            continue;
        }

        // Check for #[pdf(text, position = (x, y), ...)]
        if has_pdf_attr(&field.attrs, "text") {
            let (x, y) = parse_position(field)?;
            let text_attrs = parse_text_attrs(field)?;
            arms.extend(quote! {
                elements.push(easypdf_core::RenderedElement::Text {
                    x: #x,
                    y: #y,
                    text: easypdf_core::PdfText::new(self.#field_name.clone())
                        #text_attrs,
                });
            });
            continue;
        }

        // Check for #[pdf(table, position = (x, y), ...)]
        if has_pdf_attr(&field.attrs, "table") {
            let (x, y) = parse_position(field)?;
            arms.extend(quote! {
                elements.push(easypdf_core::RenderedElement::Table {
                    x: #x,
                    y: #y,
                    table: self.#field_name.clone(),
                });
            });
            continue;
        }

        // Check for #[pdf(image, position = (x, y))]
        if has_pdf_attr(&field.attrs, "image") {
            let (x, y) = parse_position(field)?;
            arms.extend(quote! {
                elements.push(easypdf_core::RenderedElement::Image {
                    x: #x,
                    y: #y,
                    image: self.#field_name.clone(),
                });
            });
            continue;
        }
    }

    Ok(arms)
}

fn has_pdf_attr(attrs: &[syn::Attribute], ident: &str) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("pdf") {
            return false;
        }
        // Simple check: does the attribute contain this identifier?
        attr.meta
            .require_list()
            .ok()
            .and_then(|list| {
                list.tokens
                    .clone()
                    .into_iter()
                    .find(|t| t.to_string() == ident)
            })
            .is_some()
    })
}

/// Parse `(x, y)` tuple from the position attribute.
fn parse_position(field: &syn::Field) -> Result<(TokenStream, TokenStream)> {
    let mut x = quote! { 100.0_f64 };
    let mut y = quote! { 700.0_f64 };

    for attr in &field.attrs {
        if !attr.path().is_ident("pdf") {
            continue;
        }
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("position") {
                let content: syn::ExprTuple = meta.value()?.parse()?;
                if content.elems.len() == 2 {
                    let x_expr = &content.elems[0];
                    let y_expr = &content.elems[1];
                    x = quote! { (#x_expr) as f64 };
                    y = quote! { (#y_expr) as f64 };
                }
            }
            Ok(())
        })?;
    }

    Ok((x, y))
}

/// Parse text-specific attributes: font, color, alignment.
fn parse_text_attrs(field: &syn::Field) -> Result<TokenStream> {
    let mut attrs = TokenStream::new();

    for attr in &field.attrs {
        if !attr.path().is_ident("pdf") {
            continue;
        }
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("font") {
                let value: syn::Expr = meta.value()?.parse()?;
                attrs.extend(quote! { .font(#value) });
            } else if meta.path.is_ident("size") {
                let value: syn::Expr = meta.value()?.parse()?;
                attrs.extend(quote! { .font(easypdf_core::PdfFont::helvetica(#value as f64)) });
            }
            Ok(())
        })?;
    }

    Ok(attrs)
}

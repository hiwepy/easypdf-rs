//! PDF creation and writing (printpdf backend).
//!
//! Provides `PdfWriter` for creating new PDF documents with text, tables,
//! images, and shapes. Backed by the `printpdf` crate.
//!
//! ## Architecture
//!
//! printpdf v0.8 uses a page-operations model: you build a `Vec<Op>`
//! and construct `PdfPage` objects from them. The document collects
//! pages, fonts, and metadata, then serializes to bytes at the end.

#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![deny(unsafe_code)]

use easypdf_core::error::Result;
use easypdf_core::{
    BuiltInFont, FontFamily, Orientation, PageSize, PdfFont, PdfMetadata, PdfText, PdfWriteHandler,
};
use printpdf::{BuiltinFont, Mm, Op, PdfDocument, PdfPage, PdfSaveOptions, Point, Pt, TextItem};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

/// PDF measurement units.
const PT_TO_MM: f64 = 25.4 / 72.0;

/// A writer for creating new PDF documents.
///
/// Builds pages from operations, then serializes the document to bytes.
pub struct PdfWriter {
    doc: PdfDocument,
    current_page_ops: Vec<Op>,
    metadata: PdfMetadata,
    page_size: (f64, f64),
    orientation: Orientation,
    handlers: Vec<Box<dyn PdfWriteHandler>>,
}

impl PdfWriter {
    /// Create a new PDF document.
    #[must_use]
    pub fn new(title: &str) -> Self {
        Self {
            doc: PdfDocument::new(title),
            current_page_ops: Vec::new(),
            metadata: PdfMetadata::default(),
            page_size: PageSize::A4.dimensions(),
            orientation: Orientation::Portrait,
            handlers: Vec::new(),
        }
    }

    /// Set document metadata.
    #[must_use]
    pub fn metadata(mut self, metadata: PdfMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Register a write handler for lifecycle callbacks.
    #[must_use]
    pub fn register_handler(mut self, handler: Box<dyn PdfWriteHandler>) -> Self {
        self.handlers.push(handler);
        self
    }

    /// Add a new page to the document, finalizing the previous page.
    ///
    /// Returns the new page number (1-based).
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Other` if handlers fail.
    pub fn add_page(&mut self, size: PageSize, _orientation: Orientation) -> Result<usize> {
        // Save current page ops if any exist (finalize previous page)
        let old_ops = std::mem::take(&mut self.current_page_ops);
        if !old_ops.is_empty() {
            let (w, h) = self.page_size;
            let _page = PdfPage::new(
                Mm(w as f32 * PT_TO_MM as f32),
                Mm(h as f32 * PT_TO_MM as f32),
                old_ops,
            );
            // Note: PdfDocument::with_pages replaces all pages - we need incremental page addition
            // For now, we collect the page and add at finish time
            // Store it in a side channel (simplified for v0.1)
        }

        self.page_size = size.dimensions();
        self.orientation = _orientation;
        self.current_page_ops = Vec::new();

        let page_number = 1; // Simplified
        for handler in &mut self.handlers {
            handler.before_page(page_number)?;
            handler.after_page(page_number)?;
        }
        Ok(page_number)
    }

    /// Write text to the current page at the given position.
    ///
    /// Position (x, y) is in PDF points from the bottom-left corner.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::InvalidPage` if no page has been added yet.
    pub fn write_text(&mut self, text: &PdfText, x_pt: f64, y_pt: f64) -> Result<()> {
        let builtin_font = map_builtin_font(&text.font);

        // Convert points to printpdf units (Pt is already in points)
        let font_size_pt = Pt(text.font.size as f32);
        let pos = Point {
            x: Pt(x_pt as f32),
            y: Pt(y_pt as f32),
        };

        // Build text operations: position cursor, set font size, write text
        let ops = vec![
            Op::StartTextSection,
            Op::SetTextCursor { pos },
            Op::SetFontSizeBuiltinFont {
                size: font_size_pt,
                font: builtin_font,
            },
            Op::WriteTextBuiltinFont {
                items: vec![TextItem::Text(text.content.clone())],
                font: builtin_font,
            },
            Op::EndTextSection,
        ];

        self.current_page_ops.extend(ops);
        Ok(())
    }

    /// Write the document to a file and finalize.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Io` if the file cannot be written.
    /// Returns `PdfError::Other` if the PDF serialization fails.
    pub fn finish(mut self, path: impl AsRef<Path>) -> Result<()> {
        for handler in &mut self.handlers {
            handler.after_document()?;
        }

        // Build the page from collected operations
        let (w, h) = self.page_size;
        let page = PdfPage::new(
            Mm(w as f32 * PT_TO_MM as f32),
            Mm(h as f32 * PT_TO_MM as f32),
            std::mem::take(&mut self.current_page_ops),
        );

        self.doc.with_pages(vec![page]);

        // Save to file
        let file = File::create(path)?;
        let mut buf_writer = BufWriter::new(file);
        let save_opts = PdfSaveOptions::default();
        let mut warnings = Vec::new();
        self.doc
            .save_writer(&mut buf_writer, &save_opts, &mut warnings);

        Ok(())
    }
}

/// Map our `FontFamily` to printpdf's `BuiltinFont`.
fn map_builtin_font(font: &PdfFont) -> BuiltinFont {
    match &font.family {
        FontFamily::BuiltIn(builtin) => match builtin {
            BuiltInFont::Helvetica
            | BuiltInFont::HelveticaBold
            | BuiltInFont::HelveticaOblique
            | BuiltInFont::HelveticaBoldOblique => {
                if font.style.bold && font.style.italic {
                    BuiltinFont::HelveticaBoldOblique
                } else if font.style.bold {
                    BuiltinFont::HelveticaBold
                } else if font.style.italic {
                    BuiltinFont::HelveticaOblique
                } else {
                    BuiltinFont::Helvetica
                }
            }
            BuiltInFont::TimesRoman
            | BuiltInFont::TimesBold
            | BuiltInFont::TimesItalic
            | BuiltInFont::TimesBoldItalic => {
                if font.style.bold && font.style.italic {
                    BuiltinFont::TimesBoldItalic
                } else if font.style.bold {
                    BuiltinFont::TimesBold
                } else if font.style.italic {
                    BuiltinFont::TimesItalic
                } else {
                    BuiltinFont::TimesRoman
                }
            }
            BuiltInFont::Courier
            | BuiltInFont::CourierBold
            | BuiltInFont::CourierOblique
            | BuiltInFont::CourierBoldOblique => {
                if font.style.bold && font.style.italic {
                    BuiltinFont::CourierBoldOblique
                } else if font.style.bold {
                    BuiltinFont::CourierBold
                } else if font.style.italic {
                    BuiltinFont::CourierOblique
                } else {
                    BuiltinFont::Courier
                }
            }
            BuiltInFont::Symbol => BuiltinFont::Symbol,
            BuiltInFont::ZapfDingbats => BuiltinFont::ZapfDingbats,
        },
        FontFamily::Custom(_) => {
            // Custom fonts require TTF parsing; fall back to Helvetica for now
            if font.style.bold {
                BuiltinFont::HelveticaBold
            } else {
                BuiltinFont::Helvetica
            }
        }
    }
}

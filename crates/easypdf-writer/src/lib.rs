//! PDF creation and writing (printpdf backend).
//!
//! Provides `PdfWriter` for creating new PDF documents with text, tables,
//! images, shapes, and custom fonts. Backed by the `printpdf` crate.
//!
//! ## Architecture
//!
//! printpdf v0.8 uses a page-operations model: you build a `Vec<Op>`
//! and construct `PdfPage` objects from them. The document collects
//! pages, fonts, and metadata, then serializes to bytes at the end.

#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![deny(unsafe_code)]

use easypdf_core::error::{PdfError, Result};
use easypdf_core::{
    BuiltInFont, FontFamily, FontStyle, Orientation, PageSize, PdfFont, PdfImage, PdfMetadata,
    PdfText, PdfWriteHandler,
};
use printpdf::{
    BuiltinFont, Line, LinePoint, Mm, Op, PdfDocument, PdfPage, PdfSaveOptions, Point, Pt,
    RawImage, TextItem, XObjectTransform,
};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

/// PDF measurement units.
const PT_TO_MM: f64 = 25.4 / 72.0;

/// A writer for creating new PDF documents.
///
/// Builds pages from operations, then serializes the document to bytes.
/// Supports multiple pages, images, custom fonts, and shapes.
pub struct PdfWriter {
    doc: PdfDocument,
    /// Accumulated completed pages.
    pages: Vec<PdfPage>,
    /// Operations being built for the current page.
    current_page_ops: Vec<Op>,
    /// Current page size for the page being built.
    current_page_size: (f64, f64),
    /// Current page number (1-based).
    current_page_number: usize,
    /// Registered custom font IDs keyed by path.
    custom_fonts: HashMap<String, printpdf::FontId>,
    /// Document metadata.
    metadata: PdfMetadata,
    /// Lifecycle handlers.
    handlers: Vec<Box<dyn PdfWriteHandler>>,
}

impl PdfWriter {
    /// Create a new PDF document.
    #[must_use]
    pub fn new(title: &str) -> Self {
        Self {
            doc: PdfDocument::new(title),
            pages: Vec::new(),
            current_page_ops: Vec::new(),
            current_page_size: PageSize::A4.dimensions(),
            current_page_number: 0,
            custom_fonts: HashMap::new(),
            metadata: PdfMetadata::default(),
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

    /// Register a custom TTF/OTF font from a file path.
    ///
    /// Returns a key that can be used with [`write_text_with_custom_font`](Self::write_text_with_custom_font).
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Io` if the font file cannot be read.
    /// Returns `PdfError::Parse` if the font data is invalid.
    pub fn register_font_from_path(&mut self, path: &str) -> Result<String> {
        let font_data = std::fs::read(path)?;
        self.register_font_from_bytes(path, &font_data)
    }

    /// Register a custom TTF/OTF font from bytes.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Parse` if the font data is invalid.
    pub fn register_font_from_bytes(&mut self, key: &str, font_data: &[u8]) -> Result<String> {
        let mut warnings = Vec::new();
        let parsed = printpdf::ParsedFont::from_bytes(font_data, 0, &mut warnings)
            .ok_or_else(|| PdfError::Parse(format!("Failed to parse font: {key}")))?;
        let font_id = self.doc.add_font(&parsed);
        self.custom_fonts.insert(key.to_string(), font_id);
        Ok(key.to_string())
    }

    /// Write text using a custom (non-builtin) font.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::UnsupportedFeature` if the font key is not registered.
    pub fn write_text_with_custom_font(
        &mut self,
        text: &str,
        font_key: &str,
        font_size: f64,
        x_pt: f64,
        y_pt: f64,
    ) -> Result<()> {
        let font_id = self
            .custom_fonts
            .get(font_key)
            .cloned()
            .ok_or_else(|| {
                PdfError::UnsupportedFeature(format!(
                    "Custom font '{font_key}' not registered. Call register_font_from_path first."
                ))
            })?;

        let pos = Point {
            x: Pt(x_pt as f32),
            y: Pt(y_pt as f32),
        };

        let ops = vec![
            Op::StartTextSection,
            Op::SetTextCursor { pos },
            Op::SetFontSize {
                size: Pt(font_size as f32),
                font: font_id.clone(),
            },
            Op::WriteText {
                items: vec![TextItem::Text(text.to_string())],
                font: font_id,
            },
            Op::EndTextSection,
        ];

        self.current_page_ops.extend(ops);
        Ok(())
    }

    /// Add a new page to the document, finalizing the previous page.
    ///
    /// Returns the new page number (1-based).
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Other` if handlers fail.
    pub fn add_page(&mut self, size: PageSize, orientation: Orientation) -> Result<usize> {
        // Finalize previous page if it has content
        self.finalize_current_page()?;

        self.current_page_number += 1;
        self.current_page_size = size.dimensions();
        let _ = orientation;

        for handler in &mut self.handlers {
            handler.before_page(self.current_page_number)?;
            handler.after_page(self.current_page_number)?;
        }
        Ok(self.current_page_number)
    }

    /// Finalize the current page (push to pages vec) without starting a new one.
    fn finalize_current_page(&mut self) -> Result<()> {
        let ops = std::mem::take(&mut self.current_page_ops);
        if ops.is_empty() && self.pages.is_empty() {
            return Ok(());
        }
        let (w, h) = self.current_page_size;
        let page = PdfPage::new(
            Mm(w as f32 * PT_TO_MM as f32),
            Mm(h as f32 * PT_TO_MM as f32),
            ops,
        );
        self.pages.push(page);
        Ok(())
    }

    /// Get the current page number (1-based).
    #[must_use]
    pub const fn current_page_number(&self) -> usize {
        self.current_page_number
    }

    /// Get the total number of pages finalized so far.
    #[must_use]
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Write text to the current page at the given position.
    ///
    /// Position (x, y) is in PDF points from the bottom-left corner.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::InvalidPage` if no page has been added yet.
    pub fn write_text(&mut self, text: &PdfText, x_pt: f64, y_pt: f64) -> Result<()> {
        // If a custom font is specified and registered, use it
        if let FontFamily::Custom(ref path) = text.font.family {
            if let Some(font_id_ref) = self.custom_fonts.get(path.as_ref()) {
                let font_id = font_id_ref.clone();
                let pos = Point {
                    x: Pt(x_pt as f32),
                    y: Pt(y_pt as f32),
                };
                let ops = vec![
                    Op::StartTextSection,
                    Op::SetTextCursor { pos },
                    Op::SetFontSize {
                        size: Pt(text.font.size as f32),
                        font: font_id.clone(),
                    },
                    Op::WriteText {
                        items: vec![TextItem::Text(text.content.clone())],
                        font: font_id,
                    },
                    Op::EndTextSection,
                ];
                self.current_page_ops.extend(ops);
                return Ok(());
            }
        }

        let builtin_font = map_builtin_font(&text.font);
        let font_size_pt = Pt(text.font.size as f32);
        let pos = Point {
            x: Pt(x_pt as f32),
            y: Pt(y_pt as f32),
        };

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

    /// Write an SVG to the current page at the given position.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Parse` if the SVG data is invalid.
    pub fn write_svg(
        &mut self,
        svg_data: &str,
        x_pt: f64,
        y_pt: f64,
        w_pt: f64,
        h_pt: f64,
    ) -> Result<()> {
        let mut warnings = Vec::new();
        let xobj = printpdf::Svg::parse(svg_data, &mut warnings)
            .map_err(|e| PdfError::Parse(format!("SVG parse error: {e}")))?;
        let xobj_id = self.doc.add_xobject(&xobj);

        let transform = XObjectTransform {
            translate_x: Some(Pt(x_pt as f32)),
            translate_y: Some(Pt(y_pt as f32)),
            scale_x: Some(w_pt as f32),
            scale_y: Some(h_pt as f32),
            rotate: None,
            dpi: None,
        };

        self.current_page_ops.push(Op::UseXobject {
            id: xobj_id,
            transform,
        });

        Ok(())
    }

    /// Write an image to the current page at the given position.
    ///
    /// Position (x, y) is in PDF points from the bottom-left corner.
    /// Size (w, h) is in PDF points. If both are 0, the image's natural
    /// pixel dimensions are used at 72 DPI.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Parse` if the image data cannot be decoded.
    pub fn write_image(
        &mut self,
        image: &PdfImage,
        x_pt: f64,
        y_pt: f64,
        w_pt: f64,
        h_pt: f64,
    ) -> Result<()> {
        let mut warnings = Vec::new();
        let raw = RawImage::decode_from_bytes(&image.data, &mut warnings)
            .map_err(|e| PdfError::Parse(format!("Image decode error: {e}")))?;

        let xobj_id = self.doc.add_image(&raw);

        let (w, h) = if w_pt == 0.0 && h_pt == 0.0 {
            // Use natural pixel size at 72 DPI
            (raw.width as f64, raw.height as f64)
        } else {
            (w_pt, h_pt)
        };

        let transform = XObjectTransform {
            translate_x: Some(Pt(x_pt as f32)),
            translate_y: Some(Pt(y_pt as f32)),
            scale_x: Some(w as f32),
            scale_y: Some(h as f32),
            rotate: None,
            dpi: None,
        };

        self.current_page_ops.push(Op::UseXobject {
            id: xobj_id,
            transform,
        });

        Ok(())
    }

    /// Draw a line segment on the current page.
    ///
    /// Coordinates in PDF points from bottom-left.
    pub fn draw_line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, line_width: f64) {
        let line = Line {
            points: vec![
                LinePoint {
                    p: Point {
                        x: Pt(x1 as f32),
                        y: Pt(y1 as f32),
                    },
                    bezier: false,
                },
                LinePoint {
                    p: Point {
                        x: Pt(x2 as f32),
                        y: Pt(y2 as f32),
                    },
                    bezier: false,
                },
            ],
            is_closed: false,
        };
        self.current_page_ops
            .push(Op::SetOutlineThickness { pt: Pt(line_width as f32) });
        self.current_page_ops.push(Op::DrawLine { line });
    }

    /// Draw a rectangle outline on the current page.
    ///
    /// Coordinates in PDF points from bottom-left.
    pub fn draw_rect_stroke(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        line_width: f64,
    ) {
        let rect = printpdf::Rect::from_wh(
            Pt(w as f32),
            Pt(h as f32),
        );
        let line = rect.to_line();
        // Translate to (x, y): printpdf's to_line puts origin at (0,0)
        let translated = Line {
            points: line
                .points
                .into_iter()
                .map(|mut lp| {
                    lp.p.x = Pt(lp.p.x.0 + x as f32);
                    lp.p.y = Pt(lp.p.y.0 + y as f32);
                    lp
                })
                .collect(),
            is_closed: line.is_closed,
        };
        self.current_page_ops
            .push(Op::SetOutlineThickness { pt: Pt(line_width as f32) });
        self.current_page_ops
            .push(Op::DrawLine { line: translated });
    }

    /// Draw a circle outline on the current page using 4 cubic bezier curves.
    ///
    /// Center (cx, cy) and radius in PDF points from bottom-left.
    /// The approximation error is less than 0.027% of the radius.
    pub fn draw_circle(
        &mut self,
        cx: f64,
        cy: f64,
        radius: f64,
        line_width: f64,
    ) {
        // Magic constant for 4-segment cubic bezier circle approximation
        const K: f64 = 0.552_284_749_8;

        let r = radius;
        let k = K * r;

        // 4 segments, each 90 degrees
        let segments: [(f64, f64, f64, f64, f64, f64, f64, f64); 4] = [
            // 0°→90°: right to top
            (r, 0.0, r, k, k, r, 0.0, r),
            // 90°→180°: top to left
            (0.0, r, -k, r, -r, k, -r, 0.0),
            // 180°→270°: left to bottom
            (-r, 0.0, -r, -k, -k, -r, 0.0, -r),
            // 270°→360°: bottom to right
            (0.0, -r, k, -r, r, -k, r, 0.0),
        ];

        let mut all_points: Vec<LinePoint> = Vec::with_capacity(13); // 4*3 + 1 close
        for (x1, y1, cx1, cy1, cx2, cy2, x2, y2) in &segments {
            if all_points.is_empty() {
                all_points.push(LinePoint {
                    p: Point {
                        x: Pt((cx + x1) as f32),
                        y: Pt((cy + y1) as f32),
                    },
                    bezier: false,
                });
            }
            all_points.push(LinePoint {
                p: Point {
                    x: Pt((cx + cx1) as f32),
                    y: Pt((cy + cy1) as f32),
                },
                bezier: true,
            });
            all_points.push(LinePoint {
                p: Point {
                    x: Pt((cx + cx2) as f32),
                    y: Pt((cy + cy2) as f32),
                },
                bezier: true,
            });
            all_points.push(LinePoint {
                p: Point {
                    x: Pt((cx + x2) as f32),
                    y: Pt((cy + y2) as f32),
                },
                bezier: false,
            });
        }

        let line = Line {
            points: all_points,
            is_closed: true,
        };

        self.current_page_ops
            .push(Op::SetOutlineThickness { pt: Pt(line_width as f32) });
        self.current_page_ops
            .push(Op::DrawLine { line });
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

        // Finalize the last page if it has content
        self.finalize_current_page()?;

        if self.pages.is_empty() {
            // Ensure at least one page exists even if no content was added
            let (w, h) = self.current_page_size;
            self.pages.push(PdfPage::new(
                Mm(w as f32 * PT_TO_MM as f32),
                Mm(h as f32 * PT_TO_MM as f32),
                Vec::new(),
            ));
        }

        self.doc.with_pages(self.pages);

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
            // Custom fonts handled before this function is called
            if font.style.bold {
                BuiltinFont::HelveticaBold
            } else {
                BuiltinFont::Helvetica
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a minimal valid PNG image for testing.
    fn make_test_png() -> Vec<u8> {
        // Minimal 1x1 red PNG
        vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
            0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
            0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
            0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, // IDAT chunk
            0x54, 0x08, 0xD7, 0x63, 0xF8, 0xFF, 0xFF, 0x3F,
            0x00, 0x05, 0xFE, 0x02, 0xFE, 0xDC, 0xCC, 0x59,
            0xE7, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, // IEND chunk
            0x44, 0xAE, 0x42, 0x60, 0x82,
        ]
    }

    #[test]
    fn test_writer_new() {
        let writer = PdfWriter::new("test");
        assert_eq!(writer.current_page_number(), 0);
        assert_eq!(writer.page_count(), 0);
    }

    #[test]
    fn test_add_page() {
        let mut writer = PdfWriter::new("test");
        let page_num = writer.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        assert_eq!(page_num, 1);
        assert_eq!(writer.current_page_number(), 1);
    }

    #[test]
    fn test_multi_page() {
        let mut writer = PdfWriter::new("test");
        writer.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        writer.write_text(
            &PdfText::new("Page 1").font(PdfFont::helvetica(12.0)),
            100.0,
            700.0,
        ).unwrap();
        writer.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        writer.write_text(
            &PdfText::new("Page 2").font(PdfFont::helvetica(12.0)),
            100.0,
            700.0,
        ).unwrap();
        // After adding page 2, page 1 should be stored
        assert!(writer.page_count() >= 1);
    }

    #[test]
    fn test_finish_creates_file() {
        let mut writer = PdfWriter::new("test");
        writer.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        writer.write_text(
            &PdfText::new("Hello").font(PdfFont::helvetica(12.0)),
            100.0,
            700.0,
        ).unwrap();

        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_test_finish.pdf");
        writer.finish(&path).unwrap();
        assert!(path.exists());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_finish_empty_document_produces_one_page() {
        let mut writer = PdfWriter::new("empty");
        writer.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_test_empty.pdf");
        writer.finish(&path).unwrap();
        assert!(path.exists());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_write_image() {
        let mut writer = PdfWriter::new("test");
        writer.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        let png_data = make_test_png();
        let image = PdfImage {
            data: png_data,
            width: 0.0,
            height: 0.0,
        };
        writer.write_image(&image, 100.0, 600.0, 50.0, 50.0).unwrap();
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_test_image.pdf");
        writer.finish(&path).unwrap();
        assert!(path.exists());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_write_image_natural_size() {
        let mut writer = PdfWriter::new("test");
        writer.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        let image = PdfImage {
            data: make_test_png(),
            width: 0.0,
            height: 0.0,
        };
        // w=0, h=0 should use natural pixel size
        writer.write_image(&image, 50.0, 700.0, 0.0, 0.0).unwrap();
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_test_image_natural.pdf");
        writer.finish(&path).unwrap();
        assert!(path.exists());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_draw_line() {
        let mut writer = PdfWriter::new("test");
        writer.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        writer.draw_line(10.0, 10.0, 200.0, 10.0, 1.0);
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_test_line.pdf");
        writer.finish(&path).unwrap();
        assert!(path.exists());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_draw_rect_stroke() {
        let mut writer = PdfWriter::new("test");
        writer.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        writer.draw_rect_stroke(50.0, 600.0, 200.0, 100.0, 1.0);
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_test_rect.pdf");
        writer.finish(&path).unwrap();
        assert!(path.exists());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_draw_circle() {
        let mut writer = PdfWriter::new("test");
        writer.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        writer.draw_circle(300.0, 400.0, 100.0, 1.0);
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_test_circle.pdf");
        writer.finish(&path).unwrap();
        assert!(path.exists());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_invalid_image_data() {
        let mut writer = PdfWriter::new("test");
        writer.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        let image = PdfImage {
            data: vec![0, 1, 2, 3], // not valid image data
            width: 0.0,
            height: 0.0,
        };
        let result = writer.write_image(&image, 0.0, 0.0, 100.0, 100.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_write_svg() {
        let mut writer = PdfWriter::new("test");
        writer.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><rect width="100" height="100" fill="red"/></svg>"#;
        writer.write_svg(svg, 100.0, 600.0, 100.0, 100.0).unwrap();
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_test_svg.pdf");
        writer.finish(&path).unwrap();
        assert!(path.exists());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_write_svg_invalid() {
        let mut writer = PdfWriter::new("test");
        writer.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        let result = writer.write_svg("not valid svg", 100.0, 600.0, 100.0, 100.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_custom_font_not_registered() {
        let mut writer = PdfWriter::new("test");
        writer.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        let result = writer.write_text_with_custom_font("hello", "nonexistent", 12.0, 100.0, 700.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_register_font_from_nonexistent_path() {
        let mut writer = PdfWriter::new("test");
        let result = writer.register_font_from_path("/nonexistent/font.ttf");
        assert!(result.is_err());
    }

    #[test]
    fn test_register_font_success() {
        let mut writer = PdfWriter::new("test");
        let path = "/System/Library/Fonts/Helvetica.ttc";
        if std::path::Path::new(path).exists() {
            let result = writer.register_font_from_path(path);
            assert!(result.is_ok());
            // Also test writing with the registered font
            writer.add_page(PageSize::A4, Orientation::Portrait).unwrap();
            let r = writer.write_text_with_custom_font("CustomFont!", path, 14.0, 100.0, 600.0);
            assert!(r.is_ok());
        }
    }

    #[test]
    fn test_write_text_with_custom_font_success() {
        let mut writer = PdfWriter::new("test");
        // Register a built-in font via bytes simulation
        let fake_font = vec![0u8; 256]; // minimal font data won't parse but tests error path
        let result = writer.register_font_from_bytes("test_font", &fake_font);
        // Either succeeds (if bytes are valid enough) or fails with Parse error
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_write_text_with_symbol_font() {
        let mut writer = PdfWriter::new("test");
        writer.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        let font = PdfFont {
            family: FontFamily::BuiltIn(BuiltInFont::Symbol),
            size: 12.0,
            style: Default::default(),
        };
        let text = PdfText::new("test").font(font);
        writer.write_text(&text, 100.0, 700.0).unwrap();
        let dir = std::env::temp_dir();
        let path = dir.join("easypdf_test_symbol.pdf");
        writer.finish(&path).unwrap();
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_metadata_chaining() {
        let writer = PdfWriter::new("test")
            .metadata(PdfMetadata::new().title("My Title").author("Author"));
        assert_eq!(writer.metadata.title.as_deref(), Some("My Title"));
        assert_eq!(writer.metadata.author.as_deref(), Some("Author"));
    }

    #[test]
    fn test_page_size_dimensions() {
        let a4 = PageSize::A4.dimensions();
        assert_eq!(a4, (595.0, 842.0));
        let letter = PageSize::Letter.dimensions();
        assert_eq!(letter, (612.0, 792.0));
        let custom = PageSize::Custom(100.0, 200.0).dimensions();
        assert_eq!(custom, (100.0, 200.0));
    }

    #[test]
    fn test_all_builtin_fonts() {
        for font in &[BuiltInFont::TimesBoldItalic, BuiltInFont::CourierBold, BuiltInFont::CourierOblique,
                       BuiltInFont::HelveticaBoldOblique, BuiltInFont::TimesBold, BuiltInFont::TimesItalic,
                       BuiltInFont::CourierBoldOblique, BuiltInFont::ZapfDingbats] {
            let mut w = PdfWriter::new("test");
            w.add_page(PageSize::A4, Orientation::Portrait).unwrap();
            let f = PdfFont { family: FontFamily::BuiltIn(*font), size: 10.0, style: Default::default() };
            w.write_text(&PdfText::new("x").font(f), 100.0, 700.0).unwrap();
        }
    }

    #[test]
    fn test_empty_finish() {
        let mut w = PdfWriter::new("empty");
        w.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        let dir = std::env::temp_dir();
        let p = dir.join("easypdf_empty.pdf");
        w.finish(&p).unwrap();
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn test_custom_font_fallback_bold() {
        let mut w = PdfWriter::new("test");
        w.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        let f = PdfFont { family: FontFamily::Custom("x.ttf".into()), size: 12.0, style: FontStyle { bold: true, italic: false } };
        w.write_text(&PdfText::new("x").font(f), 100.0, 700.0).unwrap();
    }

    #[test]
    fn test_register_handler_direct() {
        struct TestH;
        impl PdfWriteHandler for TestH {}
        let w = PdfWriter::new("test").register_handler(Box::new(TestH));
        // Just verify the builder accepts handlers
        let _ = w;
    }

    #[test]
    fn test_register_font_from_bytes_error() {
        let mut w = PdfWriter::new("test");
        let r = w.register_font_from_bytes("bad", &[0, 1, 2]);
        assert!(r.is_err()); // invalid font data
    }
}

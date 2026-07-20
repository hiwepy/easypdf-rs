//! # easypdf-rs
//!
//! An idiomatic Rust library for quick PDF operations: creation, reading,
//! manipulation, and template filling. Inspired by Alibaba EasyExcel's
//! builder-pattern API design.
//!
//! ## Quick examples
//!
//! **Create a PDF:**
//! ```ignore
//! use easypdf::prelude::*;
//!
//! EasyPdf::create("output.pdf")
//!     .page(PageSize::A4)
//!     .add_text("Hello, world!")
//!         .font(PdfFont::helvetica(12.0))
//!     .do_write()?;
//! ```
//!
//! **Read a PDF:**
//! ```ignore
//! let text = EasyPdf::read("input.pdf").extract_text()?;
//! ```
//!
//! **Merge PDFs:**
//! ```ignore
//! EasyPdf::merge(&["a.pdf", "b.pdf"], "merged.pdf")?;
//! ```
//!
//! **Fill a form:**
//! ```ignore
//! #[derive(PdfModel)]
//! struct MyData {
//!     #[pdf(field = "name")]
//!     name: String,
//! }
//!
//! EasyPdf::fill_form("template.pdf", &MyData { name: "Alice".into() })
//!     .save("filled.pdf")?;
//! ```

#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![deny(unsafe_code)]

// --- Re-exports from sub-crates ---

// Core types (flat re-export)
pub use easypdf_core::*;

// Derive macro
pub use easypdf_derive::PdfModel;

// Reader
pub use easypdf_reader::PdfReader;

// Writer
pub use easypdf_writer::PdfWriter;

// Manipulate
pub use easypdf_manipulate::PdfManipulator;

// Template
pub use easypdf_template::PdfTemplateFiller;

// --- Builder entry points ---

use std::path::{Path, PathBuf};

/// The main entry point for all `easypdf-rs` operations.
///
/// Provides static factory methods that return ergonomic builder chains
/// for creating, reading, manipulating, and filling PDFs.
pub struct EasyPdf;

impl EasyPdf {
    // --- Create ---

    /// Start building a new PDF document.
    ///
    /// Returns a [`PdfCreateBuilder`] for configuring pages, content, and metadata.
    #[must_use]
    pub fn create(path: impl Into<PathBuf>) -> PdfCreateBuilder {
        PdfCreateBuilder::new(path)
    }

    // --- Read ---

    /// Start building a PDF reader for text extraction.
    ///
    /// Returns a [`PdfReadBuilder`] for configuring page ranges and extraction modes.
    #[must_use]
    pub fn read(path: impl Into<PathBuf>) -> PdfReadBuilder {
        PdfReadBuilder::new(path)
    }

    // --- Merge ---

    /// Merge multiple PDF files into a single output file.
    ///
    /// # Errors
    ///
    /// Returns an error if any input file cannot be read or the output cannot be written.
    pub fn merge(input_paths: &[impl AsRef<Path>], output: impl AsRef<Path>) -> Result<()> {
        easypdf_manipulate::PdfManipulator::merge_files(input_paths, output)
    }

    // --- Split ---

    /// Start building a PDF split operation.
    #[must_use]
    pub fn split(path: impl Into<PathBuf>) -> PdfSplitBuilder {
        PdfSplitBuilder::new(path)
    }

    // --- Manipulate ---

    /// Start building a PDF manipulation (rotate, reorder, etc.).
    #[must_use]
    pub fn manipulate(path: impl Into<PathBuf>) -> PdfManipulateBuilder {
        PdfManipulateBuilder::new(path)
    }

    // --- Template / Form filling ---

    /// Fill a PDF form template with data.
    ///
    /// Returns a [`PdfFillBuilder`] for configuring field values and saving.
    #[must_use]
    pub fn fill_form(
        template_path: impl Into<PathBuf>,
        data: &dyn easypdf_core::PdfModel,
    ) -> PdfFillBuilder {
        PdfFillBuilder::new(template_path, data)
    }
}

// ======================================================================
// Builder types
// ======================================================================

/// Builder for creating new PDF documents.
#[must_use]
pub struct PdfCreateBuilder {
    path: PathBuf,
    title: String,
    page_size: PageSize,
    orientation: Orientation,
    metadata: PdfMetadata,
    #[allow(dead_code)]
    fonts: Vec<PdfFont>,
    handlers: Vec<Box<dyn easypdf_core::PdfWriteHandler>>,
}

impl PdfCreateBuilder {
    fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            title: String::from("Untitled"),
            page_size: PageSize::A4,
            orientation: Orientation::default(),
            metadata: PdfMetadata::default(),
            fonts: Vec::new(),
            handlers: Vec::new(),
        }
    }

    /// Set the document title.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the default page size.
    #[must_use]
    pub const fn page_size(mut self, size: PageSize) -> Self {
        self.page_size = size;
        self
    }

    /// Set the page orientation.
    #[must_use]
    pub const fn orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Set document metadata.
    #[must_use]
    pub fn metadata(mut self, metadata: PdfMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Register a write handler.
    #[must_use]
    pub fn register_handler(mut self, handler: Box<dyn easypdf_core::PdfWriteHandler>) -> Self {
        self.handlers.push(handler);
        self
    }

    /// Write text and finalize the document in one call.
    ///
    /// This is a convenience method for simple single-page PDFs.
    ///
    /// # Errors
    ///
    /// Returns an error if the PDF cannot be written.
    pub fn add_text(self, content: impl Into<String>) -> PdfTextBuilder<Self> {
        PdfTextBuilder {
            parent: self,
            text: PdfText::new(content),
        }
    }

    /// Build the writer for manual page-by-page construction.
    ///
    /// # Errors
    ///
    /// Returns an error if the writer cannot be initialized.
    pub fn build(self) -> Result<easypdf_writer::PdfWriter> {
        let mut writer = easypdf_writer::PdfWriter::new(&self.title);
        writer = writer.metadata(self.metadata);
        for handler in self.handlers {
            writer = writer.register_handler(handler);
        }
        Ok(writer)
    }

    /// Build, add a default page, write text, and save — all in one call.
    ///
    /// # Errors
    ///
    /// Returns an error if the PDF cannot be created or written.
    pub fn do_write(self) -> Result<PathBuf> {
        let path = self.path.clone();
        let page_size = self.page_size;
        let orientation = self.orientation;
        let mut writer = self.build()?;
        writer.add_page(page_size, orientation)?;
        writer.finish(&path)?;
        Ok(path)
    }
}

/// Builder for adding text to a PDF, returned by [`PdfCreateBuilder::add_text`].
#[must_use]
pub struct PdfTextBuilder<P> {
    parent: P,
    text: PdfText,
}

impl PdfTextBuilder<PdfCreateBuilder> {
    /// Set the font for this text.
    #[must_use]
    pub fn font(mut self, font: PdfFont) -> Self {
        self.text = self.text.font(font);
        self
    }

    /// Set the position as (x, y) in PDF points.
    #[must_use]
    pub fn position(self, x: f64, y: f64) -> PdfPositionedTextBuilder {
        PdfPositionedTextBuilder {
            parent: self.parent,
            text: self.text,
            x,
            y,
        }
    }

    /// Finalize by writing the text at the default position (100, 700).
    ///
    /// # Errors
    ///
    /// Returns an error if the PDF cannot be created or written.
    pub fn do_write(self) -> Result<PathBuf> {
        let path = self.parent.path.clone();
        let page_size = self.parent.page_size;
        let orientation = self.parent.orientation;
        let mut writer = self.parent.build()?;
        writer.add_page(page_size, orientation)?;
        writer.write_text(&self.text, 100.0, 700.0)?;
        writer.finish(&path)?;
        Ok(path)
    }
}

/// Builder for text with an explicit position.
#[must_use]
pub struct PdfPositionedTextBuilder {
    parent: PdfCreateBuilder,
    text: PdfText,
    x: f64,
    y: f64,
}

impl PdfPositionedTextBuilder {
    /// Finalize and write the PDF.
    ///
    /// # Errors
    ///
    /// Returns an error if the PDF cannot be created or written.
    pub fn do_write(self) -> Result<PathBuf> {
        let path = self.parent.path.clone();
        let page_size = self.parent.page_size;
        let orientation = self.parent.orientation;
        let mut writer = self.parent.build()?;
        writer.add_page(page_size, orientation)?;
        writer.write_text(&self.text, self.x, self.y)?;
        writer.finish(&path)?;
        Ok(path)
    }
}

// --- Read builder ---

/// Builder for reading/extracting content from PDFs.
#[must_use]
pub struct PdfReadBuilder {
    path: PathBuf,
    pages: Option<std::ops::Range<usize>>,
}

impl PdfReadBuilder {
    fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            pages: None,
        }
    }

    /// Limit extraction to a specific page range (0-based).
    #[must_use]
    pub fn pages(mut self, range: std::ops::Range<usize>) -> Self {
        self.pages = Some(range);
        self
    }

    /// Extract all text from the PDF.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Parse` if the PDF cannot be read.
    pub fn extract_text(self) -> Result<String> {
        let mut reader = easypdf_reader::PdfReader::open(&self.path)?;
        if let Some(range) = self.pages {
            reader = reader.pages(range);
        }
        reader.extract_text()
    }

    /// Extract PDF metadata.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Parse` if the PDF cannot be read.
    pub fn metadata(self) -> Result<PdfMetadata> {
        easypdf_reader::PdfReader::open(&self.path)?.extract_metadata()
    }

    /// Get the number of pages.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Parse` if the PDF cannot be read.
    pub fn page_count(self) -> Result<usize> {
        easypdf_reader::PdfReader::open(&self.path)?.page_count()
    }
}

// --- Split builder ---

/// Builder for splitting a PDF into individual pages.
#[must_use]
pub struct PdfSplitBuilder {
    path: PathBuf,
    pages_per_file: usize,
}

impl PdfSplitBuilder {
    fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            pages_per_file: 1,
        }
    }

    /// Set the number of pages per split file (default: 1).
    #[must_use]
    pub const fn every_n_pages(mut self, n: usize) -> Self {
        self.pages_per_file = n;
        self
    }

    /// Split the PDF and save pages to a directory.
    ///
    /// Files are named `page_001.pdf`, `page_002.pdf`, etc.
    ///
    /// # Errors
    ///
    /// Returns an error if the PDF cannot be read or split files cannot be written.
    pub fn save_to_dir(self, output_dir: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
        let manipulator = easypdf_manipulate::PdfManipulator::open(&self.path)?;
        let total_pages = manipulator.page_count();
        let output_dir = output_dir.as_ref();
        std::fs::create_dir_all(output_dir)?;

        let mut output_paths = Vec::new();
        let mut start = 0;

        while start < total_pages {
            let end = std::cmp::min(start + self.pages_per_file, total_pages);
            let mut chunk = manipulator.extract_pages(start..end)?;
            let filename = format!("page_{:03}.pdf", start / self.pages_per_file + 1);
            let output_path = output_dir.join(&filename);
            chunk.save(&output_path)?;
            output_paths.push(output_path);
            start = end;
        }

        Ok(output_paths)
    }
}

// --- Manipulate builder ---

/// Builder for PDF manipulation operations (rotate, reorder, watermark).
#[must_use]
pub struct PdfManipulateBuilder {
    path: PathBuf,
    rotations: Vec<(usize, Rotation)>,
    order: Option<Vec<usize>>,
}

impl PdfManipulateBuilder {
    fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            rotations: Vec::new(),
            order: None,
        }
    }

    /// Rotate a specific page (1-based index).
    #[must_use]
    pub fn rotate_page(mut self, page_number: usize, rotation: Rotation) -> Self {
        self.rotations.push((page_number, rotation));
        self
    }

    /// Rotate all pages.
    #[must_use]
    pub fn rotate_all(self, rotation: Rotation) -> Self {
        // This will be applied inside save() by iterating all pages
        self.rotate(rotation)
    }

    /// Rotate all pages (alias for builder chain).
    #[must_use]
    pub fn rotate(mut self, rotation: Rotation) -> Self {
        self.rotations.push((0, rotation)); // 0 means "all pages"
        self
    }

    /// Reorder pages according to the given permutation (0-based).
    #[must_use]
    pub fn reorder_pages(mut self, order: &[usize]) -> Self {
        self.order = Some(order.to_vec());
        self
    }

    /// Apply all manipulations and save to the output file.
    ///
    /// # Errors
    ///
    /// Returns an error if the PDF cannot be read or saved.
    pub fn save(self, output: impl AsRef<Path>) -> Result<()> {
        let mut manipulator = easypdf_manipulate::PdfManipulator::open(&self.path)?;

        for (page_num, rotation) in &self.rotations {
            if *page_num == 0 {
                // Apply to all pages
                let count = manipulator.page_count();
                for p in 1..=count {
                    manipulator.rotate_page(p, *rotation)?;
                }
            } else {
                manipulator.rotate_page(*page_num, *rotation)?;
            }
        }

        if let Some(order) = &self.order {
            manipulator.reorder_pages(order)?;
        }

        manipulator.save(output)
    }
}

// --- Fill builder ---

/// Builder for filling PDF form templates.
#[must_use]
pub struct PdfFillBuilder {
    template_path: PathBuf,
    fields: Vec<(String, String)>,
}

impl PdfFillBuilder {
    fn new(template_path: impl Into<PathBuf>, data: &dyn easypdf_core::PdfModel) -> Self {
        let _ = data; // The PdfModel trait will be used to extract field mappings
        Self {
            template_path: template_path.into(),
            fields: Vec::new(),
        }
    }

    /// Add a field value to fill.
    #[must_use]
    pub fn field(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.push((name.into(), value.into()));
        self
    }

    /// Add multiple field values.
    #[must_use]
    pub fn fields(
        mut self,
        fields: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        for (name, value) in fields {
            self.fields.push((name.into(), value.into()));
        }
        self
    }

    /// Fill form fields and save to the output file.
    ///
    /// # Errors
    ///
    /// Returns an error if the template cannot be read or the output cannot be written.
    pub fn save(self, output: impl AsRef<Path>) -> Result<()> {
        let mut filler = easypdf_template::PdfTemplateFiller::open(&self.template_path)?;
        for (name, value) in &self.fields {
            filler.fill_field(name, value)?;
        }
        filler.save(output)
    }
}

/// Convenience re-exports in a `prelude` module.
pub mod prelude {
    pub use super::EasyPdf;
    pub use easypdf_core::*;
    pub use easypdf_derive::PdfModel;
}

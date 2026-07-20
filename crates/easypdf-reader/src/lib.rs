//! PDF reading and text extraction (lopdf backend).
//!
//! Provides `PdfReader` for parsing PDF documents and extracting text,
//! metadata, and page information.

#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![deny(unsafe_code)]

use easypdf_core::error::{PdfError, Result};
use easypdf_core::{PdfMetadata, PdfReadListener};
use std::path::Path;

/// A reader for extracting content from PDF documents.
///
/// Backed by the `lopdf` crate for low-level PDF parsing.
pub struct PdfReader {
    path: std::path::PathBuf,
    pages: Option<std::ops::Range<usize>>,
}

impl PdfReader {
    /// Open a PDF file for reading.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Parse` if the file cannot be opened or is not a valid PDF.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        // Verify the file exists and can be opened
        let _doc = lopdf::Document::load(&path).map_err(|e| PdfError::Parse(e.to_string()))?;
        Ok(Self { path, pages: None })
    }

    /// Limit extraction to a specific page range (0-based).
    #[must_use]
    pub fn pages(mut self, range: std::ops::Range<usize>) -> Self {
        self.pages = Some(range);
        self
    }

    /// Extract text from all selected pages, joined with newlines.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Parse` if the PDF content cannot be read.
    pub fn extract_text(&self) -> Result<String> {
        let doc = lopdf::Document::load(&self.path).map_err(|e| PdfError::Parse(e.to_string()))?;
        let pages_map = doc.get_pages();
        let page_count = pages_map.len();
        let range = self.pages.clone().unwrap_or(0..page_count);

        let mut all_text = String::new();
        // get_pages() returns BTreeMap<u32, ObjectId>, sorted by page number
        for (page_num, _page_id) in &pages_map {
            let idx = *page_num as usize;
            if !range.contains(&idx) {
                continue;
            }
            // extract_text takes &[u32] — page numbers
            if let Ok(text) = doc.extract_text(&[*page_num]) {
                if !all_text.is_empty() {
                    all_text.push('\n');
                }
                all_text.push_str(&text);
            }
        }
        Ok(all_text)
    }

    /// Extract metadata from the PDF document.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Parse` if the document cannot be read.
    pub fn extract_metadata(&self) -> Result<PdfMetadata> {
        let doc = lopdf::Document::load(&self.path).map_err(|e| PdfError::Parse(e.to_string()))?;

        // Try to read the /Info dictionary from the trailer
        let title = doc
            .trailer
            .get(b"Info")
            .ok()
            .and_then(|info| {
                let info_id = info.as_reference().ok()?;
                doc.get_object(info_id).ok()
            })
            .and_then(|obj| obj.as_dict().ok())
            .and_then(|dict| {
                dict.get(b"Title")
                    .ok()
                    .and_then(|v| v.as_string().ok())
                    .map(|s| s.to_string())
            });

        let author = doc
            .trailer
            .get(b"Info")
            .ok()
            .and_then(|info| {
                let info_id = info.as_reference().ok()?;
                doc.get_object(info_id).ok()
            })
            .and_then(|obj| obj.as_dict().ok())
            .and_then(|dict| {
                dict.get(b"Author")
                    .ok()
                    .and_then(|v| v.as_string().ok())
                    .map(|s| s.to_string())
            });

        Ok(PdfMetadata {
            title,
            author,
            subject: None,
            keywords: None,
            creator: None,
            producer: None,
        })
    }

    /// Get the total number of pages in the document.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Parse` if the document cannot be read.
    pub fn page_count(&self) -> Result<usize> {
        let doc = lopdf::Document::load(&self.path).map_err(|e| PdfError::Parse(e.to_string()))?;
        Ok(doc.get_pages().len())
    }

    /// Read the document with an event-driven listener.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Parse` if the document cannot be read.
    pub fn read_with_listener(&self, listener: &mut dyn PdfReadListener) -> Result<()> {
        let text = self.extract_text()?;
        // Split by double newline as a crude page separator
        let pages: Vec<&str> = text.split("\n\n").filter(|p| !p.is_empty()).collect();

        for (i, page_text) in pages.iter().enumerate() {
            listener.on_page_start(i + 1)?;
            listener.on_text(i + 1, page_text)?;
            listener.on_page_end(i + 1)?;
        }
        listener.on_document_end()?;
        Ok(())
    }
}

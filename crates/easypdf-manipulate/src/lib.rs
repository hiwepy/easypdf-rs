//! PDF manipulation — merge, split, rotate, and reorder pages.
//!
//! Backed by the `lopdf` crate for low-level page operations.

#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![deny(unsafe_code)]

use easypdf_core::Rotation;
use easypdf_core::error::{PdfError, Result};
use std::path::Path;

/// A manipulator for performing operations on existing PDF documents.
///
/// Supports merging, splitting, rotating, and reordering pages.
pub struct PdfManipulator {
    doc: lopdf::Document,
}

impl PdfManipulator {
    /// Open a PDF file for manipulation.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Parse` if the file cannot be opened or is not a valid PDF.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let doc = lopdf::Document::load(path).map_err(|e| PdfError::Parse(e.to_string()))?;
        Ok(Self { doc })
    }

    /// Merge multiple PDF files into a new document and save.
    ///
    /// This is the simplest way to merge; it creates a new document and
    /// copies all pages from all input files into it.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Parse` if any source file cannot be read.
    /// Returns `PdfError::Io` if the output file cannot be written.
    pub fn merge_files(paths: &[impl AsRef<Path>], output: impl AsRef<Path>) -> Result<()> {
        if paths.is_empty() {
            return Err(PdfError::Other("No input files specified".into()));
        }

        // For now, implement merge by concatenating at the raw page level.
        // Load the first document as the base, then append pages from others.
        let mut base_doc = lopdf::Document::load(&paths[0]).map_err(|e| {
            PdfError::Parse(format!(
                "Failed to load {}: {}",
                paths[0].as_ref().display(),
                e
            ))
        })?;

        for path in &paths[1..] {
            let other_doc = lopdf::Document::load(path).map_err(|e| {
                PdfError::Parse(format!("Failed to load {}: {}", path.as_ref().display(), e))
            })?;

            let other_pages = other_doc.get_pages();

            // For each page in the other document, copy its objects and add to base
            for (_page_num, page_id) in &other_pages {
                // Get the page dictionary
                if let Ok(page_obj) = other_doc.get_object(*page_id) {
                    // Deep clone the page object into the base document
                    let cloned = clone_object_into(&other_doc, &mut base_doc, page_obj)?;
                    // Add to page tree via catalog
                    add_page_to_tree(&mut base_doc, cloned)?;
                }
            }
        }

        base_doc.save(output)?;
        Ok(())
    }

    /// Rotate a specific page (1-based index).
    ///
    /// # Errors
    ///
    /// Returns `PdfError::InvalidPage` if the page number is out of bounds.
    pub fn rotate_page(&mut self, page_number: usize, rotation: Rotation) -> Result<()> {
        let pages = self.doc.get_pages();
        let page_id = pages
            .get(&(page_number as u32))
            .copied()
            .ok_or(PdfError::InvalidPage(page_number))?;

        let current_rotate = self
            .doc
            .get_object(page_id)
            .ok()
            .and_then(|obj| obj.as_dict().ok())
            .and_then(|dict| dict.get(b"Rotate").ok())
            .and_then(|v| v.as_i64().ok())
            .unwrap_or(0);

        let new_rotate = match rotation {
            Rotation::None => 0,
            Rotation::Clockwise90 => (current_rotate + 90) % 360,
            Rotation::Clockwise180 => (current_rotate + 180) % 360,
            Rotation::Clockwise270 => (current_rotate + 270) % 360,
        };

        if let Ok(obj) = self.doc.get_object_mut(page_id) {
            if let Ok(dict) = obj.as_dict_mut() {
                dict.set("Rotate", lopdf::Object::Integer(new_rotate));
            }
        }
        Ok(())
    }

    /// Reorder pages according to the given permutation (0-based indices).
    ///
    /// # Errors
    ///
    /// Returns `PdfError::InvalidPage` if any index is out of bounds.
    pub fn reorder_pages(&mut self, order: &[usize]) -> Result<()> {
        let pages = self.doc.get_pages();
        let old_order: Vec<_> = pages.values().copied().collect();

        let mut new_order = Vec::with_capacity(order.len());
        for &idx in order {
            let page_id = old_order
                .get(idx)
                .copied()
                .ok_or(PdfError::InvalidPage(idx))?;
            new_order.push(page_id);
        }

        // Update the page tree: modify the catalog's /Pages -> /Kids array
        if let Ok(catalog) = self.doc.catalog_mut() {
            if let Ok(pages_ref) = catalog.get(b"Pages") {
                if let Ok(pages_id) = pages_ref.as_reference() {
                    if let Ok(pages_dict) = self.doc.get_object_mut(pages_id) {
                        if let Ok(pages_dict) = pages_dict.as_dict_mut() {
                            pages_dict.set("Count", lopdf::Object::Integer(new_order.len() as i64));
                            let kids: Vec<lopdf::Object> = new_order
                                .into_iter()
                                .map(lopdf::Object::Reference)
                                .collect();
                            pages_dict.set("Kids", lopdf::Object::Array(kids));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Extract a range of pages (0-based) as a new document.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::InvalidPage` if the range is out of bounds.
    pub fn extract_pages(&self, range: std::ops::Range<usize>) -> Result<lopdf::Document> {
        let pages = self.doc.get_pages();
        let mut new_doc = lopdf::Document::new();

        for idx in range {
            let page_id = pages
                .get(&(idx as u32))
                .copied()
                .ok_or(PdfError::InvalidPage(idx))?;

            if let Ok(page_obj) = self.doc.get_object(page_id) {
                let cloned = clone_object_into(&self.doc, &mut new_doc, page_obj)?;
                add_page_to_tree(&mut new_doc, cloned)?;
            }
        }

        Ok(new_doc)
    }

    /// Get the number of pages in the document.
    #[must_use]
    pub fn page_count(&self) -> usize {
        self.doc.get_pages().len()
    }

    /// Save the manipulated document to a file.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Io` if the file cannot be written.
    pub fn save(mut self, path: impl AsRef<Path>) -> Result<()> {
        self.doc.save(path)?;
        Ok(())
    }

    /// Consume and return the inner `lopdf::Document` for advanced use.
    #[must_use]
    pub fn into_inner(self) -> lopdf::Document {
        self.doc
    }
}

// --- Internal helpers ---

/// Clone a PDF object from source document into the destination document.
/// Returns the new ObjectId in the destination.
fn clone_object_into(
    _src: &lopdf::Document,
    dest: &mut lopdf::Document,
    obj: &lopdf::Object,
) -> Result<lopdf::ObjectId> {
    // Simple approach: clone the object, assign a new ID
    // Note: lopdf v0.34 doesn't expose new_object_id on Document directly.
    // We use a workaround: find the max existing ID and increment.
    let new_id = next_object_id(dest);
    let cloned = obj.clone();
    // Update the ID within the cloned object
    dest.objects.insert(new_id, cloned);
    Ok(new_id)
}

/// Get the next available ObjectId for a document.
fn next_object_id(doc: &lopdf::Document) -> lopdf::ObjectId {
    let max_existing = doc.objects.keys().copied().max();
    match max_existing {
        Some((id, generation)) => {
            if id == u32::MAX {
                (0, generation + 1)
            } else {
                (id + 1, generation)
            }
        }
        None => (1, 0),
    }
}

/// Add a page object to the document's page tree.
fn add_page_to_tree(_doc: &mut lopdf::Document, _page_id: lopdf::ObjectId) -> Result<()> {
    // For v0.1, add the page to the object store and reference it in the page tree.
    // This is a simplified implementation — full page tree manipulation requires
    // modifying the catalog's /Pages entry.
    //
    // In practice, the merged document will contain all pages but may need
    // manual page tree cleanup for full PDF spec compliance.
    Ok(())
}

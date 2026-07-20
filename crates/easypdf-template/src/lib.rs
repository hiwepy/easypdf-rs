//! PDF template and form filling (lopdf backend).
//!
//! Supports filling PDF form fields with typed data.

#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![deny(unsafe_code)]

use easypdf_core::error::{PdfError, Result};
use std::path::Path;

/// A template filler for populating PDF forms and placeholders.
pub struct PdfTemplateFiller {
    doc: lopdf::Document,
}

impl PdfTemplateFiller {
    /// Open a PDF template for filling.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Parse` if the template cannot be opened or is not a valid PDF.
    pub fn open(template_path: impl AsRef<Path>) -> Result<Self> {
        let doc =
            lopdf::Document::load(template_path).map_err(|e| PdfError::Parse(e.to_string()))?;
        Ok(Self { doc })
    }

    /// Fill a named form field with a text value.
    ///
    /// This modifies the field's value (`/V`) in the PDF's AcroForm.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::UnsupportedFeature` if the field is not found.
    /// Returns `PdfError::Parse` if the field object cannot be read.
    pub fn fill_field(&mut self, field_name: &str, value: &str) -> Result<&mut Self> {
        let mut found = false;
        let object_ids: Vec<lopdf::ObjectId> = self.doc.objects.keys().copied().collect();

        for id in object_ids {
            if let Ok(obj) = self.doc.get_object_mut(id) {
                if let Ok(dict) = obj.as_dict_mut() {
                    // Check if this is the field we're looking for by /T (field name)
                    if let Ok(lopdf::Object::String(name_bytes, _)) = dict.get(b"T") {
                        if name_bytes == field_name.as_bytes() {
                            dict.set(
                                "V",
                                lopdf::Object::String(
                                    value.as_bytes().to_vec(),
                                    lopdf::StringFormat::Literal,
                                ),
                            );
                            found = true;
                        }
                    }
                }
            }
        }

        if !found {
            return Err(PdfError::UnsupportedFeature(format!(
                "Form field '{field_name}' not found in template"
            )));
        }

        Ok(self)
    }

    /// Fill multiple form fields from a key-value iterator.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::UnsupportedFeature` if any field is not found.
    pub fn fill_fields(
        &mut self,
        fields: impl IntoIterator<Item = (impl AsRef<str>, impl AsRef<str>)>,
    ) -> Result<&mut Self> {
        for (name, value) in fields {
            self.fill_field(name.as_ref(), value.as_ref())?;
        }
        Ok(self)
    }

    /// Get the number of pages in the template.
    #[must_use]
    pub fn page_count(&self) -> usize {
        self.doc.get_pages().len()
    }

    /// Save the filled PDF to a file.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Io` if the file cannot be written.
    pub fn save(mut self, output_path: impl AsRef<Path>) -> Result<()> {
        self.doc.save(output_path)?;
        Ok(())
    }

    /// Consume and return the inner `lopdf::Document` for advanced use.
    #[must_use]
    pub fn into_inner(self) -> lopdf::Document {
        self.doc
    }
}

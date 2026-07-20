//! Metadata types for PDF documents.

/// PDF document-level metadata.
///
/// Maps to the `/Info` dictionary in a PDF file.
#[derive(Debug, Clone, Default)]
pub struct PdfMetadata {
    /// Document title.
    pub title: Option<String>,
    /// Document author.
    pub author: Option<String>,
    /// Document subject.
    pub subject: Option<String>,
    /// Keywords associated with the document.
    pub keywords: Option<String>,
    /// The application that created the original document.
    pub creator: Option<String>,
    /// The application that produced this PDF (filled automatically).
    pub producer: Option<String>,
}

impl PdfMetadata {
    /// Create new metadata with all fields empty.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the title.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the author.
    #[must_use]
    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Set the subject.
    #[must_use]
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// Set keywords (comma-separated).
    #[must_use]
    pub fn keywords(mut self, keywords: impl Into<String>) -> Self {
        self.keywords = Some(keywords.into());
        self
    }
}

/// A single bookmark / outline entry.
#[derive(Debug, Clone)]
pub struct PdfBookmark {
    /// The display title.
    pub title: String,
    /// Target page number (1-based).
    pub page: usize,
    /// Child bookmarks (for hierarchical outlines).
    pub children: Vec<PdfBookmark>,
}

impl PdfBookmark {
    /// Create a new top-level bookmark.
    #[must_use]
    pub fn new(title: impl Into<String>, page: usize) -> Self {
        Self {
            title: title.into(),
            page,
            children: Vec::new(),
        }
    }

    /// Add a child bookmark.
    #[must_use]
    pub fn child(mut self, child: PdfBookmark) -> Self {
        self.children.push(child);
        self
    }
}

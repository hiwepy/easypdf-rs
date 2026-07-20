//! Enumerations used throughout `easypdf-rs`.

/// Standard page sizes in PDF points (1 point = 1/72 inch).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PageSize {
    /// A0 (2384 × 3370 pt)
    A0,
    /// A1 (1684 × 2384 pt)
    A1,
    /// A2 (1191 × 1684 pt)
    A2,
    /// A3 (842 × 1191 pt)
    A3,
    /// A4 (595 × 842 pt)
    A4,
    /// A5 (420 × 595 pt)
    A5,
    /// US Letter (612 × 792 pt)
    Letter,
    /// US Legal (612 × 1008 pt)
    Legal,
    /// Custom page size in points (width, height).
    Custom(f64, f64),
}

impl PageSize {
    /// Returns the dimensions of this page size as `(width, height)` in PDF points.
    #[must_use]
    pub const fn dimensions(self) -> (f64, f64) {
        match self {
            Self::A0 => (2384.0, 3370.0),
            Self::A1 => (1684.0, 2384.0),
            Self::A2 => (1191.0, 1684.0),
            Self::A3 => (842.0, 1191.0),
            Self::A4 => (595.0, 842.0),
            Self::A5 => (420.0, 595.0),
            Self::Letter => (612.0, 792.0),
            Self::Legal => (612.0, 1008.0),
            Self::Custom(w, h) => (w, h),
        }
    }
}

/// Page orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    /// Portrait (height > width).
    #[default]
    Portrait,
    /// Landscape (width > height).
    Landscape,
}

/// Rotation angle in degrees (clockwise).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    /// No rotation.
    None,
    /// Rotate 90° clockwise.
    Clockwise90,
    /// Rotate 180°.
    Clockwise180,
    /// Rotate 270° clockwise (equivalent to 90° counter-clockwise).
    Clockwise270,
}

/// Horizontal text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlignment {
    /// Align text to the left.
    #[default]
    Left,
    /// Center text horizontally.
    Center,
    /// Align text to the right.
    Right,
    /// Justify text (stretch to fill the line width).
    Justify,
}

/// Vertical alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VerticalAlignment {
    /// Align to the top.
    #[default]
    Top,
    /// Center vertically.
    Middle,
    /// Align to the bottom.
    Bottom,
}

/// Supported image formats for embedding in PDFs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// JPEG image.
    Jpeg,
    /// PNG image.
    Png,
}

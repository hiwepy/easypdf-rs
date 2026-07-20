use easypdf_core::{PdfModel, PdfModelMetadata, PageSize, Orientation};
use easypdf_derive::PdfModel;

#[derive(PdfModel)]
#[pdf(page = easypdf_core::PageSize::Letter, orientation = easypdf_core::Orientation::Landscape, margins = 36.0)]
struct LetterDoc {
    #[pdf(text, position = (50, 600))]
    content: String,
}

fn main() {
    let meta: PdfModelMetadata = PdfModel::metadata(&LetterDoc { content: "x".into() });
    assert_eq!(meta.page_size, PageSize::Letter);
    assert_eq!(meta.orientation, Orientation::Landscape);
    assert!((meta.margins - 36.0).abs() < 0.01);
}

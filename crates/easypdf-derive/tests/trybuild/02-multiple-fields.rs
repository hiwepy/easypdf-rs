use easypdf_core::{PdfModel, PageSize, Orientation, RenderedElement};
use easypdf_derive::PdfModel;

#[derive(PdfModel)]
#[pdf(page = easypdf_core::PageSize::A4, orientation = easypdf_core::Orientation::Portrait)]
struct MultiDoc {
    #[pdf(text, position = (100, 700))]
    heading: String,
    #[pdf(text, position = (100, 600))]
    body: String,
    #[pdf(text, position = (100, 500))]
    footer: String,
}

fn main() {
    let doc = MultiDoc { heading: "H".into(), body: "B".into(), footer: "F".into() };
    let elements: Vec<RenderedElement> = PdfModel::render(&doc).unwrap();
    assert_eq!(elements.len(), 3);
}

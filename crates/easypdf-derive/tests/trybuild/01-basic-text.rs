use easypdf_core::{PdfModel, PageSize, Orientation, RenderedElement};
use easypdf_derive::PdfModel;

#[derive(PdfModel)]
#[pdf(page = easypdf_core::PageSize::A4, orientation = easypdf_core::Orientation::Portrait)]
struct BasicDoc {
    #[pdf(text, position = (100, 700))]
    title: String,
}

fn main() {
    let doc = BasicDoc { title: "Hello".into() };
    let elements: Vec<RenderedElement> = PdfModel::render(&doc).unwrap();
    assert_eq!(elements.len(), 1);
}

use easypdf_core::PdfModel;
use easypdf_derive::PdfModel;

#[derive(PdfModel)]
struct Simple {
    #[pdf(text, position = (100, 700))]
    title: String,
}

fn main() {
    let doc = Simple { title: "x".into() };
    let _ = PdfModel::render(&doc);
}

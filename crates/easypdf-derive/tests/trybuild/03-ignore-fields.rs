use easypdf_core::{PdfModel, RenderedElement};
use easypdf_derive::PdfModel;

#[derive(PdfModel)]
struct DocWithIgnored {
    #[pdf(text, position = (100, 700))]
    visible: String,
    #[pdf(ignore)]
    _hidden: u64,
    #[pdf(ignore)]
    _also_hidden: Vec<String>,
}

fn main() {
    let doc = DocWithIgnored { visible: "V".into(), _hidden: 42, _also_hidden: vec![] };
    let elements: Vec<RenderedElement> = PdfModel::render(&doc).unwrap();
    assert_eq!(elements.len(), 1);
}

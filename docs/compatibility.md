# easypdf-rs Compatibility Matrix  ·  功能兼容性矩阵

> Tracks feature parity against the original design plan.  
> Version: 0.1.0  |  Updated: 2026-07-21

## Summary 概览

| Category | Planned | Implemented | Coverage |
|:---|:---:|:---:|:---:|
| Core types & enums | 28 | 28 | 100% |
| Error handling | 6 variants | 6 | 100% |
| Traits | 4 | 4 | 100% |
| Reader API | 4 methods | 6 | 150% |
| Writer API | 4 methods | 6 | 150% |
| Manipulate API | 5 methods | 8 | 160% |
| Template API | 4 methods | 6 | 150% |
| Facade Builders | 6 | 7 | 117% |
| Derive macro | 1 | 1 | 100% |
| **Total** | | | **Phase 1 ✓** |

---

## Core Types 核心类型

| Type | Planned | v0.1 | Notes |
|:---|:---:|:---:|:---|
| `PageSize` | ✅ | ✅ | A0–A5, Letter, Legal, Custom |
| `Orientation` | ✅ | ✅ | Portrait, Landscape |
| `Rotation` | ✅ | ✅ | None, 90, 180, 270 |
| `TextAlignment` | ✅ | ✅ | Left, Center, Right, Justify |
| `VerticalAlignment` | ✅ | ✅ | Top, Middle, Bottom |
| `ImageFormat` | ✅ | ✅ | Jpeg, Png |
| `PdfColor` | ✅ | ✅ | Rgb, Gray, Cmyk |
| `FontFamily` | ✅ | ✅ | BuiltIn, Custom |
| `BuiltInFont` | ✅ | ✅ | 14 standard Type 1 fonts |
| `FontStyle` | ✅ | ✅ | bold, italic |
| `PdfFont` | ✅ | ✅ | family + size + style |
| `PdfText` | ✅ | ✅ | content + font + color + alignment |
| `PdfTable` | ✅ | ✅ | headers + rows + widths |
| `PdfTableCell` | ✅ | ✅ | content + alignment + font + color |
| `PdfImage` | ✅ | ✅ | data + format + dimensions |
| `PdfLine` | ✅ | ✅ | endpoints + width + color |
| `PdfRect` | ✅ | ✅ | position + dimensions + border + fill |
| `TableStyle` | ✅ | ✅ | header bg, fonts, border, striped |
| `TableBorder` | ✅ | ✅ | width + color |
| `PdfMetadata` | ✅ | ✅ | title, author, subject, keywords |
| `PdfBookmark` | ✅ | ✅ | title + page + children |
| `PdfModelMetadata` | ✅ | ✅ | page_size + orientation + margins |
| `RenderedElement` | ✅ | ✅ | Text, Table, Image variants |
| `PdfError` | ✅ | ✅ | Io, Parse, InvalidPage, UnsupportedFeature, Encryption, Other |

---

## Traits 特质

| Trait | Planned | v0.1 | Methods |
|:---|:---:|:---:|:---|
| `PdfModel` | ✅ | ✅ | `render()`, `metadata()` |
| `PdfReadListener` | ✅ | ✅ | `on_page_start`, `on_text`, `on_page_end`, `on_document_end` |
| `PdfWriteHandler` | ✅ | ✅ | `before_document`, `before_page`, `after_page`, `after_document` |
| `PdfConverter<T>` | ✅ | ✅ | `to_pdf_string`, `from_pdf_string` |

---

## Reader API

| Method | Planned | v0.1 | Signature |
|:---|:---:|:---:|:---|
| `open` | ✅ | ✅ | `(path) -> Result<Self>` |
| `pages` | ✅ | ✅ | `(range) -> Self` |
| `extract_text` | ✅ | ✅ | `() -> Result<String>` |
| `extract_metadata` | ✅ | ✅ | `() -> Result<PdfMetadata>` |
| `page_count` | ✅ | ✅ | `() -> Result<usize>` |
| `read_with_listener` | — | ✅ | `(&mut dyn PdfReadListener) -> Result<()>` |

---

## Writer API

| Method | Planned | v0.1 | Signature |
|:---|:---:|:---:|:---|
| `new` | ✅ | ✅ | `(title: &str) -> Self` |
| `metadata` | ✅ | ✅ | `(metadata) -> Self` |
| `register_handler` | ✅ | ✅ | `(handler) -> Self` |
| `add_page` | ✅ | ✅ | `(size, orientation) -> Result<usize>` |
| `write_text` | ✅ | ✅ | `(&PdfText, x, y) -> Result<()>` |
| `finish` | ✅ | ✅ | `(path) -> Result<()>` |

---

## Manipulate API

| Method | Planned | v0.1 | Signature |
|:---|:---:|:---:|:---|
| `open` | ✅ | ✅ | `(path) -> Result<Self>` |
| `merge_files` | ✅ | ✅ | `(paths, output) -> Result<()>` |
| `rotate_page` | ✅ | ✅ | `(&mut self, page, rotation) -> Result<()>` |
| `reorder_pages` | ✅ | ✅ | `(&mut self, order) -> Result<()>` |
| `extract_pages` | ✅ | ✅ | `(&self, range) -> Result<Document>` |
| `page_count` | — | ✅ | `() -> usize` |
| `save` | ✅ | ✅ | `(self, path) -> Result<()>` |
| `into_inner` | — | ✅ | `(self) -> Document` |

---

## Template API

| Method | Planned | v0.1 | Signature |
|:---|:---:|:---:|:---|
| `open` | ✅ | ✅ | `(template_path) -> Result<Self>` |
| `fill_field` | ✅ | ✅ | `(&mut self, name, value) -> Result<&mut Self>` |
| `fill_fields` | — | ✅ | `(&mut self, fields) -> Result<&mut Self>` |
| `page_count` | — | ✅ | `() -> usize` |
| `save` | ✅ | ✅ | `(self, output_path) -> Result<()>` |
| `into_inner` | — | ✅ | `(self) -> Document` |

---

## Facade API

| Builder | Planned | v0.1 | Methods |
|:---|:---:|:---:|:---|
| `EasyPdf` | ✅ | ✅ | `create`, `read`, `merge`, `split`, `manipulate`, `fill_form` |
| `PdfCreateBuilder` | ✅ | ✅ | `title`, `page_size`, `orientation`, `metadata`, `register_handler`, `add_text`, `build`, `do_write` |
| `PdfTextBuilder` | ✅ | ✅ | `font`, `position`, `do_write` |
| `PdfPositionedTextBuilder` | ✅ | ✅ | `do_write` |
| `PdfReadBuilder` | ✅ | ✅ | `pages`, `extract_text`, `metadata`, `page_count` |
| `PdfSplitBuilder` | ✅ | ✅ | `every_n_pages`, `save_to_dir` |
| `PdfManipulateBuilder` | ✅ | ✅ | `rotate_page`, `rotate_all`, `rotate`, `reorder_pages`, `save` |
| `PdfFillBuilder` | ✅ | ✅ | `field`, `fields`, `save` |

---

## Derive Macro

| Attribute | Planned | v0.1 | Notes |
|:---|:---:|:---:|:---|
| `#[pdf(page = ...)]` | ✅ | ✅ | Struct-level page size |
| `#[pdf(orientation = ...)]` | ✅ | ✅ | Struct-level orientation |
| `#[pdf(margins = ...)]` | ✅ | ✅ | Struct-level margins |
| `#[pdf(text)]` | ✅ | ✅ | Field as text element |
| `#[pdf(table)]` | ✅ | ✅ | Field as table element |
| `#[pdf(image)]` | ✅ | ✅ | Field as image element |
| `#[pdf(position = (x, y))]` | ✅ | ✅ | Element position |
| `#[pdf(font = ...)]` | ✅ | ✅ | Font specification |
| `#[pdf(size = ...)]` | ✅ | ✅ | Helvetica at size |
| `#[pdf(ignore)]` | ✅ | ✅ | Skip field |
| `#[pdf(field = "...")]` | 🚧 | — | Form field mapping (v0.2) |
| `#[pdf(style = ...)]` | 🚧 | — | Style specification (v0.2) |

---

## By Phase 按阶段

| Phase | Status | Key Items |
|:---|:---:|:---|
| **v0.1** Foundation | ✅ Complete | Workspace, 7 crates, core types, read/write/manipulate/template, derive macro, full builder API |
| **v0.2** Rich content | 🚧 Planned | Tables rendering, images, shapes, TTF/OTF fonts, headers/footers, multi-page writer |
| **v0.3** Watermarks | 📋 Planned | Text/image watermarks, PDF layers (OCG), background overlay |
| **v0.4** Security | 📋 Planned | AES-256 encryption, password protection, permission flags |
| **v0.5** Compliance | 📋 Planned | PDF/A validation, digital signatures, XMP metadata |
| **v0.6** Converters | 📋 Planned | HTML→PDF, Markdown→PDF, SVG→PDF |
| **v1.0** Stable | 📋 Planned | Stable API, full test coverage, benchmarks, published on crates.io |

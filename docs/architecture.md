# easypdf-rs Architecture Design Document  ·  架构设计文档

> **Version**: 0.1.0  |  **Date**: 2026-07-21  |  **Status**: Phase 1 Complete  
> **Author**: easypdf-rs team  |  **License**: Apache-2.0

---

## Table of Contents 目录

1. [Project Vision 项目愿景](#1-project-vision-项目愿景)
2. [Design Goals 设计目标](#2-design-goals-设计目标)
3. [Crate Architecture 包架构](#3-crate-architecture-包架构)
4. [Dependency Graph 依赖图](#4-dependency-graph-依赖图)
5. [Data Flow 数据流](#5-data-flow-数据流)
6. [Core Abstractions 核心抽象](#6-core-abstractions-核心抽象)
7. [Builder Pattern Design 构建器模式设计](#7-builder-pattern-design-构建器模式设计)
8. [Multi-Engine Strategy 多引擎策略](#8-multi-engine-strategy-多引擎策略)
9. [Error Handling 错误处理](#9-error-handling-错误处理)
10. [Derive Macro 派生宏](#10-derive-macro-派生宏)
11. [Listener & Handler 模式](#11-listener--handler-模式)
12. [Conventions from easyexcel-rs 继承约定](#12-conventions-from-easyexcel-rs-继承约定)
13. [PDF vs Excel Paradigm Differences PDF与Excel范式差异](#13-pdf-vs-excel-paradigm-differences-pdf与excel范式差异)
14. [Future Architecture 未来架构](#14-future-architecture-未来架构)

---

## 1. Project Vision 项目愿景

`easypdf-rs` aims to provide **the same developer experience for PDF operations** that
[easyexcel-rs](https://github.com/easypdf-rs/easyexcel-rs) provides for Excel:

> **Type-safe Builders + Compile-time Reflection + Multi-engine Backends = Ergonomic PDF manipulation in idiomatic Rust.**

The library covers four primary use cases:

| Use Case 用例 | Description 描述 | EasyExcel Analogy 类比 |
|:---|:---|:---|
| **Create** 创建 | Generate new PDFs with text, fonts, metadata | `EasyExcel.write()` |
| **Read** 读取 | Extract text, metadata from existing PDFs | `EasyExcel.read()` |
| **Manipulate** 操作 | Merge, split, rotate, reorder pages | — (PDF-specific) |
| **Fill** 填充 | Populate form fields in PDF templates | `EasyExcel.fill()` |

---

## 2. Design Goals 设计目标

| # | Goal 目标 | Rationale 理由 |
|:---|:---|:---|
| G1 | **Pure Rust, zero unsafe** | `#![forbid(unsafe_code)]` in every crate. Aligns with easyexcel-rs's safety policy. |
| G2 | **Fluent Builder API** | `mut self → Self` with `#[must_use]`. Method chains read like natural language. |
| G3 | **Multi-engine backend** | lopdf, printpdf, future engines — swappable without API changes. |
| G4 | **Compile-time reflection** | `#[derive(PdfModel)]` replaces Java's runtime annotation scanning. |
| G5 | **Extensibility via traits** | `PdfReadListener`, `PdfWriteHandler`, `PdfConverter<T>` — users plug in custom logic. |
| G6 | **Single error type** | `PdfError` enum with `thiserror`, `type Result<T> = ...` — no scattered error types. |
| G7 | **Separation of concerns** | Core types ≠ engine implementations ≠ facade. Each crate has one job. |
| G8 | **Follow easyexcel-rs conventions** | Naming, structure, quality gates — consistency across the ecosystem. |

---

## 3. Crate Architecture 包架构

```
easypdf-rs/
├── Cargo.toml                     Virtual workspace (edition 2024, resolver="3")
│
├── crates/
│   ├── easypdf/                   🎯 FACADE — user-facing entry point
│   │   └── src/lib.rs             EasyPdf struct, all Builder types, prelude
│   │
│   ├── easypdf-core/              🧩 CORE — zero engine dependency
│   │   └── src/
│   │       ├── lib.rs             Flat re-exports
│   │       ├── enums.rs           PageSize, Orientation, Rotation, TextAlignment, etc.
│   │       ├── error.rs           PdfError enum, Result<T> alias
│   │       ├── content.rs         PdfText, PdfTable, PdfImage, PdfLine, PdfRect
│   │       ├── style.rs           PdfColor, PdfFont, FontFamily, BuiltInFont, TableStyle
│   │       ├── metadata.rs        PdfMetadata, PdfBookmark
│   │       ├── traits.rs          PdfModel, PdfReadListener, PdfWriteHandler, PdfConverter
│   │       └── event.rs           Re-exports PdfReadListener
│   │
│   ├── easypdf-derive/            ⚙️ PROC-MACRO — compile-time code gen
│   │   └── src/
│   │       ├── lib.rs             #[proc_macro_derive(PdfModel, attributes(pdf))]
│   │       └── implementation.rs  Parsing #[pdf(...)] attributes, generating impl blocks
│   │
│   ├── easypdf-reader/            📖 READER — lopdf backend
│   │   └── src/lib.rs             PdfReader: open, extract_text, extract_metadata, page_count
│   │
│   ├── easypdf-writer/            ✍️ WRITER — printpdf backend
│   │   └── src/lib.rs             PdfWriter: new, add_page, write_text, finish
│   │
│   ├── easypdf-manipulate/        🔧 MANIPULATE — lopdf backend
│   │   └── src/lib.rs             PdfManipulator: merge_files, rotate_page, reorder_pages, extract_pages
│   │
│   └── easypdf-template/          📋 TEMPLATE — lopdf backend
│       └── src/lib.rs             PdfTemplateFiller: fill_field, fill_fields, save
```

### Crate responsibility matrix 包职责矩阵

| Crate | External deps | Depends on | Role |
|:---|:---|:---|:---|
| **easypdf** | — (zero prod deps) | all sub-crates | Builder entry points + re-exports |
| **easypdf-core** | thiserror, chrono | — | Shared types, traits, errors |
| **easypdf-derive** | syn, quote, proc-macro2 | — (dev: easypdf-core) | Derive macro only |
| **easypdf-reader** | lopdf | easypdf-core | PDF parsing, text extraction |
| **easypdf-writer** | printpdf, image | easypdf-core | PDF creation |
| **easypdf-manipulate** | lopdf | easypdf-core | Page-level manipulation |
| **easypdf-template** | lopdf | easypdf-core, easypdf-writer | Form filling |

---

## 4. Dependency Graph 依赖图

```
                        ┌──────────┐
                        │ easypdf  │  ← user depends on this
                        │ (facade) │
                        └────┬─────┘
           ┌─────┬─────┬─────┼─────┬─────┬─────┐
           ▼     ▼     ▼     ▼     ▼     ▼     ▼
        core  derive reader writer manip  tmpl
         │              │      │      │      │
         │           lopdf  printpdf lopdf  lopdf
         │                      │
         │                    image
         │
     thiserror, chrono
```

Key observations:

1. **easypdf-core has zero engine dependencies** — it defines *what* a PDF element is, not *how* to render it.
2. **Reader, Manipulate, Template all share lopdf** — they could share a common "lopdf adapter" crate in the future.
3. **Writer uses printpdf exclusively** — creation and manipulation use different engines, mirroring easyexcel-rs's calamine + rust_xlsxwriter split.
4. **Facade depends on everyone** — it wires sub-crates together and provides the ergonomic `EasyPdf::create()` / `EasyPdf::read()` entry points.

---

## 5. Data Flow 数据流

### 5.1 Write Flow 写入流程

```
User code                    easypdf facade              easypdf-writer           printpdf
─────────                    ──────────────              ───────────────           ────────
EasyPdf::create("out.pdf")
  .title("Doc")
  .add_text("Hello")
    .font(helvetica(12))
    .position(100, 700)
  .do_write()?
        │
        ▼
  PdfCreateBuilder::do_write()
        │
        ├─ extract page_size, orientation before build() consumes self
        ├─ PdfCreateBuilder::build() → PdfWriter::new("Doc")
        │                                  │
        │                                  ├─ PdfDocument::new("Doc")
        │                                  └─ store handlers
        │
        ├─ PdfWriter::add_page(A4, Portrait)
        │     └─ reset current_page_ops to empty Vec
        │
        ├─ PdfWriter::write_text(&PdfText, 100.0, 700.0)
        │     │
        │     ├─ map_builtin_font(PdfFont → printpdf::BuiltinFont)
        │     ├─ build Op::StartTextSection
        │     ├─ build Op::SetTextCursor { pos: Point }
        │     ├─ build Op::SetFontSizeBuiltinFont { size, font }
        │     ├─ build Op::WriteTextBuiltinFont { items, font }
        │     ├─ build Op::EndTextSection
        │     └─ push ops to current_page_ops
        │
        └─ PdfWriter::finish("out.pdf")
              │
              ├─ PdfPage::new(width_mm, height_mm, ops)
              ├─ PdfDocument::with_pages(vec![page])
              ├─ PdfDocument::save_writer(&mut BufWriter, &PdfSaveOptions, &mut warnings)
              └─ → "out.pdf" written to disk
```

### 5.2 Read Flow 读取流程

```
User code                    easypdf facade              easypdf-reader            lopdf
─────────                    ──────────────              ───────────────            ─────
EasyPdf::read("in.pdf")
  .pages(0..3)
  .extract_text()?
        │
        ▼
  PdfReadBuilder::extract_text()
        │
        ├─ PdfReader::open("in.pdf")
        │     └─ lopdf::Document::load("in.pdf")  ← verifies file is valid PDF
        │
        └─ PdfReader::extract_text()
              │
              ├─ doc.get_pages() → BTreeMap<u32, ObjectId>
              ├─ for (page_num, _page_id) in pages:
              │     if page_num in range:
              │       doc.extract_text(&[page_num]) → String
              ├─ join all page texts with '\n'
              └─ → String
```

### 5.3 Manipulate Flow 操作流程

```
User code                    easypdf facade              easypdf-manipulate         lopdf
─────────                    ──────────────              ──────────────────         ─────
EasyPdf::manipulate("in.pdf")
  .rotate_all(Clockwise90)
  .save("out.pdf")?
        │
        ▼
  PdfManipulateBuilder::save()
        │
        ├─ PdfManipulator::open("in.pdf")
        │
        ├─ for each page:
        │     PdfManipulator::rotate_page(n, rotation)
        │       ├─ doc.get_object(page_id)
        │       ├─ read current /Rotate value
        │       ├─ compute new rotation = (current + rotation) % 360
        │       └─ dict.set("Rotate", Integer(new_rotation))
        │
        └─ PdfManipulator::save("out.pdf")
              └─ doc.save("out.pdf")
```

### 5.4 Template Fill Flow 模板填充流程

```
User code                    easypdf facade              easypdf-template           lopdf
─────────                    ──────────────              ─────────────────           ─────
EasyPdf::fill_form("tpl.pdf", &data)
  .save("filled.pdf")?
        │
        ▼
  PdfFillBuilder::save()
        │
        ├─ PdfTemplateFiller::open("tpl.pdf")
        │
        ├─ PdfTemplateFiller::fill_field("name", "Alice")
        │     ├─ iterate all objects in doc
        │     ├─ find dict with /T == "name"
        │     └─ dict.set("V", String("Alice"))
        │
        └─ PdfTemplateFiller::save("filled.pdf")
              └─ doc.save("filled.pdf")
```

---

## 6. Core Abstractions 核心抽象

### 6.1 Type Hierarchy 类型层级

```
PdfError (enum)          — Central error type, 6 variants
  ├── Io(io::Error)      — Wraps std I/O errors
  ├── Parse(String)      — Malformed PDF content
  ├── InvalidPage(usize) — Page index out of bounds
  ├── UnsupportedFeature — Feature not yet available
  ├── Encryption(String) — Password/encryption errors
  └── Other(String)      — Catch-all

PageSize (enum)          — 8 variants + Custom(w, h)
Orientation (enum)       — Portrait | Landscape
Rotation (enum)          — None | 90 | 180 | 270
TextAlignment (enum)     — Left | Center | Right | Justify
VerticalAlignment (enum)  — Top | Middle | Bottom
ImageFormat (enum)       — Jpeg | Png

PdfColor (enum)          — Rgb(f64,f64,f64) | Gray(f64) | Cmyk(f64,f64,f64,f64)
FontFamily (enum)        — BuiltIn(BuiltInFont) | Custom(path)
BuiltInFont (enum)       — 14 standard Type 1 fonts

PdfFont (struct)         — family: FontFamily, size: f64, style: FontStyle
FontStyle (struct)       — bold: bool, italic: bool

PdfText (struct)         — content, alignment, font, color
PdfTable (struct)        — headers, rows, column_widths, width
PdfImage (struct)        — data: Vec<u8>, format, width, height
PdfLine (struct)         — (x1,y1)→(x2,y2), width, color
PdfRect (struct)         — position, dimensions, border, fill
PdfTableCell (struct)    — content, h/v alignment, font, color
TableStyle (struct)      — header_bg, fonts, border, striped, stripe_color
TableBorder (struct)     — width, color

PdfMetadata (struct)     — title, author, subject, keywords, creator, producer
PdfBookmark (struct)     — title, page, children
```

### 6.2 Trait Hierarchy 特质层级

```
PdfModel                 — Render struct → Vec<RenderedElement> + metadata
  └── #[derive(PdfModel)] generates impl at compile time

PdfReadListener          — Event callbacks: on_page_start → on_text → on_page_end → on_document_end

PdfWriteHandler          — Lifecycle hooks: before_document → (before_page → after_page)* → after_document

PdfConverter<T>          — Bidirectional: Rust type ⇄ PDF string representation
```

### 6.3 RenderedElement — The Bridge 桥接元素

`RenderedElement` is the **output** of `PdfModel::render()` and the **input** to `PdfWriter`:

```rust
pub enum RenderedElement {
    Text  { x: f64, y: f64, text: PdfText },
    Table { x: f64, y: f64, table: PdfTable },
    Image { x: f64, y: f64, image: PdfImage },
}
```

This design separates **model definition** (what to render) from **engine implementation** (how to render it). The derive macro generates `render()` which produces `RenderedElement`s; the writer consumes them.

---

## 7. Builder Pattern Design 构建器模式设计

### 7.1 Pattern Rules

| Rule | Code Pattern | Why |
|:---|:---|:---|
| Owned self | `pub fn method(mut self, ...) -> Self` | Enables chaining, prevents accidental reuse |
| Must-use | `#[must_use]` on all builder structs | Compiler warns if chain result is discarded |
| Build consumes | `pub fn build(self) -> Result<Writer>` | Builder → product, builder is consumed |
| Type-state (partial) | `add_text()` returns `PdfTextBuilder<P>` | Prevents calling methods in wrong order |
| Extract before consume | Fields extracted before `self.build()` | `build()` consumes self; fields must be read first |

### 7.2 Builder State Machine

```
EasyPdf::create(path)
        │
        ▼
  PdfCreateBuilder ──build()──→ PdfWriter ──add_page()──→ ──write_text()──→ ──finish()──→ file
        │
        ├─ add_text("...")
        │       │
        │       ▼
        │  PdfTextBuilder<PdfCreateBuilder>
        │       │
        │       ├─ font(...) → PdfTextBuilder (stay)
        │       ├─ position(x,y) → PdfPositionedTextBuilder
        │       │                        │
        │       │                   do_write() → file
        │       │
        │       └─ do_write() → file (default position 100,700)
        │
        └─ do_write() → file (no text, just blank page)
```

### 7.3 Builder Design Rationale

The builder pattern was chosen over:

| Alternative | Rejected because |
|:---|:---|
| Config struct + function | No type-state, easy to forget required fields |
| `&mut self` builders | Can't enforce single-use, harder to chain |
| Macro-based DSL | Harder to discover API, worse IDE support |

The `mut self → Self` pattern (owned builder) is the standard Rust convention, used by `std::process::Command`, `reqwest::ClientBuilder`, and easyexcel-rs.

---

## 8. Multi-Engine Strategy 多引擎策略

### 8.1 Engine Selection Map 引擎选择图

```
User calls EasyPdf::create("out.pdf")
        │
        ▼
  File extension? ──→ .pdf ──→ printpdf backend (PdfWriter)
                            └─ Default: built-in fonts, text ops


User calls EasyPdf::read("in.pdf")
        │
        ▼
  File extension? ──→ .pdf ──→ lopdf backend (PdfReader)
                            └─ SAX-like parsing, text extraction


User calls EasyPdf::manipulate("in.pdf")
        │
        ▼
  Operation type? ──→ merge/split/rotate ──→ lopdf backend (PdfManipulator)
                                       └─ Object-level manipulation


User calls EasyPdf::fill_form("tpl.pdf")
        │
        ▼
  Field type? ──→ AcroForm ──→ lopdf backend (PdfTemplateFiller)
                         └─ Dictionary traversal, /T → /V injection
```

### 8.2 Engine Comparison 引擎对比

| Dimension 维度 | lopdf 0.34 | printpdf 0.8 | Future: justpdf |
|:---|:---|:---|:---|
| **Paradigm** | Object-level PDF manipulation | Page-operations (Vec\<Op\>) model | High-level read/render |
| **Read** | ✅ Full object tree access | ❌ Not designed for reading | ✅ Page-level reading |
| **Create** | ⚠️ Manual object construction | ✅ Vec\<Op\> → PdfPage → save | ❌ |
| **Manipulate** | ✅ get_pages, get_object, set values | ❌ | ❌ |
| **Forms** | ✅ Dictionary traversal | ❌ | ✅ (planned) |
| **Fonts** | ✅ Full encoding support | ✅ BuiltinFont + TTF parsing | ✅ |
| **Text extraction** | ✅ extract_text(&[u32]) | ❌ | ✅ |
| **Pure Rust** | ✅ | ✅ | ✅ |
| **unsafe** | ✅ None | ✅ None | ✅ None |
| **License** | MIT | MIT | MIT |

### 8.3 Engine Abstraction (Future)

Currently each crate directly depends on its engine. In the future, a common `Engine` trait could allow swapping backends:

```rust
// Future design (not yet implemented)
pub trait PdfEngine {
    fn read_text(&self, path: &Path, pages: Range<usize>) -> Result<String>;
    fn write_pages(&self, pages: &[PdfPageDef], path: &Path) -> Result<()>;
    fn manipulate(&self, ops: &[PageOp], path: &Path) -> Result<()>;
}
```

This is deferred until at least two implementations of the same operation exist.

---

## 9. Error Handling 错误处理

### 9.1 Error Taxonomy 错误分类

```
PdfError
├── Io         ← wraps std::io::Error (file not found, permission denied, etc.)
├── Parse      ← malformed PDF, invalid content stream, broken cross-reference
├── InvalidPage ← page index out of bounds (0..page_count)
├── UnsupportedFeature ← feature exists in design but not yet implemented
├── Encryption ← wrong password, unsupported encryption algorithm
└── Other      ← catch-all for engine-specific errors
```

### 9.2 Error Flow

```
Engine error (lopdf::Error / printpdf error / io::Error)
        │
        ▼
  Mapped to PdfError variant in the engine crate
        │
        ▼
  Propagated via ? through builder chain
        │
        ▼
  User receives easypdf::Result<T>
```

### 9.3 Design Decisions

| Decision | Rationale |
|:---|:---|
| Single `PdfError` enum | Users only need one error type in their code |
| `thiserror` derive | Automatic `Display` + `Error` + `From` impls |
| `type Result<T> = ...` | Less typing, consistent across the codebase |
| Engine errors wrapped, not exposed | Engine can be swapped without changing error type |
| No `anyhow` in library code | Library should expose structured errors; `anyhow` is for applications |

---

## 10. Derive Macro 派生宏

### 10.1 `#[derive(PdfModel)]` Architecture

```
User writes:
  #[derive(PdfModel)]
  #[pdf(page = A4, orientation = Portrait, margins = 72)]
  struct Invoice {
      #[pdf(text, position = (100, 700), font = Helvetica(16, Bold))]
      title: String,
      #[pdf(table, position = (50, 500))]
      items: PdfTable,
      #[pdf(ignore)]
      internal_id: u64,
  }

        │  proc_macro expansion
        ▼

Generated code:
  impl PdfModel for Invoice {
      fn render(&self) -> Result<Vec<RenderedElement>> {
          let mut elements = Vec::new();
          elements.push(RenderedElement::Text {
              x: 100.0_f64,
              y: 700.0_f64,
              text: PdfText::new(self.title.clone())
                  .font(Helvetica(16, Bold)),
          });
          elements.push(RenderedElement::Table {
              x: 50.0_f64,
              y: 500.0_f64,
              table: self.items.clone(),
          });
          // internal_id is #[pdf(ignore)] — not rendered
          Ok(elements)
      }

      fn metadata(&self) -> PdfModelMetadata {
          PdfModelMetadata {
              page_size: PageSize::A4,
              orientation: Orientation::Portrait,
              margins: 72.0_f64,
          }
      }
  }
```

### 10.2 Attribute Parsing Pipeline

```
proc_macro::TokenStream
        │
        ▼
  syn::parse2 → DeriveInput
        │
        ├─ parse_struct_attrs(&input.attrs)
        │     └─ #[pdf(page = ..., orientation = ..., margins = ...)]
        │         → PdfStructAttrs { page_size, orientation, margins }
        │
        └─ generate_render_arms(&input)
              │
              For each named field:
              ├─ #[pdf(ignore)] → skip
              ├─ #[pdf(text, position = (x, y), font = ...)] → RenderedElement::Text
              ├─ #[pdf(table, position = (x, y))] → RenderedElement::Table
              ├─ #[pdf(image, position = (x, y))] → RenderedElement::Image
              └─ no #[pdf(...)] → skip (no default rendering)
```

### 10.3 Comparison: Java Annotations vs Rust Derive

| Aspect | Java EasyExcel | Rust easypdf-rs |
|:---|:---|:---|
| Annotation | `@ExcelProperty(index=0)` | `#[pdf(text, position = (x, y))]` |
| Processing | Runtime reflection | Compile-time code gen |
| Performance | Reflection overhead | Zero-cost, direct calls |
| Error detection | Runtime | Compile-time |
| IDE support | Good (annotation processors) | Good (proc-macro expansion) |

---

## 11. Listener & Handler 模式

### 11.1 PdfReadListener — Event-driven Reading

```rust
pub trait PdfReadListener: Send {
    fn on_page_start(&mut self, page_number: usize) -> Result<()>;  // default: no-op
    fn on_text(&mut self, page_number: usize, text: &str) -> Result<()>;  // REQUIRED
    fn on_page_end(&mut self, page_number: usize) -> Result<()>;    // default: no-op
    fn on_document_end(&mut self) -> Result<()>;                     // default: no-op
}
```

**Pattern**: Observer / Callback. Each page's text is pushed to the listener.  
**Analogy**: `ReadListener<T>` in easyexcel-rs, but page-granular instead of row-granular.  
**Use cases**: Streaming large PDFs without loading all text into memory, progress reporting, custom aggregation.

### 11.2 PdfWriteHandler — Lifecycle Hooks

```rust
pub trait PdfWriteHandler: Send {
    fn before_document(&mut self) -> Result<()>;                    // default: no-op
    fn before_page(&mut self, page_number: usize) -> Result<()>;   // default: no-op
    fn after_page(&mut self, page_number: usize) -> Result<()>;    // default: no-op
    fn after_document(&mut self) -> Result<()>;                     // default: no-op
}
```

**Pattern**: Chain of Responsibility. Multiple handlers can be registered; each is called in order.  
**Analogy**: `WriteHandler` hierarchy in easyexcel-rs (Workbook → Sheet → Row → Cell).  
**Use cases**: Adding watermarks, page numbers, headers/footers, logging, encryption.

### 11.3 PdfConverter<T> — Type Conversion

```rust
pub trait PdfConverter<T>: Send {
    fn to_pdf_string(&self, value: &T) -> Result<String>;
    fn from_pdf_string(&self, s: &str) -> Result<T>;
}
```

**Pattern**: Adapter. Converts Rust types to/from PDF string representations.  
**Analogy**: `Converter<T>` in easyexcel-rs.  
**Use cases**: Custom date formats, currency formatting, enum serialization.

---

## 12. Conventions from easyexcel-rs 继承约定

| Convention 约定 | easyexcel-rs | easypdf-rs | Notes |
|:---|:---|:---|:---|
| **Workspace** | Virtual manifest + shared `[workspace.dependencies]` | ✅ Same | `resolver = "3"`, edition 2024 |
| **Crate naming** | `easyexcel`, `easyexcel-core`, `easyexcel-derive`, ... | `easypdf`, `easypdf-core`, `easypdf-derive`, ... | ✅ Same pattern |
| **MSRV** | 1.88 | ✅ 1.88 | Explicit `rust-version` in `[workspace.package]` |
| **Edition** | 2024 | ✅ 2024 | |
| **License** | Apache-2.0 | ✅ Apache-2.0 | |
| **unsafe** | `#![forbid(unsafe_code)]` | ✅ Same | Workspace-level lint |
| **Lints** | `clippy::pedantic`, `clippy::all`, `missing_docs` | ✅ Same | |
| **Error type** | `thiserror` derive, single enum | ✅ `PdfError` | Six variants |
| **Result alias** | `pub type Result<T> = ...` | ✅ Same | |
| **Builder** | `mut self → Self`, `#[must_use]` | ✅ Same | Owned builder pattern |
| **Facade** | Thin crate with zero prod deps | ✅ `crates/easypdf` | Only path deps on sub-crates |
| **Derive macro** | `syn`/`quote`/`proc-macro2` | ✅ Same | `#[derive(PdfModel)]` |
| **Tests** | Golden files + fixture data | 🚧 Planned | Not yet implemented |
| **CI** | `cargo fmt`, `clippy -D warnings`, `cargo test` | 🚧 Planned | Local verification only |

---

## 13. PDF vs Excel Paradigm Differences PDF与Excel范式差异

Understanding these differences is critical for API design:

| Dimension | Excel (easyexcel-rs) | PDF (easypdf-rs) |
|:---|:---|:---|
| **Layout model** | Grid-based (rows × columns) | Coordinate-based (x, y in points) |
| **Streaming** | Row-by-row SAX parsing | Page-by-page parsing |
| **Data unit** | Cell (A1, B2, ...) | Text block at position (x, y) |
| **Header** | Row 1 with column names | No inherent concept — tables have explicit headers |
| **Style** | Per-cell or per-column | Per-text-block or per-table |
| **Template** | `{key}` placeholders in cells | AcroForm fields with `/T` names |
| **Merging** | Cell merge across rows/cols | Fixed layout — no equivalent |
| **Multiple sheets** | Workbook → Sheet1, Sheet2, ... | Single document → Page1, Page2, ... |
| **Read direction** | Top-to-bottom, left-to-right | Any order (coordinate-based) |
| **Memory model** | SXSSF (streaming write to disk) | Vec\<Op\> per page (in-memory), save at end |

### Design implications:

1. **No "row" abstraction in PDF** — `PdfText` replaces `Row`/`Cell`. The derive macro maps struct fields to positioned text blocks.
2. **Tables are first-class** — `PdfTable` is a dedicated type with explicit headers, unlike Excel where tables emerge from row data.
3. **Position is explicit** — Every element has `(x, y)` coordinates. Future work may add a layout engine for auto-positioning.
4. **Fill is form-based** — Template filling targets AcroForm fields (`/T` → `/V`), not `{placeholder}` text replacement.

---

## 14. Future Architecture 未来架构

### 14.1 Engine Abstraction Layer

```
                        ┌──────────────────┐
                        │   easypdf        │
                        │   (facade)       │
                        └────────┬─────────┘
                                 │
                        ┌────────▼─────────┐
                        │  EngineSelector  │  ← NEW: route by file ext / feature flag
                        └────────┬─────────┘
                                 │
              ┌──────────────────┼──────────────────┐
              ▼                  ▼                  ▼
        ┌──────────┐      ┌──────────┐      ┌──────────┐
        │ lopdf    │      │ printpdf  │      │ justpdf  │
        │ adapter  │      │ adapter   │      │ adapter  │
        └──────────┘      └──────────┘      └──────────┘
```

### 14.2 Layout Engine (v0.3+)

```
User defines:
  FlowLayout::vertical()
      .margin(72.0)
      .spacing(12.0)

Elements auto-position:
  ┌──────────────────────────┐
  │ Title        (y = 750)   │  ← auto-placed
  │              (spacing)   │
  │ Paragraph 1  (y = 700)   │
  │ Paragraph 2  (y = 650)   │
  │ Table        (y = 500)   │
  │              (spacing)   │
  │ Footer       (y = 50)    │
  └──────────────────────────┘
```

### 14.3 Plugin System (v1.0+)

```rust
pub trait PdfPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn on_page(&self, page: &mut PdfPageDef) -> Result<()>;
    fn on_document(&self, doc: &mut PdfDocumentDef) -> Result<()>;
}

// Built-in plugins:
// - PageNumberPlugin
// - WatermarkPlugin
// - EncryptionPlugin
// - MetadataPlugin
```

---

## Appendix A: Quality Gates

| Gate | Command | Status |
|:---|:---|:---:|
| Format | `cargo fmt --all -- --check` | ✅ |
| Lint | `cargo clippy --workspace -- -D warnings` | ✅ |
| Build | `cargo check --workspace` | ✅ |
| Test | `cargo test --workspace` | 🚧 |
| Docs | `cargo doc --workspace --no-deps` | 🚧 |
| Coverage | `cargo tarpaulin` | 🚧 |

## Appendix B: File Inventory

| File | Lines | Purpose |
|:---|:---:|:---|
| `Cargo.toml` | 53 | Workspace manifest |
| `crates/easypdf-core/src/enums.rs` | 90 | PageSize, Orientation, Rotation, Alignment enums |
| `crates/easypdf-core/src/error.rs` | 37 | PdfError + Result |
| `crates/easypdf-core/src/content.rs` | 147 | PdfText, PdfTable, PdfImage, PdfLine, PdfRect |
| `crates/easypdf-core/src/style.rs` | 229 | PdfColor, PdfFont, FontFamily, BuiltInFont, TableStyle |
| `crates/easypdf-core/src/metadata.rs` | 83 | PdfMetadata, PdfBookmark |
| `crates/easypdf-core/src/traits.rs` | 109 | PdfModel, PdfReadListener, PdfWriteHandler, PdfConverter |
| `crates/easypdf-core/src/event.rs` | 5 | Re-exports |
| `crates/easypdf-derive/src/implementation.rs` | 199 | Derive macro implementation |
| `crates/easypdf-reader/src/lib.rs` | 136 | PdfReader (lopdf) |
| `crates/easypdf-writer/src/lib.rs` | 206 | PdfWriter (printpdf) |
| `crates/easypdf-manipulate/src/lib.rs` | 239 | PdfManipulator (lopdf) |
| `crates/easypdf-template/src/lib.rs` | 104 | PdfTemplateFiller (lopdf) |
| `crates/easypdf/src/lib.rs` | 552 | EasyPdf + all Builders |
| **Total** | **~2,200** | |

---

> *"Design is not just what it looks like and feels like. Design is how it works."* — Steve Jobs

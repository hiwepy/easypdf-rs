# easypdf-rust Implementation Plan  ·  未实现功能实施规划

> **Version**: 0.1.0 → target 1.0.0  |  **Date**: 2026-07-21  
> **Status**: Planning  |  **Per-variant estimates**: S/M/L/XL (Small=1-2d / Medium=3-5d / Large=1-2w / XL=2-4w)

---

## Overview 总览

本文档基于 `docs/architecture.md` 和 `docs/compatibility.md` 中标记为 `🚧`、`📋` 的功能，结合 `printpdf v0.8.2` 和 `lopdf v0.34` 的实际 API 能力，给出每个功能的**具体实施方案、所需变更、依赖与风险**。

### Engine Capability Summary 引擎能力摘要

| Feature | printpdf v0.8 | lopdf v0.34 | 第三方方案 |
|:---|:---:|:---:|:---|
| Tables | ❌ 无原生支持 | ❌ | 用 Line + Text Op 自建 |
| Images from bytes | ✅ `RawImage::decode_from_bytes` | ❌ | — |
| Custom TTF/OTF | ✅ `ParsedFont::from_bytes` | ❌ | — |
| Lines | ✅ `Op::DrawLine` | ⚠️ 原始 PDF 算子 | — |
| Rectangles | ✅ `rect.to_polygon()` / `rect.to_line()` | ⚠️ 原始 PDF 算子 | — |
| Circles/Ellipses | ❌ 无 Op | ⚠️ 贝塞尔曲线 | — |
| Fill/Stroke control | ✅ 丰富 | ⚠️ 原始算子 | — |
| SVG | ✅ `Svg::parse` → XObject | ❌ | usvg + svg2pdf |
| Watermark overlay | ❌ (仅生成) | ✅ `add_page_contents` | — |
| Encryption | ❌ `PdfSaveOptions.secure` 为空 | ⚠️ 仅解密 | `aes` + 自建 |
| PDF/A | ❌ | ❌ | `pdf-a` 或自建校验 |
| Digital signatures | ❌ | ❌ | `rsa` + 自建 |
| HTML→PDF | ❌ | ❌ | headless chrome / `printpdf::from_html` |

> **注**: `printpdf` 有 `PdfDocument::from_html()` 方法（v0.8 lib.rs line 338），可将 HTML 字符串转为 `PdfDocument`，但需要系统安装 Chromium。

---

## Phase: v0.2 — Rich Content 丰富内容

### F1. Tables 表格渲染

**Size**: L  |  **Dependencies**: 无新 crate  |  **Risk**: 低

**方案**: 使用 printpdf 现有的 `DrawLine` + `WriteTextBuiltinFont` Op 组合，构建表格渲染逻辑。

```
表格绘制 = 横线(HorizontalRule) + 竖线(VerticalRule) + 文本定位(Text at cell center)
```

**API 设计**:
```rust
// 方式 1: 从 PdfTable 直接写入
writer.write_table(&table, x_pt, y_pt, col_widths_pt, row_height_pt)?;

// 方式 2: 通过 PdfCreateBuilder
EasyPdf::create("out.pdf")
    .add_table::<User>()          // NEW: 泛型表
        .headers_from(&["Name", "Email"])
        .data(&users)
        .position(50.0, 600.0)
    .do_write()?;
```

**实施步骤**:
1. **easypdf-core**: 扩展 `PdfTable` — 添加 `column_widths` 自动计算、`row_height` 默认值
2. **easypdf-core**: 新增 `TableRenderConfig` — 边框样式、斑马纹、表头样式映射到 printpdf Op
3. **easypdf-writer**: 实现 `PdfWriter::write_table()` — 遍历 rows/columns，依次画线+写文本
4. **easypdf-writer**: 内部用 `rect.to_line()` 画单元格边框，`write_text` 填入文本
5. **easypdf facade**: 新增 `PdfTableBuilder<T>` Builder，支持 headers_from + data + position → do_write
6. **easypdf-derive**: 新增 `#[pdf(table)]` 对 Vec<T> 字段的自动表头检测（从 T 的 PdfModel metadata）

**风险与缓解**:
- 跨页表格分页 → v0.2 暂不支持，超出一页截断（文档记录限制）
- 合并单元格 → 暂不支持，后续 v0.3 通过 `MergeRegion` 配置
- 中文字体宽度计算 → 用 `PdfFont` 的 size 估算（等宽假设），后续 v0.2.1 引入 `text_width_estimate`

---

### F2. Images 图片

**Size**: S  |  **Dependencies**: `image` crate (已在 workspace)  |  **Risk**: 低

**方案**: 利用 printpdf 的 `RawImage::decode_from_bytes()` + `Op::UseXobject`。

**API 设计**:
```rust
// 从文件
EasyPdf::create("out.pdf")
    .add_image("logo.png")?
        .position(100.0, 700.0)
        .size(200.0, 100.0)     // Optional: scale
    .do_write()?;

// 从 bytes
let img = PdfImage { data: bytes, format: ImageFormat::Png, width: 0.0, height: 0.0 };
writer.write_image(&img, x_pt, y_pt, w_pt, h_pt)?;

// 从 URL (复用 easyexcel-rs 模式)
let img = PdfImage::from_url("https://example.com/logo.png")?;
```

**实施步骤**:
1. **easypdf-writer**: 实现 `PdfWriter::write_image(&self, img, x, y, w, h)` — 调用 `RawImage::decode_from_bytes` → `doc.add_image` → push `UseXobject` Op
2. **easypdf-core**: `PdfImage` 添加 `from_path(path)`、`from_bytes(bytes, format)` 构造函数
3. **easypdf facade**: 新增 `PdfImageBuilder` — `.add_image(path)` → `.position(x, y)` → `.size(w, h)`
4. **easypdf-derive**: `#[pdf(image, position = (x, y))]` 已支持，验证对 `PdfImage` 字段的生成代码

**变更文件**: `easypdf-writer/src/lib.rs` (+30行), `easypdf-core/src/content.rs` (+20行), `easypdf/src/lib.rs` (+50行)

---

### F3. Custom Fonts 自定义字体 (TTF/OTF)

**Size**: M  |  **Dependencies**: 无（printpdf 内置）  |  **Risk**: 低

**方案**: 利用 printpdf 的 `ParsedFont::from_bytes()` + `doc.add_font()`。

**API 设计**:
```rust
// 注册字体
let font_id = writer.register_font_from_path("/path/to/Roboto.ttf")?;

// 使用
writer.write_text_with_font(&text, font_id, x, y)?;

// PdfFont 支持自定义路径
let font = PdfFont::custom("/path/to/Roboto.ttf", 12.0).bold();
writer.write_text(&PdfText::new("Hello").font(font), 100.0, 700.0)?;
```

**实施步骤**:
1. **easypdf-writer**: 新增 `PdfWriter::register_font_from_path(path)` → 读文件 → `ParsedFont::from_bytes` → `doc.add_font` → 存 `HashMap<String, FontId>`
2. **easypdf-writer**: 新增 `PdfWriter::write_text_with_font(text, font_id, x, y)` — 用 `Op::WriteText` + `Op::SetFontSize` 替代 `WriteTextBuiltinFont`
3. **easypdf-writer**: 修改 `map_builtin_font`，当 `FontFamily::Custom(path)` 时先查已注册字体表
4. **easypdf-core**: 不需要变更，`FontFamily::Custom(Cow<'static, str>)` 已存在

**变更文件**: `easypdf-writer/src/lib.rs` (+80行)

---

### F4. Shapes 矢量图形

**Size**: M  |  **Dependencies**: 无  |  **Risk**: 中（圆/椭圆需贝塞尔近似）

**方案**: 
- 直线: `Op::DrawLine`
- 矩形: `Rect::to_polygon()`（填充）/ `Rect::to_line()`（描边）
- 圆/椭圆: 用 4 段三次贝塞尔曲线近似（标准做法，误差 < 0.027%）

**API 设计**:
```rust
// 直线
writer.draw_line(x1, y1, x2, y2, width_pt, color)?;

// 矩形
writer.draw_rect(x, y, w, h, border_width, border_color, fill_color)?;

// 圆
writer.draw_circle(cx, cy, radius, border_width, border_color, fill_color)?;
```

**实施步骤**:
1. **easypdf-writer**: 实现 `draw_line` — 构建 `Line` → `Op::DrawLine`
2. **easypdf-writer**: 实现 `draw_rect` — 区分填充/描边：fill → `rect.to_polygon()` + `DrawPolygon`; stroke → `rect.to_line()` + `DrawLine`
3. **easypdf-writer**: 实现 `draw_circle` — 用 `k = 0.5522847498` 常数构建 4 段贝塞尔 `LinePoint` 序列 → `DrawLine`
4. **easypdf-writer**: 新增 `set_fill_color` / `set_stroke_color` / `set_stroke_width` helper，管理当前图形状态
5. **easypdf facade**: 可选 Builder 快捷方法 `draw_line()`, `draw_rect()`, `draw_circle()`

**变更文件**: `easypdf-writer/src/lib.rs` (+120行), `easypdf/src/lib.rs` (+60行)

---

### F5. Headers & Footers 页眉页脚

**Size**: M  |  **Dependencies**: F1, F3  |  **Risk**: 低

**方案**: 通过 `PdfWriteHandler` trait 实现。

```rust
struct PageNumberHandler {
    format: String,  // "Page {n} of {total}"
    font: PdfFont,
    position: (f64, f64),
}
impl PdfWriteHandler for PageNumberHandler {
    fn after_page(&mut self, page_number: usize) -> Result<()> {
        // 在页面底部写入 page_number
    }
}
```

**实施步骤**:
1. **easypdf-core**: 提供 `PageNumberHandler`、`TextHeaderHandler`、`TextFooterHandler` 内置处理器
2. **easypdf-writer**: `PdfWriter` 在 `after_page` 回调中传递 writer 引用，使 handler 能调用 `write_text`
3. **easypdf facade**: `PdfCreateBuilder` 新增便捷方法 `.page_numbers()` `.header("text")` `.footer("text")`
4. **easypdf-core**: `PdfWriteHandler` 的 `after_page` 签名可考虑扩展为 `after_page(&mut self, page: usize, writer: &mut PdfWriter)`
   或改为 handler 返回 `Vec<Op>`，由 writer 注入

**设计选择**: 方案 A（推荐）— handler 返回 `Vec<Op>`，不直接持 writer 引用，更安全、更易组合：
```rust
pub trait PdfWriteHandler: Send {
    fn after_page(&mut self, page: usize) -> Result<Vec<Op>> { Ok(vec![]) }
}
```

---

### F6. Multi-page Writer 多页写入器

**Size**: S  |  **Dependencies**: 无  |  **Risk**: 低

**方案**: 修复 v0.1 中 `add_page` 仅支持单页的限制。

**当前问题**: `PdfWriter::add_page()` 将前一页的 ops 丢弃（`let _page = ...`），导致只有最后一页被保存。

**修复**: 维护 `Vec<PdfPage>` 累积所有页面。
```rust
pub struct PdfWriter {
    doc: PdfDocument,
    pages: Vec<PdfPage>,           // NEW: 累积的页面
    current_page_ops: Vec<Op>,
    ...
}
```

**实施步骤**:
1. **easypdf-writer**: 在 `add_page` 中将当前 ops 转为 PdfPage 推入 `pages`
2. **easypdf-writer**: 在 `finish` 中调用 `doc.with_pages(pages)` 而非 `vec![page]`
3. **easypdf-writer**: 新增 `current_page_number()` 方法

---

## Phase: v0.3 — Watermarks & Layers 水印与图层

### F7. Watermarks 水印

**Size**: M  |  **Dependencies**: F2 (images), F3 (fonts)  |  **Risk**: 中

**方案**: 利用 lopdf 的 `add_page_contents()` 注入原始 PDF 内容流。

```
水印 = 文本水印 | 图片水印
文本水印 = 在每页内容流末尾追加 BT/ET 文本块（半透明、居中、旋转45°）
图片水印 = 在每页内容流中追加 Do 操作符引用 XObject
```

**API 设计**:
```rust
// 文本水印
EasyPdf::manipulate("in.pdf")
    .add_text_watermark("CONFIDENTIAL")
        .font(PdfFont::helvetica(48.0).bold())
        .color(PdfColor::rgb_u8(255, 0, 0))
        .opacity(0.3)
        .rotation(45.0)
    .save("watermarked.pdf")?;

// 图片水印
EasyPdf::manipulate("in.pdf")
    .add_image_watermark("logo.png")?
        .opacity(0.1)
        .position(Center)
    .save("watermarked.pdf")?;
```

**实施步骤**:
1. **easypdf-manipulate**: 新增 `PdfWatermarkBuilder` — 收集水印参数
2. **easypdf-manipulate**: 实现 `add_text_watermark` — 遍历 `page_iter()`，对每页构建原始 PDF 内容流（BT → Tr(extGState for opacity) → Tm(matrix for rotation) → Tj → ET），调用 `add_page_contents`
3. **easypdf-manipulate**: 实现 `add_image_watermark` — 将 `RawImage::decode_from_bytes` 后的数据注入为 XObject，在内容流中 `Do` 引用
4. **easypdf-core**: 新增 `PdfWatermark` 类型
5. **easypdf facade**: `PdfManipulateBuilder` 新增 `add_text_watermark()` / `add_image_watermark()`

**关键 PDF 算子**（需要拼装的原始内容流）:
```
/GS1 gs           % 设置 ExtGState (透明度)
q                  % 保存图形状态
1 0 0 1 300 400 cm % 平移+旋转矩阵
BT                 % 开始文本
/F1 48 Tf          % 选字体
0.5 0.5 0.5 rg    % 灰色
(...) Tj           % 画文本
ET                 % 结束文本
Q                  % 恢复图形状态
```

**风险**: 需要手动构造 PDF ExtGState 字典（透明度），并对每页注入。lopdf 支持 `add_page_contents` 和对象字典操作，可以实现。

---

### F8. PDF Layers (OCG) PDF 图层

**Size**: L  |  **Dependencies**: lopdf 对象字典操作  |  **Risk**: 高

**方案**: 通过 lopdf 操作 Catalog 字典中的 `/OCProperties` 条目添加 Optional Content Groups。

**实施步骤**:
1. **easypdf-manipulate**: 实现 OCG 字典创建（`/OCG`、`/OCProperties`、`/D`）
2. **easypdf-manipulate**: 对每页内容流用 `/OC` 标记包裹
3. **easypdf facede**: 提供 `add_layer(name)` → 返回 layer ID → `draw_on_layer(layer_id, ...)`

**注意**: 此功能用户需求量较低、实现复杂度高，建议推迟到 v1.0 后评估。

---

## Phase: v0.4 — Security 安全

### F9. Encryption 加密

**Size**: L  |  **Dependencies**: `aes` crate  |  **Risk**: 高

**现状**: printpdf 的 `PdfSaveOptions.secure` 字段为空（只定义了 `secure: bool`，无实际加密逻辑）。lopdf 仅支持 RC4 解密。

**方案**: 参考 `ms-offcrypto-writer`（easyexcel-rs 已用）的设计，实现 ECMA-376 Agile Encryption。

**实施步骤**:
1. **easypdf-writer**: 在 `finish()` 后，对已生成的 PDF bytes 进行后处理：
   - 生成随机 AES-256 密钥
   - 用密钥加密所有 stream 和 string 对象
   - 添加 `/Encrypt` 字典（V=5, R=6, SubFilter=adbe.pkcs7.s5）
   - 用用户密码加密密钥本身（SHA-512 → AES-256 wrap）
2. **easypdf-reader**: 在 `open()` 后检测 `/Encrypt` 字典，调用 lopdf 的 `decrypt(password)`
3. **easypdf-core**: `PdfError` 新增 `Encryption` 变体（已存在）
4. **easypdf facade**: `PdfCreateBuilder` 新增 `.password("...")` 方法

**变更文件**: 新建 `easypdf-crypto` 子 crate（可选），或直接在 writer/reader 中实现。

---

### F10. Permission Flags 权限标志

**Size**: S  |  **Dependencies**: F9  |  **Risk**: 低

**方案**: 加密时设置 `/Encrypt` 字典中的 `/P` 权限位掩码。

```
Bit  1: Printing
Bit  2: ModifyContents
Bit  4: CopyText
Bit  8: ModifyAnnotations
Bit 16: FillFormFields
Bit 32: ExtractText (accessibility)
Bit 256: AssembleDocument
Bit 512: PrintHighQuality
```

**API**:
```rust
EasyPdf::create("out.pdf")
    .password("secret")
    .permissions(Permissions::default()
        .allow_printing()
        .deny_modification())
    .do_write()?;
```

---

## Phase: v0.5 — Compliance 合规

### F11. PDF/A Validation PDF/A 校验

**Size**: XL  |  **Dependencies**: 需自建或等待社区 crate  |  **Risk**: 高

**方案**: PDF/A 是一组复杂的 ISO 约束（PDF/A-1, -2, -3），包括：
- 禁止加密
- 禁止 LZW/JPEG2000 压缩
- 禁止外部引用
- 强制 XMP 元数据
- 强制嵌入所有字体
- 设备无关颜色空间

**建议**: 此功能推迟到社区出现成熟的 `pdf-a` crate 后再集成。easypdf-rust 可以提供校验钩子 `validate_pdfa(level: PdfALevel) -> Vec<ValidationError>`，但完整的实现工作量大。

---

### F12. XMP Metadata XMP 元数据

**Size**: S  |  **Dependencies**: 无  |  **Risk**: 低

**方案**: XMP 是 PDF/A 必需、普通 PDF 也支持的标准化元数据格式。作为 XML 包嵌入 PDF 的 `/Metadata` 流中。

**实施步骤**:
1. **easypdf-core**: 扩展 `PdfMetadata` — 添加 `create_date`, `modify_date`, `document_id` 等 XMP 字段
2. **easypdf-writer**: 生成 XMP XML 字符串，作为 PDF stream 对象添加到 document catalog

**变更文件**: `easypdf-core/src/metadata.rs` (+30行), `easypdf-writer/src/lib.rs` (+40行)

---

### F13. Digital Signatures 数字签名

**Size**: XL  |  **Dependencies**: `rsa`, `x509-cert`, `der`  |  **Risk**: 高

**方案**: PDF 签名需要在文件中预留签名占位空间（`/ByteRange`），需要增量保存机制。lopdf 没有增量保存支持。

**建议**: 此功能推迟。需要：
1. 实现 PDF 增量保存（修改一个 PDF 而不重写整个文件）
2. 集成 RSA/SHA-256 签名算法
3. 构建 `/Sig` 字典、`/ByteRange` 数组、PKCS#7 签名容器

这与 `lopdf` 或 `printpdf` 都不兼容，需要从底层实现。

---

## Phase: v0.6 — Converters 转换器

### F14. HTML → PDF

**Size**: L  |  **Dependencies**: printpdf 的 `from_html()` 或 headless Chrome  |  **Risk**: 中

**printpdf 内置方案**:
```rust
// printpdf v0.8 已有
let doc = PdfDocument::from_html(html_str, &options)?;
```

**优点**: 零额外依赖。  
**缺点**: 需要系统安装 Chromium（或通过 `CHROMIUM_PATH` 环境变量指定）；渲染质量依赖 printpdf 的 HTML→PDF 转换器。

**API**:
```rust
EasyPdf::from_html("<h1>Hello</h1><p>World</p>")?
    .page(PageSize::A4)
    .save("out.pdf")?;

// 从 URL
EasyPdf::from_url("https://example.com")?
    .save("out.pdf")?;

// 指定 Chromium 路径
EasyPdf::from_html(html)
    .chromium_path("/usr/bin/chromium")
    .save("out.pdf")?;
```

---

### F15. SVG → PDF

**Size**: S  |  **Dependencies**: 无（printpdf 内置 `Svg::parse`）  |  **Risk**: 低

**API**:
```rust
EasyPdf::create("out.pdf")
    .add_svg(svg_string)?
        .position(100.0, 700.0)
        .size(200.0, 200.0)
    .do_write()?;

// 从文件
EasyPdf::create("out.pdf")
    .add_svg_file("diagram.svg")?
    .do_write()?;
```

**实施**:
1. **easypdf-writer**: 调用 `Svg::parse(svg_str)` → `doc.add_xobject` → `Op::UseXobject`
2. **easypdf facade**: 新增 `add_svg()` / `add_svg_file()` Builder 方法

---

### F16. Markdown → PDF

**Size**: M  |  **Dependencies**: 复用 F14 (HTML→PDF)  |  **Risk**: 低

**方案**: Markdown → HTML → PDF，两阶段转换。

```rust
EasyPdf::from_markdown("# Title\n\n**Bold** text")?
    .save("out.pdf")?;
```

**实施**:
1. **easypdf facade**: 新增 `EasyPdf::from_markdown(md)` — 用 `pulldown-cmark`（或类似的 Rust Markdown 库）转为 HTML
2. **easypdf facade**: 将 HTML 字符串传入 `PdfDocument::from_html()`

---

## Cross-cutting: Architecture Improvements 架构改进

### C1. Engine Abstraction Layer 引擎抽象层

**Phase**: v0.6+  |  **Size**: L  |  **Risk**: 中

**方案**: 定义 engine trait，允许运行时/编译时切换后端。

```rust
pub trait PdfEngine: Send + Sync {
    fn create_writer(&self, path: &Path) -> Result<Box<dyn PdfEngineWriter>>;
    fn create_reader(&self, path: &Path) -> Result<Box<dyn PdfEngineReader>>;
    fn create_manipulator(&self, path: &Path) -> Result<Box<dyn PdfEngineManipulator>>;
    fn engine_name(&self) -> &str;
    fn supported_features(&self) -> EngineFeatures;
}

// Feature flags:
// easypdf = { features = ["engine-lopdf", "engine-printpdf"] }
// easypdf = { features = ["engine-justpdf"] }  // future
```

**推迟理由**: 当前只有一组有效引擎（lopdf 读/操作、printpdf 写），抽象层在仅有一个实现时增加复杂度而不带来价值。等待 `justpdf` 或其他引擎成熟后再引入。

---

### C2. Layout Engine 自动布局引擎

**Phase**: v0.3+  |  **Size**: L  |  **Risk**: 中

**方案**: 提供流式布局引擎，自动计算元素的 (x, y) 位置。

```rust
EasyPdf::create("out.pdf")
    .use_layout(FlowLayout::vertical().spacing(12.0))
    .add_text("Title").font(helvetica(18).bold())
    .add_text("Paragraph 1...")
    .add_table(&data)
    .add_image("chart.png")?
    .do_write()?;
```

**核心数据结构**:
```rust
pub struct FlowLayout {
    direction: Direction,       // Vertical | Horizontal
    margins: Margins,
    spacing: f64,
    cursor: f64,                // current y (for vertical) or x (for horizontal)
    page_height: f64,
}
impl FlowLayout {
    pub fn next_position(&mut self, element_height: f64) -> (f64, f64);
    pub fn remaining_space(&self) -> f64;
    pub fn new_page(&mut self);  // reset cursor to top
}
```

---

## Implementation Priority Matrix 优先级矩阵

按**用户价值 × 实现成本**排序：

| Priority | Feature | Phase | Size | Value | Cost | Score |
|:---:|:---|:---:|:---:|:---:|:---:|:---:|
| 🔴 1 | F6 Multi-page Writer | v0.2 | S | ⭐⭐⭐⭐⭐ | S | **最高** |
| 🔴 2 | F2 Images | v0.2 | S | ⭐⭐⭐⭐⭐ | S | **最高** |
| 🔴 3 | F3 Custom Fonts | v0.2 | M | ⭐⭐⭐⭐ | M | 高 |
| 🟡 4 | F4 Shapes (line/rect) | v0.2 | M | ⭐⭐⭐ | M | 中 |
| 🟡 5 | F15 SVG→PDF | v0.6 | S | ⭐⭐⭐ | S | 中 |
| 🟡 6 | F5 Headers/Footers | v0.2 | M | ⭐⭐⭐⭐ | M | 中 |
| 🟡 7 | F1 Tables | v0.2 | L | ⭐⭐⭐⭐⭐ | L | 中 |
| 🟢 8 | F12 XMP Metadata | v0.5 | S | ⭐⭐ | S | 低 |
| 🟢 9 | F7 Watermarks | v0.3 | M | ⭐⭐⭐ | M | 低 |
| 🟢 10 | F14 HTML→PDF | v0.6 | L | ⭐⭐⭐ | L | 低 |
| 🟢 11 | F16 Markdown→PDF | v0.6 | M | ⭐⭐ | M | 低 |
| ⚪ 12 | F9 Encryption | v0.4 | L | ⭐⭐⭐⭐ | L | 推迟 |
| ⚪ 13 | F10 Permissions | v0.4 | S | ⭐⭐ | S | 推迟 |
| ⚪ 14 | F4 Circle/Ellipse | v0.2 | M | ⭐⭐ | M | 推迟 |
| ⚪ 15 | F8 PDF Layers | v0.3 | L | ⭐ | L | 推迟 |
| ⚪ 16 | F11 PDF/A | v0.5 | XL | ⭐⭐ | XL | 推迟 |
| ⚪ 17 | F13 Digital Sig. | v0.5 | XL | ⭐⭐ | XL | 推迟 |

---

## v0.2 Recommended Sprint Plan v0.2 推荐冲刺计划

```
Sprint 1 (1 week):
  Day 1-2: F6 Multi-page Writer (S)    ← 修复 v0.1 核心缺陷
  Day 3-4: F2 Images (S)               ← 高频需求
  Day 5:   F3 Custom Fonts (M) 开始

Sprint 2 (1 week):
  Day 1-3: F3 Custom Fonts (M) 完成
  Day 4-5: F4 Shapes line + rect (M)

Sprint 3 (1 week):
  Day 1-3: F5 Headers/Footers (M)
  Day 4-5: F1 Tables basic (L) 开始

Sprint 4 (2 weeks):
  Day 1-7: F1 Tables 完成 (L)
  Day 8-10: F15 SVG→PDF (S) — 快赢功能
```

---

## Risk Register 风险登记册

| Risk | Likelihood | Impact | Mitigation |
|:---|:---:|:---:|:---|
| printpdf API 大版本变更 | 中 | 高 | 锁版本，关注 changelog，提前适配 |
| lopdf API 大版本变更 | 中 | 中 | 同上 |
| Chromium 依赖（HTML→PDF） | 高 | 中 | 提供 feature gate，默认不启用 |
| 字体子集化复杂度（TTF） | 中 | 低 | v0.2 嵌入完整字体，后续子集化 |
| 加密实现安全性 | 高 | 高 | 参考成熟实现（ms-offcrypto-writer），加入安全审计 |
| PDF 规范兼容性 | 中 | 中 | 用真实 PDF reader 验证输出（Adobe Acrobat 等） |

# OCR 引擎替换（Windows OCR → PaddleOCR）实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 OCR 引擎从 `Windows.Media.Ocr` 替换为 PP-OCRv4（ONNX Runtime，通过 `pure-onnx-ocr` crate），同步改进列对齐算法。

**Architecture:** 保持 `OcrWord` 接口不变（`gacha.rs` 不感知引擎变化），新增 `paddle.rs` 封装 PP-OCRv4 ONNX 引擎，`ocr.rs` 内部调换实现。ONNX 模型文件已下载到 `assets/models/`，引擎使用 `OnceLock` 延迟初始化。列对齐从"列起始点"匹配改为"列区间"匹配。

**Tech Stack:** Rust、`pure-onnx-ocr`（crates.io）、tract 推理引擎（纯 Rust）、PP-OCRv4 ONNX 模型

---

### Task 1: 添加 pure-onnx-ocr 依赖并创建模块骨架

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/paddle.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 在 Cargo.toml 添加依赖**

在 `[dependencies]` 区块末尾添加：

```toml
# OCR 引擎（PP-OCRv4 ONNX + pure Rust tract 后端）
pure-onnx-ocr = { version = "0.1", features = ["image"] }
```

- [ ] **Step 2: 读取当前 `src-tauri/Cargo.toml` 确认依赖区块位置**

Read `src-tauri/Cargo.toml` → 找到 `[dependencies]` 区块末尾，在其前插入上述行。

- [ ] **Step 3: 创建 paddle.rs 模块骨架**

```rust
use anyhow::Result;

/// PP-OCRv4 引擎封装
pub struct PaddleOcrEngine {
    // 将在后续步骤填充
}

impl PaddleOcrEngine {
    /// 加载检测模型和识别模型
    pub fn new(det_model_path: &str, rec_model_path: &str, keys_path: &str) -> Result<Self> {
        todo!("Implement engine initialization using pure_onnx_ocr::OcrEngineBuilder")
    }

    /// 对图片字节进行 OCR，返回文字及其坐标
    pub fn recognize(&self, image_data: &[u8]) -> Result<Vec<crate::ocr::OcrWord>> {
        todo!("Implement OCR recognition via pure_onnx_ocr")
    }
}
```

- [ ] **Step 4: 在 lib.rs 注册模块**

在 `src-tauri/src/lib.rs` 的 `mod ocr;` 行后添加：

```rust
mod paddle;
```

- [ ] **Step 5: 验证编译**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```
Expected: 编译通过（有 `todo!()` 警告可接受）

- [ ] **Step 6: 提交**

```bash
git add src-tauri/Cargo.toml src-tauri/src/paddle.rs src-tauri/src/lib.rs
git commit -m "feat: add pure-onnx-ocr dependency and paddle module skeleton"
```

---

### Task 2: 实现 PaddleOcrEngine（加载 ONNX 模型 + 推理）

**前提条件:** `src-tauri/assets/models/` 目录下已存在以下文件（已下载完成）：
- `ch_PP-OCRv4_det_infer.onnx` (4.6 MB)
- `ch_PP-OCRv4_rec_infer.onnx` (11 MB)
- `ppocr_keys_v1.txt` (26 KB)

**Files:**
- Modify: `src-tauri/src/paddle.rs`

- [ ] **Step 1: 实现完整的 PaddleOcrEngine**

替换 paddle.rs 中的骨架代码为完整实现：

```rust
use anyhow::{Context, Result};
use image::GenericImageView;
use pure_onnx_ocr::{OcrEngineBuilder, OcrResult};

pub struct PaddleOcrEngine {
    engine: pure_onnx_ocr::OcrEngine,
}

impl PaddleOcrEngine {
    pub fn new(det_model_path: &str, rec_model_path: &str, keys_path: &str) -> Result<Self> {
        let engine = OcrEngineBuilder::new()
            .det_model_path(det_model_path)
            .rec_model_path(rec_model_path)
            .dictionary_path(keys_path)
            .build()
            .context("Failed to initialize PP-OCRv4 engine")?;

        Ok(Self { engine })
    }

    pub fn recognize(&self, image_data: &[u8]) -> Result<Vec<crate::ocr::OcrWord>> {
        let img = image::load_from_memory(image_data)
            .context("Failed to load image for OCR")?;
        let (orig_w, orig_h) = img.dimensions();

        // 调用 PP-OCRv4 推理
        let results: Vec<OcrResult> = self.engine.run_from_image(&img)
            .context("PP-OCRv4 recognition failed")?;

        // 转换为 OcrWord — OcrResult 包含 text + confidence + bounding_box
        // bounding_box 是 polygon: [(x1,y1), (x2,y2), (x3,y3), (x4,y4)]
        let words = results.into_iter()
            .filter(|r| !r.text.trim().is_empty())
            .map(|r| {
                let (x, y, w, h) = bounding_rect(&r.bounding_box);
                crate::ocr::OcrWord {
                    text: r.text,
                    x,
                    y,
                    width: w,
                    height: h,
                }
            })
            .collect();

        Ok(words)
    }
}

/// 将多边形 bounding box 转换为矩形 (x, y, width, height)
fn bounding_rect(poly: &geo_types::Polygon<f32>) -> (f64, f64, f64, f64) {
    use geo_types::Coord;
    let exterior = poly.exterior();
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    for coord in exterior.coords() {
        let x = coord.x as f64;
        let y = coord.y as f64;
        if x < min_x { min_x = x; }
        if y < min_y { min_y = y; }
        if x > max_x { max_x = x; }
        if y > max_y { max_y = y; }
    }
    (min_x, min_y, max_x - min_x, max_y - min_y)
}
```

> **注意：** `pure_onnx_ocr::OcrEngine` 的 `run_from_image` 方法签名可能为 `run_from_path`，也可能接受 `&image::DynamicImage`。如编译报错，调整调用方式。`geo-types` 是 `pure_onnx_ocr` 的传递依赖，可通过 `Cargo.toml` 显式引入以使用 `geo_types::Polygon`。

- [ ] **Step 2: 如果需要，在 Cargo.toml 添加 geo-types 依赖**

```toml
geo-types = "0.7"
```

- [ ] **Step 3: 验证编译**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```
Expected: 编译通过

- [ ] **Step 4: 提交**

```bash
git add src-tauri/Cargo.toml src-tauri/src/paddle.rs
git commit -m "feat: implement PaddleOCR engine with PP-OCRv4 ONNX models"
```

---

### Task 3: 替换 ocr.rs 实现（Windows OCR → PP-OCRv4）

**Files:**
- Modify: `src-tauri/src/ocr.rs`

- [ ] **Step 1: 重写 ocr_image 函数**

将 `ocr_image` 内部从 `Windows.Media.Ocr` 改为调用 `PaddleOcrEngine`，使用 `OnceLock` 实现引擎延迟初始化：

```rust
use anyhow::{Result, Context};
use std::sync::OnceLock;

pub struct OcrWord {
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

static OCR_ENGINE: OnceLock<crate::paddle::PaddleOcrEngine> = OnceLock::new();

fn get_or_init_engine() -> Result<&'static crate::paddle::PaddleOcrEngine> {
    OCR_ENGINE.get_or_try_init(|| {
        let model_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::env::current_dir().unwrap())
            .join("assets")
            .join("models");

        let det_path = model_dir.join("ch_PP-OCRv4_det_infer.onnx");
        let rec_path = model_dir.join("ch_PP-OCRv4_rec_infer.onnx");
        let keys_path = model_dir.join("ppocr_keys_v1.txt");

        crate::paddle::PaddleOcrEngine::new(
            det_path.to_str().context("Invalid det model path")?,
            rec_path.to_str().context("Invalid rec model path")?,
            keys_path.to_str().context("Invalid keys path")?,
        )
    })
}

/// 对图片字节进行 OCR，返回文字及其坐标
/// 坐标是原始图片尺寸
pub fn ocr_image(image_data: &[u8]) -> Result<Vec<OcrWord>> {
    let engine = get_or_init_engine()?;
    engine.recognize(image_data)
}
```

> **删掉的旧代码：** `windows::Storage::StorageFile`、`windows::Globalization::Language`、`windows::Media::Ocr::OcrEngine`、`windows::Graphics::Imaging::BitmapDecoder` 等 WinRT 相关导入和调用。以及 `image::imageops::resize`、灰度化、临时文件读写等预处理逻辑（PP-OCRv4 内部自带预处理）。

- [ ] **Step 2: 保留 normalize_item_name（保持不变）**

`normalize_item_name` 函数与 OCR 引擎无关，完全保留。

- [ ] **Step 3: 验证编译**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```
Expected: 编译通过

- [ ] **Step 4: 提交**

```bash
git add src-tauri/src/ocr.rs
git commit -m "feat: replace Windows OCR with PP-OCRv4 ONNX engine"
```

---

### Task 4: 改善列对齐算法（区间匹配）

**Files:**
- Modify: `src-tauri/src/commands/gacha.rs`

- [ ] **Step 1: 替换列分配逻辑**

找到 `gacha.rs:271-278` 的字分配到列的逻辑（当前用列起始 X 做 reverse 遍历），替换为区间匹配：

```rust
// ── 4. 对表头下所有行，用表头 X 范围判断每字属于哪列 ──
let mut data_rows: Vec<[String; 4]> = Vec::new();
let mut found_header = false;
let mut row_count = 0usize;

for ry in &row_centers {
    let mut rw: Vec<&crate::ocr::OcrWord> = all_words.iter()
        .filter(|w| (w.y + w.height / 2.0 - ry).abs() < half_span)
        .collect();
    rw.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
    let concat: String = rw.iter().map(|w| w.text.trim()).collect::<Vec<_>>().join("");

    if concat.contains("对象类型") { found_header = true; continue; }
    if !found_header { continue; }
    if rw.len() <= 2 { continue; }

    let mut cells: [Vec<String>; 4] = Default::default();
    for w in &rw {
        let cx = w.x + w.width / 2.0;
        // 用列区间匹配代替单点比较
        let mut ci: Option<usize> = None;
        for (hi, &(col_start, col_end)) in hdr_cols.iter().enumerate() {
            // 给列区间加 5px 缓冲
            if cx >= col_start - 5.0 && cx < col_end + 5.0 {
                ci = Some(hi);
                break;
            }
        }
        // fallback: 如果没落入任何区间，分配到最近的列起始点
        let ci = ci.unwrap_or_else(|| {
            hdr_cols.iter()
                .enumerate()
                .min_by(|(_, &(s1, _)), (_, &(s2, _))| {
                    let d1 = (cx - s1).abs();
                    let d2 = (cx - s2).abs();
                    d1.partial_cmp(&d2).unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(i, _)| i)
                .unwrap_or(3)
        });
        cells[ci].push(w.text.trim().to_string());
    }
    let merged: [String; 4] = [
        cells[0].join(""), cells[1].join(""),
        cells[2].join(""), cells[3].join(""),
    ];
    data_rows.push(merged);
    row_count += 1;
    if row_count >= 5 { break; }
}
```

**改动说明：**
1. 原来用 `hdr_x`（列起始 X 数组）做 `cx >= hx` 倒序遍历 → 改为用 `hdr_cols`（列区间 `(x_start, x_end)`）做区间包含检查
2. 每列区间加 5px 缓冲
3. 字落在所有区间外时 fallback 到最近列
4. 删掉 `hdr_x` 数组构造

- [ ] **Step 2: 验证编译**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```
Expected: 编译通过

- [ ] **Step 3: 确认测试仍通过**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -- --test-threads=1
```
Expected: `test_clustering` 等测试通过

- [ ] **Step 4: 提交**

```bash
git add src-tauri/src/commands/gacha.rs
git commit -m "fix: improve column alignment with interval-based matching"
```

---

### Task 5: 编译验证 + 清理

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: 确认不再使用的 windows crate features**

检查 `src-tauri/Cargo.toml` 中 `windows` crate 的 features。仍需要的：
- `UI_Notifications` — 可能用于 toast 通知
- `Win32_Foundation`、`Win32_UI_WindowsAndMessaging` — 窗口检测
- `Foundation_Collections`、`Globalization` — 当前是否还有引用？
- `Graphics_Imaging`、`Storage_Streams`、`Media_Ocr` — 如果 `ocr.rs` 不再使用，移除

**逐个确认后移除无用 feature。**

- [ ] **Step 2: 全局编译检查**

```bash
cargo check --manifest-path src-tauri/Cargo.toml 2>&1
```
Expected: 无 warning（除正常的 unused import 清理后）

- [ ] **Step 3: 运行全部测试**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -- --test-threads=1
```
Expected: 全部通过

- [ ] **Step 4: 提交**

```bash
git add src-tauri/Cargo.toml
git commit -m "chore: clean up unused dependencies after OCR engine swap"
```

---

### Task 6: 端到端验证

**Files:** （无代码修改）

- [ ] **Step 1: 确认模型文件路径正确**

```bash
ls -la src-tauri/assets/models/
```
Expected: 三个模型文件存在（.onnx + .txt）

- [ ] **Step 2: 确认测试全部通过**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -- --test-threads=1
```

- [ ] **Step 3: 启动应用做手动测试**

```bash
pnpm tauri dev
```

用一张真实跃迁记录截图导入，观察：
- OCR 能否识别出文字
- 列对齐是否正确
- 去重逻辑是否正常
- 日志中 `[WARP]`/`[IMPORT]` 输出是否合理

- [ ] **Step 4: 提交最终状态**

```bash
git add -A
git commit -m "feat: complete OCR engine replacement with PP-OCRv4"
```

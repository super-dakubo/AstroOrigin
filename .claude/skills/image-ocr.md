---
name: image-ocr
description: 通用图片文字提取 — PP-OCRv4 ONNX 离线 OCR 模块，为需要从截图中提取文字的功能提供标准化接口
model: opus
---

# image-ocr：通用图片文字提取

基于 PP-OCRv4（ONNX + tract 推理引擎）的纯离线中文/英文 OCR 模块，已集成到 Tauri 2 + Rust 项目中的标准方案。

## 前置条件

### 1. Cargo.toml 依赖

```toml
# 纯 Rust ONNX 推理引擎
pure-onnx-ocr = "0.1"
# 图片加载（OCR 模块依赖）
image = { version = "0.25", default-features = false, features = ["png"] }
# 错误处理（现有项目一般已有）
anyhow = "1"
```

如果需要从 bounding box polygon 提取坐标，可能还需：
```toml
geo-types = "0.7"  # pure-onnx-ocr 的传递依赖
```

### 2. 模型文件

PP-OCRv4 ONNX 文件在 `src-tauri/assets/models/` 目录下，已预下载：

| 文件 | 大小 | 用途 |
| --- | --- | --- |
| `ch_PP-OCRv4_det_infer.onnx` | ~4.6 MB | 文字检测模型 |
| `ch_PP-OCRv4_rec_infer.onnx` | ~11 MB | 文字识别模型 |
| `ppocr_keys_v1.txt` | ~26 KB | 中文字符表 |

> 模型来源：RapidOCR（ModelScope CDN）。如需在其他机器部署，从以下 URL 下载：
> - 检测: `https://www.modelscope.cn/models/RapidAI/RapidOCR/resolve/v3.4.0/onnx/PP-OCRv4/det/ch_PP-OCRv4_det_infer.onnx`
> - 识别: `https://www.modelscope.cn/models/RapidAI/RapidOCR/resolve/v3.4.0/onnx/PP-OCRv4/rec/ch_PP-OCRv4_rec_infer.onnx`
> - 字符表: `https://www.modelscope.cn/models/RapidAI/RapidOCR/resolve/v3.4.0/paddle/PP-OCRv4/rec/ch_PP-OCRv4_rec_infer/ppocr_keys_v1.txt`

## 模块结构

新增两个文件，修改 Cargo.toml + lib.rs：

```
src-tauri/
├── src/
│   ├── paddle.rs    ← 引擎封装（线程安全包装 + 模型加载）
│   ├── ocr.rs       ← 公开 API：ocr_image() → Vec<OcrWord>
│   └── lib.rs       ← 加 mod paddle;
└── assets/
    └── models/      ← ONNX 模型文件
```

## 完整代码

### paddle.rs — 引擎封装

```rust
use anyhow::Result;
use pure_onnx_ocr::OcrEngine;
use std::sync::{Mutex, OnceLock};

struct SafeOcrEngine(Mutex<OcrEngine>);

// Safety: OcrEngine uses RefCell internally (not Send+Sync),
// we wrap it in a Mutex to guarantee single-threaded access.
unsafe impl Send for SafeOcrEngine {}
unsafe impl Sync for SafeOcrEngine {}

static OCR_ENGINE: OnceLock<SafeOcrEngine> = OnceLock::new();

pub struct PaddleOcrEngine;

impl PaddleOcrEngine {
    /// 初始化引擎，加载检测模型 + 识别模型
    pub fn init(det_model_path: &str, rec_model_path: &str, dict_path: &str) -> Result<()> {
        let engine = pure_onnx_ocr::OcrEngineBuilder::new()
            .det_model_path(det_model_path)
            .rec_model_path(rec_model_path)
            .dictionary_path(dict_path)
            .det_limit_side_len(640)   // 加快检测速度
            .rec_batch_size(8)         // 批量识别文本区域
            .build()?;
        OCR_ENGINE
            .set(SafeOcrEngine(Mutex::new(engine)))
            .map_err(|_| anyhow::anyhow!("OCR engine already initialized"))?;
        Ok(())
    }

    /// 执行 OCR，返回文字及坐标
    pub fn recognize(image_data: &[u8]) -> Result<Vec<crate::ocr::OcrWord>> {
        let safe_engine = OCR_ENGINE
            .get()
            .ok_or_else(|| anyhow::anyhow!("OCR engine not initialized"))?;
        let engine = safe_engine.0
            .lock()
            .map_err(|_| anyhow::anyhow!("OCR engine lock poisoned"))?;
        let img = image::load_from_memory(image_data)?;
        let results = engine.run_from_image(&img)?;

        let words = results.into_iter()
            .filter(|r| !r.text.trim().is_empty())
            .map(|r| {
                let bbox = &r.bounding_box;
                let coords: Vec<(f64, f64)> = bbox.exterior().points()
                    .map(|p| (p.x(), p.y())).collect();
                let x = coords.iter().map(|(x, _)| *x).fold(f64::MAX, f64::min);
                let y = coords.iter().map(|(_, y)| *y).fold(f64::MAX, f64::min);
                let max_x = coords.iter().map(|(x, _)| *x).fold(f64::MIN, f64::max);
                let max_y = coords.iter().map(|(_, y)| *y).fold(f64::MIN, f64::max);
                crate::ocr::OcrWord {
                    text: r.text, x, y,
                    width: max_x - x, height: max_y - y,
                }
            })
            .collect();
        Ok(words)
    }
}
```

### ocr.rs — 公开 API

```rust
//! 通用图片文字提取（OCR）模块
//!
//! # 用法
//! ```ignore
//! let image_data = std::fs::read("截图.png")?;
//! let words = ocr::ocr_image(&image_data)?;
//! for w in &words {
//!     println!("{} @ ({}, {})", w.text, w.x, w.y);
//! }
//! ```

use anyhow::{Context, Result};
use std::sync::OnceLock;

/// OCR 识别出的文字块，包含文本和位置
#[derive(Debug, Clone)]
pub struct OcrWord {
    pub text: String,
    pub x: f64,      // 左上角 x（原始图片尺寸）
    pub y: f64,      // 左上角 y
    pub width: f64,  // 包围盒宽度
    pub height: f64, // 包围盒高度
}

static OCR_INIT: OnceLock<Result<()>> = OnceLock::new();

// 模型路径解析：开发/生产/兜底三路 fallback
fn resolve_model_dir() -> std::path::PathBuf {
    // 1) 生产：可执行文件同级
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let p = parent.join("assets").join("models");
            if p.join("ch_PP-OCRv4_det_infer.onnx").exists() { return p; }
        }
    }
    // 2) 开发：CARGO_MANIFEST_DIR
    let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets").join("models");
    if p.join("ch_PP-OCRv4_det_infer.onnx").exists() { return p; }
    // 3) 兜底
    std::env::current_dir().unwrap_or_default().join("assets").join("models")
}

fn ensure_engine_initialized() -> Result<()> {
    let result = OCR_INIT.get_or_init(|| {
        let model_dir = resolve_model_dir();
        crate::paddle::PaddleOcrEngine::init(
            model_dir.join("ch_PP-OCRv4_det_infer.onnx").to_str().context("...")?,
            model_dir.join("ch_PP-OCRv4_rec_infer.onnx").to_str().context("...")?,
            model_dir.join("ppocr_keys_v1.txt").to_str().context("...")?,
        )
    });
    result.as_ref().map_err(|e| anyhow::anyhow!("{}", e)).copied()
}

/// 对图片进行 OCR，提取所有文字及其坐标（引擎延迟初始化）
pub fn ocr_image(image_data: &[u8]) -> Result<Vec<OcrWord>> {
    ensure_engine_initialized()?;
    crate::paddle::PaddleOcrEngine::recognize(image_data)
}
```

### lib.rs 注册

```rust
mod paddle;
mod ocr;
```

## API 用法

```rust
// 1. 基础用法：提取所有文字
let words = ocr::ocr_image(&image_data)?;

// 2. 按区域过滤（如只提取屏幕上半部分）
let top_words: Vec<_> = words.into_iter()
    .filter(|w| w.y < 500.0)
    .collect();

// 3. 按行聚类输出
let mut rows: Vec<Vec<OcrWord>> = Vec::new();
for w in &words {
    let cy = w.y + w.height / 2.0;
    if let Some(r) = rows.iter_mut().find(|r: &&mut Vec<OcrWord>| {
        (cy - (r[0].y + r[0].height / 2.0)).abs() < 25.0
    }) {
        r.push(w.clone());
    } else {
        rows.push(vec![w.clone()]);
    }
}
// rows 中每个 Vec 就是一行文字

// 4. 异步调用（spawn_blocking，因为引擎内部是同步推理）
tokio::task::spawn_blocking(move || {
    let words = ocr::ocr_image(&img_bytes)?;
    Ok(words)
}).await?;
```

## 性能特征

| 指标 | 值 |
|------|-----|
| 单张截图（1920×1080） | ~2-3s |
| 模型加载（首次调用） | ~200-500ms |
| 内存占用 | ~50MB（含tract+模型） |
| 引擎初始化 | 一次性，OnceLock 延迟初始化 |
| 线程安全 | Mutex 保护，单线程推理 |
| 精度（中文） | ~95-97%（游戏UI实际表现优异） |

## 已知限制

- `pure-onnx-ocr` 用 `tract`（纯 Rust ONNX），比 C++ ONNX Runtime 慢 2-3x
- `OcrEngine` 内部用 `RefCell`，不是 `Send + Sync`，需要 `Mutex` 包装
- 无法多线程并行（Mutex 排队），如需并行需 N 份引擎实例（N×15MB）
- 输出是按文本区域（词组）而非单字，与 WinRT OCR 行为不同

## 快速生成指引

在需要新的 OCR 功能时：
1. 确保 `Cargo.toml` 已有 `pure-onnx-ocr` 依赖
2. 确认 `lib.rs` 已有 `mod paddle; mod ocr;`
3. 模型文件在 `assets/models/` 目录
4. 在 commands 模块中直接调用 `ocr::ocr_image(&img_bytes)?`
5. 如需进度上报，注入全局 `APP_HANDLE` 并用 `emit_phase()` 模式

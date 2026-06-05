//! 通用图片文字提取（OCR）模块
//!
//! 基于 PP-OCRv4（ONNX + tract 推理引擎），提供纯离线的中文文字提取。
//! 任何需要从截图中提取文字的功能，统一调用此模块即可。
//!
//! # 用法
//!
//! ```ignore
//! let image_data = std::fs::read("截图.png")?;
//! let words = ocr::ocr_image(&image_data)?;
//! for w in &words {
//!     println!("{} @ ({}, {}) {}x{}", w.text, w.x, w.y, w.width, w.height);
//! }
//! ```
//!
//! 输出坐标是原始图片尺寸。模型文件在首次调用时自动加载（`OnceLock` 延迟初始化）。

use anyhow::{Context, Result};
use std::sync::OnceLock;

/// OCR 识别出的单个文字块，包含文本内容和它在图片中的位置
///
/// 坐标系统：原点在图片左上角，x 向右增大，y 向下增大。
/// `width` 和 `height` 是该文字块的包围盒尺寸。
#[derive(Debug, Clone)]
pub struct OcrWord {
    /// 识别出的文本内容
    pub text: String,
    /// 文字块左上角 x 坐标（原始图片尺寸）
    pub x: f64,
    /// 文字块左上角 y 坐标（原始图片尺寸）
    pub y: f64,
    /// 文字块宽度
    pub width: f64,
    /// 文字块高度
    pub height: f64,
}

static OCR_INIT: OnceLock<Result<()>> = OnceLock::new();

/// 尝试多个可能的位置寻找模型目录
fn resolve_model_dir() -> std::path::PathBuf {
    // 1) 生产模式：可执行文件同级 assets/models/
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let p = parent.join("assets").join("models");
            if p.join("ch_PP-OCRv4_det_infer.onnx").exists() {
                return p;
            }
        }
    }
    // 2) 开发模式：src-tauri/assets/models/（CARGO_MANIFEST_DIR）
    let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("models");
    if p.join("ch_PP-OCRv4_det_infer.onnx").exists() {
        return p;
    }
    // 3) 兜底：当前目录
    std::env::current_dir().unwrap_or_default().join("assets").join("models")
}

fn ensure_engine_initialized() -> Result<()> {
    let result = OCR_INIT.get_or_init(|| {
        let model_dir = resolve_model_dir();

        let det_path = model_dir.join("ch_PP-OCRv4_det_infer.onnx");
        let rec_path = model_dir.join("ch_PP-OCRv4_rec_infer.onnx");
        let keys_path = model_dir.join("ppocr_keys_v1.txt");

        eprintln!("[OCR] Initializing PP-OCRv4 engine from: {:?}", model_dir);

        crate::paddle::PaddleOcrEngine::init(
            det_path.to_str().context("Invalid det model path")?,
            rec_path.to_str().context("Invalid rec model path")?,
            keys_path.to_str().context("Invalid keys path")?,
        )
    });
    result.as_ref().map_err(|e| anyhow::anyhow!("{}", e)).copied()
}

/// 对解码后的图片进行 OCR，提取所有文字及其坐标
///
/// 这是本模块的公开入口之一。接受已解码的图片（`image::DynamicImage`），
/// 返回按从上到下、从左到右大致排序的文字块列表。
///
/// 坐标以原始图片尺寸为基准（不做任何内部缩放换算）。
///
/// 引擎采用延迟初始化：第一次调用时加载 ~15MB 的 ONNX 模型文件，
/// 后续调用直接复用缓存实例。
///
/// # 示例
///
/// ```ignore
/// let img = image::load_from_memory(&image_data)?;
/// let words = ocr_image(&img)?;
/// ```
pub fn ocr_image(img: &image::DynamicImage) -> Result<Vec<OcrWord>> {
    ensure_engine_initialized()?;
    crate::paddle::PaddleOcrEngine::recognize(img)
}

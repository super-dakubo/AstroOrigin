use anyhow::{Context, Result};
use std::sync::OnceLock;

/// OCR 识别出的单个文字/词，带坐标
#[derive(Debug, Clone)]
pub struct OcrWord {
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

static OCR_INIT: OnceLock<Result<()>> = OnceLock::new();

fn ensure_engine_initialized() -> Result<()> {
    let result = OCR_INIT.get_or_init(|| {
        let model_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::env::current_dir().unwrap())
            .join("assets")
            .join("models");

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

/// 对图片字节进行 OCR，返回文字及其坐标
/// 坐标是原始图片尺寸
pub fn ocr_image(image_data: &[u8]) -> Result<Vec<OcrWord>> {
    ensure_engine_initialized()?;
    crate::paddle::PaddleOcrEngine::recognize(image_data)
}

/// 规范化物品名称
pub fn normalize_item_name(name: &str, normalizations: &[(&str, &str)]) -> String {
    let trimmed = name.trim();
    for (from, to) in normalizations {
        if trimmed.contains(from) {
            return to.to_string();
        }
    }
    trimmed.to_string()
}

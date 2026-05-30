use anyhow::Result;
use pure_onnx_ocr::OcrEngine;
use std::sync::{Mutex, OnceLock};

/// `OcrEngine` 内部使用 `RefCell`，不是 `Send` + `Sync`。
/// 此包装器用 `Mutex` 保证单线程访问，并标记为 Send/Sync 以允许静态存储。
struct SafeOcrEngine(Mutex<OcrEngine>);

// Safety: OcrEngine is only accessed through the Mutex, guaranteeing
// mutual exclusion. The inner RefCell-backed cache in tract is never
// accessed concurrently.
unsafe impl Send for SafeOcrEngine {}
unsafe impl Sync for SafeOcrEngine {}

static OCR_ENGINE: OnceLock<SafeOcrEngine> = OnceLock::new();

/// PaddleOCR 引擎封装，管理引擎初始化与推理
pub struct PaddleOcrEngine;

impl PaddleOcrEngine {
    /// 初始化 OCR 引擎，加载 ONNX 模型
    pub fn init(det_model_path: &str, rec_model_path: &str, dict_path: &str) -> Result<()> {
        let engine = pure_onnx_ocr::OcrEngineBuilder::new()
            .det_model_path(det_model_path)
            .rec_model_path(rec_model_path)
            .dictionary_path(dict_path)
            .build()?;
        OCR_ENGINE
            .set(SafeOcrEngine(Mutex::new(engine)))
            .map_err(|_| anyhow::anyhow!("OCR engine already initialized"))?;
        Ok(())
    }

    /// 对图片字节进行 OCR，返回文字及坐标
    pub fn recognize(image_data: &[u8]) -> Result<Vec<crate::ocr::OcrWord>> {
        let safe_engine = OCR_ENGINE
            .get()
            .ok_or_else(|| anyhow::anyhow!("OCR engine not initialized"))?;
        let engine = safe_engine
            .0
            .lock()
            .map_err(|_| anyhow::anyhow!("OCR engine lock poisoned"))?;
        let img = image::load_from_memory(image_data)?;
        let results = engine.run_from_image(&img)?;

        let words = results
            .into_iter()
            .filter(|r| !r.text.trim().is_empty())
            .map(|r| {
                let bbox = &r.bounding_box;
                let coords: Vec<(f64, f64)> = bbox
                    .exterior()
                    .points()
                    .map(|p| (p.x(), p.y()))
                    .collect();
                let x = coords
                    .iter()
                    .map(|(x, _)| *x)
                    .fold(f64::MAX, f64::min);
                let y = coords
                    .iter()
                    .map(|(_, y)| *y)
                    .fold(f64::MAX, f64::min);
                let max_x = coords
                    .iter()
                    .map(|(x, _)| *x)
                    .fold(f64::MIN, f64::max);
                let max_y = coords
                    .iter()
                    .map(|(_, y)| *y)
                    .fold(f64::MIN, f64::max);

                crate::ocr::OcrWord {
                    text: r.text,
                    x,
                    y,
                    width: max_x - x,
                    height: max_y - y,
                }
            })
            .collect();

        Ok(words)
    }
}

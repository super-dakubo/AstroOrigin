use anyhow::Result;
use image::GenericImageView;
use std::io::Write;

/// OCR 识别出的单个文字/词，带坐标
#[derive(Debug, Clone)]
pub struct OcrWord {
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// 对图片字节进行 OCR，返回文字及其坐标
/// 坐标是原始图片尺寸（内部 2x 放大后换算回原始坐标）
pub fn ocr_image(image_data: &[u8]) -> Result<Vec<OcrWord>> {
    let img = image::load_from_memory(image_data)?;
    let (orig_w, orig_h) = img.dimensions();

    // 灰度化 + 2x 放大，提高识别率
    let gray = img.grayscale();
    let enlarged = image::imageops::resize(
        &gray,
        gray.width() * 2,
        gray.height() * 2,
        image::imageops::FilterType::CatmullRom,
    );
    let mut png_buf = std::io::Cursor::new(Vec::new());
    enlarged.write_to(&mut png_buf, image::ImageFormat::Png)?;
    let png_bytes = png_buf.into_inner();

    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join(format!("astrorigin_ocr_{}.png", std::process::id()));
    let mut file = std::fs::File::create(&temp_path)?;
    file.write_all(&png_bytes)?;
    file.flush()?;
    drop(file);

    let path_str = temp_path.to_str().ok_or_else(|| anyhow::anyhow!("Invalid temp path"))?;
    let file = windows::Storage::StorageFile::GetFileFromPathAsync(
        &windows::core::HSTRING::from(path_str),
    )?.get()?;

    let stream = file.OpenAsync(windows::Storage::FileAccessMode::Read)?.get()?;

    let language =
        windows::Globalization::Language::CreateLanguage(&windows::core::HSTRING::from("zh-CN"))?;

    let engine = windows::Media::Ocr::OcrEngine::TryCreateFromLanguage(&language)?;

    let decoder = windows::Graphics::Imaging::BitmapDecoder::CreateAsync(&stream)?.get()?;
    let frame = decoder.GetFrameAsync(0)?.get()?;
    let bitmap = frame.GetSoftwareBitmapAsync()?.get()?;

    let result = engine.RecognizeAsync(&bitmap)?.get()?;

    let _ = std::fs::remove_file(&temp_path);

    // 提取带坐标的文字，坐标从 2x 放大回退到原始尺寸
    let scale_x = orig_w as f64 / (enlarged.width() as f64);
    let scale_y = orig_h as f64 / (enlarged.height() as f64);

    let mut words = Vec::new();
    for line in result.Lines()?.into_iter() {
        for word in line.Words()?.into_iter() {
            let text = match word.Text() {
                Ok(t) => t.to_string(),
                Err(_) => continue,
            };
            if text.trim().is_empty() { continue; }
            let rect = match word.BoundingRect() {
                Ok(r) => r,
                Err(_) => continue,
            };
            words.push(OcrWord {
                text,
                x: rect.X as f64 * scale_x,
                y: rect.Y as f64 * scale_y,
                width: rect.Width as f64 * scale_x,
                height: rect.Height as f64 * scale_y,
            });
        }
    }

    Ok(words)
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

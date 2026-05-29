use anyhow::{Context, Result};
use std::io::Write;

/// 对图片字节进行 OCR，返回识别出的文本行
/// 使用 Windows.Media.Ocr API
/// 注意：先存临时文件再让 WinRT 读取，避免流所有权问题
pub fn ocr_image(image_data: &[u8]) -> Result<Vec<String>> {
    let img = image::load_from_memory(image_data)?;

    // 灰度化 + 2x 放大，提高 OCR 识别率
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
    use windows::Media::Ocr::OcrLine;

    let lines: Vec<String> = result
        .Lines()?
        .into_iter()
        .filter_map(|line: OcrLine| line.Text().ok())
        .map(|h| h.to_string())
        .collect();

    let _ = std::fs::remove_file(&temp_path);
    Ok(lines)
}

/// 对图片裁剪区域进行 OCR
pub fn ocr_region(image_data: &[u8], region: (u32, u32, u32, u32)) -> Result<Vec<String>> {
    let img = image::load_from_memory(image_data).context("Failed to decode image")?;
    let cropped = img.crop_imm(region.0, region.1, region.2, region.3);
    let mut buf = std::io::Cursor::new(Vec::new());
    cropped
        .write_to(&mut buf, image::ImageFormat::Png)
        .context("Failed to encode cropped region")?;
    ocr_image(buf.get_ref())
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

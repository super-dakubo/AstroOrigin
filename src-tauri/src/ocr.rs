use anyhow::{Context, Result};
use std::io::Write;

/// 对图片字节进行 OCR，返回识别出的文本行
/// 使用 Windows.Media.Ocr API
/// 注意：先存临时文件再让 WinRT 读取，避免流所有权问题
pub fn ocr_image(image_data: &[u8]) -> Result<Vec<String>> {
    eprintln!("[OCR] Step 1: loading image from memory ({} bytes)", image_data.len());
    let img = image::load_from_memory(image_data)?;

    eprintln!("[OCR] Step 2: encoding to PNG ({}x{})", img.width(), img.height());
    let mut png_buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut png_buf, image::ImageFormat::Png)?;
    let png_bytes = png_buf.into_inner();

    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join(format!("astrorigin_ocr_{}.png", std::process::id()));
    eprintln!("[OCR] Step 3: writing temp file {:?}", temp_path);
    let mut file = std::fs::File::create(&temp_path)?;
    file.write_all(&png_bytes)?;
    file.flush()?;
    drop(file);

    let path_str = temp_path.to_str().ok_or_else(|| anyhow::anyhow!("Invalid temp path"))?;
    eprintln!("[OCR] Step 4: StorageFile::GetFileFromPathAsync");
    let file = windows::Storage::StorageFile::GetFileFromPathAsync(
        &windows::core::HSTRING::from(path_str),
    )?.get()?;

    eprintln!("[OCR] Step 5: file.OpenAsync(Read)");
    let stream = file.OpenAsync(windows::Storage::FileAccessMode::Read)?.get()?;

    eprintln!("[OCR] Step 6: Language::CreateLanguage(zh-CN)");
    let language =
        windows::Globalization::Language::CreateLanguage(&windows::core::HSTRING::from("zh-CN"))?;

    eprintln!("[OCR] Step 7: OcrEngine::TryCreateFromLanguage");
    let engine = windows::Media::Ocr::OcrEngine::TryCreateFromLanguage(&language)?;

    eprintln!("[OCR] Step 8: BitmapDecoder::CreateAsync");
    let decoder = windows::Graphics::Imaging::BitmapDecoder::CreateAsync(&stream)?.get()?;

    eprintln!("[OCR] Step 9: decoder.GetFrameAsync(0)");
    let frame = decoder.GetFrameAsync(0)?.get()?;

    eprintln!("[OCR] Step 10: frame.GetSoftwareBitmapAsync");
    let bitmap = frame.GetSoftwareBitmapAsync()?.get()?;

    eprintln!("[OCR] Step 11: engine.RecognizeAsync");
    let result = engine.RecognizeAsync(&bitmap)?.get()?;

    eprintln!("[OCR] Step 12: extracting text lines");
    use windows::Media::Ocr::OcrLine;

    let lines: Vec<String> = result
        .Lines()?
        .into_iter()
        .filter_map(|line: OcrLine| line.Text().ok())
        .map(|h| h.to_string())
        .collect();

    let _ = std::fs::remove_file(&temp_path);
    eprintln!("[OCR] Done: {} lines extracted", lines.len());
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

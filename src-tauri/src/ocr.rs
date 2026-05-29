use anyhow::{Context, Result};
use std::io::Write;

/// 对图片字节进行 OCR，返回识别出的文本行
/// 使用 Windows.Media.Ocr API
/// 注意：先存临时文件再让 WinRT 读取，避免流所有权问题
pub fn ocr_image(image_data: &[u8]) -> Result<Vec<String>> {
    // 使用 image 库加载图片
    let img = image::load_from_memory(image_data)
        .context("Failed to decode image for OCR")?;

    // 编码为 PNG 格式供 Windows OCR 使用
    let mut png_buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut png_buf, image::ImageFormat::Png)
        .context("Failed to encode image as PNG")?;
    let png_bytes = png_buf.into_inner();

    // 写入临时文件（WinRT 从文件解码更稳定，避免 InMemoryRandomAccessStream 的 RO_E_CLOSED）
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join(format!("astrorigin_ocr_{}.png", std::process::id()));
    let mut file = std::fs::File::create(&temp_path)
        .context("Failed to create temp file for OCR")?;
    file.write_all(&png_bytes)
        .context("Failed to write temp file for OCR")?;
    file.flush()?;

    // 用 WinRT API 打开临时文件
    let path_str = temp_path.to_str().context("Invalid temp path")?;
    let file = windows::Storage::StorageFile::GetFileFromPathAsync(
        &windows::core::HSTRING::from(path_str),
    )?
    .get()
    .context("Failed to open temp file")?;

    let stream = file.OpenAsync(windows::Storage::FileAccessMode::Read)?
        .get()
        .context("Failed to open file stream")?;

    // 获取 OCR 引擎（中文简体）
    let language =
        windows::Globalization::Language::CreateLanguage(&windows::core::HSTRING::from("zh-CN"))?;
    let engine = windows::Media::Ocr::OcrEngine::TryCreateFromLanguage(&language)
        .context("Failed to create OCR engine for zh-CN")?;

    // 解码图片为 SoftwareBitmap
    let decoder = windows::Graphics::Imaging::BitmapDecoder::CreateAsync(&stream)?
        .get()
        .context("Failed to create bitmap decoder")?;
    let frame = decoder.GetFrameAsync(0)?.get()
        .context("Failed to get frame")?;
    let bitmap = frame.GetSoftwareBitmapAsync()?.get()
        .context("Failed to get software bitmap")?;

    // 执行 OCR
    let result = engine.RecognizeAsync(&bitmap)?.get()
        .context("OCR recognition failed")?;

    // 清理临时文件
    let _ = std::fs::remove_file(&temp_path);

    // 提取文本行
    use windows::Media::Ocr::OcrLine;

    let lines: Vec<String> = result
        .Lines()?
        .into_iter()
        .filter_map(|line: OcrLine| line.Text().ok())
        .map(|h| h.to_string())
        .collect();

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

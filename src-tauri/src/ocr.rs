use anyhow::{Context, Result};

/// 对图片字节进行 OCR，返回识别出的文本行
/// 使用 Windows.Media.Ocr API
pub fn ocr_image(image_data: &[u8]) -> Result<Vec<String>> {
    // 使用 image 库加载图片
    let img = image::load_from_memory(image_data)
        .context("Failed to decode image for OCR")?;

    // 编码为 PNG 格式供 Windows OCR 使用
    let mut png_buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut png_buf, image::ImageFormat::Png)
        .context("Failed to encode image as PNG")?;

    // 创建内存流供 Windows API 使用
    let stream = windows::Storage::Streams::InMemoryRandomAccessStream::new()?;

    // 写入 PNG 数据到流
    {
        use windows::Storage::Streams::DataWriter;
        let writer = DataWriter::CreateDataWriter(&stream)?;
        let bytes = png_buf.into_inner();
        writer.WriteBytes(&bytes)?;
        writer.StoreAsync()?.get()?;
        writer.FlushAsync()?.get()?;
    }

    // 获取 OCR 引擎（中文简体）
    let language =
        windows::Globalization::Language::CreateLanguage(&windows::core::HSTRING::from("zh-CN"))?;
    let engine = windows::Media::Ocr::OcrEngine::TryCreateFromLanguage(&language)
        .context("Failed to create OCR engine for zh-CN")?;

    // 解码图片为 SoftwareBitmap
    let decoder = windows::Graphics::Imaging::BitmapDecoder::CreateAsync(&stream)?
        .get()?;
    let frame = decoder.GetFrameAsync(0)?.get()?;
    let bitmap = frame.GetSoftwareBitmapAsync()?.get()?;

    // 执行 OCR
    let result = engine.RecognizeAsync(&bitmap)?.get()?;

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

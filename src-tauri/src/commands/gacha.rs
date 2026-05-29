use serde::{Deserialize, Serialize};
use crate::db::DbPool;
use crate::error::TauriResult;
use crate::game::GameKind;
use anyhow::Context;
use image::GenericImageView;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GachaRecord {
    pub id: i64,
    pub game_kind: String,
    pub item_name: String,
    pub star_rating: i32,
    pub record_date: String,
    pub is_won: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GachaStats {
    pub total_pulls: i64,
    pub five_star_count: i64,
    pub lost_count: i64,
    pub current_pity: i32,
    pub avg_pulls_per_five_star: f64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GachaImportResult {
    pub imported: usize,
    pub duplicates: usize,
}

#[tauri::command]
pub async fn get_gacha_records(
    pool: tauri::State<'_, DbPool>,
    game_kind: String,
    limit: Option<i64>,
) -> TauriResult<Vec<GachaRecord>> {
    let limit = limit.unwrap_or(100);
    let pool = pool.inner().clone();

    tokio::task::spawn_blocking(move || {
        let conn = pool.get().context("Failed to get DB connection")?;
        let mut stmt = conn.prepare(
            "SELECT id, game_kind, item_name, star_rating, record_date, is_won
             FROM gacha_records
             WHERE game_kind = ?
             ORDER BY record_date DESC, id DESC
             LIMIT ?",
        )?;

        let records = stmt
            .query_map(rusqlite::params![game_kind, limit], |row| {
                Ok(GachaRecord {
                    id: row.get(0)?,
                    game_kind: row.get(1)?,
                    item_name: row.get(2)?,
                    star_rating: row.get(3)?,
                    record_date: row.get(4)?,
                    is_won: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to collect records")?;

        Ok(records)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .map_err(|e: anyhow::Error| format!("{:#}", e))
}

#[tauri::command]
pub async fn get_gacha_stats(
    pool: tauri::State<'_, DbPool>,
    game_kind: String,
) -> TauriResult<GachaStats> {
    let pool = pool.inner().clone();

    tokio::task::spawn_blocking(move || {
        let conn = pool.get().context("Failed to get DB connection")?;

        let total: i64 = conn.query_row(
            "SELECT COUNT(*) FROM gacha_records WHERE game_kind = ?",
            rusqlite::params![game_kind],
            |row| row.get(0),
        )?;

        let five_star: i64 = conn.query_row(
            "SELECT COUNT(*) FROM gacha_records WHERE game_kind = ? AND star_rating = 5",
            rusqlite::params![game_kind],
            |row| row.get(0),
        )?;

        let lost: i64 = conn.query_row(
            "SELECT COUNT(*) FROM gacha_records WHERE game_kind = ? AND star_rating = 5 AND is_won = 0",
            rusqlite::params![game_kind],
            |row| row.get(0),
        )?;

        let latest_five_star_id: Option<i64> = conn
            .query_row(
                "SELECT MAX(id) FROM gacha_records WHERE game_kind = ? AND star_rating = 5",
                rusqlite::params![game_kind],
                |row| row.get(0),
            )
            .ok();

        let current_pity: i32 = if let Some(max_id) = latest_five_star_id {
            conn.query_row(
                "SELECT COUNT(*) FROM gacha_records WHERE game_kind = ? AND id > ?",
                rusqlite::params![game_kind, max_id],
                |row| row.get(0),
            )?
        } else {
            conn.query_row(
                "SELECT COUNT(*) FROM gacha_records WHERE game_kind = ?",
                rusqlite::params![game_kind],
                |row| row.get(0),
            )?
        };

        let avg_pulls = if five_star > 0 {
            total as f64 / five_star as f64
        } else {
            0.0
        };

        Ok(GachaStats {
            total_pulls: total,
            five_star_count: five_star,
            lost_count: lost,
            current_pity,
            avg_pulls_per_five_star: avg_pulls,
        })
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .map_err(|e: anyhow::Error| format!("{:#}", e))
}

#[tauri::command]
pub async fn import_gacha_screenshot(
    pool: tauri::State<'_, DbPool>,
    image_path: String,
    game_kind: String,
) -> TauriResult<GachaImportResult> {
    let kind = GameKind::from_str(&game_kind)
        .ok_or_else(|| format!("Invalid game_kind: {}", game_kind))?;
    let features = kind.features();

    let img_bytes = std::fs::read(&image_path)
        .map_err(|e| format!("Failed to read image: {}", e))?;

    let pool = pool.inner().clone();
    let game_kind_clone = game_kind.clone();

    let result = tokio::task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| format!("DB error: {}", e))?;

        // 1) 全图 OCR 只做标题检测
        let ocr_lines = crate::ocr::ocr_image(&img_bytes)
            .map_err(|e| format!("OCR failed: {}", e))?;
        let clean: Vec<String> = ocr_lines.iter()
            .map(|l| l.chars().filter(|c| !c.is_whitespace()).collect())
            .collect();

        let has_title = clean.iter().any(|line| {
            features.title_keywords.iter().any(|kw| line.contains(kw))
        });
        if !has_title {
            return Err("截图不是抽卡记录页面，请确认截图包含标题".to_string());
        }
        eprintln!("[IMPORT] Title detected, parsing rows...");

        // 2) 逐行裁剪 + OCR 提取数据（避免 OCR 按列读表的问题）
        let img = image::load_from_memory(&img_bytes)
            .map_err(|e| format!("Image decode error: {}", e))?;
        let (w, h) = img.dimensions();

        // 行区域：假设记录表格在标题下 10%-90% 区域
        let row_data_y = (h as f64 * 0.10) as u32;
        let row_data_h = (h as f64 * 0.80) as u32;
        let row_x = (w as f64 * 0.02) as u32;
        let row_w = (w as f64 * 0.96) as u32;
        let row_h = (h as f64 * 0.06) as u32;  // 每行高度约 6%
        let max_rows = 15;

        let mut imported = 0usize;
        let mut duplicates = 0usize;
        let kind_str = &game_kind_clone;

        for i in 0..max_rows {
            let y = row_data_y + i * row_h;
            if y + row_h > row_data_y + row_data_h { break; }

            let row = img.crop_imm(row_x, y, row_w, row_h);
            let mut buf = std::io::Cursor::new(Vec::new());
            if row.write_to(&mut buf, image::ImageFormat::Png).is_err() { continue; }

            let lines = match crate::ocr::ocr_image(buf.get_ref()) {
                Ok(l) => l,
                Err(_) => continue,
            };
            // 去空格
            let lines: Vec<String> = lines.iter()
                .map(|l| l.chars().filter(|c| !c.is_whitespace()).collect())
                .collect();

            if lines.len() < 3 { continue; }  // 至少要有物品名

            // 尝试找物品名（跳过"对象类型""光锥"等表头词）
            let item_name = lines.iter()
                .find(|l| {
                    let t = l.trim();
                    !t.is_empty()
                        && t != "对象类型" && t != "对象名称"
                        && t != "跃迁类型" && t != "跃迁时间"
                        && t != "光锥" && t != "角色"
                        && t != "角色活动跃迁" && t != "光锥活动跃迁"
                        && !t.contains("可在本页面")
                        && !t.contains("历史记录")
                })
                .map(|s| s.trim().to_string());

            let item_name = match item_name {
                Some(n) => crate::ocr::normalize_item_name(&n, features.name_normalizations),
                None => continue,
            };

            // 尝试找日期
            let record_date = lines.iter()
                .find(|l| l.contains('-') || l.contains('·'))
                .map(|s| s.trim().to_string())
                .unwrap_or_default();

            if record_date.is_empty() { continue; }
            // 标准化日期格式：OCR 常用 · 代替 -
            let record_date = record_date.replace('·', "-").replace(':', ":");

            // 星级判断：5星附近有"5"标记，3星光锥常用名硬编码
            let star_rating = if lines.iter().any(|l| l.contains('5') || l.contains('五')) {
                5
            } else if ["轮契", "齐颂", "蕃息", "嘉果"].contains(&item_name.as_str()) {
                3
            } else {
                4
            };

            // 去重
            let exists: bool = conn
                .query_row(
                    "SELECT COUNT(*) > 0 FROM gacha_records
                     WHERE game_kind = ? AND item_name = ? AND record_date = ? AND star_rating = ?",
                    rusqlite::params![kind_str, &item_name, &record_date, star_rating],
                    |row| row.get(0),
                )
                .unwrap_or(false);

            if exists {
                duplicates += 1;
            } else {
                conn.execute(
                    "INSERT INTO gacha_records (game_kind, item_name, star_rating, record_date, is_won)
                     VALUES (?, ?, ?, ?, ?)",
                    rusqlite::params![kind_str, &item_name, star_rating, &record_date, star_rating < 5],
                )
                .map_err(|e| format!("Insert error: {}", e))?;
                imported += 1;
                eprintln!("[IMPORT]   Imported: {} ({}★, {})", item_name, star_rating, record_date);
            }
        }

        Ok(GachaImportResult { imported, duplicates })
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?;

    result
}

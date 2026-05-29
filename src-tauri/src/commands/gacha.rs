use serde::{Deserialize, Serialize};
use crate::db::DbPool;
use crate::error::TauriResult;
use crate::game::GameKind;
use anyhow::Context;

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

        // 1) OCR 全图，拿到文字 + 坐标
        let words = crate::ocr::ocr_image(&img_bytes)
            .map_err(|e| format!("OCR failed: {}", e))?;

        eprintln!("[IMPORT] OCR returned {} words", words.len());

        // 2) 按 Y 坐标分组 → 恢复行顺序（OCR 本身已从左到右、从上到下读）
        // 同一行内 Y 坐标差不超过 15px（容差）
        let mut rows: Vec<Vec<crate::ocr::OcrWord>> = Vec::new();
        let row_tolerance = 15.0;
        'outer: for word in words {
            let y_center = word.y + word.height / 2.0;
            for row in rows.iter_mut() {
                if let Some(first) = row.first() {
                    let row_y = first.y + first.height / 2.0;
                    if (y_center - row_y).abs() < row_tolerance {
                        row.push(word);
                        continue 'outer;
                    }
                }
            }
            rows.push(vec![word]);
        }

        // 每行按 X 排序，然后合成行文本
        let mut row_texts: Vec<String> = Vec::new();
        for row in rows.iter_mut() {
            row.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
            let text: String = row.iter().map(|w| w.text.trim()).collect();
            let clean: String = text.chars().filter(|c| !c.is_whitespace()).collect();
            if !clean.is_empty() {
                row_texts.push(clean);
            }
        }

        eprintln!("[IMPORT] Grouped into {} rows:", row_texts.len());
        for (i, t) in row_texts.iter().enumerate() {
            eprintln!("  Row {}: {:?}", i, t);
        }

        // 3) 找标题确认是抽卡页
        let has_title = row_texts.iter().any(|line| {
            features.title_keywords.iter().any(|kw| line.contains(kw))
        });
        if !has_title {
            return Err("截图不是抽卡记录页面，请确认截图包含标题".to_string());
        }
        eprintln!("[IMPORT] Title detected!");

        // 4) 找到各列表头位置，按列提取数据
        let col_headers = ["对象类型", "对象名称", "跃迁类型", "跃迁时间"];
        let col_values: Vec<Vec<String>> = col_headers.iter().map(|header| {
            if let Some(pos) = row_texts.iter().position(|l| l.contains(header)) {
                let next_pos = col_headers.iter()
                    .filter_map(|h| {
                        if h == header { return None; }
                        row_texts.iter().position(|l| l.contains(h))
                    })
                    .filter(|p| *p > pos)
                    .min()
                    .unwrap_or(row_texts.len());
                row_texts[pos + 1 .. next_pos].iter()
                    .filter(|l| {
                        !l.is_empty()
                        && !l.contains("历史记录")
                        && !l.contains("可在本页面")
                        && l != header
                    })
                    .cloned()
                    .collect()
            } else {
                vec![]
            }
        }).collect();

        if col_values.iter().any(|v| v.is_empty()) || col_values[0].len() != col_values[1].len() {
            eprintln!("[IMPORT] Table parsing failed, col lengths: {:?}",
                col_values.iter().map(|v| v.len()).collect::<Vec<_>>());
            return Err("无法解析表格结构，请确认截图包含完整的抽卡记录表格".to_string());
        }

        let num_rows = col_values[0].len();
        eprintln!("[IMPORT] Parsed {} rows", num_rows);

        // 5) 组装记录
        let mut imported = 0usize;
        let mut duplicates = 0usize;
        let kind_str = &game_kind_clone;

        for i in 0..num_rows {
            let obj_type = col_values[0][i].trim().to_string();
            let item_name_raw = col_values[1][i].trim();
            let date_raw = col_values[3][i].trim();

            let item_name = crate::ocr::normalize_item_name(item_name_raw, features.name_normalizations);

            let record_date = date_raw
                .replace('·', "-").replace('：', ":");
            let record_date = if record_date.len() > 10 && !record_date[10..].starts_with(' ') {
                format!("{} {}", &record_date[..10], &record_date[10..])
            } else {
                record_date
            };

            let star_rating = if obj_type == "角色" {
                if item_name.contains('5') || item_name.contains('五') { 5 } else { 4 }
            } else if ["轮契", "齐颂", "蕃息", "嘉果"].contains(&item_name.as_str()) {
                3
            } else if item_name.contains('5') || item_name.contains('五') {
                5
            } else {
                4
            };

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
                eprintln!("[IMPORT]   Imported: {} ({}★, {} | {})", item_name, star_rating, obj_type, record_date);
            }
        }

        Ok(GachaImportResult { imported, duplicates })
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?;

    result
}

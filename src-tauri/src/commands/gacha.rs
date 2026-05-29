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

        // 全图 OCR，不裁剪
        eprintln!("[IMPORT] Running full-image OCR ({} bytes)...", img_bytes.len());
        let all_lines = crate::ocr::ocr_image(&img_bytes)
            .map_err(|e| format!("OCR failed: {}", e))?;

        eprintln!("[IMPORT] OCR returned {} lines", all_lines.len());

        // OCR 会在中文字间加空格，全部去掉
        let all_lines: Vec<String> = all_lines.iter()
            .map(|l| l.chars().filter(|c| !c.is_whitespace()).collect())
            .collect();

        for (i, line) in all_lines.iter().enumerate() {
            eprintln!("[IMPORT]   Line {}: {:?}", i, line);
        }

        // 检查是否有标题关键词
        let has_title = all_lines.iter().any(|line| {
            features.title_keywords.iter().any(|kw| line.contains(kw))
        });

        if !has_title {
            return Err("截图不是抽卡记录页面，请确认截图包含标题".to_string());
        }

        // 找到关键词所在行之后的内容
        let title_idx = all_lines.iter().position(|l| {
            features.title_keywords.iter().any(|kw| l.contains(kw))
        }).unwrap_or(0);

        let data_lines: Vec<&String> = all_lines.iter().skip(title_idx + 1).collect();
        eprintln!("[IMPORT] Data lines after title: {}", data_lines.len());

        let mut imported = 0usize;
        let mut duplicates = 0usize;
        let kind_str = &game_kind_clone;

        // 每 2~3 行为一条记录（日期、物品名、可能还有额外行）
        let mut i = 0;
        while i + 1 < data_lines.len() {
            let record_date = data_lines[i].trim().to_string();
            let item_line = &data_lines[i + 1];

            // 检查是否是日期格式（跳过非数据行）
            if record_date.len() < 6 && !record_date.contains('-') {
                i += 1;
                continue;
            }

            let item_name = crate::ocr::normalize_item_name(item_line, features.name_normalizations);

            let star_rating = if item_line.contains('5') || item_line.contains('五') {
                5
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
                let is_won = star_rating < 5;
                conn.execute(
                    "INSERT INTO gacha_records (game_kind, item_name, star_rating, record_date, is_won)
                     VALUES (?, ?, ?, ?, ?)",
                    rusqlite::params![kind_str, &item_name, star_rating, &record_date, is_won],
                )
                .map_err(|e| format!("Insert error: {}", e))?;
                imported += 1;
            }

            i += 2; // 下一条记录
        }

        Ok(GachaImportResult { imported, duplicates })
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?;

    result
}

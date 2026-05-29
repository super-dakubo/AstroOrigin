use serde::{Deserialize, Serialize};
use crate::db::DbPool;
use crate::error::TauriResult;
use anyhow::Context;

/// 星穹铁道常见角色/光锥名（OCR 模糊匹配修正用）
const STARRAIL_NAMES: &[&str] = &[
    "轮契", "齐颂", "蕃息", "嘉果", "素裳", "三月七", "丹恒", "希露瓦",
    "黑塔", "阿兰", "艾丝妲", "青雀", "停云", "驭空", "佩拉", "卢卡",
    "米沙", "雪衣", "寒鸦", "加拉赫",
    "希儿", "景元", "刃", "卡芙卡", "银狼", "罗刹",
    "布洛妮娅", "杰帕德", "克拉拉", "彦卿", "白露", "姬子", "瓦尔特",
    "虎克", "娜塔莎", "桑博", "桂乃芬",
];

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
    let img_bytes = std::fs::read(&image_path)
        .map_err(|e| format!("Failed to read image: {}", e))?;

    let pool = pool.inner().clone();
    let game_kind_clone = game_kind.clone();

    tokio::task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| format!("DB error: {}", e))?;

        // 获取图片尺寸
        // ── 1. OCR 全图 ──
        let all_words = crate::ocr::ocr_image(&img_bytes)
            .map_err(|e| format!("OCR failed: {}", e))?;
        eprintln!("[WARP] {} raw words", all_words.len());

        // ── 2. Y 聚类 → 分出所有行 ──
        let mut y_vals: Vec<f64> = all_words.iter().map(|w| w.y + w.height / 2.0).collect();
        y_vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mut y_clusters: Vec<Vec<f64>> = Vec::new();
        for yv in y_vals {
            let mut placed = false;
            for cl in y_clusters.iter_mut() {
                if (yv - cl[0]).abs() < 25.0 { cl.push(yv); placed = true; break; }
            }
            if !placed { y_clusters.push(vec![yv]); }
        }
        let mut row_centers: Vec<f64> = y_clusters.iter()
            .map(|cl| cl.iter().sum::<f64>() / cl.len() as f64)
            .collect();
        row_centers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        eprintln!("[WARP] {} rows: {:?}", row_centers.len(), row_centers);

        // ── 3. 对每行按 X 间隙分列，只取恰好 4 列的行 ──
        let half_span = if row_centers.len() > 1 { (row_centers[1] - row_centers[0]) / 2.0 } else { 30.0 };
        let mut table_rows: Vec<Vec<String>> = Vec::new();

        for ry in &row_centers {
            // 收集 Y 在此行的词
            let mut rw: Vec<&crate::ocr::OcrWord> = all_words.iter()
                .filter(|w| (w.y + w.height / 2.0 - ry).abs() < half_span)
                .collect();
            rw.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
            if rw.is_empty() { continue; }

            // 用 X 间隙切分列
            let mut cells: Vec<Vec<String>> = vec![vec![rw[0].text.trim().to_string()]];
            let mut prev_right = rw[0].x + rw[0].width;
            for w in rw.iter().skip(1) {
                if w.x - prev_right > 30.0 {
                    cells.push(vec![w.text.trim().to_string()]);
                } else {
                    cells.last_mut().unwrap().push(w.text.trim().to_string());
                }
                prev_right = w.x + w.width;
            }
            let merged: Vec<String> = cells.iter()
                .map(|c| c.join("").chars().filter(|ch| !ch.is_whitespace()).collect())
                .collect();
            if merged.len() == 4 {
                table_rows.push(merged);
            }
        }
        eprintln!("[WARP] {} rows split into 4 columns", table_rows.len());
        for (i, r) in table_rows.iter().enumerate() {
            eprintln!("  Row {}: {:?}", i, r);
        }

        // ── 4. 表格 = 6 行（1 表头 + 5 数据） ──
        // 表头特征：包含"对象类型"等词
        let header_idx = table_rows.iter().position(|r| {
            r[0].contains("对象类型") || r[0].contains("对象") && r[1].contains("名称")
        });

        let data_rows = if let Some(h) = header_idx {
            if h + 5 < table_rows.len() {
                &table_rows[h + 1 ..= h + 5]
            } else {
                &table_rows[h + 1 ..]
            }
        } else {
            &table_rows[..]
        };
        eprintln!("[WARP] Using {} data rows (header at {:?})", data_rows.len(), header_idx);

        // ── 5. 去重入库 ──
        let mut imported = 0usize;
        let mut duplicates = 0usize;
        let kind_str = &game_kind_clone;

        for cells in data_rows {
            // 补齐不足 4 列的（例如 OCR 漏了某格）
            let mut c: [&str; 4] = ["", "", "", ""];
            for (i, cell) in cells.iter().enumerate() {
                if i < 4 { c[i] = cell.trim(); }
            }

            let obj_type = c[0];
            let item_name = fuzzy_match_name(c[1]);
            let record_date = normalize_date(c[3]);

            // 星数：提示性赋值，用户可手动改
            let star_rating = if obj_type == "角色" { 4 } else { 3 };

            // 温和去重：只有名称 + 日期都非空才去重
            let exists = if !item_name.is_empty() && !record_date.is_empty() {
                conn.query_row(
                    "SELECT COUNT(*) > 0 FROM gacha_records
                     WHERE game_kind = ? AND item_name = ? AND record_date = ? AND star_rating = ?",
                    rusqlite::params![kind_str, &item_name, &record_date, star_rating],
                    |row| row.get(0),
                ).unwrap_or(false)
            } else {
                false
            };

            if exists {
                duplicates += 1;
            } else {
                conn.execute(
                    "INSERT INTO gacha_records (game_kind, item_name, star_rating, record_date, is_won)
                     VALUES (?, ?, ?, ?, ?)",
                    rusqlite::params![kind_str, &item_name, star_rating, &record_date, star_rating < 5],
                ).map_err(|e| format!("Insert error: {}", e))?;
                imported += 1;
                eprintln!("[IMPORT]   Imported: '{}' ({}★, {} | {})", item_name, star_rating, obj_type, record_date);
            }
        }

        Ok(GachaImportResult { imported, duplicates })
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))
    .and_then(|inner| inner)
}

#[tauri::command]
pub async fn update_gacha_record(
    pool: tauri::State<'_, DbPool>,
    id: i64,
    item_name: String,
    star_rating: i32,
    record_date: String,
    is_won: bool,
) -> TauriResult<bool> {
    let pool = pool.inner().clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| format!("DB error: {}", e))?;
        conn.execute(
            "UPDATE gacha_records SET item_name = ?, star_rating = ?, record_date = ?, is_won = ? WHERE id = ?",
            rusqlite::params![item_name, star_rating, record_date, is_won, id],
        ).map_err(|e| format!("Update error: {}", e))?;
        Ok(true)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub async fn delete_gacha_record(
    pool: tauri::State<'_, DbPool>,
    id: i64,
) -> TauriResult<bool> {
    let pool = pool.inner().clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| format!("DB error: {}", e))?;
        conn.execute("DELETE FROM gacha_records WHERE id = ?", rusqlite::params![id])
            .map_err(|e| format!("Delete error: {}", e))?;
        Ok(true)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

// ── 工具函数（模块级别，方便测试） ──
fn normalize_date(s: &str) -> String {
    let s = s.replace('·', "-").replace('：', ":")
        .chars().filter(|c| !c.is_whitespace()).collect::<String>();
    let colon_idx = s.chars().position(|c| c == ':');
    if let Some(idx) = colon_idx {
        if idx > 4 {
            let date_part: String = s.chars().take(idx - 2).collect();
            let time_part: String = s.chars().skip(idx - 2).collect();
            return format!("{} {}", date_part.trim(), time_part.trim());
        }
    }
    s
}

fn fuzzy_match_name(ocr: &str) -> String {
    let ocr_clean: String = ocr.chars().filter(|&c| !c.is_whitespace() && c != '·' && c != '-' && c != ':' && c != '：').collect();
    if ocr_clean.is_empty() { return ocr.to_string(); }
    let mut best = ocr.to_string();
    let mut best_len = 0usize;
    for &known in STARRAIL_NAMES {
        if ocr.contains(known) || known.contains(&ocr_clean) {
            if known.len() > best_len { best = known.to_string(); best_len = known.len(); }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use crate::ocr::OcrWord;

    /// 模拟 OCR 输出：基于用户截图的实际数据
    fn mock_starrail_words() -> Vec<OcrWord> {
        // 按行/列排布，x 从左到右递增，y 从上到下递增
        // 每行 y 间距 ~70px，每列 x 间距 ~140px
        let mut words = Vec::new();
        let rows = [
            // (x, y, text) — 头部信息行
            (50.0, 10.0, "0"), (80.0, 10.0, "0"), (110.0, 10.0, "0"),
            // 历史记录标题
            (20.0, 80.0, "查看详情"), (200.0, 80.0, "历史记录"),
            // 说明文字
            (20.0, 150.0, "可在本页面查询最近6个月的跃迁历史记录"),
            // 表头行
            (30.0, 220.0, "对"), (50.0, 220.0, "象"), (70.0, 220.0, "类"), (90.0, 220.0, "型"),
            (180.0, 220.0, "对"), (200.0, 220.0, "象"), (220.0, 220.0, "名"), (240.0, 220.0, "称"),
            (350.0, 220.0, "跃"), (370.0, 220.0, "迁"), (390.0, 220.0, "类"), (410.0, 220.0, "型"),
            (500.0, 220.0, "跃"), (520.0, 220.0, "迁"), (540.0, 220.0, "时"), (560.0, 220.0, "间"),
            // 数据行 1：光锥 | 嘉果 | 角色活动跃迁 | 2026·05·27 23:05:36
            (30.0, 290.0, "光"), (50.0, 290.0, "锥"),
            (180.0, 290.0, "嘉"), (200.0, 290.0, "果"),
            (350.0, 290.0, "角色活动跃迁"),
            (500.0, 290.0, "2026·05·27"), (570.0, 290.0, "23:05:36"),
            // 数据行 2：光锥 | 轮契 | 角色活动跃迁 | 2026·05·26 19:56:00
            (30.0, 360.0, "光"), (50.0, 360.0, "锥"),
            (180.0, 360.0, "轮"), (200.0, 360.0, "契"),
            (350.0, 360.0, "角色活动跃迁"),
            (500.0, 360.0, "2026·05·26"), (570.0, 360.0, "19:56:00"),
            // 数据行 3：光锥 | 齐颂 | 角色活动跃迁 | 2026·05·25 22:18:29
            (30.0, 430.0, "光"), (50.0, 430.0, "锥"),
            (180.0, 430.0, "齐"), (200.0, 430.0, "颂"),
            (350.0, 430.0, "角色活动跃迁"),
            (500.0, 430.0, "2026·05·25"), (570.0, 430.0, "22:18:29"),
            // 数据行 4：光锥 | 蕃息 | 角色活动跃迁 | 2026·05·24 10:28:49
            (30.0, 500.0, "光"), (50.0, 500.0, "锥"),
            (180.0, 500.0, "蕃"), (200.0, 500.0, "息"),
            (350.0, 500.0, "角色活动跃迁"),
            (500.0, 500.0, "2026·05·24"), (570.0, 500.0, "10:28:49"),
            // 数据行 5：角色 | 素裳 | 角色活动跃迁 | 2026·05·24 10:28:42
            (30.0, 570.0, "角"), (50.0, 570.0, "色"),
            (180.0, 570.0, "素"), (200.0, 570.0, "裳"),
            (350.0, 570.0, "角色活动跃迁"),
            (500.0, 570.0, "2026·05·24"), (570.0, 570.0, "10:28:42"),
            // 页码
            (500.0, 1000.0, "1"),
        ];
        for &(x, y, text) in &rows {
            words.push(OcrWord {
                text: text.to_string(),
                x, y,
                width: text.len() as f64 * 14.0,
                height: 20.0,
            });
        }
        words
    }

    #[test]
    fn test_normalize_date() {
        let result = super::normalize_date("2026·05·2723：05：36");
        assert_eq!(result, "2026-05-27 23:05:36");
    }

    #[test]
    fn test_fuzzy_match() {
        let result = super::fuzzy_match_name("轮契");
        assert!(!result.is_empty());
    }

    #[test]
    fn test_clustering() {
        let words = mock_starrail_words();
        let h = 1080.0;
        // 过滤 Y
        let top = h * 0.15;
        let bot = h * 0.85;
        let filtered: Vec<_> = words.into_iter()
            .filter(|w| (w.y + w.height / 2.0) > top && (w.y + w.height / 2.0) < bot)
            .collect();
        // 行聚类
        let mut rows: Vec<Vec<OcrWord>> = Vec::new();
        'outer: for word in filtered {
            let cy = word.y + word.height / 2.0;
            for r in rows.iter_mut() {
                let ry = r[0].y + r[0].height / 2.0;
                if (cy - ry).abs() < 30.0 { r.push(word); continue 'outer; }
            }
            rows.push(vec![word]);
        }
        // 忽略标题/说明行，只算数据行（含表头）
        assert!(rows.len() >= 6, "Expected 6+ rows, got {}", rows.len());
    }

}

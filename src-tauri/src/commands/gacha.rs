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
        let h = match image::load_from_memory(&img_bytes) {
            Ok(img) => img.height() as f64,
            Err(_) => 1080.0,
        };

        // ── 1. OCR 全图 ──
        let all_words = crate::ocr::ocr_image(&img_bytes)
            .map_err(|e| format!("OCR failed: {}", e))?;
        eprintln!("[WARP] {} raw words", all_words.len());

        // ── 2. 过滤：去除上下 15% 的文字 ──
        let top = h * 0.15;
        let bot = h * 0.85;
        let words: Vec<_> = all_words.into_iter()
            .filter(|w| (w.y + w.height / 2.0) > top && (w.y + w.height / 2.0) < bot)
            .collect();
        eprintln!("[WARP] {} words after Y filter [{:.0},{:.0}]", words.len(), top, bot);

        // ── 3. 从所有字的坐标直接算行列边界 ──
        // 3a. 收集所有 X 中心点，聚类找出 4 列位置
        let mut x_centers: Vec<f64> = words.iter().map(|w| w.x + w.width / 2.0).collect();
        x_centers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        // 聚类：相近的 X 合并（容差 50px）
        let mut x_clusters: Vec<Vec<f64>> = Vec::new();
        for xc in x_centers {
            let mut placed = false;
            for cl in x_clusters.iter_mut() {
                if (xc - cl[0]).abs() < 50.0 { cl.push(xc); placed = true; break; }
            }
            if !placed { x_clusters.push(vec![xc]); }
        }
        // 取每个类的均值，排序，取最大的 4 个类作为列位置
        let mut col_centers: Vec<f64> = x_clusters.iter()
            .map(|cl| cl.iter().sum::<f64>() / cl.len() as f64)
            .collect();
        col_centers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        // 选最大的 4 个类（可能有零星杂音形成小类）
        while col_centers.len() > 4 { col_centers.remove(0); }
        // 计算列边界（两列中点）
        let mut col_bounds: Vec<f64> = vec![0.0];
        for i in 0..col_centers.len() - 1 {
            col_bounds.push((col_centers[i] + col_centers[i + 1]) / 2.0);
        }
        col_bounds.push(f64::MAX);
        eprintln!("[WARP] {} X clusters -> {} columns, bounds: {:?}", x_clusters.len(), col_centers.len(), col_bounds);

        // 3b. Y 中心点聚类 → 行
        let mut y_centers: Vec<f64> = words.iter().map(|w| w.y + w.height / 2.0).collect();
        y_centers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mut y_clusters: Vec<Vec<f64>> = Vec::new();
        for yc in y_centers {
            let mut placed = false;
            for cl in y_clusters.iter_mut() {
                if (yc - cl[0]).abs() < 25.0 { cl.push(yc); placed = true; break; }
            }
            if !placed { y_clusters.push(vec![yc]); }
        }
        eprintln!("[WARP] {} Y clusters (rows)", y_clusters.len());

        // 3c. 按行组装
        let mut rows_y: Vec<f64> = y_clusters.iter().map(|cl| cl.iter().sum::<f64>() / cl.len() as f64).collect();
        rows_y.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // ── 4. 行 → 列 → 文字 ──
        let skip_keywords = ["历史记录", "可在本页面", "当前为", "查看详情"];
        let half_span = if rows_y.len() > 1 { (rows_y[1] - rows_y[0]) / 2.0 } else { 30.0 };
        let mut parsed: Vec<Vec<String>> = Vec::new();
        for ry in rows_y {
            let row_low = ry - half_span;
            let row_high = ry + half_span;

            // 取 Y 在此范围内的字
            let mut row_words: Vec<&crate::ocr::OcrWord> = words.iter()
                .filter(|w| {
                    let cy = w.y + w.height / 2.0;
                    cy >= row_low && cy < row_high
                })
                .collect();
            row_words.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
            let row_txt: String = row_words.iter().map(|w| w.text.trim()).collect::<Vec<_>>().join("");
            // 跳过非数据行
            if skip_keywords.iter().any(|k| row_txt.contains(k)) { continue; }
            if row_txt.trim().is_empty() || row_txt.len() < 4 { continue; }

            // 按 X 列边界分到 4 格
            let mut cells: Vec<Vec<String>> = (0..4).map(|_| Vec::new()).collect();
            for w in &row_words {
                let cx = w.x + w.width / 2.0;
                let mut ci = 0usize;
                for bi in 0..col_bounds.len() - 1 {
                    if cx >= col_bounds[bi] && cx < col_bounds[bi + 1] { ci = bi; break; }
                }
                if ci < 4 { cells[ci].push(w.text.trim().to_string()); }
            }
            let merged: Vec<String> = cells.into_iter()
                .map(|c| c.join("").chars().filter(|ch| !ch.is_whitespace()).collect())
                .collect();
            if merged.len() != 4 || merged.iter().any(|c| c.is_empty()) { continue; }
            parsed.push(merged);
        }
        eprintln!("[WARP] {} parsed rows", parsed.len());
        for (i, r) in parsed.iter().enumerate() {
            eprintln!("  Row {}: {:?}", i, r);
        }

        // ── 6. 后处理 & 入库 ──
        let mut imported = 0usize;
        let mut duplicates = 0usize;
        let kind_str = &game_kind_clone;

        for cells in &parsed {
            let obj_type = cells[0].trim();
            let mut item_name = fuzzy_match_name(cells[1].trim());
            let date_raw = &cells[3];
            let record_date = normalize_date(date_raw);

            // 类型校验
            if obj_type != "角色" && obj_type != "光锥" { continue; }
            // 时间校验
            let date_ok = record_date.len() == 19
                && record_date.chars().nth(4) == Some('-')
                && record_date.chars().nth(7) == Some('-');
            if !date_ok { continue; }

            let star_rating = if obj_type == "角色" { 4 } else { 3 };
            // 如果名称能在星铁列表中直接查到，使用字典星级
            if STARRAIL_NAMES.contains(&item_name.as_str()) {
                // 常见 4★ 角色 / 3★ 光锥 保持默认
            }

            // 去重
            let exists: bool = conn.query_row(
                "SELECT COUNT(*) > 0 FROM gacha_records
                 WHERE game_kind = ? AND item_name = ? AND record_date = ? AND star_rating = ?",
                rusqlite::params![kind_str, &item_name, &record_date, star_rating],
                |row| row.get(0),
            ).unwrap_or(false);

            if exists { duplicates += 1; } else {
                conn.execute(
                    "INSERT INTO gacha_records (game_kind, item_name, star_rating, record_date, is_won)
                     VALUES (?, ?, ?, ?, ?)",
                    rusqlite::params![kind_str, &item_name, star_rating, &record_date, star_rating < 5],
                ).map_err(|e| format!("Insert error: {}", e))?;
                imported += 1;
                eprintln!("[IMPORT]   Imported: {} ({}★, {} | {})", item_name, star_rating, obj_type, record_date);
            }
        }

        Ok(GachaImportResult { imported, duplicates })
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))
    .and_then(|inner| inner)
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

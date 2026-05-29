use serde::{Deserialize, Serialize};
use crate::db::DbPool;
use crate::error::TauriResult;
use crate::game::GameKind;
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
    let kind = GameKind::from_str(&game_kind)
        .ok_or_else(|| format!("Invalid game_kind: {}", game_kind))?;
    let features = kind.features();

    let img_bytes = std::fs::read(&image_path)
        .map_err(|e| format!("Failed to read image: {}", e))?;

    let pool = pool.inner().clone();
    let game_kind_clone = game_kind.clone();

    tokio::task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| format!("DB error: {}", e))?;

        // 获取图片尺寸
        let (orig_w, h) = match image::load_from_memory(&img_bytes) {
            Ok(img) => (img.width() as f64, img.height() as f64),
            Err(_) => (1920.0, 1080.0),
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

        // ── 3. 行聚类，Y 中心 30px 容差 ──
        let mut rows: Vec<Vec<crate::ocr::OcrWord>> = Vec::new();
        'outer: for word in words {
            let cy = word.y + word.height / 2.0;
            for r in rows.iter_mut() {
                let ry = r[0].y + r[0].height / 2.0;
                if (cy - ry).abs() < 30.0 { r.push(word); continue 'outer; }
            }
            rows.push(vec![word]);
        }
        // 每行按 X 排序
        for r in rows.iter_mut() { r.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal)); }
        eprintln!("[WARP] {} rows after clustering", rows.len());
        for (i, r) in rows.iter().enumerate() {
            let t: String = r.iter().map(|w| w.text.trim()).collect::<Vec<_>>().join(" ");
            eprintln!("  Row {}: {:?}", i, t);
        }

        // ── 4. 找表头行 ──
        let header_idx = rows.iter().position(|r| {
            let txt: String = r.iter().map(|w| w.text.trim()).collect();
            txt.contains("对象类型") && txt.contains("跃迁时间")
        });

        let col_bounds: Vec<f64> = if let Some(hi) = header_idx {
            let row = &rows[hi];
            // 拼合表头文本，记录每字的 X
            let concat: Vec<(f64, char)> = row.iter()
                .flat_map(|w| w.text.chars().map(move |c| (w.x, c)))
                .filter(|(_, c)| !c.is_whitespace())
                .collect();
            let full_text: String = concat.iter().map(|(_, c)| c).collect();
            // 在拼合文本中找"对象类型""对象名称""跃迁类型""跃迁时间"
            let labels = ["对象类型", "对象名称", "跃迁类型", "跃迁时间"];
            let mut boundaries: Vec<f64> = labels.iter().filter_map(|label| {
                full_text.find(label).map(|pos| concat[pos].0)
            }).collect();
            boundaries.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            boundaries.push(f64::MAX);
            boundaries
        } else {
            // 回退：硬编码列宽（基于 1920×1080 比例缩放）
            eprintln!("[WARP] Header not found, using fallback column widths");
            let scale = orig_w / 1920.0;
            vec![0.0, 140.0 * scale, 360.0 * scale, 540.0 * scale, f64::MAX]
        };
        eprintln!("[WARP] Column boundaries: {:?}", col_bounds);

        // ── 5. 遍历数据行，按 X 中心点分列 ──
        let skip_keywords = ["历史记录", "可在本页面", "对象类型", "跃迁记录", "当前为"];
        let mut parsed: Vec<Vec<String>> = Vec::new();
        for (ri, row) in rows.iter().enumerate() {
            if header_idx.is_some() && ri == header_idx.unwrap() { continue; }
            let txt: String = row.iter().map(|w| w.text.trim()).collect();
            if skip_keywords.iter().any(|k| txt.contains(k)) { continue; }
            if txt.trim().is_empty() || txt.len() <= 2 { continue; }

            let mut cells: Vec<Vec<String>> = (0..4).map(|_| Vec::new()).collect();
            for word in row {
                let cx = word.x + word.width / 2.0;
                let mut ci = 0usize;
                for bi in 0..col_bounds.len() - 1 {
                    if cx >= col_bounds[bi] && cx < col_bounds[bi + 1] { ci = bi; break; }
                }
                if ci < 4 { cells[ci].push(word.text.trim().to_string()); }
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
        fn normalize_date(s: &str) -> String {
            let s = s.replace('·', "-").replace('：', ":")
                .chars().filter(|c| !c.is_whitespace()).collect::<String>();
            // 找第一个 : 的位置（用 char 索引安全处理中文）
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
            let ocr_clean: Vec<char> = ocr.chars().filter(|&c| !c.is_whitespace() && c != '·' && c != '-' && c != ':' && c != '：').collect();
            if ocr_clean.is_empty() { return ocr.to_string(); }
            // 在已知列表中找最长公共子串匹配
            let mut best = ocr.to_string();
            let mut best_len = 0usize;
            for &known in STARRAIL_NAMES {
                // 简单前缀/包含匹配
                if ocr.contains(known) || known.contains(&ocr_clean.iter().collect::<String>().as_str()) {
                    if known.len() > best_len { best = known.to_string(); best_len = known.len(); }
                }
            }
            best
        }

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

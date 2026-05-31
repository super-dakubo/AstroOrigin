use serde::{Deserialize, Serialize};
use crate::db::DbPool;
use crate::error::TauriResult;
use anyhow::Context;
use image::GenericImageView;
use std::sync::OnceLock;

/// 全局 AppHandle，用于在 spawn_blocking 内发进度事件
static APP_HANDLE: OnceLock<tauri::AppHandle> = OnceLock::new();

pub fn init_app_handle(handle: tauri::AppHandle) {
    let _ = APP_HANDLE.set(handle);
}

fn emit_phase(file_idx: usize, file_total: usize, phase: &str, file: Option<&str>) {
    use tauri::Emitter;
    if let Some(handle) = APP_HANDLE.get() {
        let mut payload = serde_json::json!({
            "current": file_idx + 1,
            "total": file_total,
            "phase": phase,
        });
        if let Some(f) = file {
            payload["file"] = serde_json::Value::String(f.to_string());
        }
        let _ = handle.emit("import-progress", payload);
    }
}

/// 星穹铁道常见角色/光锥名（OCR 模糊匹配修正用）
const STARRAIL_NAMES: &[&str] = &[
    // 3★ 光锥
    "轮契", "齐颂", "蕃息", "嘉果",
    "锋镝", "物穰", "睿见",
    // 4★ 角色
    "素裳", "三月七", "丹恒", "希露瓦",
    "黑塔", "阿兰", "艾丝妲", "青雀", "停云", "驭空", "佩拉", "卢卡",
    "米沙", "雪衣", "寒鸦", "加拉赫",
    "虎克", "娜塔莎", "桑博", "桂乃芬",
    // 5★ 角色
    "希儿", "景元", "刃", "卡芙卡", "银狼", "罗刹",
    "布洛妮娅", "杰帕德", "克拉拉", "彦卿", "白露", "姬子", "瓦尔特",
];

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GachaRecord {
    pub id: i64,
    pub game_kind: String,
    pub item_name: String,
    pub item_type: String,
    pub star_rating: i32,
    pub record_date: String,
    pub is_won: bool,
    pub banner_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GachaRecordsResponse {
    pub records: Vec<GachaRecord>,
    pub total: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GachaStats {
    pub total_pulls: i64,
    pub five_star_count: i64,
    pub lost_count: i64,
    pub current_pity: i32,
    pub avg_pulls_per_five_star: f64,
    pub by_banner: Vec<BannerStats>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BannerStats {
    pub banner_type: String,
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
    page: Option<i64>,
    page_size: Option<i64>,
    banner: Option<String>,
    star_filter: Option<i32>,
    sort_by: Option<String>,
    sort_order: Option<String>,
) -> TauriResult<GachaRecordsResponse> {
    let page_num = page.unwrap_or(1).max(1);
    let limit = page_size.unwrap_or(20).clamp(1, 200);
    let offset = (page_num - 1) * limit;
    let pool = pool.inner().clone();

    tokio::task::spawn_blocking(move || {
        let conn = pool.get().context("Failed to get DB connection")?;

        // Build dynamic WHERE clause
        let mut conditions = vec!["game_kind = ?1".to_string()];
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        param_values.push(Box::new(game_kind.clone()));

        if let Some(ref b) = banner {
            if !b.is_empty() && b != "全部" {
                let idx = param_values.len() + 1;
                conditions.push(format!("banner_type LIKE ?{}", idx));
                param_values.push(Box::new(format!("%{}%", b)));
            }
        }
        if let Some(sf) = star_filter {
            if sf > 0 {
                let idx = param_values.len() + 1;
                conditions.push(format!("star_rating = ?{}", idx));
                param_values.push(Box::new(sf));
            }
        }

        let where_clause = conditions.join(" AND ");

        // Build dynamic ORDER BY
        let order_clause = match (sort_by.as_deref(), sort_order.as_deref()) {
            (Some("date"), Some("asc")) => "record_date ASC, id ASC".to_string(),
            (Some("star"), Some("asc")) => "star_rating ASC, record_date DESC".to_string(),
            (Some("star"), _) => "star_rating DESC, record_date DESC".to_string(),
            _ => "record_date DESC, id DESC".to_string(),
        };

        // COUNT query
        let count_sql = format!(
            "SELECT COUNT(*) FROM gacha_records WHERE {}",
            where_clause
        );
        let total: i64 = conn.query_row(
            &count_sql,
            rusqlite::params_from_iter(param_values.iter().map(|p| p.as_ref())),
            |row| row.get(0),
        ).context("Failed to count gacha records")?;

        // SELECT query — add LIMIT and OFFSET params
        // num_where_params 是 WHERE 条件参数个数，LIMIT/OFFSET 编号在其后
        let num_where_params = param_values.len();
        param_values.push(Box::new(limit));
        param_values.push(Box::new(offset));

        let limit_idx = num_where_params + 1;
        let offset_idx = num_where_params + 2;

        let select_sql = format!(
            "SELECT id, game_kind, item_name, item_type, star_rating, record_date, is_won, banner_type
             FROM gacha_records
             WHERE {}
             ORDER BY {}
             LIMIT ?{} OFFSET ?{}",
            where_clause, order_clause, limit_idx, offset_idx,
        );

        let mut stmt = conn.prepare(&select_sql)?;

        let records = stmt
            .query_map(
                rusqlite::params_from_iter(param_values.iter().map(|p| p.as_ref())),
                |row| {
                    Ok(GachaRecord {
                        id: row.get(0)?,
                        game_kind: row.get(1)?,
                        item_name: row.get(2)?,
                        item_type: row.get(3)?,
                        star_rating: row.get(4)?,
                        record_date: row.get(5)?,
                        is_won: row.get(6)?,
                        banner_type: row.get(7)?,
                    })
                },
            )?
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to collect records")?;

        Ok(GachaRecordsResponse { records, total })
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

        // 按 banner_type 分组统计
        let mut by_banner = Vec::new();
        let mut banner_stmt = conn.prepare(
            "SELECT banner_type, COUNT(*), SUM(CASE WHEN star_rating = 5 THEN 1 ELSE 0 END),
                    SUM(CASE WHEN star_rating = 5 AND is_won = 0 THEN 1 ELSE 0 END)
             FROM gacha_records
             WHERE game_kind = ?
             GROUP BY banner_type
             ORDER BY banner_type"
        )?;

        let banner_rows = banner_stmt.query_map(rusqlite::params![game_kind], |row| {
            let bt: String = row.get(0)?;
            let total: i64 = row.get(1)?;
            let five_star: i64 = row.get(2)?;
            let lost: i64 = row.get(3)?;
            Ok((bt, total, five_star, lost))
        })?
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to collect banner stats")?;

        for (bt, total, five_star, lost) in &banner_rows {
            let latest_five_id: Option<i64> = conn.query_row(
                "SELECT MAX(id) FROM gacha_records WHERE game_kind = ? AND star_rating = 5 AND banner_type = ?",
                rusqlite::params![game_kind, bt],
                |row| row.get(0),
            ).ok();

            let pity: i32 = if let Some(max_id) = latest_five_id {
                conn.query_row(
                    "SELECT COUNT(*) FROM gacha_records WHERE game_kind = ? AND id > ? AND banner_type = ?",
                    rusqlite::params![game_kind, max_id, bt],
                    |row| row.get(0),
                )?
            } else {
                conn.query_row(
                    "SELECT COUNT(*) FROM gacha_records WHERE game_kind = ? AND banner_type = ?",
                    rusqlite::params![game_kind, bt],
                    |row| row.get(0),
                )?
            };

            let avg = if *five_star > 0 { *total as f64 / *five_star as f64 } else { 0.0 };

            by_banner.push(BannerStats {
                banner_type: bt.clone(),
                total_pulls: *total,
                five_star_count: *five_star,
                lost_count: *lost,
                current_pity: pity,
                avg_pulls_per_five_star: avg,
            });
        }

        Ok(GachaStats {
            total_pulls: total,
            five_star_count: five_star,
            lost_count: lost,
            current_pity,
            avg_pulls_per_five_star: avg_pulls,
            by_banner,
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

    tokio::task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| format!("DB error: {}", e))?;
        process_one_screenshot(&conn, &img_bytes, &game_kind, 0, 0)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .map_err(|e| format!("{:#}", e))
}
fn process_one_screenshot(
    conn: &rusqlite::Connection,
    img_bytes: &[u8],
    game_kind: &str,
    file_idx: usize,
    file_total: usize,
) -> Result<GachaImportResult, String> {
    // ── 阶段 1: 文本检测 ──
    emit_phase(file_idx, file_total, "detect", None);
    let all_words = crate::ocr::ocr_image(img_bytes)
        .map_err(|e| format!("OCR failed: {}", e))?;
    eprintln!("[WARP] {} raw words", all_words.len());
    for (i, w) in all_words.iter().enumerate().take(30) {
        eprintln!("[WARP]   word[{}]: {:?} @ ({:.0},{:.0}) {}x{}",
            i, w.text, w.x, w.y, w.width, w.height);
    }

    // 加载图片用于颜色星级检测
    let img = image::load_from_memory(img_bytes)
        .map_err(|e| format!("Image decode failed: {}", e))?;

    // ── 阶段 2: 文字识别完成 ──
    emit_phase(file_idx, file_total, "recognize", None);

    // ── 2. Y 聚类 → 行 ──
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
    eprintln!("[WARP] {} rows", row_centers.len());

    // ── 阶段 3: 表格解析（表头检测 → 列对齐）──
    emit_phase(file_idx, file_total, "parse", None);

    // ── 3. 找表头行，提取 4 列名的 X 范围 ──
    let half_span = if row_centers.len() > 1 { (row_centers[1] - row_centers[0]) / 2.0 } else { 30.0 };
    let mut hdr_cols: Vec<(f64, f64)> = Vec::new();

    for ry in &row_centers {
        let mut rw: Vec<&crate::ocr::OcrWord> = all_words.iter()
            .filter(|w| (w.y + w.height / 2.0 - ry).abs() < half_span)
            .collect();
        rw.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
        let concat: String = rw.iter().map(|w| w.text.trim()).collect::<Vec<_>>().join("");
        if !concat.contains("对象类型") || !concat.contains("跃迁时间") { continue; }

        let gaps: Vec<(usize, f64)> = rw.windows(2).enumerate()
            .map(|(i, pair)| (i, pair[1].x - (pair[0].x + pair[0].width)))
            .filter(|(_, g)| *g > 0.0)
            .collect();
        let gap_count = gaps.len().min(3);
        let mut sorted = gaps.clone();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let mut split_idx: Vec<usize> = sorted.iter().take(gap_count).map(|(i, _)| *i).collect();
        split_idx.sort();

        if split_idx.len() < 3 && rw.len() > 4 {
            eprintln!("[WARP]   header row has {} words but only {} gaps, skipping", rw.len(), split_idx.len());
            break;
        }

        let mut start = 0usize;
        for si in &split_idx {
            let end = *si;
            let xs: Vec<f64> = rw[start..=end].iter().map(|w| w.x).collect();
            let x_min = xs.iter().cloned().fold(f64::MAX, f64::min);
            let x_max = rw[start..=end].iter()
                .map(|w| w.x + w.width).fold(f64::MIN, f64::max);
            hdr_cols.push((x_min, x_max));
            start = *si + 1;
        }
        if start < rw.len() {
            let xs: Vec<f64> = rw[start..].iter().map(|w| w.x).collect();
            let x_min = xs.iter().cloned().fold(f64::MAX, f64::min);
            let x_max = rw[start..].iter().map(|w| w.x + w.width).fold(f64::MIN, f64::max);
            hdr_cols.push((x_min, x_max));
        }
        eprintln!("[WARP] Header columns: {:?}", hdr_cols);
        break;
    }

    if hdr_cols.len() != 4 {
        return Err("无法定位表头4列，截图可能不完整".to_string());
    }

    // ── 4. 对表头下所有行，用表头列区间判断每个字属于哪列 ──
    // 每个元素：(合并后4列文本, 颜色星级)
    let mut data_rows: Vec<([String; 4], Option<i32>)> = Vec::new();
    let mut found_header = false;

    for ry in &row_centers {
        let mut rw: Vec<&crate::ocr::OcrWord> = all_words.iter()
            .filter(|w| (w.y + w.height / 2.0 - ry).abs() < half_span)
            .collect();
        rw.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
        let concat: String = rw.iter().map(|w| w.text.trim()).collect::<Vec<_>>().join("");

        if concat.contains("对象类型") { found_header = true; continue; }
        if !found_header { continue; }
        if rw.len() <= 2 { continue; }

        let mut cells: [Vec<String>; 4] = Default::default();
        // 保留列1（对象名称）的 OCR 词对象，用于采样文字颜色
        let mut col1_words: Vec<&crate::ocr::OcrWord> = Vec::new();

        for w in &rw {
            let cx = w.x + w.width / 2.0;
            let mut ci: Option<usize> = None;
            for (hi, &(col_start, col_end)) in hdr_cols.iter().enumerate() {
                if cx >= col_start - 5.0 && cx < col_end + 5.0 {
                    ci = Some(hi);
                    break;
                }
            }
            let ci = ci.unwrap_or_else(|| {
                hdr_cols.iter()
                    .enumerate()
                    .min_by(|(_, &(s1, _)), (_, &(s2, _))| {
                        (cx - s1).abs().partial_cmp(&(cx - s2).abs()).unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|(i, _)| i)
                    .unwrap_or(3)
            });
            cells[ci].push(w.text.trim().to_string());
            if ci == 1 { col1_words.push(w); }
        }

        // 采样对象名称文字颜色 → 星级
        let color_rating = star_rating_from_text(&img, &col1_words);

        let merged: [String; 4] = [
            cells[0].join(""), cells[1].join(""),
            cells[2].join(""), cells[3].join(""),
        ];
        data_rows.push((merged, color_rating));
        if data_rows.len() >= 5 { break; }
    }
    eprintln!("[WARP] {} data rows", data_rows.len());

    // ── 阶段 4: 入库 ──
    emit_phase(file_idx, file_total, "save", None);

    // ── 5. 入库（重复则更新星级）──
    let mut imported = 0usize;

    for (cells, color_rating) in &data_rows {
        let c: [&str; 4] = [
            cells[0].trim(), cells[1].trim(),
            cells[2].trim(), cells[3].trim(),
        ];

        let obj_type = c[0];
        let item_name = fuzzy_match_name(c[1]);
        let banner_type = c[2];
        let record_date = normalize_date(c[3]);

        // 优先级：文字颜色 > 已知物品名 > 类型启发式
        let star_rating = (*color_rating)
            .or_else(|| known_star_rating(&item_name))
            .unwrap_or_else(|| if obj_type == "角色" { 4 } else { 3 });

        if !item_name.is_empty() && !record_date.is_empty() {
            conn.execute(
                "INSERT INTO gacha_records (game_kind, item_name, item_type, star_rating, record_date, is_won, banner_type)
                 VALUES (?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT(game_kind, item_name, record_date) DO UPDATE SET
                   item_type = excluded.item_type,
                   star_rating = excluded.star_rating,
                   is_won = excluded.is_won,
                   banner_type = excluded.banner_type",
                rusqlite::params![game_kind, &item_name, obj_type, star_rating, &record_date, star_rating < 5, banner_type],
            ).map_err(|e| format!("Insert error: {}", e))?;
            imported += 1;
            eprintln!("[IMPORT]   '{}' ({}★, {} | {})", item_name, star_rating, obj_type, record_date);
        }
    }

    Ok(GachaImportResult { imported, duplicates: 0 })
}

#[tauri::command]
pub async fn import_gacha_screenshots(
    pool: tauri::State<'_, DbPool>,
    app_handle: tauri::AppHandle,
    image_paths: Vec<String>,
    game_kind: String,
) -> TauriResult<GachaImportResult> {
    use tauri::Emitter;
    let pool = pool.inner().clone();
    let total = image_paths.len();

    tokio::task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| format!("DB error: {}", e))?;
        let mut total_imported = 0usize;
        let mut total_duplicates = 0usize;

        for (idx, path) in image_paths.iter().enumerate() {
            eprintln!("[BATCH] [{}/{}] Processing: {}", idx + 1, total, path);

            let img_bytes = match std::fs::read(path) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("[BATCH]   Skip: read error {}", e);
                    total_duplicates += 1;
                    continue;
                }
            };

            match process_one_screenshot(&conn, &img_bytes, &game_kind, idx, total) {
                Ok(r) => {
                    total_imported += r.imported;
                    total_duplicates += r.duplicates;
                    eprintln!("[BATCH]   +{} imported, {} dupes", r.imported, r.duplicates);
                }
                Err(e) => {
                    eprintln!("[BATCH]   Skip: {}", e);
                    total_duplicates += 1;
                }
            }
        }

        // 完成事件
        let _ = app_handle.emit("import-progress", serde_json::json!({
            "current": total,
            "total": total,
            "done": true,
        }));

        Ok(GachaImportResult { imported: total_imported, duplicates: total_duplicates })
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
    item_type: String,
    star_rating: i32,
    record_date: String,
    is_won: bool,
    banner_type: String,
) -> TauriResult<bool> {
    let pool = pool.inner().clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| format!("DB error: {}", e))?;
        let result = conn.execute(
            "UPDATE gacha_records SET item_name = ?, item_type = ?, star_rating = ?, record_date = ?, is_won = ?, banner_type = ? WHERE id = ?",
            rusqlite::params![item_name, item_type, star_rating, record_date, is_won, banner_type, id],
        );
        match result {
            Ok(_) => Ok(true),
            Err(rusqlite::Error::SqliteFailure(e, _)) if e.code == rusqlite::ErrorCode::ConstraintViolation => {
                Err("保存失败：相同日期+物品的记录已存在".to_string())
            }
            Err(e) => Err(format!("Update error: {}", e)),
        }
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
    // 先替换常见 OCR 错误字符
    let s = s.replace('·', "-").replace('：', ":").replace('厶', "4");
    // 去掉 PP-OCRv4 的 [UNK] 标记和空格
    let s = s.replace("[UNK]", "");
    let compact: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    eprintln!("[DATE] normalize: {:?} -> {:?}", s, compact);

    // 如果已经是完整格式 YYYY-MM-DD HH:MM:SS，直接返回
    if compact.len() == 19
        && compact.chars().nth(4) == Some('-')
        && compact.chars().nth(7) == Some('-')
        && compact.chars().nth(13) == Some(':')
    {
        let result = format!("{} {}",
            &compact[..10],  // YYYY-MM-DD
            &compact[11..]   // HH:MM:SS
        );
        eprintln!("[DATE] already formatted: {:?}", result);
        return result;
    }

    // 尝试找时间分隔符来拆分日期和时间
    if let Some(idx) = compact.find(|c: char| c == ':') {
        if idx > 4 {
            // idx 是字节索引，但对于 ASCII 数字来说等于字符索引
            let date_part = &compact[..idx - 2];
            let time_part = &compact[idx - 2..];
            let result = format!("{} {}", date_part.trim(), time_part.trim());
            eprintln!("[DATE] split at {} -> {:?}", idx, result);
            return result;
        }
    }
    // 兜底：原样返回
    eprintln!("[DATE] no split needed, keeping: {:?}", compact);
    compact
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

/// 从对象名称的文字颜色判断星级
/// 崩铁抽卡记录：5★橙/金、4★紫、3★黑
fn star_rating_from_text(img: &image::DynamicImage, col1_words: &[&crate::ocr::OcrWord]) -> Option<i32> {
    if col1_words.is_empty() {
        return None;
    }
    let (w, h) = img.dimensions();

    // 在整个词框区域内均匀采样，找最有色彩的像素
    // 中文词框中心可能有空心区域，需要扩大覆盖
    let mut best_r = 0u32;
    let mut best_g = 0u32;
    let mut best_b = 0u32;
    let mut best_sat = 0u32; // 饱和度 = max(R,G,B) - min(R,G,B)
    let mut sampled = 0u32;

    for word in col1_words {
        let x0 = word.x.max(0.0) as u32;
        let y0 = word.y.max(0.0) as u32;
        let x1 = ((word.x + word.width).min((w - 1) as f64).max(0.0)) as u32;
        let y1 = ((word.y + word.height).min((h - 1) as f64).max(0.0)) as u32;

        // 以 6px 步长遍历整个词框
        let mut yy = y0;
        while yy <= y1 {
            let mut xx = x0;
            while xx <= x1 {
                let p = img.get_pixel(xx, yy);
                let r = p[0] as u32;
                let g = p[1] as u32;
                let b = p[2] as u32;
                let mx = r.max(g).max(b);
                let mn = r.min(g).min(b);
                let sat = mx - mn;
                if sat > best_sat {
                    best_sat = sat;
                    best_r = r;
                    best_g = g;
                    best_b = b;
                }
                sampled += 1;
                xx += 6;
            }
            yy += 6;
        }
    }

    eprintln!("[COLOR] name text: RGB({},{},{}) sat={} ({} samples)", best_r, best_g, best_b, best_sat, sampled);

    // 彩色文字（高饱和度）才按颜色判断
    if best_sat > 40 {
        // 5★ 橙/金色: R 主导
        if best_r > best_b + 40 && best_r > 160 {
            return Some(5);
        }
        // 4★ 紫色: B 主导
        if best_b > best_r && best_b > 120 && best_r > 60 {
            return Some(4);
        }
    }

    // 3★ 黑色文字：低饱和度，RGB 均低
    // 用亮度区分黑色文字 vs 误采到浅色背景
    let brightness = best_r + best_g + best_b;
    if brightness < 300 {
        return Some(3);
    }

    None
}

/// 已知物品名 → 星级映射（颜色检测失败时回退）
fn known_star_rating(name: &str) -> Option<i32> {
    match name {
        // 5★ 角色
        "希儿" | "景元" | "刃" | "卡芙卡" | "银狼" | "罗刹"
        | "布洛妮娅" | "杰帕德" | "克拉拉" | "彦卿" | "白露" | "姬子" | "瓦尔特" => Some(5),
        // 4★ 角色
        "素裳" | "三月七" | "丹恒" | "希露瓦" | "黑塔" | "阿兰" | "艾丝妲"
        | "青雀" | "停云" | "驭空" | "佩拉" | "卢卡" | "米沙" | "雪衣"
        | "寒鸦" | "加拉赫" | "虎克" | "娜塔莎" | "桑博" | "桂乃芬" => Some(4),
        // 3★ 光锥
        "轮契" | "齐颂" | "蕃息" | "嘉果" | "锋镝" | "物穰" | "睿见" => Some(3),
        _ => None,
    }
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
        assert_eq!(super::normalize_date("2026·05·2723：05：36"), "2026-05-27 23:05:36");
        assert_eq!(super::normalize_date("2026-05-27 23:05:36"), "2026-05-27 23:05:36");
        assert_eq!(super::normalize_date("2026-05-2723:05:36"), "2026-05-27 23:05:36");
        // OCR 误识别
        assert_eq!(super::normalize_date("2026·05·2厶23：05：36"), "2026-05-24 23:05:36");
        // PP-OCRv4 [UNK] 标记
        assert_eq!(super::normalize_date("2026-05-27[UNK]23:05:36"), "2026-05-27 23:05:36");
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

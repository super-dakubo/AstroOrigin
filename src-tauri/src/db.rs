use anyhow::{Context, Result};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

pub type DbPool = Pool<SqliteConnectionManager>;

pub fn init_pool(db_path: &str) -> Result<DbPool> {
    let manager = SqliteConnectionManager::file(db_path);
    let pool = Pool::builder()
        .max_size(4)
        .build(manager)
        .context("Failed to create database pool")?;

    let conn = pool.get().context("Failed to get connection for migration")?;
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS gacha_records (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            game_kind   TEXT NOT NULL,
            item_name   TEXT NOT NULL,
            star_rating INTEGER NOT NULL,
            record_date TEXT NOT NULL,
            is_won      BOOLEAN DEFAULT 1,
            created_at  TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_gacha_game_date
            ON gacha_records(game_kind, record_date);

        CREATE UNIQUE INDEX IF NOT EXISTS idx_gacha_dedup
            ON gacha_records(game_kind, item_name, record_date);

        CREATE TABLE IF NOT EXISTS playtime_records (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            game_kind   TEXT NOT NULL,
            date        TEXT NOT NULL,
            minutes     INTEGER NOT NULL,
            created_at  TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_playtime_game_date
            ON playtime_records(game_kind, date);

        CREATE TABLE IF NOT EXISTS screenshot_tags (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            file_path   TEXT NOT NULL UNIQUE,
            tags        TEXT NOT NULL DEFAULT '[]',
            ocr_text    TEXT,
            created_at  TEXT NOT NULL DEFAULT (datetime('now'))
        );
        ",
    )
    .context("Failed to run database migrations")?;

    // Migration: 新增列（兼容旧数据库）
    for col in &["item_type", "banner_type"] {
        let has = conn
            .prepare(&format!("SELECT {} FROM gacha_records LIMIT 0", col))
            .is_ok();
        if !has {
            conn.execute_batch(&format!(
                "ALTER TABLE gacha_records ADD COLUMN {} TEXT NOT NULL DEFAULT '';",
                col
            ))
            .with_context(|| format!("Failed to add {} column", col))?;
        }
    }

    Ok(pool)
}

mod commands;
mod db;
mod error;
mod game;
mod ocr;
mod paddle;

use anyhow::Context;
use db::init_pool;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_dir = app
                .path()
                .app_data_dir()
                .context("Failed to get app data dir")?;
            std::fs::create_dir_all(&app_dir).context("Failed to create app data dir")?;
            let db_path = app_dir.join("companion.db");
            let pool = init_pool(
                db_path
                    .to_str()
                    .context("Database path contains invalid UTF-8")?,
            )
            .context("Failed to initialize database")?;

            app.manage(pool);
            commands::gacha::init_app_handle(app.handle().clone());
            Ok(())
        })

        .invoke_handler(tauri::generate_handler![
            commands::gacha::get_gacha_records,
            commands::gacha::get_gacha_stats,
            commands::gacha::import_gacha_screenshot,
            commands::gacha::import_gacha_screenshots,
            commands::gacha::delete_gacha_record,
            commands::gacha::update_gacha_record,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

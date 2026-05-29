use anyhow::Result;

/// 将 anyhow::Error 转为 Tauri 可接受的 String
pub fn to_tauri_err(e: anyhow::Error) -> String {
    format!("{:#}", e)
}

/// Tauri command 返回类型
pub type TauriResult<T> = Result<T, String>;

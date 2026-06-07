use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::Manager;

use crate::error::TauriResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameConfig {
    pub log_dirs: Vec<String>,
    pub api_url: String,
    #[serde(default)]
    pub extra_params: String,
    pub gacha_types: HashMap<String, String>,
    #[serde(default)]
    pub authkey: Option<String>,
    #[serde(default)]
    pub authkey_expires_at: Option<String>,
}

impl GameConfig {
    /// 检查缓存的 authkey 是否未过期（24h 有效期）
    pub fn is_authkey_valid(&self) -> bool {
        let Some(expires) = &self.authkey_expires_at else { return false };
        let Some(_) = &self.authkey else { return false };
        let Ok(expires_at) = chrono::DateTime::parse_from_rfc3339(expires) else {
            return false;
        };
        chrono::Utc::now() < expires_at
    }

    /// 设置 authkey + 24h 过期时间
    pub fn set_authkey(&mut self, authkey: String) {
        let expires = chrono::Utc::now() + chrono::Duration::hours(24);
        self.authkey = Some(authkey);
        self.authkey_expires_at = Some(expires.to_rfc3339());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GachaConfig {
    pub games: HashMap<String, GameConfig>,
}

fn default_genshin() -> GameConfig {
    let mut gacha_types = HashMap::new();
    gacha_types.insert("100".into(), "新手祈愿".into());
    gacha_types.insert("200".into(), "常驻祈愿".into());
    gacha_types.insert("301".into(), "角色活动祈愿".into());
    gacha_types.insert("302".into(), "武器活动祈愿".into());
    gacha_types.insert("500".into(), "集录祈愿".into());

    GameConfig {
        log_dirs: vec![
            "%USERPROFILE%/AppData/LocalLow/miHoYo/原神".into(),
            "%LOCALAPPDATA%/Genshin Impact".into(),
        ],
        api_url: "https://public-operation-hk4e.mihoyo.com/gacha_info/api/getGachaLog".into(),
        extra_params: "region=cn_gf01&game_biz=hk4e_cn".into(),
        gacha_types,
        authkey: None,
        authkey_expires_at: None,
    }
}

fn default_starrail() -> GameConfig {
    let mut gacha_types = HashMap::new();
    gacha_types.insert("1".into(), "常驻跃迁".into());
    gacha_types.insert("2".into(), "新手跃迁".into());
    gacha_types.insert("11".into(), "角色活动跃迁".into());
    gacha_types.insert("12".into(), "光锥活动跃迁".into());

    GameConfig {
        log_dirs: vec![
            "%USERPROFILE%/AppData/LocalLow/miHoYo/崩坏：星穹铁道".into(),
            "%LOCALAPPDATA%/StarRail".into(),
        ],
        api_url:
            "https://public-operation-hkrpg.mihoyo.com/common/hkrpg_gacha_record/api/getGachaLog"
                .into(),
        extra_params: "region=prod_gf_cn&game_biz=hkrpg_cn".into(),
        gacha_types,
        authkey: None,
        authkey_expires_at: None,
    }
}

pub fn default_config() -> GachaConfig {
    let mut games = HashMap::new();
    games.insert("genshin".into(), default_genshin());
    games.insert("starrail".into(), default_starrail());
    GachaConfig { games }
}

fn config_path(app_data_dir: &std::path::Path) -> PathBuf {
    app_data_dir.join("gacha_config.json")
}

/// 初始化配置文件（不存在时写入默认值）
pub fn init_config(app_data_dir: &std::path::Path) -> anyhow::Result<()> {
    let path = config_path(app_data_dir);
    if !path.exists() {
        let config = default_config();
        let json = serde_json::to_string_pretty(&config)?;
        std::fs::write(&path, &json)?;
    }
    Ok(())
}

/// 读取配置，缺字段回退默认值
pub fn load_config(app_data_dir: &std::path::Path) -> anyhow::Result<GachaConfig> {
    let path = config_path(app_data_dir);
    if !path.exists() {
        init_config(app_data_dir)?;
    }
    let content = std::fs::read_to_string(&path)?;
    let config: GachaConfig = serde_json::from_str(&content)?;
    Ok(config)
}

/// 保存配置到文件
pub fn save_config(app_data_dir: &std::path::Path, config: &GachaConfig) -> anyhow::Result<()> {
    let path = config_path(app_data_dir);
    let json = serde_json::to_string_pretty(config)?;
    std::fs::write(&path, &json)?;
    Ok(())
}

/// 展开路径中的 %USERPROFILE% / %LOCALAPPDATA% 等环境变量
pub fn expand_env_vars(s: &str) -> String {
    let s = s.replace("%USERPROFILE%", &std::env::var("USERPROFILE").unwrap_or_default());
    let s = s.replace("%LOCALAPPDATA%", &std::env::var("LOCALAPPDATA").unwrap_or_default());
    s
}

#[tauri::command]
pub fn get_gacha_config(app_handle: tauri::AppHandle) -> TauriResult<GachaConfig> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("获取应用数据目录失败: {}", e))?;
    load_config(&app_dir).map_err(|e| format!("{:#}", e))
}

#[tauri::command]
pub fn save_gacha_config(
    app_handle: tauri::AppHandle,
    config: GachaConfig,
) -> TauriResult<()> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("获取应用数据目录失败: {}", e))?;
    let path = config_path(&app_dir);
    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("序列化配置失败: {}", e))?;
    std::fs::write(&path, &json).map_err(|e| format!("写入配置文件失败: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn reset_gacha_config(app_handle: tauri::AppHandle, game: String) -> TauriResult<GachaConfig> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("获取应用数据目录失败: {}", e))?;
    let path = config_path(&app_dir);

    // 读现有配置
    let mut config = if path.exists() {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("读取配置文件失败: {}", e))?;
        serde_json::from_str(&content).unwrap_or_else(|_| default_config())
    } else {
        default_config()
    };

    // 重置指定游戏的配置为默认值
    match game.as_str() {
        "genshin" => {
            config.games.insert("genshin".into(), default_genshin());
        }
        "starrail" => {
            config.games.insert("starrail".into(), default_starrail());
        }
        _ => {}
    }

    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("序列化配置失败: {}", e))?;
    std::fs::write(&path, &json).map_err(|e| format!("写入配置文件失败: {}", e))?;
    Ok(config)
}

//! 用户设置：外观、语言、GitLab group 地址、Access Token、开机自启、关闭到托盘、启动检查更新。

use serde::{Deserialize, Serialize};
use std::fs;
use tauri::AppHandle;

fn default_true() -> bool {
    true
}

fn default_theme() -> String {
    "teal".into()
}

fn default_locale() -> String {
    "zh-CN".into()
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GeneralSettings {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_locale")]
    pub locale: String,
    #[serde(default)]
    pub launch_at_login: bool,
    #[serde(default = "default_true")]
    pub close_to_tray: bool,
    #[serde(default = "default_true")]
    pub auto_check_update: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            locale: default_locale(),
            launch_at_login: false,
            close_to_tray: true,
            auto_check_update: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct GitlabSettings {
    #[serde(default)]
    pub access_token: String,
    #[serde(default)]
    pub group_url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default)]
    pub general: GeneralSettings,
    #[serde(default)]
    pub gitlab: GitlabSettings,
}

pub fn settings_file_exists() -> Result<bool, String> {
    Ok(crate::platform::settings_file()?.is_file())
}

pub fn load_settings() -> Result<AppSettings, String> {
    let path = crate::platform::settings_file()?;
    if !path.is_file() {
        return Ok(AppSettings::default());
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("读取配置失败: {e}"))
}

pub fn save_settings(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let path = crate::platform::settings_file()?;
    let text = serde_json::to_string_pretty(settings).map_err(|e| format!("序列化配置失败: {e}"))?;
    fs::write(&path, text).map_err(|e| e.to_string())?;
    sync_launch_at_login(app, settings.general.launch_at_login)?;
    Ok(())
}

/// 首次启动：写入默认配置。已有文件时直接读取。
pub fn initialize_settings(app: &AppHandle) -> Result<AppSettings, String> {
    if settings_file_exists()? {
        return load_settings();
    }
    let settings = AppSettings::default();
    save_settings(app, &settings)?;
    Ok(settings)
}

pub fn sync_launch_at_login(app: &AppHandle, enabled: bool) -> Result<(), String> {
    #[cfg(desktop)]
    {
        use tauri_plugin_autostart::ManagerExt;
        let launcher = app.autolaunch();
        let result = if enabled {
            launcher.enable()
        } else {
            launcher.disable()
        };
        if let Err(e) = result {
            eprintln!("[CentralDevOpsKit] sync launch_at_login failed: {e}");
        }
    }
    let _ = (app, enabled);
    Ok(())
}

pub fn close_to_tray_enabled() -> bool {
    load_settings()
        .map(|s| s.general.close_to_tray)
        .unwrap_or(true)
}

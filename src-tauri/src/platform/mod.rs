//! 用户配置目录。macOS / Linux：`~/.central-devops-kit/`；Windows：`%APPDATA%\central-devops-kit\`。

use std::path::PathBuf;

const DIR_NAME: &str = "central-devops-kit";

/// 返回用户配置目录（确保已创建）。
pub fn config_dir() -> Result<PathBuf, String> {
    #[cfg(windows)]
    {
        let base = std::env::var("APPDATA").map_err(|_| "无法获取 APPDATA".to_string())?;
        let dir = PathBuf::from(base).join(DIR_NAME);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        return Ok(dir);
    }
    #[cfg(not(windows))]
    {
        let home = std::env::var("HOME").map_err(|_| "无法获取 HOME 目录".to_string())?;
        let dir = PathBuf::from(home).join(format!(".{DIR_NAME}"));
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        Ok(dir)
    }
}

/// 设置文件完整路径。
pub fn settings_file() -> Result<PathBuf, String> {
    Ok(config_dir()?.join("settings.json"))
}

//! Tauri IPC 薄封装。业务以后加在独立模块，这里只转发。

use crate::settings::{self, AppSettings};
use tauri::AppHandle;

#[tauri::command]
pub fn runtime_settings_exists() -> Result<bool, String> {
    settings::settings_file_exists()
}

#[tauri::command]
pub fn load_runtime_settings() -> Result<AppSettings, String> {
    settings::load_settings()
}

#[tauri::command]
pub fn save_runtime_settings(app: AppHandle, settings: AppSettings) -> Result<(), String> {
    settings::save_settings(&app, &settings)
}

#[tauri::command]
pub fn initialize_runtime_settings(app: AppHandle) -> Result<AppSettings, String> {
    settings::initialize_settings(&app)
}

#[tauri::command]
pub fn exit_app(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub async fn gitlab_load_variables(
    token: String,
    group_url: String,
    page: Option<u32>,
) -> crate::gitlab::GitlabLoadResult {
    crate::gitlab::load_variables(token, group_url, page.unwrap_or(1)).await
}

#[tauri::command]
pub async fn gitlab_delete_variable(
    token: String,
    group_url: String,
    key: String,
    environment_scope: String,
) -> crate::gitlab::GitlabActionResult {
    crate::gitlab::delete_variable(token, group_url, key, environment_scope).await
}

#[tauri::command]
pub async fn gitlab_create_variable(
    token: String,
    group_url: String,
    key: String,
    value: String,
    environment_scope: String,
    variable_type: String,
    is_protected: bool,
    is_masked: bool,
    expanded: bool,
    description: String,
) -> crate::gitlab::GitlabActionResult {
    crate::gitlab::create_variable(
        token,
        group_url,
        key,
        value,
        environment_scope,
        variable_type,
        is_protected,
        is_masked,
        expanded,
        description,
    )
    .await
}

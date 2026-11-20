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
pub async fn gitlab_list_projects(token: String, group_url: String) -> crate::gitlab::GitlabProjectsResult {
    crate::gitlab::list_projects(token, group_url).await
}

#[tauri::command]
pub async fn gitlab_list_pipelines(
    token: String,
    group_url: String,
    project_id: u64,
) -> crate::gitlab::GitlabPipelinesResult {
    crate::gitlab::list_pipelines(token, group_url, project_id).await
}

#[tauri::command]
pub async fn gitlab_list_branches(
    token: String,
    group_url: String,
    project_id: u64,
    search: String,
) -> crate::gitlab::GitlabBranchesResult {
    crate::gitlab::list_branches(token, group_url, project_id, search).await
}

#[tauri::command]
pub async fn gitlab_create_pipeline(
    token: String,
    group_url: String,
    project_id: u64,
    git_ref: String,
) -> crate::gitlab::GitlabPipelinesResult {
    crate::gitlab::create_pipeline(token, group_url, project_id, git_ref).await
}

#[tauri::command]
pub async fn gitlab_list_jobs(
    token: String,
    group_url: String,
    project_id: u64,
    pipeline_id: u64,
) -> crate::gitlab::GitlabJobsResult {
    crate::gitlab::list_jobs(token, group_url, project_id, pipeline_id).await
}

#[tauri::command]
pub async fn gitlab_run_job(
    token: String,
    group_url: String,
    project_id: u64,
    job_id: u64,
    action: String,
) -> crate::gitlab::GitlabJobsResult {
    crate::gitlab::run_job(token, group_url, project_id, job_id, action).await
}

#[tauri::command]
pub fn ports_lookup(query: String, fuzzy: bool) -> Result<Vec<crate::ports::PortOccupant>, String> {
    crate::ports::lookup_port(&query, fuzzy)
}

#[tauri::command]
pub fn ports_kill(pid: u32) -> Result<(), String> {
    crate::ports::kill_process(pid)
}

#[tauri::command]
pub fn gcp_inspect_project(path: String) -> crate::gcp::GcpInspectResult {
    crate::gcp::inspect_project(&path)
}

#[tauri::command]
pub fn gcp_add_whitelist_ip(
    path: String,
    cidr: String,
    comment: String,
    keys: Vec<String>,
) -> crate::gcp::GcpAddResult {
    crate::gcp::add_whitelist_ip(&path, &cidr, &comment, &keys)
}

#[tauri::command]
pub fn gcp_save_whitelist_files(
    path: String,
    files: Vec<crate::gcp::GcpSaveFile>,
) -> crate::gcp::GcpSaveResult {
    crate::gcp::save_whitelist_files(&path, &files)
}

#[tauri::command]
pub fn gcp_fetch_whitelist(app: tauri::AppHandle, path: String) -> crate::gcp::GcpApplyResult {
    std::thread::spawn(move || {
        let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::gcp::fetch_whitelist_from_gcp(&app, &path);
        }));
        if panicked.is_err() {
            crate::gcp::emit_fetch_done(&app, "查询过程发生异常，已停止");
        }
    });
    crate::gcp::GcpApplyResult { code: "started".into() }
}

#[tauri::command]
pub fn gcp_apply_whitelist(app: tauri::AppHandle, path: String, keys: Vec<String>) -> crate::gcp::GcpApplyResult {
    std::thread::spawn(move || {
        let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::gcp::apply_whitelist(&app, &path, &keys);
        }));
        if panicked.is_err() {
            crate::gcp::emit_apply_done(&app, "执行过程发生异常，已停止", "error");
        }
    });
    crate::gcp::GcpApplyResult { code: "started".into() }
}

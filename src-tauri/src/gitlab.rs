//! 读取用户配置的 GitLab group 下的 CI/CD 变量。
//! Group 地址和 Access Token 只从参数进入请求，不写入日志，也不写在程序里。

use serde::Serialize;
use std::time::Duration;

const FETCH_PAGE_SIZE: u32 = 100;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitlabVariableRow {
    pub key: String,
    pub value: String,
    pub hidden: bool,
    pub variable_type: String,
    pub is_protected: bool,
    pub is_masked: bool,
    pub expanded: bool,
    pub environment_scope: String,
    pub description: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitlabLoadResult {
    pub status: String,
    pub code: String,
    pub message: String,
    pub username: String,
    pub resource_kind: String,
    pub resource_name: String,
    pub resource_path: String,
    pub page: u32,
    pub per_page: u32,
    pub total: u32,
    pub variables: Vec<GitlabVariableRow>,
}

impl GitlabLoadResult {
    fn empty(status: &str, code: &str, message: impl Into<String>) -> Self {
        Self {
            status: status.into(),
            code: code.into(),
            message: message.into(),
            username: String::new(),
            resource_kind: String::new(),
            resource_name: String::new(),
            resource_path: String::new(),
            page: 1,
            per_page: FETCH_PAGE_SIZE,
            total: 0,
            variables: Vec::new(),
        }
    }

    fn ready(
        username: String,
        resource: Resource,
        page: VariablePage,
    ) -> Self {
        Self {
            status: "ok".into(),
            code: String::new(),
            message: String::new(),
            username,
            resource_kind: resource.kind,
            resource_name: resource.name,
            resource_path: resource.path,
            page: 1,
            per_page: FETCH_PAGE_SIZE,
            total: page.total,
            variables: page.rows,
        }
    }
}

struct Resource {
    kind: String,
    name: String,
    path: String,
    id: u64,
}

enum ApiError {
    Invalid,
    NotFound,
    Forbidden,
    BadResponse,
    Network(String),
    Status(String),
}

struct HttpText {
    status: u16,
    next_page: Option<u32>,
    total: Option<u32>,
    body: String,
}

struct VariablePage {
    total: u32,
    rows: Vec<GitlabVariableRow>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitlabProjectRow {
    pub id: u64,
    pub name: String,
    pub path: String,
    pub web_url: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitlabPipelineRow {
    pub id: u64,
    pub iid: u64,
    pub status: String,
    #[serde(rename = "ref")]
    pub git_ref: String,
    pub sha: String,
    pub source: String,
    pub web_url: String,
    pub created_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitlabBranchRow {
    pub name: String,
    pub default: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitlabProjectsResult {
    pub status: String,
    pub code: String,
    pub message: String,
    pub projects: Vec<GitlabProjectRow>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitlabPipelinesResult {
    pub status: String,
    pub code: String,
    pub message: String,
    pub pipelines: Vec<GitlabPipelineRow>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitlabBranchesResult {
    pub status: String,
    pub code: String,
    pub message: String,
    pub total: u32,
    pub branches: Vec<GitlabBranchRow>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitlabJobRow {
    pub id: u64,
    pub name: String,
    pub stage: String,
    pub status: String,
    pub when: String,
    pub allow_failure: bool,
    pub duration: Option<f64>,
    pub started_at: String,
    pub finished_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitlabJobsResult {
    pub status: String,
    pub code: String,
    pub message: String,
    pub jobs: Vec<GitlabJobRow>,
}

pub async fn list_projects(token: String, group_url: String) -> GitlabProjectsResult {
    let connected = match connect(&token, &group_url).await {
        Ok(value) => value,
        Err(result) => {
            return GitlabProjectsResult {
                status: result.status,
                code: result.code,
                message: result.message,
                projects: Vec::new(),
            };
        }
    };
    match fetch_projects(&connected.client, &connected.token, &connected.origin, connected.group_id).await {
        Ok(projects) => GitlabProjectsResult {
            status: "ok".into(),
            code: String::new(),
            message: String::new(),
            projects,
        },
        Err(error) => {
            let (status, code, message) = action_error(error);
            GitlabProjectsResult {
                status,
                code,
                message,
                projects: Vec::new(),
            }
        }
    }
}

pub async fn list_pipelines(token: String, group_url: String, project_id: u64) -> GitlabPipelinesResult {
    let connected = match connect(&token, &group_url).await {
        Ok(value) => value,
        Err(result) => {
            return GitlabPipelinesResult {
                status: result.status,
                code: result.code,
                message: result.message,
                pipelines: Vec::new(),
            };
        }
    };
    match fetch_pipelines(&connected.client, &connected.token, &connected.origin, project_id).await {
        Ok(pipelines) => GitlabPipelinesResult {
            status: "ok".into(),
            code: String::new(),
            message: String::new(),
            pipelines,
        },
        Err(error) => {
            let (status, code, message) = action_error(error);
            GitlabPipelinesResult {
                status,
                code,
                message,
                pipelines: Vec::new(),
            }
        }
    }
}

pub async fn list_branches(
    token: String,
    group_url: String,
    project_id: u64,
    search: String,
) -> GitlabBranchesResult {
    let connected = match connect(&token, &group_url).await {
        Ok(value) => value,
        Err(result) => {
            return GitlabBranchesResult {
                status: result.status,
                code: result.code,
                message: result.message,
                total: 0,
                branches: Vec::new(),
            };
        }
    };
    match fetch_branches(
        &connected.client,
        &connected.token,
        &connected.origin,
        project_id,
        search.trim(),
    )
    .await
    {
        Ok((total, branches)) => GitlabBranchesResult {
            status: "ok".into(),
            code: String::new(),
            message: String::new(),
            total,
            branches,
        },
        Err(error) => {
            let (status, code, message) = action_error(error);
            GitlabBranchesResult {
                status,
                code,
                message,
                total: 0,
                branches: Vec::new(),
            }
        }
    }
}

pub async fn create_pipeline(
    token: String,
    group_url: String,
    project_id: u64,
    git_ref: String,
) -> GitlabPipelinesResult {
    let git_ref = git_ref.trim().to_string();
    if git_ref.is_empty() {
        return GitlabPipelinesResult {
            status: "failed".into(),
            code: "bad_ref".into(),
            message: String::new(),
            pipelines: Vec::new(),
        };
    }
    let connected = match connect(&token, &group_url).await {
        Ok(value) => value,
        Err(result) => {
            return GitlabPipelinesResult {
                status: result.status,
                code: result.code,
                message: result.message,
                pipelines: Vec::new(),
            };
        }
    };
    match post_pipeline(
        &connected.client,
        &connected.token,
        &connected.origin,
        project_id,
        &git_ref,
    )
    .await
    {
        Ok(pipeline) => GitlabPipelinesResult {
            status: "ok".into(),
            code: String::new(),
            message: String::new(),
            pipelines: vec![pipeline],
        },
        Err(error) => {
            let (status, code, message) = action_error(error);
            GitlabPipelinesResult {
                status,
                code,
                message,
                pipelines: Vec::new(),
            }
        }
    }
}

pub async fn list_jobs(
    token: String,
    group_url: String,
    project_id: u64,
    pipeline_id: u64,
) -> GitlabJobsResult {
    let connected = match connect(&token, &group_url).await {
        Ok(value) => value,
        Err(result) => {
            return GitlabJobsResult {
                status: result.status,
                code: result.code,
                message: result.message,
                jobs: Vec::new(),
            };
        }
    };
    match fetch_jobs(
        &connected.client,
        &connected.token,
        &connected.origin,
        project_id,
        pipeline_id,
    )
    .await
    {
        Ok(jobs) => GitlabJobsResult {
            status: "ok".into(),
            code: String::new(),
            message: String::new(),
            jobs,
        },
        Err(error) => {
            let (status, code, message) = action_error(error);
            GitlabJobsResult {
                status,
                code,
                message,
                jobs: Vec::new(),
            }
        }
    }
}

pub async fn run_job(
    token: String,
    group_url: String,
    project_id: u64,
    job_id: u64,
    action: String,
) -> GitlabJobsResult {
    let action = action.trim().to_string();
    if action != "play" && action != "retry" {
        return GitlabJobsResult {
            status: "failed".into(),
            code: "bad_action".into(),
            message: String::new(),
            jobs: Vec::new(),
        };
    }
    let connected = match connect(&token, &group_url).await {
        Ok(value) => value,
        Err(result) => {
            return GitlabJobsResult {
                status: result.status,
                code: result.code,
                message: result.message,
                jobs: Vec::new(),
            };
        }
    };
    match post_job(
        &connected.client,
        &connected.token,
        &connected.origin,
        project_id,
        job_id,
        &action,
    )
    .await
    {
        Ok(job) => GitlabJobsResult {
            status: "ok".into(),
            code: String::new(),
            message: String::new(),
            jobs: vec![job],
        },
        Err(error) => {
            let (status, code, message) = action_error(error);
            GitlabJobsResult {
                status,
                code,
                message,
                jobs: Vec::new(),
            }
        }
    }
}

struct Connected {
    client: reqwest::Client,
    token: String,
    origin: String,
    group_id: u64,
}

async fn connect(token: &str, group_url: &str) -> Result<Connected, GitlabLoadResult> {
    let token = token.trim();
    let group_url = group_url.trim();
    if token.is_empty() || group_url.is_empty() {
        return Err(GitlabLoadResult::empty("missing", "missing", ""));
    }
    let (origin, path) = match parse_group_address(group_url) {
        Ok(value) => value,
        Err(()) => return Err(GitlabLoadResult::empty("failed", "bad_address", "")),
    };
    let client = match http_client() {
        Ok(client) => client,
        Err(message) => return Err(GitlabLoadResult::empty("failed", "network", message)),
    };
    let resource = lookup_group(&client, token, &origin, &path)
        .await
        .map_err(|error| {
            let (status, code, message) = action_error(error);
            GitlabLoadResult::empty(&status, &code, message)
        })?;
    Ok(Connected {
        client,
        token: token.to_string(),
        origin,
        group_id: resource.id,
    })
}

fn action_error(error: ApiError) -> (String, String, String) {
    match error {
        ApiError::Invalid => ("invalid".into(), "unauthorized".into(), String::new()),
        ApiError::NotFound => ("failed".into(), "not_found".into(), String::new()),
        ApiError::Forbidden => ("failed".into(), "forbidden".into(), String::new()),
        ApiError::BadResponse => ("failed".into(), "bad_response".into(), String::new()),
        ApiError::Network(message) => ("failed".into(), "network".into(), message),
        ApiError::Status(message) => ("failed".into(), "error".into(), message),
    }
}

async fn fetch_projects(
    client: &reqwest::Client,
    token: &str,
    origin: &str,
    group_id: u64,
) -> Result<Vec<GitlabProjectRow>, ApiError> {
    let url = format!("{origin}/api/v4/groups/{group_id}/projects");
    let mut page = 1u32;
    let mut rows = Vec::new();
    for _ in 0..20 {
        let page_no = page.to_string();
        let per_page = FETCH_PAGE_SIZE.to_string();
        let http = get_text(
            client,
            token,
            &url,
            &[
                ("include_subgroups", "true"),
                ("archived", "false"),
                ("simple", "true"),
                ("order_by", "name"),
                ("sort", "asc"),
                ("per_page", per_page.as_str()),
                ("page", page_no.as_str()),
            ],
        )
        .await?;
        ensure_ok(&http)?;
        let batch: Vec<RawProject> = serde_json::from_str(&http.body).map_err(|_| ApiError::BadResponse)?;
        let count = batch.len() as u32;
        rows.extend(batch.into_iter().map(|raw| GitlabProjectRow {
            id: raw.id,
            name: if raw.name.is_empty() { raw.path.clone() } else { raw.name },
            path: if raw.path_with_namespace.is_empty() {
                raw.path
            } else {
                raw.path_with_namespace
            },
            web_url: raw.web_url,
        }));
        if let Some(next) = http.next_page.filter(|next| *next > page) {
            page = next;
            continue;
        }
        if count < FETCH_PAGE_SIZE {
            break;
        }
        page += 1;
    }
    rows.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.path.to_lowercase().cmp(&right.path.to_lowercase()))
    });
    Ok(rows)
}

async fn fetch_pipelines(
    client: &reqwest::Client,
    token: &str,
    origin: &str,
    project_id: u64,
) -> Result<Vec<GitlabPipelineRow>, ApiError> {
    let url = format!("{origin}/api/v4/projects/{project_id}/pipelines");
    let http = get_text(
        client,
        token,
        &url,
        &[("per_page", "30"), ("order_by", "id"), ("sort", "desc")],
    )
    .await?;
    ensure_ok(&http)?;
    let batch: Vec<RawPipeline> = serde_json::from_str(&http.body).map_err(|_| ApiError::BadResponse)?;
    Ok(batch.into_iter().map(map_pipeline).collect())
}

async fn fetch_jobs(
    client: &reqwest::Client,
    token: &str,
    origin: &str,
    project_id: u64,
    pipeline_id: u64,
) -> Result<Vec<GitlabJobRow>, ApiError> {
    let url = format!("{origin}/api/v4/projects/{project_id}/pipelines/{pipeline_id}/jobs");
    let mut page = 1u32;
    let mut rows = Vec::new();
    for _ in 0..5 {
        let page_no = page.to_string();
        let http = get_text(
            client,
            token,
            &url,
            &[
                ("per_page", "100"),
                ("page", page_no.as_str()),
                ("include_retried", "false"),
            ],
        )
        .await?;
        ensure_ok(&http)?;
        let batch: Vec<RawJob> = serde_json::from_str(&http.body).map_err(|_| ApiError::BadResponse)?;
        let count = batch.len() as u32;
        rows.extend(batch.into_iter().map(map_job));
        if let Some(next) = http.next_page.filter(|next| *next > page) {
            page = next;
            continue;
        }
        if count < 100 {
            break;
        }
        page += 1;
    }
    rows.sort_by_key(|job| job.id);
    Ok(rows)
}

async fn post_job(
    client: &reqwest::Client,
    token: &str,
    origin: &str,
    project_id: u64,
    job_id: u64,
    action: &str,
) -> Result<GitlabJobRow, ApiError> {
    let url = format!("{origin}/api/v4/projects/{project_id}/jobs/{job_id}/{action}");
    let response = client
        .post(url)
        .header("PRIVATE-TOKEN", token)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .body("{}")
        .send()
        .await
        .map_err(|err| ApiError::Network(err.to_string()))?;
    let status = response.status().as_u16();
    let body = response
        .text()
        .await
        .map_err(|err| ApiError::Network(err.to_string()))?;
    let http = HttpText {
        status,
        next_page: None,
        total: None,
        body,
    };
    ensure_ok(&http)?;
    let raw: RawJob = serde_json::from_str(&http.body).map_err(|_| ApiError::BadResponse)?;
    Ok(map_job(raw))
}

fn map_job(raw: RawJob) -> GitlabJobRow {
    GitlabJobRow {
        id: raw.id,
        name: raw.name,
        stage: raw.stage,
        status: raw.status,
        when: raw.when,
        allow_failure: raw.allow_failure,
        duration: raw.duration,
        started_at: raw.started_at.unwrap_or_default(),
        finished_at: raw.finished_at.unwrap_or_default(),
    }
}

async fn fetch_branches(
    client: &reqwest::Client,
    token: &str,
    origin: &str,
    project_id: u64,
    search: &str,
) -> Result<(u32, Vec<GitlabBranchRow>), ApiError> {
    let url = format!("{origin}/api/v4/projects/{project_id}/repository/branches");
    let mut query = vec![("per_page", "100"), ("page", "1")];
    if !search.is_empty() {
        query.push(("search", search));
    }
    let http = get_text(client, token, &url, &query).await?;
    ensure_ok(&http)?;
    let batch: Vec<RawBranch> = serde_json::from_str(&http.body).map_err(|_| ApiError::BadResponse)?;
    let mut rows: Vec<GitlabBranchRow> = batch
        .into_iter()
        .map(|raw| GitlabBranchRow {
            name: raw.name,
            default: raw.default,
        })
        .collect();
    rows.sort_by(|left, right| right.default.cmp(&left.default).then(left.name.cmp(&right.name)));
    let total = http.total.unwrap_or(rows.len() as u32);
    Ok((total, rows))
}

async fn post_pipeline(
    client: &reqwest::Client,
    token: &str,
    origin: &str,
    project_id: u64,
    git_ref: &str,
) -> Result<GitlabPipelineRow, ApiError> {
    let url = format!("{origin}/api/v4/projects/{project_id}/pipeline");
    let response = client
        .post(url)
        .header("PRIVATE-TOKEN", token)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .body(serde_json::json!({ "ref": git_ref }).to_string())
        .send()
        .await
        .map_err(|err| ApiError::Network(err.to_string()))?;
    let status = response.status().as_u16();
    let body = response
        .text()
        .await
        .map_err(|err| ApiError::Network(err.to_string()))?;
    let http = HttpText {
        status,
        next_page: None,
        total: None,
        body,
    };
    ensure_ok(&http)?;
    let raw: RawPipeline = serde_json::from_str(&http.body).map_err(|_| ApiError::BadResponse)?;
    Ok(map_pipeline(raw))
}

fn map_pipeline(raw: RawPipeline) -> GitlabPipelineRow {
    GitlabPipelineRow {
        id: raw.id,
        iid: raw.iid,
        status: raw.status,
        git_ref: raw.git_ref,
        sha: raw.sha,
        source: raw.source,
        web_url: raw.web_url,
        created_at: raw.created_at,
    }
}

pub async fn load_variables(token: String, group_url: String, _page: u32) -> GitlabLoadResult {
    let token = token.trim();
    let group_url = group_url.trim();
    if token.is_empty() || group_url.is_empty() {
        return GitlabLoadResult::empty("missing", "missing", "");
    }
    let (origin, path) = match parse_group_address(group_url) {
        Ok(value) => value,
        Err(()) => return GitlabLoadResult::empty("failed", "bad_address", ""),
    };
    let client = match http_client() {
        Ok(client) => client,
        Err(message) => return GitlabLoadResult::empty("failed", "network", message),
    };
    match load_with(&client, token, &origin, &path).await {
        Ok(result) => result,
        Err(ApiError::Invalid) => GitlabLoadResult::empty("invalid", "unauthorized", ""),
        Err(ApiError::NotFound) => GitlabLoadResult::empty("failed", "not_found", ""),
        Err(ApiError::Forbidden) => GitlabLoadResult::empty("failed", "forbidden", ""),
        Err(ApiError::BadResponse) => GitlabLoadResult::empty("failed", "bad_response", ""),
        Err(ApiError::Network(message)) => GitlabLoadResult::empty("failed", "network", message),
        Err(ApiError::Status(message)) => GitlabLoadResult::empty("failed", "error", message),
    }
}

async fn load_with(
    client: &reqwest::Client,
    token: &str,
    origin: &str,
    path: &str,
) -> Result<GitlabLoadResult, ApiError> {
    let username = probe_user(client, token, origin).await?;
    let resource = lookup_group(client, token, origin, path).await?;
    let page = list_variables(client, token, origin, &resource).await?;
    Ok(GitlabLoadResult::ready(username, resource, page))
}

fn parse_group_address(raw: &str) -> Result<(String, String), ()> {
    let url = reqwest::Url::parse(raw).map_err(|_| ())?;
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err(());
    }
    let origin = url.origin().ascii_serialization();
    if !origin.starts_with("http://") && !origin.starts_with("https://") {
        return Err(());
    }
    let path = url.path().trim_matches('/');
    if path.is_empty() {
        return Err(());
    }
    Ok((origin, path.to_string()))
}

fn group_api_url(origin: &str, path: &str) -> String {
    let encoded = path.replace('/', "%2F");
    format!("{origin}/api/v4/groups/{encoded}")
}

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .user_agent("central-devops-kit")
        .build()
        .map_err(|err| err.to_string())
}

async fn probe_user(client: &reqwest::Client, token: &str, origin: &str) -> Result<String, ApiError> {
    let url = format!("{origin}/api/v4/user");
    let http = get_text(client, token, &url, &[]).await?;
    if http.status == 401 {
        return Err(ApiError::Invalid);
    }
    // 没有 read_user 时 /user 会 403，项目变量仍可能读得到。
    if http.status == 403 {
        return Ok(String::new());
    }
    ensure_ok(&http)?;
    let user: RawUser = serde_json::from_str(&http.body).map_err(|_| ApiError::BadResponse)?;
    Ok(user.username)
}

async fn lookup_group(
    client: &reqwest::Client,
    token: &str,
    origin: &str,
    path: &str,
) -> Result<Resource, ApiError> {
    let http = get_text(client, token, &group_api_url(origin, path), &[]).await?;
    if http.status == 404 {
        return Err(ApiError::NotFound);
    }
    if http.status == 401 {
        return Err(ApiError::Invalid);
    }
    if http.status == 403 {
        return Err(ApiError::Forbidden);
    }
    ensure_ok(&http)?;
    let raw: RawResource = serde_json::from_str(&http.body).map_err(|_| ApiError::BadResponse)?;
    let resolved = if !raw.full_path.is_empty() {
        raw.full_path
    } else if !raw.path_with_namespace.is_empty() {
        raw.path_with_namespace
    } else {
        path.to_string()
    };
    Ok(Resource {
        kind: "group".into(),
        name: raw.name,
        path: resolved,
        id: raw.id,
    })
}

async fn list_variables(
    client: &reqwest::Client,
    token: &str,
    origin: &str,
    resource: &Resource,
) -> Result<VariablePage, ApiError> {
    let url = format!("{origin}/api/v4/groups/{}/variables", resource.id);
    let mut page = 1u32;
    let mut rows = Vec::new();
    for _ in 0..100 {
        let page_no = page.to_string();
        let per_page = FETCH_PAGE_SIZE.to_string();
        let http = get_text(
            client,
            token,
            &url,
            &[("per_page", per_page.as_str()), ("page", page_no.as_str())],
        )
        .await?;
        if http.status == 401 {
            return Err(ApiError::Invalid);
        }
        if http.status == 403 {
            return Err(ApiError::Forbidden);
        }
        if http.status == 404 {
            return Err(ApiError::NotFound);
        }
        ensure_ok(&http)?;
        let batch: Vec<RawVariable> =
            serde_json::from_str(&http.body).map_err(|_| ApiError::BadResponse)?;
        let count = batch.len() as u32;
        rows.extend(batch.into_iter().map(map_variable));
        if let Some(next) = http.next_page.filter(|next| *next > page) {
            page = next;
            continue;
        }
        if count < FETCH_PAGE_SIZE {
            break;
        }
        page += 1;
    }
    let total = rows.len() as u32;
    Ok(VariablePage { total, rows })
}

fn map_variable(raw: RawVariable) -> GitlabVariableRow {
    let scope = raw.environment_scope.unwrap_or_default();
    GitlabVariableRow {
        key: raw.key,
        value: raw.value.unwrap_or_default(),
        hidden: raw.hidden,
        variable_type: if raw.variable_type.is_empty() {
            "env_var".into()
        } else {
            raw.variable_type
        },
        is_protected: raw.protected,
        is_masked: raw.masked,
        expanded: raw.raw == Some(false),
        environment_scope: if scope.is_empty() { "*".into() } else { scope },
        description: raw.description.unwrap_or_default(),
    }
}

async fn get_text(
    client: &reqwest::Client,
    token: &str,
    url: &str,
    query: &[(&str, &str)],
) -> Result<HttpText, ApiError> {
    let response = client
        .get(url)
        .header("PRIVATE-TOKEN", token)
        .header("Accept", "application/json")
        .query(query)
        .send()
        .await
        .map_err(|err| ApiError::Network(err.to_string()))?;
    let status = response.status().as_u16();
    let next_page = header_u32(response.headers(), "x-next-page");
    let total = header_u32(response.headers(), "x-total");
    let body = response
        .text()
        .await
        .map_err(|err| ApiError::Network(err.to_string()))?;
    Ok(HttpText {
        status,
        next_page,
        total,
        body,
    })
}

fn header_u32(headers: &reqwest::header::HeaderMap, name: &str) -> Option<u32> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|value| value.parse().ok())
}

fn ensure_ok(http: &HttpText) -> Result<(), ApiError> {
    if (200..300).contains(&http.status) {
        return Ok(());
    }
    if http.status == 401 {
        return Err(ApiError::Invalid);
    }
    if http.status == 403 {
        return Err(ApiError::Forbidden);
    }
    if http.status == 404 {
        return Err(ApiError::NotFound);
    }
    Err(ApiError::Status(brief(&http.body)))
}

fn brief(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.is_empty() || trimmed.starts_with('<') {
        return String::new();
    }
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(message) = value.get("message") {
            if let Some(text) = message.as_str() {
                return clip(text);
            }
            if !message.is_null() {
                return clip(&message.to_string());
            }
        }
        if let Some(message) = value.get("error").and_then(|item| item.as_str()) {
            return clip(message);
        }
    }
    clip(trimmed)
}

fn clip(text: &str) -> String {
    text.chars().take(180).collect()
}

#[derive(serde::Deserialize)]
struct RawUser {
    #[serde(default)]
    username: String,
}

#[derive(serde::Deserialize)]
struct RawResource {
    id: u64,
    #[serde(default)]
    name: String,
    #[serde(default)]
    path_with_namespace: String,
    #[serde(default)]
    full_path: String,
}

#[derive(serde::Deserialize)]
struct RawProject {
    id: u64,
    #[serde(default)]
    name: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    path_with_namespace: String,
    #[serde(default)]
    web_url: String,
}

#[derive(serde::Deserialize)]
struct RawPipeline {
    id: u64,
    #[serde(default)]
    iid: u64,
    #[serde(default)]
    status: String,
    #[serde(rename = "ref", default)]
    git_ref: String,
    #[serde(default)]
    sha: String,
    #[serde(default)]
    source: String,
    #[serde(default)]
    web_url: String,
    #[serde(default)]
    created_at: String,
}

#[derive(serde::Deserialize)]
struct RawJob {
    id: u64,
    #[serde(default)]
    name: String,
    #[serde(default)]
    stage: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    when: String,
    #[serde(default)]
    allow_failure: bool,
    #[serde(default)]
    duration: Option<f64>,
    #[serde(default)]
    started_at: Option<String>,
    #[serde(default)]
    finished_at: Option<String>,
}

#[derive(serde::Deserialize)]
struct RawBranch {
    #[serde(default)]
    name: String,
    #[serde(default)]
    default: bool,
}

#[derive(serde::Deserialize)]
struct RawVariable {
    key: String,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    variable_type: String,
    #[serde(default)]
    protected: bool,
    #[serde(default)]
    masked: bool,
    #[serde(default)]
    raw: Option<bool>,
    #[serde(default)]
    hidden: bool,
    #[serde(default)]
    environment_scope: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

#[cfg(test)]
mod tests {
    #[test]
    fn group_address_encodes_nested_path() {
        let (origin, path) =
            super::parse_group_address("https://gitlab.example.com/parent/child").unwrap();
        assert_eq!(origin, "https://gitlab.example.com");
        assert_eq!(path, "parent/child");
        let url = reqwest::Url::parse(&super::group_api_url(&origin, &path)).unwrap();
        assert!(url.as_str().contains("parent%2Fchild"), "{}", url.as_str());
    }
}

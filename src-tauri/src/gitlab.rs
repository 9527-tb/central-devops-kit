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
    body: String,
}

struct VariablePage {
    total: u32,
    rows: Vec<GitlabVariableRow>,
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
    let body = response
        .text()
        .await
        .map_err(|err| ApiError::Network(err.to_string()))?;
    Ok(HttpText {
        status,
        next_page,
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
        if let Some(message) = value.get("message").and_then(|item| item.as_str()) {
            return clip(message);
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

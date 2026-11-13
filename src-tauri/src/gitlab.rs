//! 读取用户配置的 GitLab group 下的 CI/CD 变量。
//! group 地址和 Access Token 只从参数进入，不写入源码。

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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitlabActionResult {
    pub status: String,
    pub code: String,
    pub message: String,
}

impl GitlabActionResult {
    fn ok() -> Self {
        Self {
            status: "ok".into(),
            code: String::new(),
            message: String::new(),
        }
    }

    fn failed(code: &str, message: impl Into<String>) -> Self {
        let status = if code == "missing" || code == "invalid" {
            code
        } else {
            "failed"
        };
        Self {
            status: status.into(),
            code: code.into(),
            message: message.into(),
        }
    }
}

pub async fn delete_variable(
    token: String,
    group_url: String,
    key: String,
    environment_scope: String,
) -> GitlabActionResult {
    let token = token.trim();
    let group_url = group_url.trim();
    let key = key.trim();
    let scope = environment_scope.trim();
    if token.is_empty() || group_url.is_empty() || key.is_empty() {
        return GitlabActionResult::failed("missing", "");
    }
    let (origin, path) = match parse_group_address(group_url) {
        Ok(value) => value,
        Err(()) => return GitlabActionResult::failed("bad_address", ""),
    };
    let client = match http_client() {
        Ok(client) => client,
        Err(message) => return GitlabActionResult::failed("network", message),
    };
    match delete_with(&client, token, &origin, &path, key, scope).await {
        Ok(()) => GitlabActionResult::ok(),
        Err(ApiError::Invalid) => GitlabActionResult::failed("invalid", ""),
        Err(ApiError::NotFound) => GitlabActionResult::failed("not_found", ""),
        Err(ApiError::Forbidden) => GitlabActionResult::failed("forbidden", ""),
        Err(ApiError::BadResponse) => GitlabActionResult::failed("bad_response", ""),
        Err(ApiError::Network(message)) => GitlabActionResult::failed("network", message),
        Err(ApiError::Status(message)) => GitlabActionResult::failed("error", message),
    }
}

async fn delete_with(
    client: &reqwest::Client,
    token: &str,
    origin: &str,
    path: &str,
    key: &str,
    scope: &str,
) -> Result<(), ApiError> {
    let resource = lookup_group(client, token, origin, path).await?;
    let url = format!(
        "{origin}/api/v4/groups/{}/variables/{}",
        resource.id,
        encode_component(key)
    );
    let http = delete_text(
        client,
        token,
        &url,
        &[("filter[environment_scope]", scope)],
    )
    .await?;
    if http.status == 404 {
        return Err(ApiError::NotFound);
    }
    if http.status == 401 {
        return Err(ApiError::Invalid);
    }
    if http.status == 403 {
        return Err(ApiError::Forbidden);
    }
    ensure_ok(&http)
}

pub async fn create_variable(
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
) -> GitlabActionResult {
    let token = token.trim();
    let group_url = group_url.trim();
    let key = key.trim();
    let scope = environment_scope.trim();
    let description = description.trim();
    if token.is_empty() || group_url.is_empty() || scope.is_empty() {
        return GitlabActionResult::failed("missing", "");
    }
    if !variable_key_ok(key) {
        return GitlabActionResult::failed("bad_key", "");
    }
    let (origin, path) = match parse_group_address(group_url) {
        Ok(value) => value,
        Err(()) => return GitlabActionResult::failed("bad_address", ""),
    };
    let client = match http_client() {
        Ok(client) => client,
        Err(message) => return GitlabActionResult::failed("network", message),
    };
    let kind = if variable_type.trim() == "file" {
        "file"
    } else {
        "env_var"
    };
    match create_with(
        &client,
        token,
        &origin,
        &path,
        key,
        &value,
        scope,
        kind,
        is_protected,
        is_masked,
        expanded,
        description,
    )
    .await
    {
        Ok(()) => GitlabActionResult::ok(),
        Err(ApiError::Invalid) => GitlabActionResult::failed("invalid", ""),
        Err(ApiError::NotFound) => GitlabActionResult::failed("not_found", ""),
        Err(ApiError::Forbidden) => GitlabActionResult::failed("forbidden", ""),
        Err(ApiError::BadResponse) => GitlabActionResult::failed("bad_response", ""),
        Err(ApiError::Network(message)) => GitlabActionResult::failed("network", message),
        Err(ApiError::Status(message)) => GitlabActionResult::failed("error", message),
    }
}

async fn create_with(
    client: &reqwest::Client,
    token: &str,
    origin: &str,
    path: &str,
    key: &str,
    value: &str,
    scope: &str,
    variable_type: &str,
    is_protected: bool,
    is_masked: bool,
    expanded: bool,
    description: &str,
) -> Result<(), ApiError> {
    let resource = lookup_group(client, token, origin, path).await?;
    let url = format!("{origin}/api/v4/groups/{}/variables", resource.id);
    let body = NewVariable {
        key,
        value,
        variable_type,
        protected: is_protected,
        masked: is_masked,
        raw: !expanded,
        environment_scope: scope,
        description,
    };
    let http = post_json(client, token, &url, &body).await?;
    if http.status == 401 {
        return Err(ApiError::Invalid);
    }
    if http.status == 403 {
        return Err(ApiError::Forbidden);
    }
    if http.status == 404 {
        return Err(ApiError::NotFound);
    }
    ensure_ok(&http)
}

fn variable_key_ok(key: &str) -> bool {
    !key.is_empty() && key.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

pub async fn update_variable(
    token: String,
    group_url: String,
    key: String,
    environment_scope: String,
    value: String,
) -> GitlabActionResult {
    let token = token.trim();
    let group_url = group_url.trim();
    let key = key.trim();
    let scope = environment_scope.trim();
    if token.is_empty() || group_url.is_empty() || key.is_empty() || scope.is_empty() {
        return GitlabActionResult::failed("missing", "");
    }
    let (origin, path) = match parse_group_address(group_url) {
        Ok(value) => value,
        Err(()) => return GitlabActionResult::failed("bad_address", ""),
    };
    let client = match http_client() {
        Ok(client) => client,
        Err(message) => return GitlabActionResult::failed("network", message),
    };
    match update_with(&client, token, &origin, &path, key, scope, &value).await {
        Ok(()) => GitlabActionResult::ok(),
        Err(ApiError::Invalid) => GitlabActionResult::failed("invalid", ""),
        Err(ApiError::NotFound) => GitlabActionResult::failed("not_found", ""),
        Err(ApiError::Forbidden) => GitlabActionResult::failed("forbidden", ""),
        Err(ApiError::BadResponse) => GitlabActionResult::failed("bad_response", ""),
        Err(ApiError::Network(message)) => GitlabActionResult::failed("network", message),
        Err(ApiError::Status(message)) => GitlabActionResult::failed("error", message),
    }
}

async fn update_with(
    client: &reqwest::Client,
    token: &str,
    origin: &str,
    path: &str,
    key: &str,
    scope: &str,
    value: &str,
) -> Result<(), ApiError> {
    let resource = lookup_group(client, token, origin, path).await?;
    let url = format!(
        "{origin}/api/v4/groups/{}/variables/{}",
        resource.id,
        encode_component(key)
    );
    let body = UpdateValue { value };
    let http = put_json(
        client,
        token,
        &url,
        &[("filter[environment_scope]", scope)],
        &body,
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
    ensure_ok(&http)
}

#[derive(Serialize)]
struct UpdateValue<'a> {
    value: &'a str,
}

#[derive(Serialize)]
struct NewVariable<'a> {
    key: &'a str,
    value: &'a str,
    variable_type: &'a str,
    protected: bool,
    masked: bool,
    raw: bool,
    environment_scope: &'a str,
    description: &'a str,
}

fn encode_component(raw: &str) -> String {
    let mut out = String::new();
    for byte in raw.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
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
    // 没有 read_user 时 /user 会 403，group 变量仍可能读得到。
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

async fn delete_text(
    client: &reqwest::Client,
    token: &str,
    url: &str,
    query: &[(&str, &str)],
) -> Result<HttpText, ApiError> {
    let response = client
        .delete(url)
        .header("PRIVATE-TOKEN", token)
        .header("Accept", "application/json")
        .query(query)
        .send()
        .await
        .map_err(|err| ApiError::Network(err.to_string()))?;
    let status = response.status().as_u16();
    let body = response
        .text()
        .await
        .map_err(|err| ApiError::Network(err.to_string()))?;
    Ok(HttpText {
        status,
        next_page: None,
        body,
    })
}

async fn post_json(
    client: &reqwest::Client,
    token: &str,
    url: &str,
    body: &impl Serialize,
) -> Result<HttpText, ApiError> {
    let payload = serde_json::to_string(body).map_err(|_| ApiError::BadResponse)?;
    let response = client
        .post(url)
        .header("PRIVATE-TOKEN", token)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .body(payload)
        .send()
        .await
        .map_err(|err| ApiError::Network(err.to_string()))?;
    let status = response.status().as_u16();
    let body = response
        .text()
        .await
        .map_err(|err| ApiError::Network(err.to_string()))?;
    Ok(HttpText {
        status,
        next_page: None,
        body,
    })
}

async fn put_json(
    client: &reqwest::Client,
    token: &str,
    url: &str,
    query: &[(&str, &str)],
    body: &impl Serialize,
) -> Result<HttpText, ApiError> {
    let payload = serde_json::to_string(body).map_err(|_| ApiError::BadResponse)?;
    let response = client
        .put(url)
        .header("PRIVATE-TOKEN", token)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .query(query)
        .body(payload)
        .send()
        .await
        .map_err(|err| ApiError::Network(err.to_string()))?;
    let status = response.status().as_u16();
    let body = response
        .text()
        .await
        .map_err(|err| ApiError::Network(err.to_string()))?;
    Ok(HttpText {
        status,
        next_page: None,
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
            super::parse_group_address("https://gitlab.example.com/parent/child/").unwrap();
        assert_eq!(origin, "https://gitlab.example.com");
        assert_eq!(path, "parent/child");
        let url = super::group_api_url(&origin, &path);
        assert!(url.contains("/api/v4/groups/parent%2Fchild"), "{url}");
        assert!(super::parse_group_address("https://gitlab.example.com").is_err());
        assert!(super::parse_group_address("gitlab.example.com/parent").is_err());
    }

    #[test]
    fn variable_key_is_encoded_in_the_path() {
        assert_eq!(super::encode_component("API_TOKEN"), "API_TOKEN");
        assert_eq!(super::encode_component("a b/c"), "a%20b%2Fc");
    }

    #[test]
    fn variable_key_allows_letters_digits_and_underscore() {
        assert!(super::variable_key_ok("API_TOKEN"));
        assert!(!super::variable_key_ok(""));
        assert!(!super::variable_key_ok("has space"));
        assert!(!super::variable_key_ok("a-b"));
    }
}

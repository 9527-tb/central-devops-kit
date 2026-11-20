//! central-devops 白名单：确认 master、四个 YAML，并读出 notRemoteIpBlocks。

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::Emitter;

const FILES: &[(&str, &str, &str)] = &[
    (
        "connect-prod",
        "mclinica-connect-prod",
        "deploy/mclinica-connect-prod/psplink-ip-whitelist-production.yaml",
    ),
    (
        "connect-staging",
        "mclinica-connect-staging",
        "deploy/mclinica-connect-staging/psplink-ip-whitelist-staging.yaml",
    ),
    (
        "indo-prod",
        "psplink-indo-prod",
        "deploy/psplink-indo-prod/psplink-ip-whitelist-production.yaml",
    ),
    (
        "indo-staging",
        "psplink-indo-staging",
        "deploy/psplink-indo-staging/psplink-ip-whitelist-staging.yaml",
    ),
];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GcpIp {
    pub cidr: String,
    pub comment: String,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GcpWhitelistFile {
    pub key: String,
    pub title: String,
    pub relative_path: String,
    pub exists: bool,
    pub error: String,
    pub ips: Vec<GcpIp>,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GcpAlignedCell {
    pub cidr: String,
    pub comment: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GcpAlignedRow {
    pub cells: Vec<GcpAlignedCell>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GcpInspectResult {
    pub path: String,
    pub directory: bool,
    pub repository: bool,
    pub branch: String,
    pub on_master: bool,
    pub git_error: String,
    pub files: Vec<GcpWhitelistFile>,
    pub rows: Vec<GcpAlignedRow>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GcpAddResult {
    pub code: String,
    pub added: Vec<String>,
    pub duplicates: Vec<String>,
    pub missing: Vec<String>,
    pub failed: Vec<String>,
    pub inspect: GcpInspectResult,
}

pub fn inspect_project(path: &str) -> GcpInspectResult {
    let cleaned = path.trim().trim_matches('"').trim();
    let root = PathBuf::from(cleaned);
    let directory = root.is_dir();
    let (repository, branch, git_error) = if directory {
        match current_branch(&root) {
            Ok(name) => (true, name, String::new()),
            Err(error) => (false, String::new(), error),
        }
    } else {
        (false, String::new(), String::new())
    };
    let files = FILES
        .iter()
        .map(|(key, title, relative)| read_whitelist_file(&root, directory, key, title, relative))
        .collect::<Vec<_>>();
    let lists = files.iter().map(|file| file.ips.clone()).collect::<Vec<_>>();
    let rows = align_ip_lists(&lists);
    GcpInspectResult {
        path: cleaned.to_string(),
        directory,
        repository,
        on_master: branch == "master",
        branch,
        git_error,
        files,
        rows,
    }
}

pub fn add_whitelist_ip(path: &str, cidr: &str, comment: &str, keys: &[String]) -> GcpAddResult {
    let cidr = cidr.trim();
    let comment = comment.trim();
    let inspect = inspect_project(path);
    let selected = FILES
        .iter()
        .filter(|(key, _, _)| keys.iter().any(|item| item == key))
        .copied()
        .collect::<Vec<_>>();
    if !inspect.directory {
        return add_result("not_directory", &[], &[], &[], &[], inspect);
    }
    if !inspect.repository || !inspect.on_master {
        return add_result("not_master", &[], &[], &[], &[], inspect);
    }
    if cidr.is_empty() || comment.contains('\n') || comment.contains('\r') || !valid_cidr(cidr) {
        return add_result("invalid", &[], &[], &[], &[], inspect);
    }
    if selected.is_empty() {
        return add_result("empty", &[], &[], &[], &[], inspect);
    }
    let root = PathBuf::from(inspect.path.clone());
    let mut added = Vec::new();
    let mut duplicates = Vec::new();
    let mut missing = Vec::new();
    let mut failed = Vec::new();
    for (key, _, relative) in selected {
        let full = root.join(relative);
        if !full.is_file() {
            missing.push(key.to_string());
            continue;
        }
        let text = match fs::read_to_string(&full) {
            Ok(text) => text,
            Err(_) => {
                failed.push(key.to_string());
                continue;
            }
        };
        if parse_not_remote_ip_blocks(&text).iter().any(|ip| ip.cidr == cidr) {
            duplicates.push(key.to_string());
            continue;
        }
        let updated = match append_not_remote_ip(&text, cidr, comment) {
            Ok(updated) => updated,
            Err(_) => {
                failed.push(key.to_string());
                continue;
            }
        };
        if write_text(&full, &updated).is_err() {
            failed.push(key.to_string());
            continue;
        }
        added.push(key.to_string());
    }
    let code = if !added.is_empty() && duplicates.is_empty() && missing.is_empty() && failed.is_empty() {
        "ok"
    } else if !added.is_empty() {
        "partial"
    } else if !duplicates.is_empty() && missing.is_empty() && failed.is_empty() {
        "duplicate"
    } else if !missing.is_empty() && duplicates.is_empty() && failed.is_empty() {
        "missing"
    } else {
        "failed"
    };
    let inspect = inspect_project(path);
    add_result(code, &added, &duplicates, &missing, &failed, inspect)
}

fn add_result(
    code: &str,
    added: &[String],
    duplicates: &[String],
    missing: &[String],
    failed: &[String],
    inspect: GcpInspectResult,
) -> GcpAddResult {
    GcpAddResult {
        code: code.to_string(),
        added: added.to_vec(),
        duplicates: duplicates.to_vec(),
        missing: missing.to_vec(),
        failed: failed.to_vec(),
        inspect,
    }
}

fn valid_cidr(value: &str) -> bool {
    let (addr, prefix) = match value.rsplit_once('/') {
        Some((addr, prefix)) => (addr, Some(prefix)),
        None => (value, None),
    };
    if addr.is_empty() {
        return false;
    }
    let Ok(ip) = addr.parse::<std::net::IpAddr>() else {
        return false;
    };
    let Some(prefix) = prefix else {
        return true;
    };
    if prefix.is_empty() || !prefix.bytes().all(|byte| byte.is_ascii_digit()) {
        return false;
    }
    let Ok(bits) = prefix.parse::<u16>() else {
        return false;
    };
    match ip {
        std::net::IpAddr::V4(_) => bits <= 32,
        std::net::IpAddr::V6(_) => bits <= 128,
    }
}

/// 把一条 IP 追加到第一个 `notRemoteIpBlocks` 列表末尾，保留文件其余内容。
pub fn append_not_remote_ip(text: &str, cidr: &str, comment: &str) -> Result<String, String> {
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut lines: Vec<String> = text
        .split('\n')
        .map(|line| line.trim_end_matches('\r').to_string())
        .collect();
    let mut index = 0;
    while index < lines.len() {
        let Some(indent) = key_indent(&lines[index], "notRemoteIpBlocks:") else {
            index += 1;
            continue;
        };
        let mut last_item = None;
        let mut cursor = index + 1;
        while cursor < lines.len() {
            let line = &lines[cursor];
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                cursor += 1;
                continue;
            }
            if leading_spaces(line) <= indent {
                break;
            }
            if parse_list_item(trimmed).is_some() {
                last_item = Some(cursor);
                cursor += 1;
                continue;
            }
            break;
        }
        let (insert_at, prefix) = if let Some(item_index) = last_item {
            let prefix: String = lines[item_index]
                .chars()
                .take_while(|ch| *ch == ' ' || *ch == '\t')
                .collect();
            (item_index + 1, prefix)
        } else {
            (index + 1, " ".repeat(indent + 2))
        };
        let mut line = format!("{prefix}- \"{cidr}\"");
        if !comment.is_empty() {
            line.push_str("   # ");
            line.push_str(comment);
        }
        lines.insert(insert_at, line);
        return Ok(lines.join(newline));
    }
    Err("notRemoteIpBlocks not found".to_string())
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GcpSaveFile {
    pub key: String,
    pub ips: Vec<GcpIp>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GcpSaveResult {
    pub code: String,
    pub inspect: GcpInspectResult,
}

pub fn save_whitelist_files(path: &str, files: &[GcpSaveFile]) -> GcpSaveResult {
    let inspect = inspect_project(path);
    if !inspect.directory {
        return GcpSaveResult { code: "not_directory".into(), inspect };
    }
    if !inspect.repository || !inspect.on_master {
        return GcpSaveResult { code: "not_master".into(), inspect };
    }
    let root = PathBuf::from(&inspect.path);
    for spec in files {
        let Some(relative) = FILES.iter().find(|(key, _, _)| *key == spec.key).map(|file| file.2) else {
            return GcpSaveResult { code: "failed".into(), inspect };
        };
        for ip in &spec.ips {
            if !valid_cidr(&ip.cidr) || ip.cidr.contains('"') || ip.comment.contains('\n') || ip.comment.contains('\r') {
                return GcpSaveResult { code: "invalid".into(), inspect };
            }
        }
        let full = root.join(relative);
        if !full.is_file() {
            return GcpSaveResult { code: "missing".into(), inspect: inspect_project(path) };
        }
        let text = match fs::read_to_string(&full) {
            Ok(text) => text,
            Err(_) => return GcpSaveResult { code: "failed".into(), inspect: inspect_project(path) },
        };
        let mut ips = spec.ips.clone();
        ips.sort_by(|left, right| ip_ord(&left.cidr).cmp(&ip_ord(&right.cidr)));
        let updated = match replace_not_remote_ip_blocks(&text, &ips) {
            Ok(updated) => updated,
            Err(_) => return GcpSaveResult { code: "failed".into(), inspect: inspect_project(path) },
        };
        if write_text(&full, &updated).is_err() {
            return GcpSaveResult { code: "failed".into(), inspect: inspect_project(path) };
        }
    }
    GcpSaveResult { code: "ok".into(), inspect: inspect_project(path) }
}

/// 用给定清单替换第一个 `notRemoteIpBlocks` 列表，保留文件其余内容。
pub fn replace_not_remote_ip_blocks(text: &str, ips: &[GcpIp]) -> Result<String, String> {
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut lines: Vec<String> = text
        .split('\n')
        .map(|line| line.trim_end_matches('\r').to_string())
        .collect();
    let mut index = 0;
    while index < lines.len() {
        let Some(indent) = key_indent(&lines[index], "notRemoteIpBlocks:") else {
            index += 1;
            continue;
        };
        let mut item_indexes = Vec::new();
        let mut cursor = index + 1;
        while cursor < lines.len() {
            let line = &lines[cursor];
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                cursor += 1;
                continue;
            }
            if leading_spaces(line) <= indent {
                break;
            }
            if parse_list_item(trimmed).is_some() {
                item_indexes.push(cursor);
                cursor += 1;
                continue;
            }
            break;
        }
        let prefix = if let Some(first) = item_indexes.first() {
            lines[*first]
                .chars()
                .take_while(|ch| *ch == ' ' || *ch == '\t')
                .collect::<String>()
        } else {
            " ".repeat(indent + 2)
        };
        let insert_at = item_indexes.first().copied().unwrap_or(index + 1);
        for item_index in item_indexes.iter().rev() {
            lines.remove(*item_index);
        }
        let rendered = ips
            .iter()
            .map(|ip| {
                let mut line = format!("{prefix}- \"{}\"", ip.cidr);
                if !ip.comment.is_empty() {
                    line.push_str("   # ");
                    line.push_str(ip.comment.trim());
                }
                line
            })
            .collect::<Vec<_>>();
        for (offset, line) in rendered.into_iter().enumerate() {
            lines.insert(insert_at + offset, line);
        }
        return Ok(lines.join(newline));
    }
    Err("notRemoteIpBlocks not found".to_string())
}

fn write_text(path: &Path, text: &str) -> Result<(), String> {
    let tmp = path.with_extension("yaml.tmp");
    fs::write(&tmp, text).map_err(|error| error.to_string())?;
    fs::rename(&tmp, path).map_err(|error| {
        let _ = fs::remove_file(&tmp);
        error.to_string()
    })
}

fn read_whitelist_file(root: &Path, directory: bool, key: &str, title: &str, relative: &str) -> GcpWhitelistFile {
    let full = root.join(relative);
    if !directory || !full.is_file() {
        return GcpWhitelistFile {
            key: key.to_string(),
            title: title.to_string(),
            relative_path: relative.to_string(),
            exists: false,
            error: String::new(),
            ips: Vec::new(),
        };
    }
    match fs::read_to_string(&full) {
        Ok(text) => GcpWhitelistFile {
            key: key.to_string(),
            title: title.to_string(),
            relative_path: relative.to_string(),
            exists: true,
            error: String::new(),
            ips: parse_not_remote_ip_blocks(&text),
        },
        Err(error) => GcpWhitelistFile {
            key: key.to_string(),
            title: title.to_string(),
            relative_path: relative.to_string(),
            exists: true,
            error: error.to_string(),
            ips: Vec::new(),
        },
    }
}

fn current_branch(root: &Path) -> Result<String, String> {
    let parsed = git_output(root, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    if parsed.status.success() {
        let branch = String::from_utf8_lossy(&parsed.stdout).trim().to_string();
        if !branch.is_empty() && branch != "HEAD" {
            return Ok(branch);
        }
    }
    let symbolic = git_output(root, &["symbolic-ref", "--short", "HEAD"])?;
    if symbolic.status.success() {
        let branch = String::from_utf8_lossy(&symbolic.stdout).trim().to_string();
        if !branch.is_empty() {
            return Ok(branch);
        }
    }
    let stderr = String::from_utf8_lossy(if parsed.stderr.is_empty() { &symbolic.stderr } else { &parsed.stderr })
        .trim()
        .to_string();
    if stderr.is_empty() {
        Err("git rev-parse --abbrev-ref HEAD 失败".to_string())
    } else {
        Err(stderr)
    }
}

fn git_output(root: &Path, args: &[&str]) -> Result<std::process::Output, String> {
    let git = find_tool("git").ok_or_else(|| "找不到 git 命令".to_string())?;
    let mut argv = vec!["-C".to_string(), root.display().to_string()];
    argv.extend(args.iter().map(|arg| (*arg).to_string()));
    let refs: Vec<&str> = argv.iter().map(String::as_str).collect();
    let (mut command, _) = tool_command(&git, &refs);
    command.output().map_err(|error| error.to_string())
}

fn find_tool(name: &str) -> Option<PathBuf> {
    tool_in_path(name).or_else(|| tool_in_known_dirs(name)).or_else(|| tool_in_login_shell(name))
}

/// 按文件顺序取出 `notRemoteIpBlocks` 下列出的 IP，保留行尾注释。
pub fn parse_not_remote_ip_blocks(text: &str) -> Vec<GcpIp> {
    let lines: Vec<&str> = text.lines().collect();
    let mut ips = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let Some(indent) = key_indent(lines[index], "notRemoteIpBlocks:") else {
            index += 1;
            continue;
        };
        index += 1;
        while index < lines.len() {
            let line = lines[index];
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                index += 1;
                continue;
            }
            if leading_spaces(line) <= indent {
                break;
            }
            if let Some(ip) = parse_list_item(trimmed) {
                ips.push(ip);
                index += 1;
                continue;
            }
            break;
        }
    }
    ips
}

fn key_indent(line: &str, key: &str) -> Option<usize> {
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') || !trimmed.starts_with(key) {
        return None;
    }
    Some(leading_spaces(line))
}

fn leading_spaces(line: &str) -> usize {
    line.chars().take_while(|ch| *ch == ' ' || *ch == '\t').count()
}

fn parse_list_item(trimmed: &str) -> Option<GcpIp> {
    let rest = trimmed.strip_prefix('-')?.trim();
    if rest.is_empty() {
        return None;
    }
    let (cidr, after) = if rest.starts_with('"') || rest.starts_with('\'') {
        let quote = rest.chars().next()?;
        let body = &rest[quote.len_utf8()..];
        let end = body.find(quote)?;
        (body[..end].trim().to_string(), body[end + quote.len_utf8()..].trim())
    } else {
        split_comment(rest)
    };
    if cidr.is_empty() {
        return None;
    }
    let comment = after
        .strip_prefix('#')
        .map(str::trim)
        .unwrap_or("")
        .to_string();
    Some(GcpIp { cidr, comment })
}

fn split_comment(text: &str) -> (String, &str) {
    if let Some(mark) = text.find('#') {
        (text[..mark].trim().to_string(), text[mark..].trim())
    } else {
        (text.trim().to_string(), "")
    }
}

/// 同一 IP 占同一行，某列没有该 IP 时留空，整表按 IP 从小到大排。
pub fn align_ip_lists(lists: &[Vec<GcpIp>]) -> Vec<GcpAlignedRow> {
    let mut rows: Vec<Vec<GcpAlignedCell>> = Vec::new();
    let mut width = 0;
    for list in lists {
        if width == 0 {
            rows = list.iter().cloned().map(|ip| vec![cell_of(ip)]).collect();
        } else {
            rows = align_next(&rows, list, width);
        }
        width += 1;
    }
    rows.sort_by(|left, right| ip_ord(&row_cidr(left)).cmp(&ip_ord(&row_cidr(right))));
    rows.into_iter().map(|cells| GcpAlignedRow { cells }).collect()
}

fn row_cidr(row: &[GcpAlignedCell]) -> String {
    row.iter()
        .find(|cell| !cell.cidr.is_empty())
        .map(|cell| cell.cidr.clone())
        .unwrap_or_default()
}

fn ip_ord(cidr: &str) -> (u8, u128, u16, String) {
    let (addr, prefix) = match cidr.rsplit_once('/') {
        Some((addr, prefix)) => (addr, prefix.parse::<u16>().unwrap_or(u16::MAX)),
        None => (cidr, u16::MAX),
    };
    match addr.parse::<std::net::IpAddr>() {
        Ok(std::net::IpAddr::V4(ip)) => (4, u32::from(ip) as u128, prefix, String::new()),
        Ok(std::net::IpAddr::V6(ip)) => (6, u128::from(ip), prefix, String::new()),
        Err(_) => (255, 0, prefix, cidr.to_string()),
    }
}

fn cell_of(ip: GcpIp) -> GcpAlignedCell {
    GcpAlignedCell {
        cidr: ip.cidr,
        comment: ip.comment,
    }
}

fn gap_cell() -> GcpAlignedCell {
    GcpAlignedCell {
        cidr: String::new(),
        comment: String::new(),
    }
}

fn align_next(rows: &[Vec<GcpAlignedCell>], list: &[GcpIp], width: usize) -> Vec<Vec<GcpAlignedCell>> {
    let left: Vec<String> = rows
        .iter()
        .map(|row| {
            row.iter()
                .find(|cell| !cell.cidr.is_empty())
                .map(|cell| cell.cidr.clone())
                .unwrap_or_default()
        })
        .collect();
    let right: Vec<String> = list.iter().map(|ip| ip.cidr.clone()).collect();
    let pairs = lcs_pairs(&left, &right);
    let mut aligned = Vec::new();
    let mut left_index = 0;
    let mut right_index = 0;
    for (matched_left, matched_right) in pairs {
        while left_index < matched_left {
            let mut row = rows[left_index].clone();
            row.push(gap_cell());
            aligned.push(row);
            left_index += 1;
        }
        while right_index < matched_right {
            let mut row = vec![gap_cell(); width];
            row.push(cell_of(list[right_index].clone()));
            aligned.push(row);
            right_index += 1;
        }
        let mut row = rows[left_index].clone();
        row.push(cell_of(list[right_index].clone()));
        aligned.push(row);
        left_index += 1;
        right_index += 1;
    }
    while left_index < rows.len() {
        let mut row = rows[left_index].clone();
        row.push(gap_cell());
        aligned.push(row);
        left_index += 1;
    }
    while right_index < list.len() {
        let mut row = vec![gap_cell(); width];
        row.push(cell_of(list[right_index].clone()));
        aligned.push(row);
        right_index += 1;
    }
    aligned
}

fn lcs_pairs(left: &[String], right: &[String]) -> Vec<(usize, usize)> {
    let rows = left.len();
    let cols = right.len();
    let mut scores = vec![vec![0u16; cols + 1]; rows + 1];
    for i in 1..=rows {
        for j in 1..=cols {
            if left[i - 1] == right[j - 1] && !left[i - 1].is_empty() {
                scores[i][j] = scores[i - 1][j - 1] + 1;
            } else {
                scores[i][j] = scores[i - 1][j].max(scores[i][j - 1]);
            }
        }
    }
    let mut pairs = Vec::new();
    let mut i = rows;
    let mut j = cols;
    while i > 0 && j > 0 {
        if left[i - 1] == right[j - 1] && !left[i - 1].is_empty() && scores[i][j] == scores[i - 1][j - 1] + 1
        {
            pairs.push((i - 1, j - 1));
            i -= 1;
            j -= 1;
        } else if scores[i - 1][j] >= scores[i][j - 1] {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    pairs.reverse();
    pairs
}

struct ApplyEnv {
    key: &'static str,
    title: &'static str,
    context: &'static str,
    project: &'static str,
    relative: &'static str,
}

const APPLY_ENVS: &[ApplyEnv] = &[
    ApplyEnv {
        key: "connect-prod",
        title: "mclinica-connect-prod",
        context: "gke_mclinica-connect_asia-southeast1-a_v01-connect-k8s-cluster-production",
        project: "mclinica-connect",
        relative: "deploy/mclinica-connect-prod/psplink-ip-whitelist-production.yaml",
    },
    ApplyEnv {
        key: "connect-staging",
        title: "mclinica-connect-staging",
        context: "gke_mclinica-connect_asia-southeast1-a_v01-connect-k8s-cluster-production",
        project: "mclinica-connect",
        relative: "deploy/mclinica-connect-staging/psplink-ip-whitelist-staging.yaml",
    },
    ApplyEnv {
        key: "indo-prod",
        title: "psplink-indo-prod",
        context: "gke_psplink-indo-prod_asia-southeast2_psplink-indo-prod-gke-cluster",
        project: "psplink-indo-prod",
        relative: "deploy/psplink-indo-prod/psplink-ip-whitelist-production.yaml",
    },
    ApplyEnv {
        key: "indo-staging",
        title: "psplink-indo-staging",
        context: "gke_psplink-indo-staging_asia-southeast2-a_psplink-indo-staging-gke-cluster",
        project: "psplink-indo-staging",
        relative: "deploy/psplink-indo-staging/psplink-ip-whitelist-staging.yaml",
    },
];

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GcpApplyLog {
    pub text: String,
    pub tone: String,
    pub done: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GcpApplyResult {
    pub code: String,
}

/// 每个环境依次：切换 context、切换 project、kubectl apply -f。
fn apply_commands(root: &Path, env: &ApplyEnv) -> Vec<Vec<String>> {
    let file = root.join(env.relative);
    vec![
        vec!["kubectl".into(), "config".into(), "use-context".into(), env.context.into()],
        vec!["gcloud".into(), "config".into(), "set".into(), "project".into(), env.project.into()],
        vec!["kubectl".into(), "apply".into(), "-f".into(), file.display().to_string()],
    ]
}

fn command_args(command: &[String]) -> Vec<&str> {
    command.iter().skip(1).map(String::as_str).collect()
}

fn selected_apply_envs(keys: &[String]) -> Vec<&'static ApplyEnv> {
    APPLY_ENVS.iter().filter(|env| keys.iter().any(|key| key == env.key)).collect()
}

fn local_stamp() -> String {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as libc::time_t).unwrap_or(0);
    #[cfg(unix)]
    unsafe {
        let mut tm = std::mem::zeroed();
        if !libc::localtime_r(&secs, &mut tm).is_null() {
            return format!("{:02}:{:02}:{:02}", tm.tm_hour, tm.tm_min, tm.tm_sec);
        }
    }
    let secs = secs.max(0) as u64;
    format!("{:02}:{:02}:{:02}Z", (secs / 3600) % 24, (secs / 60) % 60, secs % 60)
}

fn emit_on(app: &tauri::AppHandle, event: &str, text: impl AsRef<str>, tone: &str, done: bool) {
    let _ = app.emit(
        event,
        GcpApplyLog {
            text: format!("[{}] {}", local_stamp(), text.as_ref()),
            tone: tone.to_string(),
            done,
        },
    );
}

fn emit_log(app: &tauri::AppHandle, text: impl AsRef<str>, tone: &str) {
    emit_on(app, "gcp-apply-log", text, tone, false);
}

pub fn emit_apply_done(app: &tauri::AppHandle, text: impl AsRef<str>, tone: &str) {
    emit_on(app, "gcp-apply-log", text, tone, true);
}

fn format_command(argv: &[String]) -> String {
    let parts: Vec<String> = argv.iter().map(|part| {
        if part.is_empty() || part.contains(char::is_whitespace) {
            format!("\"{part}\"")
        } else {
            part.clone()
        }
    }).collect();
    format!("$ {}", parts.join(" "))
}

fn resolve_tool(app: &tauri::AppHandle, name: &str) -> Option<PathBuf> {
    emit_log(app, format!("正在查找 {name}"), "out");
    if let Some(path) = tool_in_path(name) {
        emit_log(app, format!("在 PATH 中找到 {name}：{}", path.display()), "ok");
        return Some(path);
    }
    if let Some(path) = tool_in_known_dirs(name) {
        emit_log(app, format!("在常见安装目录找到 {name}：{}", path.display()), "ok");
        return Some(path);
    }
    let fallback = if cfg!(windows) { "where" } else { "交互式登录 shell" };
    emit_log(app, format!("PATH 和常见目录里没有 {name}，改用 {fallback} 查找，最多等 8 秒"), "warn");
    if let Some(path_env) = std::env::var_os("PATH") {
        emit_log(app, format!("当前 PATH：{}", path_env.to_string_lossy()), "out");
    }
    match tool_in_login_shell(name) {
        Some(path) => {
            emit_log(app, format!("{fallback} 找到 {name}：{}", path.display()), "ok");
            Some(path)
        }
        None => {
            emit_log(app, missing_tool_message(name), "error");
            None
        }
    }
}

fn tool_in_known_dirs(name: &str) -> Option<PathBuf> {
    for dir in known_tool_dirs() {
        for candidate in tool_candidates(&dir, name) {
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn known_tool_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if cfg!(windows) {
        for key in ["ProgramFiles", "ProgramFiles(x86)", "ProgramData", "LOCALAPPDATA"] {
            if let Some(value) = std::env::var_os(key) {
                let root = PathBuf::from(value);
                dirs.push(root.join("Google/Cloud SDK/google-cloud-sdk/bin"));
                dirs.push(root.join("chocolatey/bin"));
            }
        }
    } else {
        dirs.extend([
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from("/opt/homebrew/share/google-cloud-sdk/bin"),
            PathBuf::from("/usr/local/bin"),
            PathBuf::from("/usr/local/share/google-cloud-sdk/bin"),
            PathBuf::from("/usr/bin"),
            PathBuf::from("/bin"),
            PathBuf::from("/usr/lib/google-cloud-sdk/bin"),
            PathBuf::from("/snap/google-cloud-sdk/current/bin"),
            PathBuf::from("/snap/google-cloud-cli/current/bin"),
        ]);
    }
    for key in ["HOME", "USERPROFILE"] {
        if let Some(value) = std::env::var_os(key) {
            let home = PathBuf::from(value);
            dirs.extend(cloud_sdk_dirs(&home));
            dirs.push(home.join("bin"));
            dirs.push(home.join(".local/bin"));
            dirs.push(home.join("AppData/Local/Google/Cloud SDK/google-cloud-sdk/bin"));
            dirs.push(home.join("scoop/apps/gcloud/current/bin"));
            dirs.push(home.join("scoop/shims"));
        }
    }
    dirs
}

/// 官方安装器默认写到家目录，自定义前缀常见于 software、Downloads。
fn cloud_sdk_dirs(home: &Path) -> Vec<PathBuf> {
    [
        "google-cloud-sdk/bin",
        "software/google-cloud-sdk/bin",
        "Downloads/google-cloud-sdk/bin",
        "sdk/google-cloud-sdk/bin",
        "src/google-cloud-sdk/bin",
        "tools/google-cloud-sdk/bin",
        "opt/google-cloud-sdk/bin",
        ".google-cloud-sdk/bin",
    ]
    .into_iter()
    .map(|relative| home.join(relative))
    .collect()
}

fn missing_tool_message(name: &str) -> String {
    format!("找不到 {name} 命令。打包后的应用没有终端 PATH，已检查系统 PATH、常见安装目录，以及会读取 ~/.zshrc 或 ~/.bashrc 的交互式登录 shell")
}

fn tool_in_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        for candidate in tool_candidates(&dir, name) {
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn tool_candidates(dir: &Path, name: &str) -> Vec<PathBuf> {
    if cfg!(windows) {
        vec![dir.join(format!("{name}.exe")), dir.join(format!("{name}.cmd")), dir.join(format!("{name}.bat")), dir.join(name)]
    } else {
        vec![dir.join(name)]
    }
}

fn tool_in_login_shell(name: &str) -> Option<PathBuf> {
    if name != "gcloud" && name != "kubectl" && name != "git" {
        return None;
    }
    let text = command_lookup_text(name)?;
    for line in text.lines() {
        let path = PathBuf::from(line.trim());
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

fn command_lookup_text(name: &str) -> Option<String> {
    #[cfg(windows)]
    {
        let mut child = Command::new("where.exe").arg(name).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()?;
        return read_child_text(&mut child, Duration::from_secs(8));
    }
    #[cfg(not(windows))]
    {
        let script = format!("command -v {name}");
        for shell in lookup_shells() {
            if !Path::new(&shell).is_file() {
                continue;
            }
            let mut command = Command::new(&shell);
            command.args(shell_lookup_args(&shell)).arg(&script).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
            let Some(mut child) = command.spawn().ok() else { continue };
            let Some(text) = read_child_text(&mut child, Duration::from_secs(8)) else { continue };
            if text.lines().any(|line| PathBuf::from(line.trim()).is_file()) {
                return Some(text);
            }
        }
        None
    }
}

fn lookup_shells() -> Vec<String> {
    let mut shells = Vec::new();
    if let Ok(shell) = std::env::var("SHELL") {
        if !shell.is_empty() {
            shells.push(shell);
        }
    }
    for fallback in ["/bin/zsh", "/bin/bash", "/bin/sh"] {
        if !shells.iter().any(|item| item == fallback) {
            shells.push(fallback.to_string());
        }
    }
    shells
}

/// 打包后的应用 PATH 只有 /usr/bin:/bin。gcloud 安装器把路径写进 ~/.zshrc，必须用交互式登录 shell 才能读到。
fn shell_lookup_args(shell: &str) -> Vec<&'static str> {
    if shell.ends_with("fish") { vec!["-l", "-c"] } else { vec!["-lic"] }
}

fn read_child_text(child: &mut std::process::Child, timeout: Duration) -> Option<String> {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return None;
                }
                let mut text = String::new();
                child.stdout.as_mut()?.read_to_string(&mut text).ok()?;
                return Some(text);
            }
            Ok(None) if started.elapsed() > timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(_) => return None,
        }
    }
}

fn is_batch_script(bin: &Path) -> bool {
    matches!(
        bin.extension().and_then(|ext| ext.to_str()).map(|ext| ext.to_ascii_lowercase()).as_deref(),
        Some("cmd") | Some("bat")
    )
}

/// Windows 命令行引号规则。路径里有空格时，`.cmd` 必须整段交给 cmd.exe。
fn quote_windows(arg: &str) -> String {
    if !arg.is_empty() && !arg.chars().any(|ch| " \t\"&|<>^%()".contains(ch)) {
        return arg.to_string();
    }
    let mut out = String::from('"');
    let mut slashes = 0;
    for ch in arg.chars() {
        if ch == '\\' {
            slashes += 1;
            continue;
        }
        if ch == '"' {
            out.push_str(&"\\".repeat(slashes * 2 + 1));
            out.push('"');
            slashes = 0;
            continue;
        }
        if slashes > 0 {
            out.push_str(&"\\".repeat(slashes));
            slashes = 0;
        }
        out.push(ch);
    }
    if slashes > 0 {
        out.push_str(&"\\".repeat(slashes * 2));
    }
    out.push('"');
    out
}

fn hide_console(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    {
        let _ = command;
    }
}

fn tool_command(bin: &Path, args: &[&str]) -> (Command, String) {
    if cfg!(windows) && is_batch_script(bin) {
        let comspec = std::env::var("ComSpec").unwrap_or_else(|_| "cmd.exe".into());
        let mut cmdline = quote_windows(&bin.to_string_lossy());
        for arg in args {
            cmdline.push(' ');
            cmdline.push_str(&quote_windows(arg));
        }
        let mut command = Command::new(&comspec);
        command.arg("/D").arg("/S").arg("/C").arg(&cmdline);
        hide_console(&mut command);
        (command, format!("{comspec} /D /S /C {cmdline}"))
    } else {
        let mut command = Command::new(bin);
        command.args(args);
        hide_console(&mut command);
        (command, "直接执行".into())
    }
}

fn prepend_bin_path(command: &mut Command, bin: &Path) {
    let Some(parent) = bin.parent() else { return };
    let mut dirs = vec![parent.to_path_buf()];
    if let Some(current) = std::env::var_os("PATH") {
        dirs.extend(std::env::split_paths(&current));
    }
    if let Ok(joined) = std::env::join_paths(dirs) {
        command.env("PATH", joined);
    }
}

/// gcloud 把正常结果也写到 stderr。只有真正的错误和警告才上色。
fn line_tone(stream: &str, text: &str) -> &'static str {
    if stream != "stderr" {
        return "out";
    }
    let trimmed = text.trim().to_ascii_lowercase();
    if trimmed.starts_with("error")
        || trimmed.contains("fatal")
        || trimmed.contains("traceback")
        || trimmed.contains("exception")
        || trimmed.contains("unauthorized")
        || trimmed.contains("permission denied")
    {
        return "error";
    }
    if trimmed.starts_with("warning") || trimmed.starts_with("warn:") || trimmed.contains("warning:") {
        return "warn";
    }
    "out"
}

fn explain_failure(text: &str) -> Option<&'static str> {
    let trimmed = text.to_ascii_lowercase();
    if trimmed.contains("failed to download openapi") || (trimmed.contains("dial tcp") && trimmed.contains("i/o timeout")) {
        Some("原因：本机连不上 Kubernetes API。kubectl apply 会先下载集群 OpenAPI 做校验，这一步 TCP 超时。通常是没连上能访问该集群的网络、当前 IP 不在 GKE 授权网络中，或 kubeconfig 里的 API 地址已失效。这不是 YAML 内容错误；关掉校验后，apply 仍然要访问同一个地址。")
    } else {
        None
    }
}

fn run_tool(app: &tauri::AppHandle, display: &str, bin: &Path, args: &[&str], cwd: Option<&Path>, event: &'static str) -> bool {
    let mut argv = vec![display.to_string()];
    argv.extend(args.iter().map(|arg| (*arg).to_string()));
    emit_on(app, event, format_command(&argv), "cmd", false);
    emit_on(app, event, format!("可执行文件：{}", bin.display()), "out", false);
    if let Some(dir) = cwd {
        emit_on(app, event, format!("工作目录：{}", dir.display()), "out", false);
    }
    let started = Instant::now();
    let (mut command, launch) = tool_command(bin, args);
    emit_on(app, event, format!("启动方式：{launch}"), "out", false);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("CLOUDSDK_CORE_DISABLE_PROMPTS", "1")
        .env("CLOUDSDK_COMPONENT_MANAGER_DISABLE_UPDATE_CHECK", "1")
        .env("CLOUDSDK_CORE_DISABLE_USAGE_REPORTING", "1")
        .env("GIT_TERMINAL_PROMPT", "0");
    if let Some(dir) = cwd {
        command.current_dir(dir);
    }
    prepend_bin_path(&mut command, bin);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            emit_on(app, event, format!("无法启动 {display}：{error}"), "error", false);
            return false;
        }
    };
    emit_on(app, event, format!("{display} 已启动，进程号 {}", child.id()), "out", false);
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let app_out = app.clone();
    let app_err = app.clone();
    let stdout_thread = thread::spawn(move || {
        if let Some(stdout) = stdout {
            read_pipe(stdout, &app_out, "stdout", event);
        }
    });
    let stderr_thread = thread::spawn(move || {
        if let Some(stderr) = stderr {
            read_pipe(stderr, &app_err, "stderr", event);
        }
    });
    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    let app_beat = app.clone();
    let beat_name = display.to_string();
    let beat_hint = if args.iter().any(|arg| *arg == "apply") {
        "正在连接集群 API。校验或提交期间可能没有新输出，命令仍在执行"
    } else {
        "暂时没有新输出，命令仍在执行"
    };
    let pid = child.id();
    let beat_thread = thread::spawn(move || {
        let mut waited = 0u64;
        loop {
            match stop_rx.recv_timeout(Duration::from_secs(5)) {
                Ok(_) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    waited += 5;
                    emit_on(
                        &app_beat,
                        event,
                        format!("{beat_name} 仍在执行，进程号 {pid}，已等待 {waited} 秒。{beat_hint}"),
                        "busy",
                        false,
                    );
                }
            }
        }
    });
    let status = match child.wait() {
        Ok(status) => status,
        Err(error) => {
            let _ = stop_tx.send(());
            emit_on(app, event, format!("等待 {display} 结束失败：{error}"), "error", false);
            let _ = stdout_thread.join();
            let _ = stderr_thread.join();
            let _ = beat_thread.join();
            return false;
        }
    };
    let _ = stop_tx.send(());
    let _ = stdout_thread.join();
    let _ = stderr_thread.join();
    let _ = beat_thread.join();
    let elapsed = format!("{:.1} 秒", started.elapsed().as_secs_f32());
    if status.success() {
        emit_on(app, event, format!("{display} 完成，退出码 0，用时 {elapsed}"), "ok", false);
        true
    } else {
        let code = status.code().map(|value| value.to_string()).unwrap_or_else(|| "信号中断".into());
        emit_on(app, event, format!("{display} 失败，退出码 {code}，用时 {elapsed}"), "error", false);
        false
    }
}

fn read_pipe<R: Read>(reader: R, app: &tauri::AppHandle, stream: &str, event: &str) {
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                let text = line.trim_end_matches(['\r', '\n']);
                if text.is_empty() {
                    continue;
                }
                let tone = line_tone(stream, text);
                emit_on(app, event, text, tone, false);
                if let Some(reason) = explain_failure(text) {
                    emit_on(app, event, reason, "error", false);
                }
            }
            Err(error) => {
                emit_on(app, event, format!("读取 {stream} 失败：{error}"), "error", false);
                break;
            }
        }
    }
}

fn ensure_tool(app: &tauri::AppHandle, name: &str, args: &[&str]) -> Option<PathBuf> {
    let bin = resolve_tool(app, name)?;
    emit_log(app, format!("开始执行 {name}，确认命令可用"), "out");
    if !run_tool(app, name, &bin, args, None, "gcp-apply-log") {
        emit_log(app, format!("{name} 不可用，已停止"), "error");
        return None;
    }
    Some(bin)
}

fn emit_env_rule(app: &tauri::AppHandle, title: &str) {
    emit_log(app, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━", "rule");
    emit_log(app, title, "rule");
    emit_log(app, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━", "rule");
}

pub fn apply_whitelist(app: &tauri::AppHandle, path: &str, keys: &[String]) -> GcpApplyResult {
    let root = PathBuf::from(path);
    emit_log(app, "开始执行应用到 GCP", "out");
    emit_log(app, format!("运行系统：{} {}", std::env::consts::OS, std::env::consts::ARCH), "out");
    emit_log(app, "已关闭 gcloud 交互提示和组件更新检查，避免命令停在等待输入", "out");
    emit_log(app, format!("项目路径：{}", root.display()), "out");
    emit_log(app, format!("勾选环境：{}", if keys.is_empty() { "（无）".into() } else { keys.join("、") }), "out");
    if !root.is_dir() {
        return finish(app, "项目路径不是目录，已停止", "error", "stopped");
    }
    let Some(gcloud) = ensure_tool(app, "gcloud", &["version"]) else {
        return finish(app, "gcloud 检查未通过，已停止", "error", "stopped");
    };
    let Some(kubectl) = ensure_tool(app, "kubectl", &["version", "--client"]) else {
        return finish(app, "kubectl 检查未通过，已停止", "error", "stopped");
    };
    let selected = selected_apply_envs(keys);
    if selected.is_empty() {
        return finish(app, "没有勾选环境，已停止", "error", "stopped");
    }
    emit_log(app, format!("将按顺序处理 {} 个环境", selected.len()), "out");
    let mut failed = false;
    for env in selected {
        emit_env_rule(app, env.title);
        emit_log(app, format!("YAML：{}", env.relative), "out");
        let file = root.join(env.relative);
        if !file.is_file() {
            emit_log(app, format!("找不到 {}，已跳过该环境", file.display()), "error");
            failed = true;
            continue;
        }
        let commands = apply_commands(&root, env);
        let context_ok = run_tool(app, "kubectl", &kubectl, &command_args(&commands[0]), Some(&root), "gcp-apply-log");
        let project_ok = run_tool(app, "gcloud", &gcloud, &command_args(&commands[1]), Some(&root), "gcp-apply-log");
        if !context_ok || !project_ok {
            emit_log(app, "切换失败，已跳过 kubectl apply", "error");
            failed = true;
            continue;
        }
        if !run_tool(app, "kubectl", &kubectl, &command_args(&commands[2]), Some(&root), "gcp-apply-log") {
            failed = true;
        }
    }
    if failed {
        finish(app, "执行结束，有环境失败", "error", "failed")
    } else {
        finish(app, "执行结束", "ok", "ok")
    }
}

fn finish(app: &tauri::AppHandle, text: &str, tone: &str, code: &str) -> GcpApplyResult {
    emit_apply_done(app, text, tone);
    GcpApplyResult { code: code.to_string() }
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct GcpFetchUpdate {
    key: String,
    status: String,
    message: String,
    yaml: String,
    replace_yaml: bool,
    done: bool,
}

fn emit_fetch_to(app: &tauri::AppHandle, key: &str, status: &str, message: impl AsRef<str>, yaml: &str, replace_yaml: bool, done: bool) {
    if key.is_empty() && !done {
        let text = message.as_ref().to_string();
        for env in APPLY_ENVS {
            emit_fetch(app, env.key, status, &text, yaml, replace_yaml, false);
        }
        return;
    }
    emit_fetch(app, key, status, message, yaml, replace_yaml, done);
}

fn emit_fetch(app: &tauri::AppHandle, key: &str, status: &str, message: impl AsRef<str>, yaml: &str, replace_yaml: bool, done: bool) {
    let _ = app.emit(
        "gcp-fetch-update",
        GcpFetchUpdate {
            key: key.to_string(),
            status: status.to_string(),
            message: message.as_ref().to_string(),
            yaml: yaml.to_string(),
            replace_yaml,
            done,
        },
    );
}

pub fn emit_fetch_done(app: &tauri::AppHandle, message: &str) {
    emit_fetch(app, "", "error", message, "", false, true);
}

struct CommandCapture {
    ok: bool,
    stdout: String,
    stderr: String,
}

fn run_capture(app: &tauri::AppHandle, key: &str, display: &str, bin: &Path, args: &[&str], cwd: Option<&Path>) -> CommandCapture {
    let started = Instant::now();
    let (mut command, _) = tool_command(bin, args);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("CLOUDSDK_CORE_DISABLE_PROMPTS", "1")
        .env("CLOUDSDK_COMPONENT_MANAGER_DISABLE_UPDATE_CHECK", "1")
        .env("CLOUDSDK_CORE_DISABLE_USAGE_REPORTING", "1");
    if let Some(dir) = cwd {
        command.current_dir(dir);
    }
    prepend_bin_path(&mut command, bin);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return CommandCapture { ok: false, stdout: String::new(), stderr: format!("无法启动 {display}：{error}") };
        }
    };
    let pid = child.id();
    emit_fetch_to(app, key, "running", format!("{display} 已启动，进程号 {pid}"), "", false, false);
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdout_thread = thread::spawn(move || stdout.map(read_to_string).unwrap_or_default());
    let stderr_thread = thread::spawn(move || stderr.map(read_to_string).unwrap_or_default());
    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    let app_beat = app.clone();
    let beat_key = key.to_string();
    let beat_name = display.to_string();
    let beat_thread = thread::spawn(move || {
        let mut waited = 0u64;
        loop {
            match stop_rx.recv_timeout(Duration::from_secs(5)) {
                Ok(_) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    waited += 5;
                    emit_fetch_to(
                        &app_beat,
                        &beat_key,
                        "running",
                        format!("{beat_name} 仍在执行，已等待 {waited} 秒"),
                        "",
                        false,
                        false,
                    );
                }
            }
        }
    });
    let status = match child.wait() {
        Ok(status) => status,
        Err(error) => {
            let _ = stop_tx.send(());
            let _ = stdout_thread.join();
            let _ = stderr_thread.join();
            let _ = beat_thread.join();
            return CommandCapture { ok: false, stdout: String::new(), stderr: format!("等待 {display} 结束失败：{error}") };
        }
    };
    let _ = stop_tx.send(());
    let stdout = stdout_thread.join().unwrap_or_default();
    let stderr = stderr_thread.join().unwrap_or_default();
    let _ = beat_thread.join();
    let elapsed = format!("{:.1} 秒", started.elapsed().as_secs_f32());
    if status.success() {
        emit_fetch_to(app, key, "running", format!("{display} 完成，用时 {elapsed}"), "", false, false);
        CommandCapture { ok: true, stdout, stderr }
    } else {
        let code = status.code().map(|value| value.to_string()).unwrap_or_else(|| "信号中断".into());
        let mut detail = stderr.trim().to_string();
        if detail.is_empty() {
            detail = stdout.trim().to_string();
        }
        if let Some(reason) = explain_failure(&detail) {
            detail = format!("{detail}\n\n{reason}");
        }
        if detail.is_empty() {
            detail = format!("{display} 失败，退出码 {code}，用时 {elapsed}");
        } else {
            detail = format!("{detail}\n\n{display} 失败，退出码 {code}，用时 {elapsed}");
        }
        CommandCapture { ok: false, stdout, stderr: detail }
    }
}

fn read_to_string<R: Read>(reader: R) -> String {
    let mut reader = BufReader::new(reader);
    let mut all = String::new();
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => all.push_str(&line),
        }
    }
    all
}

fn fetch_get_args(env: &ApplyEnv) -> Vec<String> {
    vec!["kubectl".into(), "get".into(), "-f".into(), env.relative.into(), "-o".into(), "yaml".into()]
}

pub fn fetch_whitelist_from_gcp(app: &tauri::AppHandle, path: &str) {
    let root = PathBuf::from(path);
    if !root.is_dir() {
        for env in APPLY_ENVS {
            emit_fetch(app, env.key, "error", "项目路径不是目录", "项目路径不是目录", true, false);
        }
        emit_fetch_done(app, "项目路径不是目录，已停止");
        return;
    }
    let Some(gcloud) = find_tool("gcloud") else {
        let message = missing_tool_message("gcloud");
        for env in APPLY_ENVS {
            emit_fetch(app, env.key, "error", "找不到 gcloud 命令", &message, true, false);
        }
        emit_fetch_done(app, "找不到 gcloud 命令，已停止");
        return;
    };
    let Some(kubectl) = find_tool("kubectl") else {
        let message = missing_tool_message("kubectl");
        for env in APPLY_ENVS {
            emit_fetch(app, env.key, "error", "找不到 kubectl 命令", &message, true, false);
        }
        emit_fetch_done(app, "找不到 kubectl 命令，已停止");
        return;
    };
    for env in APPLY_ENVS {
        emit_fetch(app, env.key, "running", format!("检查 gcloud：{}", gcloud.display()), "", true, false);
    }
    let gcloud_check = run_capture(app, "", "gcloud", &gcloud, &["version"], None);
    if !gcloud_check.ok {
        let detail = if gcloud_check.stderr.is_empty() { "gcloud version 失败，已停止".into() } else { gcloud_check.stderr };
        for env in APPLY_ENVS {
            emit_fetch(app, env.key, "error", "gcloud 不可用", &detail, true, false);
        }
        emit_fetch_done(app, "gcloud 不可用，已停止");
        return;
    }
    for env in APPLY_ENVS {
        emit_fetch(app, env.key, "running", format!("检查 kubectl：{}", kubectl.display()), "", false, false);
    }
    let kubectl_check = run_capture(app, "", "kubectl", &kubectl, &["version", "--client"], None);
    if !kubectl_check.ok {
        let detail = if kubectl_check.stderr.is_empty() { "kubectl version --client 失败，已停止".into() } else { kubectl_check.stderr };
        for env in APPLY_ENVS {
            emit_fetch(app, env.key, "error", "kubectl 不可用", &detail, true, false);
        }
        emit_fetch_done(app, "kubectl 不可用，已停止");
        return;
    }
    for env in APPLY_ENVS {
        let file = root.join(env.relative);
        emit_fetch(app, env.key, "running", format!("准备查询 {}", env.relative), "", true, false);
        if !file.is_file() {
            let message = format!("找不到 {}", file.display());
            emit_fetch(app, env.key, "error", &message, &message, true, false);
            continue;
        }
        let context_args = ["config", "use-context", env.context];
        emit_fetch(app, env.key, "running", format!("$ kubectl config use-context {}", env.context), "", false, false);
        let context = run_capture(app, env.key, "kubectl", &kubectl, &context_args, Some(&root));
        if !context.ok {
            emit_fetch(app, env.key, "error", "切换 context 失败，已跳过查询", &context.stderr, true, false);
            continue;
        }
        let project_args = ["config", "set", "project", env.project];
        emit_fetch(app, env.key, "running", format!("$ gcloud config set project {}", env.project), "", false, false);
        let project = run_capture(app, env.key, "gcloud", &gcloud, &project_args, Some(&root));
        if !project.ok {
            emit_fetch(app, env.key, "error", "切换 project 失败，已跳过查询", &project.stderr, true, false);
            continue;
        }
        let get_args = fetch_get_args(env);
        let get_refs: Vec<&str> = get_args.iter().skip(1).map(String::as_str).collect();
        emit_fetch(app, env.key, "running", format!("$ kubectl get -f {} -o yaml", env.relative), "", false, false);
        let fetched = run_capture(app, env.key, "kubectl", &kubectl, &get_refs, Some(&root));
        if fetched.ok {
            let yaml = fetched.stdout.trim_end().to_string();
            let message = if fetched.stderr.trim().is_empty() {
                format!("已读取 {}", env.relative)
            } else {
                format!("已读取 {}。{}", env.relative, fetched.stderr.trim())
            };
            emit_fetch(app, env.key, "ok", message, &yaml, true, false);
        } else {
            emit_fetch(app, env.key, "error", "查询失败", &fetched.stderr, true, false);
        }
    }
    emit_fetch(app, "", "ok", "四个环境已查询结束", "", false, true);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_quoted_ips_and_comments_in_order() {
        let text = r#"
spec:
  rules:
    - from:
        - source:
            notRemoteIpBlocks:
              - "172.16.26.104/16"   # cluster internal
              - "101.52.220.36/32"   # Shanghai
              - "165.225.230.164"
              - "165.225.230.164"    # again
      to:
        - operation:
            hosts:
              - example.com
"#;
        let ips = parse_not_remote_ip_blocks(text);
        assert_eq!(
            ips,
            vec![
                GcpIp { cidr: "172.16.26.104/16".into(), comment: "cluster internal".into() },
                GcpIp { cidr: "101.52.220.36/32".into(), comment: "Shanghai".into() },
                GcpIp { cidr: "165.225.230.164".into(), comment: String::new() },
                GcpIp { cidr: "165.225.230.164".into(), comment: "again".into() },
            ]
        );
    }

    #[test]
    fn aligns_shared_ips_and_leaves_gaps() {
        let lists = vec![
            vec![ip("10.0.0.0/8"), ip("101.52.220.36/32"), ip("162.44.224.30")],
            vec![ip("10.0.0.0/8"), ip("101.52.220.36/32")],
            vec![ip("34.101.111.59/32"), ip("101.52.220.36/32"), ip("162.44.224.30")],
            vec![ip("172.16.26.104/16"), ip("101.52.220.36/32")],
        ];
        let rows = align_ip_lists(&lists);
        assert_eq!(column(&rows, 0), vec!["10.0.0.0/8", "101.52.220.36/32", "162.44.224.30"]);
        assert_eq!(column(&rows, 1), vec!["10.0.0.0/8", "101.52.220.36/32"]);
        assert_eq!(column(&rows, 2), vec!["34.101.111.59/32", "101.52.220.36/32", "162.44.224.30"]);
        assert_eq!(column(&rows, 3), vec!["101.52.220.36/32", "172.16.26.104/16"]);
        assert_eq!(
            rows.iter().map(|row| row_cidr(&row.cells)).collect::<Vec<_>>(),
            vec![
                "10.0.0.0/8",
                "34.101.111.59/32",
                "101.52.220.36/32",
                "162.44.224.30",
                "172.16.26.104/16",
            ]
        );
        let shared = rows
            .iter()
            .find(|row| row.cells.iter().any(|cell| cell.cidr == "101.52.220.36/32"))
            .unwrap();
        assert!(shared.cells.iter().all(|cell| cell.cidr == "101.52.220.36/32"));
        let only_indo = rows
            .iter()
            .find(|row| row.cells.iter().any(|cell| cell.cidr == "172.16.26.104/16"))
            .unwrap();
        assert_eq!(only_indo.cells[3].cidr, "172.16.26.104/16");
        assert!(only_indo.cells[..3].iter().all(|cell| cell.cidr.is_empty()));
    }

    #[test]
    fn aligns_when_an_earlier_file_has_no_ips() {
        let lists = vec![Vec::new(), vec![ip("10.0.0.0/8")], Vec::new(), vec![ip("172.16.26.104/16")]];
        let rows = align_ip_lists(&lists);
        assert!(rows.iter().all(|row| row.cells.len() == 4));
        assert_eq!(column(&rows, 1), vec!["10.0.0.0/8"]);
        assert_eq!(column(&rows, 3), vec!["172.16.26.104/16"]);
        assert!(column(&rows, 0).is_empty());
    }

    #[test]
    fn keeps_duplicate_ips_in_order() {
        let lists = vec![
            vec![ip("165.225.230.164"), ip("162.44.224.30"), ip("165.225.230.164")],
            vec![ip("165.225.230.164")],
        ];
        let rows = align_ip_lists(&lists);
        assert_eq!(
            column(&rows, 0),
            vec!["162.44.224.30", "165.225.230.164", "165.225.230.164"]
        );
        assert_eq!(column(&rows, 1), vec!["165.225.230.164"]);
    }

    #[test]
    fn inspects_master_branch_and_required_files() {
        let dir = std::env::temp_dir().join(format!("cdk-gcp-whitelist-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let missing = inspect_project(dir.join("missing").to_str().unwrap());
        assert!(!missing.directory);
        assert_eq!(missing.files.len(), 4);
        assert!(missing.files.iter().all(|file| !file.exists));

        git(&dir, &["init"]);
        git(&dir, &["symbolic-ref", "HEAD", "refs/heads/master"]);
        for relative in FILES.iter().map(|file| file.2) {
            let full = dir.join(relative);
            fs::create_dir_all(full.parent().unwrap()).unwrap();
            fs::write(
                &full,
                "spec:\n  rules:\n    - from:\n        - source:\n            notRemoteIpBlocks:\n              - \"10.0.0.0/8\" # k8s\n",
            )
            .unwrap();
        }
        let ready = inspect_project(dir.to_str().unwrap());
        assert!(ready.directory);
        assert!(ready.repository, "{}", ready.git_error);
        assert_eq!(ready.branch, "master");
        assert!(ready.on_master);
        assert!(ready.files.iter().all(|file| file.exists && file.ips.len() == 1));
        assert!(ready.rows.iter().any(|row| row.cells.iter().all(|cell| cell.cidr == "10.0.0.0/8")));

        fs::remove_file(dir.join(FILES[3].2)).unwrap();
        let incomplete = inspect_project(dir.to_str().unwrap());
        assert!(!incomplete.files[3].exists);
        assert!(incomplete.files[..3].iter().all(|file| file.exists));

        git(&dir, &["symbolic-ref", "HEAD", "refs/heads/develop"]);
        let other = inspect_project(dir.to_str().unwrap());
        assert_eq!(other.branch, "develop");
        assert!(!other.on_master);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn reads_the_real_central_devops_lists_when_present() {
        let root = Path::new("/Users/tb/workspace_psp/central-devops");
        if !root.join(FILES[0].2).is_file() {
            return;
        }
        let result = inspect_project(root.to_str().unwrap());
        assert!(result.files.iter().all(|file| file.exists && file.error.is_empty()));
        let counts: Vec<usize> = result.files.iter().map(|file| file.ips.len()).collect();
        assert!(counts[0] >= 23 && counts[1] >= 19 && counts[2] >= 23 && counts[3] >= 21, "{counts:?}");
        assert_eq!(result.files[3].ips[0].cidr, "172.16.26.104/16");
        assert_eq!(result.files[3].ips[0].comment, "cluster internal");
        for (index, file) in result.files.iter().enumerate() {
            let mut expected: Vec<String> = file.ips.iter().map(|ip| ip.cidr.clone()).collect();
            expected.sort_by(|left, right| ip_ord(left).cmp(&ip_ord(right)));
            assert_eq!(column(&result.rows, index), expected);
        }
        let shown: Vec<String> = result.rows.iter().map(|row| row_cidr(&row.cells)).collect();
        let mut sorted = shown.clone();
        sorted.sort_by(|left, right| ip_ord(left).cmp(&ip_ord(right)));
        assert_eq!(shown, sorted);
        let shanghai = result
            .rows
            .iter()
            .find(|row| row.cells.iter().any(|cell| cell.cidr == "101.52.220.36/32"))
            .unwrap();
        assert!(shanghai.cells.iter().all(|cell| cell.cidr == "101.52.220.36/32"));
        let pod = result
            .rows
            .iter()
            .find(|row| row.cells.iter().any(|cell| cell.cidr == "172.16.20.0/23"))
            .unwrap();
        assert!(pod.cells[..3].iter().all(|cell| cell.cidr.is_empty()));
        assert_eq!(pod.cells[3].cidr, "172.16.20.0/23");
    }

    #[test]
    fn appends_an_ip_before_the_following_section() {
        let text = "spec:\n  rules:\n    - from:\n        - source:\n            notRemoteIpBlocks:\n              - \"10.0.0.0/8\" # k8s\n      to:\n        - operation:\n";
        let updated = append_not_remote_ip(text, "203.0.113.10/32", "Office").unwrap();
        let ips = parse_not_remote_ip_blocks(&updated);
        assert_eq!(ips.len(), 2);
        assert_eq!(ips[1].cidr, "203.0.113.10/32");
        assert_eq!(ips[1].comment, "Office");
        assert!(updated.find("203.0.113.10/32").unwrap() < updated.find("      to:").unwrap());
        let again = append_not_remote_ip(&updated, "203.0.113.10/32", "Office");
        assert!(again.is_ok());
    }

    #[test]
    fn adds_ip_only_to_the_selected_environments() {
        let dir = std::env::temp_dir().join(format!("cdk-gcp-add-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        git(&dir, &["init"]);
        git(&dir, &["symbolic-ref", "HEAD", "refs/heads/master"]);
        for relative in FILES.iter().map(|file| file.2) {
            let full = dir.join(relative);
            fs::create_dir_all(full.parent().unwrap()).unwrap();
            fs::write(
                &full,
                "spec:\n  rules:\n    - from:\n        - source:\n            notRemoteIpBlocks:\n              - \"10.0.0.0/8\" # k8s\n      to:\n        - operation:\n",
            )
            .unwrap();
        }
        let path = dir.to_str().unwrap();
        let keys = vec!["connect-prod".to_string(), "indo-staging".to_string()];
        let added = add_whitelist_ip(path, "203.0.113.10/32", "Office", &keys);
        assert_eq!(added.code, "ok");
        assert_eq!(added.added, keys);
        assert!(added.inspect.files[0].ips.iter().any(|ip| ip.cidr == "203.0.113.10/32" && ip.comment == "Office"));
        assert!(added.inspect.files[1].ips.iter().all(|ip| ip.cidr != "203.0.113.10/32"));
        assert!(added.inspect.files[2].ips.iter().all(|ip| ip.cidr != "203.0.113.10/32"));
        assert!(added.inspect.files[3].ips.iter().any(|ip| ip.cidr == "203.0.113.10/32"));
        let duplicate = add_whitelist_ip(path, "203.0.113.10/32", "Office", &keys);
        assert_eq!(duplicate.code, "duplicate");
        assert!(duplicate.added.is_empty());
        git(&dir, &["symbolic-ref", "HEAD", "refs/heads/develop"]);
        let blocked = add_whitelist_ip(path, "198.51.100.8/32", "Other", &keys);
        assert_eq!(blocked.code, "not_master");
        assert!(blocked.inspect.files[0].ips.iter().all(|ip| ip.cidr != "198.51.100.8/32"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn replaces_the_list_in_sorted_order_and_keeps_the_rest() {
        let text = "spec:\n  rules:\n    - from:\n        - source:\n            notRemoteIpBlocks:\n              - \"172.16.26.104/16\"   # cluster\n              - \"10.0.0.0/8\" # k8s\n      to:\n        - operation:\n";
        let updated = replace_not_remote_ip_blocks(
            text,
            &[
                GcpIp { cidr: "10.0.0.0/8".into(), comment: "k8s".into() },
                GcpIp { cidr: "203.0.113.10/32".into(), comment: "Office".into() },
            ],
        )
        .unwrap();
        assert!(updated.find("      to:").unwrap() > updated.find("203.0.113.10/32").unwrap());
        assert!(!updated.contains("172.16.26.104/16"));
        let ips = parse_not_remote_ip_blocks(&updated);
        assert_eq!(ips[0].cidr, "10.0.0.0/8");
        assert_eq!(ips[0].comment, "k8s");
        assert_eq!(ips[1].comment, "Office");
    }

    #[test]
    fn saves_sorted_lists_only_on_master() {
        let dir = std::env::temp_dir().join(format!("cdk-gcp-save-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        git(&dir, &["init"]);
        git(&dir, &["symbolic-ref", "HEAD", "refs/heads/develop"]);
        for relative in FILES.iter().map(|file| file.2) {
            let full = dir.join(relative);
            fs::create_dir_all(full.parent().unwrap()).unwrap();
            fs::write(
                &full,
                "spec:\n  rules:\n    - from:\n        - source:\n            notRemoteIpBlocks:\n              - \"172.16.0.1/32\" # late\n              - \"10.0.0.0/8\" # early\n      to:\n        - operation:\n",
            )
            .unwrap();
        }
        let path = dir.to_str().unwrap();
        let files = vec![GcpSaveFile {
            key: "connect-prod".into(),
            ips: vec![
                GcpIp { cidr: "172.16.0.1/32".into(), comment: "late".into() },
                GcpIp { cidr: "10.1.0.0/32".into(), comment: "new".into() },
            ],
        }];
        let blocked = save_whitelist_files(path, &files);
        assert_eq!(blocked.code, "not_master");
        git(&dir, &["symbolic-ref", "HEAD", "refs/heads/master"]);
        let saved = save_whitelist_files(path, &files);
        assert_eq!(saved.code, "ok");
        let prod = &saved.inspect.files[0].ips;
        assert_eq!(prod.iter().map(|ip| ip.cidr.as_str()).collect::<Vec<_>>(), vec!["10.1.0.0/32", "172.16.0.1/32"]);
        assert_eq!(prod[0].comment, "new");
        assert_eq!(saved.inspect.files[1].ips.len(), 2);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn apply_commands_match_the_four_environments() {
        let root = Path::new("/repo/central-devops");
        let cases = [
            (
                "connect-prod",
                "gke_mclinica-connect_asia-southeast1-a_v01-connect-k8s-cluster-production",
                "mclinica-connect",
                "deploy/mclinica-connect-prod/psplink-ip-whitelist-production.yaml",
            ),
            (
                "connect-staging",
                "gke_mclinica-connect_asia-southeast1-a_v01-connect-k8s-cluster-production",
                "mclinica-connect",
                "deploy/mclinica-connect-staging/psplink-ip-whitelist-staging.yaml",
            ),
            (
                "indo-prod",
                "gke_psplink-indo-prod_asia-southeast2_psplink-indo-prod-gke-cluster",
                "psplink-indo-prod",
                "deploy/psplink-indo-prod/psplink-ip-whitelist-production.yaml",
            ),
            (
                "indo-staging",
                "gke_psplink-indo-staging_asia-southeast2-a_psplink-indo-staging-gke-cluster",
                "psplink-indo-staging",
                "deploy/psplink-indo-staging/psplink-ip-whitelist-staging.yaml",
            ),
        ];
        let keys: Vec<String> = cases.iter().map(|item| item.0.to_string()).collect();
        assert_eq!(selected_apply_envs(&keys).len(), 4);
        assert!(selected_apply_envs(&["unknown".into()]).is_empty());
        for (key, context, project, relative) in cases {
            let env = APPLY_ENVS.iter().find(|item| item.key == key).unwrap();
            let commands = apply_commands(root, env);
            let shown: Vec<Vec<&str>> = commands.iter().map(|argv| argv.iter().map(String::as_str).collect()).collect();
            assert_eq!(shown[0], vec!["kubectl", "config", "use-context", context]);
            assert_eq!(shown[1], vec!["gcloud", "config", "set", "project", project]);
            assert_eq!(shown[2][0], "kubectl");
            assert_eq!(shown[2][1], "apply");
            assert_eq!(shown[2][2], "-f");
            assert!(shown[2][3].ends_with(relative));
        }
    }

    #[test]
    fn fetch_reads_each_environment_yaml_with_kubectl_get() {
        for env in APPLY_ENVS {
            let args = fetch_get_args(env);
            assert_eq!(args, vec![
                "kubectl".to_string(),
                "get".into(),
                "-f".into(),
                env.relative.into(),
                "-o".into(),
                "yaml".into(),
            ]);
        }
    }

    #[test]
    fn gcloud_success_text_on_stderr_stays_normal() {
        assert_eq!(line_tone("stderr", "[environment: untagged] Read more to tag: g.co/cloud/project-env-tag."), "out");
        assert_eq!(line_tone("stderr", "Updated property [core/project]."), "out");
        assert_eq!(line_tone("stderr", "ERROR: (gcloud) You do not currently have an active account selected."), "error");
        assert_eq!(line_tone("stderr", "WARNING: Could not setup log file"), "warn");
        assert_eq!(line_tone("stdout", "Client Version: v1.36.1"), "out");
    }

    #[test]
    fn cloud_sdk_dirs_include_custom_software_prefix() {
        let home = Path::new("/Users/example");
        let dirs = cloud_sdk_dirs(home);
        assert!(dirs.iter().any(|dir| dir == &home.join("software/google-cloud-sdk/bin")));
        assert!(dirs.iter().any(|dir| dir == &home.join("google-cloud-sdk/bin")));
    }

    #[test]
    fn packaged_app_reads_zshrc_with_interactive_login_shell() {
        assert_eq!(shell_lookup_args("/bin/zsh"), vec!["-lic"]);
        assert_eq!(shell_lookup_args("/bin/bash"), vec!["-lic"]);
        assert_eq!(shell_lookup_args("/opt/homebrew/bin/fish"), vec!["-l", "-c"]);
    }

    #[test]
    fn windows_batch_paths_are_quoted_for_cmd() {
        assert_eq!(quote_windows("version"), "version");
        assert_eq!(
            quote_windows(r"C:\Program Files\Google\Cloud SDK\google-cloud-sdk\bin\gcloud.cmd"),
            r#""C:\Program Files\Google\Cloud SDK\google-cloud-sdk\bin\gcloud.cmd""#
        );
        assert!(is_batch_script(Path::new(r"C:\sdk\bin\gcloud.cmd")));
        assert!(is_batch_script(Path::new("/opt/sdk/gcloud.CMD")));
        assert!(!is_batch_script(Path::new("/usr/local/bin/kubectl")));
        assert!(!is_batch_script(Path::new(r"C:\bin\kubectl.exe")));
    }

    #[test]
    fn api_timeout_is_explained_as_network_not_yaml() {
        let text = r#"error validating "whitelist.yaml": error validating data: failed to download openapi: Get "https://34.34.220.130:443/openapi/v2?timeout=32s": dial tcp 34.34.220.130:443: i/o timeout"#;
        let reason = explain_failure(text).unwrap();
        assert!(reason.contains("连不上 Kubernetes API"));
        assert!(reason.contains("不是 YAML"));
        assert!(explain_failure("Updated property [core/project].").is_none());
    }

    fn ip(cidr: &str) -> GcpIp {
        GcpIp { cidr: cidr.to_string(), comment: String::new() }
    }

    fn column(rows: &[GcpAlignedRow], index: usize) -> Vec<String> {
        rows.iter()
            .filter_map(|row| {
                let cidr = &row.cells[index].cidr;
                if cidr.is_empty() { None } else { Some(cidr.clone()) }
            })
            .collect()
    }

    fn git(dir: &Path, args: &[&str]) {
        let status = Command::new("git").arg("-C").arg(dir).args(args).status().expect("git");
        assert!(status.success(), "git {args:?}");
    }
}

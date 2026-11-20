/**
 * GCP 白名单项目检查。
 */

import { invokeSafe } from "../../api/tauri.js";

/** @param {string} path */
export function inspectGcpProject(path) {
  return invokeSafe("gcp_inspect_project", { path: String(path ?? "") });
}

/**
 * @param {string} path
 * @param {string} cidr
 * @param {string} comment
 * @param {string[]} keys
 */
export function addGcpWhitelistIp(path, cidr, comment, keys) {
  return invokeSafe("gcp_add_whitelist_ip", {
    path: String(path ?? ""),
    cidr: String(cidr ?? ""),
    comment: String(comment ?? ""),
    keys,
  });
}

/** @param {string} path @param {{ key: string, ips: { cidr: string, comment: string }[] }[]} files */
export function saveGcpWhitelist(path, files) {
  return invokeSafe("gcp_save_whitelist_files", {
    path: String(path ?? ""),
    files,
  });
}

/** @param {string} path @param {string[]} keys */
export function applyGcpWhitelist(path, keys) {
  return invokeSafe("gcp_apply_whitelist", {
    path: String(path ?? ""),
    keys,
  });
}

/** @param {string} path */
export function fetchGcpWhitelist(path) {
  return invokeSafe("gcp_fetch_whitelist", { path: String(path ?? "") });
}

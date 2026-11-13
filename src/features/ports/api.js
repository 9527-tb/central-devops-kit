/**
 * 端口检测 IPC。
 */

import { invokeSafe } from "../../api/tauri.js";

/**
 * @param {string|number} query
 * @param {boolean} [fuzzy=false]
 */
export async function lookupPort(query, fuzzy = false) {
  return invokeSafe("ports_lookup", {
    query: String(query ?? "").trim(),
    fuzzy: !!fuzzy,
  });
}

/** @param {number} pid */
export async function killPid(pid) {
  return invokeSafe("ports_kill", { pid });
}

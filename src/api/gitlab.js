/**
 * GitLab 变量 IPC。
 */

import { invokeSafe } from "./tauri.js";

export function loadGitlabVariables(token, groupUrl, page = 1) {
  return invokeSafe("gitlab_load_variables", { token, groupUrl, page });
}

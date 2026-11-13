/**
 * GitLab 变量 IPC。
 */

import { invokeSafe } from "./tauri.js";

export function loadGitlabVariables(token, page = 1) {
  return invokeSafe("gitlab_load_variables", { token, page });
}

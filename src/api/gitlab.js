/**
 * GitLab 变量 IPC。
 */

import { invokeSafe } from "./tauri.js";

export function loadGitlabVariables(token, groupUrl, page = 1) {
  return invokeSafe("gitlab_load_variables", { token, groupUrl, page });
}

export function deleteGitlabVariable(token, groupUrl, key, environmentScope) {
  return invokeSafe("gitlab_delete_variable", { token, groupUrl, key, environmentScope });
}

export function createGitlabVariable(payload) {
  return invokeSafe("gitlab_create_variable", payload);
}

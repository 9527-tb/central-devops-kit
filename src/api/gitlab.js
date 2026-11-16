/**
 * GitLab 变量 IPC。
 */

import { invokeSafe } from "./tauri.js";

export function loadGitlabVariables(token, groupUrl, page = 1) {
  return invokeSafe("gitlab_load_variables", { token, groupUrl, page });
}

export function listGitlabProjects(token, groupUrl) {
  return invokeSafe("gitlab_list_projects", { token, groupUrl });
}

export function listGitlabPipelines(token, groupUrl, projectId) {
  return invokeSafe("gitlab_list_pipelines", { token, groupUrl, projectId });
}

export function listGitlabBranches(token, groupUrl, projectId, search = "") {
  return invokeSafe("gitlab_list_branches", { token, groupUrl, projectId, search });
}

export function createGitlabPipeline(token, groupUrl, projectId, gitRef) {
  return invokeSafe("gitlab_create_pipeline", { token, groupUrl, projectId, gitRef });
}

export function listGitlabJobs(token, groupUrl, projectId, pipelineId) {
  return invokeSafe("gitlab_list_jobs", { token, groupUrl, projectId, pipelineId });
}

export function runGitlabJob(token, groupUrl, projectId, jobId, action) {
  return invokeSafe("gitlab_run_job", { token, groupUrl, projectId, jobId, action });
}

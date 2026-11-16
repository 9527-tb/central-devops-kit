/**
 * 读取已保存的 GitLab group 下每个项目的 Pipeline，并按分支新建。
 */

import { onBeforeUnmount, ref, watch } from "vue";
import { message } from "antdv-next";
import { isTauri } from "../../api/tauri.js";
import {
  createGitlabPipeline,
  listGitlabBranches,
  listGitlabJobs,
  listGitlabPipelines,
  listGitlabProjects,
  runGitlabJob,
} from "../../api/gitlab.js";
import { createTranslator } from "../../i18n/index.js";
import { locale, persistSettings, settings, settingsLoading } from "../../stores/settings.js";

let ticket = 0;
let pipelineTicket = 0;
let jobTicket = 0;
let branchTicket = 0;
let branchTimer = 0;

export function useGitlabPipelines() {
  const t = createTranslator(locale);
  const loading = ref(true);
  const pipelinesLoading = ref(false);
  const creating = ref(false);
  const status = ref("loading");
  const messageText = ref("");
  const projects = ref([]);
  const projectId = ref(null);
  const pipelines = ref([]);
  const branches = ref([]);
  const branchesLoading = ref(false);
  const branchTotal = ref(0);
  const jobsOpen = ref(false);
  const jobsLoading = ref(false);
  const jobs = ref([]);
  const activePipeline = ref(null);
  const runningJobId = ref(null);
  const createOpen = ref(false);
  const branchName = ref("");
  const settingsOpen = ref(false);
  const tokenInput = ref("");
  const groupUrlInput = ref("");
  let suppressAutoSettings = false;

  function failText(result) {
    if (!result) return t("gitlabNeedDesktop");
    if (result.code === "not_found") return t("gitlabNotFound");
    if (result.code === "forbidden") return t("gitlabForbidden");
    if (result.code === "bad_response") return t("gitlabBadResponse");
    if (result.code === "bad_address") return t("gitlabGroupInvalid");
    if (result.code === "bad_ref") return t("pipelineBranchRequired");
    if (result.code === "bad_action") return t("pipelineJobRunFail", { error: result.code });
    if (result.code === "network") return t("gitlabNetwork", { error: result.message || "—" });
    return t("gitlabLoadFail", { error: result.message || result.code || "—" });
  }

  function savedGitlab() {
    return {
      token: String(settings.value.gitlab?.accessToken || "").trim(),
      groupUrl: String(settings.value.gitlab?.groupUrl || "").trim(),
    };
  }

  function fillSettingsForm() {
    const saved = savedGitlab();
    tokenInput.value = saved.token;
    groupUrlInput.value = saved.groupUrl;
  }

  function openSettings() {
    fillSettingsForm();
    settingsOpen.value = true;
  }

  function closeSettings() {
    settingsOpen.value = false;
    if (status.value === "missing" || status.value === "invalid") suppressAutoSettings = true;
  }

  function showSettingsIfNeeded() {
    if ((status.value === "missing" || status.value === "invalid") && !suppressAutoSettings && !settingsOpen.value) {
      openSettings();
    }
  }

  function groupAddressOk(value) {
    try {
      const url = new URL(value);
      const path = url.pathname.replace(/^\/+|\/+$/g, "");
      return (url.protocol === "http:" || url.protocol === "https:") && Boolean(url.host && path);
    } catch {
      return false;
    }
  }

  async function loadPipelines(id) {
    const { token, groupUrl } = savedGitlab();
    const current = ++pipelineTicket;
    pipelinesLoading.value = true;
    try {
      const result = await listGitlabPipelines(token, groupUrl, id);
      if (current !== pipelineTicket || projectId.value !== id) return;
      if (!result || result.status !== "ok") {
        pipelines.value = [];
        message.error(failText(result));
        return;
      }
      pipelines.value = Array.isArray(result.pipelines) ? result.pipelines : [];
    } catch (error) {
      if (current !== pipelineTicket) return;
      pipelines.value = [];
      message.error(t("gitlabLoadFail", { error: String(error) }));
    } finally {
      if (current === pipelineTicket) pipelinesLoading.value = false;
    }
  }

  async function refresh() {
    const current = ++ticket;
    loading.value = true;
    status.value = "loading";
    const { token, groupUrl } = savedGitlab();
    if (!token || !groupUrl) {
      if (current !== ticket) return;
      projects.value = [];
      pipelines.value = [];
      projectId.value = null;
      closeJobs();
      status.value = "missing";
      loading.value = false;
      showSettingsIfNeeded();
      return;
    }
    if (!isTauri) {
      if (current !== ticket) return;
      status.value = "failed";
      messageText.value = t("gitlabNeedDesktop");
      loading.value = false;
      return;
    }
    try {
      const result = await listGitlabProjects(token, groupUrl);
      if (current !== ticket) return;
      if (!result || result.status === "missing") {
        projects.value = [];
        pipelines.value = [];
        projectId.value = null;
        closeJobs();
        status.value = "missing";
        showSettingsIfNeeded();
        return;
      }
      if (result.status === "invalid") {
        projects.value = [];
        pipelines.value = [];
        projectId.value = null;
        closeJobs();
        status.value = "invalid";
        showSettingsIfNeeded();
        return;
      }
      if (result.status !== "ok") {
        projects.value = [];
        pipelines.value = [];
        projectId.value = null;
        closeJobs();
        status.value = "failed";
        messageText.value = failText(result);
        return;
      }
      projects.value = Array.isArray(result.projects) ? result.projects : [];
      status.value = "ready";
      settingsOpen.value = false;
      const stillThere = projects.value.some((item) => item.id === projectId.value);
      if (!stillThere) {
        projectId.value = null;
        pipelines.value = [];
        closeJobs();
        return;
      }
      await loadPipelines(projectId.value);
      if (jobsOpen.value && activePipeline.value) await loadJobs(activePipeline.value.id);
    } catch (error) {
      if (current !== ticket) return;
      status.value = "failed";
      messageText.value = t("gitlabLoadFail", { error: String(error) });
    } finally {
      if (current === ticket) loading.value = false;
    }
  }

  async function selectProject(id) {
    if (!id || id === projectId.value) return;
    projectId.value = id;
    pipelines.value = [];
    closeJobs();
    await loadPipelines(id);
  }

  function closeJobs() {
    jobsOpen.value = false;
    jobs.value = [];
    activePipeline.value = null;
    runningJobId.value = null;
    jobTicket += 1;
  }

  async function loadJobs(pipelineId) {
    if (!projectId.value || !pipelineId) return;
    const { token, groupUrl } = savedGitlab();
    const current = ++jobTicket;
    jobsLoading.value = true;
    try {
      const result = await listGitlabJobs(token, groupUrl, projectId.value, pipelineId);
      if (current !== jobTicket || activePipeline.value?.id !== pipelineId) return;
      if (!result || result.status !== "ok") {
        jobs.value = [];
        message.error(failText(result));
        return;
      }
      jobs.value = (Array.isArray(result.jobs) ? result.jobs : []).map((job) => ({
        id: job.id,
        name: job.name || "",
        stage: job.stage || "—",
        status: job.status || "",
        when: job.when || "",
        duration: job.duration,
      }));
    } catch (error) {
      if (current !== jobTicket) return;
      jobs.value = [];
      message.error(t("gitlabLoadFail", { error: String(error) }));
    } finally {
      if (current === jobTicket) jobsLoading.value = false;
    }
  }

  async function openJobs(pipeline) {
    if (!pipeline || !projectId.value) return;
    activePipeline.value = pipeline;
    jobs.value = [];
    jobsOpen.value = true;
    await loadJobs(pipeline.id);
  }

  async function runJob(job, action) {
    if (!job || !projectId.value || !activePipeline.value || runningJobId.value) return;
    const { token, groupUrl } = savedGitlab();
    runningJobId.value = job.id;
    try {
      const result = await runGitlabJob(token, groupUrl, projectId.value, job.id, action);
      if (!result || result.status !== "ok") {
        message.error(t("pipelineJobRunFail", { error: failText(result) }));
        return;
      }
      message.success(t("pipelineJobRan", { name: job.name || job.id }));
      await loadJobs(activePipeline.value.id);
      await loadPipelines(projectId.value);
    } catch (error) {
      message.error(t("pipelineJobRunFail", { error: String(error) }));
    } finally {
      runningJobId.value = null;
    }
  }

  async function queryBranches(search) {
    if (!projectId.value) return;
    const { token, groupUrl } = savedGitlab();
    const current = ++branchTicket;
    branchesLoading.value = true;
    try {
      const result = await listGitlabBranches(token, groupUrl, projectId.value, search);
      if (current !== branchTicket) return;
      if (!result || result.status !== "ok") {
        branches.value = [];
        branchTotal.value = 0;
        message.error(failText(result));
        return;
      }
      branches.value = Array.isArray(result.branches) ? result.branches : [];
      branchTotal.value = Number(result.total) || branches.value.length;
    } catch (error) {
      if (current !== branchTicket) return;
      branches.value = [];
      branchTotal.value = 0;
      message.error(t("gitlabLoadFail", { error: String(error) }));
    } finally {
      if (current === branchTicket) branchesLoading.value = false;
    }
  }

  function openCreate() {
    if (!projectId.value) return;
    createOpen.value = true;
    branchName.value = "";
    branches.value = [];
    branchTotal.value = 0;
    window.clearTimeout(branchTimer);
    queryBranches("");
  }

  function searchBranches(value) {
    const search = String(value || "");
    window.clearTimeout(branchTimer);
    branchTimer = window.setTimeout(() => queryBranches(search.trim()), 250);
  }

  function closeCreate() {
    if (creating.value) return;
    createOpen.value = false;
  }

  async function runPipeline() {
    const gitRef = String(branchName.value || "").trim();
    if (!gitRef || !projectId.value) {
      message.warning(t("pipelineBranchRequired"));
      return;
    }
    const { token, groupUrl } = savedGitlab();
    creating.value = true;
    try {
      const result = await createGitlabPipeline(token, groupUrl, projectId.value, gitRef);
      if (!result || result.status !== "ok") {
        message.error(t("pipelineCreateFail", { error: failText(result) }));
        return;
      }
      const created = result.pipelines?.[0];
      message.success(t("pipelineCreated", { id: created?.id || "" }));
      createOpen.value = false;
      await loadPipelines(projectId.value);
    } catch (error) {
      message.error(t("pipelineCreateFail", { error: String(error) }));
    } finally {
      creating.value = false;
    }
  }

  async function saveGitlabSettings() {
    const token = tokenInput.value.trim();
    const groupUrl = groupUrlInput.value.trim();
    if (!token || !groupUrl) {
      message.warning(t("gitlabSettingsRequired"));
      return;
    }
    if (!groupAddressOk(groupUrl)) {
      message.warning(t("gitlabGroupInvalid"));
      return;
    }
    settingsOpen.value = false;
    suppressAutoSettings = false;
    try {
      await persistSettings({
        ...settings.value,
        gitlab: { accessToken: token, groupUrl },
      });
    } catch (error) {
      settingsOpen.value = true;
      message.error(t("settingsSaveFail", { error: String(error) }));
      return;
    }
    await refresh();
  }

  watch(settingsLoading, (loadingNow) => {
    if (!loadingNow) refresh();
  }, { immediate: true });

  onBeforeUnmount(() => {
    ticket += 1;
    pipelineTicket += 1;
    jobTicket += 1;
    branchTicket += 1;
    window.clearTimeout(branchTimer);
  });

  return {
    loading,
    pipelinesLoading,
    creating,
    status,
    messageText,
    projects,
    projectId,
    pipelines,
    branches,
    branchesLoading,
    branchTotal,
    jobsOpen,
    jobsLoading,
    jobs,
    activePipeline,
    runningJobId,
    createOpen,
    branchName,
    settingsOpen,
    tokenInput,
    groupUrlInput,
    refresh,
    selectProject,
    openJobs,
    closeJobs,
    runJob,
    openCreate,
    searchBranches,
    closeCreate,
    runPipeline,
    openSettings,
    closeSettings,
    saveGitlabSettings,
  };
}

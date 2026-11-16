<!--
  GitLab Pipeline：先选项目，再看该项目的流水线和任务，并在这里手动执行。
-->
<script setup>
import { computed, ref } from "vue";
import {
  CaretRightOutlined,
  CheckCircleFilled,
  ClockCircleOutlined,
  CloseCircleFilled,
  ExclamationCircleFilled,
  LeftOutlined,
  LoadingOutlined,
  MinusCircleFilled,
  PauseCircleFilled,
  RedoOutlined,
  ReloadOutlined,
  SettingOutlined,
} from "@antdv-next/icons";
import { createTranslator } from "../i18n/index.js";
import { locale } from "../stores/settings.js";
import { useGitlabPipelines } from "../features/gitlab/useGitlabPipelines.js";
import "../styles/settings-layout.css";

const t = createTranslator(locale);
const projectQuery = ref("");
const {
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
} = useGitlabPipelines();

const needsConfig = computed(() => status.value === "missing" || status.value === "invalid");
const setupMessage = computed(() =>
  status.value === "invalid" ? t("gitlabTokenInvalid") : t("gitlabTokenMissing"),
);
const selectedProject = computed(() =>
  projects.value.find((item) => item.id === projectId.value) || null,
);
const visibleProjects = computed(() => {
  const query = projectQuery.value.trim().toLowerCase();
  const rows = projects.value.filter((item) => {
    if (!query) return true;
    const name = String(item.name || "").toLowerCase();
    const path = String(item.path || "").toLowerCase();
    return name.includes(query) || path.includes(query);
  });
  return [...rows].sort((left, right) => {
    const byName = String(left.name || left.path || "").localeCompare(
      String(right.name || right.path || ""),
      undefined,
      { sensitivity: "base" },
    );
    if (byName !== 0) return byName;
    return String(left.path || "").localeCompare(String(right.path || ""), undefined, { sensitivity: "base" });
  });
});

const branchOptions = computed(() => {
  const rows = branches.value.map((branch) => ({
    value: branch.name,
    label: branch.default ? `${branch.name} · default` : branch.name,
  }));
  const selected = String(branchName.value || "");
  if (selected && !rows.some((item) => item.value === selected)) {
    rows.unshift({ value: selected, label: selected });
  }
  return rows;
});

const columns = computed(() => [
  { title: t("pipelineColId"), dataIndex: "id", key: "id", width: 110 },
  { title: t("pipelineColStatus"), dataIndex: "status", key: "status", width: 130 },
  { title: t("pipelineColRef"), dataIndex: "ref", key: "ref", ellipsis: true },
  { title: t("pipelineColSha"), key: "sha", width: 110 },
  { title: t("pipelineColSource"), dataIndex: "source", key: "source", width: 110 },
  { title: t("pipelineColCreated"), key: "created", width: 180 },
  { title: t("pipelineColAction"), key: "action", width: 90 },
]);

const jobStages = computed(() => {
  const groups = new Map();
  for (const job of jobs.value) {
    const stage = job.stage || "—";
    if (!groups.has(stage)) groups.set(stage, []);
    groups.get(stage).push(job);
  }
  return [...groups.entries()]
    .map(([name, items]) => ({
      name,
      order: Math.min(...items.map((job) => Number(job.id) || 0)),
      jobs: [...items].sort((left, right) =>
        String(left.name || "").localeCompare(String(right.name || ""), undefined, { sensitivity: "base" }),
      ),
    }))
    .sort((left, right) => left.order - right.order);
});

function statusColor(value) {
  const statusName = String(value || "").toLowerCase();
  if (statusName === "success") return "success";
  if (statusName === "failed") return "error";
  if (statusName === "running" || statusName === "pending" || statusName === "preparing" || statusName === "created") {
    return "processing";
  }
  if (statusName === "canceled" || statusName === "cancelled" || statusName === "skipped") return "default";
  if (statusName === "manual") return "warning";
  return "default";
}

function jobAction(job) {
  const statusName = String(job?.status || "").toLowerCase();
  if (statusName === "manual") return "play";
  if (statusName === "failed" || statusName === "canceled" || statusName === "cancelled" || statusName === "success") {
    return "retry";
  }
  return "";
}

function shortSha(value) {
  const text = String(value || "");
  return text ? text.slice(0, 8) : "—";
}

function createdText(value) {
  if (!value) return "—";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString();
}

function jobStatus(job) {
  return String(job?.status || "").toLowerCase();
}

function pipelineRow(record) {
  return {
    onClick: () => openJobs(record),
  };
}

function cellText(column, record, text) {
  if (text !== undefined && text !== null && text !== "") return text;
  const field = column?.dataIndex;
  if (typeof field === "string") return record?.[field] ?? "";
  return "";
}
</script>

<template>
  <div class="gitlab-page">
    <main class="settings-main pipeline-main">
      <section class="settings-panel pipeline-panel">
        <div class="gitlab-head">
          <h2>{{ t("gitlabPipeline") }}</h2>
          <div class="gitlab-head-actions">
            <a-button :loading="loading" @click="refresh">
              <template #icon><ReloadOutlined /></template>
              {{ t("pipelineRefresh") }}
            </a-button>
            <a-button @click="openSettings">
              <template #icon><SettingOutlined /></template>
            </a-button>
          </div>
        </div>

        <div class="gitlab-body">
          <div v-if="loading" class="gitlab-mask">
            <a-spin spinning />
          </div>
          <p v-if="!loading && needsConfig" class="gitlab-need-setup">{{ setupMessage }}</p>
          <div v-else-if="!loading && status === 'failed'" class="cfg-group">
            <div class="cfg-group-b">
              <a-alert type="error" show-icon :message="messageText" />
            </div>
          </div>
          <div v-else-if="status === 'ready'" class="pipeline-split">
            <aside class="pipeline-projects">
              <div class="pipeline-side-head">{{ t("pipelineProjects") }}</div>
              <a-input
                v-model:value="projectQuery"
                allow-clear
                :placeholder="t('pipelineProjectSearch')"
              />
              <p v-if="!projects.length" class="pipeline-side-empty">{{ t("pipelineEmptyProjects") }}</p>
              <p v-else-if="!visibleProjects.length" class="pipeline-side-empty">{{ t("pipelineProjectNone") }}</p>
              <button
                v-for="project in visibleProjects"
                :key="project.id"
                type="button"
                class="pipeline-project"
                :class="{ active: project.id === projectId }"
                @click="selectProject(project.id)"
              >
                <strong>{{ project.name || project.path }}</strong>
                <span v-if="project.path && project.path !== project.name">{{ project.path }}</span>
              </button>
            </aside>

            <section class="pipeline-detail" :class="{ 'is-board': jobsOpen && selectedProject }">
              <div v-if="!selectedProject" class="pipeline-pick">{{ t("pipelinePickProject") }}</div>
              <template v-else>
                <div class="pipeline-detail-head">
                  <div>
                    <div class="pipeline-detail-kicker">{{ t("pipelineBoundProject") }}</div>
                    <h3>{{ selectedProject.path || selectedProject.name }}</h3>
                  </div>
                  <a-button v-if="!jobsOpen" type="primary" @click="openCreate">{{ t("pipelineNew") }}</a-button>
                  <a-button v-else @click="closeJobs">
                    <template #icon><LeftOutlined /></template>
                    {{ t("pipelineBack") }}
                  </a-button>
                </div>
                <div v-if="jobsOpen" class="stage-board">
                  <div class="stage-board-head">
                    <strong v-if="activePipeline">#{{ activePipeline.id }}</strong>
                    <a-tag v-if="activePipeline" :color="statusColor(activePipeline.status)">
                      {{ activePipeline.status || "—" }}
                    </a-tag>
                    <span class="stage-ref">{{ activePipeline?.ref || "" }}</span>
                    <span class="stage-group">{{ t("pipelineGroupStage") }}</span>
                  </div>
                  <div v-if="jobsLoading && !jobs.length" class="stage-loading">
                    <a-spin spinning />
                  </div>
                  <p v-else-if="!jobs.length" class="pipeline-side-empty">{{ t("pipelineJobEmpty") }}</p>
                  <div v-else class="stage-columns">
                    <section v-for="stage in jobStages" :key="stage.name" class="stage-col">
                      <h4>{{ stage.name }}</h4>
                      <div v-for="job in stage.jobs" :key="job.id" class="job-pill">
                        <CheckCircleFilled v-if="jobStatus(job) === 'success'" class="job-ico job-ico-success" />
                        <CloseCircleFilled v-else-if="jobStatus(job) === 'failed'" class="job-ico job-ico-failed" />
                        <LoadingOutlined v-else-if="jobStatus(job) === 'running'" class="job-ico job-ico-running" />
                        <PauseCircleFilled v-else-if="jobStatus(job) === 'manual'" class="job-ico job-ico-manual" />
                        <MinusCircleFilled
                          v-else-if="jobStatus(job) === 'canceled' || jobStatus(job) === 'cancelled' || jobStatus(job) === 'skipped'"
                          class="job-ico job-ico-muted"
                        />
                        <ExclamationCircleFilled v-else-if="jobStatus(job) === 'warning'" class="job-ico job-ico-manual" />
                        <ClockCircleOutlined v-else class="job-ico job-ico-muted" />
                        <span class="job-name" :title="job.name">{{ job.name || "—" }}</span>
                        <button
                          v-if="jobAction(job)"
                          type="button"
                          class="job-act"
                          :title="jobAction(job) === 'play' ? t('pipelineJobRun') : t('pipelineJobRetry')"
                          :disabled="!!runningJobId"
                          @click="runJob(job, jobAction(job))"
                        >
                          <LoadingOutlined v-if="runningJobId === job.id" class="job-ico-running" />
                          <CaretRightOutlined v-else-if="jobAction(job) === 'play'" />
                          <RedoOutlined v-else />
                        </button>
                      </div>
                    </section>
                  </div>
                </div>
                <a-table
                  v-else
                  size="small"
                  :loading="pipelinesLoading"
                  :columns="columns"
                  :data-source="pipelines"
                  :pagination="false"
                  row-key="id"
                  :on-row="pipelineRow"
                  :locale="{ emptyText: t('pipelineEmpty') }"
                >
                  <template #bodyCell="{ column, record, text }">
                    <template v-if="column.key === 'id'">#{{ record.id }}</template>
                    <template v-else-if="column.key === 'status'">
                      <a-tag :color="statusColor(record.status)">{{ record.status || "—" }}</a-tag>
                    </template>
                    <template v-else-if="column.key === 'sha'">
                      <code>{{ shortSha(record.sha) }}</code>
                    </template>
                    <template v-else-if="column.key === 'created'">
                      {{ createdText(record.createdAt) }}
                    </template>
                    <template v-else-if="column.key === 'action'">
                      <a-button size="small" @click.stop="openJobs(record)">{{ t("pipelineView") }}</a-button>
                    </template>
                    <template v-else>{{ cellText(column, record, text) }}</template>
                  </template>
                </a-table>
              </template>
            </section>
          </div>
        </div>
      </section>
    </main>

    <a-modal
      :open="createOpen"
      :title="t('pipelineNew')"
      :mask-closable="false"
      :confirm-loading="creating"
      :ok-text="t('pipelineRun')"
      :cancel-text="t('cancel')"
      @ok="runPipeline"
      @cancel="closeCreate"
    >
      <div class="pipeline-create">
        <label>
          <span>{{ t("pipelineBoundProject") }}</span>
          <strong>{{ selectedProject?.path || selectedProject?.name }}</strong>
        </label>
        <label>
          <span>{{ t("pipelineBranch") }}</span>
          <a-select
            v-model:value="branchName"
            show-search
            :filter-option="false"
            :loading="branchesLoading"
            :options="branchOptions"
            :placeholder="t('pipelineBranchSearch')"
            @search="searchBranches"
          />
          <p v-if="branchTotal > branches.length" class="pipeline-branch-more">
            {{ t("pipelineBranchMore", { shown: branches.length, total: branchTotal }) }}
          </p>
        </label>
      </div>
    </a-modal>

    <a-modal
      :open="settingsOpen"
      :title="t('gitlabSettings')"
      :mask-closable="false"
      :footer="null"
      @cancel="closeSettings"
    >
      <div class="gitlab-settings-form">
        <label>
          <span>{{ t("gitlabGroupLabel") }}</span>
          <a-input
            v-model:value="groupUrlInput"
            spellcheck="false"
            :placeholder="t('gitlabGroupPlaceholder')"
            @press-enter="saveGitlabSettings"
          />
        </label>
        <label>
          <span>{{ t("gitlabTokenLabel") }}</span>
          <a-input-password
            v-model:value="tokenInput"
            autocomplete="new-password"
            spellcheck="false"
            :placeholder="t('gitlabTokenPlaceholder')"
            @press-enter="saveGitlabSettings"
          />
        </label>
      </div>
      <div class="gitlab-settings-actions">
        <a-button @click="closeSettings">{{ t("gitlabSettingsCancel") }}</a-button>
        <a-button type="primary" @click="saveGitlabSettings">{{ t("gitlabTokenSave") }}</a-button>
      </div>
    </a-modal>
  </div>
</template>

<style scoped>
.gitlab-page {
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--surface-muted);
}
.pipeline-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.pipeline-panel {
  flex: 1 1 auto;
  display: flex;
  flex-direction: column;
  min-height: 0;
}
.gitlab-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 12px;
}
.gitlab-head h2 {
  margin: 0;
}
.gitlab-head-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}
.gitlab-body {
  position: relative;
  flex: 1 1 auto;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
.gitlab-mask {
  position: absolute;
  inset: 0;
  z-index: 5;
  display: grid;
  place-items: center;
  background: color-mix(in srgb, var(--bg) 62%, transparent);
}
.gitlab-need-setup {
  margin: 0;
  color: var(--muted);
}
.pipeline-split {
  flex: 1;
  min-height: 0;
  display: grid;
  grid-template-columns: 280px minmax(0, 1fr);
  gap: 12px;
}
.pipeline-projects,
.pipeline-detail {
  min-height: 0;
  overflow: auto;
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--panel);
}
.pipeline-projects {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 10px;
}
.pipeline-side-head {
  padding: 2px 8px 6px;
  font-size: 12px;
  font-weight: 650;
  color: var(--muted);
}
.pipeline-side-empty,
.pipeline-pick {
  margin: 8px;
  color: var(--muted);
}
.pipeline-project {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 2px;
  width: 100%;
  padding: 8px 10px;
  border: 1px solid transparent;
  border-radius: var(--radius);
  background: transparent;
  color: inherit;
  text-align: left;
  cursor: pointer;
}
.pipeline-project strong {
  font-size: 13px;
  font-weight: 650;
}
.pipeline-project span {
  font-size: 12px;
  color: var(--muted);
  word-break: break-all;
}
.pipeline-project:hover {
  background: var(--surface-hover);
}
.pipeline-project.active {
  border-color: var(--line-hover);
  background: var(--surface-selected);
}
.pipeline-project.active strong {
  color: var(--teal);
}
.pipeline-detail {
  padding: 14px;
}
.pipeline-pick {
  min-height: 180px;
  display: grid;
  place-items: center;
}
.pipeline-detail-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 14px;
}
.pipeline-detail-head h3 {
  margin: 2px 0 0;
  font-size: 15px;
  font-weight: 700;
  line-height: 1.35;
  word-break: break-all;
}
.pipeline-detail-kicker {
  font-size: 12px;
  color: var(--muted);
}
.pipeline-detail.is-board {
  display: flex;
  flex-direction: column;
  overflow: auto;
}
.pipeline-detail :deep(.ant-table-row) {
  cursor: pointer;
}
.stage-board {
  flex: 1 1 auto;
  min-height: 240px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.stage-board-head {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  padding: 8px 10px;
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--surface-muted);
}
.stage-ref,
.stage-group {
  color: var(--muted);
  font-size: 12px;
}
.stage-ref {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.stage-group {
  margin-left: auto;
  padding: 2px 8px;
  border-radius: var(--radius);
  background: var(--panel);
  border: 1px solid var(--line);
}
.stage-loading {
  flex: 1;
  display: grid;
  place-items: center;
}
.stage-columns {
  flex: 1 1 auto;
  min-height: 180px;
  display: flex;
  align-items: flex-start;
  gap: 12px;
  overflow: auto;
  padding: 2px 2px 8px;
}
.stage-col {
  flex: 0 0 272px;
  padding: 10px;
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--surface-muted);
}
.stage-col h4 {
  margin: 0 4px 10px;
  font-size: 13px;
  font-weight: 700;
}
.job-pill {
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 36px;
  margin-bottom: 6px;
  padding: 4px 4px 4px 10px;
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--panel);
}
.job-pill:last-child {
  margin-bottom: 0;
}
.job-name {
  flex: 1;
  min-width: 0;
  font-size: 12px;
  line-height: 1.35;
  word-break: break-word;
}
.job-ico {
  flex: 0 0 auto;
  font-size: 15px;
}
.job-ico-success {
  color: #108548;
}
.job-ico-failed {
  color: #dd2b0e;
}
.job-ico-manual {
  color: #ab6100;
}
.job-ico-muted {
  color: var(--muted);
}
.job-ico-running {
  color: var(--teal);
  animation: job-spin 1s linear infinite;
}
.job-act {
  flex: 0 0 auto;
  display: grid;
  place-items: center;
  width: 28px;
  height: 28px;
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--panel);
  color: var(--ink-soft);
  cursor: pointer;
}
.job-act:hover:not(:disabled) {
  border-color: var(--line-hover);
  background: var(--surface-hover);
  color: var(--teal);
}
.job-act:disabled {
  cursor: default;
  opacity: 0.6;
}
@keyframes job-spin {
  to {
    transform: rotate(360deg);
  }
}
.pipeline-create {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.pipeline-create label {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.pipeline-branch-more {
  margin: 0;
  color: var(--muted);
  font-size: 12px;
}
.gitlab-settings-form {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.gitlab-settings-form label {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.gitlab-settings-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 16px;
}
</style>

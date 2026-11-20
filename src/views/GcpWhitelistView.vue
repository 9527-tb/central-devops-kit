<!--
  GCP WhiteList：选择 central-devops，核对 master 和四个 YAML，并按 diff 对齐 IP。
-->
<script setup>
import { computed, nextTick, onUnmounted, reactive, ref, watch } from "vue";
import { CopyOutlined, DeleteOutlined } from "@antdv-next/icons";
import { message } from "antdv-next";
import { createTranslator } from "../i18n/index.js";
import { isTauri } from "../api/tauri.js";
import { alignIpLists } from "../features/gcp/align.js";
import { inspectGcpProject, saveGcpWhitelist, applyGcpWhitelist, fetchGcpWhitelist } from "../features/gcp/api.js";
import { locale, persistSettings, settings, settingsLoading } from "../stores/settings.js";
import "../styles/settings-layout.css";

const FILE_DEFS = [
  {
    key: "connect-prod",
    title: "mclinica-connect-prod",
    relativePath: "deploy/mclinica-connect-prod/psplink-ip-whitelist-production.yaml",
  },
  {
    key: "connect-staging",
    title: "mclinica-connect-staging",
    relativePath: "deploy/mclinica-connect-staging/psplink-ip-whitelist-staging.yaml",
  },
  {
    key: "indo-prod",
    title: "psplink-indo-prod",
    relativePath: "deploy/psplink-indo-prod/psplink-ip-whitelist-production.yaml",
  },
  {
    key: "indo-staging",
    title: "psplink-indo-staging",
    relativePath: "deploy/psplink-indo-staging/psplink-ip-whitelist-staging.yaml",
  },
];

const t = createTranslator(locale);
const pathInput = ref("");
const checking = ref(false);
const saving = ref(false);
const applyGcpOpen = ref(false);
const applyLogs = ref([]);
const applyLogEl = ref(null);
const applying = ref(false);
const applyPhase = ref("");
const applyWaited = ref(0);
const applyOutcome = ref("");
const fetchOpen = ref(false);
const fetching = ref(false);
const fetchCols = reactive(FILE_DEFS.map((file) => ({
  key: file.key,
  title: file.title,
  relativePath: file.relativePath,
  status: "idle",
  message: "",
  yaml: "",
})));
let applyTimer = 0;
let commandStarted = 0;

function scrollApplyLog() {
  nextTick(() => {
    const el = applyLogEl.value;
    if (!el) return;
    el.scrollTop = el.scrollHeight;
  });
}

watch(applyLogs, scrollApplyLog, { deep: true });
watch(applyGcpOpen, (open) => {
  if (open) scrollApplyLog();
});
const applyEnvChecked = reactive(Object.fromEntries(FILE_DEFS.map((file) => [file.key, true])));
const report = ref(null);
const drafts = reactive(Object.fromEntries(FILE_DEFS.map((file) => [file.key, []])));
const createOpen = ref(false);
const createIp = ref("");
const createComment = ref("");
const commentOpen = ref(false);
const commentIp = ref("");
const commentText = ref("");
const commentEnvLabel = ref("");
const commentTarget = ref(null);
const envChecked = reactive(Object.fromEntries(FILE_DEFS.map((file) => [file.key, true])));
let nextId = 1;

const columns = computed(() => {
  const remote = new Map((report.value?.files || []).map((file) => [file.key, file]));
  return FILE_DEFS.map((file) => {
    const found = remote.get(file.key);
    return {
      ...file,
      exists: found ? !!found.exists : null,
      error: found?.error || "",
      count: (drafts[file.key] || []).filter((item) => item.status !== "deleted").length,
    };
  });
});

const rows = computed(() => {
  const aligned = alignIpLists(FILE_DEFS.map((file) => drafts[file.key] || []));
  return aligned.map((row) => {
    const source = row.cells.find((cell) => cell.cidr && cell.status !== "deleted")
      || row.cells.find((cell) => cell.cidr);
    return {
      cells: row.cells.map((cell, index) => {
        if (cell.cidr) return cell;
        return {
          ...cell,
          fileKey: FILE_DEFS[index].key,
          rowCidr: source?.cidr || "",
          rowComment: source?.comment || "",
        };
      }),
    };
  });
});
const hasDirty = computed(() =>
  FILE_DEFS.some((file) => (drafts[file.key] || []).some((item) => item.status !== "saved")),
);

const summary = computed(() => {
  const list = rows.value;
  if (!list.length) return null;
  let same = 0;
  let gap = 0;
  for (const row of list) {
    const filled = (row.cells || []).filter((cell) => cell?.cidr).length;
    if (filled === columns.value.length) same += 1;
    else gap += 1;
  }
  return { same, gap };
});

const notes = computed(() => {
  const current = report.value;
  if (!current) return [];
  const items = [];
  if (!current.directory) {
    items.push({ tone: "is-bad", text: t("gcpNotDirectory") });
    return items;
  }
  if (!current.repository) {
    const gitError = String(current.gitError || "");
    const notRepo = !gitError || gitError.includes("not a git repository");
    items.push({ tone: "is-bad", text: notRepo ? t("gcpNotRepo") : gitError });
  } else if (current.onMaster) {
    items.push({ tone: "is-ok", text: t("gcpBranchMaster") });
  } else if (current.branch) {
    items.push({ tone: "is-bad", text: t("gcpBranchOther", { branch: current.branch }) });
  } else {
    items.push({ tone: "is-bad", text: t("gcpBranchUnknown") });
  }
  const missing = columns.value.filter((file) => file.exists === false).length;
  items.push({
    tone: missing ? "is-bad" : "is-ok",
    text: missing ? t("gcpFilesMissing", { count: missing }) : t("gcpFilesReady"),
  });
  if (summary.value) {
    items.push({
      tone: summary.value.gap ? "is-warn" : "is-ok",
      text: t("gcpAlignSummary", summary.value),
    });
  }
  return items;
});

async function inspect(options = {}) {
  const path = pathInput.value.trim();
  if (!path) {
    if (!options.quiet) message.warning(t("gcpProjectRequired"));
    return;
  }
  checking.value = true;
  try {
    await persistSettings({
      ...settings.value,
      gcp: { ...settings.value.gcp, projectPath: path },
    });
    const dirty = hasDirty.value;
    const result = await inspectGcpProject(path);
    if (!result) {
      if (!options.quiet) message.warning(t("gcpDesktopOnly"));
      return;
    }
    report.value = result;
    loadDrafts(result.files || []);
    if (dirty && !options.quiet) message.warning(t("gcpUnsavedDiscarded"));
  } catch (error) {
    if (!options.quiet) message.error(t("gcpInspectFail", { error: String(error) }));
  } finally {
    checking.value = false;
  }
}

async function commitPath() {
  const path = pathInput.value.trim();
  if (!path || checking.value) return;
  if (path === (report.value?.path || "").trim()) return;
  await inspect();
}

async function browse() {
  if (!isTauri) {
    message.warning(t("gcpDesktopOnly"));
    return;
  }
  const { open } = await import("@tauri-apps/plugin-dialog");
  const selected = await open({
    directory: true,
    multiple: false,
    title: t("gcpProjectLabel"),
  });
  if (typeof selected !== "string" || !selected) return;
  pathInput.value = selected;
  await inspect();
}

watch(settingsLoading, (loading) => {
  if (loading) return;
  const saved = settings.value.gcp?.projectPath || "";
  if (!saved || pathInput.value) return;
  pathInput.value = saved;
  inspect({ quiet: true });
}, { immediate: true });

function loadDrafts(files) {
  for (const file of FILE_DEFS) {
    const found = (files || []).find((item) => item.key === file.key);
    drafts[file.key] = (found?.ips || []).map((ip) => ({
      id: nextId++,
      fileKey: file.key,
      cidr: ip.cidr,
      comment: ip.comment || "",
      originalCidr: ip.cidr,
      originalComment: ip.comment || "",
      status: "saved",
    }));
  }
}

function findEntry(fileKey, id) {
  return (drafts[fileKey] || []).find((item) => item.id === id);
}

function refreshStatus(entry) {
  if (!entry.originalCidr) {
    entry.status = "added";
    return;
  }
  const same = entry.cidr === entry.originalCidr && entry.comment === entry.originalComment;
  entry.status = same ? "saved" : "edited";
}

function fileStatus(file) {
  if (file.exists === null) return "";
  if (!file.exists) return t("gcpFileMissing");
  return t("gcpFileFound", { count: file.count });
}

function envMissing(key) {
  return columns.value.find((file) => file.key === key)?.exists === false;
}

function envNames(keys) {
  return keys
    .map((key) => FILE_DEFS.find((file) => file.key === key)?.title || key)
    .join(", ");
}

function validCidr(value) {
  const text = String(value || "").trim();
  if (!text || /\s/.test(text)) return false;
  const slash = text.indexOf("/");
  if (slash !== text.lastIndexOf("/")) return false;
  const addr = slash === -1 ? text : text.slice(0, slash);
  const prefix = slash === -1 ? null : text.slice(slash + 1);
  if (prefix !== null && !/^\d+$/.test(prefix)) return false;
  const bits = prefix === null ? null : Number(prefix);
  if (/^\d{1,3}(\.\d{1,3}){3}$/.test(addr)) {
    const octets = addr.split(".").every((part) => Number(part) <= 255);
    return octets && (bits === null || bits <= 32);
  }
  if (addr.includes(":") && /^[0-9a-fA-F:]+$/.test(addr)) {
    return bits === null || bits <= 128;
  }
  return false;
}

function openCreate() {
  createIp.value = "";
  createComment.value = "";
  for (const file of FILE_DEFS) {
    envChecked[file.key] = !envMissing(file.key);
  }
  createOpen.value = true;
}

function closeCreate() {
  createOpen.value = false;
}

function openComment(cell) {
  if (!cell?.cidr || cell.status === "deleted" || !canChangeCell(cell)) return;
  const entry = findEntry(cell.fileKey, cell.id);
  if (!entry) return;
  const file = FILE_DEFS.find((item) => item.key === cell.fileKey);
  commentTarget.value = { fileKey: cell.fileKey, id: cell.id };
  commentIp.value = entry.cidr;
  commentText.value = entry.comment || "";
  commentEnvLabel.value = file ? `${file.key} · ${file.title}` : cell.fileKey;
  commentOpen.value = true;
}

function closeComment() {
  commentOpen.value = false;
}

function saveComment() {
  const target = commentTarget.value;
  const entry = target ? findEntry(target.fileKey, target.id) : null;
  if (!entry || entry.status === "deleted") {
    commentOpen.value = false;
    return;
  }
  const comment = commentText.value.trim();
  if (comment.includes("\n")) {
    message.warning(t("gcpCreateInvalid"));
    return;
  }
  entry.comment = comment;
  refreshStatus(entry);
  commentOpen.value = false;
}

function saveCreate() {
  const cidr = createIp.value.trim();
  const comment = createComment.value.trim();
  const keys = FILE_DEFS.filter((file) => envChecked[file.key]).map((file) => file.key);
  if (!pathInput.value.trim() || !report.value?.directory) {
    message.warning(t("gcpProjectRequired"));
    return;
  }
  if (!cidr) {
    message.warning(t("gcpCreateRequired"));
    return;
  }
  if (comment.includes("\n") || !validCidr(cidr)) {
    message.warning(t("gcpCreateInvalid"));
    return;
  }
  if (!keys.length) {
    message.warning(t("gcpCreateEnvRequired"));
    return;
  }
  const added = [];
  const duplicates = [];
  const missing = [];
  for (const key of keys) {
    if (envMissing(key)) {
      missing.push(key);
      continue;
    }
    const existing = (drafts[key] || []).find((item) => item.cidr === cidr && item.status !== "deleted");
    if (existing) {
      duplicates.push(key);
      continue;
    }
    const removed = (drafts[key] || []).find((item) => item.cidr === cidr && item.status === "deleted");
    if (removed) {
      removed.comment = comment;
      refreshStatus(removed);
      added.push(key);
      continue;
    }
    drafts[key].push({
      id: nextId++,
      fileKey: key,
      cidr,
      comment,
      originalCidr: "",
      originalComment: "",
      status: "added",
    });
    added.push(key);
  }
  if (added.length) message.success(t("gcpCreateOk"));
  if (duplicates.length) message.warning(t("gcpCreateDuplicate", { envs: envNames(duplicates) }));
  if (missing.length) message.warning(t("gcpCreateMissing", { envs: envNames(missing) }));
  if (added.length) createOpen.value = false;
}

function canChangeCell(cell) {
  return Boolean(cell.fileKey) && !envMissing(cell.fileKey);
}

function copyCell(cell) {
  if (!canChangeCell(cell) || cell.cidr || !cell.rowCidr) return;
  const list = drafts[cell.fileKey] || [];
  list.push({
    id: nextId++,
    fileKey: cell.fileKey,
    cidr: cell.rowCidr,
    comment: cell.rowComment || "",
    originalCidr: "",
    originalComment: "",
    status: "added",
    slot: cell.slot ?? 0,
  });
}

function openApplyGcp() {
  for (const file of FILE_DEFS) {
    applyEnvChecked[file.key] = !envMissing(file.key);
  }
  applyGcpOpen.value = true;
}

function closeApplyGcp() {
  applyGcpOpen.value = false;
}

function resetFetchCols() {
  for (const col of fetchCols) {
    col.status = "idle";
    col.message = "";
    col.yaml = "";
  }
}

function openFetchGcp() {
  fetchOpen.value = true;
  if (fetching.value) return;
  resetFetchCols();
  if (!pathInput.value.trim() || !report.value?.directory) {
    for (const col of fetchCols) {
      col.status = "error";
      col.message = t("gcpProjectRequired");
      col.yaml = t("gcpProjectRequired");
    }
    return;
  }
  if (!isTauri) {
    for (const col of fetchCols) {
      col.status = "error";
      col.message = t("gcpDesktopOnly");
      col.yaml = t("gcpDesktopOnly");
    }
    return;
  }
  runFetchGcp();
}

function closeFetchGcp() {
  fetchOpen.value = false;
}

function fetchStatusText(status) {
  if (status === "running") return t("gcpFetchRunning");
  if (status === "ok") return t("gcpFetchOk");
  if (status === "error") return t("gcpFetchError");
  return t("gcpFetchIdle");
}

async function runFetchGcp() {
  if (fetching.value) return;
  fetching.value = true;
  for (const col of fetchCols) {
    col.status = "running";
    col.message = t("gcpFetchPreparing");
    col.yaml = "";
  }
  let finished = false;
  const finish = (unlisten) => {
    if (finished) return;
    finished = true;
    fetching.value = false;
    unlisten?.();
  };
  let unlisten = () => {};
  try {
    const { listen } = await import("@tauri-apps/api/event");
    unlisten = await listen("gcp-fetch-update", (event) => {
      const line = event.payload || {};
      const col = fetchCols.find((item) => item.key === line.key);
      if (col) {
        if (line.status) col.status = line.status;
        if (line.message) col.message = line.message;
        if (line.replaceYaml) col.yaml = line.yaml || "";
      }
      if (line.done) queueMicrotask(() => finish(unlisten));
    });
    const result = await fetchGcpWhitelist(pathInput.value.trim());
    if (!result) {
      for (const col of fetchCols) {
        col.status = "error";
        col.message = t("gcpDesktopOnly");
        col.yaml = t("gcpDesktopOnly");
      }
      finish(unlisten);
    }
  } catch (error) {
    const text = String(error?.message || error);
    for (const col of fetchCols) {
      col.status = "error";
      col.message = text;
      col.yaml = text;
    }
    finish(unlisten);
  }
}

function pushApplyLog(text, tone = "out") {
  applyLogs.value.push({ text, tone });
  if (tone === "cmd") {
    applyPhase.value = String(text).replace(/^\$\s*/, "");
    commandStarted = Date.now();
    applyWaited.value = 0;
  } else if (tone === "rule" && text && !/^━+$/.test(text)) {
    applyPhase.value = text;
    commandStarted = Date.now();
    applyWaited.value = 0;
  }
}

function beginApplyStatus() {
  applyOutcome.value = "";
  applyPhase.value = t("gcpApplyPreparing");
  applyWaited.value = 0;
  commandStarted = Date.now();
  window.clearInterval(applyTimer);
  applyTimer = window.setInterval(() => {
    applyWaited.value = Math.max(0, Math.floor((Date.now() - commandStarted) / 1000));
  }, 500);
}

function endApplyStatus(tone) {
  window.clearInterval(applyTimer);
  applyTimer = 0;
  applyOutcome.value = tone === "ok" ? "ok" : "error";
}

onUnmounted(() => window.clearInterval(applyTimer));

async function runApplyGcp() {
  if (applying.value) return;
  const keys = FILE_DEFS.filter((file) => applyEnvChecked[file.key]).map((file) => file.key);
  if (!keys.length) return;
  applyLogs.value = [];
  if (!pathInput.value.trim() || !report.value?.directory) {
    pushApplyLog(t("gcpProjectRequired"), "error");
    return;
  }
  if (hasDirty.value) {
    pushApplyLog(t("gcpApplyUnsaved"), "error");
    return;
  }
  if (!isTauri) {
    pushApplyLog(t("gcpDesktopOnly"), "error");
    return;
  }
  applying.value = true;
  beginApplyStatus();
  pushApplyLog("准备执行，日志会随着命令输出逐行出现", "out");
  pushApplyLog(`项目路径：${pathInput.value.trim()}`, "out");
  pushApplyLog(`勾选环境：${keys.join("、")}`, "out");
  let stop = () => {};
  let finished = { value: false };
  const finish = (unlisten) => {
    if (finished.value) return;
    finished.value = true;
    applying.value = false;
    unlisten?.();
  };
  let unlisten = stop;
  try {
    const { listen } = await import("@tauri-apps/api/event");
    unlisten = await listen("gcp-apply-log", (event) => {
      const line = event.payload || {};
      if (line.text) pushApplyLog(String(line.text), line.tone || "out");
      if (line.done) {
        endApplyStatus(line.tone);
        queueMicrotask(() => finish(unlisten));
      }
    });
    const result = await applyGcpWhitelist(pathInput.value.trim(), keys);
    if (!result) {
      pushApplyLog(t("gcpDesktopOnly"), "error");
      endApplyStatus("error");
      finish(unlisten);
    }
  } catch (error) {
    pushApplyLog(String(error?.message || error), "error");
    endApplyStatus("error");
    finish(unlisten);
  }
}

function markDeleted(cell) {
  const entry = findEntry(cell.fileKey, cell.id);
  if (!entry || entry.status === "deleted") return;
  entry.status = "deleted";
}

function cellClass(cell, row) {
  return {
    "is-gap": !cell.cidr,
    "is-only": cell.cidr && row.cells.some((item) => !item.cidr),
    "is-deleted": cell.status === "deleted",
  };
}

async function saveToFiles() {
  if (!hasDirty.value || saving.value) return;
  if (!pathInput.value.trim() || !report.value?.directory) {
    message.warning(t("gcpProjectRequired"));
    return;
  }
  if (!report.value.onMaster) {
    message.warning(report.value.branch
      ? t("gcpBranchOther", { branch: report.value.branch })
      : t("gcpCreateNotMaster"));
    return;
  }
  saving.value = true;
  try {
    const files = FILE_DEFS.filter((file) => !envMissing(file.key)).map((file) => ({
      key: file.key,
      ips: (drafts[file.key] || [])
        .filter((item) => item.status !== "deleted")
        .map((item) => ({ cidr: item.cidr, comment: item.comment || "" })),
    }));
    const result = await saveGcpWhitelist(pathInput.value.trim(), files);
    if (!result) {
      message.warning(t("gcpDesktopOnly"));
      return;
    }
    if (result.inspect) {
      report.value = result.inspect;
      if (result.code === "ok") loadDrafts(result.inspect.files || []);
    }
    if (result.code === "ok") {
      message.success(t("gcpSaveOk"));
      return;
    }
    if (result.code === "not_master") {
      message.warning(t("gcpCreateNotMaster"));
      return;
    }
    if (result.code === "not_directory") {
      message.warning(t("gcpNotDirectory"));
      return;
    }
    if (result.code === "invalid") {
      message.warning(t("gcpCreateInvalid"));
      return;
    }
    message.error(t("gcpSaveFail", { error: result.code }));
  } catch (error) {
    message.error(t("gcpSaveFail", { error: String(error) }));
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <div class="gcp-page">
    <main class="settings-main">
      <section class="settings-panel">
        <div class="gcp-head">
          <h2>{{ t("gcpWhiteList") }}</h2>
        </div>
        <form class="gcp-path" @submit.prevent="commitPath">
          <label>
            <span>{{ t("gcpProjectLabel") }}</span>
            <a-input
              v-model:value="pathInput"
              spellcheck="false"
              :placeholder="t('gcpProjectPlaceholder')"
              @press-enter="commitPath"
              @blur="commitPath"
            />
          </label>
          <a-button :disabled="checking" :loading="checking" @click="browse">{{ t("gcpProjectBrowse") }}</a-button>
          <span class="gcp-split" aria-hidden="true" />
          <a-button html-type="button" @click="openCreate">{{ t("gcpCreate") }}</a-button>
          <a-button type="primary" html-type="button" :disabled="!hasDirty" :loading="saving" @click="saveToFiles">
            {{ t("gcpSave") }}
          </a-button>
          <a-button html-type="button" @click="openApplyGcp">{{ t("gcpApplyGcp") }}</a-button>
          <a-button html-type="button" :loading="fetching" @click="openFetchGcp">{{ t("gcpFetch") }}</a-button>
        </form>
        <ul v-if="notes.length" class="gcp-notes">
          <li v-for="note in notes" :key="note.text" :class="note.tone">{{ note.text }}</li>
        </ul>
        <p v-if="!report" class="gcp-empty">{{ t("gcpProjectEmpty") }}</p>
        <p v-else-if="report.directory && !rows.length" class="gcp-empty">{{ t("gcpIpEmpty") }}</p>
        <div v-if="report?.directory" class="gcp-diff">
          <div v-for="file in columns" :key="file.key" class="gcp-col-head" :title="file.relativePath">
            <b>{{ file.title }}</b>
            <span :class="{ 'is-missing': file.exists === false, 'is-ok': file.exists }">{{ fileStatus(file) }}</span>
            <small>{{ file.relativePath }}</small>
          </div>
          <template v-for="(row, rowIndex) in rows" :key="rowIndex">
            <div
              v-for="(cell, colIndex) in row.cells"
              :key="cell.id || `${rowIndex}-${colIndex}`"
              class="gcp-cell"
              :class="cellClass(cell, row)"
            >
              <div
                v-if="cell.cidr"
                class="ip-line"
                :class="{ 'is-deleted': cell.status === 'deleted', 'is-editable': canChangeCell(cell) && cell.status !== 'deleted' }"
                @click="openComment(cell)"
              >
                <code class="ip-cidr">{{ cell.cidr }}</code>
                <sup
                  v-if="cell.status === 'added' || cell.status === 'edited'"
                  class="ip-dirty"
                  :title="t('gcpUnsaved')"
                >*</sup>
                <span v-if="cell.comment" class="ip-note">{{ cell.comment }}</span>
              </div>
              <div v-if="canChangeCell(cell) && (cell.cidr || cell.rowCidr)" class="ip-actions">
                <button
                  v-if="!cell.cidr"
                  type="button"
                  class="ip-action"
                  :title="t('gcpApply')"
                  :aria-label="t('gcpApply')"
                  @click.stop="copyCell(cell)"
                >
                  <CopyOutlined />
                </button>
                <button
                  v-if="cell.cidr"
                  type="button"
                  class="ip-action is-delete"
                  :title="t('gcpDelete')"
                  :aria-label="t('gcpDelete')"
                  :disabled="cell.status === 'deleted'"
                  @click.stop="markDeleted(cell)"
                >
                  <DeleteOutlined />
                </button>
              </div>
            </div>
          </template>
        </div>
      </section>
    </main>
    <a-modal
      :open="createOpen"
      :title="t('gcpCreateTitle')"
      :width="520"
      :mask-closable="false"
      :footer="null"
      @cancel="closeCreate"
    >
      <form class="gcp-create" @submit.prevent="saveCreate">
        <div class="gcp-create-fields">
          <label>
            <span>{{ t("gcpCreateIp") }}</span>
            <a-input
              v-model:value="createIp"
              spellcheck="false"
              :placeholder="t('gcpCreateIpPlaceholder')"
            />
          </label>
          <label>
            <span>{{ t("gcpCreateComment") }}</span>
            <a-input
              v-model:value="createComment"
              spellcheck="false"
              :placeholder="t('gcpCreateCommentPlaceholder')"
            />
          </label>
        </div>
        <div class="gcp-create-envs">
          <span>{{ t("gcpCreateEnvs") }}</span>
          <div class="env-grid">
            <button
              v-for="file in FILE_DEFS"
              :key="file.key"
              type="button"
              class="env-card"
              :class="{ 'is-on': envChecked[file.key] }"
              :disabled="envMissing(file.key)"
              :aria-pressed="envChecked[file.key]"
              @click="envChecked[file.key] = !envChecked[file.key]"
            >
              <span class="env-mark" />
              <span class="env-copy">
                <b>{{ file.key }}</b>
                <small>{{ file.title }}</small>
              </span>
            </button>
          </div>
        </div>
        <div class="gcp-create-actions">
          <a-button @click="closeCreate">{{ t("gcpCreateCancel") }}</a-button>
          <a-button type="primary" html-type="submit">{{ t("gcpCreateSave") }}</a-button>
        </div>
      </form>
    </a-modal>
    <a-modal
      :open="commentOpen"
      :title="t('gcpCommentTitle')"
      :width="520"
      :mask-closable="false"
      :footer="null"
      @cancel="closeComment"
    >
      <form class="gcp-create" @submit.prevent="saveComment">
        <div class="gcp-create-fields">
          <label>
            <span>{{ t("gcpCreateEnvs") }}</span>
            <a-input :value="commentEnvLabel" disabled />
          </label>
          <label>
            <span>{{ t("gcpCreateIp") }}</span>
            <a-input :value="commentIp" disabled />
          </label>
          <label>
            <span>{{ t("gcpCreateComment") }}</span>
            <a-input
              v-model:value="commentText"
              spellcheck="false"
              :placeholder="t('gcpCreateCommentPlaceholder')"
            />
          </label>
        </div>
        <p class="gcp-comment-hint">{{ t("gcpCommentHint") }}</p>
        <div class="gcp-create-actions">
          <a-button @click="closeComment">{{ t("gcpCreateCancel") }}</a-button>
          <a-button type="primary" html-type="submit">{{ t("gcpCommentSave") }}</a-button>
        </div>
      </form>
    </a-modal>
    <a-modal
      :open="applyGcpOpen"
      class="gcp-apply-modal"
      :title="t('gcpApplyGcpTitle')"
      width="80%"
      :mask-closable="false"
      :footer="null"
      @cancel="closeApplyGcp"
    >
      <div class="gcp-apply">
        <div class="gcp-apply-envs">
          <span>{{ t("gcpCreateEnvs") }}</span>
          <div class="gcp-apply-checks">
            <a-checkbox
              v-for="file in FILE_DEFS"
              :key="file.key"
              v-model:checked="applyEnvChecked[file.key]"
              class="gcp-apply-check"
              :class="{ 'is-off': envMissing(file.key) }"
              :disabled="envMissing(file.key)"
            >
              <span class="gcp-apply-env">
                <b>{{ file.key }}</b>
                <small>{{ file.title }}</small>
              </span>
            </a-checkbox>
          </div>
        </div>
        <div class="gcp-apply-bar">
          <span>{{ t("gcpApplyGcpLog") }}</span>
          <a-button
            type="primary"
            html-type="button"
            :disabled="applying || !FILE_DEFS.some((file) => applyEnvChecked[file.key])"
            :loading="applying"
            @click="runApplyGcp"
          >{{ applying ? t("gcpApplyRunning") : t("gcpApplyGcpRun") }}</a-button>
        </div>
        <div class="gcp-apply-status" :class="applying ? 'is-running' : applyOutcome ? `is-${applyOutcome}` : ''">
          <i />
          <b>{{ applying ? t("gcpApplyRunning") : applyOutcome === "ok" ? t("gcpApplyDone") : applyOutcome === "error" ? t("gcpApplyFailed") : t("gcpApplyIdle") }}</b>
          <span v-if="applying">{{ applyPhase }} · {{ t("gcpApplyWaited", { seconds: applyWaited }) }}</span>
        </div>
        <div ref="applyLogEl" class="gcp-log" role="log" aria-live="polite">
          <p v-if="!applyLogs.length" class="gcp-log-empty">{{ t("gcpApplyGcpLogEmpty") }}</p>
          <p
            v-for="(line, index) in applyLogs"
            :key="index"
            class="gcp-log-line"
            :class="line.tone ? `is-${line.tone}` : ''"
          >{{ line.text }}</p>
        </div>
      </div>
    </a-modal>
    <a-modal
      :open="fetchOpen"
      class="gcp-apply-modal"
      :title="t('gcpFetchTitle')"
      width="92%"
      :mask-closable="false"
      :footer="null"
      @cancel="closeFetchGcp"
    >
      <div class="gcp-fetch">
        <div class="gcp-fetch-grid">
          <section v-for="col in fetchCols" :key="col.key" class="gcp-fetch-col">
            <header>
              <b>{{ col.title }}</b>
              <span :class="`is-${col.status}`">{{ fetchStatusText(col.status) }}</span>
              <small>{{ col.relativePath }}</small>
              <em v-if="col.message">{{ col.message }}</em>
            </header>
            <pre>{{ col.yaml || (col.status === "running" ? t("gcpFetchWaiting") : t("gcpFetchEmpty")) }}</pre>
          </section>
        </div>
      </div>
    </a-modal>
  </div>
</template>

<style scoped>
.gcp-page {
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--surface-muted);
}
.settings-main {
  flex: 1;
  overflow: auto;
}
.gcp-head {
  margin-bottom: 14px;
}
.gcp-head h2 {
  margin: 0;
}
.gcp-create {
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.gcp-create-fields {
  display: grid;
  grid-template-columns: minmax(0, 1.1fr) minmax(0, 0.9fr);
  gap: 10px;
}
.gcp-create-fields label,
.gcp-create-envs {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
}
.gcp-create-fields span,
.gcp-create-envs > span {
  color: var(--muted);
  font-size: 12px;
  font-weight: 650;
}
.env-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}
.env-card {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  margin: 0;
  padding: 8px 10px;
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--panel);
  color: var(--ink);
  font: inherit;
  text-align: left;
  cursor: pointer;
}
.env-card.is-on {
  border-color: var(--teal);
  background: var(--teal-soft);
}
.env-card:disabled {
  cursor: not-allowed;
  opacity: 0.45;
}
.env-mark {
  position: relative;
  flex: 0 0 auto;
  width: 14px;
  height: 14px;
  border: 1px solid var(--line-hover);
  border-radius: 2px;
  background: var(--panel);
}
.env-card.is-on .env-mark {
  border-color: var(--teal);
  background: var(--teal);
}
.env-card.is-on .env-mark::after {
  content: "";
  position: absolute;
  top: 1px;
  left: 4px;
  width: 4px;
  height: 7px;
  border: solid #fff;
  border-width: 0 1.5px 1.5px 0;
  transform: rotate(45deg);
}
.env-copy {
  display: flex;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
}
.env-copy b,
.env-copy small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.env-copy b {
  color: var(--ink-strong);
  font-size: 12px;
  font-weight: 700;
  line-height: 1.3;
}
.env-copy small {
  color: var(--muted);
  font-size: 10px;
  line-height: 1.3;
}
.gcp-create-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 2px;
  padding-top: 12px;
  border-top: 1px solid var(--line-soft);
}
.gcp-apply {
  display: flex;
  flex-direction: column;
  gap: 10px;
  height: calc(100vh - 196px);
  min-height: 360px;
}
.gcp-apply-envs {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.gcp-apply-envs > span {
  color: var(--muted);
  font-size: 12px;
}
.gcp-apply-checks {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}
.gcp-apply-check {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  margin: 0;
  padding: 8px 10px;
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--panel);
}
.gcp-apply-check.is-off {
  cursor: not-allowed;
  opacity: 0.55;
}
.gcp-apply-env {
  display: flex;
  flex-direction: column;
  min-width: 0;
}
.gcp-apply-env b,
.gcp-apply-env small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.gcp-apply-env b {
  color: var(--ink-strong);
  font-size: 12px;
  font-weight: 700;
  line-height: 1.3;
}
.gcp-apply-env small {
  color: var(--muted);
  font-size: 10px;
  line-height: 1.3;
}
.gcp-apply-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.gcp-apply-bar span {
  color: var(--muted);
  font-size: 12px;
}
.gcp-apply-status {
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 36px;
  margin-top: 10px;
  padding: 8px 12px;
  border-radius: var(--radius);
  background: #1c2430;
  color: #d7e0ea;
  font-size: 13px;
}
.gcp-apply-status i {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #8b98a8;
  flex: none;
}
.gcp-apply-status b {
  flex: none;
}
.gcp-apply-status span {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.gcp-apply-status.is-running {
  background: #1e3a5f;
  color: #dbeafe;
}
.gcp-apply-status.is-running i {
  background: #38bdf8;
  animation: gcp-pulse 1.2s infinite;
}
.gcp-apply-status.is-running b {
  color: #7dd3fc;
}
.gcp-apply-status.is-ok {
  background: #14532d;
  color: #dcfce7;
}
.gcp-apply-status.is-ok i,
.gcp-apply-status.is-ok b {
  background: transparent;
  color: #86efac;
}
.gcp-apply-status.is-ok i {
  background: #4ade80;
}
.gcp-apply-status.is-error {
  background: #7f1d1d;
  color: #fee2e2;
}
.gcp-apply-status.is-error i {
  background: #fca5a5;
}
.gcp-apply-status.is-error b {
  color: #fecaca;
}
@keyframes gcp-pulse {
  0% { box-shadow: 0 0 0 0 rgba(56, 189, 248, 0.75); }
  70% { box-shadow: 0 0 0 8px rgba(56, 189, 248, 0); }
  100% { box-shadow: 0 0 0 0 rgba(56, 189, 248, 0); }
}
.gcp-log {
  flex: 1;
  min-height: 0;
  overflow-x: hidden;
  overflow-y: scroll;
  margin: 0;
  padding: 12px 14px;
  border: 1px solid #1c2430;
  border-radius: var(--radius);
  background: #12181f;
  color: #d7e0ea;
  font-family: "IBM Plex Mono", ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 12px;
  line-height: 1.55;
  scrollbar-color: #5c6b7a #1a222c;
  scrollbar-width: thin;
}
.gcp-log::-webkit-scrollbar {
  width: 10px;
}
.gcp-log::-webkit-scrollbar-track {
  background: #1a222c;
}
.gcp-log::-webkit-scrollbar-thumb {
  background: #5c6b7a;
  border-radius: 6px;
}
.gcp-log-empty,
.gcp-log-line {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
}
.gcp-log-empty {
  color: #8b98a8;
}
.gcp-log-line.is-cmd {
  color: #9fd0ff;
}
.gcp-log-line.is-busy {
  color: #7dd3fc;
}
.gcp-log-line.is-rule {
  color: #f2f5f8;
  font-weight: 700;
}
.gcp-log-line.is-ok {
  color: #86efac;
  font-weight: 700;
}
.gcp-log-line.is-warn {
  color: #ffd27a;
  font-weight: 700;
  background: #5c3b12;
}
.gcp-log-line.is-error {
  color: #ffd2d2;
  font-weight: 700;
  background: #7f1d1d;
}
.gcp-fetch {
  height: calc(100vh - 180px);
  min-height: 420px;
}
.gcp-fetch-grid {
  height: 100%;
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 8px;
}
.gcp-fetch-col {
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  border: 1px solid #1c2430;
  border-radius: var(--radius);
  overflow: hidden;
  background: #12181f;
}
.gcp-fetch-col header {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 8px 10px;
  background: #1a222c;
  border-bottom: 1px solid #1c2430;
}
.gcp-fetch-col header b {
  color: #f2f5f8;
  font-size: 12px;
}
.gcp-fetch-col header span,
.gcp-fetch-col header small,
.gcp-fetch-col header em {
  color: #8b98a8;
  font-size: 11px;
  font-style: normal;
  line-height: 1.35;
  word-break: break-all;
}
.gcp-fetch-col header span.is-running {
  color: #7dd3fc;
}
.gcp-fetch-col header span.is-ok {
  color: #86efac;
}
.gcp-fetch-col header span.is-error {
  color: #ffd2d2;
}
.gcp-fetch-col pre {
  flex: 1;
  min-height: 0;
  margin: 0;
  padding: 10px;
  overflow: auto;
  color: #d7e0ea;
  font-family: "IBM Plex Mono", ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 11px;
  line-height: 1.45;
  white-space: pre;
}
.gcp-path {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto 12px auto auto auto auto;
  gap: 8px;
  align-items: end;
  margin-bottom: 12px;
}
.gcp-split {
  width: 1px;
  height: 22px;
  align-self: end;
  justify-self: center;
  background: var(--line);
}
.gcp-path label {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
  font-size: 12px;
  color: var(--muted);
}
.gcp-notes {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin: 0 0 12px;
  padding: 0;
  list-style: none;
}
.gcp-notes li {
  margin: 0;
  padding: 4px 8px;
  border-radius: var(--radius);
  font-size: 12px;
  line-height: 1.4;
}
.gcp-notes .is-ok {
  color: #166534;
  background: #dcfce7;
}
.gcp-notes .is-warn {
  color: #b45309;
  background: #fef3c7;
}
.gcp-notes .is-bad {
  color: #b91c1c;
  background: #fee2e2;
}
.gcp-empty {
  margin: 0 0 12px;
  color: var(--muted);
}
.gcp-diff {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--panel);
}
.gcp-col-head,
.gcp-cell {
  min-width: 0;
}
.gcp-diff > :not(:nth-child(4n + 1)) {
  border-left: 1px solid var(--line);
}
.gcp-col-head {
  position: sticky;
  top: 0;
  z-index: 1;
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 8px;
  background: var(--surface-tint);
}
.gcp-col-head b {
  font-size: 12px;
  line-height: 1.3;
  word-break: break-all;
}
.gcp-col-head span,
.gcp-col-head small {
  color: var(--muted);
  font-size: 10px;
  line-height: 1.35;
  word-break: break-all;
}
.gcp-col-head span.is-ok {
  color: #166534;
}
.gcp-col-head span.is-missing {
  color: #b91c1c;
}
.gcp-cell {
  position: relative;
  display: flex;
  align-items: center;
  min-height: 34px;
  padding: 6px 8px;
  border-top: 1px solid var(--line);
}
.gcp-cell:hover {
  padding-right: 52px;
}
.ip-actions {
  position: absolute;
  top: 50%;
  right: 4px;
  display: none;
  align-items: center;
  gap: 2px;
  transform: translateY(-50%);
}
.gcp-cell:hover .ip-actions {
  display: flex;
}
.ip-action {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  padding: 0;
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--panel);
  color: var(--ink-soft);
  cursor: pointer;
}
.ip-action:hover {
  border-color: var(--teal);
  color: var(--teal);
}
.ip-action.is-delete:hover {
  border-color: #dc2626;
  color: #dc2626;
}
.ip-action:disabled {
  cursor: default;
  opacity: 0.35;
}
.ip-line.is-deleted .ip-cidr,
.ip-line.is-deleted .ip-note {
  color: #dc2626;
  text-decoration: line-through;
}
.ip-dirty {
  margin-right: 2px;
  color: #dc2626;
  font-size: 12px;
  font-weight: 700;
  line-height: 1;
}
.gcp-cell.is-gap {
  background: #eceff1;
}
.gcp-cell.is-only {
  background: #fff7ed;
}
.ip-line {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 6px;
  min-width: 0;
}
.ip-line.is-editable {
  cursor: pointer;
}
.gcp-comment-hint {
  margin: 0;
  color: var(--muted);
  font-size: 12px;
}
.ip-cidr {
  margin: 0;
  color: var(--ink-strong);
  font-size: 12px;
  font-weight: 650;
  line-height: 1.35;
  word-break: break-all;
}
.ip-note {
  color: var(--muted);
  font-size: 11px;
  line-height: 1.35;
}
</style>

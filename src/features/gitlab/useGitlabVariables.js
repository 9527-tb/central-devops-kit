/**
 * 进入变量页时检查本机保存的 group 地址和 Access Token，齐全后再读取变量。
 */

import { onBeforeUnmount, ref, watch } from "vue";
import { message } from "antdv-next";
import { isTauri } from "../../api/tauri.js";
import { loadGitlabVariables, deleteGitlabVariable, createGitlabVariable, updateGitlabVariable } from "../../api/gitlab.js";
import { createTranslator } from "../../i18n/index.js";
import {
  locale,
  persistSettings,
  settings,
  settingsLoading,
} from "../../stores/settings.js";

let ticket = 0;

export function useGitlabVariables() {
  const t = createTranslator(locale);
  const loading = ref(true);
  const status = ref("loading");
  const messageText = ref("");
  const tokenInput = ref("");
  const groupUrlInput = ref("");
  const settingsOpen = ref(false);
  let suppressAutoSettings = false;
  const rows = ref([]);
  const summary = ref(null);
  const page = ref(1);
  const pageSize = ref(50);
  const total = ref(0);

  function failText(result) {
    if (!result) return t("gitlabNeedDesktop");
    if (result.code === "not_found") return t("gitlabNotFound");
    if (result.code === "forbidden") return t("gitlabForbidden");
    if (result.code === "bad_response") return t("gitlabBadResponse");
    if (result.code === "bad_address") return t("gitlabGroupInvalid");
    if (result.code === "network") {
      return t("gitlabNetwork", { error: result.message || "—" });
    }
    return t("gitlabLoadFail", { error: result.message || result.code || "—" });
  }

  function sameVariable(row, key, scope) {
    return String(row?.key || "") === key && String(row?.environmentScope || "").trim() === scope;
  }

  function dropInFlightLoad() {
    ticket += 1;
    loading.value = false;
    if (status.value === "loading") status.value = "ready";
  }

  function forgetVariable(key, scope) {
    const next = rows.value.filter((row) => !sameVariable(row, key, scope));
    const removed = rows.value.length - next.length;
    if (!removed) return;
    dropInFlightLoad();
    rows.value = next;
    total.value = Math.max(0, Number(total.value) - removed);
  }

  function rememberVariable(input) {
    const key = String(input?.key || "").trim();
    const environmentScope = String(input?.environmentScope || "").trim() || "*";
    if (!key) return;
    const row = {
      key,
      value: String(input?.value ?? ""),
      hidden: false,
      variableType: input?.variableType === "file" ? "file" : "env_var",
      isProtected: !!input?.isProtected,
      isMasked: !!input?.isMasked,
      expanded: !!input?.expanded,
      environmentScope,
      description: String(input?.description || "").trim(),
    };
    dropInFlightLoad();
    status.value = "ready";
    const index = rows.value.findIndex((item) => sameVariable(item, key, environmentScope));
    if (index >= 0) {
      const next = rows.value.slice();
      next[index] = row;
      rows.value = next;
      return;
    }
    rows.value = [...rows.value, row];
    total.value = Number(total.value) + 1;
    status.value = "ready";
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
    if (status.value === "missing" || status.value === "invalid") {
      suppressAutoSettings = true;
    }
  }

  function showSettingsIfNeeded() {
    if ((status.value === "missing" || status.value === "invalid") && !suppressAutoSettings && !settingsOpen.value) {
      openSettings();
    }
  }

  async function refresh(options = {}) {
    const nextPage = Math.max(1, Number(options.page) || 1);
    const current = ++ticket;
    if (options.reopenSettings) suppressAutoSettings = false;
    loading.value = true;
    status.value = "loading";
    page.value = nextPage;
    const { token, groupUrl } = savedGitlab();
    if (!token || !groupUrl) {
      if (current !== ticket) return;
      rows.value = [];
      summary.value = null;
      total.value = 0;
      status.value = "missing";
      loading.value = false;
      showSettingsIfNeeded();
      return;
    }
    if (!isTauri) {
      if (current !== ticket) return;
      rows.value = [];
      summary.value = null;
      total.value = 0;
      status.value = "failed";
      messageText.value = t("gitlabNeedDesktop");
      loading.value = false;
      return;
    }
    try {
      const result = await loadGitlabVariables(token, groupUrl, nextPage);
      if (current !== ticket) return;
      if (!result || result.status === "missing") {
        rows.value = [];
        summary.value = null;
        total.value = 0;
        status.value = "missing";
        showSettingsIfNeeded();
        return;
      }
      if (result.status === "invalid") {
        rows.value = [];
        summary.value = null;
        total.value = 0;
        status.value = "invalid";
        showSettingsIfNeeded();
        return;
      }
      if (result.status !== "ok") {
        rows.value = [];
        summary.value = null;
        total.value = 0;
        status.value = "failed";
        messageText.value = failText(result);
        return;
      }
      rows.value = Array.isArray(result.variables) ? result.variables : [];
      page.value = Number(result.page) || nextPage;
      pageSize.value = Number(result.perPage) || 50;
      total.value = Number.isFinite(Number(result.total)) ? Number(result.total) : rows.value.length;
      summary.value = {
        username: result.username || "",
        resourceKind: result.resourceKind || "project",
        resourceName: result.resourceName || "",
        resourcePath: result.resourcePath || "",
      };
      status.value = "ready";
      tokenInput.value = "";
    } catch (error) {
      if (current !== ticket) return;
      rows.value = [];
      summary.value = null;
      total.value = 0;
      status.value = "failed";
      messageText.value = t("gitlabLoadFail", { error: String(error) });
    } finally {
      if (current === ticket) loading.value = false;
    }
  }

  function changePage(next) {
    const target = Math.max(1, Number(next) || 1);
    if (target === page.value && status.value === "ready") return;
    return refresh({ page: target });
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
    const current = ++ticket;
    loading.value = true;
    status.value = "loading";
    settingsOpen.value = false;
    suppressAutoSettings = false;
    try {
      await persistSettings({
        ...settings.value,
        gitlab: { accessToken: token, groupUrl },
      });
    } catch (error) {
      if (current !== ticket) return;
      loading.value = false;
      status.value = "missing";
      settingsOpen.value = true;
      message.error(t("settingsSaveFail", { error: String(error) }));
      return;
    }
    if (current !== ticket) return;
    await refresh();
  }

  async function removeVariable(record) {
    const key = String(record?.key || "").trim();
    const environmentScope = String(record?.environmentScope || "").trim();
    const { token, groupUrl } = savedGitlab();
    if (!token || !groupUrl || !key) {
      message.warning(t("gitlabSettingsRequired"));
      return false;
    }
    if (!isTauri) {
      message.info(t("gitlabNeedDesktop"));
      return false;
    }
    try {
      const result = await deleteGitlabVariable(token, groupUrl, key, environmentScope);
      if (!result || result.status !== "ok") {
        const text =
          result?.code === "not_found"
            ? t("gitlabDeleteMissing", { key })
            : failText(result);
        message.error(text);
        return false;
      }
    } catch (error) {
      message.error(t("gitlabDeleteFail", { error: String(error) }));
      return false;
    }
    message.success(t("gitlabDeleted", { key }));
    forgetVariable(key, environmentScope);
    return true;
  }

  async function updateVariable(record, value) {
    const key = String(record?.key || "").trim();
    const environmentScope = String(record?.environmentScope || "").trim();
    const { token, groupUrl } = savedGitlab();
    if (!token || !groupUrl || !key || !environmentScope) {
      message.warning(t("gitlabSettingsRequired"));
      return false;
    }
    if (!isTauri) {
      message.info(t("gitlabNeedDesktop"));
      return false;
    }
    try {
      const result = await updateGitlabVariable(token, groupUrl, key, environmentScope, String(value ?? ""));
      if (!result || result.status !== "ok") {
        const detail = String(result?.message || "").trim();
        const text =
          result?.code === "not_found"
            ? t("gitlabDeleteMissing", { key })
            : detail
              ? t("gitlabEditFail", { error: detail })
              : failText(result);
        message.error(text);
        return false;
      }
    } catch (error) {
      message.error(t("gitlabEditFail", { error: String(error) }));
      return false;
    }
    rememberVariable({
      ...record,
      key,
      environmentScope,
      value: String(value ?? ""),
      hidden: false,
    });
    message.success(t("gitlabEdited", { key }));
    return true;
  }

  async function createVariable(input, options = {}) {
    const key = String(input?.key || "").trim();
    const environmentScope = String(input?.environmentScope || "").trim();
    const { token, groupUrl } = savedGitlab();
    if (!token || !groupUrl) {
      message.warning(t("gitlabSettingsRequired"));
      return false;
    }
    if (!isTauri) {
      message.info(t("gitlabNeedDesktop"));
      return false;
    }
    if (!/^[A-Za-z0-9_]+$/.test(key)) {
      message.warning(t("gitlabCreateKeyInvalid"));
      return false;
    }
    if (!environmentScope) {
      message.warning(t("gitlabCreateNeedScope"));
      return false;
    }
    try {
      const result = await createGitlabVariable({
        token,
        groupUrl,
        key,
        value: String(input?.value ?? ""),
        environmentScope,
        variableType: input?.variableType === "file" ? "file" : "env_var",
        isProtected: !!input?.isProtected,
        isMasked: !!input?.isMasked,
        expanded: !!input?.expanded,
        description: String(input?.description || "").trim(),
      });
      if (!result || result.status !== "ok") {
        const detail = String(result?.message || "").trim();
        message.error(detail ? t("gitlabCreateFail", { error: detail }) : failText(result));
        return false;
      }
    } catch (error) {
      message.error(t("gitlabCreateFail", { error: String(error) }));
      return false;
    }
    if (!options.quiet) message.success(t("gitlabCreated", { key }));
    rememberVariable({ ...input, key, environmentScope });
    return true;
  }

  async function createVariables(items) {
    const list = Array.isArray(items) ? items : [];
    if (!list.length) {
      message.warning(t("gitlabCreateNeedScope"));
      return false;
    }
    for (const item of list) {
      const ok = await createVariable(item, { quiet: true });
      if (!ok) return false;
    }
    message.success(t("gitlabCreatedMany", { count: list.length }));
    return true;
  }

  watch(
    settingsLoading,
    (loadingNow) => {
      if (!loadingNow) refresh();
    },
    { immediate: true },
  );

  onBeforeUnmount(() => {
    ticket += 1;
  });

  return {
    loading,
    status,
    messageText,
    tokenInput,
    groupUrlInput,
    settingsOpen,
    rows,
    summary,
    page,
    pageSize,
    total,
    refresh,
    changePage,
    openSettings,
    closeSettings,
    saveGitlabSettings,
    removeVariable,
    updateVariable,
    createVariable,
    createVariables,
  };
}

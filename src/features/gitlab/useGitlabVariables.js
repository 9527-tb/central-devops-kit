/**
 * 进入变量页时检查本机保存的 group 地址和 Access Token，齐全后再读取变量。
 */

import { onBeforeUnmount, ref, watch } from "vue";
import { message } from "antdv-next";
import { isTauri } from "../../api/tauri.js";
import { loadGitlabVariables } from "../../api/gitlab.js";
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

  function groupAddressOk(value) {
    try {
      const url = new URL(value);
      const path = url.pathname.replace(/^\/+|\/+$/g, "");
      return (url.protocol === "http:" || url.protocol === "https:") && Boolean(url.host && path);
    } catch {
      return false;
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
      settingsOpen.value = false;
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
  };
}

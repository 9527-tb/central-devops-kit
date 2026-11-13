/**
 * 进入变量页时检查 Access Token，有效后再拉取固定路径下的变量。
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
    if (result.code === "network") {
      return t("gitlabNetwork", { error: result.message || "—" });
    }
    return t("gitlabLoadFail", { error: result.message || result.code || "—" });
  }

  async function refresh(options = {}) {
    const nextPage = Math.max(1, Number(options.page) || 1);
    const current = ++ticket;
    loading.value = true;
    status.value = "loading";
    page.value = nextPage;
    const token = String(settings.value.gitlab?.accessToken || "").trim();
    if (!token) {
      if (current !== ticket) return;
      rows.value = [];
      summary.value = null;
      total.value = 0;
      status.value = "missing";
      loading.value = false;
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
      const result = await loadGitlabVariables(token, nextPage);
      if (current !== ticket) return;
      if (!result || result.status === "missing") {
        rows.value = [];
        summary.value = null;
        total.value = 0;
        status.value = "missing";
        return;
      }
      if (result.status === "invalid") {
        rows.value = [];
        summary.value = null;
        total.value = 0;
        status.value = "invalid";
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

  async function saveToken() {
    const token = tokenInput.value.trim();
    if (!token) {
      message.warning(t("gitlabTokenRequired"));
      return;
    }
    const current = ++ticket;
    loading.value = true;
    status.value = "loading";
    try {
      await persistSettings({
        ...settings.value,
        gitlab: { accessToken: token },
      });
    } catch (error) {
      if (current !== ticket) return;
      loading.value = false;
      status.value = String(settings.value.gitlab?.accessToken || "").trim()
        ? "invalid"
        : "missing";
      message.error(t("settingsSaveFail", { error: String(error) }));
      return;
    }
    if (current !== ticket) return;
    await refresh();
  }

  function editToken() {
    status.value = "edit";
    messageText.value = "";
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
    rows,
    summary,
    page,
    pageSize,
    total,
    refresh,
    changePage,
    saveToken,
    editToken,
  };
}

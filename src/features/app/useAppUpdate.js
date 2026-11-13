/**
 * 应用内更新。清单地址和签名公钥配在 tauri.conf.json 的 plugins.updater。
 * 当前未配置清单，检查会失败并给出说明。
 */

import { onMounted, onBeforeUnmount, ref } from "vue";
import { message } from "antdv-next";
import { getVersion } from "@tauri-apps/api/app";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { isTauri } from "../../api/tauri.js";
import { createTranslator } from "../../i18n/index.js";
import { locale, previewMode, settings } from "../../stores/settings.js";

const t = createTranslator(locale);

const STARTUP_DELAY_MS = 3000;
const CHECK_TIMEOUT_MS = 15_000;
const IS_DEV = !!import.meta.env.DEV;

export function useAppUpdate() {
  const updateOpen = ref(false);
  const updateInfo = ref(null);
  const downloading = ref(false);
  const downloadPercent = ref(0);
  const checking = ref(false);
  const appVersion = ref("");
  const updateAvailable = ref(false);

  /** @type {import('@tauri-apps/plugin-updater').Update | null} */
  let pendingUpdate = null;
  let startupTimer = null;

  async function refreshAppVersion() {
    if (!isTauri || previewMode.value) {
      appVersion.value = "";
      return;
    }
    try {
      appVersion.value = await getVersion();
    } catch {
      appVersion.value = "";
    }
  }

  async function checkForUpdate(opts = {}) {
    const silent = !!opts.silent;
    if (!isTauri || previewMode.value) {
      if (!silent) message.info(t("previewNeedTauriShort"));
      return false;
    }
    if (checking.value || downloading.value) return false;

    if (!silent && pendingUpdate && updateAvailable.value) {
      updateOpen.value = true;
      return true;
    }

    checking.value = true;
    try {
      const update = await check({ timeout: CHECK_TIMEOUT_MS });
      if (!update) {
        updateAvailable.value = false;
        pendingUpdate = null;
        updateInfo.value = null;
        if (!silent) message.success(t("updateUpToDate"));
        return false;
      }
      pendingUpdate = update;
      updateAvailable.value = true;
      updateInfo.value = {
        version: update.version,
        notes: update.body || "",
        date: update.date || "",
      };
      updateOpen.value = true;
      return true;
    } catch (error) {
      const raw = String(error || "");
      if (!silent) {
        message.error(t("updateCheckFail", { error: friendlyUpdateError(raw) }));
      } else {
        console.warn("[update] silent check failed", raw);
      }
      return false;
    } finally {
      checking.value = false;
    }
  }

  function friendlyUpdateError(raw) {
    const s = String(raw || "");
    if (/404|Not Found|Could not fetch|endpoint|no updater/i.test(s)) {
      return t("updateCheckNoRelease");
    }
    if (/signature|verify|pubkey|minisign/i.test(s)) {
      return t("updateCheckBadSignature");
    }
    if (/network|fetch|timed out|timeout|time\s*out|dns|error sending request/i.test(s)) {
      return t("updateCheckNetwork");
    }
    return s;
  }

  function dismissUpdate() {
    if (downloading.value) return;
    updateOpen.value = false;
    downloadPercent.value = 0;
  }

  async function installUpdate() {
    if (!pendingUpdate || downloading.value) return;
    downloading.value = true;
    downloadPercent.value = 0;
    try {
      let downloaded = 0;
      let contentLength = 0;
      await pendingUpdate.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            contentLength = Number(event.data.contentLength) || 0;
            downloaded = 0;
            downloadPercent.value = 0;
            break;
          case "Progress": {
            downloaded += Number(event.data.chunkLength) || 0;
            if (contentLength > 0) {
              downloadPercent.value = Math.min(
                99,
                Math.round((downloaded / contentLength) * 100),
              );
            }
            break;
          }
          case "Finished":
            downloadPercent.value = 100;
            break;
          default:
            break;
        }
      });
      message.success(t("updateInstalled"));
      await relaunch();
    } catch (error) {
      message.error(t("updateInstallFail", { error: String(error) }));
      downloading.value = false;
    }
  }

  function scheduleStartupCheck() {
    if (!isTauri || previewMode.value || IS_DEV) return;
    if (startupTimer) clearTimeout(startupTimer);
    startupTimer = setTimeout(() => {
      startupTimer = null;
      if (settings.value.general?.autoCheckUpdate === false) return;
      checkForUpdate({ silent: true });
    }, STARTUP_DELAY_MS);
  }

  onMounted(async () => {
    await refreshAppVersion();
  });

  onBeforeUnmount(() => {
    if (startupTimer) clearTimeout(startupTimer);
  });

  return {
    updateOpen,
    updateInfo,
    updateAvailable,
    downloading,
    downloadPercent,
    checking,
    appVersion,
    checkForUpdate,
    dismissUpdate,
    installUpdate,
    scheduleStartupCheck,
  };
}

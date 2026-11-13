/**
 * 设置状态：外观、语言、GitLab group 地址、Access Token、保存。
 */

import { ref } from "vue";
import { message } from "antdv-next";
import { THEME_OPTIONS, applyThemeToDocument } from "../themes/index.js";
import { createTranslator, LOCALE_OPTIONS } from "../i18n/index.js";
import { isTauri } from "../api/tauri.js";
import { saveRuntimeSettings } from "../api/settings.js";

export const SETTINGS_KEY = "central-devops-kit.settings";
export const previewMode = ref(!isTauri);

export function emptySettings() {
  return {
    general: {
      theme: "teal",
      locale: "zh-CN",
      launchAtLogin: false,
      closeToTray: true,
      autoCheckUpdate: true,
    },
    gitlab: {
      accessToken: "",
      groupUrl: "",
    },
  };
}

export function normalizeSettings(raw) {
  const base = emptySettings();
  if (!raw || typeof raw !== "object") return base;
  const themeId = THEME_OPTIONS.some((x) => x.id === raw.general?.theme)
    ? raw.general.theme
    : "teal";
  const localeId = LOCALE_OPTIONS.some((x) => x.id === raw.general?.locale)
    ? raw.general.locale
    : "zh-CN";
  return {
    general: {
      theme: themeId,
      locale: localeId,
      launchAtLogin: !!raw.general?.launchAtLogin,
      closeToTray: raw.general?.closeToTray !== false,
      autoCheckUpdate: raw.general?.autoCheckUpdate !== false,
    },
    gitlab: {
      accessToken:
        typeof raw.gitlab?.accessToken === "string" ? raw.gitlab.accessToken : "",
      groupUrl: typeof raw.gitlab?.groupUrl === "string" ? raw.gitlab.groupUrl : "",
    },
  };
}

export const settings = ref(emptySettings());
export const saving = ref(false);
export const settingsPage = ref(false);
export const settingsLoading = ref(false);

export const locale = ref("zh-CN");
export const colorThemeId = ref("teal");

let settingsSaveTimer = null;

export function applyAppearance(next) {
  const g = next?.general || emptySettings().general;
  colorThemeId.value = g.theme || "teal";
  locale.value = g.locale || "zh-CN";
  applyThemeToDocument(colorThemeId.value);
  document.documentElement.lang = locale.value;
}

export function loadPreviewSettings() {
  try {
    const raw = localStorage.getItem(SETTINGS_KEY);
    if (!raw) {
      const created = emptySettings();
      localStorage.setItem(SETTINGS_KEY, JSON.stringify(created));
      return created;
    }
    return normalizeSettings(JSON.parse(raw));
  } catch {
    return emptySettings();
  }
}

export async function persistSettings(next = settings.value, options = {}) {
  const { notify = false } = options;
  const t = createTranslator(locale);
  saving.value = true;
  try {
    const payload = normalizeSettings(next);
    settings.value = payload;
    applyAppearance(payload);
    if (previewMode.value) {
      localStorage.setItem(SETTINGS_KEY, JSON.stringify(payload));
    } else {
      await saveRuntimeSettings(payload);
    }
    if (notify) message.success(t("settingsSaveOk"));
  } catch (error) {
    if (notify) {
      message.error(t("settingsSaveFail", { error: String(error) }));
    }
    throw error;
  } finally {
    saving.value = false;
  }
}

export function schedulePersistSettings() {
  clearTimeout(settingsSaveTimer);
  settingsSaveTimer = setTimeout(async () => {
    try {
      await persistSettings(settings.value, { notify: true });
    } catch {
      /* persistSettings already surfaces the error */
    }
  }, 280);
}

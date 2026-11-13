/**
 * 设置页：加载、保存、外观。
 */

import { message } from "antdv-next";
import { applyThemeToDocument } from "../../themes/index.js";
import {
  settings,
  settingsPage,
  settingsLoading,
  locale,
  colorThemeId,
  previewMode,
  persistSettings,
  schedulePersistSettings,
  applyAppearance,
  loadPreviewSettings,
  normalizeSettings,
} from "../../stores/settings.js";
import {
  loadRuntimeSettings,
  runtimeSettingsExists,
  initializeRuntimeSettings,
} from "../../api/settings.js";
import { goHome, goSettings } from "../../stores/appNav.js";

export function useSettings() {
  async function load() {
    settingsLoading.value = true;
    try {
      if (previewMode.value) {
        settings.value = loadPreviewSettings();
      } else {
        settings.value = normalizeSettings(await loadRuntimeSettings());
      }
      applyAppearance(settings.value);
    } catch (error) {
      message.error(String(error));
    } finally {
      settingsLoading.value = false;
    }
  }

  async function bootstrap() {
    settingsLoading.value = true;
    try {
      if (previewMode.value) {
        settings.value = loadPreviewSettings();
      } else {
        const exists = await runtimeSettingsExists();
        const raw = exists
          ? await loadRuntimeSettings()
          : await initializeRuntimeSettings();
        settings.value = normalizeSettings(raw);
      }
      applyAppearance(settings.value);
    } catch (error) {
      message.error(String(error));
    } finally {
      settingsLoading.value = false;
    }
  }

  function openSettings() {
    goSettings();
  }

  function closeSettings() {
    goHome();
  }

  function toggleSettings() {
    if (settingsPage.value) goHome();
    else goSettings();
  }

  function onThemeChange(id) {
    settings.value = {
      ...settings.value,
      general: { ...settings.value.general, theme: id },
    };
    applyThemeToDocument(id);
    colorThemeId.value = id;
    schedulePersistSettings();
  }

  function onLocaleChange(id) {
    settings.value = {
      ...settings.value,
      general: { ...settings.value.general, locale: id },
    };
    locale.value = id;
    document.documentElement.lang = id;
    schedulePersistSettings();
  }

  function onLaunchAtLoginChange(checked) {
    settings.value = {
      ...settings.value,
      general: { ...settings.value.general, launchAtLogin: !!checked },
    };
    schedulePersistSettings();
  }

  function onCloseToTrayChange(checked) {
    settings.value = {
      ...settings.value,
      general: { ...settings.value.general, closeToTray: !!checked },
    };
    schedulePersistSettings();
  }

  function onAutoCheckUpdateChange(checked) {
    settings.value = {
      ...settings.value,
      general: { ...settings.value.general, autoCheckUpdate: !!checked },
    };
    schedulePersistSettings();
  }

  return {
    settings,
    settingsPage,
    settingsLoading,
    load,
    bootstrap,
    persistSettings,
    openSettings,
    closeSettings,
    toggleSettings,
    onThemeChange,
    onLocaleChange,
    onLaunchAtLoginChange,
    onCloseToTrayChange,
    onAutoCheckUpdateChange,
  };
}

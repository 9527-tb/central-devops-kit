/**
 * 左侧图标轨的当前页。各页互斥。
 * 新功能：在 AppRail 增加一项，这里补一个 goXxx，并在 App.vue 挂上对应视图。
 */

import { computed, ref } from "vue";
import { settingsPage } from "./settings.js";

/** @typedef {"home" | "gitlab" | "ports" | "settings"} AppMode */

/** 启动时打开左侧第一个功能。 */
export const gitlabPage = ref(true);
export const portsPage = ref(false);

export const appMode = computed(() => {
  if (settingsPage.value) return "settings";
  if (portsPage.value) return "ports";
  if (gitlabPage.value) return "gitlab";
  return "home";
});

export function goHome() {
  settingsPage.value = false;
  gitlabPage.value = false;
  portsPage.value = false;
}

export function goGitlab() {
  settingsPage.value = false;
  portsPage.value = false;
  gitlabPage.value = true;
}

export function goPorts() {
  settingsPage.value = false;
  gitlabPage.value = false;
  portsPage.value = true;
}

export function goSettings() {
  gitlabPage.value = false;
  portsPage.value = false;
  settingsPage.value = true;
}

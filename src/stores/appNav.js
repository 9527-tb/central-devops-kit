/**
 * 左侧图标轨的当前页。各页互斥。
 * 新功能：在 AppRail 增加一项，这里补一个 goXxx，并在 App.vue 挂上对应视图。
 */

import { computed, ref } from "vue";
import { settingsPage } from "./settings.js";

/** @typedef {"home" | "gitlab" | "pipeline" | "gcp" | "ports" | "settings"} AppMode */

/** 启动时打开左侧第一个功能。 */
export const gitlabPage = ref(false);
export const pipelinePage = ref(false);
export const gcpPage = ref(false);
export const portsPage = ref(true);

export const appMode = computed(() => {
  if (settingsPage.value) return "settings";
  if (portsPage.value) return "ports";
  if (gcpPage.value) return "gcp";
  if (pipelinePage.value) return "pipeline";
  if (gitlabPage.value) return "gitlab";
  return "home";
});

export function goHome() {
  settingsPage.value = false;
  gitlabPage.value = false;
  pipelinePage.value = false;
  gcpPage.value = false;
  portsPage.value = false;
}

export function goGitlab() {
  settingsPage.value = false;
  pipelinePage.value = false;
  gcpPage.value = false;
  portsPage.value = false;
  gitlabPage.value = true;
}

export function goPipeline() {
  settingsPage.value = false;
  gitlabPage.value = false;
  gcpPage.value = false;
  portsPage.value = false;
  pipelinePage.value = true;
}

export function goGcp() {
  settingsPage.value = false;
  gitlabPage.value = false;
  pipelinePage.value = false;
  portsPage.value = false;
  gcpPage.value = true;
}

export function goPorts() {
  settingsPage.value = false;
  gitlabPage.value = false;
  pipelinePage.value = false;
  gcpPage.value = false;
  portsPage.value = true;
}

export function goSettings() {
  gitlabPage.value = false;
  pipelinePage.value = false;
  gcpPage.value = false;
  portsPage.value = false;
  settingsPage.value = true;
}

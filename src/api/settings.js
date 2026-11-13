/**
 * 运行时设置 IPC。
 */

import { invokeSafe } from "./tauri.js";

export async function runtimeSettingsExists() {
  return invokeSafe("runtime_settings_exists");
}

export async function loadRuntimeSettings() {
  return invokeSafe("load_runtime_settings");
}

export async function saveRuntimeSettings(settings) {
  return invokeSafe("save_runtime_settings", { settings });
}

export async function initializeRuntimeSettings() {
  return invokeSafe("initialize_runtime_settings");
}

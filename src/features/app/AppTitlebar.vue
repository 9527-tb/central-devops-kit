<script setup>
import { computed, inject, onMounted, onBeforeUnmount, ref } from "vue";
import { CloudDownloadOutlined } from "@antdv-next/icons";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { isMac, isLinux, useCustomWindowControls } from "../../shared/platform.js";
import { isTauri } from "../../api/tauri.js";
import WindowControls from "./WindowControls.vue";

defineProps({
  t: { type: Function, required: true },
});

const appUpdate = inject("appUpdate", null);
const updateAvailable = computed(() => !!appUpdate?.updateAvailable?.value);
const updateChecking = computed(() => !!appUpdate?.checking?.value);
const appVersion = computed(() => appUpdate?.appVersion?.value || "—");

const showWinControls = useCustomWindowControls;
const controlsVariant = isLinux ? "linux" : "windows";
const macTrafficInset = ref(isMac);

function onCheckUpdate() {
  appUpdate?.checkForUpdate?.({ silent: false });
}

let unlistenResize;
let unlistenScale;
let lastFullscreen = null;

async function refreshMacTrafficInset() {
  if (!isMac || !isTauri) {
    macTrafficInset.value = false;
    return;
  }
  try {
    const fullscreen = await getCurrentWindow().isFullscreen();
    if (fullscreen === lastFullscreen) return;
    lastFullscreen = fullscreen;
    macTrafficInset.value = !fullscreen;
  } catch {
    macTrafficInset.value = true;
  }
}

onMounted(async () => {
  await refreshMacTrafficInset();
  if (!isTauri || !isMac) return;
  try {
    const win = getCurrentWindow();
    unlistenResize = await win.onResized(() => {
      refreshMacTrafficInset();
    });
    unlistenScale = await win.onScaleFactorChanged(() => {
      refreshMacTrafficInset();
    });
  } catch {
    /* ignore */
  }
});

onBeforeUnmount(() => {
  if (typeof unlistenResize === "function") unlistenResize();
  if (typeof unlistenScale === "function") unlistenScale();
});
</script>

<template>
  <header
    class="app-titlebar"
    :class="{
      'is-mac': isMac,
      'is-mac-traffic-inset': isMac && macTrafficInset,
      'is-win-controls': showWinControls,
    }"
  >
    <div class="app-titlebar-drag" data-tauri-drag-region />

    <div class="app-titlebar-inner">
      <div class="app-titlebar-center">
        <span class="app-title">{{ t("appName") }}</span>
      </div>

      <div class="app-titlebar-right">
        <div class="app-titlebar-actions">
          <a-badge :dot="updateAvailable" :offset="[-4, 4]">
            <button
              type="button"
              class="title-icon-btn"
              :class="{ loading: updateChecking }"
              :title="`${t('checkForUpdate')} · v${appVersion}`"
              :disabled="updateChecking"
              @click="onCheckUpdate"
            >
              <CloudDownloadOutlined />
            </button>
          </a-badge>
        </div>
        <WindowControls v-if="showWinControls" :variant="controlsVariant" />
      </div>
    </div>
  </header>
</template>

<style scoped>
.app-titlebar {
  --titlebar-h: 34px;
  --titlebar-control-h: 26px;
  position: relative;
  z-index: 60;
  flex: none;
  height: var(--titlebar-h);
  box-sizing: border-box;
  display: flex;
  align-items: center;
  background: color-mix(in srgb, var(--panel) 92%, transparent);
  box-shadow: inset 0 -1px 0 color-mix(in srgb, var(--line) 80%, white);
  backdrop-filter: blur(12px);
  user-select: none;
}

.app-titlebar.is-mac {
  --titlebar-h: 38px;
  padding-left: 1px;
}
.app-titlebar.is-mac.is-mac-traffic-inset {
  padding-left: 78px;
}

.app-titlebar-drag {
  position: absolute;
  inset: 0;
  z-index: 0;
}

.app-titlebar-inner {
  position: relative;
  z-index: 1;
  flex: 1;
  min-width: 0;
  height: 100%;
  display: grid;
  grid-template-columns: 1fr auto 1fr;
  align-items: center;
  column-gap: 8px;
  padding: 0 8px;
  pointer-events: none;
  box-sizing: border-box;
}

.app-titlebar.is-win-controls .app-titlebar-inner {
  padding-right: 0;
}

.app-titlebar-right,
.app-titlebar-actions,
.app-titlebar-center,
.title-icon-btn {
  pointer-events: auto;
  -webkit-app-region: no-drag;
  app-region: no-drag;
}

.app-titlebar-center {
  grid-column: 2;
  justify-self: center;
  z-index: 2;
  display: flex;
  align-items: center;
  justify-content: center;
  max-width: min(480px, 52vw);
  height: var(--titlebar-control-h);
}

.app-title {
  font-size: 12px;
  font-weight: 650;
  color: var(--ink-soft);
  letter-spacing: 0.01em;
}

.app-titlebar-right {
  grid-column: 3;
  justify-self: end;
  display: flex;
  align-items: center;
  gap: 2px;
  min-width: 0;
  height: var(--titlebar-control-h);
  z-index: 2;
}

.app-titlebar.is-win-controls .app-titlebar-right {
  align-self: stretch;
  height: 100%;
}

.app-titlebar-actions {
  display: flex;
  align-items: center;
  gap: 0;
  height: var(--titlebar-control-h);
  padding-right: 2px;
}

.app-titlebar-actions :deep(.ant-badge) {
  display: inline-flex;
  align-items: center;
  height: 100%;
  line-height: 1;
}

.title-icon-btn {
  width: 26px;
  height: 26px;
  margin: 0;
  padding: 0;
  border: 0;
  border-radius: var(--radius, 3px);
  background: transparent;
  color: var(--ink-soft);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  font-size: 14px;
  line-height: 1;
}
.title-icon-btn:hover:not(:disabled) {
  background: var(--teal-soft);
  color: var(--teal);
}
.title-icon-btn.active {
  color: var(--teal);
  background: var(--teal-soft);
}
.title-icon-btn:disabled {
  opacity: 0.55;
  cursor: default;
}
.title-icon-btn.loading {
  opacity: 0.7;
}
</style>

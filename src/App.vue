<script setup>
import { computed, onMounted, onBeforeUnmount, provide } from "vue";
import { getTheme, BORDER_RADIUS_PX } from "./themes/index.js";
import { locale, colorThemeId } from "./stores/settings.js";
import { appMode } from "./stores/appNav.js";
import { useSettings } from "./features/settings/useSettings.js";
import { useAppUpdate } from "./features/app/useAppUpdate.js";
import AppRail from "./features/app/AppRail.vue";
import UpdateDialog from "./features/app/UpdateDialog.vue";
import SettingsView from "./views/SettingsView.vue";
import HomeView from "./views/HomeView.vue";
import GitlabVariablesView from "./views/GitlabVariablesView.vue";
import { createTranslator } from "./i18n/index.js";

const t = createTranslator(locale);
const { bootstrap } = useSettings();
const mode = computed(() => appMode.value);
const {
  updateOpen,
  updateInfo,
  updateAvailable,
  downloading,
  downloadPercent,
  checking: updateChecking,
  appVersion,
  checkForUpdate,
  dismissUpdate,
  installUpdate,
  scheduleStartupCheck,
} = useAppUpdate();

provide("appUpdate", {
  checkForUpdate,
  checking: updateChecking,
  appVersion,
  updateAvailable,
});

const theme = computed(() => {
  const pack = getTheme(colorThemeId.value);
  return {
    token: {
      colorPrimary: pack.primary,
      borderRadius: BORDER_RADIUS_PX,
      fontSize: 13,
      fontFamily: '"IBM Plex Sans", "Noto Sans SC", system-ui, sans-serif',
    },
  };
});

function suppressSystemContextMenu(event) {
  event.preventDefault();
}

onMounted(async () => {
  document.addEventListener("contextmenu", suppressSystemContextMenu);
  await bootstrap();
  scheduleStartupCheck();
});

onBeforeUnmount(() => {
  document.removeEventListener("contextmenu", suppressSystemContextMenu);
});
</script>

<template>
  <a-config-provider :theme="theme">
    <div class="app-shell">
      <div class="app-body">
        <AppRail :t="t" />
        <div class="app-main">
          <div v-show="mode === 'home'" class="app-view">
            <HomeView />
          </div>
          <div v-if="mode === 'gitlab'" class="app-view">
            <GitlabVariablesView />
          </div>
          <div v-if="mode === 'settings'" class="app-view">
            <SettingsView />
          </div>
        </div>
      </div>
      <UpdateDialog
        v-model:open="updateOpen"
        :version="updateInfo?.version || ''"
        :notes="updateInfo?.notes || ''"
        :downloading="downloading"
        :download-percent="downloadPercent"
        @later="dismissUpdate"
        @install="installUpdate"
      />
    </div>
  </a-config-provider>
</template>

<style>
@import url("https://fonts.googleapis.com/css2?family=IBM+Plex+Sans:wght@400;500;600;700&family=Noto+Sans+SC:wght@400;500;700&display=swap");

:root {
  --radius: 3px;
  --bg: #eef2f1;
  --panel: #ffffff;
  --ink: #15201e;
  --ink-strong: #24322e;
  --ink-soft: #30433e;
  --muted: #6b7a76;
  --line: #d7e0dc;
  --line-soft: #eef3f0;
  --line-hover: #b9cdc5;
  --teal: #0f766e;
  --teal-soft: #e8f5f2;
  --teal-mid: #1a8f84;
  --bg-wash-a: #d7ebe6;
  --bg-wash-b: #e7efe8;
  --surface-tint: #e8efec;
  --surface-muted: #f5f7f6;
  --surface-hover: #f3f7f5;
  --surface-selected: #e8f4ef;
  color-scheme: light;
  font-family: "IBM Plex Sans", "Noto Sans SC", system-ui, sans-serif;
  font-size: 13px;
  background: var(--bg);
  color: var(--ink);
}

* { box-sizing: border-box; }
html, body, #app { margin: 0; min-height: 100%; height: 100%; }
body {
  background:
    radial-gradient(900px 420px at 8% -12%, var(--bg-wash-a) 0%, transparent 55%),
    radial-gradient(700px 360px at 100% 0%, var(--bg-wash-b) 0%, transparent 50%),
    var(--bg);
  font-size: 13px;
  color: var(--ink);
}

html,
body,
#app,
.app-shell {
  -webkit-user-select: none;
  user-select: none;
}

input,
textarea,
select,
[contenteditable="true"],
.ant-input,
.ant-input-affix-wrapper,
.ant-select-selection-search-input {
  -webkit-user-select: text;
  user-select: text;
}

.app-shell {
  height: 100%;
  min-height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background:
    radial-gradient(circle at top left, var(--bg-wash-a), transparent 34%),
    radial-gradient(circle at top right, var(--bg-wash-b), transparent 28%),
    linear-gradient(180deg, var(--bg), var(--surface-muted));
}

.app-body {
  flex: 1;
  min-height: 0;
  display: flex;
  overflow: hidden;
}

.app-main {
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  position: relative;
}

.app-view {
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.app-view > * {
  flex: 1;
  min-width: 0;
  min-height: 0;
}

.app-main > .app-view[style*="display: none"] {
  display: none !important;
  flex: none !important;
}
</style>

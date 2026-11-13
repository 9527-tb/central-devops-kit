<!--
  左侧图标轨。上方放功能入口，设置固定在底部。
  加功能时往 items 里追加一项（不要设 bottom）。
-->
<script setup>
import { computed } from "vue";
import { GitlabOutlined, SettingOutlined } from "@antdv-next/icons";
import { appMode, goGitlab, goHome, goSettings } from "../../stores/appNav.js";

defineProps({
  t: { type: Function, required: true },
});

const mode = computed(() => appMode.value);

const items = computed(() => [
  {
    id: "gitlab",
    title: "gitlabVariables",
    icon: GitlabOutlined,
    onClick: () => (mode.value === "gitlab" ? goHome() : goGitlab()),
  },
  {
    id: "settings",
    title: "settings",
    icon: SettingOutlined,
    bottom: true,
    onClick: () => (mode.value === "settings" ? goHome() : goSettings()),
  },
]);

const topItems = computed(() => items.value.filter((item) => !item.bottom));
const bottomItems = computed(() => items.value.filter((item) => item.bottom));
</script>

<template>
  <nav class="app-rail" aria-label="main">
    <button
      v-for="item in topItems"
      :key="item.id"
      type="button"
      class="app-rail-btn"
      :class="{ active: mode === item.id }"
      :title="t(item.title)"
      @click="item.onClick"
    >
      <component :is="item.icon" class="app-rail-icon" />
    </button>
    <div class="app-rail-gap" />
    <button
      v-for="item in bottomItems"
      :key="item.id"
      type="button"
      class="app-rail-btn"
      :class="{ active: mode === item.id }"
      :title="t(item.title)"
      @click="item.onClick"
    >
      <component :is="item.icon" class="app-rail-icon" />
    </button>
  </nav>
</template>

<style scoped>
.app-rail {
  flex: none;
  width: 44px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  padding: 4px 0 8px 0;
  border-right: 1px solid var(--line);
  background: color-mix(in srgb, var(--panel) 88%, white);
  z-index: 40;
}
.app-rail-gap {
  flex: 1;
}
.app-rail-btn {
  width: 34px;
  height: 34px;
  margin: 0;
  padding: 0;
  border: 0;
  border-radius: var(--radius, 3px);
  background: transparent;
  color: var(--muted);
  display: grid;
  place-items: center;
  cursor: pointer;
  position: relative;
  transition: color 0.15s, background 0.15s;
}
.app-rail-btn:hover {
  color: var(--ink);
  background: var(--line-soft);
}
.app-rail-btn.active {
  color: var(--teal);
  background: var(--teal-soft);
}
.app-rail-btn.active::before {
  content: "";
  position: absolute;
  left: -5px;
  width: 3px;
  height: 14px;
  border-radius: 0 2px 2px 0;
  background: var(--teal);
}
.app-rail-icon {
  font-size: 16px;
  line-height: 1;
  display: inline-flex;
}
.app-rail-icon :deep(svg) {
  width: 16px;
  height: 16px;
  display: block;
}
</style>

<script setup>
import { settingsLoading } from "../stores/settings.js";
import SettingsGeneral from "../features/settings/SettingsGeneral.vue";
import "../styles/settings-layout.css";
</script>

<template>
  <a-config-provider component-size="small">
    <div class="settings-page">
      <a-spin :spinning="settingsLoading" class="settings-main-spin">
        <main class="settings-main">
          <SettingsGeneral />
        </main>
      </a-spin>
    </div>
  </a-config-provider>
</template>

<style>
.theme-grid {
  display: grid;
  grid-template-columns: repeat(6, minmax(0, 1fr));
  gap: 8px;
}
@media (max-width: 900px) {
  .theme-grid { grid-template-columns: repeat(3, minmax(0, 1fr)); }
}
.theme-swatch {
  -webkit-appearance: none;
  appearance: none;
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 8px;
  border-radius: var(--radius);
  border: 1px solid var(--line);
  background-color: var(--surface-muted);
  color: inherit;
  font: inherit;
  cursor: pointer;
  text-align: left;
}
.theme-swatch:hover { border-color: var(--line-hover); }
.theme-swatch.active {
  border-color: var(--teal);
  box-shadow: 0 0 0 1px var(--teal);
  background-color: var(--teal-soft);
}
.theme-swatch-bar {
  display: block;
  width: 100%;
  height: 22px;
  min-height: 22px;
  flex-shrink: 0;
  border-radius: var(--radius);
  background-color: #999;
}
.theme-swatch--teal .theme-swatch-bar { background-color: #0f766e; }
.theme-swatch--ocean .theme-swatch-bar { background-color: #0c6a8a; }
.theme-swatch--forest .theme-swatch-bar { background-color: #3d6b4f; }
.theme-swatch--slate .theme-swatch-bar { background-color: #4a5d6a; }
.theme-swatch--amber .theme-swatch-bar { background-color: #b45309; }
.theme-swatch--rose .theme-swatch-bar { background-color: #9f1239; }
.theme-swatch-name {
  font-size: 11px;
  font-weight: 700;
  color: var(--ink-soft);
}
.locale-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.locale-chip {
  height: 30px;
  padding: 0 12px;
  border-radius: var(--radius);
  border: 1px solid var(--line);
  background: var(--surface-muted);
  font-size: 12px;
  font-weight: 600;
  color: var(--ink-soft);
  cursor: pointer;
}
.locale-chip:hover { border-color: var(--line-hover); }
.locale-chip.active {
  border-color: var(--teal);
  background: var(--teal-soft);
  color: var(--teal);
}
</style>

<style scoped>
.settings-page {
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
</style>

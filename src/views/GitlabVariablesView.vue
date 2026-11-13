<!--
  GitLab 变量页。首次打开时配置 group 地址和 Access Token，齐全后再读取变量。
  检查和读取时遮住标题以下的内容，避免加载时继续操作。
-->
<script setup>
import { computed } from "vue";
import { message } from "antdv-next";
import { EditOutlined, ReloadOutlined, SettingOutlined } from "@antdv-next/icons";
import { createTranslator } from "../i18n/index.js";
import { locale } from "../stores/settings.js";
import { useGitlabVariables } from "../features/gitlab/useGitlabVariables.js";
import "../styles/settings-layout.css";

const t = createTranslator(locale);
const {
  loading,
  status,
  messageText,
  tokenInput,
  groupUrlInput,
  settingsOpen,
  rows,
  refresh,
  openSettings,
  closeSettings,
  saveGitlabSettings,
} = useGitlabVariables();

const STAGING = "connect-staging";
const PRODUCTION = "connect-production";

const needsConfig = computed(() => status.value === "missing" || status.value === "invalid");
const setupMessage = computed(() =>
  status.value === "invalid" ? t("gitlabTokenInvalid") : t("gitlabTokenMissing"),
);

const compareRows = computed(() => {
  const staging = new Map();
  const production = new Map();
  for (const row of rows.value) {
    const scope = String(row.environmentScope || "").trim();
    if (scope === STAGING) staging.set(row.key, row);
    else if (scope === PRODUCTION) production.set(row.key, row);
  }
  const keys = [...new Set([...staging.keys(), ...production.keys()])].sort((a, b) =>
    a.localeCompare(b, undefined, { numeric: true, sensitivity: "base" }),
  );
  return keys.map((key) => {
    const left = staging.get(key) || null;
    const right = production.get(key) || null;
    let state = "same";
    if (!left || !right) state = "gap";
    else if (String(left.value || "") !== String(right.value || "")) state = "changed";
    return { key, left, right, state };
  });
});

function hasValue(record) {
  return Boolean(record.value);
}

function valueText(record) {
  if (record.value) return record.value;
  if (record.hidden || record.isMasked) return t("gitlabValueHidden");
  return "—";
}

async function copyText(text, okKey) {
  try {
    await navigator.clipboard.writeText(String(text ?? ""));
    message.success(t(okKey));
  } catch {
    message.error(t("gitlabCopyFail"));
  }
}
</script>

<template>
    <div class="gitlab-page">
      <main class="settings-main">
          <section class="settings-panel">
            <div class="gitlab-head">
              <h2>{{ t("gitlabVariables") }}</h2>
              <div class="gitlab-head-actions">
                <a-button class="gitlab-refresh" :loading="loading" @click="refresh({ reopenSettings: true })">
                  <template #icon><ReloadOutlined /></template>
                  {{ t("gitlabRefresh") }}
                </a-button>
                <a-button @click="openSettings">
                  <template #icon><SettingOutlined /></template>
                  {{ t("gitlabSettings") }}
                </a-button>
              </div>
            </div>
            <div class="gitlab-body">
              <div v-if="loading" class="gitlab-mask">
                <a-spin spinning :tip="t('gitlabLoading')" />
              </div>

            <p v-if="!loading && needsConfig" class="gitlab-need-setup">{{ setupMessage }}</p>

            <div v-else-if="!loading && status === 'failed'" class="cfg-group">
              <div class="cfg-group-b">
                <a-alert type="error" show-icon :message="messageText" />
                <div class="gitlab-actions">
                  <a-button type="primary" @click="refresh({ reopenSettings: true })">{{ t("gitlabRetry") }}</a-button>
                  <a-button @click="openSettings">{{ t("gitlabSettings") }}</a-button>
                </div>
              </div>
            </div>

            <div v-else-if="!loading && status === 'ready'" class="diff-wrap">
              <p v-if="!compareRows.length" class="settings-desc">{{ t("gitlabCompareEmpty") }}</p>
              <div v-else class="diff">
                <div class="diff-head">connect-staging</div>
                <div class="diff-head">connect-production</div>
                <template v-for="row in compareRows" :key="row.key">
                  <div class="diff-cell" :class="{ 'is-gap': !row.left, 'is-diff': row.state === 'changed' }">
                    <template v-if="row.left">
                      <div class="var-top">
                        <div class="var-name">
                          <code
                            class="var-key is-copy"
                            :title="t('gitlabCopyKey')"
                            @click="copyText(row.left.key, 'gitlabCopiedKey')"
                          >{{ row.left.key }}</code>
                          <div
                            v-if="row.left.isProtected || row.left.isMasked || row.left.expanded || row.left.variableType === 'file'"
                            class="var-tags"
                          >
                            <span v-if="row.left.isProtected" class="var-tag">{{ t("gitlabColProtected") }}</span>
                            <span v-if="row.left.isMasked" class="var-tag">{{ t("gitlabColMasked") }}</span>
                            <span v-if="row.left.expanded" class="var-tag">{{ t("gitlabColExpanded") }}</span>
                            <span v-if="row.left.variableType === 'file'" class="var-tag">{{ t("gitlabTypeFile") }}</span>
                          </div>
                        </div>
                        <a-button
                          class="var-edit"
                          size="small"
                          :title="t('gitlabEdit')"
                          :aria-label="t('gitlabEdit')"
                        >
                          <template #icon><EditOutlined /></template>
                        </a-button>
                      </div>
                      <p v-if="row.left.description" class="var-desc">{{ row.left.description }}</p>
                      <pre
                        class="var-value"
                        :class="{ muted: !hasValue(row.left), 'is-copy': hasValue(row.left) }"
                        :title="hasValue(row.left) ? t('gitlabCopyValue') : undefined"
                        @click="hasValue(row.left) && copyText(row.left.value, 'gitlabCopiedValue')"
                      >{{ valueText(row.left) }}</pre>
                    </template>
                    <p v-else class="diff-gap">{{ t("gitlabCompareMissing", { key: row.key }) }}</p>
                  </div>
                  <div class="diff-cell" :class="{ 'is-gap': !row.right, 'is-diff': row.state === 'changed' }">
                    <template v-if="row.right">
                      <div class="var-top">
                        <div class="var-name">
                          <code
                            class="var-key is-copy"
                            :title="t('gitlabCopyKey')"
                            @click="copyText(row.right.key, 'gitlabCopiedKey')"
                          >{{ row.right.key }}</code>
                          <div
                            v-if="row.right.isProtected || row.right.isMasked || row.right.expanded || row.right.variableType === 'file'"
                            class="var-tags"
                          >
                            <span v-if="row.right.isProtected" class="var-tag">{{ t("gitlabColProtected") }}</span>
                            <span v-if="row.right.isMasked" class="var-tag">{{ t("gitlabColMasked") }}</span>
                            <span v-if="row.right.expanded" class="var-tag">{{ t("gitlabColExpanded") }}</span>
                            <span v-if="row.right.variableType === 'file'" class="var-tag">{{ t("gitlabTypeFile") }}</span>
                          </div>
                        </div>
                        <a-button
                          class="var-edit"
                          size="small"
                          :title="t('gitlabEdit')"
                          :aria-label="t('gitlabEdit')"
                        >
                          <template #icon><EditOutlined /></template>
                        </a-button>
                      </div>
                      <p v-if="row.right.description" class="var-desc">{{ row.right.description }}</p>
                      <pre
                        class="var-value"
                        :class="{ muted: !hasValue(row.right), 'is-copy': hasValue(row.right) }"
                        :title="hasValue(row.right) ? t('gitlabCopyValue') : undefined"
                        @click="hasValue(row.right) && copyText(row.right.value, 'gitlabCopiedValue')"
                      >{{ valueText(row.right) }}</pre>
                    </template>
                    <p v-else class="diff-gap">{{ t("gitlabCompareMissing", { key: row.key }) }}</p>
                  </div>
                </template>
              </div>
            </div>
            </div>
          </section>
      </main>
      <a-modal
        :open="settingsOpen"
        :title="t('gitlabSettings')"
        :mask-closable="false"
        :footer="null"
        @cancel="closeSettings"
      >
        <div class="gitlab-settings-form">
          <label>
            <span>{{ t("gitlabGroupLabel") }}</span>
            <a-input
              v-model:value="groupUrlInput"
              spellcheck="false"
              :placeholder="t('gitlabGroupPlaceholder')"
              @press-enter="saveGitlabSettings"
            />
          </label>
          <label>
            <span>{{ t("gitlabTokenLabel") }}</span>
            <a-input-password
              v-model:value="tokenInput"
              autocomplete="new-password"
              spellcheck="false"
              :placeholder="t('gitlabTokenPlaceholder')"
              @press-enter="saveGitlabSettings"
            />
          </label>
        </div>
        <div class="gitlab-settings-actions">
          <a-button @click="closeSettings">{{ t("gitlabSettingsCancel") }}</a-button>
          <a-button type="primary" @click="saveGitlabSettings">{{ t("gitlabTokenSave") }}</a-button>
        </div>
      </a-modal>
    </div>
</template>

<style scoped>
.gitlab-page {
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--surface-muted);
}
.settings-main {
  flex: 1;
  display: flex;
  flex-direction: column;
}
.settings-panel {
  flex: 1 1 auto;
  display: flex;
  flex-direction: column;
  min-height: 100%;
}
.gitlab-body {
  position: relative;
  flex: 1 1 auto;
}
.gitlab-mask {
  position: absolute;
  inset: 0;
  z-index: 5;
  display: grid;
  place-items: center;
  background: color-mix(in srgb, var(--bg) 62%, transparent);
}
.gitlab-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 10px;
}
.gitlab-need-setup {
  margin: 0;
  color: var(--muted);
  font-size: 13px;
}
.gitlab-settings-form {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.gitlab-settings-form label {
  display: flex;
  flex-direction: column;
  gap: 6px;
  color: var(--ink-strong);
  font-size: 12px;
  font-weight: 650;
}
.gitlab-settings-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 16px;
}
.gitlab-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 12px;
}
.gitlab-head-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 0 0 auto;
}
.gitlab-head h2 {
  margin: 0;
  min-width: 0;
}
.gitlab-refresh {
  flex: 0 0 auto;
}
.diff-wrap {
  width: 100%;
  min-width: 0;
}
.diff {
  display: grid;
  width: 100%;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--panel);
  overflow: hidden;
}
.diff-head,
.diff-cell {
  width: 100%;
  min-width: 0;
  max-width: 100%;
  overflow: hidden;
}
.diff-head {
  position: sticky;
  top: 0;
  z-index: 2;
  padding: 8px 10px;
  background: var(--surface-tint);
  color: var(--ink-strong);
  font-size: 12px;
  font-weight: 700;
}
.diff-head:nth-child(even),
.diff-cell:nth-child(even) {
  border-left: 1px solid var(--line);
}
.diff-cell {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 8px 10px;
  border-top: 1px solid var(--line);
}
.diff-cell.is-gap {
  --gap-bg: #eceff1;
  align-items: center;
  justify-content: center;
  background: var(--gap-bg);
}
.diff-cell.is-diff {
  background: var(--surface-selected);
}
.diff-gap {
  width: 100%;
  margin: 0;
  color: color-mix(in srgb, var(--gap-bg), #3a4246 42%);
  font-size: 12px;
  font-weight: 500;
  line-height: 1.4;
  text-align: center;
}
.var-key.is-copy,
.var-value.is-copy {
  cursor: pointer;
}
.var-top,
.var-name,
.var-tags,
.var-desc,
.diff-gap {
  min-width: 0;
  max-width: 100%;
}
.var-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.var-name {
  display: flex;
  flex: 1 1 auto;
  flex-wrap: wrap;
  align-items: center;
  gap: 4px;
}
.var-key {
  flex: 0 1 auto;
  width: auto;
  min-width: 0;
  max-width: 100%;
  margin: 0;
  padding: 0;
  color: var(--ink-strong);
  font-size: 12px;
  font-weight: 650;
  line-height: 18px;
  word-break: break-all;
  user-select: text;
}
.var-tags {
  display: inline-flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 4px;
}
.var-tag {
  display: inline-flex;
  align-items: center;
  height: 16px;
  padding: 0 4px;
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--surface-muted);
  color: var(--ink-soft);
  font-size: 10px;
  font-weight: 600;
  line-height: 1;
}
.var-edit {
  flex: 0 0 auto;
}
.var-desc {
  margin: 4px 0 0;
  color: var(--muted);
  font-size: 11px;
  line-height: 1.45;
}
.var-value {
  flex: 1;
  width: 100%;
  min-width: 0;
  max-width: 100%;
  margin: 0;
  padding: 6px 8px;
  border-radius: var(--radius);
  background: var(--surface-muted);
  color: var(--ink);
  font: 12px/1.45 "IBM Plex Sans", ui-monospace, SFMono-Regular, monospace;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 96px;
  overflow: auto;
  user-select: text;
}
.var-value.muted {
  color: var(--muted);
  font-family: inherit;
}
</style>

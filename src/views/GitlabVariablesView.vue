<!--
  GitLab 变量页。group 地址和 Access Token 存在本机设置里。
  未设置时和点击设置时使用同一个弹窗。读取时遮住标题以下的内容。
-->
<script setup>
import { computed, reactive, ref } from "vue";
import { message } from "antdv-next";
import { DeleteOutlined, EditOutlined, PlusOutlined, ReloadOutlined, SettingOutlined } from "@antdv-next/icons";
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
  removeVariable,
  updateVariable,
  createVariable,
  createVariables,
} = useGitlabVariables();

const STAGING = "connect-staging";
const PRODUCTION = "connect-production";

const needsConfig = computed(() => status.value === "missing" || status.value === "invalid");
const setupMessage = computed(() =>
  status.value === "invalid" ? t("gitlabTokenInvalid") : t("gitlabTokenMissing"),
);
const DELETE_WORD = "DELETE";
const deleteTarget = ref(null);
const deleteConfirmText = ref("");
const deleting = ref(false);
const deleteReady = computed(() => deleteConfirmText.value.trim() === DELETE_WORD);

function openDelete(record) {
  deleteTarget.value = record;
  deleteConfirmText.value = "";
}

function closeDelete() {
  if (deleting.value) return;
  deleteTarget.value = null;
  deleteConfirmText.value = "";
}

const editTarget = ref(null);
const editValue = ref("");
const editing = ref(false);

function openEdit(record) {
  editTarget.value = record;
  editValue.value = String(record?.value ?? "");
}

function closeEdit() {
  if (editing.value) return;
  editTarget.value = null;
  editValue.value = "";
}

async function submitEdit() {
  if (!editTarget.value || editing.value) return;
  editing.value = true;
  const ok = await updateVariable(editTarget.value, editValue.value);
  editing.value = false;
  if (!ok) return;
  editTarget.value = null;
  editValue.value = "";
}

async function submitDelete() {
  if (!deleteTarget.value || deleting.value) return;
  if (!deleteReady.value) {
    message.warning(t("gitlabDeleteNeedConfirm"));
    return;
  }
  const record = deleteTarget.value;
  deleting.value = true;
  await removeVariable(record);
  deleteTarget.value = null;
  deleteConfirmText.value = "";
  deleting.value = false;
}

const CREATE_ENVS = [
  { id: "staging", labelKey: "gitlabEnvStaging", scope: STAGING, indo: false },
  { id: "production", labelKey: "gitlabEnvProduction", scope: PRODUCTION, indo: false },
  { id: "indo-staging", labelKey: "gitlabEnvIndoStaging", scope: STAGING, indo: true },
  { id: "indo-production", labelKey: "gitlabEnvIndoProduction", scope: PRODUCTION, indo: true },
];
const createOpen = ref(false);
const creating = ref(false);
const createStep = ref("edit");
const createForm = reactive(emptyCreateForm());

function emptyCreateForm() {
  return {
    scopes: [],
    key: "",
    value: "",
    description: "",
    variableType: "env_var",
    isProtected: false,
    isMasked: false,
    expanded: false,
  };
}

function plannedKey(base, indo) {
  const key = String(base || "").trim();
  if (!indo) return key;
  return key.endsWith("_INDO") ? key : `${key}_INDO`;
}

const createPlan = computed(() =>
  CREATE_ENVS.filter((env) => createForm.scopes.includes(env.id)).map((env) => ({
    id: env.id,
    label: t(env.labelKey),
    scope: env.scope,
    key: plannedKey(createForm.key, env.indo),
    value: String(createForm.value ?? ""),
  })),
);

function openCreate() {
  Object.assign(createForm, emptyCreateForm());
  createForm.scopes = [];
  createStep.value = "edit";
  createOpen.value = true;
}

function closeCreate() {
  if (creating.value) return;
  createOpen.value = false;
  createStep.value = "edit";
}

function reviewCreate() {
  const key = createForm.key.trim();
  if (!/^[A-Za-z0-9_]+$/.test(key)) {
    message.warning(t("gitlabCreateKeyInvalid"));
    return;
  }
  if (!createPlan.value.length) {
    message.warning(t("gitlabCreateNeedScope"));
    return;
  }
  createStep.value = "confirm";
}

async function confirmCreate() {
  if (creating.value || !createPlan.value.length) return;
  creating.value = true;
  const items = createPlan.value.map((item) => ({
    key: item.key,
    environmentScope: item.scope,
    value: createForm.value,
    description: createForm.description,
    variableType: createForm.variableType,
    isProtected: createForm.isProtected,
    isMasked: createForm.isMasked,
    expanded: createForm.expanded,
  }));
  const ok = await createVariables(items);
  creating.value = false;
  if (!ok) return;
  createOpen.value = false;
  createStep.value = "edit";
}

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
                <a-button @click="openCreate">
                  <template #icon><PlusOutlined /></template>
                  {{ t("gitlabCreate") }}
                </a-button>
                <span class="gitlab-head-split" aria-hidden="true" />
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
                <a-spin spinning size="large" :tip="t('gitlabLoading')" />
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
                        <div class="var-actions">
                          <a-button
                            class="var-edit"
                            size="small"
                            :title="t('gitlabEdit')"
                            :aria-label="t('gitlabEdit')"
                            @click="openEdit(row.left)"
                          >
                            <template #icon><EditOutlined /></template>
                          </a-button>
                          <a-button
                            class="var-edit"
                            size="small"
                            danger
                            :title="t('gitlabDelete')"
                            :aria-label="t('gitlabDelete')"
                            @click="openDelete(row.left)"
                          >
                            <template #icon><DeleteOutlined /></template>
                          </a-button>
                        </div>
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
                        <div class="var-actions">
                          <a-button
                            class="var-edit"
                            size="small"
                            :title="t('gitlabEdit')"
                            :aria-label="t('gitlabEdit')"
                            @click="openEdit(row.right)"
                          >
                            <template #icon><EditOutlined /></template>
                          </a-button>
                          <a-button
                            class="var-edit"
                            size="small"
                            danger
                            :title="t('gitlabDelete')"
                            :aria-label="t('gitlabDelete')"
                            @click="openDelete(row.right)"
                          >
                            <template #icon><DeleteOutlined /></template>
                          </a-button>
                        </div>
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
      <a-modal
        :open="!!deleteTarget"
        :title="t('gitlabDeleteTitle')"
        :mask-closable="false"
        :footer="null"
        @cancel="closeDelete"
      >
        <p class="gitlab-delete-hint">
          {{ t("gitlabDeleteHint", { key: deleteTarget?.key || "", scope: deleteTarget?.environmentScope || "" }) }}
        </p>
        <a-input
          v-model:value="deleteConfirmText"
          spellcheck="false"
          :disabled="deleting"
          :placeholder="DELETE_WORD"
          @keydown.enter.prevent
        />
        <div class="gitlab-settings-actions">
          <a-button :disabled="deleting" @click="closeDelete">{{ t("gitlabSettingsCancel") }}</a-button>
          <a-button danger type="primary" :disabled="!deleteReady" :loading="deleting" @click="submitDelete">
            {{ t("gitlabDelete") }}
          </a-button>
        </div>
      </a-modal>
      <a-modal
        :open="!!editTarget"
        :width="560"
        :title="t('gitlabEditTitle')"
        :mask-closable="false"
        :footer="null"
        @cancel="closeEdit"
      >
        <div class="gitlab-edit-meta">
          <div>
            <span>{{ t("gitlabCreateKey") }}</span>
            <b>{{ editTarget?.key }}</b>
          </div>
          <div>
            <span>{{ t("gitlabCreateScope") }}</span>
            <b>{{ editTarget?.environmentScope }}</b>
          </div>
        </div>
        <p v-if="editTarget && (editTarget.hidden || editTarget.isMasked) && !hasValue(editTarget)" class="gitlab-edit-hint">
          {{ t("gitlabEditHidden") }}
        </p>
        <label class="gitlab-edit-value">
          <span>{{ t("gitlabCreateValue") }}</span>
          <a-textarea v-model:value="editValue" :disabled="editing" :auto-size="{ minRows: 4, maxRows: 10 }" />
        </label>
        <div class="gitlab-settings-actions">
          <a-button :disabled="editing" @click="closeEdit">{{ t("gitlabSettingsCancel") }}</a-button>
          <a-button type="primary" :loading="editing" @click="submitEdit">{{ t("gitlabEditSave") }}</a-button>
        </div>
      </a-modal>
      <a-modal
        :open="createOpen"
        :width="640"
        :title="createStep === 'confirm' ? t('gitlabCreateConfirmTitle') : t('gitlabCreateTitle')"
        :mask-closable="false"
        :footer="null"
        @cancel="closeCreate"
      >
        <div v-if="createStep === 'edit'" class="gitlab-create-form">
          <div class="gitlab-create-field">
            <span>{{ t("gitlabCreateScope") }}</span>
            <a-checkbox-group
              v-model:value="createForm.scopes"
              class="gitlab-create-scopes"
              :disabled="creating"
            >
              <a-checkbox v-for="env in CREATE_ENVS" :key="env.id" :value="env.id">
                <span class="gitlab-create-scope-name">{{ t(env.labelKey) }}</span>
                <span v-if="env.indo" class="gitlab-create-scope-hint">{{ t("gitlabEnvIndoHint") }}</span>
              </a-checkbox>
            </a-checkbox-group>
          </div>
          <div class="gitlab-create-split">
            <label>
              <span>{{ t("gitlabCreateKey") }}</span>
              <a-input v-model:value="createForm.key" spellcheck="false" :disabled="creating" />
            </label>
            <label>
              <span>{{ t("gitlabCreateType") }}</span>
              <a-select
                v-model:value="createForm.variableType"
                :disabled="creating"
                :options="[
                  { value: 'env_var', label: t('gitlabTypeEnv') },
                  { value: 'file', label: t('gitlabTypeFile') },
                ]"
              />
            </label>
          </div>
          <label>
            <span>{{ t("gitlabCreateValue") }}</span>
            <a-textarea
              v-model:value="createForm.value"
              :disabled="creating"
              :auto-size="{ minRows: 4, maxRows: 8 }"
            />
          </label>
          <label>
            <span>{{ t("gitlabCreateDescription") }}</span>
            <a-input v-model:value="createForm.description" :disabled="creating" />
          </label>
          <div class="gitlab-create-flags">
            <label>
              <span>{{ t("gitlabColProtected") }}</span>
              <a-switch v-model:checked="createForm.isProtected" :disabled="creating" />
            </label>
            <label>
              <span>{{ t("gitlabColMasked") }}</span>
              <a-switch v-model:checked="createForm.isMasked" :disabled="creating" />
            </label>
            <label>
              <span>{{ t("gitlabColExpanded") }}</span>
              <a-switch v-model:checked="createForm.expanded" :disabled="creating" />
            </label>
          </div>
        </div>
        <div v-else class="gitlab-create-plan">
          <p>{{ t("gitlabCreatePlanHint", { count: createPlan.length }) }}</p>
          <div v-for="item in createPlan" :key="item.id" class="gitlab-create-plan-item">
            <div class="gitlab-create-plan-top">
              <b>{{ item.label }}</b>
              <code>{{ item.key }}</code>
            </div>
            <div class="gitlab-create-plan-scope">{{ item.scope }}</div>
            <pre>{{ item.value || "—" }}</pre>
          </div>
        </div>
        <div class="gitlab-settings-actions">
          <a-button v-if="createStep === 'confirm'" :disabled="creating" @click="createStep = 'edit'">
            {{ t("gitlabCreateBack") }}
          </a-button>
          <a-button v-else :disabled="creating" @click="closeCreate">{{ t("gitlabSettingsCancel") }}</a-button>
          <a-button v-if="createStep === 'edit'" type="primary" @click="reviewCreate">
            {{ t("gitlabCreateReview") }}
          </a-button>
          <a-button v-else type="primary" :loading="creating" @click="confirmCreate">
            {{ t("gitlabCreateConfirm") }}
          </a-button>
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
  margin: 28px 0 0;
  color: var(--muted);
  font-size: 12px;
  text-align: center;
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
.gitlab-edit-meta {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
  margin-bottom: 12px;
}
.gitlab-edit-meta div {
  min-width: 0;
  padding: 8px 10px;
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--surface-muted);
}
.gitlab-edit-meta span,
.gitlab-edit-value span {
  display: block;
  margin-bottom: 4px;
  color: var(--muted);
  font-size: 12px;
  font-weight: 650;
}
.gitlab-edit-meta b {
  display: block;
  color: var(--ink-strong);
  font-size: 13px;
  word-break: break-all;
}
.gitlab-edit-hint {
  margin: 0 0 12px;
  color: var(--muted);
  font-size: 12px;
}
.gitlab-edit-value {
  display: flex;
  flex-direction: column;
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
.gitlab-head h2 {
  margin: 0;
  min-width: 0;
}
.gitlab-head-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 0 0 auto;
}
.gitlab-head-split {
  width: 1px;
  align-self: stretch;
  margin: 6px 0;
  background: var(--line);
}
.gitlab-create-form {
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.gitlab-create-form > label,
.gitlab-create-field,
.gitlab-create-split label {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
  color: var(--ink-strong);
  font-size: 12px;
  font-weight: 650;
}
.gitlab-create-split {
  display: grid;
  grid-template-columns: minmax(0, 1.6fr) minmax(160px, 0.8fr);
  gap: 12px;
}
.gitlab-create-split :deep(.ant-select) {
  width: 100%;
}
.gitlab-create-scopes {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
  width: 100%;
}
.gitlab-create-scopes :deep(.ant-checkbox-wrapper) {
  margin: 0;
  padding: 10px 12px;
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--panel);
  align-items: flex-start;
}
.gitlab-create-scopes :deep(.ant-checkbox-wrapper-checked) {
  border-color: var(--teal-mid);
  background: var(--surface-selected);
}
.gitlab-create-scope-name,
.gitlab-create-scope-hint {
  display: block;
}
.gitlab-create-scope-hint {
  margin-top: 2px;
  color: var(--muted);
  font-size: 11px;
  font-weight: 500;
}
.gitlab-create-flags {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 8px;
}
.gitlab-create-flags label {
  display: flex;
  flex-direction: row;
  align-items: center;
  justify-content: space-between;
  padding: 8px 10px;
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--panel);
}
.gitlab-create-plan p {
  margin: 0 0 12px;
  color: var(--ink-soft);
}
.gitlab-create-plan-item {
  margin-top: 8px;
  padding: 10px 12px;
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--panel);
}
.gitlab-create-plan-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.gitlab-create-plan-top code {
  color: var(--ink-strong);
  font-weight: 650;
}
.gitlab-create-plan-scope {
  margin-top: 2px;
  color: var(--muted);
  font-size: 12px;
}
.gitlab-create-plan-item pre {
  margin: 8px 0 0;
  max-height: 72px;
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-all;
  color: var(--ink);
  font: 12px/1.45 "IBM Plex Sans", ui-monospace, SFMono-Regular, monospace;
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
.var-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: 0 0 auto;
}
.var-edit {
  flex: 0 0 auto;
}
.gitlab-delete-hint {
  margin: 0 0 12px;
  color: var(--ink-soft);
  font-size: 13px;
  line-height: 1.5;
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

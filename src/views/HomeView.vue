<script setup>
import { createTranslator } from "../i18n/index.js";
import { locale } from "../stores/settings.js";

const t = createTranslator(locale);
</script>

<template>
  <main class="home">
    <h1>{{ t("homeTitle") }}</h1>
    <p>{{ t("homeHint") }}</p>
  </main>
</template>

<style scoped>
.home {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 24px;
  text-align: center;
}
.home h1 {
  margin: 0;
  font-size: 18px;
  font-weight: 650;
  color: var(--ink-strong);
}
.home p {
  margin: 0;
  max-width: 420px;
  font-size: 12px;
  line-height: 1.5;
  color: var(--muted);
}
</style>

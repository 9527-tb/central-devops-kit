/**
 * 界面国际化。默认 zh-CN / zh-TW / en / ja。
 */

import zhCN from "./locales/zh-CN.js";
import zhTW from "./locales/zh-TW.js";
import en from "./locales/en.js";
import ja from "./locales/ja.js";

export const LOCALE_OPTIONS = [
  { id: "zh-CN", label: "简体中文" },
  { id: "zh-TW", label: "繁體中文" },
  { id: "en", label: "English" },
  { id: "ja", label: "日本語" },
];

const messages = {
  "zh-CN": zhCN,
  "zh-TW": zhTW,
  en,
  ja,
};

export function createTranslator(localeRef) {
  return (key, vars = {}) => {
    const locale =
      localeRef && typeof localeRef === "object" && "value" in localeRef
        ? localeRef.value
        : localeRef || "zh-CN";
    const table = messages[locale] || messages["zh-CN"];
    let text = table[key] ?? messages["zh-CN"][key] ?? key;
    for (const [k, v] of Object.entries(vars)) {
      text = text.replaceAll(`{${k}}`, String(v));
    }
    return text;
  };
}

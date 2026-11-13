import { createApp } from "vue";
import App from "./App.vue";
import Antd from "antdv-next";
import "antdv-next/dist/reset.css";
import { applyThemeToDocument } from "./themes/index.js";

applyThemeToDocument("teal");
createApp(App).use(Antd).mount("#app");

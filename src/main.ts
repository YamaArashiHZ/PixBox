import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import "./styles/global.css";

document.addEventListener(
  "contextmenu",
  (e) => {
    const t = e.target as HTMLElement | null;
    if (t?.closest("input, textarea, [contenteditable='true']")) return;
    e.preventDefault();
  },
  { capture: true },
);

document.addEventListener(
  "dragstart",
  (e) => {
    const t = e.target as HTMLElement | null;
    if (t?.closest("input, textarea, [contenteditable='true']")) return;
    e.preventDefault();
  },
  { capture: true },
);

const app = createApp(App);
app.use(createPinia());
app.mount("#app");

import { computed, reactive, toRefs, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { ThemeMode } from "../types";

export interface AppConfig {
  proxy_enabled: boolean;
  proxy: string;
  save_dir: string;
  compress_enabled: boolean;
  compress_separate: boolean;
  compress_dir: string;
  compress_max_mb: number;
  theme: ThemeMode;
}

const DEFAULT_CONFIG: AppConfig = {
  proxy_enabled: true,
  proxy: "http://127.0.0.1:7897",
  save_dir: "",
  compress_enabled: true,
  compress_separate: true,
  compress_dir: "",
  compress_max_mb: 6,
  theme: "dark",
};

let loaded = false;
const state = reactive<AppConfig>({ ...DEFAULT_CONFIG });

export async function loadAppConfig() {
  if (loaded) return;
  try {
    const raw = await invoke<string>("get_settings");
    const parsed = JSON.parse(raw);
    Object.assign(state, { ...DEFAULT_CONFIG, ...parsed });
  } catch {
    Object.assign(state, DEFAULT_CONFIG);
  } finally {
    loaded = true;
  }
}

async function persist() {
  if (!loaded) return;
  try {
    await invoke("set_settings", { settings: JSON.stringify({ ...state }) });
  } catch {
    /* ignore */
  }
}

watch(state, () => { void persist(); }, { deep: true });

export function useAppConfig() {
  function toggleTheme() {
    state.theme = state.theme === "dark" ? "light" : "dark";
  }

  const simpleRefs = toRefs(state);

  return {
    proxy_enabled: simpleRefs.proxy_enabled,
    proxy: simpleRefs.proxy,
    save_dir: simpleRefs.save_dir,
    compress_enabled: simpleRefs.compress_enabled,
    compress_separate: simpleRefs.compress_separate,
    compress_dir: simpleRefs.compress_dir,
    compress_max_mb: simpleRefs.compress_max_mb,
    theme: simpleRefs.theme,
    toggleTheme,
    loadAppConfig,
  };
}

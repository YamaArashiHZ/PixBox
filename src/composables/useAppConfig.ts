import { computed, reactive, toRefs, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { ThemeMode } from "../types";
import { getEffectiveDir } from "../utils/subdir";

export interface AppConfig {
  proxy_enabled: boolean;
  proxy: string;
  save_dir: string;
  use_subdir: boolean;
  subdir_pattern: string;
  compress_enabled: boolean;
  compress_separate: boolean;
  compress_dir: string;
  compress_use_subdir: boolean;
  compress_subdir_pattern: string;
  compress_max_mb: number;
  image_cache_limit_mb: number | null;
  theme: ThemeMode;
}

const DEFAULT_CONFIG: AppConfig = {
  proxy_enabled: true,
  proxy: "http://127.0.0.1:7897",
  save_dir: "",
  use_subdir: true,
  subdir_pattern: "%yy_%mm%dd_%HH%MM",
  compress_enabled: true,
  compress_separate: true,
  compress_dir: "",
  compress_use_subdir: true,
  compress_subdir_pattern: "%yy_%mm%dd_%HH%MM",
  compress_max_mb: 6,
  image_cache_limit_mb: 200,
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

// 强制 compress_separate 始终为 true（原图和压缩图不可同目录）
watch(
  () => state.compress_separate,
  (v) => {
    if (!v) state.compress_separate = true;
  },
  { immediate: true }
);

async function persist() {
  if (!loaded) return;
  try {
    await invoke("set_settings", { settings: JSON.stringify({ ...state }) });
  } catch {
    /* ignore */
  }
}

watch(state, () => { void persist(); }, { deep: true });

const hasPathConflict = computed(() => {
  if (!state.compress_enabled) return false;
  const now = new Date();
  const origDir = getEffectiveDir(state.save_dir, state.use_subdir, state.subdir_pattern, now);
  const compDir = getEffectiveDir(state.compress_dir, state.compress_use_subdir, state.compress_subdir_pattern, now);
  if (!origDir || !compDir) return false;
  return origDir.toLowerCase() === compDir.toLowerCase();
});

export function useAppConfig() {
  function toggleTheme() {
    state.theme = state.theme === "dark" ? "light" : "dark";
  }

  const simpleRefs = toRefs(state);

  return {
    proxy_enabled: simpleRefs.proxy_enabled,
    proxy: simpleRefs.proxy,
    save_dir: simpleRefs.save_dir,
    use_subdir: simpleRefs.use_subdir,
    subdir_pattern: simpleRefs.subdir_pattern,
    compress_enabled: simpleRefs.compress_enabled,
    compress_separate: simpleRefs.compress_separate,
    compress_dir: simpleRefs.compress_dir,
    compress_use_subdir: simpleRefs.compress_use_subdir,
    compress_subdir_pattern: simpleRefs.compress_subdir_pattern,
    compress_max_mb: simpleRefs.compress_max_mb,
    image_cache_limit_mb: simpleRefs.image_cache_limit_mb,
    theme: simpleRefs.theme,
    hasPathConflict,
    toggleTheme,
    loadAppConfig,
  };
}

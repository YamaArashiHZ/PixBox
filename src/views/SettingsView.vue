<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import {
  NCard,
  NSpace,
  NButton,
  NInput,
  NInputNumber,
  NSelect,
  NSwitch,
  NRadioGroup,
  NRadioButton,
  NTag,
  NIcon,
  NText,
  useMessage,
} from "naive-ui";
import { FolderOpenOutline, PersonOutline, LogOutOutline } from "@vicons/ionicons5";
import { open } from "@tauri-apps/plugin-dialog";
import { useAppConfig } from "../composables/useAppConfig";
import { useAuthStore } from "../stores/auth";
import { getCacheSize, clearCache, enforceCacheLimit } from "../api";
import { invoke } from "@tauri-apps/api/core";
import SubdirPatternField from "../components/SubdirPatternField.vue";
import { buildPathPreview } from "../utils/subdir";

const message = useMessage();
const {
  proxy_enabled,
  proxy,
  save_dir,
  use_subdir,
  subdir_pattern,
  compress_enabled,
  compress_separate,
  compress_dir,
  compress_use_subdir,
  compress_subdir_pattern,
  compress_max_mb,
  image_cache_limit_mb,
} = useAppConfig();
const auth = useAuthStore();

const testing = ref(false);
const testResult = ref<{ ok: boolean; text: string } | null>(null);
const cacheSize = ref<number | null>(null);
const clearingCache = ref(false);

const limitModeOptions = [
  { label: "无上限", value: "unlimited" },
  { label: "自定义", value: "custom" },
];

function onLimitModeChange(v: string) {
  image_cache_limit_mb.value = v === "unlimited" ? null : 200;
}

// compress_separate (boolean) 与单选控件 (string) 的桥接
const compressLocation = computed({
  get: () => (compress_separate.value ? "separate" : "same"),
  set: (v: string) => { compress_separate.value = v === "separate"; },
});

// 最终落盘路径实时预览（逻辑与后端 build_subdir 一致）
const savePreview = computed(() =>
  buildPathPreview(save_dir.value, use_subdir.value, subdir_pattern.value, "12345678_p0.jpg")
);
const compressPreview = computed(() => {
  if (!compress_enabled.value) return null;
  if (!compress_separate.value) return savePreview.value; // 与原图同目录
  return buildPathPreview(
    compress_dir.value,
    compress_use_subdir.value,
    compress_subdir_pattern.value,
    "12345678_p0.jpg"
  );
});

// 上限变化后立即执行一次 LRU 清理
watch(image_cache_limit_mb, () => { void enforceCacheLimit(); });

async function chooseDir(target: "save_dir" | "compress_dir") {
  const dir = await open({ directory: true, title: "选择目录" });
  if (dir) {
    if (target === "save_dir") save_dir.value = dir as string;
    else compress_dir.value = dir as string;
  }
}

async function testConnection() {
  testing.value = true;
  testResult.value = null;
  try {
    const ms = await invoke<number>("test_proxy", { proxy: proxy.value });
    testResult.value = { ok: true, text: `连接成功，延迟 ${ms}ms` };
  } catch (e) {
    testResult.value = { ok: false, text: `连接失败: ${e}` };
  } finally {
    testing.value = false;
  }
}

async function loadCacheSize() {
  try { cacheSize.value = await getCacheSize(); } catch { cacheSize.value = null; }
}

async function doClearCache() {
  clearingCache.value = true;
  try { await clearCache(); cacheSize.value = 0; } catch { /* ignore */ }
  finally { clearingCache.value = false; }
}

function fmtBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1048576).toFixed(1)} MB`
}

onMounted(() => { loadCacheSize(); })
</script>

<template>
  <div class="settings">
    <h1 class="page-title">设置</h1>
    <p class="page-subtitle">网络、保存路径与账号配置</p>

    <n-space vertical :size="16" style="width: 100%">
      <n-card title="网络" size="small">
        <div class="setting-row toggle-row">
          <n-text depth="3">启用代理</n-text>
          <n-switch v-model:value="proxy_enabled" />
        </div>
        <Transition name="collapse">
          <div v-if="proxy_enabled" class="collapsible-section">
            <div class="setting-row">
              <n-text depth="3" class="label">代理地址</n-text>
              <div class="input-row">
                <n-input v-model:value="proxy" placeholder="http://127.0.0.1:7897" />
                <n-button secondary @click="testConnection" :loading="testing">
                  测试连接
                </n-button>
              </div>
              <div v-if="testResult" class="test-result">
                <n-tag :type="testResult.ok ? 'success' : 'error'" size="small">
                  {{ testResult.text }}
                </n-tag>
              </div>
            </div>
          </div>
        </Transition>
      </n-card>

      <n-card title="原图保存" size="small">
        <n-space vertical :size="14" style="width: 100%">
          <div class="setting-row">
            <n-text depth="3" class="label">保存路径</n-text>
            <div class="input-row">
              <n-input :value="save_dir" placeholder="选择原图保存目录..." readonly />
              <n-button secondary @click="chooseDir('save_dir')">
                <n-icon :component="FolderOpenOutline" :size="16" />
              </n-button>
            </div>
          </div>

          <div class="setting-row toggle-row">
            <n-text depth="3">每次保存在独立文件夹中</n-text>
            <n-switch v-model:value="use_subdir" />
          </div>

          <Transition name="collapse">
            <div v-if="use_subdir" class="collapsible-section indent-section">
              <SubdirPatternField v-model="subdir_pattern" label="独立文件夹命名" />
            </div>
          </Transition>

          <n-text v-if="savePreview" depth="3" class="path-preview">
            预览：{{ savePreview }}
          </n-text>
        </n-space>
      </n-card>

      <n-card size="small">
        <template #header>压缩保存</template>
        <template #header-extra>
          <n-switch v-model:value="compress_enabled" />
        </template>

        <n-text v-if="!compress_enabled" depth="3" style="font-size: 13px">
          未启用压缩，仅保存原图
        </n-text>

        <n-space v-else vertical :size="14" style="width: 100%">
          <div class="setting-row">
            <n-text depth="3" class="label">压缩最大大小</n-text>
            <div class="input-row">
              <n-input-number
                v-model:value="compress_max_mb"
                :min="1"
                :max="50"
                style="width: 140px"
              >
                <template #suffix>MB</template>
              </n-input-number>
            </div>
          </div>

          <div class="setting-row">
            <n-text depth="3" class="label">压缩图保存位置</n-text>
            <n-radio-group v-model:value="compressLocation">
              <n-radio-button value="same">与原图同目录</n-radio-button>
              <n-radio-button value="separate">独立目录</n-radio-button>
            </n-radio-group>
          </div>

          <Transition name="collapse">
            <div v-if="compress_separate" class="collapsible-section indent-section">
              <div class="setting-row">
                <n-text depth="3" class="label">压缩保存路径</n-text>
                <div class="input-row">
                  <n-input :value="compress_dir" placeholder="选择压缩图保存目录..." readonly />
                  <n-button secondary @click="chooseDir('compress_dir')">
                    <n-icon :component="FolderOpenOutline" :size="16" />
                  </n-button>
                </div>
              </div>

              <div class="setting-row toggle-row">
                <n-text depth="3">每次保存在独立文件夹中</n-text>
                <n-switch v-model:value="compress_use_subdir" />
              </div>

              <Transition name="collapse">
                <div v-if="compress_use_subdir" class="collapsible-section indent-section">
                  <SubdirPatternField v-model="compress_subdir_pattern" label="独立文件夹命名" />
                </div>
              </Transition>
            </div>
          </Transition>

          <n-text v-if="compressPreview" depth="3" class="path-preview">
            预览：{{ compressPreview }}
          </n-text>
        </n-space>
      </n-card>

      <n-card title="缓存" size="small">
        <n-space vertical :size="14" style="width: 100%">
          <div class="account-row">
            <n-text>{{ cacheSize !== null ? `缓存大小: ${fmtBytes(cacheSize)}` : '加载中...' }}</n-text>
            <n-button size="small" :loading="clearingCache" @click="doClearCache">删除缓存</n-button>
          </div>
          <div class="setting-row">
            <n-text depth="3" class="label">大图缓存上限</n-text>
            <div class="input-row">
              <n-select
                :value="image_cache_limit_mb === null ? 'unlimited' : 'custom'"
                :options="limitModeOptions"
                style="width: 110px"
                @update:value="onLimitModeChange"
              />
              <n-input-number
                v-if="image_cache_limit_mb !== null"
                v-model:value="image_cache_limit_mb"
                :min="1"
                :max="51200"
                style="width: 140px"
              >
                <template #suffix>MB</template>
              </n-input-number>
            </div>
          </div>
          <n-text depth="3" style="font-size: 12px">
            大图缓存用于加速重复预览；超出上限后自动清理最久未查看的图片
          </n-text>
        </n-space>
      </n-card>

      <n-card title="账号" size="small">
        <div class="account-row">
          <n-text v-if="auth.user">
            <n-icon :component="PersonOutline" :size="16" style="vertical-align: -2px; margin-right: 4px" />
            {{ auth.user.name }} ({{ auth.user.id }})
          </n-text>
          <n-text v-else depth="3">未登录</n-text>
          <n-button v-if="auth.user" size="small" @click="auth.doLogout()">
            <n-icon :component="LogOutOutline" :size="14" style="margin-right: 4px" />
            退出登录
          </n-button>
          <n-button v-else size="small" type="primary" :loading="auth.loading" @click="auth.login()">
            登录
          </n-button>
        </div>
      </n-card>
    </n-space>
  </div>
</template>

<style scoped>
.settings {
  width: 100%;
}

.setting-row {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.label {
  font-size: 13px;
}

.input-row {
  display: flex;
  gap: 8px;
}

.toggle-row {
  flex-direction: row;
  align-items: center;
  justify-content: space-between;
}

.test-result {
  margin-top: 4px;
}

.account-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.collapsible-section {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.indent-section {
  margin-left: 2px;
  padding-left: 12px;
  border-left: 2px solid rgba(128, 128, 128, 0.3);
}

.path-preview {
  font-size: 11px;
  font-family: Consolas, "Courier New", monospace;
  word-break: break-all;
}

.collapse-enter-active,
.collapse-leave-active {
  transition:
    max-height 0.3s cubic-bezier(0.4, 0, 0.2, 1),
    opacity 0.3s cubic-bezier(0.4, 0, 0.2, 1),
    margin-top 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  overflow: hidden;
}

.collapse-enter-from,
.collapse-leave-to {
  max-height: 0;
  opacity: 0;
  margin-top: 0;
}

.collapse-enter-to,
.collapse-leave-from {
  max-height: 500px;
  opacity: 1;
}
</style>

<script setup lang="ts">
import { ref } from "vue";
import {
  NCard,
  NSpace,
  NButton,
  NInput,
  NInputNumber,
  NSwitch,
  NTag,
  NIcon,
  NText,
  useMessage,
} from "naive-ui";
import { FolderOpenOutline, WifiOutline, PersonOutline, LogOutOutline } from "@vicons/ionicons5";
import { open } from "@tauri-apps/plugin-dialog";
import { useAppConfig } from "../composables/useAppConfig";
import { useAuthStore } from "../stores/auth";
import { invoke } from "@tauri-apps/api/core";

const message = useMessage();
const {
  proxy,
  save_dir,
  compress_enabled,
  compress_separate,
  compress_dir,
  compress_max_mb,
} = useAppConfig();
const auth = useAuthStore();

const testing = ref(false);
const testResult = ref<{ ok: boolean; text: string } | null>(null);

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
</script>

<template>
  <div class="settings">
    <h1 class="page-title">设置</h1>
    <p class="page-subtitle">网络、保存路径与账号配置</p>

    <n-space vertical :size="16" style="width: 100%">
      <n-card title="网络" size="small">
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
      </n-card>

      <n-card title="保存" size="small">
        <n-space vertical :size="14" style="width: 100%">
          <div class="setting-row">
            <n-text depth="3" class="label">原图保存路径</n-text>
            <div class="input-row">
              <n-input :value="save_dir" placeholder="选择原图保存目录..." readonly />
              <n-button secondary @click="chooseDir('save_dir')">
                <n-icon :component="FolderOpenOutline" :size="16" />
              </n-button>
            </div>
          </div>

          <div class="setting-row toggle-row">
            <n-text depth="3">启用压缩保存</n-text>
            <n-switch v-model:value="compress_enabled" />
          </div>

          <div class="setting-row toggle-row">
            <n-text depth="3">压缩独立保存</n-text>
            <n-switch v-model:value="compress_separate" />
          </div>

          <div v-if="compress_enabled" class="setting-row">
            <n-text depth="3" class="label">压缩保存路径</n-text>
            <div class="input-row">
              <n-input :value="compress_dir" placeholder="选择压缩图保存目录..." readonly />
              <n-button secondary @click="chooseDir('compress_dir')">
                <n-icon :component="FolderOpenOutline" :size="16" />
              </n-button>
            </div>
          </div>

          <div v-if="compress_enabled" class="setting-row">
            <n-text depth="3" class="label">压缩最大大小 (MB)</n-text>
            <n-input-number
              v-model:value="compress_max_mb"
              :min="1"
              :max="50"
              style="width: 120px"
            />
          </div>
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
  max-width: 640px;
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
</style>

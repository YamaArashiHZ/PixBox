<script setup lang="ts">
import { NButton, NTooltip, NIcon } from "naive-ui";
import {
  HomeOutline,
  SettingsOutline,
  InformationCircleOutline,
  MoonOutline,
  SunnyOutline,
} from "@vicons/ionicons5";
import type { AppPage, ThemeMode } from "../types";
import pixboxMark from "../assets/pixbox-mark.svg";

defineProps<{
  current: AppPage;
  theme: ThemeMode;
}>();

const emit = defineEmits<{
  navigate: [page: AppPage];
  toggleTheme: [];
}>();
</script>

<template>
  <aside class="sidebar">
    <img class="brand" :src="pixboxMark" alt="PixBox" title="PixBox" />

    <nav class="nav-top">
      <n-tooltip placement="right" :delay="400">
        <template #trigger>
          <n-button
            quaternary
            circle
            size="large"
            :type="current === 'home' ? 'primary' : 'default'"
            :class="{ active: current === 'home' }"
            @click="emit('navigate', 'home')"
          >
            <template #icon>
              <n-icon :component="HomeOutline" :size="22" />
            </template>
          </n-button>
        </template>
        主页
      </n-tooltip>

      <n-tooltip placement="right" :delay="400">
        <template #trigger>
          <n-button
            quaternary
            circle
            size="large"
            :type="current === 'settings' ? 'primary' : 'default'"
            :class="{ active: current === 'settings' }"
            @click="emit('navigate', 'settings')"
          >
            <template #icon>
              <n-icon :component="SettingsOutline" :size="22" />
            </template>
          </n-button>
        </template>
        设置
      </n-tooltip>

      <n-tooltip placement="right" :delay="400">
        <template #trigger>
          <n-button
            quaternary
            circle
            size="large"
            :type="current === 'about' ? 'primary' : 'default'"
            :class="{ active: current === 'about' }"
            @click="emit('navigate', 'about')"
          >
            <template #icon>
              <n-icon :component="InformationCircleOutline" :size="22" />
            </template>
          </n-button>
        </template>
        关于
      </n-tooltip>
    </nav>

    <div class="nav-bottom">
      <n-tooltip placement="right" :delay="400">
        <template #trigger>
          <n-button quaternary circle size="large" @click="emit('toggleTheme')">
            <template #icon>
              <n-icon :component="theme === 'dark' ? SunnyOutline : MoonOutline" :size="22" />
            </template>
          </n-button>
        </template>
        {{ theme === "dark" ? "切换浅色" : "切换深色" }}
      </n-tooltip>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  width: var(--nav-width);
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 14px 0;
  gap: 8px;
  border-right: 1px solid var(--border-color);
  background: var(--sidebar-bg);
}

.brand {
  width: 40px;
  height: 40px;
  display: block;
  margin-bottom: 10px;
  user-select: none;
  -webkit-user-drag: none;
}

.nav-top {
  display: flex;
  flex-direction: column;
  gap: 6px;
  flex: 1;
}

.nav-bottom {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.active {
  background: var(--nav-active-bg) !important;
}
</style>

<script lang="ts">
// 会话级内存缓存（模块级，组件卸载后保留）：
// 磁盘缓存命中仍需 读盘+IPC+解码（百毫秒级），内存缓存让本会话内重复打开同步可用
const MEM_CACHE_LIMIT = 30;
const memCache = new Map<string, string>();

function memGet(url: string): string | undefined {
  const v = memCache.get(url);
  if (v !== undefined) {
    // LRU：提到最新位置
    memCache.delete(url);
    memCache.set(url, v);
  }
  return v;
}

function memSet(url: string, src: string) {
  memCache.delete(url);
  memCache.set(url, src);
  while (memCache.size > MEM_CACHE_LIMIT) {
    const oldest = memCache.keys().next().value!;
    memCache.delete(oldest);
  }
}
</script>

<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted, computed } from "vue";
import { NButton, NIcon, NSpin } from "naive-ui";
import { ChevronBackOutline, ChevronForwardOutline, CloseOutline } from "@vicons/ionicons5";
import { getImageData } from "../api";

const props = defineProps<{
  url: string;
  alt: string;
  thumb: string;
  ready: boolean;
  aspect: number;
  allItems: { key: string; large_url: string }[];
  currentKey: string;
}>();

const emit = defineEmits<{
  close: [];
  navigate: [direction: -1 | 1];
}>();

const imageData = ref("");
const largeLoaded = ref(false);
const loading = ref(false);
const failed = ref(false);
const zoomed = ref(false);

const thumbSrc = computed(() =>
  props.thumb ? `data:image/jpeg;base64,${props.thumb}` : ""
);

// 大图加载完成前用缩略图垫底
const displaySrc = computed(() => imageData.value || thumbSrc.value);

// 按原图真实宽高比计算 90vw/90vh 内的 contain 显示盒子
// （缩略图是方形裁切，盒子比例必须来自原图，否则大图叠入后比例不符）
const fitStyle = computed(() => {
  const a = props.aspect > 0 ? props.aspect : 1;
  const maxW = window.innerWidth * 0.9;
  const maxH = window.innerHeight * 0.9;
  let w = maxW;
  let h = w / a;
  if (h > maxH) {
    h = maxH;
    w = h * a;
  }
  return { width: `${w}px`, height: `${h}px` };
});

async function loadImage() {
  const url = props.url;
  imageData.value = "";
  largeLoaded.value = false;
  failed.value = false;
  if (!url) return;

  // 内存缓存命中：同步可用，无需读盘/网络
  const cached = memGet(url);
  if (cached) {
    imageData.value = cached;
    return;
  }

  loading.value = true;
  try {
    const b64 = await getImageData(url);
    if (props.url !== url) return;
    const src = `data:image/jpeg;base64,${b64}`;
    // 解码完成后再上屏，避免替换瞬间出现空白帧
    await new Promise<void>((resolve) => {
      const img = new Image();
      img.onload = () => resolve();
      img.onerror = () => resolve();
      img.src = src;
    });
    if (props.url === url) {
      imageData.value = src;
      memSet(url, src);
    }
  } catch {
    if (props.url === url) failed.value = true;
  } finally {
    if (props.url === url) loading.value = false;
  }
}

watch(() => props.url, loadImage, { immediate: true });

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") emit("close");
  if (e.key === "ArrowLeft") emit("navigate", -1);
  if (e.key === "ArrowRight") emit("navigate", 1);
}

function toggleZoom() {
  zoomed.value = !zoomed.value;
}

function onBackdropClick(e: MouseEvent) {
  if ((e.target as HTMLElement).classList.contains("lightbox-backdrop")) {
    emit("close");
  }
}

onMounted(() => {
  document.addEventListener("keydown", onKeydown);
});

onUnmounted(() => {
  document.removeEventListener("keydown", onKeydown);
});

const currentIdx = computed(() => props.allItems.findIndex((i) => i.key === props.currentKey));
</script>

<template>
  <div class="lightbox-backdrop" @click="onBackdropClick">
    <n-button circle class="lightbox-close" @click="emit('close')">
      <n-icon :component="CloseOutline" :size="20" />
    </n-button>

    <n-button
      v-if="currentIdx > 0"
      circle
      class="lightbox-nav lightbox-prev"
      @click.stop="emit('navigate', -1)"
    >
      <n-icon :component="ChevronBackOutline" :size="24" />
    </n-button>

    <n-button
      v-if="currentIdx < allItems.length - 1"
      circle
      class="lightbox-nav lightbox-next"
      @click.stop="emit('navigate', 1)"
    >
      <n-icon :component="ChevronForwardOutline" :size="24" />
    </n-button>

    <div class="lightbox-content" :class="{ zoomed, visible: ready }">
      <div
        v-if="!zoomed && displaySrc"
        class="img-stack cursor-zoom-in"
        :style="fitStyle"
        @click="toggleZoom"
      >
        <img v-if="thumbSrc" class="lb-base" :src="thumbSrc" :alt="alt" />
        <img
          v-if="imageData"
          class="lb-overlay"
          :class="{ loaded: largeLoaded }"
          :src="imageData"
          :alt="alt"
          @load="largeLoaded = true"
        />
      </div>
      <img
        v-else-if="zoomed && displaySrc"
        class="lb-zoomed cursor-zoom-out"
        :src="displaySrc"
        :alt="alt"
        @click="toggleZoom"
      />
      <div v-else-if="loading" class="lightbox-loading">
        <n-spin size="large" />
      </div>
      <div v-else class="lightbox-error">图片加载失败</div>
    </div>

    <div class="lightbox-counter">
      {{ currentIdx + 1 }} / {{ allItems.length }}
    </div>
  </div>
</template>

<style scoped>
.lightbox-backdrop {
  position: fixed;
  inset: 0;
  z-index: 1000;
  background: rgba(0, 0, 0, 0.92);
  display: flex;
  align-items: center;
  justify-content: center;
}

.lightbox-close {
  position: absolute;
  top: 16px;
  right: 16px;
  z-index: 10;
  opacity: 0.7;
}

.lightbox-close:hover {
  opacity: 1;
}

.lightbox-nav {
  position: absolute;
  top: 50%;
  transform: translateY(-50%);
  z-index: 10;
  opacity: 0.7;
}

.lightbox-nav:hover {
  opacity: 1;
}

.lightbox-prev {
  left: 16px;
}

.lightbox-next {
  right: 16px;
}

.lightbox-content {
  max-width: 90vw;
  max-height: 90vh;
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0;
  transition: opacity 0.12s ease;
}

.lightbox-content.visible {
  opacity: 1;
}

.lightbox-content.zoomed {
  max-width: none;
  max-height: none;
  overflow: auto;
}

/* 缩略图垫底 + 大图叠层淡入；盒子按原图真实宽高比定尺寸（行内 fitStyle） */
.img-stack {
  position: relative;
}

/* 方形缩略图裁切填满，与卡片/克隆体显示一致 */
.lb-base {
  width: 100%;
  height: 100%;
  object-fit: cover;
  border-radius: 4px;
  display: block;
}

/* 盒子比例 == 原图比例，contain 恰好精确填满 */
.lb-overlay {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: contain;
  border-radius: 4px;
  opacity: 0;
  transition: opacity 0.15s ease;
}

.lb-overlay.loaded {
  opacity: 1;
}

.lb-zoomed {
  display: block;
  border-radius: 4px;
}

.cursor-zoom-in {
  cursor: zoom-in;
}

.cursor-zoom-out {
  cursor: zoom-out;
}

.lightbox-loading,
.lightbox-error {
  color: rgba(255, 255, 255, 0.5);
  font-size: 16px;
  padding: 48px;
}

.lightbox-counter {
  position: absolute;
  bottom: 16px;
  left: 50%;
  transform: translateX(-50%);
  color: rgba(255, 255, 255, 0.5);
  font-size: 13px;
}
</style>

<style>
/* Lightbox 本身只做背景淡入淡出，位移动画全部由 HomeView 的克隆体承担 */
.lightbox-enter-active,
.lightbox-leave-active {
  transition: opacity 0.3s ease;
}

.lightbox-enter-from,
.lightbox-leave-to {
  opacity: 0;
}
</style>

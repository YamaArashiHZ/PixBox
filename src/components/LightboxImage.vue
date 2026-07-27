<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted, computed } from "vue";
import { NSpin } from "naive-ui";
import { getImageData } from "../api";
import { memGet, memSet } from "../utils/imageCache";

const props = defineProps<{
  url: string;
  alt: string;
  aspect: number;
}>();

// ── image loading ──
const imageData = ref("");
const largeLoaded = ref(false);
const loading = ref(false);
const failed = ref(false);

// ── zoom & pan state ──
const ZOOM_MIN = 0.5;
const ZOOM_MAX = 5;
const ZOOM_STEP = 0.25;

const zoomScale = ref(1);
const panX = ref(0);
const panY = ref(0);
const isPanning = ref(false);
let panStartClient = { x: 0, y: 0 };
let panStartOffset = { x: 0, y: 0 };

// ── computed ──
const cursorClass = computed(() => {
  if (zoomScale.value <= 1) return "";
  return isPanning.value ? "cursor-grabbing" : "cursor-grab";
});

function fitBox() {
  const a = props.aspect > 0 ? props.aspect : 1;
  const maxW = window.innerWidth * 0.9;
  const maxH = window.innerHeight * 0.9;
  let w = maxW;
  let h = w / a;
  if (h > maxH) {
    h = maxH;
    w = h * a;
  }
  return { w, h };
}

const fitStyle = computed(() => {
  const { w, h } = fitBox();
  return { width: `${w}px`, height: `${h}px` };
});

const zoomLayerStyle = computed(() => {
  const t =
    zoomScale.value <= 1 && panX.value === 0 && panY.value === 0
      ? ""
      : `translate(${panX.value}px, ${panY.value}px) scale(${zoomScale.value})`;
  return {
    transform: t,
    transition: isPanning.value ? "none" : "transform 0.15s ease-out",
  };
});

// ── pan clamping ──
function clampPan() {
  const { w, h } = fitBox();
  const maxPanX = (w * (zoomScale.value - 1)) / 2;
  const maxPanY = (h * (zoomScale.value - 1)) / 2;
  panX.value = Math.max(-maxPanX, Math.min(maxPanX, panX.value));
  panY.value = Math.max(-maxPanY, Math.min(maxPanY, panY.value));
}

// ── wheel zoom (zoom-to-cursor) ──
function onWheel(e: WheelEvent) {
  e.preventDefault();
  const oldScale = zoomScale.value;
  const delta = e.deltaY > 0 ? -ZOOM_STEP : ZOOM_STEP;
  const newScale = Math.max(ZOOM_MIN, Math.min(ZOOM_MAX, oldScale + delta));
  if (newScale === oldScale) return;

  const el = e.currentTarget as HTMLElement;
  const rect = el.getBoundingClientRect();
  const mx = e.clientX - rect.left - rect.width / 2;
  const my = e.clientY - rect.top - rect.height / 2;

  const ratio = newScale / oldScale;
  panX.value = mx - (mx - panX.value) * ratio;
  panY.value = my - (my - panY.value) * ratio;

  zoomScale.value = newScale;

  if (newScale <= 1) {
    zoomScale.value = 1;
    panX.value = 0;
    panY.value = 0;
  } else {
    clampPan();
  }
}

// ── pan (drag) ──
function onMouseDown(e: MouseEvent) {
  if (zoomScale.value <= 1) return;
  e.preventDefault();
  isPanning.value = true;
  panStartClient = { x: e.clientX, y: e.clientY };
  panStartOffset = { x: panX.value, y: panY.value };
}

function onGlobalMouseMove(e: MouseEvent) {
  if (!isPanning.value) return;
  panX.value = panStartOffset.x + (e.clientX - panStartClient.x);
  panY.value = panStartOffset.y + (e.clientY - panStartClient.y);
}

function onGlobalMouseUp() {
  if (!isPanning.value) return;
  isPanning.value = false;
  clampPan();
}

// ── image loading ──
async function loadImage() {
  const url = props.url;
  if (!url) return;

  // 内存缓存命中：同步可用，enter 第一帧即显示清晰大图
  const cached = memGet(url);
  if (cached) {
    imageData.value = cached;
    largeLoaded.value = true;
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

onMounted(() => {
  document.addEventListener("mousemove", onGlobalMouseMove);
  document.addEventListener("mouseup", onGlobalMouseUp);
});

onUnmounted(() => {
  document.removeEventListener("mousemove", onGlobalMouseMove);
  document.removeEventListener("mouseup", onGlobalMouseUp);
});
</script>

<template>
  <div class="img-frame" :style="fitStyle">
    <div
      v-if="imageData"
      class="img-stack"
      :class="cursorClass"
      @wheel="onWheel"
      @mousedown="onMouseDown"
    >
      <div class="zoom-layer" :style="zoomLayerStyle">
        <img
          class="lb-overlay"
          :class="{ loaded: largeLoaded }"
          :src="imageData"
          :alt="alt"
          @load="largeLoaded = true"
        />
      </div>
    </div>
    <div v-else-if="loading" class="lightbox-loading">
      <n-spin size="large" />
    </div>
    <div v-else class="lightbox-error">图片加载失败</div>
  </div>
</template>

<style scoped>
.img-frame {
  overflow: hidden;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.img-stack {
  position: relative;
  width: 100%;
  height: 100%;
}

.zoom-layer {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  will-change: transform;
}

.lb-overlay {
  width: 100%;
  height: 100%;
  object-fit: contain;
  display: block;
  opacity: 0;
  transition: opacity 0.15s ease;
}

.lb-overlay.loaded {
  opacity: 1;
}

.cursor-grab {
  cursor: grab;
}

.cursor-grabbing {
  cursor: grabbing;
}

.lightbox-loading,
.lightbox-error {
  color: rgba(255, 255, 255, 0.5);
  font-size: 16px;
  padding: 48px;
}
</style>

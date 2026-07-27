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

// zoom-layer 以 fit 尺寸居中于全屏视口（left:50% + 负 margin），
// transform 以元素中心（== 窗口中心）为原点缩放，pan 为屏幕像素偏移
const zoomLayerStyle = computed(() => {
  const { w, h } = fitBox();
  return {
    width: `${w}px`,
    height: `${h}px`,
    marginLeft: `${-w / 2}px`,
    marginTop: `${-h / 2}px`,
    transform: `translate(${panX.value}px, ${panY.value}px) scale(${zoomScale.value})`,
    transition: isPanning.value ? "none" : "transform 0.15s ease-out",
  };
});

// ── pan clamping：视口为整个窗口；图片小于窗口的方向锁定居中 ──
function clampPan() {
  const { w, h } = fitBox();
  const vpW = window.innerWidth;
  const vpH = window.innerHeight;
  const maxPanX = Math.max(0, (w * zoomScale.value - vpW) / 2);
  const maxPanY = Math.max(0, (h * zoomScale.value - vpH) / 2);
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
  // 图片小于窗口的方向会被 clamp 到 0（居中），大于窗口的方向限制边缘
  clampPan();
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
  <div class="img-frame">
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
/* 全屏视口：放大后的图片可铺满整个窗口，超出部分被裁剪 */
.img-frame {
  position: relative;
  width: 100vw;
  height: 100vh;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
}

/* 事件层铺满视口：整个窗口内均可滚轮缩放、拖拽平移 */
.img-stack {
  position: absolute;
  inset: 0;
}

/* fit 尺寸居中于视口（left:50% + 负 margin 由内联样式提供），随 transform 缩放/平移 */
.zoom-layer {
  position: absolute;
  left: 50%;
  top: 50%;
  overflow: hidden;
  border-radius: 4px;
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

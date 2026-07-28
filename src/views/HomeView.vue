<script setup lang="ts">
import { onMounted, watch, ref, computed, nextTick } from "vue";
import {
  NButton,
  NIcon,
  NEmpty,
  NSpin,
  NTag,
  useMessage,
} from "naive-ui";
import { RefreshOutline, SaveOutline, ImageOutline } from "@vicons/ionicons5";
import { useAuthStore } from "../stores/auth";
import { useFeedStore, type FeedKind, type DisplayItem } from "../stores/feed";
import type { FeedItem, ProgressEvent } from "../api";
import { saveImages } from "../api";
import SegmentedControl from "../components/SegmentedControl.vue";
import ImageCard from "../components/ImageCard.vue";
import Lightbox from "../components/Lightbox.vue";
import { preloadImage } from "../utils/imageCache";
import SelectedTray from "../components/SelectedTray.vue";
import { useAppConfig } from "../composables/useAppConfig";

const message = useMessage();
const auth = useAuthStore();
const feed = useFeedStore();

const lightboxVisible = ref(false);
const lightboxReady = ref(false);
const lightboxKey = ref("");
const cardRect = ref<{ left: number; top: number; width: number; height: number } | null>(null);
const cardAspect = ref(1);
const cloneVisible = ref(false);
const cloneSrc = ref("");
const cloneEl = ref<HTMLElement | null>(null);
const cloneCurrent = ref({ left: "0px", top: "0px", width: "0px", height: "0px", borderRadius: "12px" });
const saving = ref(false);
const saveProgress = ref<number | null>(null);

const { hasPathConflict } = useAppConfig();
const saveWarning = computed(() =>
  hasPathConflict.value
    ? "原图与压缩图保存路径重合，请先修改设置"
    : null
);

const FLIGHT_MS = 300;
let flightToken = 0;

const feedOptions = [
  { label: "关注", value: "following" },
  { label: "推荐", value: "recommended" },
];

const selectedItems = computed<FeedItem[]>(() => feed.selectedItems);

// "帖子级"卡片（封面卡 / 收起状态的多图主图）：复选框反映整帖选择状态，点击=全选/取消全帖
function isGroupCard(item: DisplayItem): boolean {
  return !!item.cover || (item.page_count > 1 && !feed.isExpanded(item.illust_id));
}

const lightboxAllItems = computed(() =>
  feed.items.filter((i) => !i.cover).map((i) => ({ key: i.key, large_url: i.large_url }))
);

const lightboxItem = computed(() =>
  feed.items.find((i) => i.key === lightboxKey.value)
);

// 原图真实宽高比（缩略图是方形裁切，不能用作显示比例）
const lightboxAspect = computed(() => {
  const item = lightboxItem.value;
  return item && item.width > 0 && item.height > 0 ? item.width / item.height : 0;
});

const showLeftBtn = ref(false);
const showRightBtn = ref(true);

const filterRef = ref<HTMLElement | null>(null);
const indicatorStyle = ref({ left: "0px", width: "0px", opacity: "0" });

async function updateIndicator() {
  await nextTick();
  const el = filterRef.value;
  if (!el) return;
  const btns = el.querySelectorAll(".cf-btn");
  const mode = feed.contentMode;
  const idx = mode === "all" ? 0 : mode === "safe" ? 1 : 2;
  const btn = btns[idx] as HTMLElement;
  if (!btn) return;
  indicatorStyle.value = {
    left: `${btn.offsetLeft}px`,
    width: `${btn.offsetWidth}px`,
    opacity: "1",
  };
}

watch(() => feed.contentMode, updateIndicator, { immediate: true });
watch(() => feed.kind, (k) => { if (k === 'recommended') updateIndicator(); });

function setContentMode(mode: string) {
  feed.contentMode = mode;
  feed.load("recommended");
}

onMounted(() => {
  auth.checkLogin();
});

watch(
  () => auth.user,
  (u) => {
    if (u) feed.load("following");
    else feed.clearItems();
  },
  { immediate: true }
);

watch(
  () => feed.error,
  (e) => {
    if (e && (e.includes("已过期") || e.includes("请重新登录"))) {
      auth.user = null;
    }
  }
);

function handleFeedChange(kind: string) {
  feed.load(kind as FeedKind);
}

// 多图展开组：按相邻 illust_id 判定分组位置，用于描边画框样式
// 封面卡（cover）为大卡 + 收起入口；其余为缩小的子页卡
function groupClass(i: number): string | null {
  const list = feed.items;
  const item = list[i];
  if (!item || item.page_count <= 1 || !feed.isExpanded(item.illust_id)) return null;
  const start = i === 0 || list[i - 1].illust_id !== item.illust_id;
  const end = i === list.length - 1 || list[i + 1].illust_id !== item.illust_id;
  const pos = start && end ? "grp-single" : start ? "grp-start" : end ? "grp-end" : "grp-mid";
  return item.cover ? `${pos} grp-cover` : `${pos} grp-sub`;
}

const CARD_GAP = 12;
let scrollAnimRaf = 0;
let scrollTarget = 0;

function cardWidth(): number {
  const el = document.querySelector('.feed-scroll');
  if (!el) return 200;
  const first = el.querySelector('.card-wrap');
  return first ? (first as HTMLElement).offsetWidth : (el.clientHeight * 2) / 3;
}

function getScrollEl(): HTMLElement | null {
  return document.querySelector('.feed-scroll');
}

function animateScroll() {
  const el = getScrollEl();
  if (!el) { scrollAnimRaf = 0; return; }
  const cur = el.scrollLeft;
  const diff = scrollTarget - cur;
  if (Math.abs(diff) < 0.5) {
    el.scrollLeft = scrollTarget;
    scrollAnimRaf = 0;
    showLeftBtn.value = scrollTarget > 1;
    showRightBtn.value = scrollTarget + el.clientWidth < el.scrollWidth - 1;
    checkLoadMore(el);
    return;
  }
  el.scrollLeft = cur + diff * 0.2;
  showLeftBtn.value = scrollTarget > 1;
  showRightBtn.value = scrollTarget + el.clientWidth < el.scrollWidth - 1;
  checkLoadMore(el);
  scrollAnimRaf = requestAnimationFrame(animateScroll);
}

function cancelScrollAnim() {
  if (scrollAnimRaf) {
    cancelAnimationFrame(scrollAnimRaf);
    scrollAnimRaf = 0;
  }
}

function scrollTo(target: number) {
  cancelScrollAnim();
  scrollTarget = Math.max(0, target);
  showLeftBtn.value = scrollTarget > 1;
  const el = getScrollEl();
  if (el) {
    showRightBtn.value = scrollTarget + el.clientWidth < el.scrollWidth - 1;
    checkLoadMore(el);
  }
  scrollAnimRaf = requestAnimationFrame(animateScroll);
}

function checkLoadMore(el: HTMLElement) {
  if (el.scrollLeft + el.clientWidth >= el.scrollWidth * 0.8) {
    feed.loadMore();
  }
}

function onWheel(e: WheelEvent) {
  const el = e.currentTarget as HTMLElement;
  if (!el) return;
  e.preventDefault();
  const step = cardWidth() + CARD_GAP;
  const dir = e.deltaY > 0 ? 1 : -1;
  const cur = scrollAnimRaf ? scrollTarget : el.scrollLeft;
  const target = cur + dir * step;
  const max = el.scrollWidth - el.clientWidth;
  scrollTo(Math.max(0, Math.min(target, max)));
}

function arrowStep(): number {
  const el = getScrollEl();
  if (!el) return cardWidth() + CARD_GAP;
  const cardW = cardWidth() + CARD_GAP;
  const cardsPerRow = Math.floor(el.clientWidth / cardW);
  return Math.max(1, cardsPerRow - 1) * cardW;
}

function pageBackward() {
  const el = getScrollEl();
  if (!el) return;
  const cur = scrollAnimRaf ? scrollTarget : el.scrollLeft;
  scrollTo(Math.max(0, cur - arrowStep()));
}

function pageForward() {
  const el = getScrollEl();
  if (!el) return;
  const cur = scrollAnimRaf ? scrollTarget : el.scrollLeft;
  scrollTo(Math.min(el.scrollWidth - el.clientWidth, cur + arrowStep()));
}

function pageEnd() {
  const el = getScrollEl();
  if (!el) return;
  scrollTo(el.scrollWidth - el.clientWidth);
}

// 根据卡片 key 抓取缩略图位置；宽高比优先用原图真实尺寸（缩略图是方形裁切，不能代表原图比例）
function captureCard(key: string, item?: FeedItem): boolean {
  const imgEl = document.querySelector(`[data-key="${key}"] .image-wrap img`) as HTMLImageElement | null;
  const rect = imgEl?.getBoundingClientRect();
  if (!rect || rect.width === 0) return false;
  cardRect.value = { left: rect.left, top: rect.top, width: rect.width, height: rect.height };
  if (item && item.width > 0 && item.height > 0) {
    cardAspect.value = item.width / item.height;
  } else {
    cardAspect.value = imgEl!.naturalWidth > 0
      ? imgEl!.naturalWidth / imgEl!.naturalHeight
      : rect.width / rect.height;
  }
  return true;
}

// 与 Lightbox 实际显示一致的目标矩形：90vw/90vh 内 contain 居中
function centerBox() {
  const aspect = cardAspect.value || 1;
  const maxW = window.innerWidth * 0.9;
  const maxH = window.innerHeight * 0.9;
  let width = maxW;
  let height = width / aspect;
  if (height > maxH) {
    height = maxH;
    width = height * aspect;
  }
  return {
    left: (window.innerWidth - width) / 2,
    top: (window.innerHeight - height) / 2,
    width,
    height,
    radius: 4,
  };
}

function cardBox() {
  return { ...cardRect.value!, radius: 12 };
}

function setClone(box: { left: number; top: number; width: number; height: number; radius: number }) {
  cloneCurrent.value = {
    left: `${box.left}px`,
    top: `${box.top}px`,
    width: `${box.width}px`,
    height: `${box.height}px`,
    borderRadius: `${box.radius}px`,
  };
}

// 飞行结束后的交接：优先用 transitionend，超时兜底；token 防止旧回调干扰新动画
function afterFlight(token: number, cb: () => void) {
  let done = false;
  const el = cloneEl.value;
  const finish = (e?: TransitionEvent) => {
    if (done || token !== flightToken) return;
    if (e && e.propertyName !== "width") return;
    done = true;
    window.clearTimeout(timer);
    el?.removeEventListener("transitionend", finish);
    cb();
  };
  el?.addEventListener("transitionend", finish);
  const timer = window.setTimeout(finish, FLIGHT_MS + 60);
}

function preloadAdjacent(key: string) {
  const idx = lightboxAllItems.value.findIndex((i) => i.key === key);
  if (idx > 0) preloadImage(lightboxAllItems.value[idx - 1].large_url);
  if (idx < lightboxAllItems.value.length - 1) preloadImage(lightboxAllItems.value[idx + 1].large_url);
}

function openLightbox(item: DisplayItem) {
  const token = ++flightToken;
  // 封面卡预览该帖第 1 页：灯箱 key 用其后的真实页，飞行动画起点仍用被点击的封面卡
  const domKey = item.key;
  if (item.cover) {
    const idx = feed.items.findIndex((i) => i.key === item.key);
    item = feed.items[idx + 1] ?? item;
  }
  lightboxKey.value = item.key;
  lightboxVisible.value = true;
  lightboxReady.value = false;
  cardRect.value = null;
  if (!captureCard(domKey, item) || !item.thumb_b64) {
    lightboxReady.value = true;
    return;
  }
  cloneSrc.value = `data:image/jpeg;base64,${item.thumb_b64}`;
  cloneVisible.value = true;
  setClone(cardBox());
  preloadAdjacent(item.key);
  nextTick(() => {
    if (token !== flightToken) return;
    // 强制 reflow，确保起始样式先生效再触发动画
    if (cloneEl.value) void cloneEl.value.offsetWidth;
    setClone(centerBox());
    afterFlight(token, () => {
      lightboxReady.value = true;
      // 等图片区淡入完成后再撤掉克隆体，避免交接瞬间闪烁
      window.setTimeout(() => {
        if (token === flightToken) cloneVisible.value = false;
      }, 150);
    });
  });
}

function closeLightbox() {
  const token = ++flightToken;
  if (!cardRect.value) {
    lightboxVisible.value = false;
    lightboxKey.value = "";
    return;
  }
  // 图片区快速隐去，背景在飞行过程中淡出，克隆体飞回卡片位置
  lightboxReady.value = false;
  lightboxVisible.value = false;
  cloneVisible.value = true;
  setClone(centerBox());
  nextTick(() => {
    if (token !== flightToken) return;
    if (cloneEl.value) void cloneEl.value.offsetWidth;
    setClone(cardBox());
    afterFlight(token, () => {
      cloneVisible.value = false;
      lightboxKey.value = "";
      cardRect.value = null;
    });
  });
}

function lightboxNavigate(dir: -1 | 1) {
  const idx = lightboxAllItems.value.findIndex((i) => i.key === lightboxKey.value);
  const next = idx + dir;
  if (next >= 0 && next < lightboxAllItems.value.length) {
    lightboxKey.value = lightboxAllItems.value[next].key;
    // 同步更新关闭动画的返回目标和缩略图
    const item = feed.items.find((i) => i.key === lightboxKey.value);
    captureCard(lightboxKey.value, item);
    if (item) cloneSrc.value = `data:image/jpeg;base64,${item.thumb_b64}`;
    preloadAdjacent(lightboxKey.value);
  }
}

async function handleSave() {
  if (saving.value || selectedItems.value.length === 0) return;
  saving.value = true;
  saveProgress.value = 0;

  const items = selectedItems.value.map((i) => ({
    key: i.key,
    illust_id: i.illust_id,
    original_url: i.original_url,
  }));

  try {
    const report = await saveImages(items, (e: ProgressEvent) => {
      saveProgress.value = e.percent;
    });

    if (report.errors.length > 0) {
      message.warning(`完成: ${report.success}/${items.length}，失败 ${report.failed} 张`);
    } else {
      message.success(`成功保存 ${report.success} 张`);
    }

    const failedKeys = new Set(report.errors.map((e) => e.split(":")[0]));
    const bookmarked = new Set<number>();
    for (const item of items) {
      if (!failedKeys.has(item.key)) {
        bookmarked.add(item.illust_id);
      }
    }
    feed.markBookmarked(bookmarked);
    feed.clearSelection();
  } catch (e) {
    message.error(String(e));
  } finally {
    saving.value = false;
    saveProgress.value = null;
  }
}

function handleRemoveFromTray(key: string) {
  feed.toggleSelect(key);
}

function handleClearSelection() {
  feed.clearSelection();
}
</script>

<template>
  <div class="home" :style="{ paddingBottom: '200px' }">
    <template v-if="!auth.user">
      <div class="login-state">
        <NIcon :component="ImageOutline" :size="48" color="#5b7cfa" />
        <h1 class="page-title">PixBox</h1>
        <p class="page-subtitle">登录 Pixiv 以加载关注和推荐列表</p>
        <NButton type="primary" size="large" :loading="auth.loading" @click="auth.login()">
          {{ auth.loading ? "登录中..." : "登录 Pixiv" }}
        </NButton>
        <NTag v-if="auth.error" type="error" size="small" :bordered="false" closable @close="auth.clearError()">
          {{ auth.error }}
        </NTag>
      </div>
    </template>

    <template v-else>
      <h1 class="page-title">图片浏览</h1>
      <p class="page-subtitle">横向滚动浏览，点击勾选后保存到本地</p>

      <div class="feed-header">
        <SegmentedControl
          :options="feedOptions"
          :model-value="feed.kind"
          @update:model-value="handleFeedChange"
        />
        <NButton circle quaternary size="large" @click="feed.load(feed.kind)" :disabled="feed.refreshing">
          <NSpin v-if="feed.refreshing" :size="18" />
          <NIcon v-else :component="RefreshOutline" :size="20" />
        </NButton>
        <div v-if="feed.kind === 'recommended'" class="content-filter" ref="filterRef">
          <div class="cf-indicator" :style="indicatorStyle" />
          <button class="cf-btn" :class="{ active: feed.contentMode === 'all' }" @click="setContentMode('all')">全部</button>
          <button class="cf-btn" :class="{ active: feed.contentMode === 'safe' }" @click="setContentMode('safe')">全年龄</button>
          <button class="cf-btn" :class="{ active: feed.contentMode === 'r18' }" @click="setContentMode('r18')">R-18</button>
        </div>
      </div>

      <div v-if="feed.error" class="feed-error">{{ feed.error }}</div>

      <NEmpty v-if="feed.isEmpty && !feed.loading" description="暂无图片" class="feed-empty" />

      <div
        v-if="feed.items.length > 0 || feed.loading"
        class="feed-wrapper"
      >
        <button v-if="feed.items.length > 0 && showLeftBtn" class="scroll-arrow scroll-left" @click.left="pageBackward" @click.right.prevent="() => scrollTo(0)">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="15 18 9 12 15 6" /></svg>
        </button>
        <button
          v-if="feed.items.length > 0 && showRightBtn"
          class="scroll-arrow scroll-right"
          @click.left="pageForward"
          @click.right.prevent="pageEnd"
        >
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6" /></svg>
        </button>
        <div
          class="feed-scroll"
          @wheel="onWheel"
        >
          <div v-if="feed.items.length === 0" class="feed-cards skeleton-list">
            <div v-for="i in 6" :key="i" class="card-wrap">
              <div class="skeleton-card">
                <div class="skeleton-image shimmer" />
                <div class="skeleton-info">
                  <span class="skeleton-line skeleton-line-short shimmer" />
                  <span class="skeleton-line shimmer" />
                </div>
              </div>
            </div>
          </div>
          <TransitionGroup v-else name="card" tag="div" class="feed-cards">
            <div
              v-for="(item, i) in feed.items"
              :key="item.key"
              class="card-wrap"
              :class="[groupClass(i), { 'card-selected': isGroupCard(item) ? feed.isAllSelected(item.illust_id) : feed.isSelected(item.selKey ?? item.key) }]"
            >
              <ImageCard
                :item-key="item.key"
                :illust-id="item.illust_id"
                :thumb-b64="item.thumb_b64"
                :title="item.title"
                :artist="item.artist"
                :x-restrict="item.x_restrict"
                :illust-ai-type="item.illust_ai_type"
                :is-bookmarked="item.is_bookmarked"
                :selected="isGroupCard(item) ? feed.isAllSelected(item.illust_id) : feed.isSelected(item.selKey ?? item.key)"
                :indeterminate="isGroupCard(item) ? feed.isSomeSelected(item.illust_id) : false"
                :page="item.page"
                :page-count="item.page_count"
                :expanded="feed.isExpanded(item.illust_id)"
                :cover="item.cover"
                @toggle="isGroupCard(item) ? feed.toggleSelectIllust(item.illust_id) : feed.toggleSelect(item.selKey ?? item.key)"
                @preview="openLightbox(item)"
                @expand="feed.toggleExpand(item.illust_id)"
                @toggle-bookmark="feed.doToggleBookmark(item.illust_id, item.is_bookmarked)"
              />
            </div>
          </TransitionGroup>
          <div v-if="feed.items.length > 0 && feed.loading" class="scroll-loading">
            <NSpin size="large" />
          </div>
        </div>
      </div>

      <SelectedTray
        :items="selectedItems"
        :progress="saveProgress"
        :saving="saving"
        :saveWarning="saveWarning"
        @remove="handleRemoveFromTray"
        @save="handleSave"
        @clear="handleClearSelection"
      />
    </template>

    <Transition name="lightbox">
      <Lightbox
        v-if="lightboxVisible && lightboxItem"
        :url="lightboxItem.large_url"
        :alt="lightboxItem.title"
        :ready="lightboxReady"
        :aspect="lightboxAspect"
        :all-items="lightboxAllItems"
        :current-key="lightboxKey"
        @close="closeLightbox"
        @navigate="lightboxNavigate"
      />
    </Transition>

    <Teleport to="body">
      <div
        v-if="cloneVisible && cardRect"
        ref="cloneEl"
        class="clone-wrapper"
        :style="cloneCurrent"
      >
        <img :src="cloneSrc" class="clone-img" />
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.home {
  width: 100%;
  max-width: none;
  box-sizing: border-box;
  height: calc(100vh - 44px);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.login-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  text-align: center;
  gap: 16px;
  flex: 1;
}

.feed-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 2px 0;
}

.content-filter {
  display: flex;
  gap: 0;
  align-items: center;
  margin-left: auto;
  position: relative;
  padding: 2px 0;
}

.cf-indicator {
  position: absolute;
  top: 0;
  height: 100%;
  border-radius: 14px;
  background: rgba(128, 128, 128, 0.15);
  transition: left 0.25s cubic-bezier(0.4, 0, 0.2, 1), width 0.25s cubic-bezier(0.4, 0, 0.2, 1), opacity 0.2s ease;
  z-index: 0;
  pointer-events: none;
}

.cf-btn {
  position: relative;
  z-index: 1;
  padding: 3px 12px;
  border: none;
  border-radius: 14px;
  background: transparent;
  color: #999;
  font-size: 15px;
  font-weight: 600;
  line-height: 1.4;
  cursor: pointer;
  transition: color 0.18s ease;
}

.cf-btn:hover,
.cf-btn.active {
  color: #333;
}

.feed-error {
  padding: 12px 16px;
  background: rgba(239, 68, 68, 0.1);
  border-radius: 10px;
  color: #ef4444;
  font-size: 13px;
}

.feed-empty {
  padding: 48px 0;
}

.feed-wrapper {
  flex: 1;
  min-height: 0;
  max-height: 480px;
  overflow: hidden;
  position: relative;
}

.feed-scroll {
  overflow: hidden;
  height: 100%;
  position: relative;
  display: flex;
}

.feed-cards {
  display: flex;
  gap: 12px;
  padding: 8px;
  height: 100%;
}

.card-wrap {
  position: relative;
  flex-shrink: 0;
  height: 100%;
  aspect-ratio: 2/3;
}

.skeleton-card {
  width: 100%;
  height: 100%;
  overflow: hidden;
  border: 1px solid var(--border-color);
  border-radius: 12px;
  background: var(--preview-bg);
}

.skeleton-image {
  height: calc(100% - 52px);
}

.skeleton-info {
  height: 52px;
  padding: 9px 10px;
  display: flex;
  flex-direction: column;
  gap: 7px;
}

.skeleton-line {
  display: block;
  width: 72%;
  height: 9px;
  border-radius: 999px;
}

.skeleton-line-short {
  width: 42%;
}

.shimmer {
  background: linear-gradient(
    100deg,
    rgba(128, 128, 128, 0.08) 20%,
    rgba(128, 128, 128, 0.2) 40%,
    rgba(128, 128, 128, 0.08) 60%
  );
  background-size: 240% 100%;
  animation: skeleton-shimmer 1.4s ease-in-out infinite;
}

@keyframes skeleton-shimmer {
  from { background-position: 120% 0; }
  to { background-position: -120% 0; }
}

.scroll-loading {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  min-width: 120px;
  padding: 4px;
}

.scroll-arrow {
  position: absolute;
  top: 50%;
  transform: translateY(-50%);
  z-index: 10;
  width: 40px;
  height: 40px;
  border: none;
  border-radius: 50%;
  background: rgba(0, 0, 0, 0.35);
  color: #fff;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  backdrop-filter: blur(4px);
  transition: background 0.18s ease;
}

.scroll-arrow:hover {
  background: rgba(0, 0, 0, 0.55);
}

.scroll-left {
  left: 8px;
}

.scroll-right {
  right: 8px;
}

.feed-scroll::-webkit-scrollbar {
  height: 6px;
}

.feed-scroll::-webkit-scrollbar-track {
  background: transparent;
}

.feed-scroll::-webkit-scrollbar-thumb {
  background: rgba(128, 128, 128, 0.2);
  border-radius: 3px;
}

.card-wrap {
  position: relative;
  flex-shrink: 0;
  height: 100%;
  aspect-ratio: 2/3;
}

/* ===== 多图展开组：描边式画框 ===== */
/* 伪元素常驻但透明，展开时淡入、收起时淡出，避免外框突兀出现 */
.card-wrap::before {
  content: "";
  position: absolute;
  top: -2px;
  bottom: -2px;
  left: 0;
  right: 0;
  z-index: -1;
  pointer-events: none;
  opacity: 0;
  transition: opacity 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

/* 外框：整组一圈 2px 主题色描边，条带位于 [卡片外2px, 卡片边缘]，
   与封面卡描边环完全重叠，仅组两端圆角 */
.card-wrap.grp-start::before,
.card-wrap.grp-mid::before,
.card-wrap.grp-end::before,
.card-wrap.grp-single::before {
  opacity: 1;
  --grp-line: var(--primary-soft);
  box-shadow:
    inset 0 2px 0 var(--grp-line),
    inset 0 -2px 0 var(--grp-line);
}

/* 组内间隙 12px，相邻外框各延伸 8px 桥接（同色不透明，重叠无缝） */
.card-wrap.grp-start::before {
  left: -2px;
  right: -8px;
  border-radius: 14px 0 0 14px;
  box-shadow:
    inset 2px 0 0 var(--grp-line),
    inset 0 2px 0 var(--grp-line),
    inset 0 -2px 0 var(--grp-line);
}

.card-wrap.grp-mid::before {
  left: -8px;
  right: -8px;
}

/* 外框右边竖条外扩至 -8px：与末位子卡描边环拉开内边距，右端圆角相应增大 */
.card-wrap.grp-end::before {
  left: -8px;
  right: -8px;
  border-radius: 0 20px 20px 0;
  box-shadow:
    inset -2px 0 0 var(--grp-line),
    inset 0 2px 0 var(--grp-line),
    inset 0 -2px 0 var(--grp-line);
}

.card-wrap.grp-single::before {
  left: -2px;
  right: -8px;
  border-radius: 14px 20px 20px 14px;
  box-shadow:
    inset 2px 0 0 var(--grp-line),
    inset -2px 0 0 var(--grp-line),
    inset 0 2px 0 var(--grp-line),
    inset 0 -2px 0 var(--grp-line);
}

/* 封面卡：全高大卡 + 2px 描边环 */
.card-wrap.grp-cover {
  border-radius: 12px;
  box-shadow: 0 0 0 2px var(--primary-soft);
  transition: box-shadow 0.18s ease;
}

/* 组图末尾与后续卡片之间留出适度间隔（配合外框右竖条外扩） */
.card-wrap.grp-end,
.card-wrap.grp-single {
  margin-right: 6px;
}

.card-wrap.grp-cover:hover,
.card-wrap.grp-cover.card-selected {
  box-shadow: 0 0 0 2px var(--primary-soft), 0 0 12px rgba(91, 124, 250, 0.35);
}

/* 子页卡：缩小至 96% 并居中，带 2px 描边环（保留 hover/选中发光反馈） */
.card-wrap.grp-sub {
  display: flex;
  align-items: center;
  justify-content: center;
}

.card-wrap.grp-sub :deep(.image-card) {
  height: 96%;
  box-shadow: 0 0 0 2px var(--primary-soft);
}

.card-wrap.grp-sub :deep(.image-card:hover) {
  box-shadow: 0 0 0 2px var(--primary-soft), 0 0 8px rgba(91, 124, 250, 0.25);
}

.card-wrap.grp-sub :deep(.image-card.selected) {
  box-shadow: 0 0 0 2px var(--primary-soft), 0 0 12px rgba(91, 124, 250, 0.45);
}

.card-enter-active {
  transition: opacity 0.3s cubic-bezier(0.4, 0, 0.2, 1), transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.card-enter-from {
  opacity: 0;
  transform: translateX(-24px) scale(0.92);
}

/* 离开元素不脱流：height → 0（aspect-ratio 同步收拢宽度）+ 吃掉尾部间隙，
   后续卡片随布局自然平滑左移，实现"折叠收起" */
.card-leave-active {
  z-index: 1;
  transition:
    opacity 0.3s cubic-bezier(0.4, 0, 0.2, 1),
    height 0.3s cubic-bezier(0.4, 0, 0.2, 1),
    margin-right 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

/* 离开期间禁用内卡自身的 height 过渡，使其即时跟随容器收拢，避免溢出滞后 */
.card-leave-active :deep(.image-card) {
  transition: none;
}

/* 提高优先级，确保折叠时 margin 收拢覆盖组图末尾的 margin-right */
.card-wrap.card-leave-to {
  opacity: 0;
  height: 0;
  margin-right: -12px;
}

.card-move {
  transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.clone-wrapper {
  position: fixed;
  z-index: 9999;
  pointer-events: none;
  transition:
    left 0.3s cubic-bezier(0.4, 0, 0.2, 1),
    top 0.3s cubic-bezier(0.4, 0, 0.2, 1),
    width 0.3s cubic-bezier(0.4, 0, 0.2, 1),
    height 0.3s cubic-bezier(0.4, 0, 0.2, 1),
    border-radius 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  overflow: hidden;
}

.clone-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}
</style>

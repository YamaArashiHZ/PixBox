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
import { useFeedStore, type FeedKind } from "../stores/feed";
import type { FeedItem, ProgressEvent } from "../api";
import { saveImages } from "../api";
import SegmentedControl from "../components/SegmentedControl.vue";
import ImageCard from "../components/ImageCard.vue";
import Lightbox from "../components/Lightbox.vue";
import SelectedTray from "../components/SelectedTray.vue";

const message = useMessage();
const auth = useAuthStore();
const feed = useFeedStore();

const lightboxVisible = ref(false);
const lightboxKey = ref("");
const saving = ref(false);
const saveProgress = ref<number | null>(null);

const feedOptions = [
  { label: "关注", value: "following" },
  { label: "推荐", value: "recommended" },
];

const selectedItems = computed<FeedItem[]>(() =>
  feed.items.filter((i) => feed.isSelected(i.key))
);

const lightboxAllItems = computed(() =>
  feed.items.map((i) => ({ key: i.key, large_url: i.large_url }))
);

const lightboxItem = computed(() =>
  feed.items.find((i) => i.key === lightboxKey.value)
);

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

function openLightbox(item: FeedItem) {
  lightboxKey.value = item.key;
  lightboxVisible.value = true;
}

function closeLightbox() {
  lightboxVisible.value = false;
  lightboxKey.value = "";
}

function lightboxNavigate(dir: -1 | 1) {
  const idx = lightboxAllItems.value.findIndex((i) => i.key === lightboxKey.value);
  const next = idx + dir;
  if (next >= 0 && next < lightboxAllItems.value.length) {
    lightboxKey.value = lightboxAllItems.value[next].key;
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
            <div v-for="item in feed.items" :key="item.key" class="card-wrap">
              <ImageCard
                :thumb-b64="item.thumb_b64"
                :title="item.title"
                :artist="item.artist"
                :is-bookmarked="item.is_bookmarked"
                :selected="feed.isSelected(item.key)"
                :page="item.page"
                :page-count="item.page_count"
                :expanded="feed.isExpanded(item.illust_id)"
                @toggle="feed.toggleSelect(item.key)"
                @preview="openLightbox(item)"
                @expand="feed.toggleExpand(item.illust_id)"
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
        @remove="handleRemoveFromTray"
        @save="handleSave"
      />
    </template>

    <Lightbox
      v-if="lightboxVisible && lightboxItem"
      :url="lightboxItem.large_url"
      :alt="lightboxItem.title"
      :all-items="lightboxAllItems"
      :current-key="lightboxKey"
      @close="closeLightbox"
      @navigate="lightboxNavigate"
    />
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
  padding: 4px 4px 4px 4px;
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

.card-enter-active,
.card-leave-active {
  transition: opacity 0.3s cubic-bezier(0.4, 0, 0.2, 1), transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.card-enter-from {
  opacity: 0;
  transform: translateX(-24px) scale(0.92);
}

.card-leave-to {
  opacity: 0;
  transform: translateX(24px) scale(0.92);
}

.card-leave-active {
  position: absolute;
  z-index: 1;
}

.card-move {
  transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}
</style>

<script setup lang="ts">
import { onMounted, watch, ref, computed } from "vue";
import {
  NButton,
  NCard,
  NSpace,
  NIcon,
  NEmpty,
  NSpin,
  NProgress,
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
    checkLoadMore(el);
    return;
  }
  el.scrollLeft = cur + diff * 0.2;
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
        <NButton circle quaternary size="large" @click="feed.load(feed.kind)" :disabled="feed.loading">
          <NSpin v-if="feed.loading" :size="18" />
          <NIcon v-else :component="RefreshOutline" :size="20" />
        </NButton>
      </div>

      <div v-if="feed.error" class="feed-error">{{ feed.error }}</div>

      <NEmpty v-if="feed.isEmpty && !feed.loading" description="暂无图片" class="feed-empty" />

      <div
        v-if="feed.items.length > 0"
        class="feed-wrapper"
      >
        <button v-if="showLeftBtn" class="scroll-arrow scroll-left" @click="pageBackward">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="15 18 9 12 15 6" /></svg>
        </button>
        <button class="scroll-arrow scroll-right" @click="pageForward">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6" /></svg>
        </button>
        <TransitionGroup name="card" tag="div"
          class="feed-scroll"
          @wheel="onWheel"
        >
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
      </div>

      <div v-if="feed.loading && feed.items.length > 0" class="feed-loading">
        <NSpin size="small" />
        <span>加载中...</span>
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
  display: flex;
  gap: 12px;
  overflow: hidden;
  padding: 4px 4px 4px 4px;
  height: 100%;
  position: relative;
}

.card-wrap {
  position: relative;
  flex-shrink: 0;
  height: 100%;
  aspect-ratio: 2/3;
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

.feed-loading {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  opacity: 0.65;
  font-size: 13px;
  padding: 8px;
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

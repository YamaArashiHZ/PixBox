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

const scrollContainer = ref<HTMLElement | null>(null);
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

function handleScroll() {
  const el = scrollContainer.value;
  if (!el) return;
  const { scrollLeft, scrollWidth, clientWidth } = el;
  if (scrollLeft + clientWidth >= scrollWidth * 0.8) {
    feed.loadMore();
  }
}

function onWheel(e: WheelEvent) {
  if (!scrollContainer.value) return;
  e.preventDefault();
  scrollContainer.value.scrollLeft += e.deltaY;
}

function openLightbox(key: string) {
  lightboxKey.value = key;
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
        <div
          ref="scrollContainer"
          class="feed-scroll"
          @scroll="handleScroll"
          @wheel.passive="onWheel"
        >
        <ImageCard
          v-for="item in feed.items"
          :key="item.key"
          :thumb-b64="item.thumb_b64"
          :title="item.title"
          :artist="item.artist"
          :is-bookmarked="item.is_bookmarked"
          :selected="feed.isSelected(item.key)"
          :page="item.page"
          :page-count="item.page_count"
          :expanded="feed.isExpanded(item.illust_id)"
          @toggle="feed.toggleSelect(item.key)"
          @preview="openLightbox(item.key)"
          @expand="feed.toggleExpand(item.illust_id)"
        />
        </div>
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
}

.feed-scroll {
  display: flex;
  gap: 12px;
  overflow-x: auto;
  overflow-y: hidden;
  padding-bottom: 0;
  height: 100%;
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
</style>

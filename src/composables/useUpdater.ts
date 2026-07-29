import { computed, h, ref, shallowRef } from "vue";
import { isTauri } from "@tauri-apps/api/core";
import { confirm } from "@tauri-apps/plugin-dialog";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { NButton, NProgress, useMessage, useNotification } from "naive-ui";
import { useFeedStore } from "../stores/feed";

const STARTUP_CHECK_DELAY_MS = 3000;
const CHECK_TIMEOUT_MS = 30000;

const checking = ref(false);
const installing = ref(false);
const downloadPercent = ref<number | null>(null);
const pendingUpdate = shallowRef<Update | null>(null);

let activeCheck: Promise<Update | null> | null = null;
let promptedVersion: string | null = null;
let startupTimer: number | null = null;

function errorText(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function releaseUpdate(update: Update | null) {
  if (!update) return;
  if (pendingUpdate.value === update) pendingUpdate.value = null;
  void update.close().catch(() => {});
}

export function useUpdater() {
  const message = useMessage();
  const notification = useNotification();

  const updateAvailable = computed(() => pendingUpdate.value !== null);

  async function runCheck(): Promise<Update | null> {
    if (!isTauri()) return null;
    if (pendingUpdate.value) return pendingUpdate.value;
    if (activeCheck) return activeCheck;

    checking.value = true;
    activeCheck = check({ timeout: CHECK_TIMEOUT_MS });

    try {
      const update = await activeCheck;
      if (!update) return null;

      if (pendingUpdate.value && pendingUpdate.value !== update) {
        releaseUpdate(pendingUpdate.value);
      }
      pendingUpdate.value = update;
      return update;
    } finally {
      checking.value = false;
      activeCheck = null;
    }
  }

  function promptUpdate(update: Update) {
    if (promptedVersion === update.version) return;
    promptedVersion = update.version;

    const notes = (update.body ?? "").trim();
    const notice = notification.info({
      title: `发现新版本 v${update.version}`,
      content: notes
        ? `${notes.slice(0, 240)}${notes.length > 240 ? "..." : ""}`
        : "更新已通过数字签名验证，可以下载安装。",
      duration: 0,
      keepAliveOnHover: true,
      onClose: () => {
        promptedVersion = null;
      },
      action: () =>
        h(
          NButton,
          {
            size: "small",
            type: "primary",
            disabled: installing.value,
            onClick: () => {
              notice.destroy();
              void installUpdate();
            },
          },
          { default: () => "立即更新" },
        ),
    });
  }

  async function installUpdate(): Promise<void> {
    const update = pendingUpdate.value;
    if (!update || installing.value) return;

    const feed = useFeedStore();
    if (feed.saving) {
      message.warning("图片正在保存，请等待保存完成后再安装更新");
      return;
    }

    const accepted = await confirm(
      "更新安装时 PixBox 将自动退出。请确认已完成当前操作，再继续安装。",
      {
        title: `安装 PixBox v${update.version}`,
        kind: "warning",
        okLabel: "下载并安装",
        cancelLabel: "稍后",
      },
    );
    if (!accepted) return;

    if (feed.saving) {
      message.warning("图片正在保存，请等待保存完成后再安装更新");
      return;
    }

    installing.value = true;
    downloadPercent.value = 0;
    let totalBytes = 0;
    let downloadedBytes = 0;

    const progress = notification.info({
      title: `正在更新到 v${update.version}`,
      content: () =>
        h("div", { class: "updater-progress" }, [
          h(
            "div",
            downloadPercent.value === null
              ? `已下载 ${(downloadedBytes / 1024 / 1024).toFixed(1)} MB`
              : `已下载 ${downloadPercent.value}%`,
          ),
          h(NProgress, {
            type: "line",
            percentage: downloadPercent.value ?? 0,
            processing: true,
            showIndicator: false,
            height: 4,
          }),
          h("div", { class: "updater-progress-hint" }, "下载完成后应用会退出并安装更新。"),
        ]),
      duration: 0,
      keepAliveOnHover: true,
      closable: false,
    });

    try {
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") {
          totalBytes = event.data.contentLength ?? 0;
          downloadPercent.value = totalBytes > 0 ? 0 : null;
        } else if (event.event === "Progress") {
          downloadedBytes += event.data.chunkLength;
          if (totalBytes > 0) {
            downloadPercent.value = Math.min(
              100,
              Math.round((downloadedBytes / totalBytes) * 100),
            );
          }
        } else {
          downloadPercent.value = 100;
        }
      });

      // Windows exits the app from the updater install step. This is reached on
      // platforms where the app can keep running after installation.
      progress.destroy();
      message.success("更新已安装，请重新启动 PixBox");
      releaseUpdate(update);
    } catch (error) {
      progress.destroy();
      notification.error({
        title: "更新失败",
        content: errorText(error),
        duration: 6000,
      });
    } finally {
      installing.value = false;
      downloadPercent.value = null;
    }
  }

  function checkOnStartup() {
    if (!isTauri() || startupTimer !== null) return;
    startupTimer = window.setTimeout(() => {
      startupTimer = null;
      void runCheck()
        .then((update) => {
          if (update) promptUpdate(update);
        })
        .catch(() => {
          // Startup checks are intentionally silent when offline.
        });
    }, STARTUP_CHECK_DELAY_MS);
  }

  async function checkManually(): Promise<void> {
    if (!isTauri()) {
      message.info("检查更新仅在桌面应用中可用");
      return;
    }

    try {
      const update = await runCheck();
      if (update) {
        promptUpdate(update);
      } else {
        message.success("当前已是最新版本");
      }
    } catch (error) {
      notification.error({
        title: "检查更新失败",
        content: errorText(error),
        duration: 5000,
      });
    }
  }

  return {
    checking,
    installing,
    downloadPercent,
    updateAvailable,
    checkOnStartup,
    checkManually,
    installUpdate,
  };
}

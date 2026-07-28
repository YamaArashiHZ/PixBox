// 前端复刻 src-tauri/src/pixiv_api.rs 中的 build_subdir，
// 用于设置页实时预览最终保存路径。两边逻辑必须保持一致。
export function renderSubdirPattern(pattern: string): string {
  const now = new Date();
  const pad = (n: number, w: number) => String(n).padStart(w, "0");
  const chars = [...pattern];
  let result = "";
  let i = 0;

  while (i < chars.length) {
    if (chars[i] === "%" && i + 1 < chars.length) {
      const c = chars[i + 1];
      // 统计同一字母的连续重复次数
      let len = 1;
      while (i + 1 + len < chars.length && chars[i + 1 + len] === c) len++;

      let token: string | null = null;
      if (c === "y" && len === 2) token = pad(now.getFullYear() % 100, 2);
      else if (c === "y" && len === 4) token = pad(now.getFullYear(), 4);
      else if (c === "m" && len === 1) token = String(now.getMonth() + 1);
      else if (c === "m" && len === 2) token = pad(now.getMonth() + 1, 2);
      else if (c === "d" && len === 1) token = String(now.getDate());
      else if (c === "d" && len === 2) token = pad(now.getDate(), 2);
      else if (c === "H" && len === 1) token = String(now.getHours());
      else if (c === "H" && len === 2) token = pad(now.getHours(), 2);
      else if (c === "M" && len === 1) token = String(now.getMinutes());
      else if (c === "M" && len === 2) token = pad(now.getMinutes(), 2);
      else if (c === "S" && len === 1) token = String(now.getSeconds());
      else if (c === "S" && len === 2) token = pad(now.getSeconds(), 2);

      if (token !== null) {
        result += token;
        i += 1 + len;
      } else {
        // 未识别的占位符，原样输出
        result += "%";
        i += 1;
      }
    } else {
      result += chars[i];
      i += 1;
    }
  }

  return result.replace(/[\\/]/g, "_").replace(/\.\./g, "_");
}

// 拼接示例保存路径：base + 可选子文件夹 + 示例文件名
export function buildPathPreview(
  baseDir: string,
  useSubdir: boolean,
  pattern: string,
  fileName: string
): string | null {
  if (!baseDir) return null;
  const parts = [baseDir.replace(/[\\/]+$/, "")];
  if (useSubdir && pattern) parts.push(renderSubdirPattern(pattern));
  return parts.join("\\") + "\\" + fileName;
}

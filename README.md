# PixBox

Pixiv 每日存图助手 — Windows 桌面应用。浏览关注/推荐流、筛选作品、一键保存原图并自动压缩备份。

**技术栈：** Tauri 2 + Vue 3 + Rust

## 免责声明

- PixBox 是**非官方第三方客户端**，与 pixiv Inc. 无隶属、赞助或官方授权关系。
- 本项目使用了 Pixiv 未公开保证稳定性的 App API 和 Web AJAX 接口，**可能随时失效**。
- 用户应遵守 [Pixiv 服务条款](https://www.pixiv.net/terms.php)、当地法律及作品权利人的要求。
- **不鼓励**批量抓取、规避访问控制、高频请求或任何违反 Pixiv Guidelines 的行为。

## 凭据安全

- OAuth refresh token 和 Web 会话 Cookie（PHPSESSID）保存在 **Windows 凭据管理器**中，不以明文写入文件。
- 系统密钥库**不能防御**已控制本机用户会话的恶意软件。
- 卸载或不再使用前，建议先在应用内**退出登录**以清除本地凭据。
- 内置的 `CLIENT_ID` / `CLIENT_SECRET` 是 Pixiv Android 客户端的公开常量，并非本项目的私有密钥。

## 隐私

- 本项目**不收集**任何用户数据，不上传凭据至开发者服务器。
- 所有网络请求由本机直接发送至 Pixiv 或用户配置的代理。
- 问题反馈中**不得附带** token、Cookie、OAuth 回调 URL、`tokens.json` 旧文件或包含私人数据的日志。

## 构建

```powershell
npm ci
npm run check:versions
npm run build
cargo test --locked --manifest-path src-tauri/Cargo.toml
```

## 发布

项目使用 Release Please 维护 Release PR。提交消息需遵循 Conventional Commits，例如 `fix: ...`、`feat: ...` 或包含破坏性变更的 `feat!: ...`。合并 Release PR 后，GitHub Actions 会执行以下步骤：

1. 创建带 `v` 前缀的 Git tag 和草稿 GitHub Release。
2. 在 `release` Environment 中读取更新签名密钥，构建 Windows x64 NSIS 安装包。
3. 校验安装包、签名和 `latest.json` 完整且版本一致。
4. 仅在全部校验通过后公开 Release，并设为 latest。

若构建或上传失败，Release 会保持草稿状态。修复配置后可手动运行 `Release` workflow，并在 `release_tag` 中填写现有草稿标签以安全重试；工作流只接受可从 `master` 到达的标签。

首次启用发布前需要完成以下配置：

1. 将仓库推送到 `YamaArashiHZ/PixBox`，默认分支保持为 `master`。
2. 在 GitHub 仓库的 `Settings > Environments` 创建名为 `release` 的 Environment。
3. 在该 Environment 中添加 `TAURI_SIGNING_PRIVATE_KEY` 和 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 两个 secret。
4. 在 `Settings > Actions > General` 启用 GitHub Actions 创建 Pull Request 的权限。

建议为 `release` Environment 配置 required reviewer。签名私钥只应存在于离线备份和 GitHub Environment secrets 中，不能提交到仓库。

### 更新签名密钥

在 PowerShell 中生成独立的 Tauri 更新密钥：

```powershell
npm run tauri -- signer generate -w "$HOME\.tauri\pixbox.key"
```

命令会创建私钥 `pixbox.key` 和公钥 `pixbox.key.pub`。随后：

1. 将 `pixbox.key` 的完整内容写入 Environment secret `TAURI_SIGNING_PRIVATE_KEY`。
2. 将生成时设置的密码写入 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。
3. 将 `pixbox.key.pub` 的完整内容写入 `src-tauri/tauri.conf.json` 的 `plugins.updater.pubkey`。
4. 运行 `npm run check:versions -- --release` 验证发布配置。

公钥可以公开；私钥丢失后，已安装版本将无法验证后续更新，因此必须保留安全的离线备份。Tauri 更新签名密钥只负责更新包完整性，不替代 Windows Authenticode 代码签名证书。

## 许可

MIT

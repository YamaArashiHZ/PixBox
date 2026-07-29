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
npm install
npm run build
```

## 许可

MIT

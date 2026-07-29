# Changelog

## [0.3.0](https://github.com/YamaArashiHZ/PixBox/compare/v0.2.2...v0.3.0) (2026-07-29)


### Features

* 替换应用图标与侧边栏品牌标识 ([1493954](https://github.com/YamaArashiHZ/PixBox/commit/14939541934f9d003ced1553fe8bff115d40db3a))

## [0.2.2](https://github.com/YamaArashiHZ/PixBox/compare/v0.2.1...v0.2.2) (2026-07-29)


### Bug Fixes

* 校验更新清单中的 GitHub 资产地址 ([60e6250](https://github.com/YamaArashiHZ/PixBox/commit/60e62504b9c9d0ab6be163eacd4ef52aa24f86f3))

## [0.2.1](https://github.com/YamaArashiHZ/PixBox/compare/v0.2.0...v0.2.1) (2026-07-29)


### Bug Fixes

* 修复发布版本同步与草稿解析 ([0b2d8f0](https://github.com/YamaArashiHZ/PixBox/commit/0b2d8f0d93d01a7d3fa177a48487f333ddd1d3a4))
* 允许发布任务读取草稿版本 ([475a208](https://github.com/YamaArashiHZ/PixBox/commit/475a208893eeb71f85aa1aabbdcb498e482d53e3))
* 允许手动重试发布构建 ([a89259f](https://github.com/YamaArashiHZ/PixBox/commit/a89259fe7cc1afb47928316eb97b223facdd64bf))

## [0.2.0](https://github.com/YamaArashiHZ/PixBox/compare/v0.1.0...v0.2.0) (2026-07-29)


### Features

* 保存下载升级为流式+字节级进度追踪，新增清空选中按钮 ([044a874](https://github.com/YamaArashiHZ/PixBox/commit/044a87429cc753fe75a764d28cc78bab1e950331))
* 保存路径支持日期占位符子目录模板，设置页UI重构+实时预览 ([5358eb0](https://github.com/YamaArashiHZ/PixBox/commit/5358eb0db188fad1046faf229cf5cfaef5892d49))
* 凭据迁移至系统密钥库，Token刷新容错加固，添加README ([269f66c](https://github.com/YamaArashiHZ/PixBox/commit/269f66c7bc68132f121e95343d2af1c63740d6f3))
* 卡片显示AI生成/R-18标签徽章，R18模式合并推荐+日榜数据源 ([3fd74ea](https://github.com/YamaArashiHZ/PixBox/commit/3fd74ea7cc3179910f380d91601ec23c8ba62484))
* 多图帖展开组整帖选择与描边画框布局 ([08bbfd7](https://github.com/YamaArashiHZ/PixBox/commit/08bbfd7538a8c05722de3d9518f723b8eb7c7557))
* 推荐流切换至Web Discovery AJAX，与pixiv.net网页端推荐同源 ([5ffb6b0](https://github.com/YamaArashiHZ/PixBox/commit/5ffb6b00d69cd8239ed931e78d41661b812bf900))
* 推荐页R18模式接入Pixiv日榜API，全部模式推荐+日榜合并去重 ([b84da66](https://github.com/YamaArashiHZ/PixBox/commit/b84da668b5cf355ce001772b7e481dd961139315))
* 支持在应用内切换Pixiv收藏状态（收藏/取消收藏） ([348eb0d](https://github.com/YamaArashiHZ/PixBox/commit/348eb0d43d3c9861c8f39577a381e3af585f02fe))
* 添加在浏览器中打开原帖按钮，右键退出灯箱 ([34c1a7d](https://github.com/YamaArashiHZ/PixBox/commit/34c1a7da6798f231cb6c2da882b2b6749c96146b))
* 添加签名自动更新与发布流程 ([986eb68](https://github.com/YamaArashiHZ/PixBox/commit/986eb68d8f7119e0a729a963c5ea3977acb4a449))
* 选中托盘添加/移除项增加平滑过渡动画 ([61d19ea](https://github.com/YamaArashiHZ/PixBox/commit/61d19ea8a89b530dc089b724356997b68946a5de))


### Bug Fixes

* 切换推荐内容时重置滚动位置 ([0abc1cf](https://github.com/YamaArashiHZ/PixBox/commit/0abc1cfd74d001bce586daabc8d3b6481ce9fe44))
* 封面卡改为第1页就地转换，镜像子卡绑定真实key消除重复 ([1986aa5](https://github.com/YamaArashiHZ/PixBox/commit/1986aa5aec4c6a8091e2a3fe5bd2c8397bf94e8f))
* 强制压缩图独立保存，新增路径冲突检测与保存拦截 ([ba68159](https://github.com/YamaArashiHZ/PixBox/commit/ba681599aa1a387517a25dc8c4e78aaf2f6e0156))

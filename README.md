# EasyGot · 学有所获

Android 本地优先答题与 AI 学习工具。题库、教材原文、作答、笔记、错题和进度存储在设备上；AI、图片识别与联网搜索按需连接自己的接口。

**状态：0.1.0-alpha.1，首个功能预览，尚需 Android 真机和真实服务联调。** 本项目为独立实现，没有复制其他题库项目的源码。GPL-3.0-only。

## 获取安装包

- [GitHub Actions 构建记录](https://github.com/weimanmk/EasyGot/actions)：成功的构建在 Artifacts 中提供 `EasyGot-preview`。
- [Releases](https://github.com/weimanmk/EasyGot/releases)：推送 `v*` 标签自动发布。
- Android 8.0+。预览包使用独立的 `io.github.weimanmk.easygot.preview` 包名。
- 未配置正式签名时只发布明确标注的**测试签名预览包**，不会伪装为正式签名版本。测试签名跨构建不保证相同，更新前请导出备份。

## 已实现的流程

- JSON 标准题库导入、编辑、预览、导出，题目图片随 JSON 导出；[导入规范](docs/QUESTION_FORMAT.md)、[示例](examples/sample-bank.json)。
- 单选、多选、判断、简答、论述、计算；逐题提交、计时模拟考试、章节/知识点搜索后练习、最近错题复习。
- 主观题文字输入或选择/拍摄手写照片，联网调用同一个视觉模型评分。支持语义等价和部分分；题目/教材严格要求单独提供给模型。
- PDF、DOCX、TXT、Markdown 原文提取和本地关键词检索。DOCX 内嵌位图、PDF 页面可显式调用视觉模型识别；按页保存，可从中断处继续。
- 教材、考纲、往年试题用途区分；AI 问答优先本地资料，保留文件及页码/段落位置。
- 轻量 Agent 工具：本地资料检索、最近错题读取、DeepSeek Search、DeepSeek Fetch。无脚本执行及任意文件写入工具，最多6轮。生成结果需用户确认才写入。
- 按题型、题量、章节、知识点、难度、考纲权重及错题生成新题；校验配比，预览编辑后入库，AI 新题保留来源标记。
- HTTPS WebDAV：题库、教材原文件、图片、作答记录、会话、笔记、复习计划双向合并；并发修改保留为冲突，由用户选择或编辑合并。
- 本地完整 JSON 备份和恢复，历史版本恢复；API Key、搜索 Key、WebDAV 凭据使用 Android Keystore 本机加密，不进入备份和同步。

## 初次使用

1. 打开「学习」载入示例，或从「题库」导入标准 JSON。
2. 在「资料」导入教材/考纲/往年试题，勾选要参考的资料。未勾选时问答检索全部资料；出题建议明确勾选。
3. 「设置」配置 OpenAI 兼容 API Base URL、Key、模型名称。Base URL 如 `https://example.com/v1`；模型须同时支持图片输入和工具调用。仅有文本能力的模型不能识图，工具调用不兼容时 Agent 会报错。
4. DeepSeek 搜索单独配置 Key，默认 Base URL `https://api.deepseek.com`。只在助教页面勾选联网后启用。
5. 可选配置已有 WebDAV 根目录、用户名、应用密码；测试后立即同步。每台设备都需独立配置凭据。

## 构建与签名

JDK 17、Gradle 8.11.1、Android SDK 35 / Build Tools 35.0.0：

```bash
gradle testDebugUnitTest lintDebug assembleDebug
```

GitHub Actions 已配置工具链，不要求本机有 Android Studio。正式签名在仓库 Settings → Secrets and variables → Actions 添加：

| Secret | 含义 |
| --- | --- |
| `ANDROID_KEYSTORE_BASE64` | 自己持有并备份的 keystore 的 Base64 |
| `ANDROID_STORE_PASSWORD` | keystore 密码 |
| `ANDROID_KEY_ALIAS` | 签名别名 |
| `ANDROID_KEY_PASSWORD` | 私钥密码 |

不要提交 keystore 或密码。Release 发布权限由 workflow 的 `contents: write` 提供。打标签前先检查 CI 通过，再执行 `git tag v0.1.0-alpha.1 && git push origin v0.1.0-alpha.1`。

## 当前边界

- 未内置离线大模型。离线主观题先保存待评分，联网后手动批改。普通题目不会无提示发送到第三方。
- AI 评分为参考。引用验证能核对材料/位置标识，不能保证模型对教材的解释准确，应用会展示参考答案供复核。
- 首版原文检索为关键词及中文二元词召回，没有向量库、重排模型；大题库/大教材需要继续性能测试。
- Word 仅 DOCX（旧 DOC 请转换）。内嵌 PNG/JPEG 等位图支持识别；EMF/WMF 等格式会显示不支持提示。Word 图片定位到图片名，尚未绑定精确正文段落。
- PDF 文字提取最多500页，视觉识别最多120页；选取文件最多30MB。图片识别由用户显式启动，可能产生多次 API 费用。
- 手机图片选择器是否提供「拍摄」入口取决于系统；如无入口，可先用相机拍照后选择照片。首版每题一张答案图片，可先拼图。
- WebDAV 是追加日志+素材同步，首版每次上传完整版本历史，无增量压缩/垃圾回收；不做云端删除。需服务端支持 MKCOL、PROPFIND Depth:1、HEAD、GET、PUT。日志50MB、素材60MB上限，完整JSON备份素材合计60MB。
- 自动同步每6小时仅不计费网络尝试；Android 系统可延后执行，重启后需再次打开设置启用调度。不是实时协作。
- 预览 UI 为原生轻量实现；题目编辑暂用带校验的 JSON 编辑器。数学公式保存为文本，未提供公式排版编辑器。

## 文档

- [需求规格](docs/REQUIREMENTS.md)
- [架构与同步](docs/ARCHITECTURE.md)
- [题库格式](docs/QUESTION_FORMAT.md)
- [DeepSeek Search / Fetch 协议](docs/DEEPSEEK_SEARCH.md)
- [测试与验收](docs/TESTING.md)

设计参考：[DeepStudent](https://github.com/helixnow/deep-student)、[Shiroha Quiz](https://github.com/reiqr/shiroha-quiz)、[Exameow](https://github.com/heshengtao/exameow)。依赖许可证详见 [THIRD_PARTY_NOTICES](THIRD_PARTY_NOTICES.md)。

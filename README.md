# EasyGot · 学有所获

面向个人多设备使用的Android答题与AI学习工具。**学习业务与主界面使用Rust实现**；完整AI底座复用 [deepseek-harness-mobile](https://github.com/Ajwyunsx/deepseek-harness-mobile) 的 proot Ubuntu + Node.js Harness 方案，Android只承担系统桥接和上游AI运行环境承载。

**状态：0.1.0-alpha.1，开发预览，需真机及真实模型/WebDAV服务验收。GPL-3.0-only。** AI底座源代码按MIT许可复用，原始许可和版本记录保留于 `vendor/deepseek-harness-mobile/`。

## 安装与构建

- [Actions](https://github.com/weimanmk/EasyGot/actions)：通过后下载 `EasyGot-preview`。
- [Releases](https://github.com/weimanmk/EasyGot/releases)：`v*` 标签触发构建发布。
- 首版Android 8.0+、ARM64。测试包使用独立 `.preview` 包名。
- 未配置正式签名Secrets时，只发布明确标注的测试签名预览；跨构建更新前请备份。
- 完整Harness环境按上游流程在首次使用时下载，约200MB及后续依赖；APK体积与运行环境占用应分别计算。

## 学习功能

- 标准JSON题库导入、预览、编辑、导出；[字段规范](docs/QUESTION_FORMAT.md)、[示例](examples/sample-bank.json)。
- 单选、多选、判断、简答、论述、计算；逐题提交、计时整卷考试、章节/知识点筛选、最近错题复习。
- 主观题文字或手写照片，视觉模型语义评分与部分分；题目及教材的严格术语/步骤/公式要求优先。
- PDF、DOCX、UTF-8 TXT/Markdown原文导入、本地检索；文档图片及PDF页面显式调用视觉模型识别，逐页保存。
- 教材、考纲、往年试题分别标注；解释优先教材，记录文件与页码/段落引用。外部知识通过独立DeepSeek Search/Fetch接入。
- 按题型、题量、难度、章节、考纲知识点权重及错题生成变式；核对来源和配比后预览确认入库。
- WebDAV同步题库、教材、图片、作答、错题、会话、笔记和计划；HTTPS传输、云端文件不加密；并发冲突保留双方。
- 完整JSON备份、版本撤销。模型/搜索/WebDAV凭据由Android Keystore加密，仅存本机，不随学习数据同步。

## 完整AI基座如何服务学习

复用上游运行时、模型接入、持久会话、工具编排、上下文管理、移动WebView和前台保活。EasyGot去除通用终端/文件管理入口、SSH自动启动和上游APK自动更新；Rust通过本机MCP提供：

| 工具 | 用途 |
| --- | --- |
| `search_materials` | 教材/考纲/往年试题原文检索与引用 |
| `get_wrong_questions` | 错题归因、薄弱点分析 |
| `grade_answer` | 文字/图片主观题评分与详解 |
| `deepseek_search` / `deepseek_fetch` | 联网补充知识、保留外部来源 |
| `propose_question_bank` | 创建待确认新题草稿，不能直接改正式题库 |
| `save_study_plan` | 创建可撤销复习计划 |

使用流程：

1. 导入题库和教材；设置中配置自己的视觉评分模型、DeepSeek搜索和可选WebDAV。
2. 「助教」→「打开AI助教」，完成上游环境安装及Harness模型配置。
3. 返回EasyGot「助教」→「安装/更新学习工具到Harness」，成功后重启助教服务刷新工具。
4. 在AI助教提出学习任务；新题草稿回EasyGot助教页预览、编辑、确认。

原文检索与已有题库不依赖Harness安装；首次环境安装可延后。评分模型和Harness对话模型配置分开，前者须支持图片。基础评分/出题可直接调用自己的OpenAI兼容视觉模型。

## 技术与签名

Rust + egui/eframe、SQLite、HTTPS原生客户端；Android NativeActivity + SAF/PdfRenderer/Keystore桥接。上游运行时独立Android library模块，固定快照来源在 `UPSTREAM_COMMIT`。

```bash
cargo test --lib
cargo ndk -t arm64-v8a -p 26 -o app/src/main/jniLibs build --release
gradle lintDebug assembleDebug
```

需要Rust稳定版、cargo-ndk、NDK28.2、JDK17、Gradle8.11.1、SDK35。Actions自动配置。

正式签名Secrets：`ANDROID_KEYSTORE_BASE64`、`ANDROID_STORE_PASSWORD`、`ANDROID_KEY_ALIAS`、`ANDROID_KEY_PASSWORD`。用户自行持有并备份私钥，不提交到仓库。推送`v*`标签后Release上传APK及SHA256校验值。

## 当前边界

- 为复用proot在私有目录执行程序的方案，沿用上游targetSdk28；当前通过GitHub侧载分发，不承诺应用商店发布。Rust共享库按16KB页面对齐，但上游proot与下载环境的16KB设备兼容性仍须验证。
- 完整Harness仍具有上游通用工具能力；已移除通用终端/文件管理UI与SSH自动启动。学习MCP工具只允许草稿式改题，但这不等同于对完整容器建立强制沙箱。后续可继续按学习任务裁剪配置。
- 首版ARM64；旧DOC需转DOCX。Word内嵌位图支持识别，EMF/WMF提示转换；图片位置以媒体文件名定位。UTF-8文本暂不支持GBK自动识别。
- 单文件30MB，完整备份素材合计60MB，PDF原文最多500页、视觉识别120页。首版每题一张答案图片。
- 首版原文检索为关键词召回；不是向量RAG或可靠性保证。AI评分及引用需与原文核对。
- WebDAV追加日志保留完整历史，不做增量压缩或远端垃圾回收。首版Rust迁移后提供手动同步；后台定时同步后续补齐。日志50MB/素材60MB上限。
- 题目与出题参数编辑先采用JSON编辑器。公式按文本显示，界面和输入法须真机验证。

[需求](docs/REQUIREMENTS.md) · [架构](docs/ARCHITECTURE.md) · [Harness整合](docs/HARNESS.md) · [DeepSeek协议](docs/DEEPSEEK_SEARCH.md) · [测试清单](docs/TESTING.md)

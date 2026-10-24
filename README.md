# EasyGot · 学有所获

Android个人多设备学习软件，业务、主界面与内置Agent引擎均使用Rust。少量Android桥接仅用于文件选择、Keystore和PDF页面渲染。GPL-3.0-only。

**0.1.0-alpha.1开发预览，尚需真机和真实服务验收。**

[构建记录](https://github.com/weimanmk/EasyGot/actions) · [安装包](https://github.com/weimanmk/EasyGot/releases)

## 一个软件，一套配置

没有DSH应用、proot/Ubuntu、Node环境、独立WebView、MCP或插件安装流程。参考DeepSeek Harness的Agent机制，在Rust内部实现学习工具调度。

设置中填写统一模型地址、Key、模型名，讲解、评分、识图、出题和Agent共用。DeepSeek搜索默认复用适用的DeepSeek官方Key；使用第三方模型时可在高级设置覆盖搜索Key。第三方模型Key不会自动发送到DeepSeek。

## 功能

- JSON标准题库导入导出、编辑与预览，见[格式规范](docs/QUESTION_FORMAT.md)、[示例](examples/sample-bank.json)。
- 单选、多选、判断、简答、论述、计算题；逐题练习、计时考试、章节/知识点筛选、错题复习。
- 主观题文字/手写图片作答，统一视觉模型评分、识图与解析，允许语义等价与部分分，严格要求优先。
- PDF、DOCX、UTF-8 TXT/Markdown教材、考纲、往年试题导入；原文离线检索，文档图片/PDF页面可逐页识别。
- 内部Agent：多轮任务、持久事件、近期会话记忆、工具结果裁剪、重复调用检测、步骤取消、24步上限。
- 原生学习工具直接调用教材检索、错题读取、DeepSeek搜索/抓取、新题草稿、笔记修改草稿、复习计划；没有通用shell/任意文件工具。
- 按考纲权重、题型、题量、难度和错题生成变式；新题与笔记修改预览确认后写入。引用保留出处。
- HTTPS WebDAV同步全部学习数据和图片；云端明文，冲突保留双方。完整JSON备份与历史撤销。
- 凭据由Android Keystore本机加密，不进入题库备份与同步。

## 使用

载入示例或导入JSON；在资料页导入教材/考纲/往年试题并勾选；设置统一模型后直接在助教页提问。无额外AI安装步骤。图片识别和评分需要模型支持视觉输入。

## 构建与发布

Android8.0+、ARM64。Rust稳定版、cargo-ndk3.5.4、NDK28.2、JDK17、Gradle8.11.1、SDK35。Actions自动配置。

```bash
cargo test --lib
cargo ndk -t arm64-v8a -p 26 -o app/src/main/jniLibs build --release
gradle lintDebug assembleDebug
```

正式签名Secrets：ANDROID_KEYSTORE_BASE64、ANDROID_STORE_PASSWORD、ANDROID_KEY_ALIAS、ANDROID_KEY_PASSWORD。用户持有并备份私钥，不提交仓库。v*标签触发Release。没有正式签名时仅发布明确标注的测试签名预览，包名独立为.preview，跨构建更新前请备份。

## 当前边界

- 需真机、中文输入法、真实模型/识图和双设备WebDAV联调；CI不代表体验验收。
- 离线主观题先保存待评分，无内置离线大模型；AI评分及引用需对照原文。
- 原文检索为关键词召回，尚非向量RAG。Word仅DOCX，文本UTF-8；EMF/WMF需转换。每题一张答案图片。
- 单文件30MB、备份素材合计60MB，PDF最多500页，视觉识别120页。
- WebDAV暂手动同步，上传完整历史，无压缩/分片/远端清理；日志50MB、单素材60MB上限。
- 题目与出题参数暂用JSON编辑器，公式按文本显示。
- Agent取消在当前网络请求返回或步骤边界生效；上下文保留近期对话并裁剪工具输出，尚无完整语义摘要压缩。

[Agent设计](docs/HARNESS.md) · [需求](docs/REQUIREMENTS.md) · [架构](docs/ARCHITECTURE.md) · [DeepSeek协议](docs/DEEPSEEK_SEARCH.md)

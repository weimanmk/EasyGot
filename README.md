> 0.3前端重构正在 `refactor/slint-mobile` 分支进行：Slint原生布局、Rust控制器/作答状态分层、结构化题目与出题表单。尚未作为稳定版验收。[源码对照](docs/SOURCE_REVIEW.md) · [迭代记录](docs/ITERATION_LOG.md) · [验收状态](design-qa.md)。

# EasyGot · 学有所获

Android个人多设备学习软件，业务、主界面与内置Agent引擎均使用Rust。少量Android桥接仅用于文件选择、Keystore、原生粘贴和PDF排版/渲染。GPL-3.0-only。

**0.2.0-alpha.1 开发预览，尚需真机和真实服务验收。**

[构建记录](https://github.com/weimanmk/EasyGot/actions) · [安装包](https://github.com/weimanmk/EasyGot/releases)

## 一个软件，一套配置

没有DSH应用、proot/Ubuntu、Node环境、独立WebView、MCP或插件安装流程。参考DeepSeek Harness的Agent机制，在Rust内部实现学习工具调度。

设置中填写统一模型地址、Key、模型名，讲解、评分、识图、出题和Agent共用。DeepSeek搜索默认复用适用的DeepSeek官方Key；使用第三方模型时可在高级设置覆盖搜索Key。第三方模型Key不会自动发送到DeepSeek。

## 功能

- 蓝色图标、大圆角卡片、悬浮底栏、适配系统状态栏；助教改为可拖动悬浮球，自动带入当前题目或资料。
- API Key支持粘贴、显示/隐藏与Android原生输入框长按粘贴。
- 题库与教材/考纲/往年试题持久关联，关联内容用于评分、讲解与出题，并随WebDAV同步。
- 整页/多页照片识别客观题和主观题，生成可编辑草稿，缺失答案需补全后入库。
- 导出A4 PDF作答卷和独立答案解析卷；纸上作答计时、照片按固定题号识别、核对后判分、保存进度与错题解析。
- 多套卷目录：一个JSON可含10套或更多套题，支持文件夹、套卷题序和时长；按套练习、考试、导出PDF。见[10套题示例](examples/multi-paper-bank.json)。
- 系统中英文字体回退与较紧凑字号，正文/按钮14、辅助文字12–13。
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

载入示例或导入JSON；在资料页导入教材/考纲/往年试题，再从题库的「关联资料」绑定。设置统一模型后点击蓝色悬浮球提问。

纸上答题：题库筛选后点「导出纸上试卷」创建试卷，生成并保存/打印PDF；在App内开始计时，完成后上传整页/多页照片。逐题核对题号、选项和文字，确认后保存成绩。客观题按标准答案评分，主观题在联网后点击AI批改。纸卷上的手写开始/结束时间不会自动替代App计时。无额外AI安装步骤。图片识别和评分需要模型支持视觉输入。

## 构建与发布

Android8.0+、ARM64。Rust稳定版、cargo-ndk3.5.4、NDK28.2、JDK17、Gradle8.11.1、SDK35。Actions自动配置。

```bash
cargo test --lib
python3 scripts/prepare_fonts.py
cargo ndk -t arm64-v8a -p 26 -o app/src/main/jniLibs build --release
gradle lintDebug assemblePreview
```

正式签名Secrets：ANDROID_KEYSTORE_BASE64、ANDROID_STORE_PASSWORD、ANDROID_KEY_ALIAS、ANDROID_KEY_PASSWORD。用户持有并备份私钥，不提交仓库。v*标签触发Release。没有正式签名时仅发布明确标注的测试签名预览，包名独立为.preview，预览安装包关闭调试与模拟器测试入口，使用测试密钥签名。0.2开始由Actions缓存预览签名；从0.1升级可能需要先导出备份、卸载旧预览包再安装。正式签名仍应由你配置和备份Secrets。

## 当前边界

- 需真机、中文输入法、真实模型/识图和双设备WebDAV联调；CI不代表体验验收。
- 离线主观题先保存待评分，无内置离线大模型；AI评分及引用需对照原文。
- 原文检索为关键词召回，尚非向量RAG。Word仅DOCX，文本UTF-8；EMF/WMF需转换。逐题作答每题一张答案图片；整页识题和纸卷上传一次最多12张。
- 单文件30MB、备份素材合计60MB，PDF最多500页，视觉识别120页。
- WebDAV暂手动同步，上传完整历史，无压缩/分片/远端清理；日志50MB、单素材60MB上限。
- 题目与出题参数暂用JSON编辑器，公式按文本显示。
- Agent取消在当前网络请求返回或步骤边界生效；上下文保留近期对话并裁剪工具输出，尚无完整语义摘要压缩。

[Agent设计](docs/HARNESS.md) · [需求](docs/REQUIREMENTS.md) · [架构](docs/ARCHITECTURE.md) · [DeepSeek协议](docs/DEEPSEEK_SEARCH.md)

构建前安装 `fonttools==4.61.1` 并运行 `python3 scripts/prepare_fonts.py`，生成带校验清单的内置中文字体。运行时无需下载字体。

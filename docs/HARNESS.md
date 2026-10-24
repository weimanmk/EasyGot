# 完整Harness整合

按用户最新要求，AI基座不追求轻量化。直接复用Ajwyunsx/deepseek-harness-mobile的Android运行环境代码和版本对齐资源，保留MIT许可证。Rust主程序承载学习界面与数据；Android bridge仅调用系统API和启动运行时。

上游固定快照：`vendor/deepseek-harness-mobile/UPSTREAM_COMMIT`。复用目录`dsh-mobile/app/src/main`，不复制上游公开调试签名密钥。Android包装改为独立library；应用正式签名完全由EasyGot仓库Secrets控制。

## 已裁剪

- 移除主WebView的通用文件管理/终端悬浮入口和启动失败页终端按钮。
- 停止SSH自动安装/启动路径，不对外启动SSH端口。
- 去掉上游APK更新检查，避免安装上游独立应用覆盖EasyGot。
- 主manifest不申请任意共享存储访问。题库与教材通过Android SAF导入。

现有设置/容器工具类中仍有上游通用能力，后续可继续裁剪。复用完整运行环境不等于重写完整Harness，也不等于将其Node代码伪称为Rust。

## 学习工具接入

Rust `harness.rs` 提供带本机随机Bearer token的MCP Streamable HTTP服务器（127.0.0.1:38641/mcp）。token仅存设备私有目录，不进入题库、备份、WebDAV。服务在应用进程内运行，Harness前台服务维持同一进程生命周期。

应用生成`/home/dsh/easygot-learning-tools`本地bundle，使用与上游固定版本相同的`@deepseek-ai/dsh-mcp-client@0.2.1-rc.2`，由现有PluginInstaller安装。工具返回原文引用，改题只创建`bankDraft`，用户需回主界面确认入库。独立学习工具权限不会给完整容器提供硬沙箱；该区别须保留。

评分模型在EasyGot设置中配置视觉接口；Harness对话模型按上游设置管理。DeepSeek Search和Fetch独立配置，官方请求字段见DEEPSEEK_SEARCH.md。

## 人工验收

首次下载安装、国内镜像可用性、Node/Harness版本对齐、学习bundle安装与MCP握手、后台运行、中文输入和16KB页面设备兼容性，都必须真机测试；CI仅能证明编译和核心逻辑测试结果。

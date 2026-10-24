# Rust原生架构

domain.rs：题库和蓝图；store.rs：SQLite版本DAG、冲突与素材；materials.rs：教材提取和搜索；network.rs：HTTPS、DeepSeek；ai.rs：统一模型评分、识图、出题；agent.rs：内部Agent事件和学习工具；sync.rs：WebDAV；ui.rs：egui主界面。platform.rs和小型Android桥接只负责SAF、Keystore、PDF渲染。

无DSH运行时、容器、独立WebView、插件安装或MCP。工具直接调用Rust函数。凭据与学习数据分离。

同步为不可变版本日志和SHA256素材；先验证素材再合并记录，校验父链和环，保留并发头，由用户显式解决冲突。首版上传完整历史，无压缩/分片/远端清理。

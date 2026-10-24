# Rust原生架构

domain.rs：题库和蓝图；store.rs：SQLite版本DAG、冲突与素材；materials.rs：教材提取和搜索；network.rs：HTTPS、DeepSeek；ai.rs：统一模型评分、识图、出题；agent.rs：内部Agent事件和学习工具；sync.rs：WebDAV；learning.rs：题库关联、纸卷快照/题号/计时/确认作答；vision.rs：整页识题与纸卷答案转写；design.rs：样式令牌；ui.rs：egui主界面与悬浮助教。platform.rs和小型Android桥接负责SAF、Keystore、系统剪贴板/原生密钥输入、PDF排版/打印/页面渲染。

无DSH运行时、容器、独立WebView、插件安装或MCP。工具直接调用Rust函数。凭据与学习数据分离。

同步为不可变版本日志和SHA256素材；先验证素材再合并记录，校验父链和环，保留并发头，由用户显式解决冲突。首版上传完整历史，无压缩/分片/远端清理。

题库关联为独立bank记录，与单题materialIds合并后用于练习快照和评分。纸卷paper保存固定题目快照及题号，多次上传以submissionId保留各次作答；原照片是内容寻址素材，随备份与WebDAV同步。识别阶段只转写学生答案，确认后客观题使用确定性标准答案评分，主观题排队调用统一模型。

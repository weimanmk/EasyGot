# 第三方依赖与设计参考

EasyGot采用GPL-3.0-only。Agent设计参考MIT许可的DeepSeek Harness Mobile/DeepSeek Harness，参考版本和许可证在vendor中留存，不包含其容器、DSH产品或Java运行时实现。

Rust依赖包括Slint 1.18.1（本项目选用GPL-3.0-only条款，其Android渲染依赖包括BSD许可的Skia）、android-activity、jni、serde、serde_json、anyhow、uuid、sha2、base64、ureq、url、quick-xml、zip、image、pdf-extract，各自保留发行包许可证；rusqlite为MIT，SQLite为公共领域。Android系统桥接不打包Android SDK。

源码审查参考DeepStudent、Shiroha Quiz和Exameow，固定提交、阅读文件与借鉴边界见docs/SOURCE_REVIEW.md。本轮没有直接复制这三个项目的实现代码。旧egui/eframe界面已移除。

应用内嵌的 EasyGot Sans SC Regular/Bold 字体由 Noto Sans CJK SC 改名而来，遵循 SIL Open Font License 1.1。原始版权、许可及保留字体名称记录在字体文件内；完整许可见 `design/fonts/OFL-Noto-CJK.txt`。上游固定版本、校验和与可复现的改名脚本见 `design/fonts/README.md`，字体不会在运行时联网下载。

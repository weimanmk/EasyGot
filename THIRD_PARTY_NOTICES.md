# 第三方依赖与设计参考

EasyGot采用GPL-3.0-only。Agent设计参考MIT许可的DeepSeek Harness Mobile/DeepSeek Harness，参考版本和许可证在vendor中留存，不包含其容器、DSH产品或Java运行时实现。

Rust依赖包括Slint 1.18.1（本项目选用GPL-3.0-only条款，其Android渲染依赖包括BSD许可的Skia）、android-activity、jni、serde、serde_json、anyhow、uuid、sha2、base64、ureq、url、quick-xml、zip、image、pdf-extract，各自保留发行包许可证；rusqlite为MIT，SQLite为公共领域。Android系统桥接不打包Android SDK。

源码审查参考DeepStudent、Shiroha Quiz和Exameow，固定提交、阅读文件与借鉴边界见docs/SOURCE_REVIEW.md。本轮没有直接复制这三个项目的实现代码。旧egui/eframe界面已移除。

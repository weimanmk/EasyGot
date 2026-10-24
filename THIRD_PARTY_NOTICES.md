# 第三方代码与依赖

- DeepSeek Harness Mobile：MIT，Ajwyunsx/deepseek-harness-mobile，固定来源在vendor/deepseek-harness-mobile/UPSTREAM_COMMIT，完整许可在同目录LICENSE。复制其Android运行环境与对齐资源，裁剪通用终端/文件入口、SSH自动启动和更新检查。上游DeepSeek Harness及Node/npm依赖保留各自许可证。
- egui/eframe、android-activity、jni、serde、serde_json、anyhow、uuid、sha2、base64、ureq、url、quick-xml、zip、image、tiny_http等Rust依赖：各自MIT/Apache-2.0或兼容许可证，以Cargo.lock解析版本与对应发行包LICENSE为准。
- rusqlite：MIT；SQLite为公共领域。
- pdf-extract：MIT，保留其依赖各自许可证。
- Apache Commons Compress：Apache-2.0；XZ for Java：0BSD。

EasyGot源码为GPL-3.0-only；依赖及复用源码保留原许可证。本项目不使用上游公开调试keystore作为正式签名。

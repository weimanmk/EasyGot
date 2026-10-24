# Rust学习业务 + 完整Harness运行时

- `rust/src/domain.rs`：标准题库、题型、评分边界、出题蓝图。
- `store.rs`：SQLite不可变版本DAG、冲突、素材哈希、最近错题。
- `materials.rs`：Rust PDF/DOCX/文本提取和本地原文检索。
- `network.rs`：HTTPS请求与DeepSeek Search/Fetch。
- `ai.rs`：视觉评分、图片识别及有来源的新题生成。
- `sync.rs`：WebDAV追加日志、素材校验和保留冲突的双向合并。
- `ui.rs`：Rust egui学习主界面、练习、考试、资料、草稿确认与设置。
- `platform.rs` + 小型Android MainActivity：SAF、Keystore、PDF渲染、启动Harness。
- `harness.rs`：学习MCP工具服务器和上游bundle配置。
- `vendor/deepseek-harness-mobile`、`harness-runtime`：完整上游Android运行环境，按MIT复用并裁剪与学习无关入口。

## 数据同步

`EasyGot/assets/<sha256>`、`EasyGot/logs/<uuid>.json`。日志为`{syncVersion:1,revisions:[...]}`，每个版本`version/entityId/kind/parents/modifiedAt/deleted/payload`。下载素材先校验再导入关联记录，导入检查完整父链和循环；并发头保留，只有用户确认才能合并。同名不可变文件用If-None-Match避免竞争覆盖。日志上传完整历史，尚无压缩、分片或远端垃圾回收。

模型及WebDAV凭据不进业务数据库或同步包。云端内容按用户要求明文保存，仅通过HTTPS传输。完整AI运行环境的权限边界与学习MCP的草稿规则不同，详见HARNESS.md。

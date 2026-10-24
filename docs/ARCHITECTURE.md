# 架构

原生 Android Java 17 / API 26+。无后台账号服务器。系统文档选择器提供文件访问，私有目录存储内容寻址素材，SQLite 存不可变版本。

- `Domain`：JSON题库、题型和分数边界。
- `Blueprint`：考纲权重最大余数分配，题型/主知识点配比验证。
- `Store`、`RevisionGraph`：SQLite事务、版本DAG、冲突和历史版本。
- `Materials`：PDFBox文字提取、DOCX ZIP/XML解析、PDF渲染和图片识别。
- `Ai`：OpenAI-compatible Chat Completions，视觉评分、出处检查、出题、受限Agent。
- `DeepSeekSearch`：独立 Search / Fetch 请求。
- `Vault`：Android Keystore AES-GCM 加密本机配置。
- `WebDav`、`SyncJob`：HTTPS追加日志同步与系统周期任务。
- `MainActivity`：五个导航页；JSON预览、答题、资料、Agent、配置与冲突处理。

## 同步协议 v1

```
EasyGot/
  assets/<sha256>
  logs/<uuid>.json
```

日志 `{syncVersion:1,revisions:[...]}`。版本包含 `version,entityId,kind,parents,modifiedAt,deleted,payload`。每个日志自包含历史父版本，无需按时间排序下载。素材先完成 SHA256 校验再合并记录；失败不标记日志已接收。

版本父链引用同一记录；导入拒绝缺失父版本、跨记录父引用、环及同版本不同内容。一个记录有多个未被子版本引用的头时为冲突。普通编辑不能静默消除冲突，显式选择版本/手动合并才创建以双方为父的新版本。删除也是一个版本，不从远端物理删除。

每次同步先读未知日志并导入，再上传本机全部历史及所需素材。日志使用随机唯一文件名，素材按内容哈希命名；PUT使用 `If-None-Match:*`，避免设备共同写可变清单。云端不保存任何设备凭据。未来可增加日志分片、压缩、索引及带安全水位的垃圾回收；首版宁可保留历史，不隐式清除冲突。

## Agent 边界

工具只有只读检索、读取错题、DeepSeek搜索与抓取；抓取URL必须来自本轮搜索。无命令执行或任意文件系统工具。每次最多6轮、每轮最多6个工具调用。资料/网页作为不可信内容提供，不能覆盖系统指令。模型直接输出不得触发改题、改笔记。出题是独立结构化草稿流程，经格式/配比/来源验证后再预览确认。

模型、搜索、WebDAV地址仅允许HTTPS，不自动跟随重定向并转发鉴权。超时、响应体大小均有上限。API错误不向界面打印请求头、Key或服务原始错误体。

# DeepSeek Search / Fetch

依据用户指定的 [官方指南](https://api-docs.deepseek.com/zh-cn/guides/search/)，核对日期2026-10-24。

| 接口 | 方法和路径 | 本应用请求 |
| --- | --- | --- |
| Search | `POST https://api.deepseek.com/v1/tools/search` | `query`, `count:5`, `need_content:true` |
| Fetch | `POST https://api.deepseek.com/v1/tools/fetch` | `url` |

鉴权：`Authorization: Bearer <搜索Key>`；Content-Type为application/json。

Search query为1–128个Unicode字符；count为1–50。读取results数组中的title、url、snippet、summary、content等字段，保留来源供Agent引用。Fetch读取title与Markdown content。不是Chat Completions请求，也不是Tavily接口。

应用设置独立搜索Key；只在本轮勾选联网后提供搜索工具。遇到400/401/402/403等错误显示明确状态，不无限重试或回退到不知名服务。429及网络错误由用户稍后重试。

未使用用户真实Key进行付费线上请求；协议单元测试不消耗API费用。

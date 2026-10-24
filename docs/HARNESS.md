# 内部Agent引擎

最终要求：Agent服务EasyGot学习功能，不安装另一套DSH，不包含容器、Node、WebView、MCP或用户插件安装。

参考Ajwyunsx/deepseek-harness-mobile及上游DeepSeek Harness的Agent循环、会话事件、工具注册、上下文维护和循环守卫，在agent.rs以Rust实现。参考版本和MIT许可留存于vendor；不宣称直接执行其TypeScript包。

统一模型配置供Agent、视觉评分、识图和出题共用。DeepSeek搜索复用适用官方Key或使用高级覆盖项。工具直接调用业务函数；题库/笔记修改先生成草稿，确认后写入。原始事件持久保存；近期12条对话参与请求，工具结果裁剪，重复调用最多两次，单步8工具、单轮24步，取消在请求返回和步骤边界生效。

# 测试

CI：cargo test --lib；cargo ndk ARM64 release构建；gradle lintDebug assembleDebug。

核心测试包含题库校验、客观题判分、AI分数边界、版本冲突与父链、DeepSeek请求字段及Unicode长度、考纲权重和学习工具范围。

仍需真机：中文字体与输入法、文件导入、PDF/Word图片、离线恢复、考试超时、真实视觉模型评分、Agent工具及取消、DeepSeek搜索、双设备WebDAV冲突、APK安装升级。未经实际执行不得称为已验收。

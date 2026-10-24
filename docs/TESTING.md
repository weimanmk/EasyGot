# 测试记录

2026-10-24，应用源码 `ed99dfd234705379108af89b0101eac558effe36`：

- 16项Rust测试通过；包含旧JSON兼容、10套题目录按序导入/导出/重导入、错误目录引用、题库资料关联、纸卷题号/答案隔离/计时/判分及原照片素材保留。
- ARM64构建、Android lint与测试签名预览包打包通过。
- Android 35 AOSP模拟器：实际拖动悬浮球、显示密度切换后继续响应、原生密码输入框粘贴、目录/答题/助教/纸卷页面、A4 PDF生成。
- 非调试预览包覆盖安装后保留24道测试题；禁用调试访问和测试Intent；应用运行日志无崩溃。
- PDF实文件已渲染检查：中文、题号、页脚和答题区正常；作答卷不带参考答案。

[Actions](https://github.com/weimanmk/EasyGot/actions/runs/40074429762) · [截图与检查结果](https://github.com/weimanmk/EasyGot/tree/qa/run-40074429762/qa-captures) · [界面检查](../design-qa.md)

仍需真机与真实服务：中文输入法、不同文件提供器与实际PDF/Word图片、真实视觉模型评分/识图、Agent工具与取消、DeepSeek搜索、双设备WebDAV冲突、从0.1版到新签名的升级迁移。实际模型准确率未由这些模拟器测试证明。

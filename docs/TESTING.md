# 0.3.0-alpha.2 字体与密钥输入修复验收

2026-10-25，应用源码 `c6afefc7da1888166ef4f7276f7d4ad03661bf64`。

[完整通过的 Actions](https://github.com/weimanmk/EasyGot/actions/runs/40147865307) · [Android 10 证据](https://github.com/weimanmk/EasyGot/tree/qa/run-40147865307-api29/qa-captures) · [Android 15 证据](https://github.com/weimanmk/EasyGot/tree/qa/run-40147865307-api35/qa-captures) · [验收边界](../design-qa.md)

- 29项Rust测试通过：作答计时、整卷原子/幂等提交、旧会话兼容、题库目录、关联资料、纸卷、图片限制、版本冲突；新增完整备份恢复答题/纸卷照片/冲突、重复合并不重复写入、备份不含凭据、损坏或缺素材时不提交版本。
- Slint编译、适配器、格式和实际组件渲染通过；ARM64 APK、Android lint、APK实际内嵌字体字节与apksigner检查通过。
- Android 10/15实际操作通过：点选、客观题判分、主观题软键盘、强制关闭后恢复、系统返回、滚动选择第10套题并检查练习范围、悬浮球拖动、纸卷PDF生成。
- 两代系统均实际点击「粘贴」与「系统输入」，验证密码遮罩和XML无明文测试Key，保存后冷启动仍能经Android Keystore解密读取。
- 两代系统均隐藏系统中文字库，再冷启动debug和非debug APK；设置、首页、字库样本的要求标签全部通过OCR，失败列表为空。已人工查看实际截图，确认此前的缺字标签正常。常规/粗体各映射44,810码位，当前源码与示例552字无缺字。没有用删除断言处理OCR漏读。
- 350dp/412dp、字体1.0/1.3、键盘和助教浮层截图复核；非debug预览禁止run-as与测试Intent，覆盖安装保留模拟器测试数据。
- 预览证书缓存已在独立构建中恢复并匹配固定指纹。旧预览的缓存没有真正保存，旧包可能无法覆盖安装；先导出并核对完整备份，再迁移。详见[签名说明](PREVIEW_SIGNING.md)。

用户VPS尚未登录：直连网络不可达，受管SOCKS IPv4连接报告22端口连接被拒绝，用户确认使用22；密码认证尚未开始。提供[VPS本机验证脚本](VPS_QA.md)，只完成语法检查，不能声称已在该VPS运行。

以上为本轮alpha验收。真实模型/视觉批改、DeepSeek、双设备WebDAV、复杂实际资料、Android 8及OEM真机/中文输入法、长期性能、TalkBack和用户持有的正式签名仍未全部验收，不能称为商业稳定版。旧的 `40116962518` 不再作为字体兼容性证据。

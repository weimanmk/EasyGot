# Slint Android验收

final result: pending-commercial-acceptance
native frontend checks: passed

应用源码：`18328aff50b53f2ee1f9f46ce452762ea629bf51`。2026-10-25。

[最终构建与回归](https://github.com/weimanmk/EasyGot/actions/runs/40116962518) · [截图及JSON检查结果](https://github.com/weimanmk/EasyGot/tree/qa/run-40116962518/qa-captures)

## 已通过

- 27项Rust测试、Slint布局编译、适配器检查、源码格式检查、ARM64 APK及Android lint。
- 实际点选选项与确定性判分；强制关闭后恢复答案与题号；系统返回导航。
- 主观题实际键盘输入，返回键先收起键盘；原生密钥输入框粘贴。
- 实际拖动悬浮球；滚动选择第10套试题后开始练习，确认仅有该套2道题且顺序正确。
- 350dp/412dp、字体1.0/1.3；824×1832物理窗口下内容区底部784dp、行宽368dp，截图确认窗口正确填满。
- 纸卷PDF生成；预览包覆盖安装保留测试数据；禁止run-as与测试Intent。
- 已实际查看最终home、预览home、助教、设置、目录、作答、键盘、解析及大字体/宽屏证据。当前检查范围内没有未解决的阻塞界面问题。

## 迭代发现与修复

Slint语法/角色声明、Android Resume变体、host Fontconfig依赖、悬浮球遮挡、Unicode图标缺字、导航不均分、主流程入口被长目录挤下、滚动中测试点击半露出的卡片、窗口尺寸固定导致412dp留白，均已修复或改进测试。详见docs/ITERATION_LOG.md。

## 商业验收仍未通过

真实模型/视觉批改及识图准确率、DeepSeek实际服务、双设备WebDAV冲突与中断恢复、真实PDF/Word图片样本、真机性能/长期运行/中文输入法兼容、TalkBack与无障碍尚未完成。模拟器或单元测试不能证明这些场景通过。当前仅可交付测试签名alpha安装包，不发布商业稳定版。

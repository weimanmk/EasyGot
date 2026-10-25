# 源码对照与重构决定

2026-10-25。已直接下载完整仓库工作树（浅克隆），实际阅读下列文件。参考源码放在开发工作区的 `reference-projects/`，不把其他项目整个塞进 APK 或 EasyGot 仓库。

| 项目与固定提交 | 实际阅读代码 | 值得采用的设计 | EasyGot落地 |
|---|---|---|---|
| [DeepStudent](https://github.com/helixnow/deep-student/tree/c565f0c3043adc282a98f776135beffae21ca7ce) | `src/stores/questionBankStore.ts`、`src/stores/__tests__/recordPracticeAnswer.regrade.test.ts` | 区分未判分/判对/判错；重复提交和改判不能重复增加完成数；以持久数据为准 | `practice.rs`将未答与待批改分开，整卷事务写入；同一会话题目使用稳定attempt id；重启后重复交卷为空操作 |
| [Shiroha Quiz](https://github.com/reiqr/shiroha-quiz/tree/45f06728a5ddf3b760bea4586127852da525fc73) | `apps/android/app/src/native/java/com/yiqiu/shirohaquiz/ui/screens/PracticeScreen.kt`、`importer/validate/ImportValidator.kt` | 手机题目卡、可见选中状态；导入校验与作答界面分离；按范围恢复练习 | Slint组件统一交互尺寸、输入和布局；导入先预览、逐题表单编辑再确认；`controller.rs`不依赖渲染框架 |
| [Exameow](https://github.com/heshengtao/exameow/tree/ab792a9ae961a128de664b17268cd062587bb95e) | `frontend/src/stores/practice.ts`、`frontend/src/components/practice/QuestionCard.vue`、`packages/core/src/exam/judge.rs` | Rust核心/前端状态分层；主观题输入和客观题选择分别处理；兼容旧学习数据 | 保留Rust核心，Slint仅负责显示/事件；旧JSON会话可读；评分继续保留用户要求的部分分和严格要求例外 |

## 实现边界

- DeepStudent的工作台、多应用桌面和Anki管理不适合当前Android答题范围。
- Shiroha的Compose组件不直接搬进Rust项目；学习其交互和模块边界。
- Exameow的账号、教师在线考试、云端中转和MCP不引入。
- 已有Harness继续作为内置Agent机制参考，不增加独立DSH程序。
- 本轮没有复制上述三个项目的源代码段。各项目许可证分别为AGPL-3.0、GPL-3.0、Apache-2.0（以固定提交LICENSE为准）；EasyGot继续GPL-3.0-only。若以后直接复用代码，需要另行按具体文件保留许可与归属。

## 框架决定

采用[Slint 1.18.1](https://docs.rs/slint/1.18.1/slint/android/index.html)，`.slint`声明布局与可重用组件，Rust控制器负责业务。官方Android后端使用android-activity，可继续保留现有NativeActivity与系统文件/打印/Keystore桥。原egui/eframe布局与手工字体装载移除。

目标是更稳定的手机布局、输入法/选区/粘贴、可维护组件和状态测试，不预先承诺APK更小；实际大小与渲染性能以构建和设备测量为准。Slint Android无障碍和软键盘行为必须单独验收，声明accessible属性不等于TalkBack已通过。

## 当前发现及修复

1. 原`ui.rs`混合绘制、导航、异步任务和业务保存：拆为`ui/app.slint`、Slint适配器`ui.rs`、`controller.rs`和纯业务`practice.rs`。
2. 单题耗时误用整段练习耗时：记录当前题进入/离开时间，各题累计。
3. 整卷逐题写入：改为题目成绩+已交卷会话的单次SQLite事务，防止半卷写入。
4. 主观题空白也标待评分：未作答明确计0分，不发送空白答案评分。
5. 主流程直接编辑出题JSON：改为题量、题型、章节、难度表单。导入/AI草稿可逐题编辑题干、选项、答案、解析与严格要求。
6. 长题库和资料原先仅固定截断：增加继续显示/阅读入口。

## 验收方式

循环执行：源码审查 → 实现 → Rust状态测试/Slint编译 → Android构建 → 实机等效模拟器操作与截图 → 修复并重跑受影响流程。`design-qa.md`记录当前结果；未验收的服务连接和设备场景保持待验证，不把预览叫成稳定商业版。

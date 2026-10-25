# 临时 VPS 验证

`scripts/vps-qa.sh` 在专用 Debian/Ubuntu x86_64 VPS 本机运行，不包含地址、密码、令牌或自动上传功能。安装构建工具前记录系统、CPU、内存、磁盘、KVM 情况；要求至少6GB内存、24GiB可用磁盘。脚本安装JDK/Gradle/Rust/SDK/NDK，构建ARM64与模拟器APK，然后顺序运行Android 10/15的ADB回归和系统中文字体隔离检查。无KVM时只尝试明确记录的软件模拟，15分钟不能启动就失败，不报告通过。东京时间21点后不启动下一轮。

在服务器的干净源码目录内运行：

```sh
bash scripts/vps-qa.sh
```

结果默认在源码相邻的 `easygot-vps-qa/EasyGot-vps-evidence.tar.gz`，包括提交、资源情况、工具版本、退出状态、模拟器日志、安装包、截图与检查JSON。必须在临时机器关机前下载保存；脚本不改变服务器关机计划，不开放ADB端口。VPS安装包为该VPS自己的测试签名，不替代Actions签名的正式下载预览。

当前会话的直连SSH返回“网络不可达”，提权连接被执行环境的审批策略拒绝；继续使用既有受管代理尝试，HTTP隧道在SSH握手前关闭，SOCKS的IPv4连接返回code 5（连接被拒绝）。用户确认端口22，尚未进入密码认证，也未在用户VPS执行该脚本。不能把脚本准备或GitHub模拟器结果写成用户VPS验收通过。

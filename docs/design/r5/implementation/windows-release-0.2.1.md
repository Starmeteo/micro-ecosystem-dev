# R5 Windows 安装包 0.2.1

0.2.1 在两套可编辑的 R5 场景及浅塘修复基础上，加入米虾成长阶段、重复绘制去重、落叶笔刷、生物投放和活体植被编辑。打开方式见项目首页。 [0.2.0 浅塘修复与交付](windows-release-0.2.0.md) · [0.2.1 编辑与绘制](editor-followup-verification.md)

| 产物 | 大小 | SHA-256 |
| --- | ---: | --- |
| 0.2.1 安装包（本地副本已清理） | 74,388,165 字节 | `95778373B0C8F58E5CFA1A21AFBC2A84D5AB8E9120092678BFFDFE6067C284F5` |
| 0.2.1 独立程序（本地副本已清理） | 82,995,712 字节 | `F59FD1D0D808D6039E592F5F7CF104C4ED567DD873E8C736AFC9BED3BA527D15` |

## 实测

- 2026-11-26 在 Windows 11 x64 与本机 WebView2 上完成当前用户隔离安装，实际安装载荷与 NSIS 标记后的预期哈希相符。首次启动显示 R5 林下 V14，创建浅塘 V13。两套组件画布均为 576×324 设备像素。两个世界保存、退出并重开后逐项相同。卸载清理本次临时登记，隔离存档字节保持。 [安装记录](evidence/r5-release/0.2.1/install-fresh.json) · [冷重开](evidence/r5-release/0.2.1/reopened-webview-check.jsonl) · [林下组件](evidence/r5-release/0.2.1/installed-forest-panel.png) · [浅塘组件](evidence/r5-release/0.2.1/installed-pond-panel.png)
- 完成 0.1.0 到 0.2.1 覆盖升级。覆盖安装前后测试存档逐文件字节相同。两个旧世界按原版本 V4、V3 实际打开，旧字段逐项匹配，新增米虾阶段显示按原个体 ID 与保存年龄读取。旧林下渲染器按需载入后出图。 [升级记录](evidence/r5-release/0.2.1/install-upgrade.json) · [升级后回读](evidence/r5-release/0.2.1/upgraded-webview-check.jsonl) · [旧林下](evidence/r5-release/0.2.1/upgraded-legacy-v4-editor.png) · [旧浅塘](evidence/r5-release/0.2.1/upgraded-legacy-v3-editor.png)
- 128 项 Rust 工作区测试通过，前端类型检查与生产构建通过。用独立发行程序操作了两套活体植被并冷重开回读。 [工作区测试](evidence/editor-followup/workspace-tests-0.2.1.txt) · [植被存档](evidence/editor-followup/plants-reopen-checks.jsonl)

安装包为本地未签名构建。装卸验证使用静默参数和已有 WebView2 的机器。向导逐页和缺少 WebView2 的联网路径保留为未覆盖分支。设备矩阵专项和生态长跑校准按用户决定移出本轮验收。

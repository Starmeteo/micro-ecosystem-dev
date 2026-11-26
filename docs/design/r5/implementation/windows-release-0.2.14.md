# R5 浅塘多观景剖面 · 0.2.14

浅塘合池可保留两侧各自的石块、沉木和造景。两套 R5 场景均包含在 Windows 安装包中。 [上一版](windows-release-0.2.13.md)

| 产物 | 大小 | SHA-256 |
| --- | ---: | --- |
| Windows x64 安装包（本地副本已清理） | 74,598,670 字节 | `77E977F24EDE3F68620A1833622ABB0593DEA980C7E8901B6BA5EB5733C69B45` |
| 0.2.14 独立程序（本地副本已清理） | 83,706,368 字节 | `C2107DE815F7F310A3B80DB72F95C464D021E529E029C673EFEF07DCD3290449` |

## 使用方式

在「编辑环境 → 环境 → 编辑池岸形状 → 分合预览」选中承担生态活动的水域，画水道连接另一侧。预览会列出合池后的主、副剖面和水量。应用后返回场景，用画面下方的「观景剖面」切换。

- 在副剖面可放置、移动和移除石块、沉木和造景。物件编号在各剖面内独立。撤销、重做、试运行和另存为预设沿用同一套世界编辑流程。
- 一个水域最多保留一条主剖面和三条静态副剖面，整池只运行一套水量、水质和共用库存。米虾、植根水草、活体附着物和定位沉叶目前由主剖面承载。若另一侧也有这些局部生态状态，合池会被拦截。
- 再次分池时，副剖面保留原格网行和布置。它分到另一片水域时可成为那里可运行生态的主剖面。两种填岸方式都保护副剖面的完整行。

包含副剖面的存档需要 0.2.14 或更新版本读取。 [模型、分合和库存规则](../../../../crates/eco-core/POND.md)

## 验证

Windows Rust 工作区 174 项回归、前端类型检查与生产构建通过。内核用例覆盖三条剖面分两次恢复、第五条剖面原子拒绝、撤销和检查点恢复后的编号保护。运行层用例验证了试运行隔离、采用、冷重开以及保存成预设再创建世界。 [工作区回归](evidence/static-sections-0.2.14/workspace-tests.txt)

隔离 Edge 用模拟命令回执验证分合预览、两种填岸保护和副剖面编辑。同编号石块可独立选择和移动，副剖面的环境与生物面板仍读取水域实值。 [预览核对](evidence/static-sections-0.2.14/edge-preview-checks.json) · [编辑核对](evidence/static-sections-0.2.14/edge-editor-checks.json)

Windows 原生编辑器把 5 L 和 7 L 两个水域合为 12 L。原有米虾仍归主剖面。移动副剖面的 1 号石块后，主剖面的 1 号石块、水量和米虾状态保持一致。撤销、重做和副剖面保护通过。再次分池得到两个 6 L 水域，原副剖面的石块、沉木和造景一起成为新水域的布局。回到合池状态，保存退出后在新进程中打开，12 L 水量和三件副剖面物件完整回读。 [合池预览](evidence/static-sections-0.2.14/sections-merge-preview.png) · [副剖面编辑](evidence/static-sections-0.2.14/sections-secondary-edited.png) · [受保护的池岸](evidence/static-sections-0.2.14/sections-row-protected.png) · [拆分预览](evidence/static-sections-0.2.14/sections-split-preview.png) · [原生记录](evidence/static-sections-0.2.14/static-sections-checks.jsonl) · [重开画面](evidence/static-sections-0.2.14/sections-reopened.png) · [冷重开记录](evidence/static-sections-0.2.14/static-sections-reopen-checks.jsonl)

Windows 11 x64 隔离新装与从 0.2.13 覆盖升级通过。新装后林下和浅塘均可在新进程中回读。三份旧测试世界在安装前后逐字节一致，包含旧版分池后分别保存的米虾、水蚤、浮叶和静态布置。设备矩阵和长跑校准沿用已确认的验收范围。 [新装](evidence/static-sections-0.2.14/install-fresh.json) · [新装重开](evidence/static-sections-0.2.14/reopened-webview-check.jsonl) · [旧版种档](evidence/static-sections-0.2.14/prior-webview-check.jsonl) · [覆盖升级](evidence/static-sections-0.2.14/install-upgrade.json) · [升级回读](evidence/static-sections-0.2.14/upgraded-webview-check.jsonl)

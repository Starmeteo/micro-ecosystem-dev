# R5 活体蕨丛、沉水落叶与双水域 · 0.2.3

| 产物 | 大小 | SHA-256 |
| --- | ---: | --- |
| Windows x64 安装包（本地副本已清理） | 74,437,399 字节 | `D0968C879CC13957013735BEC506BE01FBB9009ABF801081CA26F1C005FC19B2` |
| 0.2.3 独立程序（本地副本已清理） | 83,258,880 字节 | `AA87B71049EE955CEACA8CEED44F1CEE1B860381643F5CF49671B460C923FCA7` |

## 林下活体蕨丛

在「编辑环境 → 内容 → 种植小型蕨丛」选土格。可选中、移植、修剪或取出同一斑块。试验档初始 15 mg C，主形株高约 140 mm、丛宽约 130 mm。蕨丛利用所在土格原有的生长和胁迫过程。移植搬动斑块的库存份额，修剪转入当地的枯落物，取出计入外部输出。

叶冠按当前库存计算遮阴潜力，当前格上限 28%，相邻共边土格为一半。与已有遮阴取较强值，并按原蒸发规则反馈到水量。植株形态依据实际库存和胁迫，可显示新叶、小丛、展开或胁迫状态。尺寸与遮阴参数均为设计和模拟档位。

隔离 Windows 编辑器实测了种植、移植、修剪和撤销。修剪令蕨丛余量从 15 降到 3 mg C，转入土格枯落物 12 mg C，叶冠遮阴潜力从 28% 降到 5.6%。 [展开](evidence/fern-leaf-0.2.3/fern-open.png) · [修剪](evidence/fern-leaf-0.2.3/fern-trimmed.png)

## 浅塘沉水落叶

在「编辑环境 → 内容 → 沉水落叶」选择中景沉叶或大沉叶，并输入 mg C，再在画面点选池底位置。投放一次计入现有整池碎屑中的碳、氮、磷。池底位置是显示锚点。移动它保留该投放点的库存份额，进入相同水域的碎屑继续共用原有分解过程。

中景整叶与大沉叶分别复用 55 mm、105 mm 的独立 R5 材料。剩余碳比例达到 60% 时显示整叶，至少 20% 且低于 60% 时显示残叶，更少时显示细碎屑。降到收起阈值后，投放点的余量归到整池未分配碎屑。精确取出某一份混合碎屑仍是后续规则。撤销投放可恢复操作前的状态。

复制当前世界保留沉叶与蕨丛的现存量。保存为初始预设时保留位置并使用初始剂量。

原生编辑器投放一次 50 mg C，整池碎屑增加 50 mg C。沿池底移动时总量保持，撤销重做保留同一投放点 ID。Edge 的实际 V13 画布另用夹具逐帧核对两种落叶的三个阶段，图像变化集中在投放位置，省略沉叶字段和空列表的结果逐像素一致。 [原生投放](evidence/fern-leaf-0.2.3/pond-deposited-leaf.png) · [中景整叶](evidence/fern-leaf-0.2.3/midground-whole.png) · [残叶](evidence/fern-leaf-0.2.3/midground-fragments.png) · [细碎屑](evidence/fern-leaf-0.2.3/midground-fine.png) · [两档像素比对](evidence/fern-leaf-0.2.3/leaf-render-checks.json)

## 浅塘双水域

从「世界库 → 从预设新建」选择「浅水池塘 · R5 双水域」。两个水域各持有水量和生态库存。编辑器的「环境」可以切换当前水域，池岸笔刷同时显示另一水域的占地。两池之间保留陆地间隔。 [几何与原版池岸](windows-release-0.2.2.md)

原生编辑器向第二池补水 0.8 L，第一池保持 18 L，第二池增至 18.8 L。尝试画出相接岸线时被拦截，第一池状态保持，沉叶仅出现在所属水域。 [两池边界](evidence/fern-leaf-0.2.3/pond-dual-boundary.png) · [原生操作记录](evidence/fern-leaf-0.2.3/plants-checks.jsonl)

## 存档与验收

以新的进程与启动代次冷重开，验证林下蕨丛及浅塘沉叶、双池的身份和状态恢复。Windows Rust 工作区 142 项回归通过，前端类型检查与生产构建通过。验收按用户决定排除设备专项与历史成对种子验证。 [冷重开](evidence/fern-leaf-0.2.3/plants-reopen-checks.jsonl) · [工作区测试](evidence/fern-leaf-0.2.3/workspace-tests.txt)

Windows 11 x64 隔离新装与 0.2.2 → 0.2.3 覆盖升级通过。新装冷重开两套场景。覆盖安装前后旧存档逐文件 SHA-256 相同，旧林下保留两格细土，旧浅塘保留 2,600 cm² 水面和 18 L 水量。两次卸载均清理临时安装并保留存档。 [新装](evidence/fern-leaf-0.2.3/install-fresh.json) · [新装冷重开](evidence/fern-leaf-0.2.3/reopened-webview-check.jsonl) · [升级](evidence/fern-leaf-0.2.3/install-upgrade.json) · [旧版编辑记录](evidence/fern-leaf-0.2.3/prior-webview-check.jsonl) · [旧档新版回读](evidence/fern-leaf-0.2.3/upgraded-webview-check.jsonl)

验收使用已有 WebView2 的环境和当前用户静默装卸。第一次升级验证把会话撤销标志误纳入持久态比对，修正夹具后使用同一个安装包重跑并通过。 [首次原始记录](../../../../.local/r5-installer-evidence/68dad0195ff8409fa1f6b6ad1c2d44de/installation.json)

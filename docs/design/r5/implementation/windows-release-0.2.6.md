# R5 分合池预览与米虾摄食 · 0.2.6

本版承接 0.2.5 的林下与浅塘场景，在池岸编辑器加入分合预览，并接入米虾接触沉木时的真实摄食反馈。 [上一版记录](windows-release-0.2.5.md)

| 产物 | 大小 | SHA-256 |
| --- | ---: | --- |
| Windows x64 安装包（本地副本已清理） | 74,510,763 字节 | `CB9593B11FAF49F328762D4531A84E6267A1A127B1079DADA3CCA8ED5C703D8E` |
| 0.2.6 独立程序（本地副本已清理） | 83,314,688 字节 | `7FFCCA5135BDE3350AFBCDD1C63AA2983C3BA02F6B1B245DE020196768C789BD` |

## 浅塘分合池预览

在「编辑环境 → 环境 → 编辑池岸形状」打开「分合预览」。用「画水道」或「填岸地」绘制一笔或多笔，图上按共边连通着色。右栏可展开受影响的水域，看原水量、生物、物件、造景以及预计各区的面积、容量、水量和平均水深。

合池预览在格面积一致时合计两池水量。分池预览以各区的剩余面积比例分配水量，因此估算采用相同平均水深。若两池每格实际面积不同，合池的面积、容量和水量估算暂留待定，同时给出原因。容量不足、原观景剖面被切断及对象归属需要确认时，右栏给出相应提示。

预览草稿只在编辑器中展示。可按 Esc 回到池岸笔刷。正式分合提交仍需确定各区身份、观景剖面及现有物质与物件的归属。

本机双水域用一格水道相连，预计合为 4,900 cm²、36 L、容量 58.8 L。另一个有窄口的 18 L 水域，填一格岸地后预计分为 400 与 2,400 cm²，按等均深分配约 2.57 与 15.43 L。两次预览前后，真实水域、库存和编辑修订逐项一致。键盘可追加一笔并清除整份预览。计算夹具另检查不同格面积、超容量、斜角不连通、整池移除及不兼容格网。 [原生合池](evidence/topology-feeding-0.2.6/merge-preview.png) · [原生分池](evidence/topology-feeding-0.2.6/split-preview.png) · [计算记录](evidence/topology-feeding-0.2.6/topology-pure-checks.json)

## 米虾接触沉木时的摄食

Rust 在真实摄食分配时记录已经接触浸水沉木、当步获得食物且保持接触至步末的米虾。画面为该只米虾显示轻微口部动作。「生物 → 接触沉木时的摄食」可查看当步从整池附着生物膜获得的碳量。该摄食量属于整池库存账，具体木石的本地食源仍待建模。

反馈存于当前运行快照。冷重开从世界存档恢复，试运行开始时从空反馈起步，后续实际推进可重新产生记录。

原生试运行在第二池观察到米虾 `pond-2-shrimp-1` 的真实摄食，最终一步收到约 0.0000249 mg C，编辑器画布与生物面板均出现反馈。丢弃副本后正式世界逐项一致。运行时回归验证开启反馈与普通推进的生态状态、资源账和序列化相同。Edge 以实际绘制链验证水线裁切、过期接触、对错沉木、干涸和同一步刷新，4× 对照中的变化集中在口部。 [原生摄食](evidence/topology-feeding-0.2.6/wood-feeding-active.png) · [停靠 4×](evidence/topology-feeding-0.2.6/resting-4x.png) · [摄食 4×](evidence/topology-feeding-0.2.6/feeding-4x.png) · [像素对比](evidence/topology-feeding-0.2.6/feeding-edge-checks.json)

## 验收

Windows Rust 工作区 145 项通过，前端类型检查与生产构建通过。本机编辑器测试和两份世界冷重开通过。设备矩阵与历史成对种子专项沿用用户确认范围。 [Rust 回归](evidence/topology-feeding-0.2.6/workspace-tests.txt) · [原生操作](evidence/topology-feeding-0.2.6/topology-feeding-checks.jsonl) · [冷重开](evidence/topology-feeding-0.2.6/topology-feeding-reopen-checks.jsonl)

Windows 11 x64 隔离新装与 0.2.5 覆盖升级通过，两套场景的组件画布均为 576×324。升级保留第二池的细砂、粗砂、沉叶和两池水量，也保留林下蕨丛。两份测试存档逐文件 SHA-256 一致，冷重开后的持久状态逐项一致。临时安装已卸载，测试档保留。使用已有 WebView2 与当前用户静默装卸。 [新装记录](evidence/topology-feeding-0.2.6/install-fresh.json) · [覆盖记录](evidence/topology-feeding-0.2.6/install-upgrade.json) · [旧档回读](evidence/topology-feeding-0.2.6/upgraded-webview-check.jsonl) · [升级后第二池](evidence/topology-feeding-0.2.6/upgraded-second-basin.png)

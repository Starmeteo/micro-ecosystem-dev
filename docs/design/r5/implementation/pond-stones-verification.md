# R5 浅塘底石验收

2026-11-25，北京时间 · P01 首批

> 以下操作为当时的验收路径。当前开发入口与预设见 [项目说明](../../../../README.md)。

从「世界库 → 从预设新建」选「浅水池塘 · R5 池底造景」，在「内容」选择主石或卵石，沿池底放置。点选石身或属性列表可选中单件。拖动，或用左右键预览、回车确认。移除、撤销和重做沿用当前世界编辑历史。用「环境」可排水、补水并观察水线。

## 实现范围

- 新定义 `pond-shallow-r5-stones/1`、模型版本 10。每个水体最多四块底石，主石参考长径 80 mm，卵石 25 mm。坐标沿一条固定的池底截面保存，水位改变时，模型和画面使用同一个池底基准。清底、石身和接触影可独立绘制。水面下的石身受水色影响，露出部分保留干面色。 [可编辑美术源](https://github.com/Starmeteo/micro-ecosystem-dev/blob/ff9ae2b0ff82050f1ea112a7013054f94c89227d/apps/desktop/src/render/source/r5_pond_bottom.py) · [绘制与落点](../../../../apps/desktop/src/render/r5/pondStoneGeometry.ts)
- 内核使用二维碰撞近似，为虾预留横向间隙和两岸水域。石头覆盖虾当前位置或和另一块过近时拒绝提交。游动检查整步轨迹，并尝试向上、侧转或返回可通行水域。水位下降导致原位置与石体相交时，虾退到上方或最近的可用侧面。完全干涸沿用原有生物结算，补水后不会自动补回虾。
- 布置命令不改变水体容量、当步水量、C/N/P 或溶氧。水质继续采用全池充分混合模型。石材和水位操作都能在 v10 试运行副本中执行，凭世界 ID、运行代次、编辑修订号和目标水体确认。旧浅塘 v3 可独立打开，保持自身的画面和规则。

## 验证结果

| 检查 | 实际证据 |
| --- | --- |
| 可解释的避障路线 | 受控内核以种子 2729 放置主石，令一只虾由左向右靠近碰撞轮廓。第一步它向上转，之后从上方抵达另一侧。移除主石的对照首步即进入原占据区。两组推进至相同步数时，水、溶氧和 C/N/P 账本完全相同。旧版 v3 和空底石新版的模拟轨迹比对了 2,500 步。 [内核专项](../../../../crates/eco-core/src/pond/tests.rs) |
| Windows 放置和编辑 | WebView2 `158.0.4370.33`、单屏 150%。通过内容抽屉放置两种底石，在石身上拖动卵石并撤销重做。原主石、虾坐标、水量及溶氧保持原值。提交重叠位置得到实际冲突回执，Esc 取消无修订。 [预览](evidence/pond-stones/pond-stone-ghost.png) · [有水画面](evidence/pond-stones/pond-stones-wet.png) · [原生记录](evidence/pond-stones/pond-stones-edit-checks.json) |
| 露出和补水 | 原生端执行 18 L → 1 L → 0 L → 18 L。逻辑画面水线 y 分别为 112、234.78、242、112。主石和卵石锚点始终相同。水浅时主石上半露出，排空时保留整块石头。补水后虾数量仍是 0，随后撤销三次恢复排水前的完整快照。 [浅水](evidence/pond-stones/pond-stones-shallow.png) · [排空](evidence/pond-stones/pond-stones-dry.png) |
| 副本、保存及重开 | 在原生试运行移除主石、撤销并推进 600 步。采用前正式世界保持原布置与模拟时刻。采用后正式保存，进程重启后石材 ID、位置、虾和水体数据逐项相同，编辑器、观赏窗和桌面组件显示同一组底石。旧 v3 世界通过世界库按原定义恢复。 [试运行](evidence/pond-stones/pond-stones-trial.png) · [桌面组件](evidence/pond-stones/pond-stones-reopened-panel.png) · [退出](evidence/pond-stones/exit-checks.json) · [重开记录](evidence/pond-stones/pond-stones-reopen-checks.json) · [旧档记录](evidence/pond-stones/pond-stones-compat-checks.json) |
| 前端和回归 | 94 项 Rust、13 项布局用例通过。TypeScript 与 Vite 构建通过，Vite 仍提示入口超过 500 kB。Edge 界面夹具在 1060×730 和 580×440 核对抽屉、Esc、键盘移动及副本命令。125% 和 150% 画面对照确认露出部分与干面像素一致，水下部分着色。 [抽屉夹具](evidence/pond-stones/browser-drawer-580x440.png) · [前端记录](evidence/pond-stones/browser-ui-checks.json) · [遮挡及水线夹具](evidence/pond-stones/browser-shallow-fixture.png) |

验收使用独立 Windows 开发版。首次原生脚本在重做成功后读到了上一帧的卵石坐标，复测等待画面与权威快照一致后才取证。实机跨屏及新安装包待验。石组、任意朝向、三维游动及附着面积属于后续范围。

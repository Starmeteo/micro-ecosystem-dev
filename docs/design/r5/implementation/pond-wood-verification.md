# R5 浅塘沉木验收

2026-11-25，北京时间 · P02 首批

从「世界库 → 从预设新建」选「浅水池塘 · R5 沉木布置」，打开「内容」选择主枝沉木或短根沉木。可以沿池底拖动，也可以用左右键预览、回车确认。在「属性」选择沉木可查看停靠数量。移除前可取消或确认释放停靠的虾。

## 实现范围

- `pond-shallow-r5-wood/1`、模型版本 11 继承底石布置和水体工具。主枝轴长 300 mm，短根 150 mm，两者以同一固定角度投影到一条池底截面。每个水体最多两件沉木。两件短根可分开放置，长主枝会占用大部分横向空间，石木间仍须留通路。石和木各有独立的实例编号。
- 沉木有分层木身、接触影和苔藓纹饰。纹饰是随父物件移动和移除的装饰层。它不参与全池生物膜计量，也不引入木材分解或浸出。木体高出水线的部分保留干面色。 [像素素材源](../../../../apps/desktop/src/render/source/r5_pond_wood.py)
- 虾接近仍浸水的木面时可自发停靠。该动作由 Rust 保存虾 ID、承载物 ID、相对横向落点和剩余停靠步数。每次停靠最多 35 步，合 3.5 模拟秒。主枝和短根中央有效落脚区宽 116 和 50 逻辑像素，画面木面覆盖整个区域及两端足部余量。持水条件变化后，仍可停靠的虾保持相对落点，木面露出时脱离，并在需要时移向可通行水域。 [生态内核](../../../../crates/eco-core/src/pond_wood.rs) · [绘制与拾取](../../../../apps/desktop/src/render/PondScene.tsx)
- 移动沉木只带走停靠在本件上的虾，通行校验通过后一次提交。合法落点显示沉木与虾的半透明预览。落点无效时原木及停靠关系继续显示，提示框标出无效位置。移除沉木解除停靠关系，虾从当前坐标继续活动。保存和复制现状保留停靠，另存为预设使用原布置及新世界的初始条件。

## 验证结果

| 检查 | 实际证据 |
| --- | --- |
| 自然停靠与结束 | Rust 专项用实际步进触发短根停靠，并验证 35 步期限后脱离。持同样石材的 v10 世界与无沉木的 v11 世界运行 2,500 步，虾轨迹、随机状态和资源账本一致。 [内核专项](../../../../crates/eco-core/src/pond/tests.rs) |
| Windows 布置和停靠 | WebView2 `158.0.4370.33`、单屏 150%。实际放置两件短根，再换主枝。正常推进试运行，到第 2,120 步观察到 2 号虾停靠在 3 号主枝上，剩余 31 步。采用前正式世界保持原状态。 [双短根](evidence/pond-wood/pond-wood-two-roots.png) · [停靠截图](evidence/pond-wood/pond-wood-naturally-resting.png) · [原生操作记录](evidence/pond-wood/pond-wood-edit-checks.json) |
| 搬移、释放、撤销 | 主枝沿池底从约 50% 拖到 52%，同一只虾横坐标增加约 0.02，纵坐标和相对落点保持一致。自由游动的虾与水量、全池溶氧等读数保持原值。取消移除不提交，确认移除解除停靠并保留原坐标，撤销恢复同一停靠。排到 1 L 时主枝原地露出，虾脱离且数量仍为三只。 [移动](evidence/pond-wood/pond-wood-moved-with-shrimp.png) · [取出后](evidence/pond-wood/pond-wood-released-shrimp.png) · [浅水](evidence/pond-wood/pond-wood-exposed.png) |
| 带虾拖动预览 | Windows 单屏 150% 下，合法预览中虾随草稿移动，越界时原木及原落点的虾继续显示。两种预览期间与 Esc 取消后，Rust 快照逐项相同。 [合法预览](evidence/pond-wood/pond-wood-valid-draft.png) · [越界预览](evidence/pond-wood/pond-wood-invalid-draft.png) · [原生记录](evidence/pond-wood/pond-wood-draft-checks.json) |
| 存档回读与兼容 | 原生保存退出并重启，核对沉木坐标、停靠虾实际坐标、相对位置、计时及水体数据。编辑器、观赏窗和桌面组件显示同一关系。旧 v10 浅塘按原定义读取原有底石。 [退出](evidence/pond-wood/exit-checks.json) · [重开记录](evidence/pond-wood/pond-wood-reopen-checks.json) · [桌面组件](evidence/pond-wood/pond-wood-reopened-panel.png) · [旧档记录](evidence/pond-wood/pond-wood-compat-checks.json) |
| 前端与回归 | 98 项 Rust、13 项布局用例通过。TypeScript 与 Vite 构建通过，Vite 仍提示入口超过 500 kB。Edge 夹具在 1060×730 和 580×440 验证抽屉、Esc 取消、试运行撤销与同编号石木独立选择。125% 和 150% 画面夹具核对停靠区域边缘的足点、拖动预览及水线。 [界面记录](evidence/pond-wood/browser-ui-checks.json) · [窄窗抽屉](evidence/pond-wood/browser-drawer-580x440.png) · [停靠边缘](evidence/pond-wood/browser-rider-fixture.png) · [拖动夹具](evidence/pond-wood/browser-drag-rider-fixture.png) |

本批使用固定二维截面、固定朝向和实验停靠时长。活体附着斑块及其生命周期属于后续批次。实机跨屏及安装包待验。

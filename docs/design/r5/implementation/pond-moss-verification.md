# R5 浅塘附着斑块验收

2026-11-25，北京时间 · P07 首批

从「世界库 → 从预设新建」选「浅水池塘 · R5 附着斑块」。先放置主石或沉木，在「内容」选择「苔藓小垫」或「分枝绒簇」，选择承载物并预览附着位置。确认后，可以从画布或「属性」中选择斑块，查看当前库存、调整位置、修剪或移除。

## 实现与边界

| 预设 | 外形跨度 | 初始库存 | 实验碳容量 |
| --- | --- | --- | --- |
| 苔藓小垫 | 30 mm | 3 mg C、0.186 mg N、0.024 mg P | 12 mg C |
| 分枝绒簇 | 50 mm | 7.5 mg C、0.465 mg N、0.06 mg P | 24 mg C |

- 模型 `pond-shallow-r5-moss/1`、版本 12 启用活体斑块。每件浸水主石或沉木可承载一处，卵石表面过小。斑块记录自己的 ID、承载物种类和 ID、相对位置及碳氮磷库存。移动承载物时保留相对位置和库存。旧世界使用自身版本，P02 木面纹饰继续作为装饰层。
- 投放将初始碳、氮、磷一次记录为外部输入。浸水时，斑块共享全池矿质库和同一氧预算。光合作用沿用原模型的外部无机碳边界并产氧，呼吸受可用氧限制，湿态衰老转入碎屑。露出水面的实验规则是暂停斑块代谢、保留库存，恢复浸水后再参加模型推进。画面按库存和浸水状态选择独立像素图层。 [生态实现](../../../../crates/eco-core/src/pond_moss.rs) · [浅塘共用账本](../../../../crates/eco-core/src/pond.rs)
- 修剪按输入的 mg C 等比例移出活体的 C/N/P，碎屑留在同一水体。移除斑块或承载物把对应的全部剩余库存转为碎屑。编辑历史恢复位置、库存和碎屑状态。复制现状、检查点保存原库存，从预设新建以定义剂量计一次初始库存。
- 现有容量、速率和暴露规则均为虚拟功能型实验默认值。原始研究测过光照、无机碳和温度对某种水生苔藓光合作用的影响。另一项研究发现干燥过程会影响复水后的恢复。工程采用浸水条件和共享资源预算来表达定性方向，真实物种、干露时长、温度耐受及恢复速率需独立标定。 [Maberly 1985 光合作用实验](https://nph.onlinelibrary.wiley.com/doi/abs/10.1111/j.1469-8137.1985.tb02765.x) · [Cruz de Carvalho 等 2011 干燥与复水实验](https://pubmed.ncbi.nlm.nih.gov/21399950/)

## 验证结果

| 检查 | 实际证据 |
| --- | --- |
| 独立账本 | Rust 用例核对初次投放、每种元素的库存移转、同水体矿质竞争、氧气不足时的共享上限、露出后的库存冻结、复水后恢复，及空斑块 v12 与 v11 的 2,500 步对照。 [内核用例](../../../../crates/eco-core/src/pond/tests.rs) · [运行层用例](../../../../crates/world-runtime/src/tests.rs) |
| Windows 放置与编辑 | WebView2 `158.0.4370.33`、单屏 150%。在同编号的主石和短根分别挂两种斑块，验证调整局部位置和随木移动。小垫从 3 修剪至 2 mg C，碎屑增加 1 mg C。随后移除小垫或主石各增加 2 mg C，移除短根增加 7.5 mg C，三次分别撤销后都回到原库存。 [放置预览](evidence/pond-moss/pond-moss-placement.png) · [两种承载物](evidence/pond-moss/pond-moss-two-carriers.png) · [原生记录](evidence/pond-moss/pond-moss-edit-checks.json) |
| 植物推进与回读 | 1 L 水量下两件斑块露出且保留库存，恢复原水位后在隔离试运行推进 120 模拟秒。小垫的 C 从 2.000 增至约 2.0023 mg，分枝绒簇从 7.500 增至约 7.5068 mg。采用前正式世界保持暂停。保存后实际重启，在编辑器、观赏窗和桌面组件核对同一世界的锚点，旧 v11 世界继续读取原沉木。 [露出状态](evidence/pond-moss/pond-moss-exposed.png) · [生长快照](evidence/pond-moss/pond-moss-grown.png) · [桌面组件](evidence/pond-moss/pond-moss-reopened-panel.png) · [退出](evidence/pond-moss/exit-checks.json) · [重开](evidence/pond-moss/pond-moss-reopen-checks.json) · [兼容](evidence/pond-moss/pond-moss-compat-checks.json) |
| 前端与回归 | 103 项 Rust 与 13 项布局用例通过，TypeScript 和 Vite 构建通过。Edge 界面夹具覆盖 1060×730 和 580×440，验证内容抽屉、承载物选择、Esc、修剪上界、试运行撤销。实际绘制夹具在 125% 和 150% 验证旧背景、露出状态和拖动承载物的附着画面。Vite 提示入口包大于 500 kB。 [界面记录](evidence/pond-moss/browser-ui-checks.json) · [窄窗](evidence/pond-moss/browser-placement-580x440.png) · [拖动夹具](evidence/pond-moss/browser-wood-carrier-drag-150.png) |

初次 Windows 回归遇到系统保留的调试端口，改用空闲端口后恢复。原生交互还发现同级面板的 React 键重复，修正后用全新页面和同一套动作重跑通过。验收使用隔离开发版。实机跨屏、安装包、真实物种和水流姿态待验证。

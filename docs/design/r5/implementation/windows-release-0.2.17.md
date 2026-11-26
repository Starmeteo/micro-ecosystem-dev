# R5 浅塘多剖面活体与水蚤观察 · 0.2.17

浅塘两侧都有米虾、附着苔藓和石面生物膜时可一起合池。Windows 安装包包含林下和浅塘两套场景。 [上一版](windows-release-0.2.16.md)

| 产物 | 大小 | SHA-256 |
| --- | ---: | --- |
| Windows x64 安装包（本地副本已清理） | 74,636,427 字节 | `0F2F40BF1CC0303037FA8CC6A54081934920F0FDF0E7727D942A6825EA45CCC7` |
| 0.2.17 独立程序（本地副本已清理） | 83,851,776 字节 | `3523117DCEBAA5E60FB6C258703C18A61C8DCB9C08B36AAD639F53B76A1FC7A4` |

## 使用方式

在「编辑环境 → 环境 → 编辑池岸形状 → 分合预览」画水道连接两片水域。预览逐水域显示米虾、附着苔藓和有膜主石计数。应用后返回场景，在画面下方切换「观景剖面」。副剖面可挂接或修剪苔藓，在「生物」中启用主石停靠和石面生物膜，也可在同一菜单投放米虾。

- 各剖面保留米虾个体、年龄和库存，便于在再次分池后识别个体。停靠和石面取食使用当前剖面的承载物。各剖面一起分配整池的氧气、氮磷和共用食物。水位改变时按新水深更新停靠和活动位置。
- 「生物 → 水蚤群 → 放大观察」读取整池水蚤实际数量与群体碳氮磷库存。画面代表点按每六只一处向上取整，最多三十二处。放大图用 2 mm 躯体参考标尺示意形态，代表点没有个体标识。
- 分合继续检查水量、格面积和受保护的剖面行。桌面组件默认观赏主剖面，可在编辑器切换查看其他剖面。保存了副剖面米虾或附着状态的新存档需要 0.2.17 或更新版本读取。 [浅塘模型](../../../../crates/eco-core/POND.md)

原始参考索引已可返回 R5 交互预览，独立标尺图恢复显示六条测量线。 [资料核对](evidence/review-materials/checks.json)

## 验证

Windows Rust 工作区 185 项回归、前端类型检查和生产构建通过。专项核对三剖面六块主石共用一份定殖预算，米虾和生产者共用氧气，氮、磷分别受限时各剖面按同一比例生长。两条剖面上同编号的主石按本地库存供食，米虾共用食物在一次分配中扣减。试运行、干涸、撤销后的编号保护、旧档和分池均覆盖到。 [工作区回归](evidence/living-sections-0.2.17/workspace-tests.txt) · [共享养分与氧气](evidence/living-sections-0.2.17/budgets-test.log) · [米虾摄食与干涸](evidence/living-sections-0.2.17/shrimp-tests.log) · [运行层试运行](evidence/living-sections-0.2.17/runtime-living-tests.log)

Windows 原生编辑器将两个 18 L 水域合为 36 L。两侧各三只米虾、一个附着斑块和一块有膜主石保留。副剖面修剪苔藓 1 mg C 并投放一只成体。试运行 32 步后，副剖面米虾年龄与位置、石面膜库存均有变化。放弃试运行后，按原库存和身份再次分池得到 3 只与 4 只米虾，撤销分池后保存重开仍可查看副剖面物件和四只米虾。 [合池预览](evidence/living-sections-0.2.17/living-merge-preview.png) · [副剖面苔藓](evidence/living-sections-0.2.17/living-secondary-moss.png) · [米虾与石面膜](evidence/living-sections-0.2.17/living-secondary-shrimp-film.png) · [分池预览](evidence/living-sections-0.2.17/living-split-preview.png) · [原生记录](evidence/living-sections-0.2.17/living-sections-checks.jsonl) · [重开画面](evidence/living-sections-0.2.17/living-sections-reopened.png) · [重开记录](evidence/living-sections-0.2.17/living-sections-reopen-checks.jsonl)

同一原生水域显示 96 只水蚤、16 个代表点及约 27.2 mg C、3.64 mg N 和 0.451 mg P。独立 Edge 用给定快照分别验证了干涸、实数为零、库存缺失和数量上限。 [水蚤原生截图](evidence/living-sections-0.2.17/daphnia-magnifier.png) · [Edge 核对](evidence/living-sections-0.2.17/daphnia-ui-checks.json)

Windows 11 x64 隔离新装和从 0.2.16 覆盖升级通过。林下与浅塘可保存重开。四个测试世界的存档在升级前后逐字节一致，旧浅塘新增的水蚤库存读数已与原始存档中的群体躯体和储备逐水域逐元素核对。设备矩阵和长跑校准沿用此前确认的验收范围。 [新装](evidence/living-sections-0.2.17/install-fresh.json) · [新装重开](evidence/living-sections-0.2.17/reopened-webview-check.jsonl) · [上一版种档](evidence/living-sections-0.2.17/prior-webview-check.jsonl) · [覆盖升级](evidence/living-sections-0.2.17/install-upgrade.json) · [升级回读](evidence/living-sections-0.2.17/upgraded-webview-check.jsonl)

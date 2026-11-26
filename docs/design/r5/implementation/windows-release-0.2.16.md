# R5 浅塘多剖面植根水草 · 0.2.16

浅塘两侧已有轮生细叶水草或带状叶水草时，可以合池保留各自的根部位置和库存。Windows 安装包包含林下与浅塘两套 R5 场景。 [上一版](windows-release-0.2.15.md)

| 产物 | 大小 | SHA-256 |
| --- | ---: | --- |
| Windows x64 安装包（本地副本已清理） | 74,614,748 字节 | `A7977AB5EBD3C24F382154F9A96D18AAF633E5BCBA1C21C6BB4323A787DD3859` |
| 0.2.16 独立程序（本地副本已清理） | 83,740,672 字节 | `463F68C8F374F084006B374B9DD87C26720335DE2946AE917F0FFF93237FB881` |

## 使用方式

在「编辑环境 → 环境 → 编辑池岸形状 → 分合预览」画水道连接两片水域。预览会显示每片结果水域的植根水草数量。应用后返回场景，在画面下方切换「观景剖面」，可从「内容」投放，或在「属性」选中、移动、修剪和移除该剖面上的水草。

- 水草沿用每条剖面最多八丛的摆放约束。不同剖面的同编号水草可以分别编辑，石木与水草根部仍需在同一剖面避让。修剪和移除把对应碳、氮、磷转入整池碎屑。
- 各剖面的水草按原有株型、当前库存和水深计算生长、呼吸需求。整池一次分配可用的氮、磷和氧气。分池时，水草及库存随根部所在剖面归属，未定位的水草库存按面积分配。
- 如果被并入的一侧还有米虾、活体附着苔藓或有库存的石面生物膜，合池仍会被拦截。包含副剖面水草的存档需要 0.2.16 或更新版本读取。 [浅塘模型](../../../../crates/eco-core/POND.md)

## 验证

Windows Rust 工作区 180 项回归、前端类型检查和生产构建通过。专项测试分别限制氮、磷和氧气，验证两条剖面上的水草与悬浮藻使用同一供给比例。内核和运行层核对局部根部碰撞、跨剖面同编号编辑、试运行采用、保存重开、预设重建及拆分前后的碳氮磷守恒。 [工作区回归](evidence/plant-sections-0.2.16/workspace-tests.txt) · [共享预算专项](evidence/plant-sections-0.2.16/budgets-test.log)

隔离 Edge 使用模拟回执验证合池、按水域预测水草数量、同编号水草独立编辑、属性中的库存变化和撤销重做。 [分合预览核对](evidence/plant-sections-0.2.16/edge-preview-checks.json) · [编辑核对](evidence/plant-sections-0.2.16/edge-editor-checks.json)

Windows 原生编辑器把一丛 50 mg C 的轮生细叶水草和另一侧 75 mg C 的带状叶水草合入同一 12 L 水域。副剖面水草移动后保持库存，修剪 15 mg C 进入整池碎屑。新增一丛再移除也完整转入碎屑。分池后，两个 6 L 水域分别保留 50 mg C 和约 60 mg C 的原有水草。恢复合池并保存后，新进程回读两条剖面及水草库存。 [合池预览](evidence/plant-sections-0.2.16/plants-merge-preview.png) · [副剖面编辑](evidence/plant-sections-0.2.16/plants-secondary-edited.png) · [分池预览](evidence/plant-sections-0.2.16/plants-split-preview.png) · [原生记录](evidence/plant-sections-0.2.16/plant-sections-checks.jsonl) · [重开画面](evidence/plant-sections-0.2.16/plants-sections-reopened.png) · [重开记录](evidence/plant-sections-0.2.16/plant-sections-reopen-checks.jsonl)

Windows 11 x64 隔离新装及从 0.2.15 覆盖升级通过。新装后林下和浅塘均可在新进程回读。升级前后四份旧测试世界逐字节一致，含旧版主副剖面沉叶的合池世界可继续切换并显示原副剖面。设备矩阵与长跑校准沿用已确认的验收范围。 [新装](evidence/plant-sections-0.2.16/install-fresh.json) · [新装重开](evidence/plant-sections-0.2.16/reopened-webview-check.jsonl) · [旧版种档](evidence/plant-sections-0.2.16/prior-webview-check.jsonl) · [覆盖升级](evidence/plant-sections-0.2.16/install-upgrade.json) · [升级回读](evidence/plant-sections-0.2.16/upgraded-webview-check.jsonl)

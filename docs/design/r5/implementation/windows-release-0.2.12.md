# R5 主石生物膜消耗与再生 · 0.2.12

浅塘新增主石独立生物膜库存，安装包同时包含 R5 林下和浅塘。 [上一版](windows-release-0.2.11.md)

| 产物 | 大小 | SHA-256 |
| --- | ---: | --- |
| Windows x64 安装包（本地副本已清理） | 74,572,797 字节 | `E048D2840A19DE8C67109F08564FF2EBDE0FB14CB5AF298E53F934AF099BFF00` |
| 0.2.12 独立程序（本地副本已清理） | 83,621,888 字节 | `F049EC54177CEFF7B8C4532CB6A4CBDB6207224EE05104C5FD103A372DA71FBD` |

## 启用和观察

在浅塘打开「编辑环境 → 生物 → 当前生物膜分配」，点击「启用石面生物膜」。按水域启用时，会从现有共用生物膜划出起始量，并启用主石停靠。未启用的水域沿用原本的共用库存规则。保存启用过的水域需要 0.2.12 或更新版本读取。

每块主石记录独立库存。米虾在有效停靠时，先从该主石的浸水可食份额扣减，再用整池共用库存补足。石面会随同池光照和养分逐步长膜，共用库存也可以少量重新附着。生长和呼吸仍进入整池的养分与氧预算。每块主石的浸水可食量按其浸水比例估算。此阶段的实验速率使用工程参数，水流与膜种群组成仍待建模。 [模型与参数](../../../../crates/eco-core/POND.md) · [附着藻类再生实验](https://pofflab.colostate.edu/wp-content/uploads/2019/08/Wellnitz_2006_Herbivorycurrentvelocityandalgalregrowth.pdf)

移动主石保留当前库存，新增主石从无膜开始，移除主石将剩余库存归还同水域的共用库存。干涸保留各块主石的库存，浸水可食量归零。复水恢复原库存的浸水份额，后续模拟再更新库存。属性和膜色取自同一生态状态。沉木仍沿用整池共用库存的当步归因。

## 验证

Windows Rust 工作区 167 项回归、前端类型检查和生产构建通过。回归覆盖局部缺货时按需求分摊、其它主石不被借用、共用库存补足、无观察者时同样扣减、暗处重新附着及有光时的养分与氧收支。另核对了移动、原子移除、旧档兼容、试运行、撤销和存档回读。设备矩阵与历史长跑专项沿用此前确认的范围。 [工作区回归](evidence/stone-film-0.2.12/workspace-tests.txt)

隔离 Edge 用模拟回执检查了启用入口和逐石读数。相同模拟步、相同整池总量下，逐石浸水库存变化仍会重绘对应主石。干涸时读数保留，石面的湿膜效果撤下。 [界面截图](evidence/stone-film-0.2.12/biology-current.png) · [检查记录](evidence/stone-film-0.2.12/edge-checks.json)

Windows 原生实测中，主石起始库存约 3.42857 mg C。捕捉到实际停靠摄食时，一次当步主石来源为 0.00002545 mg C，试运行前 2 模拟秒后的实际石面库存约 3.42824 mg C。干涸和复水保持该库存。排水移除了水中动物，复水后再推进 120 模拟秒，库存增至约 3.43008 mg C。采用结果后冷重开恢复了同一库存和可食量。 [取食截图](evidence/stone-film-0.2.12/stone-film-grazed.png) · [再生截图](evidence/stone-film-0.2.12/stone-film-recovered.png) · [实际读数](evidence/stone-film-0.2.12/stone-film-checks.jsonl) · [冷重开](evidence/stone-film-0.2.12/stone-film-reopen-checks.jsonl)

Windows 11 x64 新装和从 0.2.11 覆盖升级通过。覆盖安装前后三份旧测试存档逐字节一致，旧双水域中的浮叶、透光率和两种沉水植物完整恢复。新装后的两套世界重开及隔离卸载后保留存档通过。 [新装](evidence/stone-film-0.2.12/install-fresh.json) · [新装后重开](evidence/stone-film-0.2.12/reopened-webview-check.jsonl) · [升级](evidence/stone-film-0.2.12/install-upgrade.json) · [升级回读](evidence/stone-film-0.2.12/upgraded-webview-check.jsonl)

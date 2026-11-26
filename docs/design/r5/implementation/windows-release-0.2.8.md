# R5 沉木食源与浅塘静态布局 · 0.2.8

本版承接 0.2.7 的两套 R5 场景，接入米虾摄食来源和沿原观景剖面保留静态物件的分合池。 [上一版记录](windows-release-0.2.7.md)

| 产物 | 大小 | SHA-256 |
| --- | ---: | --- |
| Windows x64 安装包（本地副本已清理） | 74,526,071 字节 | `A84CF03E98FB462FA80B5CBCD49441A8586D541931D60DAFBA3B7DE20C84C8A3` |
| 0.2.8 独立程序（本地副本已清理） | 83,365,376 字节 | `6DD0586C18D8B6B9B19E05ABA1189CE49DFB555285C644DF902ED514AA5E907B` |

## 沉木表面食源

Rust 按当前水线与浸水承载面计算当步份额。已接触沉木的米虾获得实际食物后，同一沉木上的个体按摄入量比例分享该段木头的额度，余量记为整池共用份额。两部分合计为当步摄入，生物膜从现有库存扣减一次。源份额每步重新估算，适合表示当前浸水状态与摄食归属。

在「生物 → 接触沉木时的摄食」查看总摄入和两部分来源。旁边的生物膜分配与画面膜色使用同一份 Rust 投影。退水减少浸水份额，干涸后的生物膜库存显示在保留项里，移除沉木释放其显示份额。「来自沉木表面」表示这一步获得的食物来源。持续累积的局部耗竭和主石上的摄食接触留待后续模型。

隔离 Edge 在 576×324 画布和 4× 观察场景验证了守恒数据、同一步刷新、过期接触和浅水裁切。原生试运行捕捉到第二水域的米虾在一步内摄入约 0.0000249 mg C，该次全部来自所接触的沉木份额。退水至 0.8 L 后，主水域的沉木分配由约 8.79 mg C 降至 0.34 mg C，排空时 24 mg C 全进入保留项，复水恢复原份额，移除沉木后剩余份额分配到池底和主石。 [原生摄食](evidence/source-topology-0.2.8/wood-source-active.png) · [移除沉木](evidence/source-topology-0.2.8/food-carrier-removed.png) · [Edge 记录](evidence/source-topology-0.2.8/edge-food-source-checks.json)

## 沿原剖面保留静态布置

要合入空白水域，在环境面板选中要保留布置的水域，再进入「编辑池岸形状 → 分合预览」画出连接水道。所选水域的石头、沉木、造景及其附属关系继续留在原观景剖面上。切回水道分池时，原剖面保留布置，新水域从空布局开始。

有活体、附着活株或带位置的沉叶，以及多个水域分别放有物件的情形，仍待补齐俯视归属和资源转移规则。

原生编辑器将有主石、短根沉木和粗砂斑的一侧与无布置水域从 5 L、7 L 合成 12 L，再分成各 6 L。原剖面和三项物件的位置、ID 均一致，新池从空布局开始。把无布置水域作为合并后的保留侧时，预览显示对应拒绝原因。 [合池](evidence/source-topology-0.2.8/static-merge-ready.png) · [分池](evidence/source-topology-0.2.8/static-split-ready.png) · [选错侧](evidence/source-topology-0.2.8/static-wrong-side-blocked.png)

## 验收

Windows Rust 工作区 152 项回归通过，前端类型检查和生产构建通过。内核用例验证两段沉木、三只接触米虾及一只无位置米虾，共用一次库存扣减，所得来源合计等于每步真实分配。静态布局回归覆盖错误方向拒绝、父子引用、拆开后新池空布局、水量和溶氧守恒，以及保存和撤销分配新 ID。原生撤销、重做和两份世界冷重开通过。设备矩阵和历史成对种子专项沿用用户确认范围。 [工作区回归](evidence/source-topology-0.2.8/workspace-tests.txt) · [原生操作](evidence/source-topology-0.2.8/source-topology-checks.jsonl) · [冷重开](evidence/source-topology-0.2.8/source-topology-reopen-checks.jsonl)

Windows 11 x64 隔离新装和 0.2.7 覆盖升级通过。升级前后，林下、已布置双池和实际分过池的三份测试存档逐文件 SHA-256 一致。新版重开恢复全部持久字段，另核对新增生物膜投影的合计等于同池库存。两套场景的组件画布为 576×324。测试安装卸载后保留测试档。验收使用已有 WebView2 和当前用户静默装卸。 [新装](evidence/source-topology-0.2.8/install-fresh.json) · [覆盖升级](evidence/source-topology-0.2.8/install-upgrade.json) · [三份存档回读](evidence/source-topology-0.2.8/upgraded-webview-check.jsonl)

覆盖升级初次复测在两份浅塘之间切换时读到了上一份世界的画布。修正验收脚本的画布等待条件后，用同一安装包重跑通过。 [初次证据](../../../../.local/r5-installer-evidence/293ebd8ba4fa4e79bcb6b55faa4c9c76/installation.json)

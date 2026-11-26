# R5 浅塘分合池 · 0.2.7

本版将分合预览接入 Rust 权威编辑。世界库新增「空白浅塘 · R5 双水域」，用于先确定水域形状，再放入生物和物件。 [上一版分合预览](windows-release-0.2.6.md)

| 产物 | 大小 | SHA-256 |
| --- | ---: | --- |
| Windows x64 安装包（本地副本已清理） | 74,519,999 字节 | `90CEC533C3D05F2286A92930ABBC28B894638ED5199B6138AF43A45BB8E6CC50` |
| 0.2.7 独立程序（本地副本已清理） | 83,362,816 字节 | `FA38E05D5F38EE40A3B16B0740C93DA437BF738C10B5DCFCE50897DE40E9E75A` |

## 操作

在「编辑环境 → 环境 → 编辑池岸形状 → 分合预览」画水道连接两池，再点击「应用合池」。要再次切开，在同一工具中用「填岸地」填回窄口，再点击「应用分池」。每次应用是一条可撤销的编辑命令。

当前可提交的范围是无动物、活体植被和已布置物件的水域。已布置水域可以查看分合预览，应用前需要先解决对象归属。提交前会核对共边连通、每格面积、完整侧视剖面、水位和容量，以及全图的水域数量。

## 资源和身份

合并保留选中水域的身份与观景剖面，同时合计参与水域的水、溶氧、溶解物和无单独位置的池底库存。合并后的水温按水量加权。拆分保留原观景剖面所在水域的身份，并为其他水域分配新的稳定 ID，各自需要一个完整横剖面。各区按面积比例分水和充分混合库存，采用相同的初始水深。体积须达到 0.05 L 或为 0，全图最多四个水域。

历史存档按原有水域形状读入。写入正式分合结果后，保存会包含新的身份和剖面字段，使用 0.2.7 或之后支持该格式的版本打开这些结果。

## 验收

Rust 工作区 148 项通过。核心回归验证水、溶氧、碳氮磷、温度和账本，以及已有生物、不同格面积、容量不足、过浅、缺少完整剖面等拒绝路径。运行层另验证了重复命令、撤销重做、检查点恢复、分池 ID 分配下限、试运行采用、复制和预设另存。隔离 Edge 用受控回执复测了显式提交、失败保留草稿和重试。 [工作区回归](evidence/topology-commit-0.2.7/workspace-tests.txt) · [Edge 交互](evidence/topology-commit-0.2.7/edge-checks.json)

Windows 原生编辑器实测，初始 5 L／20 mg C 和 7 L／60 mg C 的两个水域合为 12 L／80 mg C。填回连接格后分成各 6 L／40 mg C，两次提交各产生一次编辑修订。选中水域保留原 ID 与观景剖面，新水域获得 `pond-3`。撤销、重做和冷重开恢复预期形状、库存和分池 ID。已布置的双水域仍能查看影响预览，应用按钮停用。 [合池待应用](evidence/topology-commit-0.2.7/topology-merge-ready.png) · [分池待应用](evidence/topology-commit-0.2.7/topology-split-ready.png) · [分池后](evidence/topology-commit-0.2.7/topology-restored-split.png) · [原生记录](evidence/topology-commit-0.2.7/topology-commit-checks.jsonl) · [冷重开](evidence/topology-commit-0.2.7/topology-commit-reopen-checks.jsonl)

Windows 11 x64 隔离新装及 0.2.6 → 0.2.7 覆盖升级均通过。18 个预设可读，林下和浅塘组件在本机为 576×324。覆盖前后测试存档逐文件 SHA-256 一致，冷重开保持两套世界的持久状态，含旧浅塘第二池的砂斑与沉叶。卸载临时安装后保留测试档。此处采用已有 WebView2 和当前用户静默装卸。 [新装](evidence/topology-commit-0.2.7/install-fresh.json) · [升级](evidence/topology-commit-0.2.7/install-upgrade.json) · [旧档回读](evidence/topology-commit-0.2.7/upgraded-webview-check.jsonl)

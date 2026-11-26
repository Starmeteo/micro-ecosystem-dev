# R5 浅塘活体水域分合 · 0.2.13

本版支持含活体的浅塘分池与合池，安装包同时包含林下和浅塘。 [上一版](windows-release-0.2.12.md)

| 产物 | 大小 | SHA-256 |
| --- | ---: | --- |
| Windows x64 安装包（本地副本已清理） | 74,581,244 字节 | `B1087211BCD92D786FC264507BF902C25ACC0BFAF7E4F6E8B1AF6744348D8B17` |
| 0.2.13 独立程序（本地副本已清理） | 83,645,952 字节 | `A304165BBCDF2E8AA8985A07D479AAE7C76D4B628AFCD208840BE65CB4032380` |

## 使用方式

打开「编辑环境 → 环境 → 编辑池岸形状 → 分合预览」。画水道连接两片水域可预览合池，填岸切断水道可预览分池。提交前可逐区查看水量、水深、米虾、水蚤及浮叶数量。每片水域仍需要一条完整的观景剖面。

- 浮叶按叶心实际所在的俯视格归属，随位置保留库存。删除叶心所在格会阻止提交。合池保留选中水域的浮叶编号，给合入的浮叶分配新编号。
- 水蚤随充分混合的水域按面积分配为整数。水量、溶氧、矿质与共用库存按剩余水域面积分摊。碳、氮、磷跨水域的内部转移独立记账。
- 米虾和石木、水草根部、附着物、石面生物膜与带位置的沉叶留在原观景剖面。水深改变时重新检查游动空间和停靠位置。合池先选要保留布置的一侧。另一侧如果已有依赖自己观景剖面的生物或物件，系统会拦截合池。

参与合池的每格面积须一致，变更后须满足水深、容量和个体数量限制。含内部分摊账本的存档需要 0.2.13 或更新版本读取。 [模型规则](../../../../crates/eco-core/POND.md)

## 验证

Windows Rust 工作区 170 项回归、前端类型检查与生产构建通过。专项用例核对了带主石停靠的米虾重新投影、浮叶按格归属、水蚤整数分配、同水域与全世界的物质收支、容量和数量超限的原子拒绝。运行层用例核对了试运行隔离、采用、撤销重做与冷重开。设备矩阵与历史长跑专项沿用此前确认范围。 [工作区回归](evidence/live-topology-0.2.13/workspace-tests.txt) · [专项回归](evidence/live-topology-0.2.13/core-topology-tests.txt)

隔离 Edge 使用模拟提交回执检查了预览、分合按钮、整数分配及叶心阻挡标记。 [界面预览](evidence/live-topology-0.2.13/edge-split-preview.png) · [前端检查](evidence/live-topology-0.2.13/edge-checks.json)

Windows 原生测试分别用键盘和鼠标把 5 L 和 7 L 两个有活体的水域合并，再分成两个 6 L 水域。7 只水蚤分为 4 只和 3 只，两片浮叶保留原坐标和库存，各归一侧。米虾、主石、水草和含 20 mg C 的定位沉叶留在原剖面。核对了拒绝切过浮叶中心和拒绝丢弃另一条已占用的剖面，撤销重做和冷重开通过。 [合池预览](evidence/live-topology-0.2.13/live-merge-preview.png) · [分池预览](evidence/live-topology-0.2.13/live-split-preview.png) · [阻挡提示](evidence/live-topology-0.2.13/live-cut-center-blocked.png) · [键盘记录](evidence/live-topology-0.2.13/live-topology-checks.jsonl) · [鼠标记录](evidence/live-topology-0.2.13/pointer-checks.jsonl) · [冷重开](evidence/live-topology-0.2.13/live-topology-reopen-checks.jsonl)

Windows 11 x64 新装与从 0.2.12 覆盖升级通过。安装前后三份旧测试存档逐字节一致。含浮叶、带状叶、轮生水草和活体石面生物膜的双水域存档在升级后完整回读，新装后的林下和浅塘均可重开。 [新装](evidence/live-topology-0.2.13/install-fresh.json) · [旧档升级](evidence/live-topology-0.2.13/install-upgrade.json) · [升级回读](evidence/live-topology-0.2.13/upgraded-webview-check.jsonl)

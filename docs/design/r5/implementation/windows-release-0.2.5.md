# R5 底床材质与附着生物膜 · 0.2.5

本版承接 0.2.4 的林下和浅塘场景，补齐浅塘底床的两类砂斑，并把附着生物膜显示到池底、主石和沉木。 [上一版记录](windows-release-0.2.4.md)

| 产物 | 大小 | SHA-256 |
| --- | ---: | --- |
| Windows x64 安装包（本地副本已清理） | 74,509,277 字节 | `284790F179FDED2737E8352A568530F454E5B0DB9313BDA3BE8C7C34444C0E60` |
| 0.2.5 独立程序（本地副本已清理） | 83,331,072 字节 | `24BCBF39BA9BED6D7D3ADF12E581E77F38F4716FE61692C098F973EFF82F8BC4` |

## P04 底床材质

在「世界编辑器 → 编辑环境 → 内容 → 底床材质」选择细砂斑、粗砂斑或小砾砂斑。细砂斑与粗砂斑各有独立透明图层，以 100 mm 主轴为同屏尺度。细砂较浅，颗粒较密，粗砂较深，颗粒面较大。边缘沿底床疏开，水下叠加原有光纹，退水时保留同一锚点。

三种材质共用底床落位校验，整块放置、移动和撤销。造景沿用池塘原有容量、养分及通行规则。

原生编辑器在有水的主池分别放置细砂和粗砂，撤销、重做恢复同一实例。悬空点按未提交。排空后另放一处细砂，补水和冷重开保有三处锚点。摆放前后的水体读数一致，第二池状态一致。隔离 Edge 另覆盖画布点选、露底拖动及拖出底床拒绝。 [细砂](evidence/sand-film-0.2.5/fine-sand-wet.png) · [粗砂](evidence/sand-film-0.2.5/coarse-sand-wet.png) · [露底摆放](evidence/sand-film-0.2.5/fine-sand-dry.png) · [Edge 操作](evidence/sand-film-0.2.5/sand-edge-checks.json)

## P12 附着生物膜

在「世界编辑器 → 生物 → 附着生物膜的画面分配」可展开读数。石木上的薄膜随承载物移动，使用精灵轮廓和实际水线裁切。整池的模型库存继续以 mg C 记录。画面按侧视承载物的浸水权重分配膜色，池底保留余量，干涸时用单独一行显示保留的整池库存。

原生样例的 24 mg C 在初始水位分配为池底 13.04、主石 2.17、沉木 8.79 mg C。退至 0.8 L 时两件承载物分配均下降，干涸时保留 24 mg C 且关闭膜色，恢复原水量后恢复原分配。第二水域状态一致。隔离 Edge 对实际绘制链另做了逐像素比较，验证膜色限制在水线以下，石木移动时薄膜跟随，干底的零库存与有库存画面一致。 [原生满水](evidence/sand-film-0.2.5/film-full.png) · [退水](evidence/sand-film-0.2.5/film-shallow.png) · [干涸](evidence/sand-film-0.2.5/film-dry.png) · [Edge 数值与像素](evidence/sand-film-0.2.5/film-edge-checks.json)

此批交付覆盖实际库存驱动的膜色。米虾仍按整池库存摄食。后续若要展示米虾从具体木石刮食，需要把接触位置与真正的资源转移同时送入反馈。 [实施计划](../review/05-implementation-plan.md)

## 验收

Windows Rust 工作区 143 项通过，前端类型检查与生产构建通过。底床回归覆盖三种材质、边界拒绝、撤销重做、复制、试运行和存档。设备矩阵与成对种子专项沿用用户确认的范围。 [Rust 回归](evidence/sand-film-0.2.5/workspace-tests.txt) · [原生记录](evidence/sand-film-0.2.5/sand-film-checks.jsonl) · [冷重开](evidence/sand-film-0.2.5/sand-film-reopen-checks.jsonl)

Windows 11 x64 新装与 0.2.4 → 0.2.5 覆盖升级均通过。安装包创建和重开两套场景，组件画布均为 576×324。升级保留双池水量、下放的沉叶、旧小砾砂斑及林下修剪蕨丛。安装前后存档逐文件 SHA-256 相同，两套世界冷重开后的持久状态逐项相同。卸载临时安装后保留测试档。本机使用已有 WebView2 与当前用户静默装卸。 [新装](evidence/sand-film-0.2.5/install-fresh.json) · [升级](evidence/sand-film-0.2.5/install-upgrade.json) · [旧档回读](evidence/sand-film-0.2.5/upgraded-webview-check.jsonl) · [升级后第二池](evidence/sand-film-0.2.5/upgraded-second-basin.png)

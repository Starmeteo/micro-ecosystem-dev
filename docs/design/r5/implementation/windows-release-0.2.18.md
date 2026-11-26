# R5 枝根桥、碎叶刷和浅塘通道 · 0.2.18

这一批交付 F05、F06、P03 和 P06 的可编辑物件及生态反馈。Windows x64 安装包同时包含 R5 林下和浅水池塘。 [上一版](windows-release-0.2.17.md)

## 产物

| 产物 | 大小 | SHA-256 |
| --- | ---: | --- |
| Windows x64 安装包（本地副本已清理） | 78,750,894 字节 | `A15D2CBC60F956D7191289848D93DACE47F12901EBD15E3392812B7BBBAD5D20` |
| 0.2.18 独立程序（本地副本已清理） | 87,954,432 字节 | `E0A1C335E190B74C8E9BF2D62C3EF94FB6D88A87BB4A32E8462C91DDD844BCD5` |

## 操作与边界

| 内容 | 编辑入口 | 空间与生态效果 |
| --- | --- | --- |
| F05 弯曲根拱、短枝桥 | 林下「编辑环境 → 内容」放置，选中后可移动、镜像和移除 | 跨度分别为 140 mm、200 mm，桥下净空分别为 65 × 12 mm、90 × 22 mm。两端接地实心，鼠妇按地表路径从桥下穿行，画面按前后遮挡显示。桥顶攀爬属于后续扩展。 |
| F06 碎叶多格铺设 | 林下「编辑环境 → 内容」，输入单笔剂量，选择「铺设碎叶」后拖过土格 | 一笔 0.1–240 mg C，由有效且不同的格子平分。现有落叶斑块共用最多 16 格。一笔作为一次撤销，单格可移动和取出。枯落物接受取食和分解，局部土水和库存变化驱动碎叶外观。 |
| P03 稀疏悬根、盘根网 | 浅塘「编辑环境 → 内容」，选择规格后点池底 | 宽度参考 120 mm、180 mm。根条有局部实心避障，米虾只在浸水且净空足够的孔隙内活动。 |
| P06 扁石洞 | 浅塘「编辑环境 → 内容」选择扁石洞，在可用池底放置 | 首个规格宽 110 mm。石顶和洞口使用各自的碰撞区。放水后，水下开口允许米虾游过，石顶仍会阻挡。 |

浅塘物件可换到不同的空位，继续随所在观景剖面参与分合。水位决定浸没范围，同一水域继续共享统一水量、溶氧和库存。根网与扁石洞本批承担结构与通行，附着苔藓、附着食源和停靠仍使用相应的既有承载物。使用上述新增预设或碎叶刷的存档需要 0.2.18 或更新版本读取。 [林下模型](../../../../crates/eco-core/README.md) · [浅塘模型](../../../../crates/eco-core/POND.md)

## 素材

使用内置 ImageGen 独立生成透明物件。生成要求为贴地的扭曲根拱、两脚支撑的非对称短枝桥，以及四条细根的稀疏悬根、有两处较宽通道的交错根网、薄石盖架在两端低支座上的扁石洞。桥下和水下开口需要真正的透明空间。材质采用 R5 的细像素笔触、湿木棕色与青蓝灰石色。各物件经裁切和分层后按同一场景坐标定位，投影接地点与碰撞或遮挡对齐。碎叶取自已有 R5 叶片素材并单独显示磨损。 [根拱](../../../../apps/desktop/src/render/assets/r5/forest-composed/bridge-root.png) · [短枝桥](../../../../apps/desktop/src/render/assets/r5/forest-composed/bridge-branch.png) · [稀疏悬根](../../../../apps/desktop/src/render/assets/r5/pond-composed/hanging-roots.png) · [盘根网](../../../../apps/desktop/src/render/assets/r5/pond-composed/root-network.png) · [扁石洞](../../../../apps/desktop/src/render/assets/r5/pond-composed/flat-stone-cave.png) · [浅塘共享轮廓](../../../../apps/desktop/src/render/assets/r5/pond-composed/structure-profiles.json)

## 验证

Windows Rust 工作区通过 191 项测试，TypeScript 类型检查及 NSIS 生产构建均通过。专项覆盖鼠妇通过两种桥、碎叶输入与消耗的守恒、米虾穿过三个结构的实际游动、石顶和根条的实心阻挡，以及浅塘物件跨剖面分合、排水补水和读档。设备矩阵和长跑校准沿用此前确认的范围。 [工作区测试](evidence/structures-litter-0.2.18/workspace-tests.txt) · [枝根桥](evidence/structures-litter-0.2.18/forest-bridge-tests.log) · [桥下透明净空](evidence/structures-litter-0.2.18/forest-bridge-alpha.json) · [碎叶账本](evidence/structures-litter-0.2.18/forest-fragment-tests.log) · [浅塘结构](evidence/structures-litter-0.2.18/pond-structure-tests.log)

Windows 原生编辑器在空白林下手动放置两种桥，并用一次鼠标拖动将 35 mg C 铺到 9 格。整笔撤销将库存恢复原值，重做恢复同一批落点和库存。浅塘中独立放置三个物件，补入 18 L 水后石洞从露出变为浸没。鼠标选点验证石顶阻挡、洞口可投放一只米虾。盘根网可移动，并通过撤销与重做准确恢复位置。 [林下碎叶实拍](evidence/structures-litter-0.2.18/forest-fragments-native.png) · [浅塘干床](evidence/structures-litter-0.2.18/pond-three-structures-dry.png) · [补水后的三个物件](evidence/structures-litter-0.2.18/pond-three-structures-wet.png) · [洞口内米虾](evidence/structures-litter-0.2.18/pond-cave-shrimp.png) · [操作记录](evidence/structures-litter-0.2.18/structures-litter-checks.jsonl)

原生进程退出后，使用同一个隔离档重新启动，林下与浅塘都恢复了上述物件、位置及生态状态。 [林下重开](evidence/structures-litter-0.2.18/forest-structures-reopened.png) · [浅塘重开](evidence/structures-litter-0.2.18/pond-structures-reopened.png) · [重开记录](evidence/structures-litter-0.2.18/structures-litter-reopen-checks.jsonl)

同一隔离档的桌面组件已在系统缩放 150% 下分别抓取 576 × 324 的原生画布，可以看到桥下空地和水下通道。测试画面使用空白场景，以便辨认各个物件。 [林下组件](evidence/structures-litter-0.2.18/forest-structures-component.png) · [浅塘组件](evidence/structures-litter-0.2.18/pond-structures-component.png) · [组件输出记录](evidence/structures-litter-0.2.18/structures-litter-panels-checks.jsonl)

Windows 11 x64 隔离新装和从 0.2.17 覆盖升级通过。四个升级测试世界的存档逐字节一致，重开后的生态状态散列全部匹配。测试卸载也保留了存档。安装包是本地未签名构建。 [新装](evidence/structures-litter-0.2.18/install-fresh.json) · [新装重开](evidence/structures-litter-0.2.18/reopened-webview-check.jsonl) · [覆盖升级](evidence/structures-litter-0.2.18/install-upgrade.json) · [0.2.17 种档](evidence/structures-litter-0.2.18/prior-webview-check.jsonl) · [升级回读](evidence/structures-litter-0.2.18/upgraded-webview-check.jsonl)

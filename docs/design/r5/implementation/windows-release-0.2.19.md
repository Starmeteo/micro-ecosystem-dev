# R5 种壳、菌丝、挺水植物与候选螺 · 0.2.19

本批覆盖 F08、F11、P10、F14 和 P15。种壳与挺水植物有独立编辑入口，菌丝从当前世界读取土格状态，两种候选螺在生物目录中查看。 [R5 内容矩阵](../design/r5-content-expansion.csv) · [0.2.18](windows-release-0.2.18.md)

## 产物

| 产物 | 大小 | SHA-256 |
| --- | ---: | --- |
| [Windows x64 安装包](../../../../target/release/bundle/nsis/微缩生态_0.2.19_x64-setup.exe) | 84,050,113 字节 | `67FCD7A0DD5E4906E6BFF621CC6D3600EE1BCF1A345D9AFE0609CBABE271FF37` |
| [0.2.19 独立程序](../../../../.local/archive-r5-0.2.19/micro-ecosystem-desktop.exe) | 93,316,096 字节 | `54D4E9C5EF8C085AEA642EED0CC64BBF760A22F9512DAD5682C174ACE26B053C` |

## 进入方式

| 内容 | 入口 | 行为 |
| --- | --- | --- |
| F08 种壳与枯果 | 林下「编辑环境 → 内容 → 地表点缀」 | 空种壳、橡实壳各有三档组合，分别含 2／4／7 枚。选落点后可移动、移除、撤销，摆件只计入场景布置。 |
| F11 菌丝表现 | 林下「属性」选朽木段或树皮庇护物，再展开「表面菌丝表现」 | 读取木材锚点土格的分解者碳库存。零库存隐藏，其余显示 5／15／30 mm 三档。 |
| P10 岸边莎草型叶丛 | 浅塘「编辑环境 → 内容 → 岸边莎草」 | 在俯视图选岸格，可移动、移出和撤销，同一水域最多八丛。三档 150／220／300 mm 株高，根团直径 25／40／60 mm。根颈落在岸与水的相邻格边，水位改变根区接水。 |
| F14 小型陆生蜗牛 | 林下「生物 → 小型陆生蜗牛 → 候选预览」 | 同尺看伸展与缩壳，壳宽参考 6 mm，伸展参考长 10 mm。 |
| P15 小型淡水螺 | 浅塘「生物 → 小型淡水螺 → 候选预览」 | 同尺看伸展与缩壳，壳宽参考 8 mm，伸展参考长 12 mm。 |

## 资源规则和依据

- 种壳的件长参照 5／12／20 mm，组合跨度 30／50／80 mm。壳中没有可消耗库存。菌丝使用真实土格库存驱动纹饰，阈值 0.8 与 3.2 mg C 是表现用档位，土格分解者仍按既有模型推进。
- 挺水植物采用莎草功能型。湿岸和浅水、窄叶和根茎形态参考水生莎草资料。剖面根颈设于池底上 36 mm，根深为根团直径的一半。离水时保留库存，根浸水后才从池水分配氮磷。叶片气体交换进入空气边界，浅塘仍使用整池混合的溶氧和养分。栽植与取出按外部碳、氮、磷输入与输出记账。生长速率、植物体比例及根颈高度为功能型默认值，未按物种校准。 [USDA NRCS 水生莎草指南](https://www.nrcs.usda.gov/plantmaterials/idpmcpg10239.pdf)
- 陆螺沿用 *Discus rotundatus* 的低平壳、红棕径纹及灰蓝色软体。湿林与落叶层资料支持形态与观察背景。田野和实验室研究记录了部分生活史，本项目尚缺所选食物分类与现有植物、分解者的资源分配验证。 [USDA 鉴定资料](https://idtools.org/mollusk/index.cfm?entityID=8213&packageID=1178) · [卡内基自然历史博物馆](https://carnegiemnh.org/mollusks/discus-rotundatus/) · [陆螺生活史研究](https://doi.org/10.12657/folmal.007.001)
- 淡水螺沿用 *Planorbella scalaris* 的抬高平顶螺塔和近竖直壳口。鉴定资料支持 8 mm 候选锚点的量级。入池前仍需本种的食源、呼吸、繁殖及同池兼容性验证。两种螺都按矩阵开放候选预览，投放列表仍使用已具备生态模型的动物。 [佛罗里达自然历史博物馆鉴定表第 87(b) 条](https://www.floridamuseum.ufl.edu/iz/resources/florida-snails/)

## 素材

种壳与螺的透明单体素材均使用内置 ImageGen 生成。种壳采用独立的干裂锯齿空壳和有抬起鳞片的半枚橡实杯，两者各保留壳口阴影、地面接触方向和泥土色阶。两种螺各为一张透明双帧图，提示包括陆螺的低平盘壳、红棕径纹和灰蓝足，淡水螺的平顶阶梯状螺塔、近竖直壳口和湿润琥珀色。两图左为贴地伸展，右为缩进外壳。预览按同一把标尺绘制，并保持同种切换时的壳宽。 [空种壳](../../../../apps/desktop/src/render/assets/r5/forest-composed/seed-shell.png) · [橡实壳](../../../../apps/desktop/src/render/assets/r5/forest-composed/acorn-shell.png) · [陆螺](../../../../apps/desktop/src/render/assets/r5/snails/land-snail.png) · [淡水螺](../../../../apps/desktop/src/render/assets/r5/snails/freshwater-snail.png)

莎草另用内置 ImageGen 制作两张透明素材。叶丛提示强调同一根颈、细窄弧叶、黄绿色像素高光和独立轮廓。根坡提示强调细根埋入颗粒岸坡、自然坡脚和无背景。叶片与根坡共用同一锚点，细根深度按库存所属的植株规格显示。坡脚取样原有池床纹理并渐变接地，水线分别决定岸上与浸水着色。高株可被原相机画幅裁切。 [叶丛素材](../../../../apps/desktop/src/render/assets/r5/pond-composed/sedge-shoot.png) · [根坡素材](../../../../apps/desktop/src/render/assets/r5/pond-composed/sedge-bank-roots.png) · [莎草绘制](../../../../apps/desktop/src/render/r5/pondSedgeArt.ts)

菌丝纹饰只贴在已定义的朽木段、短拱树皮和裂口半筒上，透过透明像素裁切轮廓。 [菌丝画面示例](evidence/botany-candidates-0.2.19/mycelium-30mm.png)

## 专项验证

TypeScript 类型检查通过。Windows Rust 工作区本轮运行 194 项测试并全部通过。设备矩阵和长跑校准沿用此前确认的验收范围。 [Rust 工作区回归](evidence/botany-candidates-0.2.19/workspace-tests.txt)

- 种壳内核测试覆盖六种组合、非法边界拒绝、移动、撤销、移除、保存恢复，以及同输入推进时的库存和鼠妇一致。 [种壳测试](evidence/botany-candidates-0.2.19/seed-shells-core.log)
- 菌丝内核测试覆盖土格隔离、库存来源与碳氮磷守恒。独立 Edge 将同一木材下的分解者库存依次改变，验证 5／15／30／15 mm 后隐藏。 [菌丝内核](evidence/botany-candidates-0.2.19/mycelium-core.log) · [菌丝 Edge 验证](evidence/botany-candidates-0.2.19/mycelium-edge.json)
- 莎草内核测试验证干湿切换、氮磷竞争、移植不改库存、移出记账和存档。额外的公共内核接口用例验证两株植物在分合池保留岸边锚点与库存，合池若淹没已栽植的岸格则拒绝。 [莎草内核](evidence/botany-candidates-0.2.19/shore-sedge-core.log) · [莎草分合池](evidence/botany-candidates-0.2.19/shore-sedge-topology.log)

使用新种壳或莎草的档案需要 0.2.19 或更新版本。菌丝读数从旧档已有的土格库存推导，候选预览使用界面局部状态。 [林下存档与资源](../../../../crates/eco-core/README.md) · [浅塘存档与资源](../../../../crates/eco-core/POND.md)

## Windows 原生操作

- 林下实际放入一组 7 枚橡实壳，撤销后消失，重做恢复同一物件和位置。碳氮磷等生态读数保持原值。短拱树皮读取第 2 排第 2 列的分解者库存 2.4 mg C，木面呈现 15 mm 档。 [橡实壳布置](evidence/botany-candidates-0.2.19/forest-seed-shells.png) · [木面读数](evidence/botany-candidates-0.2.19/forest-mycelium-object.png)
- 浅塘可从俯视岸图栽植、沿岸移植并撤销。大丛保持同一身份与库存，测试池水由 0 到 6 再到 18 L 时，根区浸水由 0% 到 63% 再到 100%。画布和属性均核对了水位。 [岸图](evidence/botany-candidates-0.2.19/pond-sedge-map.png) · [干床](evidence/botany-candidates-0.2.19/pond-sedge-dry.png) · [部分浸水](evidence/botany-candidates-0.2.19/pond-sedge-partial.png) · [根区完全浸水](evidence/botany-candidates-0.2.19/pond-sedge-wet.png)
- 陆螺、淡水螺分别切换两态。按原生画布像素核对，同种的壳宽标尺保持一致。切换预览后生态快照与投放选项保持原值。 [陆螺伸展](evidence/botany-candidates-0.2.19/forest-snail-extended.png) · [陆螺缩壳](evidence/botany-candidates-0.2.19/forest-snail-withdrawn.png) · [淡水螺伸展](evidence/botany-candidates-0.2.19/pond-snail-extended.png) · [淡水螺缩壳](evidence/botany-candidates-0.2.19/pond-snail-withdrawn.png) · [操作记录](evidence/botany-candidates-0.2.19/botany-checks.jsonl)
- 退出后启动新的原生进程，林下与浅塘均恢复上述布置与生态状态。系统缩放 150%，两套组件实际画布为 576 × 324。浅塘验收从空白预设开始，比对时可看清根团和水线。 [林下组件](evidence/botany-candidates-0.2.19/forest-botany-component.png) · [浅塘组件](evidence/botany-candidates-0.2.19/pond-botany-component.png) · [重开记录](evidence/botany-candidates-0.2.19/botany-reopen-checks.jsonl)

## 安装与覆盖升级

Windows 11 x64 隔离新装、两套场景建档及重开、从 0.2.18 覆盖升级和卸载均通过。覆盖升级的六个世界含上一版枝根桥、碎叶、根网、扁石洞以及多剖面活体。六份存档逐字节一致，打开后的原生态状态散列一致。两份林下档新增的分解者读数逐格核对了原始检查点。验证使用的安装包散列与交付文件一致，安装包为本地未签名构建。 [新装](evidence/botany-candidates-0.2.19/install-fresh.json) · [新装回读](evidence/botany-candidates-0.2.19/reopened-webview-check.jsonl) · [升级](evidence/botany-candidates-0.2.19/install-upgrade.json) · [原 0.2.18 种档](evidence/botany-candidates-0.2.19/prior-webview-check.jsonl) · [升级回读](evidence/botany-candidates-0.2.19/upgraded-webview-check.jsonl)

# 微缩生态 · Windows

当前提供 0.2.20 Windows x64 本地安装包，内含可分层编辑的 R5 林下与浅水池塘。首次打开进入新版林下。世界库和快捷新建可创建两套新版场景。现有存档继续按原模型版本打开。 [0.2.20 交付验收](docs/design/r5/implementation/windows-release-0.2.20.md) · [林下模型说明](crates/eco-core/README.md) · [浅塘模型说明](crates/eco-core/POND.md)

## 安装与启动

本地安装包面向 Windows 11 x64 的当前用户。安装后从系统托盘打开「世界编辑器」，即可选择林下或浅水池塘。通过托盘的「保存并退出」结束应用。安装包为未签名本地构建。 [Windows 安装包](target/release/bundle/nsis/微缩生态_0.2.20_x64-setup.exe) · [免安装程序](target/release/micro-ecosystem-desktop.exe) · [M4 历史安装记录与构建步骤](docs/m4/windows-install.md)

文档中指向 `target/` 和 `.local/` 的链接是本地构建或验收路径。从 GitHub 克隆后，可按下方步骤从源码构建安装包。外部原始风格参考图随本地交接资料保管，仓库保留文字索引和项目自有素材。

托盘和「组件设置」均可添加生态组件，并可逐张绑定世界。右键组件可在原地调整大小。编辑器的「环境」可展开观察趋势，「生物」可定位并取出鼠妇和米虾，浅塘「属性」可取出选中的沉叶。 [首版交互补全](docs/design/r5/implementation/windows-release-0.2.20.md)

进入编辑后，「内容」提供林下落叶笔刷、活体地表苔藓和浅塘轮生细叶水草。「生物」可以按数量投放鼠妇、米虾和水蚤。移动活体斑块保留其 ID 和库存。 [实际操作与验收](docs/design/r5/implementation/editor-followup-verification.md)

林下「内容 → 地表点缀」可独立放置种壳。选朽木或树皮可查看菌丝纹饰的土格来源。浅塘「内容 → 岸边莎草」可沿俯视池岸种植和移植。两套场景的「生物」分别提供陆螺和淡水螺的候选形态。 [种壳、菌丝、莎草与候选螺](docs/design/r5/implementation/windows-release-0.2.19.md)

在 R5 林下「内容」可放置根拱、枝根桥及铺设碎叶。一笔可跨多个土格，输入量由有效格平分。浅塘「内容」可放置稀疏悬根、盘根网和扁石洞。米虾可从浸水的开口穿过。 [桥、碎叶和浅塘通道](docs/design/r5/implementation/windows-release-0.2.18.md)

林下「内容」可用基质笔刷改变土格储水和蒸发。浅塘「环境 → 编辑池岸形状」可从俯视图调整面积并保留现存水量。 [基质与池岸笔刷](docs/design/r5/implementation/windows-release-0.2.2.md)

林下「内容」可种植、移植和修剪会遮阴的活体蕨丛，浅塘「内容」可在池底投放有实际碎屑库存的沉叶。要体验两池独立编辑，从世界库新建「浅水池塘 · R5 双水域」。 [蕨丛、沉叶与双水域](docs/design/r5/implementation/windows-release-0.2.3.md)

林下「生物 → 弹尾虫群」可查看左右区的库存和 4× 观察点。 [群体观察](docs/design/r5/implementation/windows-release-0.2.4.md)

浅塘「内容 → 底床材质」可摆放细砂、粗砂和小砾砂斑。「生物」可展开生物膜的画面分配，观察浸水石木和池底的膜色变化。 [底床材质与生物膜](docs/design/r5/implementation/windows-release-0.2.5.md)

浅塘「环境 → 编辑池岸形状 → 分合预览」可试画水道或填岸，查看预计水域、容量及水量。「生物」可查看米虾接触沉木期间从整池生物膜获得的食物量。 [分合池预览与米虾摄食](docs/design/r5/implementation/windows-release-0.2.6.md)

从世界库新建「空白浅塘 · R5 双水域」，进入分合预览后可对符合条件的空白水域执行「应用合池」或「应用分池」。 [正式分合与资源规则](docs/design/r5/implementation/windows-release-0.2.7.md)

浅塘「生物」可查看米虾当步摄入的沉木表面份额和整池共用份额。静态分合时选中要保留布置的水域，原观景剖面会保留石木与造景。 [沉木食源与静态分合](docs/design/r5/implementation/windows-release-0.2.8.md)

新建 R5 浅塘的米虾可在浸水主石上停靠，并在「生物 → 主石停靠与摄食」查看当步摄入来源。已有 R5 浅塘可进入环境编辑，在同一位置为当前水域启用。 [主石停靠与验收](docs/design/r5/implementation/windows-release-0.2.9.md)

浅塘编辑器的「内容」可投放三档带状叶水草。在池底放置后，从画面或「属性」选中斑块，即可移动、修剪或移除，并查看碳、氮、磷库存及水中透光比例。 [带状叶水草与验收](docs/design/r5/implementation/windows-release-0.2.10.md)

在浅塘「内容 → 编辑活体浮叶」可按俯视池岸格网放置、移动和移除浮叶。水面遮盖重叠面积只计一次，碳氮磷和入水光均取自对应水域。 [活体浮叶与验收](docs/design/r5/implementation/windows-release-0.2.11.md)

在浅塘「编辑环境 → 生物 → 当前生物膜分配」可为所选水域启用石面生物膜。米虾停靠并取食时扣减该主石的实际库存，后续增长与重新附着可补充库存，石面数值和画面取自同一份生态状态。 [石面生物膜与验收](docs/design/r5/implementation/windows-release-0.2.12.md)

浅塘「编辑环境 → 环境 → 编辑池岸形状 → 分合预览」支持含活体的水域分合。浮叶按格网落点归属，水蚤按面积分配，米虾与原剖面的水草和石木保留。合池前请选中要保留布置的水域。 [活体分合规则与验收](docs/design/r5/implementation/windows-release-0.2.13.md)

合池也可保留另一侧的静态石块、沉木和造景。应用后在画面下方切换「观景剖面」，可独立编辑副剖面的布置。副剖面与主剖面共用水量和水质。 [多剖面编辑与分合验收](docs/design/r5/implementation/windows-release-0.2.14.md)

两侧的定位沉叶可以一起合池，也可以在各自剖面投放、移动并查看剩余库存。再次分池时，沉叶碎屑会按落点分回对应水域。 [多剖面沉叶与验收](docs/design/r5/implementation/windows-release-0.2.15.md)

两侧的轮生细叶水草和带状叶水草也可一起合池。各剖面可分别投放、移动、修剪和移除，整池统一分配生长所需的氮磷和呼吸所需的氧气。 [多剖面水草与验收](docs/design/r5/implementation/windows-release-0.2.16.md)

两侧的米虾、附着苔藓和石面生物膜可以一起合池并继续运行。再次分池时，米虾编号和石木附着状态随所在剖面归属。「生物 → 水蚤群 → 放大观察」可查看整池群体数量、真实库存及带标尺的形态示意。 [多剖面活体与水蚤验收](docs/design/r5/implementation/windows-release-0.2.17.md)

从源码运行需 Microsoft Edge WebView2 Runtime、Visual Studio C++ 桌面工具和 Windows SDK、Rust 1.100.0 MSVC 及 Node.js 24 或 26 LTS。首次配置需联网拉取锁定依赖。

```powershell
npm --prefix apps/desktop ci
.\scripts\windows.ps1 -Mode dev
```

脚本会尝试载入 Visual Studio x64 开发命令环境。使用仓库级的 Rust 版本配置。若当前工作区已有 .local 下的同版本工具链，也可直接使用。

### R5 隔离体验

```powershell
.\scripts\windows.ps1 -Mode r5-terrain
```

在托盘打开「世界编辑器」，从世界库新建「林下 · R5 石木布置」。点「内容 → 放置朽木段」，把木段摆在场景中部的空地。可单独拖动或翻转木段，也可移除、撤销、试运行和保存。石木体验使用 `.local/r5-terrain/data`，可和前一阶段的双石体验同时打开。 [R5 贴地木段验收](docs/design/r5/implementation/terrain-verification.md)

同一启动方式也可从世界库新建「林下 · R5 扁石与小石组」。在「内容」里选择扁石、两石组或三石组，放置预览标出实际接地格。石组支持整组移动和撤销。 [R5 扁石与小石组验收](docs/design/r5/implementation/stonework-verification.md)

要体验可以穿行的树皮，从世界库新建「林下 · R5 树皮洞口」。打开「内容」放置短拱树皮或裂口半筒。实线框标出两个支脚，虚线框标出需要留空的通道。点选树皮可查看洞内鼠妇数量。 [R5 树皮洞口验收](docs/design/r5/implementation/bark-verification.md)

「林下 · R5 落叶布置」包含上述石木、树皮以及可取食、可分解的落叶簇和落叶堆。放置前可查看投放量，选中后可查看碳、氮、磷余量。世界库里的旧预设继续按自己的定义运行。 [R5 落叶验收](docs/design/r5/implementation/litter-verification.md)

同一启动方式支持从世界库新建「浅水池塘 · R5 池底造景」。在「内容」选择主石或卵石，沿池底放置和拖动。虾会避开底石，排水时可看到原地露出的石头。 [R5 浅塘底石验收](docs/design/r5/implementation/pond-stones-verification.md)

「浅水池塘 · R5 沉木布置」在底石之外提供主枝和短根。在画布选中沉木后可查看停靠的虾，停靠中的虾会随沉木移动。移除前会确认，虾会离开木面。 [R5 浅塘沉木验收](docs/design/r5/implementation/pond-wood-verification.md)

「浅水池塘 · R5 附着斑块」可在浸水的主石或沉木上布置苔藓小垫、分枝绒簇。先摆放承载物，再从「内容」选择附着位置。在「属性」可以查看斑块的碳、氮、磷，调整位置或修剪。 [R5 浅塘附着斑块验收](docs/design/r5/implementation/pond-moss-verification.md)

```powershell
.\scripts\windows.ps1 -Mode r5-dev
```

从托盘打开「世界编辑器」，进入「世界库 → 从预设新建」，选择「林下 · R5 石块布置」。暂停编辑时在「内容」里选苔斑岩石，再到画布上摆放。可点选或拖动各块石头，在「属性」中翻转或移除，使用工具栏撤销、重做、试运行和保存。此模式使用 `.local/r5-dev/data`，适合体验和反复重开验证。`-Mode r5-preview` 打开林下和浅塘的分层样板，并使用另一份独立目录。 [R5 B 交付与实测](docs/design/r5/implementation/verification.md)

## 操作

1. 启动后桌面直接显示当前世界的组件。左键点击 Windows 通知区域的微缩生态图标可恢复组件面板。右键菜单可以打开观赏、生态编辑器和组件设置。如果图标被收纳，先展开通知区域的隐藏图标。
2. 托盘「组件」菜单可启用生态画面、时钟或环境信息，并在组合式、自由摆放、混合之间切换。直接拖动桌面组件可移动整组，按住 Shift 拖动可调整单个组件。靠近对齐线或网格点时会吸附。混合模式中，拖入生态画面可叠入，向外拖可分离。松手后保存位置。
3. 「组件设置」可选跟随编辑器或固定观察一个世界。离开后可显示北京时间、自定义文字或保持画面。生态画面与叠入其中的组件共享离开覆盖，独立组件单独显示。组件来源和布置独立于生态存档。
4. 在托盘「打开世界编辑器」后点「世界库」，可新建、切换、复制、改名和归档。归档停止该世界的后台模拟并保留检查点。空白场景可调整环境并显式加入主题示例生物。暂停编辑时可移动石块或修改水体，地点和模拟日历随各世界保存。「将当前布置存为预设」保留布置作为初始方案，「复制现状」保留此刻的资源、生命状态、时长和随机状态。
5. 「试运行」可推进独立副本，之后可丢弃、采用进到原世界或另存为新世界。「隐藏全部窗口」后，未暂停的世界继续运行。「保存并退出」保存所有可用世界后停止进程。重开恢复之前的选择、有效检查点和组件布置，暂停和离线期间不推进。
6. 已发生变化的正式世界默认每 120 秒清醒运行时间自动建立检查点。每个世界保留最近两份可读自动检查点，显式保存和损坏的检查点照常保留。强制退出恢复最近一次成功提交的进度。 [保存与故障恢复验收](docs/m4/recovery.md)

## 验证

```powershell
.\scripts\windows.ps1 -Mode check
.\scripts\windows.ps1 -Mode test
.\scripts\windows.ps1 -Mode headless
.\scripts\windows.ps1 -Mode forest-replay
.\scripts\windows.ps1 -Mode pond-replay
.\scripts\windows.ps1 -Mode build
.\scripts\windows.ps1 -Mode package
```

`headless` 验证 M0 夹具的视图解耦。`forest-replay` 比较石块干预后的遮阴、水分、鼠妇活动与资源收支。`pond-replay` 验证补排水、干涸复水、独立水体和昼夜碎屑压力。`build` 产出 `target/debug/micro-ecosystem-desktop.exe`。 [M3 验证记录](docs/m3/verification.md) · [M2 验证记录](docs/m2/verification.md)

Windows 调试和打包的中间文件由构建命令生成。清理后首次运行 `build`、`test` 或 `package` 会重建所需缓存。当前安装包、上一版升级基线及 0.1.0 跨版本基线保留在各自的发布记录所列位置。

默认存档使用应用的 Windows 本地数据目录。用临时试验世界验证时，可在启动前设置 `MICRO_ECOSYSTEM_DATA_DIR` 指向独立目录。`MICRO_ECOSYSTEM_TRACE_FILE` 可记录运行、窗口和保存事件。复验自动保存时可用 `MICRO_ECOSYSTEM_AUTOSAVE_INTERVAL_MS` 更改本次启动的间隔，合法范围为 1,000–3,600,000 毫秒。

## 结构与范围

| 位置 | 职责 |
| --- | --- |
| crates/eco-core | Tauri 无关的固定步长林下与浅塘模型、季节、M0 兼容夹具和资源账本 |
| crates/world-runtime | 世界身份、编辑、调度边界与持久化 |
| apps/desktop/src-tauri | Windows 生命周期、托盘、IPC 和应用入口 |
| apps/desktop/src | React 界面、独立北京时间、PixiJS 林下场景与 Canvas 2D 池塘场景 |
| ecosystem-handoff | 交接基线、原参考图与审查结论 |

已有 M0 和 M1 检查点保留各自模型版本。浅塘为版本 3，原季节林下为版本 4，「林下 · R5 石块布置」为版本 5，「林下 · R5 石木布置」为版本 6，「林下 · R5 扁石与小石组」为版本 7。世界库识别以前保存的检查点并显示恢复或不可读状态，归档与世界名称独立存放。

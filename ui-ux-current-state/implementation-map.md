# 当前实现定位图

对象是 Windows x64 微缩生态 0.1.0。以下为现行产品代码和版本记录的核对结果。代码路径用来定位职责，阅读本页不需要仓库。画面、原生交互和未执行项以 [界面清单](screen-inventory.csv) 与 [流程证据](flows.md) 为准。[内容目录](content-catalog.csv) 把生态实体、资源、外观和预设分开列出。

## 原生窗口与入口

四扇窗口共用一个前端入口。`apps/desktop/src/main.tsx` 标记当前视图并挂载 React，`apps/desktop/src/world.ts` 根据 Tauri 窗口标签识别原生视图，浏览器仅以 `?view=` 选择同名分支。`apps/desktop/src/App.tsx` 分派界面。原生窗口的初始逻辑尺寸来自 `apps/desktop/src-tauri/tauri.conf.json`，下表尺寸用来识别窗口配置。实际客户区与截图像素见 [采集环境](capture-environment.json)。

| 界面与路径 | 现行内容 | 绘制与实现 |
| --- | --- | --- |
| 启动 → 桌面组件面板 `panel`，初始 384×216 | 默认只有生态画面，已有布局可恢复时读取它。点击托盘图标左键可显示面板。当前入口不含欢迎页或首次使用向导。 | Tauri 透明无边框窗口，面板自身无系统阴影、无任务栏入口并禁止最小化和最大化。`App.tsx` 的 `WorkspacePanel` 以 DOM 定位卡片，用画布绘制生态。 |
| 托盘 → 打开世界编辑器 `editor`，初始 1060×730，最小 580×440 | 标题、世界库、保存、编辑切换、快捷新建、主题操作、中央场景、情景右栏、底部提示。林下右栏有石块与环境读数，浅塘右栏有水体与资源。 | `editor/WorldEditor.tsx` 提供框架、林下检查器、试运行前后读数、日照地点和画面／存档。`editor/PondInspector.tsx` 提供浅塘表单和水体读数，`render/WorldScene.tsx` 路由到具体画布。 |
| 编辑器 → 世界库 | 「我的世界」「从预设新建」两个视图。打开、重命名、复制现状、归档／恢复到世界库、保存个人预设。新建页按主题分组选择并填写名称。 | `editor/WorldLibrary.tsx` 使用 HTML `<dialog>.showModal()`，库内表单和确认区为 React/DOM，归档组为原生 HTML `<details>`。由编辑器统一执行命令并管理回执。 |
| 编辑器 → 编辑环境 → 试运行 | 原主题场景与检查器显示副本；工具栏有推进 60 秒、按倍率推进、丢弃、采用结果、另存。采用与另存的确认或命名区在原窗口内。 | `WorldEditor.tsx` 切换状态和数据来源，无独立原生窗口。非试运行的快速新建确认和检查点读取确认也在原窗口内。 |
| 托盘 → 打开观赏窗口 `observe`，初始 720×450，最小 400×268 | 大场景、当前世界名、暂停标签、关闭入口。运行／暂停操作在编辑器，观赏窗只有观看。 | `App.tsx` 的 `ObservationWindow` 与 `Landscape`，使用与组件共享的画布分支。窗口随编辑器所开的正式世界，组件可另绑定。 |
| 托盘 → 组件设置 `layout`，初始 420×410，最小 365×350 | 桌面组件观察来源、离开显示方式、手势帮助、关闭、保存设置。可绑定一个具体世界，或随编辑器。 | `App.tsx` 的 `ArrangementWindow` 与 `layout/WorkspaceEditor.tsx` 的 `WorkspaceSettings`。`layout/workspace.ts` 读取并保存布局及监听变更。设置窗不提供布局大画布，实际拖动在桌面面板上。 |
| Windows 通知区域 → 微缩生态图标右键 | 「组件」子菜单切换生态画面、时钟、环境信息及组合式、自由摆放、混合。主菜单打开各窗口、隐藏全部、保存并退出。 | `apps/desktop/src-tauri/src/workspace_tray.rs` 原生菜单及勾选，`main.rs` 创建图标、显示窗口、处理退出。桌面卡片内部的「调整这个组件」另由 `App.tsx` 自绘，只在多块且当前未拖动时出现。 |

`main.rs` 的 Tauri 单实例插件在重复启动时唤出组件面板。编辑器、观赏窗和设置窗都使用页面内的标题与关闭按钮，支持原生窗口边框缩放。编辑器与观赏窗未显式禁止系统最小化和最大化，但无页面内同名按钮。设置窗和组件面板显式禁止系统最小化、最大化。窗口关闭拦截和页面关闭按钮将相应窗口隐藏。真正退出走托盘「保存并退出」，保存失败时进程不按成功退出。代码中未见用户可配置的置顶、锁定、透明区点击穿透或浏览器缩放功能。面板可变的外框源于内部卡片布局和当前工作区尺寸。

直接核对视图时，可从以下原生客户端截图开始。世界库及试运行的入口和动线见流程页。 [组件初始画面 UI-001](screenshots/UI-001_startup-panel.png) · [编辑器 UI-002](screenshots/UI-002_editor-idle.png) · [世界库 UI-003](screenshots/UI-003_world-library.png) · [观赏窗 UI-007](screenshots/UI-007_observe-forest.png) · [组件设置 UI-008](screenshots/UI-008_layout-settings.png)

## 共用视觉与素材入口

- 全局样式 `apps/desktop/src/styles.css` 为深色体系，系统字体栈为 `Microsoft YaHei UI`、`Segoe UI`、`system-ui`、`sans-serif`，基线 14px / 1.45。`--glass-ink`、`--glass-muted`、`--glass-edge`、`--glass-fill`、`--glass-accent` 等变量控制当前半透明玻璃样式。编辑器、观赏窗、设置窗在 Tauri 配置请求 `acrylic`；面板是无阴影透明窗，其卡片由 CSS 处理边界。皮肤没有用户切换入口。
- `editor/WorldEditor.css` 管编辑器、工具栏、世界库和检查器。`layout/workspace-editor.css` 管设置表单。没有统一组件库，按钮和表单主要是原生 HTML 控件、共享变量及分区 CSS 类。世界库是唯一的 HTML 模态对话框，其他确认通常为行内区块或折叠内容。托盘菜单由 Windows/Tauri 渲染。
- 打包和托盘采用 `apps/desktop/src-tauri/icons/icon.ico`，更早的绘制来源在 M4 素材记录中为未知。场景对象列表缩略图直接采用林下 `assets/d2/rock.png`。关闭图形是文字 `×` 等样式元素，未见独立通用图标库或随应用打包的字体。
- `render/ForestScene.tsx` 挂载 `render/pixiForest.ts` 的 PixiJS WebGL 场景。`render/assets/d1` 与 `d2` 各 16 张 PNG，底图层尺寸分别是 240×135、480×270。石块和接触影为 41×28／82×56，鼠妇每帧为 17×13／34×26。同一鼠妇用三张活动帧和一张遮蔽姿态，幼体用体型变化表示。`render/source/forest_art.py` 是可编辑生成源。
- `render/PondScene.tsx` 绘制 Canvas 2D。`render/assets/pond/v1` 有 13 张 PNG，其中 8 张全场图层为 480×270，2 张金鱼藻形水草变体为 116×112，3 张虾的活动帧为 47×29。水线、色层、碎屑片段、由种群估算的水蚤点和水纹由 Canvas 绘制。`render/source/pond_art.py` 为可编辑生成源。
- 两类场景都按视图比例适配并结合设备像素，最近邻显示。林下编辑器「画面与存档」的 240×135、480×270 控制源资源密度。浅塘只有 480×270 的逻辑画布，无同名切换。`render/seasonColor.ts` 根据模拟季节和日序调整配色，运行读数与绘图使用的动作帧分开。`prefers-reduced-motion` 可降低部分画布和界面动效。

可离线对照交付使用的代表性源图。素材索引与实机截图分开放置。 [源素材索引](asset-index.html)

## 状态、命令与存档

下列路径以 `apps/desktop/src` 表示 React，以 `apps/desktop/src-tauri/src` 表示桌面宿主，`crates` 内的 Rust 模块维护世界和数据。命令在桌面宿主 `main.rs` 注册。

| 范围 | 从界面到状态的连接 |
| --- | --- |
| 世界订阅 | `world.ts` 以 `subscribe_world`／`poll_world`／`unsubscribe_world` 获取有序快照。用 `worldId` 和 `runGeneration` 区分权威世界，用 `sequence` 接纳较新的值。`host.rs` 维护前后台世界及窗口绑定，隐藏视图解除订阅。无客户端同世界重复模拟。`eco-core/src/lib.rs` 以 100 ms 固定步长推进；`power.rs` 传递 Windows 状态。 |
| 手动暂停、编辑暂停和隐藏 | 编辑器 `set_paused` 与 `set_edit_paused` 分开。`WorldEditor` 进入编辑后暂停模拟，原先手动暂停会在「完成编辑」后保留。原先未手动暂停时按钮标成「完成编辑，开始生活」。打开或关闭窗口改变可见性，不重设手动暂停，也不结束编辑。试运行的按钮与正式世界按钮分别受模式限制。 |
| 选择对象、合法区与撤销 | 石块选择和林下位置撤销栈保存在 `WorldEditor.tsx`，按世界、代次、修订、步数限定。`pixiForest.ts` 做命中、拖拽预览、方向键微调和 Escape／失去捕获取消。`edit_rock` 提交归一化坐标，Rust `eco-core/src/forest.rs` 校验地表可摆区及占用。撤销是反向位置命令。浅塘的 `edit_pond` 与 `edit_pond_history` 都在 Rust 维护历史和合法性。修改提供 `commandId`、期望代次与修订。空白起点的一次组合投放走 `add_starter_life`，干浅塘须先补水。 |
| 试运行副本 | `WorldEditor` 通过 `start_forest_trial` 从当前正式世界生成副本，命令名称虽含 forest，也用于浅塘。`advance_forest_trial` 固定推进 600 步，`advance_trial_scaled` 以 20 步基数与 1×/10×/60× 倍率分批推进，分别为 2、20、120 模拟秒。没有连续播放切换。`discard_forest_trial` 丢弃。`adopt_trial` 校验启动时的身份、修订、步数等并提交检查点，保留正式世界 ID 与暂停设置，但新建运行代次。`save_trial_as_world` 建独立暂停世界。详见 `crates/world-runtime/src/trial.rs` 与宿主 `host/library.rs`。 |
| 世界、预设和检查点 | `WorldLibrary` 调用 `list_worlds`、`switch_world`、`create_world`、`copy_world`、`rename_world`、`archive_world`、`save_world_preset`。UI 中预设用途是重建同类起始环境，复制现状才保留完整状态。`crates/world-runtime/src/library.rs` 提供四个系统起点和个人预设读取。`host.rs`、`host/autosave.rs` 与 `crates/world-runtime/src/checkpoint.rs` 执行 `save_world`／`restore_world`、定期自动保存、损坏检查点回退。编辑器读取检查点只提供确认，无版本列表。默认每 120 秒清醒时间处理变更的正式世界，保留两份可读自动检查点及手动存档。 |
| 布局和组件来源 | `layout/layout.ts` 定义 V2 数据形状、卡片尺寸、排序、局部吸附、吸入／脱离及迁移。`layout/panelInteraction.ts` 处理整面板拖动、Shift 拖动单块、点击调整层级、混合组合预览与提交。Rust `workspace.rs` 管实际窗界、显示器工作区换算、按比例记录位置／找回。`layout/workspace.ts` 使用 `get_workspace_layout`、`save_workspace_layout`、`save_workspace_idle`，布局文件独立于世界。 |
| 组件显示与闲置 | `App.tsx` 的 `EnvironmentInfo`、`Clock` 和 `Landscape` 共处单个 `panel` 原生窗。组内结合「生态画面」的卡片共享覆盖，组外卡片独立。观赏窗也读取同一覆盖设置。`useIdleCover` 延迟 3.6 秒，鼠标进入或键盘聚焦可恢复，拖动期间阻止覆盖。时钟用 `Asia/Shanghai` 的现实时间，与世界模拟日照、试运行倍率独立。 |

组件的设计逻辑尺寸来自 `layout/layout.ts`。生态画面 384×216，独立时钟 172×112，独立环境信息 194×112，归入生态画面后分别是 148×78、162×78。尺寸随面板可用区域整体适配，不提供每块自由缩放面板。托盘每项开关控制相应类型。模型按卡片顺序叠层并把被调整项置前，没有公开的数字层级设置。

## 模型读数、显示辅助与未露出内容

数据接口在 `apps/desktop/src/world.ts` 的 `WorldSnapshot`、`ForestReadings`、`PondBasinSnapshot` 等类型。下表列出的单位来自实际文本及字段。数字受世界、采样时刻和初始定义影响。

| 位置 | 当前展示 | 作用与差异 |
| --- | --- | --- |
| 林下场景对象 | 只列石块与其左／上坐标，0–1 归一化比例。 | 真实位置存入世界。移动影响局部遮阴、掩蔽、蒸发和可通行条件。鼠妇、蕨类、苔藓可见，但不进入对象列表。 |
| 林下环境 | 左右各地表水量 mL、遮阴与掩蔽比例、蒸发 mL/h、鼠妇及其中躲藏／摄食数量。折叠区显示枯落物 mg C、可利用土壤氮 mg N、磷 mg P。季节林下另有幼体、累计出生／死亡、弹尾虫群、蕨类和苔藓的总 mg C。旧夹具以 0–1 水分相对指标显示。 | 字段来自模型聚合，个体详情、鼠妇年龄、生物量与繁殖进度没有编辑器详情卡。蕨类／苔藓胁迫用于画布外观，未显示对应数表。 |
| 浅塘环境与资源 | 水量 L、平均水深 cm、水面面积 cm²、水温 °C、全池溶氧 mg/L、虾与水蚤计数；养分折叠区有可利用氮 mg N/L、磷 mg P/L、草／藻／膜／碎屑 mg C。干涸时部分读数为 `—`。 | 水面面积与容量有约束，补水影响存量、温度、矿物和溶氧。画布用最大水深和浑浊度等输入，容量也随快照发送。三者都没有对应的直接数表。多水体选择是 `basins.length > 1` 的条件入口，内置起点只建一池，未见产品建池／分池入口。 |
| 环境信息卡 | 世界名与运行／暂停／连接状态。林下为左右地表水量 mL，旧夹具为水分比例，浅塘为第一池溶氧 mg/L 或干涸。 | 卡片跟随组件绑定世界，暂无单独字段配置。钟面单独显示真实北京时间与日期。 |
| 气候和日照 | 世界季节、模拟年与当年第几日、气温 °C、相对湿度 %。另有地点名称、经纬度输入和按北京时间表示的日出／日落。 | 季节来自世界累计模拟时间。改变地点可影响权威日照目标，模型会逐步对齐。「画面与存档」内白天、夜间预览只替换当前编辑画布的光照相位，保留正式世界及其他窗口的值。 |
| 试运行变化 | 已推进时间与起点／当前画面读数。林下对比鼠妇及枯落物，浅塘对比水量、溶氧及水蚤。 | 当前是两点对比，无时间序列图。主窗口是副本时，桌面观赏仍可从正式绑定读取。 |

`crates/eco-core` 内还存有地表格点水分、微生物、个体身体和储备、随机状态、流量及资源收支账本。通常不作为面向用户的完整表显示。池塘中氧及溶质按整池充分混合，画布水色深浅、光束与点阵不是空间化学图。模型未实现完整氮循环、pH、氨毒性、局部水流、真实物种级鉴别和动态池体拓扑。

## 设计资料与技能登记

- `README.md`、`docs/m4/verification.md`、`docs/m4/assets.md` 分别记录现有启动／操作、发行验收和可再生素材。`docs/m3/verification.md` 记录世界库、多世界和工作区的已落地范围。`crates/eco-core/README.md`、`crates/eco-core/POND.md` 是功能型模型及实验边界。
- `ecosystem-handoff/docs/01-project-baseline.md` 是项目范围，`03-editor-and-components.md` 是原设计需求，`08-visual-and-asset-guide.md` 是早期视觉目标。`ecosystem-handoff/references` 含原参考图，运行资源没有直接引用原图。`ecosystem-handoff/prototypes/editor-concept.html` 与对话历史中的桌面预览、交互白模属于探索材料，其中笔刷、素材选择等界面不能当作现行功能。
- 未找到项目目录内的 UI 技能清单、Uizze 安装文件或运行依赖。历史对话将 Uizze 作为曾参考的外部工具，实际安装版本标记为 `unknown`。用户级目录可见 `~/.agents/skills/endfield-ui/SKILL.md`、`~/.codex/skills/sakuga-motion-design/SKILL.md`，文件 frontmatter 无版本字段，也没有发现它们与本项目运行代码的绑定。桌面采集工具属于工作环境配置，独立于应用的 UI 技能。这次只做现状登记。
- 用户本轮指定后续方向为「像素工作台」。当前原生窗口仍按上文现状记录，早期白模与当前玻璃风格不作为下一轮的视觉规范。

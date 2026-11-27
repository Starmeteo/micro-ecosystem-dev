# 首版交互补全 · 0.2.20

这批交付多生态桌面组件、单件缩放、观察记录与趋势，以及动物和定点沉叶取出。原有两个 R5 主题共用这些流程。 [0.2.19](windows-release-0.2.19.md)

## 产物

| 产物 | 大小 | SHA-256 |
| --- | ---: | --- |
| [Windows x64 安装包](../../../../.local/archive-r5-0.2.20/微缩生态_0.2.20_x64-setup.exe) | 84,073,719 字节 | `27FB36802E3A93C05046887A7B8FB2F52B00BC5B0D1DDB99BA46D96BBC2E072A` |
| [独立程序](../../../../.local/archive-r5-0.2.20/micro-ecosystem-desktop.exe) | 93,367,808 字节 | `EB08FD11C6C770861D580D4576715B7B68EC06F39B7D9FD85D654D220C4AF1A3` |

## 使用方式

| 功能 | 操作 |
| --- | --- |
| 多生态组件 | 在托盘选择「添加生态组件」，也可在「组件设置」中添加。在设置中逐张指定世界，或跟随编辑器。组合、自由和混合仍用同一桌面面板，组合式由主生态组件承载时钟等附属件。 |
| 单件缩放 | 右键目标组件，选「调整尺寸」，拖右下角或用方向键微调。缩放锁定原有宽高比，范围 75%–175%。平时直接拖动仍移动整个面板，Shift 拖动可移动单个组件。 |
| 观察趋势 | 编辑器打开「环境 → 观察记录与趋势」，选择时间范围和真实指标。林下可查看鼠妇、土壤水量、枯落物或弹尾虫碳库存，浅塘可选择当前水域的溶氧、水量、米虾、水蚤或碎屑碳库存。表格可展开查看逐条记录。 |
| 逐只观察与取出 | 编辑器「生物」选择鼠妇或当前观景剖面的米虾。可看编号、行为和生命阶段，并放大定位。进入编辑后，取出需要确认，可撤销。水蚤以种群计算，可按数量取出。 |
| 定点取出沉叶 | 在浅塘点选沉叶，或在「属性」列表选择，使用「取出沉叶」。主、副剖面的选择互相独立。 |

## 状态与资源

- 同一世界的多个组件读取同一份权威状态。不同世界可分别绑定到各个组件。缩放、范围和位置存在工作区中，生态时钟、世界尺寸、资源和随机进度仍由世界自己保存。
- 短历史在每 10 模拟秒采样，保留近 10 模拟分钟。长历史每 10 模拟分钟采样，最多保留近 24 模拟小时。首次显示从可读取的当前状态起步，干池的溶氧记为无读数。试运行的记录随副本产生，丢弃时弃置，采用时随状态保存。
- 只在模拟推进到采样时刻记录趋势。正常暂停和离线不会填补额外样点。带观察历史的检查点采用新版格式，0.2.20 可读取旧格式。请使用 0.2.20 或更新版读取已写入观察历史的新检查点。
- 取出动物将该个体持有的物质记为外部输出。水蚤按取出份数导出同比例库存。沉叶的剩余碳来自所选落点的当前份额，氮磷按同池共用碎屑中的比例扣减。已经分解、摄食或转成其他库存的部分留在生态中。

## 验收

- **组件。** Windows 本机 150% 缩放下实际添加第二张生态卡，先让两张显示同一世界，再分别绑定林下和浅塘。方向键把浅塘卡从 384 × 216 CSS px 改为 422 × 238 CSS px，林下保持 384 × 216。应用退出重开后，两卡继续显示各自世界并保留尺寸。另用系统鼠标拖手柄把浅塘卡从 110% 调到 119%，林下卡保持 100%。宿主回归覆盖同世界多订阅只推进一次、独立绑定和隐藏清理。 [同一世界](evidence/first-edition-0.2.20/components-one-shared-world.png) · [两个世界](evidence/first-edition-0.2.20/components-two-worlds.png) · [重开](evidence/first-edition-0.2.20/components-reopened.png) · [系统鼠标](evidence/first-edition-0.2.20/first-edition-pointer-checks.jsonl)
- **趋势。** 林下试运行推进 60 模拟秒，产生包括起点在内的七个样点。丢弃后正式世界只显示起点，重新试运行并采用后，正式世界和进程重开均恢复七个样点。浅塘原生验证有独立指标与读数。干涸断线通过 Rust 历史专项和独立 Edge 绘制夹具验证。 [林下原生曲线](evidence/first-edition-0.2.20/forest-trend.png) · [浅塘原生记录](evidence/first-edition-0.2.20/pond-trend.png) · [干涸断线](evidence/first-edition-0.2.20/pond-oxygen-gap.png) · [趋势测试](evidence/first-edition-0.2.20/trends-runtime.log)
- **编辑。** 实测选取鼠妇并显示画面标记，取出一只后数量从 4 降到 3，撤销、重做恢复相应身份集合。浅塘副剖面取出一只米虾，主剖面的虾和碎屑保留。水蚤取出 2 只，整池数从 5 降到 3，剩余库存为原来的 60%。主副剖面各有一处编号 1 沉叶时，取出副剖面 40 mg C，主剖面仍有 24 mg C，并验证撤销、重做。内核专项验证已分解沉叶只导出剩余份额。 [鼠妇原生观察](evidence/first-edition-0.2.20/forest-individual.png) · [米虾原生观察](evidence/first-edition-0.2.20/pond-individual.png) · [定点沉叶取出](evidence/first-edition-0.2.20/pond-leaf-confirm.png) · [沉叶内核](evidence/first-edition-0.2.20/leaf-core.log) · [原生操作记录](evidence/first-edition-0.2.20/first-edition-checks.jsonl)
- **回归与安装。** Windows Rust 回归执行 203 项并全部通过，按已有范围过滤一项旧林下校准对照。15 项布局测试、TypeScript 检查和生产构建通过。Windows 11 x64 隔离新装、保存重开、卸载与 0.2.19 覆盖升级均通过。六个旧世界存档逐字节一致，回读的生态状态散列一致。设备矩阵与生态长跑沿用已确认的验收范围。 [Rust 回归](evidence/first-edition-0.2.20/workspace-tests.log) · [布局回归](evidence/first-edition-0.2.20/layout-tests.log) · [新装](evidence/first-edition-0.2.20/install-fresh.json) · [升级](evidence/first-edition-0.2.20/install-upgrade.json) · [升级回读](evidence/first-edition-0.2.20/upgraded-webview-check.jsonl)

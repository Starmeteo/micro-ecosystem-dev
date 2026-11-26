# R5 树皮洞口验收

2026-11-25，北京时间 · F04 林下结构物批次

用 `.\scripts\windows.ps1 -Mode r5-terrain` 启动隔离体验。在「世界库 → 从预设新建」选择「林下 · R5 树皮洞口」，从「内容」放置短拱树皮或裂口半筒。落地预览将支脚与通道分开标出。点可见树皮选中洞体，或从「属性」选取实例。可拖动、翻转、移除、撤销与重做。

## 实现范围

- 新定义 `forest-understory-r5-bark/1`、模型版本 8 支持至多六件树皮。旧世界按各自定义打开。短拱与半筒的参考轴长分别为 90 和 120 mm。素材设定的洞口宽均为 45 mm，拱顶名义净高分别为 18 和 28 mm。每种规格导出预览、前壳、后壁、内侧暗部和接触阴影。 [美术源](../../../../apps/desktop/src/render/source/r5_bark_shelters.py)
- 支脚占两个实际地面格，前后入口和洞下中央保持可通行。方向镜像保留支脚和路径。结构覆盖为洞下提供遮阴与庇护。放置、移动和移除直接校验同一份 Rust 布局，所有布置命令保持动物坐标及现有水和 C/N/P 库存。
- Rust 快照依据鼠妇实际落在哪个洞下格报告洞内 ID。绘制时使用其快照位置，把洞内鼠妇绘在内壁与前壳之间。通行以 12×4 的地面格网近似，适用于当前鼠妇模型。若加入体型差异，需要先定义生物横向宽度、身高和入口净空校验。附着、攀爬和树皮分解属于后续规则。

## 验证结果

| 检查 | 实际证据 |
| --- | --- |
| 鼠妇实际穿洞 | 核心回归 `animal_walks_from_back_entrance_through_roof_and_out_the_front` 以种子 1206 和受控食物、水分分布逐步推进。路线依次经过后入口 16、洞下 28、前入口 40，两脚格 27 与 29 不可走。只在洞下格计为洞内。 [路径测试](../../../../crates/eco-core/src/forest.rs) |
| Windows 放置与遮挡 | WebView2 `158.0.4370.33`、单屏 150%。两个规格通过编辑器放置，可通行格由 47 变 43。短拱围住原地的 `isopod-2`，属性显示「洞内鼠妇 1 只」。用独立岩石堵前入口的尝试被 Rust 拒绝。 [放置预览](evidence/bark/bark-placement-footprints.png) · [原生洞内截图](evidence/bark/bark-occupied-native.png) · [原生操作记录](evidence/bark/bark-edit-checks.json) |
| Windows 编辑与试运行 | 验证 Esc、从可见支脚拖动整件、翻转、移除及撤销重做。移除短拱后可通行格恢复到 45，动物坐标保持一致。在试运行里移除短拱只改变副本，丢弃后回到正式布置。 [移除后截图](evidence/bark/bark-removed-native.png) · [原生操作记录](evidence/bark/bark-edit-checks.json) |
| 保存重开与旧档 | 实际保存退出后重启，核对树皮 ID、位置、形状、朝向、遮阴及洞内成员保持，运行代次更新。从世界库打开 v7 比对既有扁石与石组，仍读取 v7 定义。 [退出记录](evidence/bark/exit-checks.json) · [重开记录](evidence/bark/reopen-checks.json) · [重开截图](evidence/bark/bark-reopened-editor.png) · [旧档兼容](evidence/bark/bark-compat-checks.json) |
| 浏览器像素与窄窗夹具 | Edge 的 125% 对照夹具复核前壳对鼠妇的部分遮挡、空洞点穿和支脚命中，以及移除后的底图。工作台在 1060×730 与 580×440 两档可进入七种内容，取消时无提交，洞内数量跟随快照。 [遮挡夹具](evidence/bark/browser-occlusion-fixture.png) · [窄窗夹具](evidence/bark/browser-drawer-580x440.png) · [抽屉检查](evidence/bark/browser-drawer-checks.json) |

`scripts/windows.ps1 -Mode test` 通过 83 项 Rust 测试。`npm.cmd run build:web --prefix apps/desktop` 与 13 项布局测试通过。Vite 提示现有入口包超过 500 kB。首次原生脚本点击使用了截图前的画布位置，导致落到冲突格并被拒绝。复测在截图后重新读取实际画布矩形，再用指针日志和已提交的坐标核对落点。原生验证限隔离开发版和单屏 150%，完整穿洞由核心用例验证。

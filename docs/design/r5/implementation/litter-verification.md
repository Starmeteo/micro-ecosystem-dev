# R5 落叶验收

2026-11-25，北京时间 · F06 地表资源首批

> 以下操作为当时的验收路径。当前开发入口与预设见 [项目说明](../../../../README.md)。

用 `.\scripts\windows.ps1 -Mode r5-terrain` 启动体验版。从「世界库 → 从预设新建」选「林下 · R5 落叶布置」，在「内容」选择落叶簇或落叶堆。可以查看投放量，放置后从「属性」分别查看碳、氮、磷余量。移除前会确认是否把剩余落叶取出世界。

## 实现与默认值

| 预设 | 画面尺度 | 投放质量 |
| --- | --- | --- |
| 落叶簇 | 主叶长 55 mm | 24 mg C、1.032 mg N、0.108 mg P |
| 落叶堆 | 斑块跨度 105 mm | 60 mg C、2.58 mg N、0.27 mg P |

- 两档剂量复用原林下枯落物配比，属于模拟试验参数。投放量由 Rust 的版本定义给出。图形使用独立的外形尺度与四档库存画面，耗尽后可以选取残迹。 [可重复生成的八张 PNG 源](https://github.com/Starmeteo/micro-ecosystem-dev/blob/ff9ae2b0ff82050f1ea112a7013054f94c89227d/apps/desktop/src/render/source/r5_leaf_patches.py)
- 每处落叶有 ID、空间锚点、所属土格、预设与独立 C/N/P 余量。当前一个格最多放一处，世界最多十六处。保留动物在该格的通行、觅食和分解，同一格会和原有枯落物共享有限供应。石木、其他落叶和树皮通道需要另选格。
- 新增记为外部输入。移动只带走剩余落叶，不改变生态总库存。取出只把现在的余量记成外部输出，已被动物摄入或已转化的部分继续留在对应库存。撤销和重做恢复布置及配对的账目，实际推进后开始新的编辑历史。
- 只有 `forest-understory-r5-leaves/1`、模型版本 9 提供落叶编辑。保留已有世界各自的版本。复制现状和检查点重开保留剩余量。另存为初始方案以相同布置和预设剂量建立新的初始库存。

## 验证结果

| 检查 | 实际证据 |
| --- | --- |
| 放置和错误回执 | Windows WebView2 `158.0.4370.33`、单屏 150%。完成两档放置，实际通行格数保持原值。在已有石块对应格放置的 Rust 校验被拒绝。取消放置和取消取出均保持正式布置，检查器读数与快照一致。 [原生放置截图](evidence/litter/litter-placement-native.png) · [原生操作记录](evidence/litter/litter-edit-checks.json) |
| 副本推进、移动与移除 | 某次试运行从 0 推进到 600 步，历时 60 模拟秒。落叶簇的 C 从 24 降到约 23.99818 mg，N 和 P 同步减少。拖动后余量相同，取出并撤销恢复同一斑块和余量。正式状态在采用前维持不变，采用后正式世界写入第 600 步和新落点。 [试运行截图](evidence/litter/litter-trial-residue.png) · [原生操作记录](evidence/litter/litter-edit-checks.json) |
| 存档重开与旧档 | 保存退出并重启后，核对世界代次变化，以及落叶 ID、数量、位置、规格、余量、暂停状态和模拟时刻完全恢复。编辑器、观赏窗和桌面组件显示同一组 ID。从世界库打开旧 v8 树皮世界并核对原结构物，旧世界仍保留 v8 规则。 [退出记录](evidence/litter/exit-checks.json) · [编辑器](evidence/litter/litter-reopened-editor.png) · [观赏窗](evidence/litter/litter-reopened-observe.png) · [桌面组件](evidence/litter/litter-reopened-panel.png) · [重开记录](evidence/litter/reopen-checks.json) · [旧档检查](evidence/litter/litter-compat-checks.json) |
| 账本和显示 | 88 项 Rust 测试通过，包含真实取食／分解、同格需求共享、旧版到空斑块新版轨迹对照、精确撤销、基线与存档恢复。TypeScript、Vite 构建与 13 项布局测试通过。Edge 125% 前端夹具验证了四档变化、耗尽后选中移动和空底恢复。 [内核](../../../../crates/eco-core/src/forest.rs) · [运行层](../../../../crates/world-runtime/src/tests.rs) · [耗尽前端夹具](evidence/litter/browser-depleted-fixture.png) |

当前两档用于单格资源试验。碎叶、铺设范围编辑、体型尺度下的叶缘遮挡、分湿润程度的画面和真实植物配比仍待扩展。四档画面通过前端夹具核对，原生流程实测了资源减少和库存回读。Vite 报告入口包超过 500 kB。验证使用隔离开发版，跨屏和安装包待测。

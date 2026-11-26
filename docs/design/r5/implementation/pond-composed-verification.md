# R5 浅塘整景 V2 验收

2026-11-25，北京时间。

从「世界库 → 从预设新建」选择「浅水池塘 · R5 水下造景」。选择「空白浅塘 · R5 水下造景」可从空砂床开始搭建。两个预设使用 `pond-shallow-r5-composed/1`，模型版本 13。已有浅塘世界按其存档定义打开。

## 场景与编辑

- 组件使用水下近景。编辑器和大观赏窗使用全池取景。三个视图读取同一世界的水位、物件位置及动物状态。
- 样板中的木、石、叶、水草、挂苔与水光分别绘制。初始有 65 件可选造景，其中 14 片挂苔附着于斜沉木。画布或属性面板均可选取物件，根物件按场景坐标移动，附着物按承载物局部坐标移动。当前内容抽屉提供七种可新增的造景。
- 420 mm 斜沉木按斜面和支脚参与碰撞。虾可以停靠浸水斜面，也可以从有净空的木下游过。默认主石依据现有物理尺度显示，虾依据世界中各自的位置和状态绘制，因此与静态样板的体量及摆位有少量差异。
- 造景水草、挂苔、落叶和石材仅改变画面。活体附着斑块保有独立的碳氮磷库存和浸水状态。通过内容抽屉放置的池底物理石木参与避障。砂粒和水光属于环境绘制。
- 编辑器「画面与存档 → 光照预览」提供白天、黄昏和夜间。光照预览只改变绘制。世界光照继续由已保存的昼夜相位、地点和季节驱动。

[场景图层](../../../../apps/desktop/src/render/assets/r5/pond-composed/definition.json) · [斜木碰撞轮廓](../../../../apps/desktop/src/render/assets/r5/pond-composed/slope-profile.json) · [布景存档](../../../../crates/eco-core/src/pond_scenery.rs)

## 实测

| 检查 | 实际结果 |
| --- | --- |
| 组件取景 | Windows WebView2 单屏 150% 缩放。组件画布原始输出为 576 × 324，与旧版、R5 原稿和已确认的 V2 样板以同样尺寸并列。全池窗口与组件读取相同水线。 [四图对照](evidence/pond-composed/widget-comparison.png) · [全池编辑器](evidence/pond-composed/editor-day.png) |
| 拖放与撤销 | 实拖斜沉木，由床面归一位置 0.235 移至约 0.260，14 片附着造景一起平移，组件更新同一锚点，撤销和重做精准恢复对应快照。另在画布放置宽落叶，从 (447, 293) 拖至 (476, 275)，核对原生画面和 Rust 坐标同步，其余摆件和木石保持原位。两次撤销分别恢复拖动前及新增前的完整状态。 [原生记录](evidence/pond-composed/exercise-checks.json) · [独立摆件记录](evidence/pond-composed/scenery-drag-checks.json) · [拖动宽落叶](evidence/pond-composed/editor-dragged-scenery.png) |
| 昼夜 | 原生界面逐次切换 0.50、0.725 和 0.00 三个光相位。截取区域平均亮度依次为 79.47、59.05 和 34.00，切换期间权威世界状态一致。 [三档画面](evidence/pond-composed/lighting.png) |
| 保存退出与重开 | 新的 Windows 进程恢复了同一世界 ID、生态和造景状态、暂停状态及模拟步数。 [回读记录](evidence/pond-composed/reopen-checks.json) |
| 工程检查 | Windows 离线 Rust 工作区测试、TypeScript 与 Vite 生产构建均通过。Vite 仍提示入口脚本超过 500 kB。 |

Windows 验收使用隔离开发版和测试存档。跨屏、安装包和长时间性能尚待验证。

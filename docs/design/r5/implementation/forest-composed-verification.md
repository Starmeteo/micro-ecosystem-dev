# R5 林下整景 V2 验收

2026-11-26，北京时间。

从「世界库 → 从预设新建」选择「林下 · R5 分层造景」查看完整场景，或选择「空白林下 · R5 分层造景」从净底开始。新预设使用 `forest-r5-composed/1` 和模型 14。旧林下存档按其定义加载。

## 场景与编辑

- 确认的 V2 底景、石木、植株、苔藓和地衣以独立素材组合。完整预设有 70 件造景、主石、朽木洞口和一处枯落堆。造景可点选、移动及移除，内容抽屉提供 12 种可新增造景。移除承载物会移除其附着造景，撤销会恢复原来的局部坐标和承载关系。
- 主石和朽木继续使用地表格计算遮蔽及通行。厚壁朽木采用 292 mm 的画面素材，生态使用 90 mm 短拱的支脚和通行格。画面枢轴相对于生态锚点右移 33 px，将可见洞口对到通行格。属性页分别显示画面尺寸和模拟规格。
- 金褐落叶表现已有枯落堆的库存，初始为 60 mg C，消耗后按库存阶段缩减。其他用于构图的落叶和附着苔为造景。画面中的鼠妇根据正式快照的个体位置、朝向和生命阶段绘制。洞内鼠妇使用实际位置在朽木透明轮廓下绘制。
- 编辑器「画面与存档 → 光照预览」提供白天、黄昏和夜间。世界光照继续读取地点和已保存的昼夜相位。

[场景素材与锚点](../../../../apps/desktop/src/render/assets/r5/forest-composed/definition.json) · [实体和库存初始化](../../../../crates/eco-core/src/lib.rs) · [布景保存规则](../../../../crates/eco-core/src/forest_scenery.rs)

## 验证结果

| 检查 | 实际结果 |
| --- | --- |
| 原生画面 | 在 Windows WebView2 单屏 150% 缩放下，桌面组件实际输出 576 × 324。编辑器、观赏窗和组件共享物件锚点。已打开 v4 和 v9 并验证各自的旧渲染路径。 [同尺寸四图对照](evidence/forest-composed/widget-comparison.png) · [原生编辑器](evidence/forest-composed/editor-day.png) |
| 实拖朽木 | 先验证了支脚落点有鼠妇时拒绝移动。随后从 x=88 移至 x=166，支脚格从 [12, 14] 变为 [14, 16]，10 个独立附着造景随木移动。此前遮挡的位置实测有 397 个像素恢复为和独立空白林下相同的底色。撤销后该区域逐像素恢复。 [原生拖动与净底](evidence/forest-composed/terrain-editing.png) · [交互记录](evidence/forest-composed/exercise-checks.json) |
| 独立编辑 | 在画布添加铜褐落叶，再从 (319, 294) 拖至 (354, 273)。两次撤销和两次重做均核对了完整状态。重叠处命中上层附着件。通过属性页换选对象后，按回车不会提交旧对象的键盘草稿。 [交互记录](evidence/forest-composed/exercise-checks.json) · [选取和键盘](evidence/forest-composed/keyboard-checks.json) |
| 光照预览 | 原生界面分别切换白天、黄昏和夜间。三档检测区域平均亮度约为 74.42、54.97 和 32.15，正式布局与生态状态一致。 [光照对照](evidence/forest-composed/lighting.png) |
| 缩放与试运行 | 2× 取景时，朽木拖出画布松手会取消。试运行推进 60 模拟秒，副本内四只鼠妇均移动，正式世界的步数、动物和资源保持一致，随后丢弃副本。 [原生补测](evidence/forest-composed/boundaries-checks.json) |
| 保存退出与重开 | 独立新进程恢复同一世界和完整的造景、实体、生物、资源、季节、地点与时间状态。组件重开继续输出 576 × 324。 [重开记录](evidence/forest-composed/reopen-checks.json) · [重开图](evidence/forest-composed/reopened-panel-native.png) |
| 工程回归 | 112 项 Rust 单元与工作区测试，通过 TypeScript 检查及 Vite 生产构建。构建保留入口 JavaScript 体积提示。 |

原生验收使用隔离开发版和专用存档。安装包、跨屏迁移和长时间渲染性能尚待验证。

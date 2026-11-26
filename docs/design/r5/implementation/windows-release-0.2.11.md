# R5 浅塘活体浮叶 · 0.2.11

浅塘新增三档可编辑活体浮叶。安装包同时包含 R5 林下和浅塘。 [上一版](windows-release-0.2.10.md)

| 产物 | 大小 | SHA-256 |
| --- | ---: | --- |
| Windows x64 安装包（本地副本已清理） | 74,572,331 字节 | `E45366019777089439345571C6E2D7C2E3B5C6EF77947846BDA558036794EAE7` |
| 0.2.11 独立程序（本地副本已清理） | 83,591,680 字节 | `F89B7C438D7BF72654C52B2CEB2AE7F82FA5747817F3F914FC256105644BC53A` |

## 俯视编辑和水下反馈

在「编辑环境 → 内容 → 编辑活体浮叶」选择 30、45 或 60 mm 叶径。在当前水域的俯视格网可摆放、移动和移除，支持鼠标点选或方向键加 Enter。画面中的活体浮叶随水位和摆放位置变化，叶片与悬根分层绘制。预设中的装饰浮叶保留原画面，新增活体浮叶参与生态计算。

覆盖面积取叶片圆形投影在本水域内的并集。水外的面积不会遮光，同一块水面被多片叶遮住时只计一次。编辑器显示水面覆盖面积、比例、剩余入水光和每片活体浮叶的碳氮磷库存。浮叶增长使用水面入射光。藻类、生物膜和沉水植物使用遮光后的入水光，之后仍按各自规则计算浑浊或叶深的影响。浅湖原位试验与受控池塘试验均记录了浮叶覆盖对水下光的影响，本版的不透明圆片是工程近似。 [浅湖原位试验](https://www.deswater.com/DWT_articles/TDWT_I_57_57_J/TDWT_A_1180641/TDWT_A_1180641_O.pdf) · [漂浮叶片遮光试验](https://apms.org/wp-content/uploads/japm-38-01-005.pdf)

移除浮叶会把余量转入碎屑。干涸时保留浮叶库存，活动覆盖归零，复水后按原位置恢复。浮叶参与同池养分和水下呼吸预算，叶片光合产氧不直接加入水中。面积对气体交换、蒸发和局部溶氧的影响留待研究。保存写入浮叶实体或其编号记录后需要 0.2.11 或更新版本读取。 [浅塘模型](../../../../crates/eco-core/POND.md)

## 验证

Windows Rust 工作区 163 项回归、前端类型检查和生产构建通过。内核对照了完整圆及岸边半圆面积、重叠去重、受光和水下生产量、元素和氧气收支、岸线编辑原子性、干涸复水、试运行和存档。设备矩阵与历史长跑专项沿用此前确认范围。 [Rust 回归](evidence/floating-0.2.11/workspace-tests.txt)

隔离 Edge 检查了 576×324 组件、三档浮叶、根长裁切、水位和位置变化，确认 0.2.10 的无浮叶画布与基线逐字节一致。模拟回执的前端检查另覆盖了键盘摆放、移动、移除、撤销和返回场景。 [三档画面](evidence/floating-0.2.11/floating-three-sizes.png) · [整景示例](evidence/floating-0.2.11/floating-in-default.png) · [渲染检查](evidence/floating-0.2.11/edge-render-check.json) · [前端编辑检查](evidence/floating-0.2.11/edge-editor-check.json)

Windows 原生编辑器在同一点重叠两片 60 mm 浮叶时，覆盖为 28.30 cm²、1.18%，移开至同一水域的远处后为 56.60 cm²、2.36%，面板显示入水光约为 97.64%。验证了移除将 32 mg C 转入碎屑、撤销恢复同一 ID、复水恢复覆盖，以及采用 60 模拟秒试运行后冷重开恢复同一位置、库存和透光比例。 [俯视编辑器](evidence/floating-0.2.11/floating-top-down.png) · [桌面组件](evidence/floating-0.2.11/floating-desktop-panel.png) · [原生记录](evidence/floating-0.2.11/floating-checks.jsonl) · [冷重开](evidence/floating-0.2.11/floating-reopen-checks.jsonl)

最终 Windows 11 x64 安装包通过新装、两套 R5 场景重开和从 0.2.10 覆盖升级。新版恢复三份旧测试世界，其中双水域世界包含带状叶、轮生水草和主石行为开关，分池世界保有石木与造景。覆盖安装前后测试存档逐字节一致，安装测试卸载后也保留全部测试档。 [新装](evidence/floating-0.2.11/install-fresh.json) · [重开](evidence/floating-0.2.11/reopened-webview-check.jsonl) · [覆盖升级](evidence/floating-0.2.11/install-upgrade.json) · [旧档回读](evidence/floating-0.2.11/upgraded-webview-check.jsonl)

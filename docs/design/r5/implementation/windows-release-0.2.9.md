# R5 浅塘主石停靠与摄食 · 0.2.9

本地 Windows x64 安装包内含 R5 浅塘与林下。浅塘米虾接近浸水主石时可以停靠。停靠坐标由生态内核判定，画面按石材素材实际坡面摆放米虾，使脚和石面接触。 [上一版](windows-release-0.2.8.md)

| 产物 | 大小 | SHA-256 |
| --- | ---: | --- |
| Windows x64 安装包（本地副本已清理） | 74,539,734 字节 | `384923428C1C39D86C5ABE150FDA1E5595216BB84E2A195E28B2B9B72484B40D` |
| 0.2.9 独立程序（本地副本已清理） | 83,410,944 字节 | `84A7595636102829E018939DC5ED92878FB89E80C3F38EE06B4A4D5D6E9A0D9C` |

## 编辑器和存档

新建 R5 浅塘默认启用主石停靠。已有 R5 浅塘保留原行为。要启用某个已有水域，选中该水域，进入「编辑环境 → 生物 → 主石停靠与摄食」，点击「启用主石停靠」。保存启用后的世界需用 0.2.9 或更新版本读取。

主石停靠最长 3.5 模拟秒。放置、移动与移除仍走原有编辑和碰撞校验。改变主石位置或移除后，米虾离开原停靠点。退水使接触点失效时也解除停靠。保存、冷重开和试运行采用保留合法停靠，摄食读数仅对应当前步。

「生物」面板显示真实摄入的主石表面份额和整池共用份额。两份摄入加起来等于实际总摄入，整池生物膜只扣一次。当前浸水轮廓决定每步的石面份额，持续局部耗竭及恢复仍待建模。 [浅塘模型](../../../../crates/eco-core/POND.md)

## 验收记录

Windows 原生编辑器在 1× 试运行推进 2 模拟秒后捕捉到主石上的米虾实际摄入约 0.00002545 mg C，该步全部归于主石份额。画面标记和面板一致。采用结果保留了同一只虾的主石 ID、相对位置和余下停靠步数。冷重开逐字段对照通过，停靠恢复，摄入标记没有重复播放。 [原生试运行](evidence/main-stone-0.2.9/main-stone-intake.png) · [采用记录](evidence/main-stone-0.2.9/main-stone-checks.jsonl) · [冷重开](evidence/main-stone-0.2.9/main-stone-reopen-checks.jsonl)

隔离 Edge 在 576×324 组件及 4× 局部观察中检查了石面接触。取样的左、中、右三处成虾足部与石面像素相接，幼虾取样误差在 1 像素内。沉木回归的 17 张画面与上一版基线逐字节一致。Windows Rust 工作区 156 项回归和前端类型检查通过。测试覆盖移动、移除、浅水、局部份额、旧档按水域启用、撤销及存档恢复。设备矩阵与历史成对种子长跑专项沿用用户确认的范围。 [4× 局部画面](evidence/main-stone-0.2.9/edge-stone-contact-4x.png) · [像素检查](evidence/main-stone-0.2.9/edge-contact-checks.json) · [沉木回归](evidence/main-stone-0.2.9/edge-wood-regression.json) · [Rust 回归](evidence/main-stone-0.2.9/workspace-tests.txt)

Windows 11 x64 隔离新装验证了两套场景和 576×324 组件，冷重开恢复了两套世界。 [新装记录](evidence/main-stone-0.2.9/install-fresh.json) · [两套场景回读](evidence/main-stone-0.2.9/reopened-webview-check.jsonl)

从 0.2.8 覆盖升级时，三份隔离测试存档的文件 SHA-256 保持一致。重开后林下、已布置双池和实际分过池的世界逐字段一致，旧浅塘仍沿用原停靠规则。通过编辑器按钮为分池档的一个水域启用主石停靠后，另一区的快照保持一致，选中区的模拟步也没有被推进。两次安装测试在卸载后均保留测试存档。 [升级安装](evidence/main-stone-0.2.9/install-upgrade.json) · [旧版测试存档](evidence/main-stone-0.2.9/prior-webview-check.jsonl) · [逐字段回读及按水域启用](evidence/main-stone-0.2.9/upgraded-webview-check.jsonl)

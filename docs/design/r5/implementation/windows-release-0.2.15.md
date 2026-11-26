# R5 浅塘多剖面沉叶 · 0.2.15

浅塘合池会保留各剖面的定位沉叶，包括只有沉叶、没有石块或沉木的一侧。本地 Windows 安装包包含林下与浅塘两套 R5 场景。 [上一版](windows-release-0.2.14.md)

| 产物 | 大小 | SHA-256 |
| --- | ---: | --- |
| Windows x64 安装包（本地副本已清理） | 74,610,321 字节 | `E5B4C63968BC71620E3A03EFAED78EA616B1A01B4C11C65BB3AA5FAB5789A93F` |
| 0.2.15 独立程序（本地副本已清理） | 83,725,824 字节 | `2A0E417C0EFC33DF09BF9BBAE68C70D9F84C4A8BA4D139D8951A1777644EE6CF` |

## 使用方式

在「编辑环境 → 环境 → 编辑池岸形状 → 分合预览」画水道连接两片水域，预览会显示预计保留的剖面和沉叶数量。应用后返回场景，在画面下方切换「观景剖面」。可以从「内容」投放沉叶，或在「属性」选中已有沉叶查看剩余份额、形态并移动。

- 沉叶位置和编号按剖面保存。移动叶片保留剂量和库存。新增沉叶只向整池碎屑库加入一次剂量，两侧沉叶都随同一次整池分解过程变化。
- 再次分池时，定位沉叶所标记的碎屑随原剖面分回对应水域，其余碎屑按面积分配。两条剖面的同编号沉叶可分别编辑。
- 另一侧有米虾、植根水草、活体附着斑块或有库存的石面生物膜时仍会拦截合池。含副剖面沉叶的新存档需要 0.2.15 或更新版本读取。 [浅塘模型](../../../../crates/eco-core/POND.md)

## 验证

Windows Rust 工作区 177 项回归、前端类型检查及生产构建通过。内核对照移除定位标记，对比分解过程中的整池账本，验证保留标记不会额外消耗氧气。运行层验证副剖面投放只记一份输入，经过 120 个模拟步后各落点按相同的比例衰减，试运行采用、冷重开、重建预设和拆分后的碳氮磷均已核对。 [工作区回归](evidence/leaf-sections-0.2.15/workspace-tests.txt) · [运行层专项](evidence/leaf-sections-0.2.15/runtime-leaves-test.log)

隔离 Edge 使用模拟命令回执，核对了只有沉叶的副剖面可合入、分合预览数量、同编号沉叶单独编辑及撤销重做。 [预览核对](evidence/leaf-sections-0.2.15/edge-preview-checks.json) · [编辑核对](evidence/leaf-sections-0.2.15/edge-editor-checks.json)

Windows 原生编辑器把两片共 12 L 的水域合池。副剖面原有 35 mg C 沉叶移动后库存保持，随后在副剖面新增 50 mg C。主剖面原有的 15 mg C 沉叶保持不动。分池预览分别显示 1 和 2 处沉叶。实际分池后，两边总碎屑分别约为 30 和 100 mg C，其中各含 15 mg C 无定位碎屑。撤销分池并保存后，新进程恢复两条剖面和合池的 130 mg C 碎屑库存。 [合池预览](evidence/leaf-sections-0.2.15/leaf-merge-preview.png) · [副剖面沉叶](evidence/leaf-sections-0.2.15/leaf-secondary-edited.png) · [分池预览](evidence/leaf-sections-0.2.15/leaf-split-preview.png) · [原生记录](evidence/leaf-sections-0.2.15/leaf-sections-checks.jsonl) · [重开画面](evidence/leaf-sections-0.2.15/leaf-sections-reopened.png) · [重开记录](evidence/leaf-sections-0.2.15/leaf-sections-reopen-checks.jsonl)

Windows 11 x64 隔离新装与从 0.2.14 覆盖升级通过。四个旧测试世界在安装前后文件逐字节一致，包含已有主剖面沉叶、副剖面石木的旧合池世界。新版完整回读旧档，原生编辑器可继续切换查看旧副剖面。设备矩阵与长跑校准沿用已确认的验收范围。 [新装](evidence/leaf-sections-0.2.15/install-fresh.json) · [新装重开](evidence/leaf-sections-0.2.15/reopened-webview-check.jsonl) · [旧版种档](evidence/leaf-sections-0.2.15/prior-webview-check.jsonl) · [覆盖升级](evidence/leaf-sections-0.2.15/install-upgrade.json) · [升级回读](evidence/leaf-sections-0.2.15/upgraded-webview-check.jsonl)

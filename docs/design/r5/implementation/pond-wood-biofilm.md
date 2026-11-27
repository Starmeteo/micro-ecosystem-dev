# 浅塘沉木生物膜库存

新建 R5 浅塘默认启用沉木库存。已有 R5 浅塘可以在「编辑环境 → 生物 → 生物膜」为当前剖面启用，并可撤销。选中一段沉木，属性页显示总库存和浸水可食量。

每个剖面按沉木物件 ID 保留一份碳、氮、磷库存。停靠的米虾先分食当前木段的浸水份额，剩余需求参与整池共用库存分配。画面膜色、属性读数和当步摄食都来自同一份运行状态。

移动沉木时库存随木段保留。干涸时可食量归零，库存留在木段，复水后重新按浸水比例开放。移除木段时，剩余库存归入同一水域的共用库存。分合池时库存随沉木所属剖面转移。全池仍使用统一的溶氧和养分预算；沉木速率沿用石面机制的功能型参数。

已启用沉木库存的世界使用检查点格式 3，导出的便携备份使用格式 2。开发版继续读取旧格式，旧浅塘在手动启用前沿用原有规则。

内核回归验证了跨剖面同编号沉木的独立库存、实际摄食扣减、整池物质收支、移动、移除、干涸复水、分合池、撤销及保存重开。 [运行模型](../../../../crates/eco-core/src/pond/biofilm.rs) · [摄食分配](../../../../crates/eco-core/src/pond/shrimp_rows.rs)

Windows 原生验证中，木段移位和撤销均保留 8.79053565 mg C。排空 18 L 后可食量为 0，补回水量后恢复。移除木段时整池生物膜总量保持 24 mg C，撤销恢复该段库存。退出重开恢复沉木坐标与库存；0.2.21 浅塘副本可手动启用，启用后原检查点字节保持。 [原生断言](evidence/pond-wood-biofilm/seed-checks.jsonl) · [库存界面](evidence/pond-wood-biofilm/wood-film-selected.png) · [干涸界面](evidence/pond-wood-biofilm/wood-film-dry.png) · [冷重开](evidence/pond-wood-biofilm/reopen-checks.jsonl) · [旧档启用](evidence/pond-wood-biofilm/legacy-checks.jsonl)

另在隔离测试世界将一只米虾放到沉木有效停靠点，使用正式界面以 1× 试运行 20 步。木段库存从 8.79053565 降至 8.79007328 mg C。第 20 步局部摄入约 0.0000254484 mg C，共用摄入为 0，画布和检查器读数一致。丢弃试运行后，原世界的步数、库存和停靠状态保持原值。 [定点测试断言](evidence/pond-wood-biofilm/feeding-checks.jsonl) · [原生摄食画面](evidence/pond-wood-biofilm/wood-film-targeted-native-feeding.png)

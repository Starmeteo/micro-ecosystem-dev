# 单世界备份与导入

在「世界库 → 我的世界」选中世界，点「导出备份」下载一个 `.microeco-world` 文件。点「从备份导入」选择文件，完成后可在世界库打开副本。支持备份当前世界、其他可读世界和归档世界。

备份保存生态库存、布置、生物、模拟步数、随机状态、日照地点及观察历史。导入核对文件大小、格式、校验值和模型定义，随后发布一个新 ID 的暂停世界。副本初始编辑修订为 0，已有世界、活动选择和组件绑定原样保留。导入文件只包含一个世界，重复导入会产生多个独立副本。

最终 0.2.22 Windows 调试版在原生 WebView2 实测下载 44,051 字节的备份，拒绝损坏和不支持的文件，连续两次导入生成不同 ID 并保留完整存档状态。手动打开其中一个副本后，独立进程重开仍在第 13 步并保持暂停。实时时钟目标按启用世界时的现实条件刷新。 [原生断言](evidence/world-backup/seed-checks.jsonl) · [导入界面](evidence/world-backup/world-backup-imported.png) · [拒绝界面](evidence/world-backup/world-backup-rejected.png) · [冷重开](evidence/world-backup/reopen-checks.jsonl)

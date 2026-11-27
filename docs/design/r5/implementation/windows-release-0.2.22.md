# R5 世界备份和沉木生物膜 · 0.2.22

本版支持导出单世界备份、将备份导入为独立世界，以及查看浅塘各段沉木上的生物膜库存和米虾摄食量。新建 R5 浅塘默认启用沉木库存，已有浅塘重新打开时保留原状态，可在编辑时按水域启用。 [备份操作](world-backup.md) · [沉木库存与行为](pond-wood-biofilm.md)

| 本地产物 | 字节 | SHA-256 |
| --- | --- | --- |
| [Windows x64 安装包](../../../../target/release/bundle/nsis/微缩生态_0.2.22_x64-setup.exe) | 74,229,446 | `0D1773AC201AE8BDDD602599FEBAEEDE9A703B8EA3385600C3EAA4437A3606CD` |
| [独立程序](../../../../target/release/micro-ecosystem-desktop.exe) | 83,716,608 | `F4E022C934ECF0FDA2178F83970C7E43116B26CE1EADBC64C0A45582F29A794C` |

## Windows 安装验收

- 在 Windows 11 x64 构建 26200 上验证当前用户 NSIS 安装。安装包未签名。新装与覆盖升级后的应用和安装注册均为 0.2.22，安装程序写入后的载荷哈希与预期一致。两次卸载均移除测试安装和注册，保留测试存档。 [新装](evidence/r5-release/0.2.22/install-fresh.json) · [升级](evidence/r5-release/0.2.22/install-upgrade.json)
- 空数据启动进入 R5 林下，世界库提供六套 R5 内置预设。实际编辑林下土格及浅塘补水后保存退出，新进程读回两个世界的状态。 [新装编辑](evidence/r5-release/0.2.22/fresh-webview.jsonl) · [冷重开](evidence/r5-release/0.2.22/fresh-reopen.jsonl)
- 用实际安装的 0.2.21 保存 R5 林下与浅塘，再覆盖升级。两个世界的 ID、模型和状态校验值在首次读回时一致，六个原检查点经过升级、冷重开和功能验证后保持逐字节一致。为升级后的浅塘显式启用沉木生物膜时，检查点格式从 1 更新到 3，整池生物膜碳库存仍为 24 mg C。该世界的备份导入后，原来的两个世界和独立副本均可在新进程恢复。 [升级读回](evidence/r5-release/0.2.22/upgrade-worlds.jsonl) · [冷重开](evidence/r5-release/0.2.22/upgrade-reopen.jsonl)

## 安装版功能验收

- 从世界库操作实际的备份文件下载和文件选择。对同一备份导入两次得到不同世界 ID，完整存档状态与来源一致。损坏或版本不支持的备份被拒绝，导入前后当前世界与桌面布置保持一致。显式打开一个暂停的副本后保存重开，仍可读回。 [文件往返](evidence/r5-release/0.2.22/backup-checks.jsonl) · [重开](evidence/r5-release/0.2.22/backup-reopen-checks.jsonl) · [世界库画面](evidence/r5-release/0.2.22/backup-imported.png)
- 实际移动、干涸复水、移除和撤销沉木。移动保留对象库存，干涸将可摄食量降为零，复水恢复可用量，移除把库存退回水域共用库存，重开可读回恢复后的对象库存。 [编辑与资源记录](evidence/r5-release/0.2.22/wood-checks.jsonl) · [重开](evidence/r5-release/0.2.22/wood-reopen-checks.jsonl)
- 定点试运行观测到米虾在对应沉木摄入约 0.00002545 mg C，编辑器画面和检查器均显示该摄食事件。丢弃试运行后，原世界保持一致。默认摆位另试跑了 240 步，在该观察窗口内没有发生沉木摄食。 [定点摄食记录](evidence/r5-release/0.2.22/wood-feeding-checks.jsonl) · [摄食画面](evidence/r5-release/0.2.22/wood-feeding.png)

## 复验

使用 Windows 开发环境和已安装的项目依赖。从仓库根目录构建，并用已归档的 0.2.21 安装包和对应独立程序实测升级。安装检查使用独立应用数据与 WebView2 配置，应在当前用户未安装正式应用时运行。

```powershell
.\scripts\windows.ps1 -Mode package
.\scripts\test-windows-installer.ps1 -Scenario Upgrade -PriorInstaller '.local/archive-r5-0.2.21/微缩生态_0.2.21_x64-setup.exe' -PriorReleaseExe '.local/archive-r5-0.2.21/micro-ecosystem-desktop.exe'
.\scripts\test-windows-installer.ps1 -Scenario R5
```

仓库脚本覆盖安装注册、存档读回、通过已安装应用的 IPC 执行备份往返和沉木启用、保存重开及卸载。本次另用工作区 `.local` 下的探针操作已安装程序的界面和文件选择器，仓库保留了该次断言与截图。 [附加探针结果](evidence/r5-release/0.2.22/features.json)

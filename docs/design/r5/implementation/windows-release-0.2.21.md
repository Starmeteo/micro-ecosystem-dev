# R5 运行素材与预设统一 · 0.2.21

本包交付统一 R5 素材、六套内置预设和旧世界首次升级选择。世界备份和沉木局部生物膜随 0.2.22 安装包交付。 [0.2.22 安装验收](windows-release-0.2.22.md)

| 本地产物 | 字节 | SHA-256 |
| --- | --- | --- |
| [Windows x64 安装包](../../../../.local/archive-r5-0.2.21/微缩生态_0.2.21_x64-setup.exe) | 74,202,575 | `5585D8BF38629C0648E8A0FF58A54B5E5A51841AFA4CDE328B46F13B21051C2F` |
| [独立程序](../../../../.local/archive-r5-0.2.21/micro-ecosystem-desktop.exe) | 83,634,688 | `116025760FD8178D787C38EB34EE20663F6041F32590B23EFBFB0103355B7608` |

## Windows 验收

- Windows 11 x64 构建 26200，当前用户隔离安装，未签名。安装注册与实际程序版本为 0.2.21，安装后的载荷哈希与预期一致。卸载移除了测试安装和注册信息，保留测试存档。 [新装记录](evidence/r5-release/0.2.21/install-fresh.json)
- 空数据启动进入 R5 林下，世界库含六个内置预设。林下修改土格基质，浅塘补水至 18.15 L。两者保存退出并在新进程读回，状态与原检查点一致。 [现场断言](evidence/r5-release/0.2.21/fresh-webview.jsonl) · [林下编辑](evidence/r5-release/0.2.21/fresh-edited-forest.png) · [浅塘编辑](evidence/r5-release/0.2.21/fresh-edited-pond.png) · [保存重开](evidence/r5-release/0.2.21/fresh-reopen.jsonl)
- 由实际安装的 0.2.20 建立 R5 林下、R5 浅塘、旧林下和旧浅塘。覆盖升级时数据逐字节一致。升级后首次启动复用已有 R5 林下，四个世界均可打开并显示 R5 画面。手动选回旧林下后冷重开仍恢复该世界，11 个原检查点校验值一致。 [升级记录](evidence/r5-release/0.2.21/install-upgrade.json) · [四世界读回](evidence/r5-release/0.2.21/upgrade-worlds.jsonl) · [冷重开](evidence/r5-release/0.2.21/upgrade-reopen.jsonl)

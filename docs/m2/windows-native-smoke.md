# M2 Windows 实机闭环

本机 Tauri Windows 调试版通过编译。工作区的 Rust 测试共 55 项通过，其中生态核心 23 项、世界运行层 21 项、Windows 应用 11 项。原生冒烟另用两次启动和独立的世界数据目录，分别生成新的 WebView2 配置。

## 编辑、试运行与冷启动

| 场景 | 实测结果 |
| --- | --- |
| 创建林下和浅塘 | 从编辑器创建季节林下版本 4，读取 3 个成体、1 个幼体、两侧的跳虫和蕨类碳库存。再创建独立浅塘版本 3，从第 0 步进入编辑暂停并写下初始检查点。 |
| 补水和撤销重做 | 在编辑器点「应用」补水 0.80 L。水量从 18.00 L 到 18.80 L，水深从 7.50 cm 到 7.83 cm，面板水线位随同变化。撤销和重做恢复对应的完整池塘快照，正式世界步数保持第 0 步。 |
| 试运行副本 | 通过编辑器实际点按进入副本、推进 60 模拟秒、丢弃副本。另用同一原生 IPC 检查副本由第 0 步到第 600 步，水温、溶氧、资源和虾位置均发生变化，正式世界仍在第 0 步，副本没有写入存档。 |
| 活态推进、存档与退出 | 解除编辑暂停，使正式世界推进至第 6 步。再次进入编辑，保存并请求「保存并退出」。两个进程都正确写出退出提交事件并以 0 退出。 |
| 新进程恢复 | 用上次的世界数据和新的 WebView2 配置冷启动。世界 ID 不变，运行代次变更，第 6 步、编辑修订号 3、编辑暂停、季节、日照相位、池塘库存以及检查点 RNG 逐项相等。编辑器、观赏窗和面板同报水线位置 106。 |

实际捕获的画面可见存档池塘和可读水深、氧浓度、种群指标，以及稀疏显示的水蚤样本。 [补水后的编辑器](../../.local/m2-native-evidence/closure-20261124-211821-997/03-pond-after-add-water.png)、[冷启动后的桌面面板](../../.local/m2-native-evidence/closure-20261124-211821-997/06-pond-reopened-panel.png)、[林下季节模型](../../.local/m2-native-evidence/closure-20261124-211821-997/00-seasonal-forest.png)

## 证据与复验

- 构建命令为 `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/windows.ps1 -Mode build`，工作区测试使用相同脚本的 `-Mode test`。测试构建的 SHA256 为 `9D9F4C43B347D7297D1225C1E080D5B5EEC1A2A96497AD5059EDF083A24C1711`。 [构建记录](../../.local/m2-native-evidence/closure-20261124-211821-997/build.json)
- 冒烟命令为 `pwsh -NoProfile -ExecutionPolicy Bypass -File .local/verify/run-m2-native-acceptance.ps1`。测试使用原生进程的 WebView2 调试协议点击实际编辑器按钮，通过真实 IPC 读取快照和保存退出。 [首次运行断言](../../.local/m2-native-evidence/closure-20261124-211821-997/checks-begin.jsonl)、[冷启动断言](../../.local/m2-native-evidence/closure-20261124-211821-997/checks-reopen.jsonl)

本次使用本机的一个显示器配置，未触发真实睡眠、未做多屏及 DPI 矩阵、未采集资源占用。两次退出时 WebView2 均在 stderr 记录一次 `Chrome_WidgetWin_0` 注销错误 1412，原生保存提交和进程退出码检查通过。

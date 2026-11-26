# Windows x64 构建与安装

## 安装与使用

运行安装包，按简体中文向导选择目录并完成安装。安装范围是当前 Windows 用户，默认目录为 `%LOCALAPPDATA%\微缩生态`。启动后可在系统托盘打开编辑器、观赏窗和组件设置，退出使用托盘的「保存并退出」。 [Windows 安装包](../../.local/archive-m4-0.1.0/微缩生态_0.1.0_x64-setup.exe) · [Tauri 安装模式](https://v2.tauri.app/distribute/windows-installer/#install-modes)

应用标识仍为 `local.micro-ecosystem.desktop`，现有世界与组件设置沿用同一数据目录。安装包使用系统的 WebView2 Runtime。如果系统缺少该组件，安装器会联网下载 Microsoft 引导程序。安装器和应用为本地未签名构建。 [Tauri WebView2 安装选项](https://v2.tauri.app/distribute/windows-installer/#webview2-installation-options)

卸载时可保留世界存档。安装器卸载页面的应用数据删除选项会一并清除该标识的数据目录，只有明确需要删除世界与组件设置时才选择。 [本版本 Tauri NSIS 模板](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.12.6/crates/tauri-bundler/src/bundle/windows/nsis/installer.nsi)

## 复现构建与验证

在装有 Visual Studio C++ 桌面工具、Windows SDK、Rust MSVC 和项目要求的 Node.js 的 Windows x64 机器上执行。首次打包需要下载 Tauri 的 NSIS 工具，构建器会校验这些下载的哈希。

```powershell
npm --prefix apps/desktop ci
.\scripts\windows.ps1 -Mode package
pwsh -NoProfile -File .\scripts\test-windows-installer.ps1
```

安装验证先检查同产品安装、快捷方式和实例，再使用暂存目录、独立世界数据和 WebView2 配置。探针通过当前用户安装记录、二进制哈希、实际渲染、世界库读数以及保存退出的检查点核对产物，最后执行临时安装的卸载。若机器已有应用实例或安装，探针保留它们并停止。

## 本机结果

在 Windows 11 x64 构建 `10.0.26200.0`、本机 150% 缩放以及已经安装 WebView2 Runtime 的条件下完成验证。锁定构建使用 Tauri CLI 2.12.6、Tauri Rust 2.12.7、NSIS 3.13 和 `nsis_tauri_utils` 0.5.3。 [安装与清理断言](../../.local/m4-installer-evidence/45e4a6692788427587354c0f6a9cb8d2/installation.json)

| 验证项 | 实测结果 |
| --- | --- |
| 当前用户安装 | 退出码 0，安装记录版本 `0.1.0`，位置指向本次独立目录。探针使用 `/NS`，无新快捷方式。 |
| 安装内容 | 已安装应用的 SHA256 与发布二进制应用 Tauri NSIS 包类型标记后的预期值一致。Tauri 完成打包后恢复了独立发布二进制的原字节。 |
| 实际启动 | 四个 WebView 初始化，桌面林下画布为 `576 × 324` 物理像素。编辑器打开「林下切片」，世界库读取到 4 个内置起点。 |
| 保存退出 | 已安装应用以 0 退出，跟踪记录包含退出提交，独立目录保有 1 份已保存检查点。 |
| 卸载 | 退出码 0。删除临时安装文件和测试登记，清除本次创建的默认安装位置记录，保留隔离数据。 |

[原生画布检查](../../.local/m4-installer-evidence/45e4a6692788427587354c0f6a9cb8d2/webview-check.jsonl) · [已安装编辑器截图](../../.local/m4-installer-evidence/45e4a6692788427587354c0f6a9cb8d2/installed-editor.png) · [保存退出记录](../../.local/m4-installer-evidence/45e4a6692788427587354c0f6a9cb8d2/installed-app.jsonl) · [Tauri 打包代码](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.12.6/crates/tauri-bundler/src/bundle.rs)

| 产物 | 字节 | SHA256 |
| --- | ---: | --- |
| `微缩生态_0.1.0_x64-setup.exe` | 2,692,798 | `56E35CC61506CD632C7960DE9CB1DABAAA510A75958DE511DD980204CB200AA7` |
| `micro-ecosystem-desktop.exe` 独立发布二进制 | 11,091,968 | `85C02123E22E2B28C59998070FC191A4E5342644A9B71EA713B133AC215A4678` |

实测使用静默装卸参数。向导交互、缺少 WebView2 时的联网安装分支和旧安装包覆盖升级尚未实测。

# R5 Windows 安装包 0.2.0

2026-11-26 在已有 WebView2 的 Windows x64 上，以独立安装和数据目录验证了这个安装包。测试使用当前用户静默装卸。产物为未签名本地构建。

| 产物 | 大小 | SHA-256 |
| --- | ---: | --- |
| 0.2.0 安装包（本地副本已清理） | 74,351,877 字节 | `04BD2431A1659B15827D0CC20DBE31644B4865933BB1462EE79E8124A22D43CD` |
| 0.2.0 独立程序（本地副本已清理） | 82,879,488 字节 | `3CDEAC06B3F5BEDD246C257E6FC6E53A2540411BA23CE29184C5D65C41EE8BA4` |

## 浅塘编辑与画面

- 键盘草稿绑定选中物件与世界修订。起草后改选物件，再按回车不会发出提交。改选、轮换回到原物件、修订推进、拖动以及干涸浮叶均覆盖。隔离 Edge 使用实际编辑器和画布，编辑桥接由夹具模拟。 [编辑回归记录](evidence/r5-release/pond-editor-regression.json)
- 浅塘分别读取藻色、浑浊、生物膜和碎屑。藻色与浑浊表现于整个浸水部分，浑浊会减弱水下焦散。生物膜和碎屑在池底表现，干池可留下碎屑。固定时间和画幅的实际像素差分确认四项可分别观察，水线以上像素保持相同，干池不出现水色效果。 [像素差分](evidence/r5-release/pond-feedback-pixels.json) · [清水](evidence/r5-release/pond-clear.png) · [浑浊水](evidence/r5-release/pond-turbid.png)

## 安装验收

| 路径 | 观察结果 |
| --- | --- |
| 0.2.0 新装 | 系统登记和 PE 版本均为 0.2.0，安装文件符合 NSIS 预期载荷哈希。首启原生林下 V14，创建浅塘 V13。两套 576×324 设备像素的组件画布分别绘出完整场景。保存退出并重新启动后，两世界状态逐项恢复。卸载保留隔离测试档字节。 [安装记录](evidence/r5-release/install-fresh.json) · [首次启动](evidence/r5-release/installed-webview-check.jsonl) · [冷重开](evidence/r5-release/reopened-webview-check.jsonl) · [林下](evidence/r5-release/installed-forest-panel.png) · [浅塘](evidence/r5-release/installed-pond-panel.png) |
| 0.1.0 覆盖升级 | 实际安装旧包，保存 V4 林下和 V3 浅塘，再原地覆盖安装 0.2.0。覆盖期间旧数据逐文件 SHA-256 相同。新程序仍按原模型版本打开，世界 ID、步数、暂停、光照、生物和资源状态逐项相同。卸载保留测试档。 [升级记录](evidence/r5-release/install-upgrade.json) · [升级前](evidence/r5-release/prior-webview-check.jsonl) · [升级后](evidence/r5-release/upgraded-webview-check.jsonl) |

本包包含两套新场景、浅塘修复和新建入口。随后开发的功能以源代码和各自记录为准。向导逐页交互与缺 WebView2 的联网安装分支本次未执行。用户已将设备矩阵专项与生态长跑校准移出本轮验收。

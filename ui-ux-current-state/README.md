# 微缩生态 UI/UX 现状采集

整理日期为 2026-11-25（北京时间）。这份资料供未接触工程的设计协作者核对 Windows 微缩生态 0.1.0 的界面、操作路径和内容构成。后续视觉深化方向是用户确认的「像素工作台」。这里保留当前产品的画面和行为。

## 采集基线

取样使用现有 Windows x64 发布版。工程入口为 target/release/micro-ecosystem-desktop.exe，SHA256 为 85C02123E22E2B28C59998070FC191A4E5342644A9B71EA713B133AC215A4678。工程另有脚本模式 dev、build 与 package，分别用于开发启动、调试构建及发布打包。采集版本、WebView2 和屏幕读数详见 [capture-environment.json](capture-environment.json)。

该工作目录无法读取 Git 工作树元数据，分支、提交与未提交改动状态均记作 unknown。版本能精确绑定到实际可执行文件哈希。

## 离线入口

解压后打开 [index.html](index.html)，可以按界面名称、编号或 F01–F05 搜索并打开原图。文字在缩略预览里较小时，直接点图查看原始文件。

| 文件 | 用途 |
| --- | --- |
| [capture-environment.json](capture-environment.json) | 核对本次实际使用的版本、环境、窗口尺寸及隔离演示世界。 |
| [screen-inventory.csv](screen-inventory.csv) | 查看 A–M 的界面、进入路径、状态、原图编号、代码入口及未覆盖原因。 |
| [flows.md](flows.md) 与 [流程导航](index.html#flows) | 阅读五条操作路线，按顺序查反馈、状态变化、返回方式与截图。 |
| [implementation-map.md](implementation-map.md) | 了解每个界面的绘制方式、共享样式、状态、命令和持久化位置。 |
| [content-catalog.csv](content-catalog.csv) | 对照真正可操作的生态对象、绘制内容、模型资源和预设。 |
| [asset-index.html](asset-index.html) | 查看代表性现有源 PNG 的预览、用途及原始尺寸。 |
| [observations.md](observations.md) | 复核实际问题、对应证据及补采方式。 |
| [capture-ledger.jsonl](capture-ledger.jsonl) | 逐张核对图名、窗口、主题、世界、模式和具体停留状态。 |

## 证据读法

界面清单中的「运行实证」以本次画面或操作观察为依据，「代码确认」记录能从实现找到的连接，「部分实现」保留可用和待核查的范围，「未实现」则明确说明入口缺失。截图编号 UI-… 对应原始文件，状态编号 SCR-… 对应界面清单，W… 对应隔离演示世界，F01–F05 对应流程。一张原图可支持多个不同状态。

Windows 应用内的 WebView2 PNG 保留视图客户区的实际内容。Windows Graphics Capture 单独标记，少量完整窗口证据保留工具输出的 JPEG 原件。多个应用窗口重叠时，Windows 补证保留实际遮挡，核对被遮住的控件时请打开同窗口的客户区 PNG。对透明外边界、托盘菜单和多窗口关系的判断请同时核对证据的捕获方式及对应限制。窗口逻辑尺寸、客户区和原图像素尺寸分别记录。动态世界以该图采样时的世界刻度和光照设置为准。

工程路径用于定位实现，资料内提供相关作用。离线阅读只需要本目录，不依赖工程、安装包、网络、字体下载或历史交互白模。

## 本次覆盖

界面清单共 111 条状态，原始图共 70 张，其中 65 张为原生 WebView2 客户区 PNG，5 张为 Windows Graphics Capture 的窗口和系统菜单 JPEG。采集记录包含启动组件、世界库、林下与浅塘的大窗口观赏、生态编辑器、桌面面板与组件设置。关键状态包括预设及空白世界、石块与水体调整、有效性提示、右栏各区段、试运行、检查点、时钟和环境卡、布局、闲置、聚焦和重开后的读回。四类窗口均有原生客户区 PNG 与 Windows 窗口补证。 [界面清单](screen-inventory.csv) · [实际流程与操作结果](flows.md)

涉及创建、编辑、采用、归档、保存失败和重开的操作都在隔离数据目录执行。窗口切换与部分布局准备调用同一发行版的原生处理入口。五条流程按实际可核对的截屏顺序呈现。 [运行环境与演示世界](capture-environment.json)

实际缩小了编辑器，并保留 Alt+Space 系统菜单。Windows 通知区的托盘菜单只做了代码定位。透明区域的点击穿透、真实最小化与最大化、跨屏和混合 DPI 未做实机验证。其他未覆盖状态及最短补采步骤见 [问题与限制](observations.md)。

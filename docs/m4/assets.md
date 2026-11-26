# 0.1.0 本地版本素材来源

| 素材 | 源与使用 |
| --- | --- |
| 林下像素场景、鼠妇姿态和石块 | 项目内原创分层绘制。`assets/d1`、`assets/d2` 从同一可编辑生成源输出不同像素密度，包括树、底材、植被、岩石、阴影、虫体及动画帧。 [源文件](../../apps/desktop/src/render/source/forest_art.py) · [图层与密度说明](../../apps/desktop/src/render/README.md) |
| 浅水池塘、虾、水草与岸边 | 项目内原创分层绘制。由可编辑生成源导出 13 张 PNG。运行时以真实水位、溶氧所影响的个体活动和植物库存映射显示。 [源文件](../../apps/desktop/src/render/source/pond_art.py) · [素材和状态映射](../../apps/desktop/src/render/source/pond-art.md) |
| 四季与昼夜色温 | 项目内编写的颜色映射，以权威模拟日历和世界光相位为输入。 [调色表](../../apps/desktop/src/render/seasonColor.ts) · [画面对照](../m3/season-render.md) |
| 应用与托盘图标 | 交付使用此前工作区保留的 ICO 文件。当前仓库未记录更早的绘制来源。 [图标文件](../../apps/desktop/src-tauri/icons/icon.ico) |
| 界面字体 | Windows 系统字体栈，不随安装包分发字体文件。 [界面样式](../../apps/desktop/src/styles.css) |

`ecosystem-handoff/references/originals` 中九张图片用于构图、遮挡和色彩参考。应用构建输入为 `apps/desktop/dist`，参考图没有作为运行贴图导入。锁定的第三方依赖由 Cargo 与 npm 锁文件标识。 [参考范围](../../ecosystem-handoff/references/README.md) · [Tauri 构建配置](../../apps/desktop/src-tauri/tauri.conf.json)

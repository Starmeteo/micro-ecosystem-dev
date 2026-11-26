# R5 画面与编辑器后续验证

## 画面

- 林下 V14 和浅塘 V13 同内容重复快照各送达 12 次，实际 Canvas 整帧绘制由各 12 次降至 0 次。优化前后像素哈希相同。生态步数、编辑修订、草稿、相位、缩放、标尺、重新打开与同一步数的新运行副本仍触发绘制。缩放通知由隔离 Edge 控制。 [优化前](evidence/editor-followup/canvas-before.json) · [优化后](evidence/editor-followup/canvas-after.json)
- 浅塘 V13 读取已持久化年龄。成虾用约 20 mm 的体轴美术档，幼虾约 9 mm。游泳和沉木停靠共用同一年龄映射。隔离 Edge 中，实际合景游泳图像整只虾的差异包围跨度为 59 和 26 像素，停靠为 58 和 26 像素。外伸触须计入此包围跨度。 [画面差分](evidence/editor-followup/shrimp-stage-pixels.json) · [生物面板](evidence/editor-followup/biology-stage.png)
- 原有 PixiJS 林下渲染器改为按需载入。最终 0.2.1 的入口 JS 为 680.69 kB，按需渲染块为 328.95 kB。Vite 仍对大于 500 kB 的入口提示。这里记录构建文件大小，实际安装包仍包含两套场景的完整素材。

## 编辑

- 林下 V14 在「内容」使用落叶笔刷。每格采用所选落叶的初始 C/N/P 剂量。绘制中只显示预览，抬起时才提交已标出的可用格。落叶上限仍是 16 处。隔离 Windows 原生编辑器实拖经过 9 格，避开 2 格后放入 7 份落叶，计入 168 mg C。一次撤销全部恢复，重做取回相同 ID，Esc 取消的下一笔保持原修订。 [拖动预览](evidence/editor-followup/brush-before-release.png) · [提交后](evidence/editor-followup/brush-committed.png) · [核验记录](evidence/editor-followup/brush-checks.jsonl)
- 林下 V14 与浅塘 V13 在「生物」提供数量和类型。鼠妇、米虾以已有成年功能型和明确落点投放。水蚤增加当前水域的群体数量。Rust 核对有效地面或水域、个体上限与碳氮磷的外部输入。隔离 Windows 编辑器投放鼠妇 4→6、米虾 3→5、水蚤 48→60，并保存退出。冷重开保留已投放的个体、位置、阶段与库存。撤销栈随进程重新创建。 [投放回执](evidence/editor-followup/stock-checks.jsonl) · [冷重开](evidence/editor-followup/stock-reopen-checks.jsonl) · [林下](evidence/editor-followup/stock-forest.png) · [浅塘](evidence/editor-followup/stock-pond.png)
- V14 林下「内容 → 种植地表苔藓」以每格一块、8 mg C 的实验剂量投放活体苔藓。一个土格的苔藓继续由现有生长、呼吸与湿度胁迫过程计算。移植只搬动所选苔藓的份额。修剪计入当前土格枯落物，取出计为世界输出。0.2.1 原生操作将 ID 4 移到另一土格，8 mg C 仍在同一斑块，修剪 2 mg C 后斑块剩余 6 mg C，土格枯落物增加 2 mg C。取出后可撤销。 [林下原生画面](evidence/editor-followup/plant-forest.png)
- V13 浅塘「内容 → 轮生细叶水草」投入每丛 50 mg C 的实验剂量，最多八丛。每丛持有原有全池水草库存的份额，未分配份额仍按原画面呈现。原生操作把同一丛沿池底微移，50 mg C 保持原量。修剪 10 mg C、移除剩余 40 mg C 都转入碎屑，撤销恢复同一丛。独立 Edge 使用实际画布与模拟宿主额外核对了拖动、干露、斑块积累量变化造成的画面档位，以及与附着苔藓的独立选择。 [浅塘原生画面](evidence/editor-followup/plant-pond.png) · [绘图与交互回归](evidence/editor-followup/pond-water-plant-canvas.json)
- 保存并重新启动 0.2.1 后，两套植被的 ID、位置、库存和世界状态均与退出前相同。 [原生操作](evidence/editor-followup/plants-checks.jsonl) · [冷重开](evidence/editor-followup/plants-reopen-checks.jsonl) · [128 项工作区测试](evidence/editor-followup/workspace-tests-0.2.1.txt)

复制当前世界保留已投放生物和斑块现有库存。另存初始预设沿用主题的起始生物，保留已设计斑块的落点并恢复其初始剂量。

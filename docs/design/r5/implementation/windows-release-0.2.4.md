# R5 群体观察与浅塘底床 · 0.2.4

本版承接 0.2.3 的两套 R5 场景、活体蕨丛、沉水落叶和双水域布局。 [0.2.3 交付与验收](windows-release-0.2.3.md)

| 产物 | 大小 | SHA-256 |
| --- | ---: | --- |
| Windows x64 安装包（本地副本已清理） | 74,447,659 字节 | `EF1DBCD9BE7C3B4CE9E77B132C6200F7FB6882F0919457FB8EB74E669B80076E` |
| 0.2.4 独立程序（本地副本已清理） | 83,266,048 字节 | `F5D7C5071AC1E96C78690DF5A5EFB9D35D0664C537B9219D997B32D629F425C1` |

## 林下 F13 弹尾虫群

在「世界编辑器 → 生物 → 弹尾虫群」查看左右区的实际库存，单位为 mg C。「显示观测点」打开全景微点，也可选择「4× 左区」或「4× 右区」。每区的库存除以 2.5 mg C 向上取整得到微点数，上限 9 个，零库存对应零个点。每个微点按 1.5 mm 的设计长度共用林下比例，石木、叶片和植物依原图层在其前方遮挡。

微点用来比较群体相对密度。世界继续按土格聚合库存运行，观测操作不提交编辑命令。微点无法拾取或用于逐只计数。常态场景默认保持原先的画面。

在 150% 的隔离 Edge 中运行正式画布，向编辑器注入左有、右有、两侧有和两侧为零的快照。相同图像条件下，只有左侧有库存改变左侧 35 个像素，只有右侧有库存改变右侧 27 个像素，两侧同时有库存叠加一致。零库存和关闭观测恢复同一画布，缺失库存有单独提示。4× 视图分别裁出左右的 144×81 参考画面。 [逐像素检查](evidence/observe-gravel-0.2.4/springtail-edge-checks.json)

Windows 原生编辑器切换全景、4× 左区、4× 右区和隐藏，快照中的碳库存、模拟步和编辑修订相同。 [全景](evidence/observe-gravel-0.2.4/springtails-full.png) · [左区 4×](evidence/observe-gravel-0.2.4/springtails-left-4x.png) · [右区 4×](evidence/observe-gravel-0.2.4/springtails-right-4x.png)

## 浅塘 P04 小砾砂斑

在「编辑环境 → 内容 → 场景摆件」选择「小砾砂斑（仅造景）」，在池底落位。斑块沿主轴约 100 mm，使用独立砾石素材组成的透明图层，细粒沿边缘疏开，并有短接触影。水下图层接受当前水面光纹和全景光色，石木和生物依原有顺序覆盖。池底露出时保留同一位置。

拿起时显示预览，松开在合法底床才提交。可整组拖动、移除、撤销和重做。水量、溶氧、营养与可通行空间继续由原生态状态决定。细砂和粗砂材质后续补独立贴合底床的素材。

原生编辑器在干底和补水后的池底各放一处。底床外侧点按和按下后拖离池底再松开均未提交。新增前后各项水体读数相同，撤销和重做找回原实例 ID。隔离 Edge 另覆盖悬空点按与拖动已有砂斑出界。 [露底](evidence/observe-gravel-0.2.4/gravel-dry.png) · [补水后](evidence/observe-gravel-0.2.4/gravel-wet.png) · [Edge 出界](evidence/observe-gravel-0.2.4/gravel-invalid.png) · [实际操作](evidence/observe-gravel-0.2.4/groups-checks.jsonl)

## 验收

Windows Rust 工作区 143 项通过，前端类型检查和生产构建通过。砂斑回归覆盖编辑历史、试运行、复制及持久化。原生 Windows 进程冷重开恢复两套世界，保留砂斑位置和水量。设备专项和成对种子专项按确认范围排除。 [Rust 回归](evidence/observe-gravel-0.2.4/workspace-tests.txt) · [冷重开](evidence/observe-gravel-0.2.4/groups-reopen-checks.jsonl)

Windows 11 x64 隔离新装及 0.2.3 → 0.2.4 覆盖升级通过。新装创建并重开两套场景，组件画布均为 576×324。覆盖前后测试存档逐文件 SHA-256 相同，旧林下保留修剪到 3 mg C 的蕨丛，浅塘两池保留 18 L 和 18.8 L，第二池保留原位置及状态的 50 mg C 沉叶。两套升级后的持久世界状态逐项一致。两次卸载清理临时安装并保留测试档。 [新装记录](evidence/observe-gravel-0.2.4/install-fresh.json) · [升级记录](evidence/observe-gravel-0.2.4/install-upgrade.json) · [旧档回读](evidence/observe-gravel-0.2.4/upgraded-webview-check.jsonl) · [第二水域实拍](evidence/observe-gravel-0.2.4/upgraded-second-basin.png)

本机验收使用已有 WebView2 与当前用户静默装卸。Vite 报告主入口 gzip 前为 711.64 kB，超过打包工具的分块提示阈值。该数值留作后续启动性能对照。 [打包日志](../../../../.local/r5-observe-gravel/package-0.2.4.log)

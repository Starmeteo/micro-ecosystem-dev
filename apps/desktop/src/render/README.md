# R5 生态场景素材

正式林下和浅水池塘场景从同一份世界快照组装独立图层。旧世界保留原来的生态状态和编辑能力，画面由 R5 的背景、物件、生物和光照层重新绘制。

| 目录 | 作用 |
| --- | --- |
| assets/r5/forest-composed | 林下地表、落叶、枝根、石块、朽木、植被和鼠妇。definition.json 记录体轴、接触锚点及配景。 |
| assets/r5/pond-composed | 浅塘水层、沉积底床、石木、植被、虾和光学层。definition.json 整理尺寸、接触锚点及配景，另外两份 JSON 定义物件轮廓。 |
| assets/r5/snails | 两种螺类的独立形态与姿态图集。 |
| assets/r5/placement-geometry.json | 编辑器预览所需的地表占地、落点范围和浅塘接触尺寸。 |

源图保留透明边缘和独立物件。编辑器、观赏窗、组件共享场景渲染，浅塘在组件采用水下近景，在编辑器与观赏窗保留全池取景。模型由 Rust 决定物件位置、生命与昼夜状态。

[R5 林下美术实拍](../../../../docs/design/r5/implementation/evidence/forest-composed/editor-day.png) · [碎叶生成](source/r5_leaf_fragments.py) · [塘底砂斑生成](source/r5_pond_sand.py)

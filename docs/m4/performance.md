# Windows 发布版资源与视图压力

使用与安装包对应的发布版可执行文件，另建世界目录和 WebView2 配置。Windows 11 构建 26200，Intel Core Ultra 9 275HX，24 逻辑处理器，约 31.4 GiB 内存，平衡电源方案，单屏 150% 缩放，WebView2 157.0.4341.60。枚举到 Intel Graphics 和 GeForce RTX 5060 Laptop GPU。内容使用默认四只鼠妇、三只小虾和 48 只水蚤。 [安装产物哈希](windows-install.md) · [硬件与驱动记录](resource-hardware.json)

## 采样口径

- CPU 取应用进程和本次 WebView2 子进程累计处理器时间的变化，按 24 个逻辑处理器换算为整机可用 CPU 的百分比。不会采集其他应用的 CPU。 [Microsoft 进程计数说明](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.management/get-process)
- GPU 读取 Windows 的 GPU Engine 计数，将本次进程树在同一 GPU 引擎的占用相加并取最忙引擎。用的是引擎占用百分比。没有测量功率、电池续航、显存构成和 DWM 的额外开销。 [Microsoft GPU 计数口径](https://devblogs.microsoft.com/directx/gpus-in-the-task-manager/)
- 内存为进程树的 Private Bytes 和工作集。多个进程映射同一共享页时，简单相加的工作集可能重复计入。循环后小幅变化只用于观察趋势。较短的分组中，P95 接近单组最大值。
- 观赏窗口另行采样 WebView2 的 `requestAnimationFrame` 间隔和 32 次快照调用往返。帧回调反映该 WebView2 的回调节奏。测量没有包含物理按键到显示的总时延，也没有从合成器取实际呈现帧。

## 本机结果

表中每格为中位数 / P95。前七组各约 8 秒，扣除阶段边沿后保留 5–6 个样本。双主题连续观赏组为 60 秒，44 个样本。首轮 CPU 采样将低于一秒的进程时间差交给整数重载，导致读数失真。下表和汇总 JSON 均为修正后复测。 [完整汇总](resource-summary.json) · [原始采样](../../.local/m4-native/performance-20261124-220549-519/resource-samples.jsonl)

| 状态 | 整机 CPU % | 最忙 GPU 引擎 % | 私有内存 MiB |
| --- | ---: | ---: | ---: |
| 单世界，隐藏全部窗口 | 0.05 / 0.14 | 0 / 0 | 447 / 447 |
| 单世界，常态组件 | 0.24 / 0.98 | 4 / 5 | 424 / 426 |
| 单世界，大观赏窗 | 0.27 / 0.93 | 7 / 9 | 452 / 453 |
| 同世界，组件和大观赏窗 | 0.34 / 0.59 | 10 / 10 | 440 / 442 |
| 单世界，组件时钟覆盖 | 0.19 / 0.25 | 0 / 0 | 457 / 458 |
| 双世界，隐藏全部窗口 | 0.00 / 0.40 | 0 / 1 | 464 / 465 |
| 双世界，显示两个视图 | 0.44 / 1.01 | 5 / 6 | 467 / 472 |
| 双世界，连续观赏 60 秒 | 0.44 / 1.06 | 5 / 6 | 452 / 464 |

在 720 × 450 逻辑像素的大观赏窗，108 次帧回调的间隔中位数 16.7 ms，P95 16.9 ms。单份权威快照序列化约 2,642 字节，32 次调用往返中位数 0.8 ms，P95 1.3 ms。组件覆盖可自动出现，获取键盘焦点后恢复，每个阶段的世界仍按固定步长推进。

两轮各重复了 24 次打开和隐藏视图。最后一次隐藏后的原生窗口事件确认可见视图和订阅数均为 0。复测双世界区间各走 895 步。首轮额外完成了 120 秒双主题连续观赏，两世界区间分别走 1,536 和 1,535 步，原生轨迹验证两份默认自动检查点均已提交。首轮可用的 88 个连续观赏内存样本中位数约 476 MiB、P95 约 496 MiB。占用以进程树为单位，隐藏后 WebView 仍保留。短时样本不能确定更长时间的内存趋势，也没有建立可运行的最大世界数。 [第二轮原生轨迹](../../.local/m4-native/performance-20261124-220549-519/native.jsonl) · [两分钟运行与自动保存轨迹](../../.local/m4-native/performance-20261124-220148-250/native.jsonl)

## 复验

运行 `pwsh -NoProfile -File .\scripts\m4-native.ps1 -Mode performance -SampleSeconds 8 -SoakSeconds 60`。用 `-SoakSeconds` 延长双世界连续观赏阶段。测量脚本每次将结果写入隔离目录，不使用原用户存档。脚本记录采样开始与结束，按阶段汇总中位数、P95、最大值和首末私有内存。 [采样脚本](../../scripts/m4-resources.ps1) · [原生场景脚本](../../scripts/m4-native.mjs)

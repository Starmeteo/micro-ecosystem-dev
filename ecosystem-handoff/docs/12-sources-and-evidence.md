# 来源、证据与使用范围

整理日期：2026-11-24。本文用于后续开发者继续核查依据，不表示模型参数已经全部获得或模型已经完成科学验证。

## 1. 用户需求与视觉证据

产品需求来自本次对话；决定状态见 01-project-baseline.md。用户对“真实”的说明以生物间、生物与环境间的相互影响为核心，对视觉偏好使用上传原图作为证据。没有给未明确的选项补造用户回复。

九张原图位于 references/originals，顺序和原文件信息见 references/manifest.json。逐图观察见 references/README.md。图像可以支持颜色、构图、遮挡、像素簇和明暗层次的分析；静态图不能证明动画帧率、原作品引擎、源像素尺寸或生物生态参数。

原编辑器草图来自前面讨论中生成的 HTML，仅能说明其中实际存在的操作概念，不证明生态或 Windows 产品能力。它没有被当作美术完成标准。

## 2. 工程资料

| ID | 来源 | 本包使用的依据 | 仍需做什么 |
| --- | --- | --- | --- |
| TECH-01 | [Tauri Calling the Frontend](https://v2.tauri.app/develop/calling-frontend/) | 命令/事件/Channel 的通信分工，持续数据传输的设计方向 | 对选定版本核查API与能力；Channel不是持久队列或跨重启恰好一次保证 |
| TECH-02 | [Tauri State Management](https://v2.tauri.app/develop/state-management/) | Tauri 应用层的共享状态管理背景 | 生态内核仍保持独立，具体并发策略由项目验证 |
| TECH-03 | [PixiJS 8 Render Loop](https://pixijs.com/8.x/guides/concepts/render-loop) | Ticker/绘制节奏的用途 | 生态时钟独立于绘制；实际后端、缩放与Windows表现需实测 |
| TECH-04 | [Rust std::time::Instant](https://doc.rust-lang.org/std/time/struct.Instant.html) | 单调时钟语义与系统挂起处理的限制 | 不能仅凭两次Instant之差承诺离线/睡眠暂停；选择明确适配方案 |
| TECH-05 | [Microsoft QueryUnbiasedInterruptTimePrecise](https://learn.microsoft.com/en-us/windows/win32/api/realtimeapiset/nf-realtimeapiset-queryunbiasedinterrupttimeprecise) | Windows 提供排除睡眠/休眠时间的计时能力 | 与生命周期事件和调度策略一起验证；不把一个API当成全部恢复机制 |

React 19、TypeScript、Rust、Tauri 2 和 PixiJS 8 是当前规划的技术方向。此包没有指定未经验证的补丁版本、包管理器、存档编码库或随机库。Codex 首轮应核查当前官方兼容性资料，开工后再锁定依赖。开发环境、打包目标、WebView 运行时和最低 Windows 支持范围都应明确记录。

## 3. 生态资料

| ID | 来源 | 支持的机制 | 不支持的外推 |
| --- | --- | --- | --- |
| ECO-S01 | [USDA NRCS Soil Biota](https://www.nrcs.usda.gov/sites/default/files/2024-05/Rangeland%20Soil%20Health%20Soil%20Biota%20SD-FS-85b%205-9-24.pdf) | 分解、微生物暂存养分与后续释放等一般土壤过程 | 不直接提供本林下预设的全部参数或稳定性保证 |
| ECO-S02 | [Hassall 等，鼠妇与枯落物分解研究](https://ueaeprints.uea.ac.uk/id/eprint/34736/) | 鼠妇摄食、碎化、枯落物状态和微环境之间的关系 | 不把所有摄食量立即转成矿物养分；不同材料效果不必相同 |
| ECO-S03 | [鼠妇温湿度与聚集相关原始研究](https://pmc.ncbi.nlm.nih.gov/articles/PMC4525041/) | 微环境影响聚集和相关生活过程，物种存在差异 | 不给所有鼠妇统一的温湿度死亡阈值 |
| ECO-S04 | [蕨类水分调节与干燥后恢复研究](https://pmc.ncbi.nlm.nih.gov/articles/PMC11688535/) | 水分调节能力与恢复能力应区分 | 不推出所有苔藓或蕨类干燥后都立即死亡/全部恢复 |
| ECO-S05 | [EPA Dissolved Oxygen](https://www.epa.gov/caddis/dissolved-oxygen) | 光合、呼吸、分解、水气交换和环境条件共同影响溶氧 | 不直接给出本项目的精确过程系数 |
| ECO-S06 | [USGS 水质昼夜变化](https://www.usgs.gov/centers/new-jersey-water-science-center/science/diurnal-variations-water-quality-surface-water) | 光合与持续呼吸可以产生水体昼夜变化 | 不把每个池塘都预设为相同幅度的氧气曲线 |
| ECO-S07 | [EPA Ammonia](https://www.epa.gov/caddis/ammonia) | 氨形态与温度/pH关系，硝化及耗氧耦合 | 本首版未实现完整氮化学，不能显示精确氨毒性结论 |
| ECO-S08 | [Neocaridina davidi 生物膜培养原始论文摘要记录](https://agris.fao.org/search/en/records/65df4cd8b766d82b1801181d) | 在研究所述条件下，生物膜可支持米虾生长和生活史过程 | 不保证任意膜层都具有完整营养，也不把米虾视为万能水体过滤器 |
| ECO-S09 | [水蚤等浮游动物对天然藻类与细菌摄食的原始研究](https://limnetica.info/pt/zooplankton-grazing-natural-algae-and-bacteria-under-hypertrophic-conditions) | 摄食受食物组成与环境影响 | 不保证水蚤抑制所有藻华，不将特定实验参数直接通用于全部物种 |
| ECO-S10 | [Ceratophyllum demersum 营养竞争研究](https://www.frontiersin.org/journals/plant-science/articles/10.3389/fpls.2022.963579/full) | 金鱼藻作为候选沉水植物，其营养利用需按实际生物学定义 | 不将其当成有根吸收沉积物养分的通用模板，不保证候选混养组合已验证 |

## 4. 从机制到模型的工作

每个参数需记录：适用物种或功能群、单位、实验条件、来源、如何映射到模型、采用范围、校准场景和不确定性。原始研究中的某个实验值不自动等于产品默认值。

对于没有足够资料的过程，可提出简化形式、合理的试验范围和验证办法，明确标记其工程近似性质。参数登记表中的空值表示待查证或校准。仅凭模型“跑得很稳定”也不能证明它科学准确；需要检查资源、过程和受控干预的响应。

## 5. 来源更新

若链接失效，优先按标题、DOI或官方文档路径寻找原始发布方的新位置。不要将未读到的全文写成已阅读。新增来源时说明它支持什么结论，以及哪些部分仍是本项目自行提出的设计。

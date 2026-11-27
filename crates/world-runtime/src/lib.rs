//! World ownership and persistence, independent of any window or system clock.

mod backup;
mod checkpoint;
mod library;
mod observation;
mod trial;
pub mod solar;

pub use backup::{WorldBackupImport, MAX_WORLD_BACKUP_BYTES};
pub use checkpoint::{SaveReceipt, SaveStore};
pub use library::{PresetEntry, PresetSource, WorldEntry, WorldEntryState, WorldTheme};
pub use observation::ObservationHistory;
pub use eco_core::{Activity, Animal as AnimalSnapshot, ForestBarkPreset, ForestBarkShelter, ForestLeafPatch, ForestLeafPreset, ForestLog, ForestRock,
    ForestRockEdit, ForestScenery, ForestSceneryParent, ForestStonework, ForestStoneworkKind, Point, PondEdit, PondMossParent, PondMossPreset,
    PondLeafDepositPreset, PondShapeError, PondShapeMode, PondStone, PondStonePreset, PondWood, PondWoodPreset, STEP_MILLIS};
pub use trial::{TrialForkBase, TrialRate};

use eco_core::{CoreError, ForestFernPatchObservation, ForestFernPresetObservation, ForestLayoutEditState, ForestLedger,
    ForestMossPatchObservation, ForestMossPresetObservation,
    ForestObservation, ForestSubstrateCellObservation, ForestSubstratePresetObservation, PondContactFeeding, PondLedger, PondObservation, SeasonObservation,
    WorldState, FOREST_R5_BARK_MODEL_VERSION, FOREST_R5_COMPOSED_MODEL_VERSION, FOREST_R5_LEAF_MODEL_VERSION, FOREST_R5_MODEL_VERSION, FOREST_R5_STONEWORK_MODEL_VERSION, FOREST_R5_TERRAIN_MODEL_VERSION,
    FOREST_SEASONAL_MODEL_VERSION, POND_MODEL_VERSION, POND_R5_COMPOSED_MODEL_VERSION, POND_R5_MOSS_MODEL_VERSION, POND_R5_STONES_MODEL_VERSION,
    POND_R5_WOOD_MODEL_VERSION};
use observation::RECENT_INTERVAL_TICKS;
use serde::{Deserialize, Serialize};
use solar::{SolarLocation, SOLAR_MODEL_VERSION};
use std::collections::BTreeMap;
use std::fmt;
use uuid::Uuid;

pub const SNAPSHOT_PROTOCOL_VERSION: u32 = 1;

fn default_solar_config() -> SolarConfig {
    SolarConfig { location: SolarLocation::beijing_sample(), model_version: SOLAR_MODEL_VERSION }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub protocol_version: u32,
    pub world_id: String,
    pub run_generation: String,
    pub edit_revision: String,
    pub simulation_tick: String,
    pub simulated_seconds: f64,
    pub simulated_millis: u64,
    pub paused: bool,
    pub manual_paused: bool,
    pub edit_paused: bool,
    pub daylight_phase: f64,
    pub model_version: u32,
    pub definition: String,
    pub pond_undo_available: bool,
    pub pond_redo_available: bool,
    pub starter_community_available: bool,
    pub rock: Point,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forest_rocks: Option<ForestRockSnapshot>,
    pub animals: Vec<AnimalSnapshot>,
    pub moisture: [f64; 2],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forest: Option<ForestObservation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pond: Option<PondObservation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub season: Option<SeasonObservation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<SolarLocation>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadReceipt {
    pub snapshot: Snapshot,
    pub recovered_previous: bool,
    pub checkpoint_serial: String,
    pub automatic: bool,
}

/// Changes to the physical state or durable operating configuration.
/// Re-sampling the real daylight target alone does not dirty a paused world.
#[derive(Clone, Debug, PartialEq)]
pub struct CheckpointStamp {
    generation: String,
    tick: u64,
    revision: u64,
    paused: bool,
    edit_paused: bool,
    location: Option<SolarLocation>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "code", rename_all = "snake_case", rename_all_fields = "camelCase")]
pub enum RuntimeError {
    InvalidWorldId,
    InvalidLibraryName,
    PresetMissing { preset_id: String },
    LibraryUnreadable { details: String },
    InvalidCommandId,
    WorldMissing { world_id: String },
    WorldAlreadyLoaded { world_id: String },
    StaleGeneration { current_generation: String },
    StaleRevision { current_revision: String },
    CommandReuse,
    EditRequiresPause,
    InvalidLocation,
    TrialMissing,
    TrialConflict,
    TrialOperationInvalid,
    InvalidTrialTicks,
    PondHistoryEmpty,
    ForestRockHistoryEmpty,
    Suspended,
    RevisionOverflow,
    InvalidFixtureInput { details: String },
    PondShape { issue: PondShapeError },
    CheckpointMissing { world_id: String },
    CheckpointUnusable { world_id: String, details: String },
    CheckpointUnsupported { world_id: String, details: String },
    CheckpointIo { details: String },
    WorldBackupInvalid,
    WorldBackupUnsupported,
    WorldBackupTooLarge,
    WorldBackupIo { details: String },
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWorldId => formatter.write_str("invalid world ID"),
            Self::InvalidLibraryName => formatter.write_str("请输入 1 到 80 个可见字符作为名称"),
            Self::PresetMissing { preset_id } => write!(formatter, "预设 {preset_id} 不存在"),
            Self::LibraryUnreadable { details } => formatter.write_str(details),
            Self::InvalidCommandId => formatter.write_str("invalid command ID"),
            Self::WorldMissing { world_id } => write!(formatter, "world {world_id} is not loaded"),
            Self::WorldAlreadyLoaded { world_id } => write!(formatter, "world {world_id} is already loaded"),
            Self::StaleGeneration { current_generation } => write!(formatter, "stale generation, current generation is {current_generation}"),
            Self::StaleRevision { current_revision } => write!(formatter, "stale edit revision, current revision is {current_revision}"),
            Self::CommandReuse => formatter.write_str("command ID already belongs to a different request"),
            Self::EditRequiresPause => formatter.write_str("编辑或启动试运行前，请先暂停世界"),
            Self::InvalidLocation => formatter.write_str("日照地点无效。请检查名称和经纬度"),
            Self::TrialMissing => formatter.write_str("试运行副本不存在"),
            Self::TrialConflict => formatter.write_str("正式世界已有变化，无法采用此次结果。可以另存试运行，或从当前世界重新开始"),
            Self::TrialOperationInvalid => formatter.write_str("请使用试运行操作修改或保存副本"),
            Self::InvalidTrialTicks => formatter.write_str("每次试运行可推进 1 到 1200 步"),
            Self::PondHistoryEmpty => formatter.write_str("本次暂停编辑中没有可撤销或重做的水域修改"),
            Self::ForestRockHistoryEmpty => formatter.write_str("本次暂停编辑中没有可撤销或重做的布置修改"),
            Self::Suspended => formatter.write_str("系统挂起期间不能推进或编辑世界"),
            Self::RevisionOverflow => formatter.write_str("edit revision would overflow"),
            Self::InvalidFixtureInput { details } | Self::CheckpointIo { details }
                | Self::WorldBackupIo { details } => formatter.write_str(details),
            Self::WorldBackupInvalid => formatter.write_str("无法读取世界备份。请检查文件格式及完整性。"),
            Self::WorldBackupUnsupported => formatter.write_str("当前版本不支持这份备份的文件格式或世界模型。"),
            Self::WorldBackupTooLarge => formatter.write_str("世界备份超出当前支持的大小。"),
            Self::PondShape { issue } => formatter.write_str(match issue {
                PondShapeError::InvalidCells => "请在水域俯视图内选择有效的方格。",
                PondShapeError::ProtectedRow => "侧面观察剖面需要保持完整，不能擦除这一排。",
                PondShapeError::Unchanged => "这次涂刷没有改变水域形状。",
                PondShapeError::Disconnected => "每个水域必须连成一片。请保留通路或连续涂刷。",
                PondShapeError::TouchesOther => "请在两个独立水域之间留出空格。",
                PondShapeError::AreaOutOfRange => "涂刷后的水面面积须在 400 至 10,000 cm² 之间。",
                PondShapeError::Capacity => "涂刷后的容量小于现有水量。请扩大水域或先排水。",
                PondShapeError::Occupied => "新的水深无法保留安全的活动空间。请减少面积变化。",
                PondShapeError::TopologyOperation => "请画一段连接两片水域的水道，或用填岸把当前水域分成多片。",
                PondShapeError::TopologyOccupied => "当前剖面布置无法安全保留。请检查分合后的水深及生物的活动空间。",
                PondShapeError::TopologyLostAnchor => "填岸会覆盖活体浮叶中心。请先移动浮叶，再重新预览分池。",
                PondShapeError::TopologySedgeAnchor => "改形会移除莎草所在的岸边。请先移动莎草。",
                PondShapeError::TopologyPopulation => "分合后至少一个水域的水蚤数量或浮叶数量会超过允许上限。",
                PondShapeError::TopologySection => "分合后每片水域须保留一整排水格作为观景剖面。",
                PondShapeError::TopologySectionLimit => "同一水域最多保留四条观景剖面。",
                PondShapeError::TopologyScale => "参与合池的水域须使用相同的单格面积。",
                PondShapeError::TopologyLimit => "世界须保留一至四片水域。",
                PondShapeError::TopologyShallow => "分配后的水域须全干或至少保有 0.05 L 水。",
            }),
            Self::CheckpointMissing { world_id } => write!(formatter, "no committed checkpoint exists for world {world_id}"),
            Self::CheckpointUnusable { world_id, details } => write!(formatter, "no usable checkpoint for world {world_id}: {details}"),
            Self::CheckpointUnsupported { world_id, details } => write!(formatter, "unsupported checkpoint for world {world_id}: {details}"),
        }
    }
}

impl std::error::Error for RuntimeError {}

impl From<CoreError> for RuntimeError {
    fn from(error: CoreError) -> Self {
        let details = match error {
            CoreError::InvalidRockSite => "请将石块放在场景前方的可用地面，留出边缘空间。",
            CoreError::OccupiedRockSite => "该地表格内有动物，请选择其他落点。",
            CoreError::OverlappingRockSite => "该地表格已有石块，请选择其他落点。",
            CoreError::ForestRockLimit => "此世界最多可放八块石头。",
            CoreError::ForestRockMissing => "所选物件已经移除。",
            CoreError::InvalidLogSite => "请将木段放在场景中部的地面，为两端留出空间。",
            CoreError::OccupiedLogSite => "木段落点覆盖的地表格内有动物，请选择其他落点。",
            CoreError::OverlappingLogSite => "落点与其他石块或木段占用的地表格重叠，请选择其他落点。",
            CoreError::ForestLogLimit => "此世界最多可放四根木段。",
            CoreError::InvalidStoneworkSite => "请把扁石或小石组放在场景地面，为每块石头留出空间。",
            CoreError::OccupiedStoneworkSite => "这个落点的接地格内有动物，请选择其他落点。",
            CoreError::OverlappingStoneworkSite => "这个落点的接地格与已有石木重叠，请选择其他落点。",
            CoreError::ForestStoneworkLimit => "此世界最多可放八件扁石或小石组。",
            CoreError::ForestNoOpenGround => "场景需要留出可通行地面，请先移开部分石木。",
            CoreError::InvalidBarkSite => "请把拱形树皮放在场景中部，给前后洞口留出位置。",
            CoreError::OccupiedBarkFeet => "树皮支脚要落地的位置有动物，请换个位置。",
            CoreError::OverlappingBarkSite => "树皮支脚与已有石木的接地格重叠，请换个位置。",
            CoreError::BlockedBarkPassage => "这会堵住树皮下方或洞口的通路，请换个位置。",
            CoreError::ForestBarkLimit => "此世界最多可放六件拱形树皮。",
            CoreError::InvalidLeafPatchBatchSize => "落叶刷至少需要一格可投放的地面。",
            CoreError::InvalidLeafFragmentDose => "每次碎叶铺设的总投放量必须为 0.1 至 240 mg C。",
            CoreError::InvalidLeafPatchSite => "请把落叶放在场景前方的地面，给边缘留出空间。",
            CoreError::OverlappingLeafPatchSite => "落叶需要独立的地表格，请避开石木、洞口通道和其他落叶。",
            CoreError::ForestLeafPatchLimit => "此世界最多可放十六处落叶。",
            CoreError::InvalidForestMossSite => "请把活苔藓放在画面前方的地面。",
            CoreError::BlockedForestMossSite => "同一地表格只能放一块活苔藓。请避开石木支脚，或先移开原来的斑块。",
            CoreError::ForestMossPatchLimit => "这个世界的活苔藓斑块数量已达上限。",
            CoreError::ForestMossPatchMissing => "所选活苔藓已移除。",
            CoreError::InvalidForestMossTrim => "修剪量必须大于零并小于当前碳储量。若要整株取走，请使用移除。",
            CoreError::InvalidForestFernSite => "请在画面前方的地面种植蕨丛。",
            CoreError::BlockedForestFernSite => "请给蕨丛留出地面空间，避开石木、树皮通道和其他蕨丛。",
            CoreError::ForestFernPatchLimit => "这个世界的蕨丛数量已达上限。",
            CoreError::ForestFernPatchMissing => "所选蕨丛已移除。",
            CoreError::InvalidForestFernTrim => "修剪量必须大于零并小于蕨丛的当前碳储量。若要整丛取走，请使用移除。",
            CoreError::InvalidForestSubstrateCells => "请选择一至四十八个不同的有效地表格。",
            CoreError::ForestSubstrateTooWet => "至少一个地表格的现有水量超过目标底材容量。本次涂刷未应用，请改用更大容量底材或等地面变干。",
            CoreError::ForestSubstrateUnchanged => "所选地表格已经是这个底材。",
            CoreError::InvalidWoodlouseStockCount => "至少投放一只鼠妇。",
            CoreError::InvalidWoodlouseStockSite => "请在画面前方的地面投放鼠妇。",
            CoreError::OccupiedWoodlouseStockSite => "这个地表格被石木或树皮支脚阻挡，请选择可通行的地面。",
            CoreError::ForestWoodlouseLimit => "此世界最多可同时容纳三十二只鼠妇。",
            CoreError::WoodlouseMissing => "选中的鼠妇已不在当前世界。",
            CoreError::InvalidForestScenery => "请选择可摆放的布景素材，并在允许的位置范围内摆放。",
            CoreError::ForestSceneryLimit => "这个世界的布景数量已达上限。",
            CoreError::ForestSceneryMissing => "所选布景已移除。",
            CoreError::ForestSceneryParentMissing => "所选附着物的承载物已经移除。",
            CoreError::PondBasinMissing => "该水体已不存在。请选择当前水体。",
            CoreError::InvalidPondEdit => "数值超出允许范围。水面面积为 400–10,000 cm²，碎屑最多 50,000 mg C，补排水量必须大于零。",
            CoreError::InvalidPondLeafDepositSite => "请将落叶放在画面中的池底范围内。",
            CoreError::PondLeafDepositLimit => "此水域最多同时显示十六处落叶投放点。",
            CoreError::PondLeafDepositMissing => "这处落叶投放点已消失。",
            CoreError::PondCapacityExceeded => "当前水量超过了这个面积的容量。请加大面积或先排水。",
            CoreError::PondShape(issue) => return Self::PondShape { issue },
            CoreError::PondInsufficientWater => "排水量大于当前水量。可选择填入全部水量。",
            CoreError::PondTooShallow => "请保留至少 0.05 L，或排空水体。",
            CoreError::InvalidPondStoneSite => "请把石头放在池底布置范围内，为两侧留出通道。",
            CoreError::OverlappingPondStoneSite => "这个位置离另一块底石太近，请留出可游过的间隔。",
            CoreError::OccupiedPondStoneSite => "虾正在这个落点附近，请换一个位置。",
            CoreError::PondStoneLimit => "此水体最多可放四块底石。",
            CoreError::PondStoneMissing => "所选底石已经移除。",
            CoreError::InvalidPondWoodSite => "请把沉木放在池底布置范围内，为两侧留出通道。",
            CoreError::OverlappingPondWoodSite => "沉木与另一件石木太近，请给虾留出间隔。",
            CoreError::OccupiedPondWoodSite => "虾正在沉木的落点附近，请换一个位置。",
            CoreError::PondWoodLimit => "此水体最多可放两件沉木。",
            CoreError::PondWoodMissing => "所选沉木已经移除。",
            CoreError::PondAttachedShrimpBlocked => "此位置无法容纳木面上的虾，请换一个位置。",
            CoreError::InvalidPondMossSite => "请选择水下的主石或沉木表面，卵石的表面太小。",
            CoreError::PondMossParentMissing => "承载物已移除，请重新选择。",
            CoreError::PondMossSiteOccupied => "每件承载物只能附着一处活体斑块。",
            CoreError::PondMossMissing => "所选附着斑块已经移除。",
            CoreError::PondWaterPlantNeedsWater => "请先补水，再种植水草。",
            CoreError::InvalidPondWaterPlantSite => "请把水草种在画面中的池底，留出边缘空间。",
            CoreError::PondWaterPlantSiteOccupied => "请避开石木的实际接地处，并在水草根部之间留出间距。",
            CoreError::PondWaterPlantLimit => "此水域最多可种八丛活体水草。",
            CoreError::PondWaterPlantMissing => "所选水草已移除。",
            CoreError::InvalidPondWaterPlantTrim => "修剪的碳量必须大于零且小于该丛水草的当前碳库存。",
            CoreError::PondFloatingPlantNeedsWater => "请先补水，再放置活体浮叶。",
            CoreError::InvalidPondFloatingPlantSite => "请把浮叶中心放在当前水域的格子内。",
            CoreError::PondFloatingPlantLimit => "此水域最多可放八组活体浮叶。",
            CoreError::PondFloatingPlantMissing => "所选浮叶已移除。",
            CoreError::InvalidPondSedgeSite => "请选相邻的水域和空岸格。",
            CoreError::PondSedgeLimit => "此水域最多可放八丛岸边莎草。",
            CoreError::PondSedgeMissing => "所选岸边莎草已移除。",
            CoreError::PondStaticSectionMissing => "所选观景剖面已变更，请重新选择。",
            CoreError::InvalidPondStaticSectionEdit => "请回到主剖面调整整片水域。当前剖面可编辑本地物件和生物。",
            CoreError::InvalidPondMossTrim => "修剪量必须大于零，且小于当前碳库存。整株转移请使用移除。",
            CoreError::PondMossLimit => "当前水体的附着斑块已达上限。",
            CoreError::InvalidPondScenery => "请选择可用的布景素材，并在允许的位置范围内摆放。",
            CoreError::PondSceneryMissing => "所选布景已经移除。",
            CoreError::PondSceneryLimit => "当前水体的布景数量已达上限。",
            CoreError::PondStockNeedsWater => "请先补水，再放入生物。",
            CoreError::InvalidPondStockCount => "投放数量至少为 1 只。",
            CoreError::InvalidPondShrimpSite => "请选择有游动空间的水下位置，避开石木和岸边。",
            CoreError::PondShrimpLimit => "当前水域最多容纳 12 只小虾。",
            CoreError::PondShrimpMissing => "选中的米虾已不在当前观景剖面。",
            CoreError::InvalidDaphniaRemovalCount => "取出数量应在 1 只与当前整池水蚤数量之间。",
            CoreError::PondDaphniaLimit => "当前水域最多容纳 2048 只水蚤。",
            CoreError::StarterCommunityUnavailable => "示例生物组合只可在空白世界中加入一次。",
            CoreError::PondStarterNeedsWater => "请先向浅塘补水，再加入示例生物组合。",
            _ => return Self::InvalidFixtureInput { details: error.to_string() },
        };
        Self::InvalidFixtureInput { details: details.to_owned() }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredWorld {
    world_id: String,
    edit_revision: u64,
    paused: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    edit_paused: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    solar_config: Option<SolarConfig>,
    #[serde(default, skip_serializing_if = "ObservationHistory::is_empty")]
    observation_history: ObservationHistory,
    state: WorldState,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SolarConfig {
    location: SolarLocation,
    model_version: u16,
}

fn is_false(value: &bool) -> bool { !*value }

impl StoredWorld {
    fn is_paused(&self) -> bool { self.paused || self.edit_paused }
}

#[derive(Clone, Debug, PartialEq)]
struct EditInput {
    expected_revision: String,
    operation: EditOperation,
}

#[derive(Clone, Debug, PartialEq)]
enum EditOperation {
    Rock { x_bits: u64, y_bits: u64 },
    ForestRock(ForestRockCommand),
    Pond(PondEdit),
    PondHistory { redo: bool },
    StarterCommunity,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "action", rename_all = "camelCase", deny_unknown_fields)]
pub enum ForestRockCommand {
    Change { edit: ForestRockEdit },
    Undo,
    Redo,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForestRockSnapshot {
    pub rocks: Vec<ForestRock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logs: Option<Vec<ForestLog>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stonework: Option<Vec<ForestStonework>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bark_shelters: Option<Vec<ForestBarkShelterSnapshot>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leaf_patches: Option<Vec<ForestLeafPatchSnapshot>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leaf_presets: Option<[ForestLeafPresetSnapshot; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moss_patches: Option<Vec<ForestMossPatchObservation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moss_preset: Option<ForestMossPresetObservation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fern_patches: Option<Vec<ForestFernPatchObservation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fern_preset: Option<ForestFernPresetObservation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub substrate_cells: Option<Vec<ForestSubstrateCellObservation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub substrate_presets: Option<[ForestSubstratePresetObservation; 3]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenery: Option<Vec<ForestScenery>>,
    pub undo_available: bool,
    pub redo_available: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForestBarkShelterSnapshot {
    #[serde(flatten)]
    pub shelter: ForestBarkShelter,
    pub foot_cells: [usize; 2],
    pub passage_cells: [usize; 3],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bridge_clearance_mm: Option<[u16; 2]>,
    pub occupant_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForestLeafPatchSnapshot {
    #[serde(flatten)]
    pub patch: ForestLeafPatch,
    pub initial_mg_cnp: [f64; 3],
    pub cell_index: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForestLeafPresetSnapshot {
    pub preset: ForestLeafPreset,
    pub initial_mg_cnp: [f64; 3],
}

impl EditOperation {
    fn rock(x: f64, y: f64) -> Self { Self::Rock { x_bits: x.to_bits(), y_bits: y.to_bits() } }
}

struct EditReplay {
    input: EditInput,
    result: Result<Snapshot, RuntimeError>,
}

struct RuntimeWorld {
    saved: StoredWorld,
    generation: String,
    edit_replays: BTreeMap<String, EditReplay>,
    pond_undo: Vec<WorldState>,
    pond_redo: Vec<WorldState>,
    pond_contact_feeding: PondContactFeeding,
    forest_undo: Vec<ForestLayoutEditState>,
    forest_redo: Vec<ForestLayoutEditState>,
}

impl RuntimeWorld {
    fn new(saved: StoredWorld) -> Self {
        Self {
            saved,
            generation: Uuid::new_v4().to_string(),
            edit_replays: BTreeMap::new(),
            pond_undo: Vec::new(),
            pond_redo: Vec::new(),
            pond_contact_feeding: Vec::new(),
            forest_undo: Vec::new(),
            forest_redo: Vec::new(),
        }
    }

    fn snapshot(&self) -> Snapshot {
        let state = &self.saved.state;
        let simulated_millis = state.simulated_millis();
        let model_version = state.model_version();
        let pond = state.pond_observation().map(|mut pond| {
            for (id, observations) in &self.pond_contact_feeding {
                if let Some(basin) = pond.basins.iter_mut().find(|basin| basin.id == *id) {
                    if !observations.wood.is_empty() {
                        basin.wood_contact_feeding = Some(observations.wood.clone());
                    }
                    if !observations.stone.is_empty() {
                        basin.stone_contact_feeding = Some(observations.stone.clone());
                    }
                    for (row, local) in &observations.sections {
                        if let Some(section) = basin.static_sections.iter_mut().find(|section| section.row == *row) {
                            section.wood_contact_feeding = (!local.wood.is_empty()).then(|| local.wood.clone());
                            section.stone_contact_feeding = (!local.stone.is_empty()).then(|| local.stone.clone());
                        }
                    }
                }
            }
            pond
        });
        Snapshot {
            protocol_version: SNAPSHOT_PROTOCOL_VERSION,
            world_id: self.saved.world_id.clone(),
            run_generation: self.generation.clone(),
            edit_revision: self.saved.edit_revision.to_string(),
            simulation_tick: state.tick().to_string(),
            simulated_seconds: simulated_millis as f64 / 1000.0,
            simulated_millis,
            paused: self.saved.is_paused(),
            manual_paused: self.saved.paused,
            edit_paused: self.saved.edit_paused,
            daylight_phase: state.daylight_phase(),
            model_version,
            definition: state.definition().to_owned(),
            pond_undo_available: !self.pond_undo.is_empty(),
            pond_redo_available: !self.pond_redo.is_empty(),
            starter_community_available: state.starter_community_available(),
            rock: state.rock(),
            forest_rocks: state.forest_rock_layout().map(|layout| ForestRockSnapshot {
                rocks: layout.rocks().to_vec(),
                logs: layout.logs().map(<[ForestLog]>::to_vec),
                stonework: layout.stonework().map(<[ForestStonework]>::to_vec),
                bark_shelters: layout.bark_shelters().map(|barks| barks.iter().map(|bark| ForestBarkShelterSnapshot {
                    shelter: *bark,
                    foot_cells: bark.foot_cells(),
                    passage_cells: bark.passage_cells(),
                    bridge_clearance_mm: bark.preset.bridge_clearance_mm(),
                    occupant_ids: state.animals().iter().filter(|animal| bark.contains(Point { x: animal.x, y: animal.y }))
                        .map(|animal| animal.id.clone()).collect(),
                }).collect()),
                leaf_patches: layout.leaf_patches().map(|patches| patches.iter().map(|patch| ForestLeafPatchSnapshot {
                    patch: *patch,
                    initial_mg_cnp: patch.initial_mg_cnp(),
                    cell_index: patch.cell_index(),
                }).collect()),
                leaf_presets: layout.leaf_patches().map(|_| [ForestLeafPreset::Leaves, ForestLeafPreset::Heap].map(|preset| ForestLeafPresetSnapshot {
                    preset,
                    initial_mg_cnp: preset.initial_mg_cnp(),
                })),
                moss_patches: state.forest_moss_patches(),
                moss_preset: state.forest_moss_preset(),
                fern_patches: state.forest_fern_patches(),
                fern_preset: state.forest_fern_preset(),
                substrate_cells: state.forest_substrate_cells(),
                substrate_presets: state.forest_substrate_presets(),
                scenery: layout.scenery().map(<[ForestScenery]>::to_vec),
                undo_available: !self.forest_undo.is_empty(),
                redo_available: !self.forest_redo.is_empty(),
            }),
            animals: state.animals().to_vec(),
            moisture: state.moisture(),
            forest: state.forest_observation(),
            pond,
            season: matches!(model_version, POND_MODEL_VERSION | POND_R5_STONES_MODEL_VERSION | POND_R5_WOOD_MODEL_VERSION | POND_R5_MOSS_MODEL_VERSION | POND_R5_COMPOSED_MODEL_VERSION
                | FOREST_SEASONAL_MODEL_VERSION | FOREST_R5_MODEL_VERSION
                | FOREST_R5_TERRAIN_MODEL_VERSION | FOREST_R5_STONEWORK_MODEL_VERSION | FOREST_R5_BARK_MODEL_VERSION
                | FOREST_R5_LEAF_MODEL_VERSION | FOREST_R5_COMPOSED_MODEL_VERSION)
                .then(|| state.season_observation()),
            location: self.saved.solar_config.as_ref().map(|config| config.location.clone()),
        }
    }

    fn require_generation(&self, expected: &str) -> Result<(), RuntimeError> {
        if self.generation != expected {
            return Err(RuntimeError::StaleGeneration { current_generation: self.generation.clone() });
        }
        Ok(())
    }

    fn edit(&mut self, expected_revision: &str, operation: &EditOperation) -> Result<Snapshot, RuntimeError> {
        let current_revision = self.saved.edit_revision.to_string();
        if current_revision != expected_revision {
            return Err(RuntimeError::StaleRevision { current_revision });
        }
        if !self.saved.is_paused() {
            return Err(RuntimeError::EditRequiresPause);
        }
        let next_revision = self.saved.edit_revision.checked_add(1).ok_or(RuntimeError::RevisionOverflow)?;
        match operation {
            EditOperation::Rock { x_bits, y_bits } => self.saved.state.edit_rock(Point {
                x: f64::from_bits(*x_bits), y: f64::from_bits(*y_bits),
            })?,
            EditOperation::ForestRock(command) => self.edit_forest_rock(command)?,
            EditOperation::Pond(edit) => {
                let before = self.saved.state.clone();
                self.saved.state.edit_pond(edit)?;
                self.pond_undo.push(before);
                self.pond_redo.clear();
            }
            EditOperation::PondHistory { redo } => {
                let (from, to) = if *redo { (&mut self.pond_redo, &mut self.pond_undo) }
                    else { (&mut self.pond_undo, &mut self.pond_redo) };
                let previous = from.last().ok_or(RuntimeError::PondHistoryEmpty)?.clone();
                let current = self.saved.state.clone();
                self.saved.state.restore_pond_edit_state(previous)?;
                from.pop();
                to.push(current);
            }
            EditOperation::StarterCommunity => {
                if !self.saved.edit_paused { return Err(RuntimeError::EditRequiresPause); }
                let before = self.saved.state.pond_observation().is_some()
                    .then(|| self.saved.state.clone());
                self.saved.state.add_starter_community()?;
                if let Some(before) = before {
                    self.pond_undo.push(before);
                    self.pond_redo.clear();
                } else if self.saved.state.model_version() == FOREST_R5_COMPOSED_MODEL_VERSION {
                    // The whole starter community changes soil as well as animals.
                    self.forest_undo.clear();
                    self.forest_redo.clear();
                }
            }
        }
        self.saved.edit_revision = next_revision;
        self.pond_contact_feeding.clear();
        Ok(self.snapshot())
    }

    fn edit_forest_rock(&mut self, command: &ForestRockCommand) -> Result<(), RuntimeError> {
        let current = self.saved.state.forest_layout_edit_state().ok_or(CoreError::WrongWorldKind)?;
        match command {
            ForestRockCommand::Change { edit } => {
                self.saved.state.edit_forest_rocks(edit)?;
                self.forest_undo.push(current);
                self.forest_redo.clear();
            }
            ForestRockCommand::Undo | ForestRockCommand::Redo => {
                let (from, to) = if matches!(command, ForestRockCommand::Redo) {
                    (&mut self.forest_redo, &mut self.forest_undo)
                } else {
                    (&mut self.forest_undo, &mut self.forest_redo)
                };
                let previous = from.last().ok_or(RuntimeError::ForestRockHistoryEmpty)?.clone();
                self.saved.state.restore_forest_layout_edit_state(previous)?;
                from.pop();
                to.push(current);
            }
        }
        Ok(())
    }

    fn clear_history(&mut self) {
        self.pond_undo.clear();
        self.pond_redo.clear();
        self.forest_undo.clear();
        self.forest_redo.clear();
    }

    fn advance(&mut self, ticks: u32) -> Result<(), RuntimeError> {
        if ticks == 0 { return Ok(()); }
        let end = self.saved.state.tick().checked_add(u64::from(ticks)).ok_or(CoreError::TickOverflow)?;
        if end > u64::MAX / STEP_MILLIS { return Err(CoreError::TickOverflow.into()); }
        if self.saved.observation_history.is_empty() {
            self.saved.observation_history.record(&self.saved.state);
        }
        // Split only on observation boundaries. The model receives the same steps and
        // the foreground contact report still describes the final step of the request.
        while self.saved.state.tick() < end {
            let tick = self.saved.state.tick();
            let to_boundary = RECENT_INTERVAL_TICKS - tick % RECENT_INTERVAL_TICKS;
            let batch = (end - tick).min(to_boundary);
            if tick + batch == end {
                self.pond_contact_feeding = self.saved.state.advance_steps_with_pond_contact_feeding(batch as u32)?;
            } else {
                self.saved.state.advance_steps(batch as u32)?;
            }
            if batch == to_boundary {
                self.saved.observation_history.record(&self.saved.state);
            }
        }
        self.clear_history();
        Ok(())
    }
}

/// Owns one state per world. The adapter serializes calls and supplies eligible ticks.
#[derive(Default)]
pub struct WorldRuntime {
    worlds: BTreeMap<String, RuntimeWorld>,
    trials: BTreeMap<String, TrialForkBase>,
    suspended: bool,
}

impl WorldRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_fixture(&mut self, world_id: &str, seed: u64) -> Result<Snapshot, RuntimeError> {
        self.create(world_id, WorldState::new_fixture(seed), None)
    }

    pub fn create_forest(&mut self, world_id: &str, seed: u64) -> Result<Snapshot, RuntimeError> {
        self.create_with_solar(world_id, WorldState::new_forest(seed))
    }

    pub fn create_seasonal_forest(&mut self, world_id: &str, seed: u64) -> Result<Snapshot, RuntimeError> {
        self.create_with_solar(world_id, WorldState::new_seasonal_forest(seed))
    }

    pub fn create_composed_forest(&mut self, world_id: &str, seed: u64) -> Result<Snapshot, RuntimeError> {
        self.create_with_solar(world_id, WorldState::new_r5_composed_forest(seed))
    }

    pub fn create_pond(&mut self, world_id: &str, seed: u64) -> Result<Snapshot, RuntimeError> {
        self.create_with_solar(world_id, WorldState::new_pond(seed))
    }

    pub fn create_forest_from_preset(&mut self, seed: u64) -> Result<Snapshot, RuntimeError> {
        let id = format!("forest-{}", Uuid::new_v4().simple());
        self.create_composed_forest(&id, seed)
    }

    pub fn create_pond_from_preset(&mut self, seed: u64) -> Result<Snapshot, RuntimeError> {
        let id = format!("pond-{}", Uuid::new_v4().simple());
        self.create_with_solar(&id, WorldState::new_r5_composed_pond(seed))
    }

    fn create_with_solar(&mut self, id: &str, state: WorldState) -> Result<Snapshot, RuntimeError> {
        self.create(id, state, Some(default_solar_config()))
    }

    fn create(&mut self, world_id: &str, state: WorldState, solar_config: Option<SolarConfig>) -> Result<Snapshot, RuntimeError> {
        validate_world_id(world_id)?;
        if self.worlds.contains_key(world_id) {
            return Err(RuntimeError::WorldAlreadyLoaded { world_id: world_id.to_owned() });
        }
        let world = RuntimeWorld::new(StoredWorld {
            world_id: world_id.to_owned(),
            edit_revision: 0,
            paused: false,
            edit_paused: false,
            solar_config,
            observation_history: ObservationHistory::default(),
            state,
        });
        let snapshot = world.snapshot();
        self.worlds.insert(world_id.to_owned(), world);
        Ok(snapshot)
    }

    pub fn snapshot(&self, world_id: &str) -> Result<Snapshot, RuntimeError> {
        Ok(self.world(world_id)?.snapshot())
    }

    pub fn observation_history(&self, world_id: &str, expected_generation: &str) -> Result<ObservationHistory, RuntimeError> {
        let world = self.world(world_id)?;
        world.require_generation(expected_generation)?;
        Ok(world.saved.observation_history.view(&world.saved.state))
    }

    pub fn forest_ledger(&self, world_id: &str) -> Result<Option<ForestLedger>, RuntimeError> {
        Ok(self.world(world_id)?.saved.state.forest_ledger())
    }

    pub fn pond_ledger(&self, world_id: &str) -> Result<Option<PondLedger>, RuntimeError> {
        Ok(self.world(world_id)?.saved.state.pond_ledger())
    }

    pub fn set_paused(&mut self, world_id: &str, paused: bool) -> Result<Snapshot, RuntimeError> {
        self.require_formal(world_id)?;
        let world = self.world_mut(world_id)?;
        world.saved.paused = paused;
        world.pond_contact_feeding.clear();
        Ok(world.snapshot())
    }

    pub fn set_edit_paused(&mut self, world_id: &str, paused: bool) -> Result<Snapshot, RuntimeError> {
        self.require_formal(world_id)?;
        let world = self.world_mut(world_id)?;
        world.saved.edit_paused = paused;
        world.pond_contact_feeding.clear();
        Ok(world.snapshot())
    }

    pub fn set_suspended(&mut self, suspended: bool) {
        self.suspended = suspended;
        if suspended {
            for world in self.worlds.values_mut() {
                world.pond_contact_feeding.clear();
            }
        }
    }

    pub fn advance_ticks(&mut self, world_id: &str, ticks: u32) -> Result<Snapshot, RuntimeError> {
        self.require_formal(world_id)?;
        let suspended = self.suspended;
        let world = self.world_mut(world_id)?;
        if !suspended && !world.saved.is_paused() {
            world.advance(ticks)?;
        }
        Ok(world.snapshot())
    }

    pub fn set_daylight_target(&mut self, world_id: &str, phase: f64) -> Result<(), RuntimeError> {
        self.require_formal(world_id)?;
        self.world_mut(world_id)?.saved.state.set_daylight_target(phase)?;
        Ok(())
    }

    pub fn set_location(&mut self, world_id: &str, location: SolarLocation) -> Result<Snapshot, RuntimeError> {
        self.require_formal(world_id)?;
        if !location.validate() {
            return Err(RuntimeError::InvalidLocation);
        }
        let world = self.world_mut(world_id)?;
        world.saved.solar_config = Some(SolarConfig { location, model_version: SOLAR_MODEL_VERSION });
        Ok(world.snapshot())
    }

    pub fn unload(&mut self, world_id: &str) {
        self.trials.remove(world_id);
        self.worlds.remove(world_id);
    }

    pub fn edit_rock(
        &mut self,
        world_id: &str,
        expected_generation: &str,
        expected_revision: &str,
        x: f64,
        y: f64,
    ) -> Result<Snapshot, RuntimeError> {
        let suspended = self.suspended;
        let world = self.world_mut(world_id)?;
        world.require_generation(expected_generation)?;
        if suspended {
            return Err(RuntimeError::Suspended);
        }
        world.edit(expected_revision, &EditOperation::rock(x, y))
    }

    pub fn edit_rock_command(
        &mut self,
        world_id: &str,
        command_id: &str,
        expected_generation: &str,
        expected_revision: &str,
        x: f64,
        y: f64,
    ) -> Result<Snapshot, RuntimeError> {
        self.edit_command(world_id, command_id, expected_generation, expected_revision, EditOperation::rock(x, y))
    }

    pub fn edit_pond_command(
        &mut self,
        world_id: &str,
        command_id: &str,
        expected_generation: &str,
        expected_revision: &str,
        edit: PondEdit,
    ) -> Result<Snapshot, RuntimeError> {
        self.edit_command(world_id, command_id, expected_generation, expected_revision, EditOperation::Pond(edit))
    }

    pub fn edit_forest_rock_command(
        &mut self, world_id: &str, command_id: &str, expected_generation: &str,
        expected_revision: &str, action: ForestRockCommand,
    ) -> Result<Snapshot, RuntimeError> {
        self.edit_command(world_id, command_id, expected_generation, expected_revision, EditOperation::ForestRock(action))
    }

    pub fn edit_pond_history_command(
        &mut self, world_id: &str, command_id: &str, expected_generation: &str,
        expected_revision: &str, redo: bool,
    ) -> Result<Snapshot, RuntimeError> {
        self.edit_command(world_id, command_id, expected_generation, expected_revision, EditOperation::PondHistory { redo })
    }

    pub fn add_starter_community_command(
        &mut self, world_id: &str, command_id: &str, expected_generation: &str, expected_revision: &str,
    ) -> Result<Snapshot, RuntimeError> {
        self.edit_command(world_id, command_id, expected_generation, expected_revision, EditOperation::StarterCommunity)
    }

    fn edit_command(
        &mut self, world_id: &str, command_id: &str, expected_generation: &str,
        expected_revision: &str, operation: EditOperation,
    ) -> Result<Snapshot, RuntimeError> {
        if self.trials.contains_key(world_id) {
            match &operation {
                EditOperation::ForestRock(_) => {},
                EditOperation::Pond(_) | EditOperation::PondHistory { .. }
                    if matches!(self.world(world_id)?.saved.state.model_version(), POND_R5_STONES_MODEL_VERSION | POND_R5_WOOD_MODEL_VERSION
                        | POND_R5_MOSS_MODEL_VERSION | POND_R5_COMPOSED_MODEL_VERSION) => {},
                _ => return Err(RuntimeError::TrialOperationInvalid),
            }
        }
        if command_id.is_empty() || command_id.len() > 128 {
            return Err(RuntimeError::InvalidCommandId);
        }
        let suspended = self.suspended;
        let world = self.world_mut(world_id)?;
        world.require_generation(expected_generation)?;
        let input = EditInput {
            expected_revision: expected_revision.to_owned(),
            operation,
        };
        if let Some(replay) = world.edit_replays.get(command_id) {
            return if replay.input == input {
                replay.result.clone()
            } else {
                Err(RuntimeError::CommandReuse)
            };
        }
        if suspended {
            return Err(RuntimeError::Suspended);
        }
        let result = world.edit(expected_revision, &input.operation);
        world.edit_replays.insert(command_id.to_owned(), EditReplay { input, result: result.clone() });
        result
    }

    pub fn save(&self, store: &SaveStore, world_id: &str) -> Result<SaveReceipt, RuntimeError> {
        self.write_checkpoint(store, world_id, false)
    }

    pub fn autosave(&self, store: &SaveStore, world_id: &str) -> Result<SaveReceipt, RuntimeError> {
        if self.suspended { return Err(RuntimeError::Suspended); }
        self.write_checkpoint(store, world_id, true)
    }

    pub fn checkpoint_stamp(&self, world_id: &str) -> Result<CheckpointStamp, RuntimeError> {
        self.require_formal(world_id)?;
        let world = self.world(world_id)?;
        Ok(CheckpointStamp {
            generation: world.generation.clone(),
            tick: world.saved.state.tick(),
            revision: world.saved.edit_revision,
            paused: world.saved.paused,
            edit_paused: world.saved.edit_paused,
            location: world.saved.solar_config.as_ref().map(|solar| solar.location.clone()),
        })
    }

    fn write_checkpoint(&self, store: &SaveStore, world_id: &str, automatic: bool) -> Result<SaveReceipt, RuntimeError> {
        if self.trials.contains_key(world_id) { return Err(RuntimeError::TrialMissing); }
        let world = self.world(world_id)?;
        let serial = if automatic { store.autosave(&world.saved)? } else { store.save(&world.saved)? };
        Ok(SaveReceipt {
            world_id: world_id.to_owned(),
            run_generation: world.generation.clone(),
            simulation_tick: world.saved.state.tick().to_string(),
            checkpoint_serial: serial.to_string(),
        })
    }

    pub fn load(&mut self, store: &SaveStore, world_id: &str) -> Result<LoadReceipt, RuntimeError> {
        self.require_formal(world_id)?;
        let mut checkpoint = store.load(world_id)?;
        if let Some(live) = self.worlds.get(world_id) {
            checkpoint.world.state.retain_pond_topology_allocator(&live.saved.state);
        }
        let world = RuntimeWorld::new(checkpoint.world);
        let snapshot = world.snapshot();
        self.worlds.insert(world_id.to_owned(), world);
        Ok(LoadReceipt {
            snapshot,
            recovered_previous: checkpoint.recovered_previous,
            checkpoint_serial: checkpoint.serial.to_string(),
            automatic: checkpoint.automatic,
        })
    }

    fn world(&self, world_id: &str) -> Result<&RuntimeWorld, RuntimeError> {
        self.worlds.get(world_id).ok_or_else(|| RuntimeError::WorldMissing { world_id: world_id.to_owned() })
    }

    fn world_mut(&mut self, world_id: &str) -> Result<&mut RuntimeWorld, RuntimeError> {
        self.worlds.get_mut(world_id).ok_or_else(|| RuntimeError::WorldMissing { world_id: world_id.to_owned() })
    }
}

fn validate_world_id(world_id: &str) -> Result<(), RuntimeError> {
    let canonical = !world_id.is_empty() && world_id.len() <= 64 && world_id.bytes().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
    });
    if !canonical {
        return Err(RuntimeError::InvalidWorldId);
    }
    Ok(())
}

#[cfg(test)]
mod tests;

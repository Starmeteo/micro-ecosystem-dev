//! Deterministic ecosystem states and versioned experimental definitions.

mod forest;
mod forest_rocks;
mod forest_scenery;
mod pond;
mod pond_float;
mod pond_sedge;
mod pond_leaf;
mod pond_moss;
mod pond_plant;
mod pond_scenery;
mod pond_shape;
mod pond_stones;
mod pond_wood;
pub mod season;

pub use forest::{ForestFernPatchObservation, ForestFernPresetObservation, ForestGroundPlantObservation, ForestLedger,
    ForestMossPatchObservation, ForestMossPresetObservation, ForestObservation, ForestSubstrate, ForestSubstrateCellObservation,
    ForestSubstratePresetObservation};
pub use forest_rocks::{
    ForestBarkPreset, ForestBarkShelter, ForestLeafPatch, ForestLeafPreset, ForestLog,
    ForestRock, ForestRockEdit, ForestRockLayout, ForestStonework, ForestStoneworkKind,
};
pub use forest_scenery::{ForestScenery, ForestSceneryParent};
pub use pond::{PondBasinContactFeeding, PondBasinLedger, PondBasinObservation, PondBiofilmDistribution, PondBiofilmSurface, PondBiofilmSurfaceKind, PondStoneBiofilmObservation, PondWoodBiofilmObservation,
    PondContactFeeding, PondEdit, PondLedger, PondObservation, PondStaticSectionObservation,
    PondShrimpLifeObservation, PondShrimpStage, PondShrimpStoneFeedingObservation, PondShrimpWoodFeedingObservation, POND_DEFAULT_BASIN_ID};
pub use pond_leaf::{PondLeafDepositObservation, PondLeafDepositPreset, PondLeafDepositStage};
pub use pond_moss::{PondMossParent, PondMossPatchObservation, PondMossPreset, PondMossPresetObservation};
pub use pond_plant::{PondRibbonPlantPreset, PondWaterPlantObservation};
pub use pond_float::{PondFloatingCanopy, PondFloatingPlantObservation, PondFloatingPlantPreset};
pub use pond_sedge::{PondSedgeObservation, PondSedgePreset, PondShoreAnchor};
pub use pond_scenery::{PondScenery, PondSceneryParent};
pub use pond_shape::{PondShapeError, PondShapeMode, PondShapeObservation};
pub use pond_stones::{PondShrimpStoneAttachment, PondStone, PondStonePreset};
pub use pond_wood::{PondShrimpAttachment, PondWood, PondWoodPreset};
pub use season::{Season, SeasonObservation};

use serde::{Deserialize, Serialize};
use std::f64::consts::TAU;
use std::fmt;

pub const MODEL_VERSION: u32 = 1;
pub const FIXTURE_DEFINITION: &str = "forest-fixture/1";
pub const FOREST_MODEL_VERSION: u32 = 2;
pub const FOREST_DEFINITION: &str = "forest-understory/1";
pub const POND_MODEL_VERSION: u32 = 3;
pub const POND_DEFINITION: &str = "pond-shallow/1";
pub const FOREST_SEASONAL_MODEL_VERSION: u32 = 4;
pub const FOREST_SEASONAL_DEFINITION: &str = "forest-understory-seasonal/1";
pub const FOREST_R5_MODEL_VERSION: u32 = 5;
pub const FOREST_R5_DEFINITION: &str = "forest-understory-r5/1";
pub const FOREST_R5_TERRAIN_MODEL_VERSION: u32 = 6;
pub const FOREST_R5_TERRAIN_DEFINITION: &str = "forest-understory-r5-terrain/1";
pub const FOREST_R5_STONEWORK_MODEL_VERSION: u32 = 7;
pub const FOREST_R5_STONEWORK_DEFINITION: &str = "forest-understory-r5-stonework/1";
pub const FOREST_R5_BARK_MODEL_VERSION: u32 = 8;
pub const FOREST_R5_BARK_DEFINITION: &str = "forest-understory-r5-bark/1";
pub const FOREST_R5_LEAF_MODEL_VERSION: u32 = 9;
pub const FOREST_R5_LEAF_DEFINITION: &str = "forest-understory-r5-leaves/1";
pub const POND_R5_STONES_MODEL_VERSION: u32 = 10;
pub const POND_R5_STONES_DEFINITION: &str = "pond-shallow-r5-stones/1";
pub const POND_R5_WOOD_MODEL_VERSION: u32 = 11;
pub const POND_R5_WOOD_DEFINITION: &str = "pond-shallow-r5-wood/1";
pub const POND_R5_MOSS_MODEL_VERSION: u32 = 12;
pub const POND_R5_MOSS_DEFINITION: &str = "pond-shallow-r5-moss/1";
pub const POND_R5_COMPOSED_MODEL_VERSION: u32 = 13;
pub const POND_R5_COMPOSED_DEFINITION: &str = "pond-shallow-r5-composed/1";
pub const FOREST_R5_COMPOSED_MODEL_VERSION: u32 = 14;
pub const FOREST_R5_COMPOSED_DEFINITION: &str = "forest-r5-composed/1";
pub const STEP_MILLIS: u64 = 100;
const TICKS_PER_DAY: f64 = 864_000.0;
const DAYLIGHT_CORRECTION_PER_TICK: f64 = 0.0015;
const ANIMAL_COUNT: usize = 4;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn is_normalized(self) -> bool {
        (0.0..=1.0).contains(&self.x) && (0.0..=1.0).contains(&self.y)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Activity {
    Moving,
    Resting,
    Sheltering,
    Foraging,
    Feeding,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Animal {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub heading: f64,
    pub activity: Activity,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum RandomAlgorithm {
    Xorshift64V1,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct RandomState {
    algorithm: RandomAlgorithm,
    state: u64,
}

impl RandomState {
    fn new(seed: u64) -> Self {
        Self {
            algorithm: RandomAlgorithm::Xorshift64V1,
            state: if seed == 0 { 0x9e37_79b9_7f4a_7c15 } else { seed },
        }
    }

    fn unit(&mut self) -> f64 {
        let mut state = self.state;
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        self.state = state;
        ((state >> 11) as f64) * (1.0 / ((1_u64 << 53) as f64))
    }
}

fn fixture_animals(rng: &mut RandomState) -> Vec<Animal> {
    (0..ANIMAL_COUNT)
        .map(|index| Animal {
            id: format!("isopod-{}", index + 1),
            x: 0.18 + 0.17 * index as f64,
            y: 0.73 + 0.08 * rng.unit(),
            heading: rng.unit() * TAU,
            activity: Activity::Resting,
        })
        .collect()
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Daylight {
    phase: f64,
    target: Option<f64>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
enum InitialHabitat {
    Blank,
    BlankStocked,
}

impl Daylight {
    fn advance(&mut self) {
        let daytime_step = 1.0 / TICKS_PER_DAY;
        self.phase = (self.phase + daytime_step).rem_euclid(1.0);
        if let Some(target) = &mut self.target {
            *target = (*target + daytime_step).rem_euclid(1.0);
            let difference = (*target - self.phase + 0.5).rem_euclid(1.0) - 0.5;
            let correction = difference.clamp(-DAYLIGHT_CORRECTION_PER_TICK, DAYLIGHT_CORRECTION_PER_TICK);
            self.phase = (self.phase + correction).rem_euclid(1.0);
        }
    }

    fn is_valid(self) -> bool {
        (0.0..1.0).contains(&self.phase)
            && self.target.map_or(true, |target| (0.0..1.0).contains(&target))
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorldState {
    model_version: u32,
    fixture_definition: String,
    tick: u64,
    daylight: Daylight,
    rock: Point,
    moisture: [f64; 2],
    animals: Vec<Animal>,
    rng: RandomState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    forest: Option<forest::ForestState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pond: Option<pond::PondState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    initial_habitat: Option<InitialHabitat>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    forest_rocks: Option<ForestRockLayout>,
}

/// State of the terrain edits at a paused simulation tick, including their material flows.
#[derive(Clone, Debug, PartialEq)]
pub struct ForestLayoutEditState {
    tick: u64,
    layout: ForestRockLayout,
    flows: forest::ForestMatterFlows,
    fauna: Option<forest::ForestFaunaEditState>,
    ground: Option<forest::ForestGroundEditState>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoreError {
    InvalidFixtureState,
    InvalidForestState,
    InvalidPondState,
    InvalidPoint,
    InvalidRockSite,
    OccupiedRockSite,
    OverlappingRockSite,
    ForestRockLimit,
    ForestRockMissing,
    InvalidLogSite,
    OccupiedLogSite,
    OverlappingLogSite,
    ForestLogLimit,
    InvalidStoneworkSite,
    OccupiedStoneworkSite,
    OverlappingStoneworkSite,
    ForestStoneworkLimit,
    ForestNoOpenGround,
    InvalidBarkSite,
    OccupiedBarkFeet,
    OverlappingBarkSite,
    BlockedBarkPassage,
    ForestBarkLimit,
    InvalidLeafPatchBatchSize,
    InvalidLeafFragmentDose,
    InvalidLeafPatchSite,
    OverlappingLeafPatchSite,
    ForestLeafPatchLimit,
    InvalidForestMossSite,
    BlockedForestMossSite,
    ForestMossPatchLimit,
    ForestMossPatchMissing,
    InvalidForestMossTrim,
    InvalidForestFernSite,
    BlockedForestFernSite,
    ForestFernPatchLimit,
    ForestFernPatchMissing,
    InvalidForestFernTrim,
    InvalidForestSubstrateCells,
    ForestSubstrateTooWet,
    ForestSubstrateUnchanged,
    InvalidWoodlouseStockCount,
    InvalidWoodlouseStockSite,
    OccupiedWoodlouseStockSite,
    ForestWoodlouseLimit,
    WoodlouseMissing,
    InvalidForestScenery,
    ForestSceneryLimit,
    ForestSceneryMissing,
    ForestSceneryParentMissing,
    WrongWorldKind,
    PondBasinMissing,
    InvalidPondEdit,
    PondStaticSectionMissing,
    InvalidPondStaticSectionEdit,
    InvalidPondLeafDepositSite,
    PondLeafDepositLimit,
    PondLeafDepositMissing,
    PondCapacityExceeded,
    PondShape(PondShapeError),
    PondInsufficientWater,
    PondTooShallow,
    InvalidPondStoneSite,
    OverlappingPondStoneSite,
    OccupiedPondStoneSite,
    PondStoneLimit,
    PondStoneMissing,
    InvalidPondWoodSite,
    OverlappingPondWoodSite,
    OccupiedPondWoodSite,
    PondWoodLimit,
    PondWoodMissing,
    PondAttachedShrimpBlocked,
    InvalidPondMossSite,
    PondMossParentMissing,
    PondMossSiteOccupied,
    PondMossMissing,
    PondWaterPlantNeedsWater,
    InvalidPondWaterPlantSite,
    PondWaterPlantSiteOccupied,
    PondWaterPlantLimit,
    PondWaterPlantMissing,
    InvalidPondWaterPlantTrim,
    PondFloatingPlantNeedsWater,
    InvalidPondFloatingPlantSite,
    PondFloatingPlantLimit,
    PondFloatingPlantMissing,
    InvalidPondSedgeSite,
    PondSedgeLimit,
    PondSedgeMissing,
    InvalidPondMossTrim,
    PondMossLimit,
    InvalidPondScenery,
    PondSceneryMissing,
    PondSceneryLimit,
    PondStockNeedsWater,
    InvalidPondStockCount,
    InvalidPondShrimpSite,
    PondShrimpLimit,
    PondShrimpMissing,
    InvalidDaphniaRemovalCount,
    PondDaphniaLimit,
    StarterCommunityUnavailable,
    PondStarterNeedsWater,
    InvalidPhase,
    TickOverflow,
    UnsupportedModel,
}

impl fmt::Display for CoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidFixtureState => "fixture state has invalid values",
            Self::InvalidForestState => "forest inventory, animal state or its resource ledger is invalid",
            Self::InvalidPondState => "pond inventory, animal state or its resource ledger is invalid",
            Self::InvalidPoint => "coordinates must be normalized to [0, 1]",
            Self::InvalidRockSite => "place the rock on the visible ground, clear of the scene edges",
            Self::OccupiedRockSite => "an animal is occupying the rock's new ground cell",
            Self::OverlappingRockSite => "a stone already occupies this ground cell",
            Self::ForestRockLimit => "the world already contains its maximum of eight stones",
            Self::ForestRockMissing => "the selected structure no longer exists",
            Self::InvalidLogSite => "place the full log on the visible ground, clear of the scene edges",
            Self::OccupiedLogSite => "an animal is occupying the log's ground footprint",
            Self::OverlappingLogSite => "a structure already occupies part of the log's ground footprint",
            Self::ForestLogLimit => "the world already contains its maximum of four logs",
            Self::InvalidStoneworkSite => "place the complete stonework preset on visible ground, clear of the scene edges",
            Self::OccupiedStoneworkSite => "an animal is occupying a stonework contact cell",
            Self::OverlappingStoneworkSite => "a structure already occupies a stonework contact cell",
            Self::ForestStoneworkLimit => "the world already contains its maximum of eight stonework objects",
            Self::ForestNoOpenGround => "keep at least one ground cell open",
            Self::InvalidBarkSite => "place the bark shelter where both entrances and feet fit on the visible ground",
            Self::OccupiedBarkFeet => "an animal is occupying a bark shelter foot cell",
            Self::OverlappingBarkSite => "a bark shelter foot conflicts with another structure or passage",
            Self::BlockedBarkPassage => "keep the covered passage and both entrances clear of structures",
            Self::ForestBarkLimit => "the world already contains its maximum of six bark shelters",
            Self::InvalidLeafPatchBatchSize => "a leaf brush stroke needs at least one ground cell",
            Self::InvalidLeafFragmentDose => "a fragment stroke needs a total dose of 0.1 to 240 mg C",
            Self::InvalidForestMossSite => "place the moss patch on visible ground",
            Self::BlockedForestMossSite => "the ground cell is already occupied by a solid or another moss patch",
            Self::ForestMossPatchLimit => "the world already contains its maximum of forty-eight ground moss patches",
            Self::ForestMossPatchMissing => "the selected ground moss patch no longer exists",
            Self::InvalidForestMossTrim => "trim less than the current patch carbon or remove the whole patch",
            Self::InvalidForestFernSite => "place the fern patch on visible ground",
            Self::BlockedForestFernSite => "the fern needs a ground cell clear of solids, bark passages, and other ferns",
            Self::ForestFernPatchLimit => "the world already contains its maximum of forty-eight fern patches",
            Self::ForestFernPatchMissing => "the selected fern patch no longer exists",
            Self::InvalidForestFernTrim => "trim less than the current fern carbon or remove the whole fern",
            Self::InvalidForestSubstrateCells => "a substrate stroke must contain one to forty-eight distinct ground cells",
            Self::ForestSubstrateTooWet => "a selected ground cell contains more water than the proposed substrate can hold",
            Self::ForestSubstrateUnchanged => "all selected cells already have the proposed substrate",
            Self::InvalidLeafPatchSite => "place the leaves on visible ground clear of the scene edges",
            Self::OverlappingLeafPatchSite => "keep leaf patches clear of other patches, solid ground contacts, and bark passages",
            Self::ForestLeafPatchLimit => "the world already contains its maximum of sixteen leaf patches",
            Self::InvalidWoodlouseStockCount => "introduce at least one woodlouse",
            Self::InvalidWoodlouseStockSite => "introduce woodlice on the visible ground",
            Self::OccupiedWoodlouseStockSite => "introduce woodlice in a walkable ground cell",
            Self::ForestWoodlouseLimit => "the woodlouse population cannot exceed thirty-two animals",
            Self::WoodlouseMissing => "the selected woodlouse is no longer present",
            Self::InvalidForestScenery => "choose an available scenery style and a valid anchor position",
            Self::ForestSceneryLimit => "the world already contains its maximum number of scenery items",
            Self::ForestSceneryMissing => "the selected scenery item no longer exists",
            Self::ForestSceneryParentMissing => "the selected scenery carrier no longer exists",
            Self::WrongWorldKind => "this command does not apply to the world's theme",
            Self::PondBasinMissing => "the requested basin does not exist",
            Self::InvalidPondEdit => "pond edit has an invalid quantity or surface area",
            Self::PondStaticSectionMissing => "the selected secondary pond section no longer exists",
            Self::InvalidPondStaticSectionEdit => "the edit must apply to the selected pond section or use the shared basin controls",
            Self::InvalidPondLeafDepositSite => "place the deposited leaves within the visible basin bed",
            Self::PondLeafDepositLimit => "the basin already shows its maximum of sixteen leaf deposits",
            Self::PondLeafDepositMissing => "the selected leaf deposit no longer exists",
            Self::PondCapacityExceeded => "the water volume exceeds the basin capacity",
            Self::PondShape(issue) => issue.code(),
            Self::PondInsufficientWater => "the drain volume exceeds the available water",
            Self::PondTooShallow => "keep at least 0.05 L in the basin or drain it completely",
            Self::InvalidPondStoneSite => "place the stone within the basin bed and leave water along both banks",
            Self::OverlappingPondStoneSite => "leave enough clear water between the two stones",
            Self::OccupiedPondStoneSite => "a shrimp is currently occupying this part of the stone's profile",
            Self::PondStoneLimit => "the basin already contains its maximum of four stones",
            Self::PondStoneMissing => "the selected basin stone no longer exists",
            Self::InvalidPondWoodSite => "place the wood within the basin bed and leave water at both ends",
            Self::OverlappingPondWoodSite => "leave enough clear water between the wood and other structures",
            Self::OccupiedPondWoodSite => "a free shrimp is currently occupying this part of the wood's profile",
            Self::PondWoodLimit => "the basin already contains its maximum of two woods",
            Self::PondWoodMissing => "the selected wood no longer exists",
            Self::PondAttachedShrimpBlocked => "a resting shrimp must have clear water at the new wood surface",
            Self::InvalidPondMossSite => "place a moss patch on a supported submerged surface within the local anchor range",
            Self::PondMossParentMissing => "the selected moss support no longer exists",
            Self::PondMossSiteOccupied => "this support already carries a moss patch",
            Self::PondMossMissing => "the selected moss patch no longer exists",
            Self::PondWaterPlantNeedsWater => "add water before planting submerged water plants",
            Self::InvalidPondWaterPlantSite => "place the water plant root on the visible basin bed, clear of the edges",
            Self::PondWaterPlantSiteOccupied => "keep water plant roots clear of solid ground contacts and nearby roots",
            Self::PondWaterPlantLimit => "the basin already contains its maximum of eight water plant patches",
            Self::PondWaterPlantMissing => "the selected water plant patch no longer exists",
            Self::InvalidPondWaterPlantTrim => "trim a positive carbon amount less than the current stock",
            Self::PondFloatingPlantNeedsWater => "add water before placing a floating plant",
            Self::InvalidPondFloatingPlantSite => "place the floating leaf center inside the basin footprint",
            Self::PondFloatingPlantLimit => "the basin already contains its maximum of eight floating plants",
            Self::PondFloatingPlantMissing => "the selected floating plant no longer exists",
            Self::InvalidPondSedgeSite => "place the sedge at an unused land and water boundary",
            Self::PondSedgeLimit => "the basin already contains its maximum of eight sedge patches",
            Self::PondSedgeMissing => "the selected sedge patch no longer exists",
            Self::InvalidPondMossTrim => "trim a positive amount smaller than the patch's current carbon stock",
            Self::PondMossLimit => "the basin already contains its maximum number of attached patches",
            Self::InvalidPondScenery => "choose a supported scenery style and place it within its anchor range",
            Self::PondSceneryMissing => "the selected scenery item no longer exists",
            Self::PondSceneryLimit => "the basin already contains its maximum number of scenery items",
            Self::PondStockNeedsWater => "fill the basin before adding animals",
            Self::InvalidPondStockCount => "specify at least one animal",
            Self::InvalidPondShrimpSite => "place the shrimp in open, swimmable water",
            Self::PondShrimpLimit => "at most twelve shrimp can occupy the basin",
            Self::PondShrimpMissing => "the selected shrimp is no longer in this section",
            Self::InvalidDaphniaRemovalCount => "remove between one and the current population of water fleas",
            Self::PondDaphniaLimit => "at most 2048 daphnia can occupy the basin",
            Self::StarterCommunityUnavailable => "the example community is only available once per empty setup",
            Self::PondStarterNeedsWater => "fill the empty pond before introducing its example community",
            Self::InvalidPhase => "daylight phase must be in [0, 1)",
            Self::TickOverflow => "simulated time exceeds its supported range",
            Self::UnsupportedModel => "world model or definition version is unsupported",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for CoreError {}

impl WorldState {
    pub fn has_wood_biofilm(&self) -> bool { self.pond.as_ref().is_some_and(pond::PondState::has_wood_biofilm) }

    pub fn new_fixture(seed: u64) -> Self {
        let mut rng = RandomState::new(seed);
        let animals = fixture_animals(&mut rng);

        Self {
            model_version: MODEL_VERSION,
            fixture_definition: FIXTURE_DEFINITION.to_owned(),
            tick: 0,
            daylight: Daylight {
                phase: 0.50,
                target: None,
            },
            rock: Point { x: 0.44, y: 0.68 },
            moisture: [0.61, 0.52],
            animals,
            rng,
            forest: None,
            pond: None,
            initial_habitat: None,
            forest_rocks: None,
        }
    }

    pub fn new_forest(seed: u64) -> Self {
        let mut world = Self::new_fixture(seed);
        world.model_version = FOREST_MODEL_VERSION;
        world.fixture_definition = FOREST_DEFINITION.to_owned();
        world.forest = Some(forest::ForestState::new(world.animals.len()));
        world
    }

    pub fn new_seasonal_forest(seed: u64) -> Self {
        let mut world = Self::new_fixture(seed);
        world.model_version = FOREST_SEASONAL_MODEL_VERSION;
        world.fixture_definition = FOREST_SEASONAL_DEFINITION.to_owned();
        world.forest = Some(forest::ForestState::new_seasonal(world.rock, &mut world.animals));
        world
    }

    pub fn new_r5_forest(seed: u64) -> Self {
        let mut world = Self::new_seasonal_forest(seed);
        world.model_version = FOREST_R5_MODEL_VERSION;
        world.fixture_definition = FOREST_R5_DEFINITION.to_owned();
        world.forest_rocks = Some(ForestRockLayout::new(world.rock));
        world
    }

    pub fn new_r5_terrain_forest(seed: u64) -> Self {
        let mut world = Self::new_seasonal_forest(seed);
        world.model_version = FOREST_R5_TERRAIN_MODEL_VERSION;
        world.fixture_definition = FOREST_R5_TERRAIN_DEFINITION.to_owned();
        world.forest_rocks = Some(ForestRockLayout::with_logs(world.rock));
        world
    }

    pub fn new_r5_stonework_forest(seed: u64) -> Self {
        let mut world = Self::new_seasonal_forest(seed);
        world.model_version = FOREST_R5_STONEWORK_MODEL_VERSION;
        world.fixture_definition = FOREST_R5_STONEWORK_DEFINITION.to_owned();
        world.forest_rocks = Some(ForestRockLayout::with_stonework(world.rock));
        world
    }

    pub fn new_r5_bark_forest(seed: u64) -> Self {
        let mut world = Self::new_seasonal_forest(seed);
        world.model_version = FOREST_R5_BARK_MODEL_VERSION;
        world.fixture_definition = FOREST_R5_BARK_DEFINITION.to_owned();
        world.forest_rocks = Some(ForestRockLayout::with_bark_shelters(world.rock));
        world
    }

    pub fn new_r5_leaf_forest(seed: u64) -> Self {
        let mut world = Self::new_seasonal_forest(seed);
        world.model_version = FOREST_R5_LEAF_MODEL_VERSION;
        world.fixture_definition = FOREST_R5_LEAF_DEFINITION.to_owned();
        world.forest_rocks = Some(ForestRockLayout::with_leaf_patches(world.rock));
        world
    }

    pub fn new_r5_composed_forest(seed: u64) -> Self {
        let mut world = Self::new_composed_forest_state(seed, false);
        let rock = Point { x: 497.0 / 576.0, y: 242.0 / 324.0 };
        // The opening is left of the composite wood sprite's visual pivot.
        let arch = Point { x: 88.0 / 576.0, y: 232.0 / 324.0 };
        let leaf = Point { x: 361.0 / 576.0, y: 254.0 / 324.0 };
        let layout = ForestRockLayout::with_scenery(rock, arch, leaf);
        forest::clear_initial_structure_cells(std::iter::once(rock), &[], &[],
            layout.bark_shelters().unwrap_or_default(), &mut world.animals);
        layout.validate(&world.animals).expect("valid bundled forest composition");
        world.forest.as_mut().expect("composed world has forest stocks")
            .reset_initial_balance(layout.leaf_patches().unwrap_or_default());
        world.rock = rock;
        world.forest_rocks = Some(layout);
        world
    }

    pub fn new_pond(seed: u64) -> Self {
        Self::pond_with_basins(seed, 1, POND_MODEL_VERSION)
    }

    pub fn new_r5_stone_pond(seed: u64) -> Self {
        Self::pond_with_basins(seed, 1, POND_R5_STONES_MODEL_VERSION)
    }

    pub fn new_r5_wood_pond(seed: u64) -> Self {
        Self::pond_with_basins(seed, 1, POND_R5_WOOD_MODEL_VERSION)
    }

    pub fn new_r5_moss_pond(seed: u64) -> Self {
        Self::pond_with_basins(seed, 1, POND_R5_MOSS_MODEL_VERSION)
    }

    pub fn new_r5_composed_pond(seed: u64) -> Self {
        Self::pond_with_basins(seed, 1, POND_R5_COMPOSED_MODEL_VERSION)
    }

    pub fn new_r5_composed_dual_pond(seed: u64) -> Self {
        Self::pond_with_basins(seed, 2, POND_R5_COMPOSED_MODEL_VERSION)
    }

    /// A substrate with a movable stone, without initial organisms or litter.
    pub fn new_blank_forest(seed: u64) -> Self {
        let mut world = Self::new_seasonal_forest(seed);
        world.animals.clear();
        world.forest = Some(forest::ForestState::new_blank(world.rock));
        world.initial_habitat = Some(InitialHabitat::Blank);
        world
    }

    pub fn new_blank_r5_composed_forest(seed: u64) -> Self {
        Self::new_composed_forest_state(seed, true)
    }

    fn new_composed_forest_state(seed: u64, blank: bool) -> Self {
        let mut world = if blank { Self::new_blank_forest(seed) } else { Self::new_seasonal_forest(seed) };
        world.model_version = FOREST_R5_COMPOSED_MODEL_VERSION;
        world.fixture_definition = FOREST_R5_COMPOSED_DEFINITION.to_owned();
        world.forest_rocks = Some(ForestRockLayout::blank_scenery());
        world
    }

    /// An empty, dry basin with no initial chemical or biological stocks.
    pub fn new_blank_pond(seed: u64) -> Self {
        let mut world = Self::new_pond(seed);
        world.pond.as_mut().expect("pond initialization has a basin").clear_initial_basin();
        world.initial_habitat = Some(InitialHabitat::Blank);
        world
    }

    pub fn new_blank_r5_stone_pond(seed: u64) -> Self {
        let mut world = Self::new_r5_stone_pond(seed);
        world.pond.as_mut().expect("pond initialization has a basin").clear_initial_basin();
        world.initial_habitat = Some(InitialHabitat::Blank);
        world
    }

    pub fn new_blank_r5_wood_pond(seed: u64) -> Self {
        let mut world = Self::new_r5_wood_pond(seed);
        world.pond.as_mut().expect("pond initialization has a basin").clear_initial_basin();
        world.initial_habitat = Some(InitialHabitat::Blank);
        world
    }

    pub fn new_blank_r5_moss_pond(seed: u64) -> Self {
        let mut world = Self::new_r5_moss_pond(seed);
        world.pond.as_mut().expect("pond initialization has a basin").clear_initial_basin();
        world.initial_habitat = Some(InitialHabitat::Blank);
        world
    }

    pub fn new_blank_r5_composed_pond(seed: u64) -> Self {
        Self::new_blank_r5_composed_pond_with_basins(seed, 1)
    }

    pub fn new_blank_r5_composed_dual_pond(seed: u64) -> Self {
        Self::new_blank_r5_composed_pond_with_basins(seed, 2)
    }

    fn new_blank_r5_composed_pond_with_basins(seed: u64, basin_count: usize) -> Self {
        let mut world = Self::pond_with_basins(seed, basin_count, POND_R5_COMPOSED_MODEL_VERSION);
        let pond = world.pond.as_mut().expect("pond initialization has a basin");
        pond.clear_initial_scenery();
        pond.clear_initial_basin();
        world.initial_habitat = Some(InitialHabitat::Blank);
        world
    }

    /// Reinitializes a layout with a fresh seed and the definition's starting stocks.
    /// It intentionally does not copy age, simulated time, climate phase, or live inventories.
    pub fn fresh_habitat(&self, seed: u64) -> Result<Self, CoreError> {
        let blank = self.initial_habitat.is_some();
        let mut template = match self.model_version {
            MODEL_VERSION => Self::new_fixture(seed),
            FOREST_MODEL_VERSION => Self::new_forest(seed),
            FOREST_SEASONAL_MODEL_VERSION if blank => Self::new_blank_forest(seed),
            FOREST_SEASONAL_MODEL_VERSION => Self::new_seasonal_forest(seed),
            FOREST_R5_MODEL_VERSION => Self::new_r5_forest(seed),
            FOREST_R5_TERRAIN_MODEL_VERSION => Self::new_r5_terrain_forest(seed),
            FOREST_R5_STONEWORK_MODEL_VERSION => Self::new_r5_stonework_forest(seed),
            FOREST_R5_BARK_MODEL_VERSION => Self::new_r5_bark_forest(seed),
            FOREST_R5_LEAF_MODEL_VERSION => Self::new_r5_leaf_forest(seed),
            FOREST_R5_COMPOSED_MODEL_VERSION => Self::new_composed_forest_state(seed, blank),
            POND_MODEL_VERSION | POND_R5_STONES_MODEL_VERSION | POND_R5_WOOD_MODEL_VERSION
            | POND_R5_MOSS_MODEL_VERSION | POND_R5_COMPOSED_MODEL_VERSION => Self::new_fixture(seed),
            _ => return Err(CoreError::UnsupportedModel),
        };
        if let Some(pond) = &self.pond {
            template.model_version = self.model_version;
            template.fixture_definition.clone_from(&self.fixture_definition);
            template.animals.clear();
            template.moisture = [0.0; 2];
            template.pond = Some(pond.fresh_habitat(seed, blank)?);
        } else if let Some(rocks) = &self.forest_rocks {
            if self.model_version == FOREST_R5_COMPOSED_MODEL_VERSION { template.rock = self.rock; }
            forest::clear_initial_structure_cells(
                forest_rocks::positions(self.rock, Some(rocks)), rocks.logs().unwrap_or_default(),
                rocks.stonework().unwrap_or_default(), rocks.bark_shelters().unwrap_or_default(), &mut template.animals,
            );
            let mut layout = rocks.clone();
            let forest = template.forest.as_mut().ok_or(CoreError::InvalidForestState)?;
            if self.model_version == FOREST_R5_COMPOSED_MODEL_VERSION {
                forest.copy_substrate_profiles(self.forest.as_ref().ok_or(CoreError::InvalidForestState)?);
            }
            let (moss, fern) = layout.ground_plants_mut();
            for (kind, patches) in [(forest::ForestPlantKind::Moss, moss), (forest::ForestPlantKind::Fern, fern)] {
                for patch in patches { patch.share = forest.add_ground_plant(kind, patch.cell_index(), false); }
            }
            if let Some(patches) = layout.leaf_patches_mut() {
                for patch in &mut *patches {
                    patch.remaining_mg_cnp = patch.initial_mg_cnp();
                }
                forest.reset_initial_balance(patches);
            }
            template.forest_rocks = Some(layout);
        } else {
            if template.forest.is_some() {
                forest::clear_initial_rock_cell(self.rock, &mut template.animals);
            }
            template.edit_rock(self.rock)?;
        }
        template.initial_habitat = blank.then_some(InitialHabitat::Blank);
        let reintroduce = self.initial_habitat == Some(InitialHabitat::BlankStocked)
            && template.pond.as_ref().is_none_or(pond::PondState::starter_site_wet);
        if reintroduce {
            template.add_starter_community()?;
            if let Some(pond) = &mut template.pond { pond.reset_initial_balance(); }
            let patches = template.forest_rocks.as_ref().and_then(ForestRockLayout::leaf_patches).unwrap_or_default();
            if let Some(forest) = &mut template.forest { forest.reset_initial_balance(patches); }
        }
        template.validate()?;
        Ok(template)
    }

    pub fn starter_community_available(&self) -> bool {
        self.initial_habitat == Some(InitialHabitat::Blank)
            && self.forest.as_ref().is_none_or(|forest| !forest.has_stocked_woodlice())
            && self.pond.as_ref().is_none_or(|pond| !pond.starter_site_has_animals())
    }

    /// Administratively introduces a theme's example life and starter food. There is no automatic refill.
    pub fn add_starter_community(&mut self) -> Result<(), CoreError> {
        if !self.starter_community_available() { return Err(CoreError::StarterCommunityUnavailable); }
        if let Some(pond) = &mut self.pond {
            pond.introduce_starter()?;
        } else if let Some(forest) = &mut self.forest {
            if !self.animals.is_empty() { return Err(CoreError::StarterCommunityUnavailable); }
            let mut rng = self.rng.clone();
            let mut animals = fixture_animals(&mut rng);
            if let Some(layout) = &self.forest_rocks {
                let rocks = std::iter::once(self.rock).chain(layout.rocks().iter().map(|rock| rock.position));
                forest::clear_initial_structure_cells(rocks, layout.logs().unwrap_or_default(),
                    layout.stonework().unwrap_or_default(), layout.bark_shelters().unwrap_or_default(), &mut animals);
            } else {
                forest::clear_initial_rock_cell(self.rock, &mut animals);
            }
            let plants = self.forest_rocks.as_mut().map(ForestRockLayout::ground_plants_mut).unwrap_or_default();
            forest.introduce_starter(self.rock, &mut animals, plants)?;
            self.animals = animals;
            self.rng = rng;
        } else {
            return Err(CoreError::WrongWorldKind);
        }
        self.initial_habitat = Some(InitialHabitat::BlankStocked);
        Ok(())
    }

    pub fn new_pond_with_basins(seed: u64, basin_count: usize) -> Result<Self, CoreError> {
        if !(1..=pond::MAX_BASINS).contains(&basin_count) {
            return Err(CoreError::InvalidPondEdit);
        }
        Ok(Self::pond_with_basins(seed, basin_count, POND_MODEL_VERSION))
    }

    pub fn new_r5_stone_pond_with_basins(seed: u64, basin_count: usize) -> Result<Self, CoreError> {
        if !(1..=pond::MAX_BASINS).contains(&basin_count) {
            return Err(CoreError::InvalidPondEdit);
        }
        Ok(Self::pond_with_basins(seed, basin_count, POND_R5_STONES_MODEL_VERSION))
    }

    pub fn new_r5_wood_pond_with_basins(seed: u64, basin_count: usize) -> Result<Self, CoreError> {
        if !(1..=pond::MAX_BASINS).contains(&basin_count) {
            return Err(CoreError::InvalidPondEdit);
        }
        Ok(Self::pond_with_basins(seed, basin_count, POND_R5_WOOD_MODEL_VERSION))
    }

    pub fn new_r5_moss_pond_with_basins(seed: u64, basin_count: usize) -> Result<Self, CoreError> {
        if !(1..=pond::MAX_BASINS).contains(&basin_count) {
            return Err(CoreError::InvalidPondEdit);
        }
        Ok(Self::pond_with_basins(seed, basin_count, POND_R5_MOSS_MODEL_VERSION))
    }

    fn pond_with_basins(seed: u64, basin_count: usize, model_version: u32) -> Self {
        let mut world = Self::new_fixture(seed);
        let (definition, pond) = match model_version {
            POND_R5_COMPOSED_MODEL_VERSION => (POND_R5_COMPOSED_DEFINITION, pond::PondState::with_scenery(seed, basin_count)),
            POND_R5_MOSS_MODEL_VERSION => (POND_R5_MOSS_DEFINITION, pond::PondState::with_moss(seed, basin_count)),
            POND_R5_WOOD_MODEL_VERSION => (POND_R5_WOOD_DEFINITION, pond::PondState::with_woods(seed, basin_count)),
            POND_R5_STONES_MODEL_VERSION => (POND_R5_STONES_DEFINITION, pond::PondState::with_stones(seed, basin_count)),
            _ => (POND_DEFINITION, pond::PondState::new(seed, basin_count)),
        };
        world.model_version = model_version;
        world.fixture_definition = definition.to_owned();
        world.animals.clear();
        world.moisture = [0.0; 2];
        world.pond = Some(pond);
        world
    }

    pub fn supports_definition(model_version: u32, definition: &str) -> bool {
        matches!(
            (model_version, definition),
            (MODEL_VERSION, FIXTURE_DEFINITION)
                | (FOREST_MODEL_VERSION, FOREST_DEFINITION)
                | (POND_MODEL_VERSION, POND_DEFINITION)
                | (FOREST_SEASONAL_MODEL_VERSION, FOREST_SEASONAL_DEFINITION)
                | (FOREST_R5_MODEL_VERSION, FOREST_R5_DEFINITION)
                | (FOREST_R5_TERRAIN_MODEL_VERSION, FOREST_R5_TERRAIN_DEFINITION)
                | (FOREST_R5_STONEWORK_MODEL_VERSION, FOREST_R5_STONEWORK_DEFINITION)
                | (FOREST_R5_BARK_MODEL_VERSION, FOREST_R5_BARK_DEFINITION)
                | (FOREST_R5_LEAF_MODEL_VERSION, FOREST_R5_LEAF_DEFINITION)
                | (FOREST_R5_COMPOSED_MODEL_VERSION, FOREST_R5_COMPOSED_DEFINITION)
                | (POND_R5_STONES_MODEL_VERSION, POND_R5_STONES_DEFINITION)
                | (POND_R5_WOOD_MODEL_VERSION, POND_R5_WOOD_DEFINITION)
                | (POND_R5_MOSS_MODEL_VERSION, POND_R5_MOSS_DEFINITION)
                | (POND_R5_COMPOSED_MODEL_VERSION, POND_R5_COMPOSED_DEFINITION)
        )
    }

    pub fn validate(&self) -> Result<(), CoreError> {
        if !Self::supports_definition(self.model_version, &self.fixture_definition) {
            return Err(CoreError::UnsupportedModel);
        }

        if self.tick > u64::MAX / STEP_MILLIS
            || !self.daylight.is_valid()
            || !self.rock.is_normalized()
            || !self.moisture.into_iter().all(|value| (0.0..=1.0).contains(&value))
            || self.rng.state == 0
            || matches!(self.model_version, FOREST_R5_MODEL_VERSION | FOREST_R5_TERRAIN_MODEL_VERSION
                | FOREST_R5_STONEWORK_MODEL_VERSION | FOREST_R5_BARK_MODEL_VERSION | FOREST_R5_LEAF_MODEL_VERSION
                | FOREST_R5_COMPOSED_MODEL_VERSION) != self.forest_rocks.is_some()
            || (self.initial_habitat.is_some() && !matches!(self.model_version,
                POND_MODEL_VERSION | POND_R5_STONES_MODEL_VERSION | POND_R5_WOOD_MODEL_VERSION
                | POND_R5_MOSS_MODEL_VERSION | POND_R5_COMPOSED_MODEL_VERSION | FOREST_SEASONAL_MODEL_VERSION
                | FOREST_R5_COMPOSED_MODEL_VERSION))
        {
            return Err(CoreError::InvalidFixtureState);
        }

        let legacy_land = matches!(self.model_version, MODEL_VERSION | FOREST_MODEL_VERSION);
        if legacy_land && self.animals.len() != ANIMAL_COUNT {
            return Err(CoreError::InvalidFixtureState);
        }
        for (index, animal) in self.animals.iter().enumerate() {
            let position = Point { x: animal.x, y: animal.y };
            if (legacy_land && animal.id != format!("isopod-{}", index + 1))
                || !position.is_normalized()
                || !(0.0..TAU).contains(&animal.heading)
            {
                return Err(CoreError::InvalidFixtureState);
            }
        }
        match (self.model_version, &self.forest, &self.pond) {
            (MODEL_VERSION, None, None) => Ok(()),
            (FOREST_MODEL_VERSION, Some(forest), None) if !forest.is_seasonal() => forest.validate(self.rock, &self.animals),
            (FOREST_SEASONAL_MODEL_VERSION, Some(forest), None) if forest.is_seasonal() => forest.validate(self.rock, &self.animals),
            (FOREST_R5_MODEL_VERSION | FOREST_R5_TERRAIN_MODEL_VERSION | FOREST_R5_STONEWORK_MODEL_VERSION
                | FOREST_R5_BARK_MODEL_VERSION | FOREST_R5_LEAF_MODEL_VERSION | FOREST_R5_COMPOSED_MODEL_VERSION,
                Some(forest), None) if forest.is_seasonal() => {
                let layout = self.forest_rocks.as_ref().ok_or(CoreError::InvalidForestState)?;
                let permits_logs = matches!(self.model_version, FOREST_R5_TERRAIN_MODEL_VERSION
                    | FOREST_R5_STONEWORK_MODEL_VERSION | FOREST_R5_BARK_MODEL_VERSION | FOREST_R5_LEAF_MODEL_VERSION
                    | FOREST_R5_COMPOSED_MODEL_VERSION);
                let permits_stonework = matches!(self.model_version, FOREST_R5_STONEWORK_MODEL_VERSION
                    | FOREST_R5_BARK_MODEL_VERSION | FOREST_R5_LEAF_MODEL_VERSION | FOREST_R5_COMPOSED_MODEL_VERSION);
                let permits_bark = matches!(self.model_version, FOREST_R5_BARK_MODEL_VERSION | FOREST_R5_LEAF_MODEL_VERSION
                    | FOREST_R5_COMPOSED_MODEL_VERSION);
                let permits_leaf_patches = matches!(self.model_version, FOREST_R5_LEAF_MODEL_VERSION | FOREST_R5_COMPOSED_MODEL_VERSION);
                if layout.logs().is_some() != permits_logs
                    || layout.stonework().is_some() != permits_stonework
                    || layout.bark_shelters().is_some() != permits_bark
                    || layout.leaf_patches().is_some() != permits_leaf_patches
                    || layout.scenery().is_some() != (self.model_version == FOREST_R5_COMPOSED_MODEL_VERSION)
                {
                    return Err(CoreError::InvalidForestState);
                }
                layout.validate(&self.animals)?;
                forest.validate_inventory(&self.animals, layout.leaf_patches().unwrap_or_default(),
                    self.model_version == FOREST_R5_COMPOSED_MODEL_VERSION)
            }
            (POND_MODEL_VERSION | POND_R5_STONES_MODEL_VERSION | POND_R5_WOOD_MODEL_VERSION
            | POND_R5_MOSS_MODEL_VERSION | POND_R5_COMPOSED_MODEL_VERSION, None, Some(pond)) if self.animals.is_empty() => {
                self.validate_pond(pond)
            }
            (POND_MODEL_VERSION | POND_R5_STONES_MODEL_VERSION | POND_R5_WOOD_MODEL_VERSION
            | POND_R5_MOSS_MODEL_VERSION | POND_R5_COMPOSED_MODEL_VERSION, _, _) => Err(CoreError::InvalidPondState),
            _ => Err(CoreError::InvalidForestState),
        }
    }

    fn validate_pond(&self, pond: &pond::PondState) -> Result<(), CoreError> {
        pond.validate(
            self.model_version != POND_MODEL_VERSION,
            matches!(self.model_version, POND_R5_WOOD_MODEL_VERSION | POND_R5_MOSS_MODEL_VERSION | POND_R5_COMPOSED_MODEL_VERSION),
            matches!(self.model_version, POND_R5_MOSS_MODEL_VERSION | POND_R5_COMPOSED_MODEL_VERSION),
            self.model_version == POND_R5_COMPOSED_MODEL_VERSION,
        )
    }

    pub fn advance_steps(&mut self, steps: u32) -> Result<(), CoreError> {
        self.advance_steps_observing(steps, None)
    }

    /// Collect latest-step contact intake without adding any state to the ecosystem or its checkpoints.
    pub fn advance_steps_with_pond_contact_feeding(&mut self, steps: u32) -> Result<PondContactFeeding, CoreError> {
        let mut feedback = Vec::new();
        let capture = (self.model_version == POND_R5_COMPOSED_MODEL_VERSION).then_some(&mut feedback);
        self.advance_steps_observing(steps, capture)?;
        Ok(feedback)
    }

    fn advance_steps_observing(&mut self, steps: u32, contact_feeding: Option<&mut PondContactFeeding>) -> Result<(), CoreError> {
        let end = self.tick.checked_add(u64::from(steps)).ok_or(CoreError::TickOverflow)?;
        if end > u64::MAX / STEP_MILLIS {
            return Err(CoreError::TickOverflow);
        }
        if steps > 0 {
            for _ in 1..steps {
                self.advance_one(None);
            }
            self.advance_one(contact_feeding);
        }
        Ok(())
    }

    fn advance_one(&mut self, contact_feeding: Option<&mut PondContactFeeding>) {
        self.tick += 1;
        self.daylight.advance();
        if let Some(pond) = &mut self.pond {
            pond.advance(self.daylight.phase, season::observe(self.tick), contact_feeding);
            return;
        }
        if let Some(forest) = &mut self.forest {
            if forest.is_seasonal() {
                let terrain = forest::ForestTerrain { legacy_rock: self.rock, layout: self.forest_rocks.as_ref() };
                let geometry = forest.interactive_geometry(terrain);
                let patches = self.forest_rocks.as_mut().and_then(ForestRockLayout::leaf_patches_mut).unwrap_or_default();
                forest.advance_seasonal(geometry, patches, self.daylight.phase, &mut self.animals, &mut self.rng, season::observe(self.tick));
            } else {
                forest.advance(self.rock, self.daylight.phase, &mut self.animals, &mut self.rng);
            }
            return;
        }
        let rock_half = usize::from(self.rock.x >= 0.5);
        for (index, value) in self.moisture.iter_mut().enumerate() {
            let goal = if index == rock_half { 0.72 } else { 0.43 };
            *value += (goal - *value) * 0.008;
        }

        for animal in &mut self.animals {
            let dx = self.rock.x - animal.x;
            let dy = self.rock.y - animal.y;
            let near_rock = dx * dx + dy * dy < 0.0144;
            let activity = self.rng.unit();
            if near_rock && activity < 0.87 {
                animal.activity = Activity::Sheltering;
            } else if activity < 0.15 {
                animal.activity = Activity::Resting;
            } else {
                animal.activity = Activity::Moving;
                let drift = (self.rng.unit() - 0.5) * 0.65;
                animal.heading = (animal.heading + drift).rem_euclid(TAU);
                let next_x = animal.x + animal.heading.cos() * 0.003;
                let next_y = animal.y + animal.heading.sin() * 0.002;
                if !(0.06..=0.94).contains(&next_x) || !(0.62..=0.91).contains(&next_y) {
                    animal.heading = (animal.heading + TAU / 2.0).rem_euclid(TAU);
                }
                animal.x = next_x.clamp(0.06, 0.94);
                animal.y = next_y.clamp(0.62, 0.91);
            }
        }
    }

    pub fn set_daylight_target(&mut self, phase: f64) -> Result<(), CoreError> {
        if !(0.0..1.0).contains(&phase) {
            return Err(CoreError::InvalidPhase);
        }
        self.daylight.target = Some(phase);
        Ok(())
    }

    pub fn daylight_target(&self) -> Option<f64> {
        self.daylight.target
    }

    pub fn clear_daylight_target(&mut self) {
        self.daylight.target = None;
    }

    pub fn edit_rock(&mut self, position: Point) -> Result<(), CoreError> {
        if self.pond.is_some() || self.forest_rocks.is_some() {
            return Err(CoreError::WrongWorldKind);
        }
        if !position.is_normalized() {
            return Err(CoreError::InvalidPoint);
        }
        if self.forest.is_some() {
            forest::validate_rock_change(position, &self.animals)?;
        }
        self.rock = position;
        Ok(())
    }

    pub fn edit_pond(&mut self, edit: &PondEdit) -> Result<(), CoreError> {
        let climate = season::observe(self.tick);
        self.pond.as_mut().ok_or(CoreError::WrongWorldKind)?.edit(edit, climate)
    }

    /// Restore a paused pond edit while keeping the current daylight target and allocated IDs.
    pub fn restore_pond_edit_state(&mut self, previous: WorldState) -> Result<(), CoreError> {
        if self.model_version != previous.model_version || self.fixture_definition != previous.fixture_definition {
            return Err(CoreError::WrongWorldKind);
        }
        if self.tick != previous.tick { return Err(CoreError::InvalidPondState); }
        let current = self.pond.as_ref().ok_or(CoreError::WrongWorldKind)?;
        let mut restored = previous.pond.ok_or(CoreError::WrongWorldKind)?;
        restored.preserve_bed_allocations(current)?;
        self.validate_pond(&restored)?;
        self.pond = Some(restored);
        self.initial_habitat = previous.initial_habitat;
        Ok(())
    }

    /// Retain allocated basin and observation-section identities when restoring an older checkpoint.
    pub fn retain_pond_topology_allocator(&mut self, current: &WorldState) {
        if self.model_version != POND_R5_COMPOSED_MODEL_VERSION || current.model_version != self.model_version {
            return;
        }
        if let Some((restored, current)) = self.pond.as_mut().zip(current.pond.as_ref()) {
            restored.retain_topology_allocator(current);
        }
    }

    pub fn forest_rock_layout(&self) -> Option<&ForestRockLayout> {
        self.forest_rocks.as_ref()
    }

    pub fn forest_moss_patches(&self) -> Option<Vec<ForestMossPatchObservation>> {
        self.forest_moss_preset()?;
        let forest = self.forest.as_ref()?;
        Some(self.forest_rocks.as_ref()?.moss_patches().iter().copied().map(|patch| forest.observe_moss_patch(patch)).collect())
    }

    pub fn forest_moss_preset(&self) -> Option<ForestMossPresetObservation> {
        (self.model_version == FOREST_R5_COMPOSED_MODEL_VERSION).then(ForestMossPresetObservation::ground)
    }

    pub fn forest_fern_patches(&self) -> Option<Vec<ForestFernPatchObservation>> {
        self.forest_fern_preset()?;
        let forest = self.forest.as_ref()?;
        Some(self.forest_rocks.as_ref()?.fern_patches().iter().copied().map(|patch| forest.observe_fern_patch(patch)).collect())
    }

    pub fn forest_fern_preset(&self) -> Option<ForestFernPresetObservation> {
        (self.model_version == FOREST_R5_COMPOSED_MODEL_VERSION).then(ForestFernPresetObservation::ground)
    }

    pub fn forest_substrate_cells(&self) -> Option<Vec<ForestSubstrateCellObservation>> {
        if self.model_version != FOREST_R5_COMPOSED_MODEL_VERSION { return None; }
        self.forest.as_ref().map(forest::ForestState::observe_substrate)
    }

    pub fn forest_substrate_presets(&self) -> Option<[ForestSubstratePresetObservation; 3]> {
        (self.model_version == FOREST_R5_COMPOSED_MODEL_VERSION).then(ForestSubstratePresetObservation::all)
    }

    pub fn edit_forest_rocks(&mut self, edit: &ForestRockEdit) -> Result<(), CoreError> {
        if let ForestRockEdit::SetSubstrate { cells, substrate } = edit {
            if self.model_version != FOREST_R5_COMPOSED_MODEL_VERSION { return Err(CoreError::WrongWorldKind); }
            return self.forest.as_mut().ok_or(CoreError::WrongWorldKind)?.set_substrate(cells, *substrate);
        }
        if let ForestRockEdit::StockWoodlice { position, count } = *edit {
            if self.model_version != FOREST_R5_COMPOSED_MODEL_VERSION { return Err(CoreError::WrongWorldKind); }
            if !forest::on_ground(position) { return Err(CoreError::InvalidWoodlouseStockSite); }
            let terrain = forest::ForestTerrain { legacy_rock: self.rock, layout: self.forest_rocks.as_ref() };
            if !terrain.walkable_at(position) { return Err(CoreError::OccupiedWoodlouseStockSite); }
            return self.forest.as_mut().ok_or(CoreError::WrongWorldKind)?
                .stock_woodlice(&mut self.animals, position, count);
        }
        if let ForestRockEdit::RemoveWoodlouse { id } = edit {
            if self.model_version != FOREST_R5_COMPOSED_MODEL_VERSION { return Err(CoreError::WrongWorldKind); }
            return self.forest.as_mut().ok_or(CoreError::WrongWorldKind)?.remove_woodlouse(&mut self.animals, id);
        }
        let forest = self.forest.as_mut().ok_or(CoreError::WrongWorldKind)?;
        let layout = self.forest_rocks.as_mut().ok_or(CoreError::WrongWorldKind)?;
        let transfer = match *edit {
            ForestRockEdit::AddLeafPatch { preset, .. } => Some((preset.initial_mg_cnp(), [0.0; 3])),
            ForestRockEdit::PaintLeafPatches { preset, ref positions } => {
                let count = positions.len() as f64;
                Some((preset.initial_mg_cnp().map(|amount| amount * count), [0.0; 3]))
            }
            ForestRockEdit::SpreadLeafFragments { mg_c, .. } => {
                Some((ForestLeafPreset::dose_for_carbon(mg_c), [0.0; 3]))
            }
            ForestRockEdit::Remove { id } => layout.leaf_patches().unwrap_or_default().iter()
                .find(|patch| patch.id == id).map(|patch| ([0.0; 3], patch.remaining_mg_cnp)),
            _ => None,
        };
        layout.edit(edit, &self.animals, forest)?;
        if let Some((incoming, outgoing)) = transfer {
            forest.record_leaf_edit(incoming, outgoing);
        }
        Ok(())
    }

    pub fn restore_forest_rocks(&mut self, layout: ForestRockLayout) -> Result<(), CoreError> {
        if matches!(self.model_version, FOREST_R5_LEAF_MODEL_VERSION | FOREST_R5_COMPOSED_MODEL_VERSION) {
            return Err(CoreError::WrongWorldKind);
        }
        self.forest_rocks.as_mut().ok_or(CoreError::WrongWorldKind)?.restore(layout, &self.animals)
    }

    pub fn forest_layout_edit_state(&self) -> Option<ForestLayoutEditState> {
        let forest = self.forest.as_ref()?;
        Some(ForestLayoutEditState {
            tick: self.tick,
            layout: self.forest_rocks.clone()?,
            flows: forest.matter_flows(),
            fauna: (self.model_version == FOREST_R5_COMPOSED_MODEL_VERSION)
                .then(|| forest.fauna_edit_state(&self.animals)).flatten(),
            ground: (self.model_version == FOREST_R5_COMPOSED_MODEL_VERSION).then(|| forest.ground_edit_state()).flatten(),
        })
    }

    pub fn restore_forest_layout_edit_state(&mut self, state: ForestLayoutEditState) -> Result<(), CoreError> {
        if self.tick != state.tick {
            return Err(CoreError::InvalidForestState);
        }
        let layout = self.forest_rocks.as_mut().ok_or(CoreError::WrongWorldKind)?;
        let forest = self.forest.as_mut().ok_or(CoreError::WrongWorldKind)?;
        let mut candidate = layout.clone();
        let allows_stocking = self.model_version == FOREST_R5_COMPOSED_MODEL_VERSION;
        if state.fauna.is_some() != allows_stocking || state.ground.is_some() != allows_stocking {
            return Err(CoreError::WrongWorldKind);
        }
        if let (Some(fauna), Some(ground)) = (state.fauna, state.ground) {
            let mut restored = forest.clone();
            let animals = restored.restore_fauna_edit_state(fauna)?;
            candidate.restore(state.layout, &animals)?;
            restored.restore_matter_flows(state.flows);
            restored.restore_ground_edit_state(ground)?;
            restored.validate_inventory(&animals, candidate.leaf_patches().unwrap_or_default(), true)?;
            *layout = candidate;
            *forest = restored;
            self.animals = animals;
            return Ok(());
        }
        candidate.restore(state.layout, &self.animals)?;
        let current_flows = forest.matter_flows();
        forest.restore_matter_flows(state.flows);
        if let Err(error) = forest.validate_inventory(&self.animals, candidate.leaf_patches().unwrap_or_default(), false) {
            forest.restore_matter_flows(current_flows);
            return Err(error);
        }
        *layout = candidate;
        Ok(())
    }

    pub fn model_version(&self) -> u32 {
        self.model_version
    }

    pub fn definition(&self) -> &str {
        &self.fixture_definition
    }

    pub fn season_observation(&self) -> SeasonObservation {
        season::observe(self.tick)
    }

    pub fn tick(&self) -> u64 {
        self.tick
    }

    pub fn simulated_millis(&self) -> u64 {
        self.tick * STEP_MILLIS
    }

    pub fn daylight_phase(&self) -> f64 {
        self.daylight.phase
    }

    pub fn rock(&self) -> Point {
        self.rock
    }

    pub fn moisture(&self) -> [f64; 2] {
        self.forest.as_ref().map_or(self.moisture, forest::ForestState::moisture)
    }

    pub fn animals(&self) -> &[Animal] {
        &self.animals
    }

    pub fn forest_observation(&self) -> Option<ForestObservation> {
        self.forest.as_ref().map(|forest| {
            if forest.is_seasonal() {
                let terrain = forest::ForestTerrain { legacy_rock: self.rock, layout: self.forest_rocks.as_ref() };
                forest.observe_seasonal(terrain, self.daylight.phase, &self.animals, self.season_observation())
            } else {
                forest.observe(self.rock, self.daylight.phase, &self.animals)
            }
        })
    }

    pub fn forest_ledger(&self) -> Option<ForestLedger> {
        let patches = self.forest_rocks.as_ref().and_then(ForestRockLayout::leaf_patches).unwrap_or_default();
        self.forest.as_ref().map(|forest| forest.ledger(patches))
    }

    pub fn pond_observation(&self) -> Option<PondObservation> {
        self.pond.as_ref().map(pond::PondState::observe)
    }

    pub fn pond_ledger(&self) -> Option<PondLedger> {
        self.pond.as_ref().map(pond::PondState::ledger)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composed_forest_starts_with_real_shelter_and_balanced_litter() {
        let seeded = WorldState::new_r5_composed_forest(1206);
        assert!(seeded.validate().is_ok());
        let layout = seeded.forest_rock_layout().unwrap();
        let arch = layout.bark_shelters().unwrap()[0];
        assert_eq!(arch.foot_cells(), [12, 14]);
        assert_eq!(arch.passage_cells(), [1, 13, 25]);
        let ledger = seeded.forest_ledger().unwrap();
        let old = WorldState::new_r5_leaf_forest(1206).forest_ledger().unwrap();
        for component in 0..3 {
            let amount = ForestLeafPreset::Heap.initial_mg_cnp()[component];
            assert!((ledger.initial_matter_mg_cnp[component] - old.initial_matter_mg_cnp[component] - amount).abs() < 1e-8);
            assert_eq!(ledger.matter_input_mg_cnp[component], 0.0);
        }
    }

    #[test]
    fn composed_blank_starter_and_custom_habitat_keep_existing_litter() {
        let mut world = WorldState::new_blank_r5_composed_forest(1206);
        let arch = Point { x: 88.0 / 576.0, y: 232.0 / 324.0 };
        let leaf = Point { x: 361.0 / 576.0, y: 254.0 / 324.0 };
        world.edit_forest_rocks(&ForestRockEdit::AddBarkShelter { position: arch, preset: ForestBarkPreset::Arch }).unwrap();
        world.edit_forest_rocks(&ForestRockEdit::AddLeafPatch { position: leaf, preset: ForestLeafPreset::Heap }).unwrap();
        let before = world.forest_rock_layout().unwrap().leaf_patches().unwrap()[0];
        world.add_starter_community().unwrap();
        assert!(world.validate().is_ok());
        let after = world.forest_rock_layout().unwrap().leaf_patches().unwrap()[0];
        assert_eq!(after, before);
        let preset = world.fresh_habitat(1206).unwrap();
        assert_eq!(preset.forest_rock_layout().unwrap().leaf_patches().unwrap(), &[before]);
        assert!(preset.validate().is_ok());
    }

    #[test]
    fn r5_stone_world_stays_compatible_with_empty_terrain_world() {
        let mut stone_world = WorldState::new_r5_forest(42);
        let original = stone_world.clone();
        let log = ForestRockEdit::AddLog { position: Point { x: 0.50, y: 0.615 } };
        assert_eq!(stone_world.edit_forest_rocks(&log), Err(CoreError::WrongWorldKind));
        assert_eq!(stone_world, original);
        assert!(stone_world.forest_rock_layout().unwrap().logs().is_none());

        let mut terrain_world = WorldState::new_r5_terrain_forest(42);
        assert!(matches!(terrain_world.forest_rock_layout().unwrap().logs(), Some([])));
        for world in [&mut stone_world, &mut terrain_world] {
            world.advance_steps(1800).unwrap();
            world.validate().unwrap();
        }
        assert_eq!(terrain_world.forest_observation(), stone_world.forest_observation());
        assert_eq!(terrain_world.forest_ledger(), stone_world.forest_ledger());
        assert_eq!(terrain_world.animals(), stone_world.animals());
    }

    #[test]
    fn r5_log_footprint_controls_ground_without_changing_inventory() {
        let mut world = WorldState::new_r5_terrain_forest(1206);
        let original = world.clone();
        let layout = world.forest_rock_layout().unwrap().clone();
        let before = world.forest_observation().unwrap();
        let occupied_site = Point { x: 0.50, y: 0.68 };
        assert_eq!(world.edit_forest_rocks(&ForestRockEdit::AddLog { position: occupied_site }), Err(CoreError::OverlappingLogSite));
        assert_eq!(world.edit_forest_rocks(&ForestRockEdit::AddLog { position: Point { x: 0.20, y: 0.615 } }), Err(CoreError::InvalidLogSite));
        assert_eq!(world, original);

        let north = Point { x: 0.50, y: 0.615 };
        let south = Point { x: 0.50, y: 0.88 };
        for position in [north, south] {
            world.edit_forest_rocks(&ForestRockEdit::AddLog { position }).unwrap();
        }
        let logs = world.forest_rock_layout().unwrap().logs().unwrap();
        assert_eq!(logs.iter().map(|log| log.id).collect::<Vec<_>>(), [2, 3]);
        let after = world.forest_observation().unwrap();
        let walkable = |observation: &ForestObservation| observation.walkable_cells.iter().sum::<u32>();
        assert_eq!(walkable(&before) - walkable(&after), 16);
        assert!(after.shade_fraction.iter().zip(before.shade_fraction).all(|(new, old)| *new > old));
        assert_eq!(world.forest_ledger(), original.forest_ledger());

        world.edit_forest_rocks(&ForestRockEdit::Mirror { id: 2 }).unwrap();
        assert_eq!(world.forest_observation(), Some(after));
        assert!(world.forest_rock_layout().unwrap().logs().unwrap()[0].mirrored);
        world.edit_forest_rocks(&ForestRockEdit::Move { id: 2, position: Point { x: 0.34, ..north } }).unwrap();
        world.edit_forest_rocks(&ForestRockEdit::Remove { id: 3 }).unwrap();
        assert_eq!(world.forest_rock_layout().unwrap().logs().unwrap().len(), 1);
        world.restore_forest_rocks(layout).unwrap();
        assert_eq!(world.forest_observation(), Some(before));
        world.edit_forest_rocks(&ForestRockEdit::AddLog { position: south }).unwrap();
        assert_eq!(world.forest_rock_layout().unwrap().logs().unwrap()[0].id, 4);
        world.advance_steps(2500).unwrap();
        world.validate().unwrap();
    }

    #[test]
    fn r5_log_rejects_animal_overlap_and_reserves_footprint_in_fresh_presets() {
        let mut world = WorldState::new_r5_terrain_forest(1206);
        let position = Point { x: 0.50, y: world.animals()[2].y };
        world.edit_forest_rocks(&ForestRockEdit::Remove { id: 1 }).unwrap();
        let original = world.clone();
        assert_eq!(world.edit_forest_rocks(&ForestRockEdit::AddLog { position }), Err(CoreError::OccupiedLogSite));
        assert_eq!(world, original);

        let log = ForestLog { id: 2, position, mirrored: false };
        forest::clear_initial_structure_cells(std::iter::empty(), &[log], &[], &[], &mut world.animals);
        world.edit_forest_rocks(&ForestRockEdit::AddLog { position }).unwrap();
        let fresh = world.fresh_habitat(1206).unwrap();
        assert_eq!(fresh.forest_rock_layout(), world.forest_rock_layout());
        let footprint = forest::log_cells(position);
        assert!(fresh.animals().iter().all(|animal| {
            let position = Point { x: animal.x, y: animal.y };
            !footprint.contains(&forest::cell_index(position))
        }));
        fresh.validate().unwrap();
    }

    #[test]
    fn r5_stonework_layouts_preserve_contact_gaps_and_inventory() {
        let mut world = WorldState::new_r5_stonework_forest(1206);
        let original = world.clone();
        let walkable = |world: &WorldState| world.forest_observation().unwrap().walkable_cells.iter().sum::<u32>();
        let pair = Point { x: 0.50, y: 0.615 };
        let cluster = Point { x: 0.47, y: 0.885 };
        for (position, preset) in [(pair, ForestStoneworkKind::Pair), (cluster, ForestStoneworkKind::Cluster)] {
            world.edit_forest_rocks(&ForestRockEdit::AddStonework { position, preset }).unwrap();
        }
        assert_eq!(walkable(&original) - walkable(&world), 5);
        assert!(world.forest_observation().unwrap().shade_fraction.iter()
            .zip(original.forest_observation().unwrap().shade_fraction).all(|(new, old)| *new > old));

        // A stone in the pair's central gap changes the ground by one cell.
        world.edit_forest_rocks(&ForestRockEdit::Add { position: pair }).unwrap();
        assert_eq!(walkable(&original) - walkable(&world), 6);
        let before_rejections = world.clone();
        assert_eq!(world.edit_forest_rocks(&ForestRockEdit::AddLog { position: cluster }), Err(CoreError::OverlappingStoneworkSite));
        assert_eq!(world.edit_forest_rocks(&ForestRockEdit::AddStonework {
            position: Point { x: 0.10, ..pair }, preset: ForestStoneworkKind::Pair,
        }), Err(CoreError::InvalidStoneworkSite));
        let animal = &world.animals()[2];
        let position = Point { x: animal.x, y: animal.y };
        assert_eq!(world.edit_forest_rocks(&ForestRockEdit::AddStonework { position, preset: ForestStoneworkKind::Slab }),
            Err(CoreError::OccupiedStoneworkSite));
        assert_eq!(world, before_rejections);

        let slab = Point { x: 0.18, ..pair };
        world.edit_forest_rocks(&ForestRockEdit::AddStonework { position: slab, preset: ForestStoneworkKind::Slab }).unwrap();
        assert_eq!(walkable(&original) - walkable(&world), 7);
        let layout = world.forest_rock_layout().unwrap().clone();
        let observation = world.forest_observation();
        world.edit_forest_rocks(&ForestRockEdit::Mirror { id: 2 }).unwrap();
        assert_eq!(world.forest_observation(), observation);
        assert!(world.forest_rock_layout().unwrap().stonework().unwrap()[0].mirrored);
        world.edit_forest_rocks(&ForestRockEdit::Move { id: 2, position: Point { x: 0.75, ..pair } }).unwrap();
        world.edit_forest_rocks(&ForestRockEdit::Remove { id: 3 }).unwrap();
        world.restore_forest_rocks(layout).unwrap();
        assert_eq!(world.forest_observation(), observation);
        assert_eq!(world.forest_ledger(), original.forest_ledger());
        let fresh = world.fresh_habitat(1206).unwrap();
        assert_eq!(fresh.forest_rock_layout(), world.forest_rock_layout());
        world.advance_steps(2500).unwrap();
        world.validate().unwrap();
    }

    #[test]
    fn r5_existing_terrain_trajectory_matches_empty_stonework_world() {
        let mut terrain = WorldState::new_r5_terrain_forest(1206);
        let mut stonework = WorldState::new_r5_stonework_forest(1206);
        let log = ForestRockEdit::AddLog { position: Point { x: 0.50, y: 0.885 } };
        let slab = ForestRockEdit::AddStonework { position: Point { x: 0.18, y: 0.615 }, preset: ForestStoneworkKind::Slab };
        assert_eq!(terrain.edit_forest_rocks(&slab), Err(CoreError::WrongWorldKind));
        assert!(terrain.forest_rock_layout().unwrap().stonework().is_none());
        assert!(matches!(stonework.forest_rock_layout().unwrap().stonework(), Some([])));
        for world in [&mut terrain, &mut stonework] {
            world.edit_forest_rocks(&log).unwrap();
            world.advance_steps(1800).unwrap();
            world.validate().unwrap();
        }
        assert_eq!(stonework.forest_observation(), terrain.forest_observation());
        assert_eq!(stonework.forest_ledger(), terrain.forest_ledger());
        assert_eq!(stonework.animals(), terrain.animals());
    }

    #[test]
    fn r5_bark_reserves_open_passages_without_moving_animals_or_inventory() {
        let mut world = WorldState::new_r5_bark_forest(1206);
        let under_roof = Point { x: 0.38, y: 0.80 };
        world.animals[1].x = under_roof.x;
        world.animals[1].y = under_roof.y;
        let previous_animals = world.animals().to_vec();
        let original = world.clone();
        let arch_on_animal = ForestRockEdit::AddBarkShelter {
            position: Point { x: 0.50, ..under_roof }, preset: ForestBarkPreset::Arch,
        };
        assert_eq!(world.edit_forest_rocks(&arch_on_animal), Err(CoreError::OccupiedBarkFeet));
        assert_eq!(world, original);

        let arch = ForestRockEdit::AddBarkShelter { position: under_roof, preset: ForestBarkPreset::Arch };
        let split = ForestRockEdit::AddBarkShelter {
            position: Point { x: 0.74, y: 0.73 }, preset: ForestBarkPreset::Split,
        };
        world.edit_forest_rocks(&arch).unwrap();
        world.edit_forest_rocks(&split).unwrap();
        let layout = world.forest_rock_layout().unwrap().clone();
        let [bark, other] = layout.bark_shelters().unwrap() else { panic!("both shelters are present") };
        assert_eq!((bark.id, other.id), (2, 3));
        assert!(bark.contains(under_roof));
        assert!(!bark.in_passage(Point { y: 0.99, ..under_roof }));
        let previous_observation = world.forest_observation();
        let walkable = |world: &WorldState| world.forest_observation().unwrap().walkable_cells.iter().sum::<u32>();
        assert_eq!(walkable(&original) - walkable(&world), 4);

        let blocking_entrance = ForestRockEdit::Add { position: Point { x: 0.38, y: 0.715 } };
        assert_eq!(world.edit_forest_rocks(&blocking_entrance), Err(CoreError::BlockedBarkPassage));
        assert_eq!(world.edit_forest_rocks(&arch), Err(CoreError::OverlappingBarkSite));
        assert_eq!(world.forest_rock_layout(), Some(&layout));
        world.edit_forest_rocks(&ForestRockEdit::Mirror { id: bark.id }).unwrap();
        assert_eq!(world.forest_observation(), previous_observation);
        world.edit_forest_rocks(&ForestRockEdit::Move { id: bark.id, position: Point { x: 0.16, y: 0.80 } }).unwrap();
        world.edit_forest_rocks(&blocking_entrance).unwrap();
        world.edit_forest_rocks(&ForestRockEdit::Remove { id: other.id }).unwrap();
        assert_eq!(world.animals(), previous_animals);
        world.restore_forest_rocks(layout).unwrap();
        assert_eq!(world.forest_observation(), previous_observation);
        assert_eq!(world.forest_ledger(), original.forest_ledger());
        assert_eq!(world.animals(), previous_animals);
        let fresh = world.fresh_habitat(1206).unwrap();
        assert_eq!(fresh.forest_rock_layout(), world.forest_rock_layout());
        world.validate().unwrap();
    }

    #[test]
    fn r5_existing_stonework_trajectory_matches_empty_bark_world() {
        let mut stonework = WorldState::new_r5_stonework_forest(1206);
        let mut bark = WorldState::new_r5_bark_forest(1206);
        let add_shelter = ForestRockEdit::AddBarkShelter {
            position: Point { x: 0.38, y: 0.80 }, preset: ForestBarkPreset::Arch,
        };
        assert_eq!(stonework.edit_forest_rocks(&add_shelter), Err(CoreError::WrongWorldKind));
        assert!(stonework.forest_rock_layout().unwrap().bark_shelters().is_none());
        assert!(matches!(bark.forest_rock_layout().unwrap().bark_shelters(), Some([])));
        for world in [&mut stonework, &mut bark] {
            world.edit_forest_rocks(&ForestRockEdit::AddStonework {
                position: Point { x: 0.18, y: 0.615 }, preset: ForestStoneworkKind::Slab,
            }).unwrap();
            world.advance_steps(1800).unwrap();
            world.validate().unwrap();
        }
        assert_eq!(bark.forest_observation(), stonework.forest_observation());
        assert_eq!(bark.forest_ledger(), stonework.forest_ledger());
        assert_eq!(bark.animals(), stonework.animals());
    }

    #[test]
    fn r5_existing_bark_trajectory_matches_empty_leaf_world() {
        let mut bark = WorldState::new_r5_bark_forest(1206);
        let mut leaves = WorldState::new_r5_leaf_forest(1206);
        let under_roof = Point { x: 0.38, y: 0.80 };
        assert!(bark.forest_rock_layout().unwrap().leaf_patches().is_none());
        assert!(matches!(leaves.forest_rock_layout().unwrap().leaf_patches(), Some([])));
        for world in [&mut bark, &mut leaves] {
            world.edit_forest_rocks(&ForestRockEdit::AddBarkShelter {
                position: under_roof, preset: ForestBarkPreset::Arch,
            }).unwrap();
            world.advance_steps(1800).unwrap();
            world.validate().unwrap();
        }
        let under_bark = ForestRockEdit::AddLeafPatch { position: under_roof, preset: ForestLeafPreset::Leaves };
        assert_eq!(bark.edit_forest_rocks(&under_bark), Err(CoreError::WrongWorldKind));
        assert_eq!(leaves.edit_forest_rocks(&under_bark), Err(CoreError::OverlappingLeafPatchSite));
        assert_eq!(leaves.forest_observation(), bark.forest_observation());
        assert_eq!(leaves.forest_ledger(), bark.forest_ledger());
        assert_eq!(leaves.animals(), bark.animals());
        assert_eq!(leaves.rng, bark.rng);
    }

    #[test]
    fn r5_leaf_patch_tracks_placed_moved_depleted_and_removed_material() {
        let mut world = WorldState::new_r5_leaf_forest(42);
        let baseline = world.forest_ledger().unwrap();
        let baseline_observation = world.forest_observation().unwrap();
        let animal = &world.animals()[0];
        let position = Point { x: animal.x, y: animal.y };
        let add = ForestRockEdit::AddLeafPatch { position, preset: ForestLeafPreset::Leaves };
        let under_stone = ForestRockEdit::AddLeafPatch { position: world.rock(), preset: ForestLeafPreset::Leaves };
        assert_eq!(world.edit_forest_rocks(&under_stone), Err(CoreError::OverlappingLeafPatchSite));
        world.edit_forest_rocks(&add).unwrap();
        let placed = world.forest_rock_layout().unwrap().leaf_patches().unwrap()[0];
        let dose = placed.initial_mg_cnp();
        assert_eq!(world.forest_ledger().unwrap().matter_input_mg_cnp, dose);
        let observation = world.forest_observation().unwrap();
        assert_eq!(observation.walkable_cells, baseline_observation.walkable_cells);
        assert_eq!(observation.litter_mg_c - baseline_observation.litter_mg_c, dose[0]);
        assert_eq!(world.edit_forest_rocks(&add), Err(CoreError::OverlappingLeafPatchSite));

        let destination = Point { x: 0.29, y: 0.615 };
        world.edit_forest_rocks(&ForestRockEdit::Move { id: placed.id, position: destination }).unwrap();
        world.edit_forest_rocks(&ForestRockEdit::Mirror { id: placed.id }).unwrap();
        let moved = world.forest_rock_layout().unwrap().leaf_patches().unwrap()[0];
        assert_eq!(moved, ForestLeafPatch { position: destination, mirrored: true, ..placed });
        let blocked = ForestRockEdit::AddStonework { position: destination, preset: ForestStoneworkKind::Slab };
        assert_eq!(world.edit_forest_rocks(&blocked), Err(CoreError::OverlappingLeafPatchSite));
        assert_eq!(world.forest_ledger().unwrap().matter_input_mg_cnp, dose);

        world.advance_steps(1800).unwrap();
        let depleted = world.forest_rock_layout().unwrap().leaf_patches().unwrap()[0];
        assert!(depleted.remaining_mg_cnp.into_iter().zip(dose).all(|(remaining, given)| remaining < given && remaining > 0.0));
        let new_habitat = world.fresh_habitat(138).unwrap();
        assert_eq!(new_habitat.forest_rock_layout().unwrap().leaf_patches().unwrap()[0].remaining_mg_cnp, dose);
        let fresh = new_habitat.forest_ledger().unwrap();
        assert_eq!(fresh.matter_input_mg_cnp, [0.0; 3]);
        assert_eq!(fresh.initial_matter_mg_cnp, std::array::from_fn(|i| baseline.initial_matter_mg_cnp[i] + dose[i]));

        let before_removal = world.forest_ledger().unwrap();
        world.edit_forest_rocks(&ForestRockEdit::Remove { id: placed.id }).unwrap();
        let after_removal = world.forest_ledger().unwrap();
        assert_eq!(after_removal.matter_output_mg_cnp, std::array::from_fn(|i| {
            before_removal.matter_output_mg_cnp[i] + depleted.remaining_mg_cnp[i]
        }));
        assert_eq!(after_removal.decomposed_mg_c, before_removal.decomposed_mg_c);
        world.validate().unwrap();
    }

    #[test]
    fn r5_leaf_brush_rejects_invalid_strokes_without_allocating_or_bookkeeping() {
        let preset = ForestLeafPreset::Leaves;
        let paint = |positions| ForestRockEdit::PaintLeafPatches { preset, positions };
        let mut world = WorldState::new_r5_leaf_forest(1206);
        let baseline = world.clone();
        let free = Point { x: 0.28, y: 0.62 };
        let same_cell = Point { x: 0.29, y: 0.63 };
        let invalid = [
            (paint(Vec::new()), CoreError::InvalidLeafPatchBatchSize),
            (paint(vec![free; 17]), CoreError::ForestLeafPatchLimit),
            (paint(vec![free, same_cell]), CoreError::OverlappingLeafPatchSite),
            (paint(vec![free, world.rock()]), CoreError::OverlappingLeafPatchSite),
            (paint(vec![free, Point { x: 0.99, y: 0.74 }]), CoreError::InvalidLeafPatchSite),
            (paint(vec![free, Point { x: 0.60, y: f64::NAN }]), CoreError::InvalidLeafPatchSite),
        ];
        for (stroke, error) in invalid {
            assert_eq!(world.edit_forest_rocks(&stroke), Err(error));
            assert_eq!(world, baseline);
        }
        let first = paint(vec![free]);
        let mut previous = WorldState::new_r5_bark_forest(1206);
        let previous_before = previous.clone();
        assert_eq!(previous.edit_forest_rocks(&first), Err(CoreError::WrongWorldKind));
        assert_eq!(previous, previous_before);

        let positions: Vec<_> = [0.62, 0.71, 0.80, 0.89].into_iter().flat_map(|y| {
            [0.28, 0.38, 0.62, 0.72].into_iter().map(move |x| Point { x, y })
        }).collect();
        world.edit_forest_rocks(&paint(positions.clone())).unwrap();
        let patches = world.forest_rock_layout().unwrap().leaf_patches().unwrap();
        assert_eq!(patches.iter().map(|patch| patch.id).collect::<Vec<_>>(), (2..=17).collect::<Vec<_>>());
        assert_eq!(patches.iter().map(|patch| patch.position).collect::<Vec<_>>(), positions);
        let full = world.clone();
        let extra = paint(vec![Point { x: 0.81, y: 0.82 }]);
        assert_eq!(world.edit_forest_rocks(&extra), Err(CoreError::ForestLeafPatchLimit));
        assert_eq!(world, full);
        world.edit_forest_rocks(&ForestRockEdit::Remove { id: 2 }).unwrap();
        world.edit_forest_rocks(&extra).unwrap();
        assert_eq!(world.forest_rock_layout().unwrap().leaf_patches().unwrap().last().unwrap().id, 18);
        world.validate().unwrap();
    }

    #[test]
    fn r5_fragment_spread_conserves_the_stroke_dose_across_ground_cells_and_history() {
        let mut world = WorldState::new_blank_r5_composed_forest(402);
        let empty = world.clone();
        let left = Point { x: 0.36, y: 0.69 };
        let middle = Point { x: 0.48, y: 0.69 };
        let right = Point { x: 0.55, y: 0.69 };
        let paint = |positions, mg_c| ForestRockEdit::SpreadLeafFragments { positions, mg_c };
        for invalid in [0.0, f64::NAN, 241.0] {
            assert_eq!(world.edit_forest_rocks(&paint(vec![left], invalid)), Err(CoreError::InvalidLeafFragmentDose));
        }
        assert_eq!(world.edit_forest_rocks(&paint(vec![left, left], 24.0)), Err(CoreError::OverlappingLeafPatchSite));
        assert_eq!(world.edit_forest_rocks(&paint(vec![left, Point { x: 0.90, y: 0.69 }], 24.0)), Err(CoreError::InvalidLeafPatchSite));
        assert_eq!(world, empty);

        let baseline = world.forest_layout_edit_state().unwrap();
        let deposit = 23.2;
        world.edit_forest_rocks(&paint(vec![left, middle, right], deposit)).unwrap();
        let patches = world.forest_rock_layout().unwrap().leaf_patches().unwrap().to_vec();
        assert_eq!(patches.len(), 3);
        assert!(patches.iter().all(|patch| patch.preset == ForestLeafPreset::Fragments));
        assert!(patches.iter().all(|patch| (patch.initial_mg_cnp()[0] - deposit / 3.0).abs() < 1e-10));
        let dose = ForestLeafPreset::dose_for_carbon(deposit);
        for (element, amount) in dose.into_iter().enumerate() {
            assert!((patches.iter().map(|patch| patch.remaining_mg_cnp[element]).sum::<f64>() - amount).abs() < 1e-10);
        }
        assert_eq!(world.forest_ledger().unwrap().matter_input_mg_cnp, dose);

        let after_spread = world.forest_layout_edit_state().unwrap();
        world.restore_forest_layout_edit_state(baseline).unwrap();
        assert!(world.forest_rock_layout().unwrap().leaf_patches().unwrap().is_empty());
        assert_eq!(world.forest_ledger(), empty.forest_ledger());
        world.restore_forest_layout_edit_state(after_spread).unwrap();
        assert_eq!(world.forest_rock_layout().unwrap().leaf_patches().unwrap(), patches);
        world.edit_forest_rocks(&ForestRockEdit::Move { id: patches[0].id, position: Point { x: 0.25, y: 0.84 } }).unwrap();
        assert_eq!(world.forest_rock_layout().unwrap().leaf_patches().unwrap()[0].remaining_mg_cnp, patches[0].remaining_mg_cnp);
        assert_eq!(world.forest_ledger().unwrap().matter_input_mg_cnp, dose);

        let saved = serde_json::to_vec(&world).unwrap();
        let mut reopened: WorldState = serde_json::from_slice(&saved).unwrap();
        assert_eq!(reopened, world);
        reopened.validate().unwrap();
        reopened.advance_steps(320).unwrap();
        let remaining = reopened.forest_rock_layout().unwrap().leaf_patches().unwrap()[0];
        assert!(remaining.remaining_mg_cnp[0] < patches[0].remaining_mg_cnp[0]);
        let reset = reopened.fresh_habitat(403).unwrap();
        assert_eq!(reset.forest_rock_layout().unwrap().leaf_patches().unwrap()[0].remaining_mg_cnp, patches[0].remaining_mg_cnp);
        let before = reopened.forest_ledger().unwrap();
        reopened.edit_forest_rocks(&ForestRockEdit::Remove { id: remaining.id }).unwrap();
        let after = reopened.forest_ledger().unwrap();
        for element in 0..3 {
            assert!((after.matter_output_mg_cnp[element] - before.matter_output_mg_cnp[element] - remaining.remaining_mg_cnp[element]).abs() < 1e-10);
        }
        reopened.validate().unwrap();
    }

    #[test]
    fn r5_woodlouse_stock_uses_walkable_ground_and_adult_inventory() {
        let stock = |position, count| ForestRockEdit::StockWoodlice { position, count };
        let mut world = WorldState::new_r5_composed_forest(1206);
        let initial = world.clone();
        let arch = world.forest_rock_layout().unwrap().bark_shelters().unwrap()[0];
        let cave = arch.position;
        let foot = Point { x: 0.235, y: cave.y };
        assert_eq!(forest::cell_index(foot), arch.foot_cells()[1]);
        let before_ledger = world.forest_ledger().unwrap();
        for (edit, error) in [
            (stock(cave, 0), CoreError::InvalidWoodlouseStockCount),
            (stock(cave, 29), CoreError::ForestWoodlouseLimit),
            (stock(world.rock(), 1), CoreError::OccupiedWoodlouseStockSite),
            (stock(foot, 1), CoreError::OccupiedWoodlouseStockSite),
            (stock(Point { y: 0.30, ..cave }, 1), CoreError::InvalidWoodlouseStockSite),
            (stock(Point { x: f64::NAN, ..cave }, 1), CoreError::InvalidWoodlouseStockSite),
        ] {
            assert_eq!(world.edit_forest_rocks(&edit), Err(error));
            assert_eq!(world, initial);
        }
        let mut v9 = WorldState::new_r5_leaf_forest(1206);
        assert_eq!(v9.edit_forest_rocks(&stock(cave, 1)), Err(CoreError::WrongWorldKind));

        world.edit_forest_rocks(&stock(cave, 2)).unwrap();
        let animals = world.animals();
        assert_eq!(&animals[..4], initial.animals());
        assert_eq!(animals[4].id, "isopod-5");
        assert_eq!(animals[5].id, "isopod-6");
        assert_eq!(Point { x: animals[4].x, y: animals[4].y }, cave);
        assert_ne!(animals[5].x, animals[4].x);
        assert!(animals[4..].iter().all(|animal| arch.in_passage(Point { x: animal.x, y: animal.y })));
        let life = world.forest_observation().unwrap().life.unwrap();
        assert_eq!(life.woodlice_born, 0);
        assert!(life.woodlice[4..].iter().all(|animal| animal.stage == forest::WoodlouseStage::Adult && animal.biomass_mg_c == 12.0));
        assert_eq!(world.rng, initial.rng);
        let ledger = world.forest_ledger().unwrap();
        for (index, expected) in [29.2, 3.98, 0.408].into_iter().enumerate() {
            assert!((ledger.matter_input_mg_cnp[index] - before_ledger.matter_input_mg_cnp[index] - expected).abs() < 1e-9);
        }
        assert_eq!(ledger.matter_output_mg_cnp, before_ledger.matter_output_mg_cnp);
        let on_leaf = world.forest_rock_layout().unwrap().leaf_patches().unwrap()[0].position;
        world.edit_forest_rocks(&stock(on_leaf, 26)).unwrap();
        assert_eq!(world.animals().len(), 32);
        assert_eq!(world.edit_forest_rocks(&stock(on_leaf, 1)), Err(CoreError::ForestWoodlouseLimit));
        world.validate().unwrap();
    }

    #[test]
    fn r5_woodlouse_stock_undo_preserves_allocations_without_preserving_the_animals() {
        let mut world = WorldState::new_blank_r5_composed_forest(1206);
        let empty = world.forest_layout_edit_state().unwrap();
        let empty_ledger = world.forest_ledger();
        let add = ForestRockEdit::StockWoodlice { position: Point { x: 0.50, y: 0.80 }, count: 2 };
        world.edit_forest_rocks(&add).unwrap();
        assert!(!world.starter_community_available());
        assert_eq!(world.add_starter_community(), Err(CoreError::StarterCommunityUnavailable));
        let stocked = world.forest_layout_edit_state().unwrap();
        let stocked_ledger = world.forest_ledger();
        world.set_daylight_target(0.71).unwrap();
        world.restore_forest_layout_edit_state(empty.clone()).unwrap();
        assert!(world.animals().is_empty() && world.starter_community_available());
        assert_eq!(world.forest_ledger(), empty_ledger);
        assert_eq!(world.daylight_target(), Some(0.71));
        world.restore_forest_layout_edit_state(stocked.clone()).unwrap();
        assert_eq!(world.forest_layout_edit_state().unwrap(), stocked);
        assert_eq!(world.forest_ledger(), stocked_ledger);
        assert_eq!(world.daylight_target(), Some(0.71));
        let blueprint = world.fresh_habitat(1206).unwrap();
        assert!(blueprint.animals().is_empty() && blueprint.starter_community_available());

        world.restore_forest_layout_edit_state(empty).unwrap();
        let mut restored: WorldState = serde_json::from_value(serde_json::to_value(&world).unwrap()).unwrap();
        restored.validate().unwrap();
        restored.edit_forest_rocks(&add).unwrap();
        assert_eq!(restored.animals()[0].id, "isopod-3");
        assert_eq!(restored.animals()[1].id, "isopod-4");
        restored.validate().unwrap();
        world.add_starter_community().unwrap();
        assert_eq!(world.animals()[0].id, "isopod-3");
        assert_eq!(world.animals().last().unwrap().id, "isopod-6");
        world.validate().unwrap();
    }

    #[test]
    fn r5_leaf_history_restores_only_current_tick_edits_and_preserves_id_allocation() {
        let mut world = WorldState::new_r5_leaf_forest(42);
        let pristine_ledger = world.forest_ledger();
        let empty = world.forest_layout_edit_state().unwrap();
        let add = ForestRockEdit::AddLeafPatch { position: Point { x: 0.50, y: 0.615 }, preset: ForestLeafPreset::Heap };
        world.edit_forest_rocks(&add).unwrap();
        let placed = world.forest_rock_layout().unwrap().leaf_patches().unwrap()[0];
        let stocked_ledger = world.forest_ledger();
        let stocked = world.forest_layout_edit_state().unwrap();
        world.set_daylight_target(0.71).unwrap();

        world.restore_forest_layout_edit_state(empty.clone()).unwrap();
        assert_eq!(world.forest_ledger(), pristine_ledger);
        world.restore_forest_layout_edit_state(stocked.clone()).unwrap();
        assert_eq!(world.forest_ledger(), stocked_ledger);
        assert_eq!(world.forest_rock_layout().unwrap().leaf_patches().unwrap(), &[placed]);
        world.restore_forest_layout_edit_state(empty).unwrap();
        world.edit_forest_rocks(&add).unwrap();
        assert!(world.forest_rock_layout().unwrap().leaf_patches().unwrap()[0].id > placed.id);
        assert_eq!(world.daylight_target(), Some(0.71));

        world.advance_steps(1).unwrap();
        let progressed = world.clone();
        assert_eq!(world.restore_forest_layout_edit_state(stocked), Err(CoreError::InvalidForestState));
        assert_eq!(world, progressed);
    }

    #[test]
    fn same_inputs_match_across_step_batching() {
        let mut a = WorldState::new_fixture(42);
        let mut b = a.clone();
        a.advance_steps(500).unwrap();
        for _ in 0..25 {
            b.advance_steps(20).unwrap();
        }
        assert_eq!(a, b);
        assert_eq!(a.tick(), 500);
        a.validate().unwrap();
    }

    #[test]
    fn moving_fixture_rock_updates_the_two_moisture_zones_after_stepping() {
        let mut control = WorldState::new_fixture(42);
        let mut edited = control.clone();
        edited.edit_rock(Point { x: 0.7, y: 0.7 }).unwrap();
        assert_eq!(edited.moisture(), control.moisture());
        control.advance_steps(20).unwrap();
        edited.advance_steps(20).unwrap();
        assert!(edited.moisture()[0] < control.moisture()[0]);
        assert!(edited.moisture()[1] > control.moisture()[1]);
    }

    #[test]
    fn daylight_correction_across_midnight_is_continuous() {
        let mut world = WorldState::new_fixture(4);
        world.daylight.phase = 0.998;
        world.set_daylight_target(0.004).unwrap();
        assert_eq!(world.daylight_phase(), 0.998);
        world.advance_steps(1).unwrap();
        assert!(world.daylight_phase() > 0.998);
        world.advance_steps(4).unwrap();
        assert!(world.daylight_phase() < 0.01);
    }

    #[test]
    fn invalid_edits_do_not_mutate_the_fixture() {
        let mut world = WorldState::new_fixture(4);
        let original = world.clone();
        assert_eq!(world.set_daylight_target(f64::NAN), Err(CoreError::InvalidPhase));
        assert_eq!(world.edit_rock(Point { x: 0.5, y: -0.1 }), Err(CoreError::InvalidPoint));
        assert_eq!(world, original);
    }
}

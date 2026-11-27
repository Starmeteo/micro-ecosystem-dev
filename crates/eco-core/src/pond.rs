use crate::pond_float::{self, FloatingPlantLayout, PondFloatingCanopy, PondFloatingPlantObservation,
    PondFloatingPlantPreset, MAX_FLOATING_PLANTS};
use crate::pond_leaf::{self, PondLeafDepositObservation, PondLeafDepositPreset, PondLeafLayout};
use crate::pond_moss::{self, PondMossLayout, PondMossParent, PondMossPatchObservation, PondMossPreset, PondMossPresetObservation, MAX_MOSS_PATCHES};
use crate::pond_plant::{self, PondPlantLayout, PondRibbonPlantPreset, PondWaterPlantObservation};
use crate::pond_scenery::{PondScenery, PondSceneryLayout, PondSceneryParent};
use crate::pond_sedge::{MAX_SEDGE_PATCHES, PondSedgeObservation, PondSedgePreset, PondShoreAnchor, SedgeLayout};
use crate::pond_shape::{self, PondShape, PondShapeError, PondShapeMode, PondShapeObservation};
use crate::pond_stones::{self, animal_point, column_projection_px, PondBedProfile, PondShrimpStoneAttachment, PondStone, PondStoneLayout, PondStonePreset, PondSwimRange, MAX_STONES};
use crate::pond_wood::{hold_or_release, settle, PondShrimpAttachment, PondWood, PondWoodLayout, PondWoodPreset, MAX_WOODS};
use crate::{Activity, Animal, CoreError, Point, RandomState, SeasonObservation, STEP_MILLIS};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::f64::consts::TAU;

#[cfg(test)]
mod tests;
mod biofilm;
mod section;
mod section_resources;
mod shrimp_rows;
mod topology;

use biofilm::StoneBiofilm;
use section::PondStaticSection;
pub use biofilm::{PondBiofilmDistribution, PondBiofilmSurface, PondBiofilmSurfaceKind, PondStoneBiofilmObservation, PondWoodBiofilmObservation};
pub use section::PondStaticSectionObservation;

pub const POND_DEFAULT_BASIN_ID: &str = "pond-main";
pub(super) const MAX_BASINS: usize = 4;
const MAX_SHRIMP: usize = 12;
const MAX_DAPHNIA: u32 = 2048;
const MAX_DEPTH_CM: f64 = 12.0;
const MIN_WATER_LITERS: f64 = 0.05;
const MIN_AREA_CM2: f64 = 400.0;
const MAX_AREA_CM2: f64 = 10_000.0;
const INITIAL_AREA_CM2: f64 = 2400.0;
const OXYGEN_MG_PER_CARBON_MG: f64 = 32.0 / 12.0;
const STEP_SECONDS: f64 = STEP_MILLIS as f64 / 1000.0;
const STEP_HOURS: f64 = STEP_SECONDS / 3600.0;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase", deny_unknown_fields)]
pub enum PondEdit {
    AddWater { basin_id: String, liters: f64 },
    DrainWater { basin_id: String, liters: f64 },
    Reshape { basin_id: String, surface_area_cm2: f64 },
    PaintShape { basin_id: String, mode: PondShapeMode, cells: Vec<u16> },
    CommitTopology { basin_id: String, mode: PondShapeMode, cells: Vec<u16> },
    EditStaticSection { basin_id: String, row: u8, edit: Box<PondEdit> },
    AddDetritus { basin_id: String, mg_c: f64 },
    AddLeafDeposit { basin_id: String, x: f64, preset: PondLeafDepositPreset, mg_c: f64 },
    MoveLeafDeposit { basin_id: String, id: u32, x: f64 },
    RemoveLeafDeposit { basin_id: String, id: u32 },
    StockShrimp { basin_id: String, position: Point, count: u32 },
    RemoveShrimp { basin_id: String, id: String },
    StockDaphnia { basin_id: String, count: u32 },
    RemoveDaphnia { basin_id: String, count: u32 },
    AddStone { basin_id: String, x: f64, preset: PondStonePreset },
    MoveStone { basin_id: String, id: u32, x: f64 },
    RemoveStone { basin_id: String, id: u32 },
    EnableStoneRest { basin_id: String },
    EnableStoneBiofilm { basin_id: String },
    EnableWoodBiofilm { basin_id: String },
    AddWood { basin_id: String, x: f64, preset: PondWoodPreset },
    MoveWood { basin_id: String, id: u32, x: f64 },
    RemoveWood { basin_id: String, id: u32 },
    AddMoss { basin_id: String, parent: PondMossParent, offset: f64, preset: PondMossPreset },
    MoveMoss { basin_id: String, id: u32, offset: f64 },
    TrimMoss { basin_id: String, id: u32, mg_c: f64 },
    RemoveMoss { basin_id: String, id: u32 },
    AddWaterPlant { basin_id: String, x: f64 },
    PlaceRibbonPlant { basin_id: String, x: f64, preset: PondRibbonPlantPreset },
    MoveWaterPlant { basin_id: String, id: u32, x: f64 },
    TrimWaterPlant { basin_id: String, id: u32, mg_c: f64 },
    RemoveWaterPlant { basin_id: String, id: u32 },
    AddFloatingPlant { basin_id: String, position: Point, preset: PondFloatingPlantPreset },
    MoveFloatingPlant { basin_id: String, id: u32, position: Point },
    RemoveFloatingPlant { basin_id: String, id: u32 },
    AddSedge { basin_id: String, anchor: PondShoreAnchor, preset: PondSedgePreset },
    MoveSedge { basin_id: String, id: u32, anchor: PondShoreAnchor },
    RemoveSedge { basin_id: String, id: u32 },
    AddScenery { basin_id: String, style: String, x: f64, y: f64 },
    MoveScenery { basin_id: String, id: u32, x: f64, y: f64 },
    RemoveScenery { basin_id: String, id: u32 },
}

impl PondEdit {
    fn basin_id(&self) -> &str {
        match self {
            Self::AddWater { basin_id, .. } | Self::DrainWater { basin_id, .. }
            | Self::Reshape { basin_id, .. } | Self::AddDetritus { basin_id, .. }
            | Self::PaintShape { basin_id, .. } | Self::CommitTopology { basin_id, .. }
            | Self::EditStaticSection { basin_id, .. }
            | Self::AddLeafDeposit { basin_id, .. } | Self::MoveLeafDeposit { basin_id, .. } | Self::RemoveLeafDeposit { basin_id, .. }
            | Self::StockShrimp { basin_id, .. } | Self::RemoveShrimp { basin_id, .. }
            | Self::StockDaphnia { basin_id, .. } | Self::RemoveDaphnia { basin_id, .. }
            | Self::AddStone { basin_id, .. } | Self::MoveStone { basin_id, .. }
            | Self::RemoveStone { basin_id, .. } | Self::EnableStoneRest { basin_id, .. }
            | Self::EnableStoneBiofilm { basin_id, .. } | Self::EnableWoodBiofilm { basin_id, .. }
            | Self::AddWood { basin_id, .. }
            | Self::MoveWood { basin_id, .. } | Self::RemoveWood { basin_id, .. }
            | Self::AddMoss { basin_id, .. } | Self::MoveMoss { basin_id, .. }
            | Self::TrimMoss { basin_id, .. } | Self::RemoveMoss { basin_id, .. }
            | Self::AddWaterPlant { basin_id, .. } | Self::PlaceRibbonPlant { basin_id, .. } | Self::MoveWaterPlant { basin_id, .. }
            | Self::TrimWaterPlant { basin_id, .. } | Self::RemoveWaterPlant { basin_id, .. }
            | Self::AddFloatingPlant { basin_id, .. } | Self::MoveFloatingPlant { basin_id, .. }
            | Self::RemoveFloatingPlant { basin_id, .. }
            | Self::AddSedge { basin_id, .. } | Self::MoveSedge { basin_id, .. } | Self::RemoveSedge { basin_id, .. }
            | Self::AddScenery { basin_id, .. } | Self::MoveScenery { basin_id, .. }
            | Self::RemoveScenery { basin_id, .. } => basin_id,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PondObservation {
    pub basins: Vec<PondBasinObservation>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PondBasinObservation {
    pub id: String,
    pub label: String,
    pub wet: bool,
    pub water_liters: f64,
    pub capacity_liters: f64,
    pub depth_cm: f64,
    pub max_depth_cm: f64,
    pub surface_area_cm2: f64,
    pub temperature_celsius: Option<f64>,
    pub oxygen_mg_per_liter: Option<f64>,
    pub available_nitrogen_mg_per_liter: Option<f64>,
    pub available_phosphorus_mg_per_liter: Option<f64>,
    pub algae_mg_c: f64,
    pub biofilm_mg_c: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub biofilm_distribution: Option<PondBiofilmDistribution>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stone_biofilm_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stone_biofilm: Option<Vec<PondStoneBiofilmObservation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wood_biofilm_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wood_biofilm: Option<Vec<PondWoodBiofilmObservation>>,
    pub plant_mg_c: f64,
    pub detritus_mg_c: f64,
    pub daphnia_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub daphnia_stock_mg_cnp: Option<[f64; 3]>,
    pub shrimp: Vec<Animal>,
    pub shrimp_life: Vec<PondShrimpLifeObservation>,
    pub turbidity_index: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stones: Option<Vec<PondStone>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub woods: Option<Vec<PondWood>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub static_sections: Vec<PondStaticSectionObservation>,
    /// A previously used shrimp ID range stays attached to this row even when it is empty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retain_observation_row: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shrimp_attachments: Option<Vec<PondShrimpAttachment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wood_contact_feeding: Option<Vec<PondShrimpWoodFeedingObservation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stone_rest_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stone_attachments: Option<Vec<PondShrimpStoneAttachment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stone_contact_feeding: Option<Vec<PondShrimpStoneFeedingObservation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moss_patches: Option<Vec<PondMossPatchObservation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moss_presets: Option<[PondMossPresetObservation; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub water_plant_patches: Option<Vec<PondWaterPlantObservation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unassigned_plant_mg_cnp: Option<[f64; 3]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub floating_plants: Option<Vec<PondFloatingPlantObservation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub floating_canopy: Option<PondFloatingCanopy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shore_sedges: Option<Vec<PondSedgeObservation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenery: Option<Vec<PondScenery>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shape: Option<PondShapeObservation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leaf_deposits: Option<Vec<PondLeafDepositObservation>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PondShrimpStage {
    Juvenile,
    Adult,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PondShrimpLifeObservation {
    pub id: String,
    pub stage: PondShrimpStage,
    pub age_seconds: f64,
}

/// Actual intake while a shrimp was already in contact with wood. The two sources sum to the whole parcel.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PondShrimpWoodFeedingObservation {
    pub shrimp_id: String,
    pub wood_id: u32,
    pub offset: f64,
    pub biofilm_mg_c: f64,
    pub wood_biofilm_mg_c: f64,
    pub shared_biofilm_mg_c: f64,
}

/// Actual intake while a shrimp was already in contact with a main stone.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PondShrimpStoneFeedingObservation {
    pub shrimp_id: String,
    pub stone_id: u32,
    pub offset: f64,
    pub biofilm_mg_c: f64,
    pub stone_biofilm_mg_c: f64,
    pub shared_biofilm_mg_c: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PondBasinContactFeeding {
    pub wood: Vec<PondShrimpWoodFeedingObservation>,
    pub stone: Vec<PondShrimpStoneFeedingObservation>,
    pub sections: Vec<(u8, PondBasinContactFeeding)>,
}

impl PondBasinContactFeeding {
    fn is_empty(&self) -> bool { self.wood.is_empty() && self.stone.is_empty() && self.sections.is_empty() }
}

pub type PondContactFeeding = Vec<(String, PondBasinContactFeeding)>;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PondLedger {
    pub basins: Vec<PondBasinLedger>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PondBasinLedger {
    pub id: String,
    pub initial_water_liters: f64,
    pub current_water_liters: f64,
    pub water_input_liters: f64,
    pub water_output_liters: f64,
    pub water_residual_liters: f64,
    pub initial_matter_mg_cnp: [f64; 3],
    pub current_matter_mg_cnp: [f64; 3],
    pub matter_input_mg_cnp: [f64; 3],
    pub matter_output_mg_cnp: [f64; 3],
    /// Net internal C, N and P received through basin repartition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matter_topology_transfer_mg_cnp: Option<[f64; 3]>,
    pub matter_residual_mg_cnp: [f64; 3],
    pub initial_oxygen_mg: f64,
    pub current_oxygen_mg: f64,
    pub oxygen_input_mg: f64,
    pub oxygen_output_mg: f64,
    pub oxygen_residual_mg: f64,
    pub photosynthesis_oxygen_mg: f64,
    pub producer_respiration_oxygen_mg: f64,
    pub animal_respiration_oxygen_mg: f64,
    pub microbial_respiration_oxygen_mg: f64,
    pub gas_import_oxygen_mg: f64,
    pub gas_export_oxygen_mg: f64,
    pub shrimp_food_mg_c: f64,
    pub daphnia_food_mg_c: f64,
    pub decomposed_mg_c: f64,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub(super) struct Matter(pub(super) [f64; 3]);

impl Matter {
    pub(super) fn with_ratios(carbon: f64, nitrogen_ratio: f64, phosphorus_ratio: f64) -> Self {
        Self([carbon, carbon * nitrogen_ratio, carbon * phosphorus_ratio])
    }

    pub(super) fn carbon(self) -> f64 { self.0[0] }

    fn add(&mut self, addition: Self) {
        for (stock, flux) in self.0.iter_mut().zip(addition.0) {
            *stock += flux;
        }
    }

    pub(super) fn scaled(self, fraction: f64) -> Self {
        Self(self.0.map(|mass| mass * fraction))
    }

    pub(super) fn take_fraction(&mut self, fraction: f64) -> Self {
        let parcel = self.scaled(fraction);
        *self = self.scaled(1.0 - fraction);
        parcel
    }

    pub(super) fn take_carbon(&mut self, carbon: f64) -> Self {
        if self.carbon() == 0.0 || carbon == 0.0 {
            return Self::default();
        }
        self.take_fraction((carbon / self.carbon()).min(1.0))
    }

    fn minerals(self) -> Self { Self([0.0, self.0[1], self.0[2]]) }

    pub(super) fn valid(self) -> bool { self.0.into_iter().all(nonnegative) }

    fn is_zero(&self) -> bool { self.0 == [0.0; 3] }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Inventory {
    water_liters: f64,
    oxygen_mg: f64,
    cn_p: Matter,
}

impl Inventory {
    fn valid(self) -> bool {
        nonnegative(self.water_liters) && nonnegative(self.oxygen_mg) && self.cn_p.valid()
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Balance {
    initial: Inventory,
    inputs: Inventory,
    outputs: Inventory,
    // Signed transfers between basins, separate from inputs and outputs at the world boundary.
    #[serde(default, skip_serializing_if = "Matter::is_zero")]
    topology_matter_transfer: Matter,
    photosynthesis_oxygen_mg: f64,
    producer_respiration_oxygen_mg: f64,
    animal_respiration_oxygen_mg: f64,
    microbial_respiration_oxygen_mg: f64,
    gas_import_oxygen_mg: f64,
    gas_export_oxygen_mg: f64,
    shrimp_food_mg_c: f64,
    daphnia_food_mg_c: f64,
    decomposed_mg_c: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Shrimp {
    view: Animal,
    body: Matter,
    reserve: Matter,
    age_seconds: f64,
    oxygen_stress_seconds: f64,
    starvation_seconds: f64,
    birth_credit: f64,
}

const SHRIMP_MATURE_SECONDS: f64 = 36.0 * 3600.0;

impl Shrimp {
    fn starter_matter(adult: bool) -> (Matter, Matter) {
        let body = Matter::with_ratios(if adult { 18.0 } else { 7.0 }, 0.135, 0.018);
        let reserve = Matter::with_ratios(if adult { 3.5 } else { 1.0 }, 0.11, 0.012);
        (body, reserve)
    }

    fn new(id: String, x: f64, y: f64, spin: f64, tissue: Matter, reserve: Matter, age_seconds: f64) -> Self {
        Self {
            view: Animal { id, x, y, heading: spin, activity: Activity::Resting },
            body: tissue,
            reserve,
            age_seconds,
            oxygen_stress_seconds: 0.0,
            starvation_seconds: 0.0,
            birth_credit: 0.0,
        }
    }

    fn stage(&self) -> PondShrimpStage {
        if self.age_seconds < SHRIMP_MATURE_SECONDS { PondShrimpStage::Juvenile }
        else { PondShrimpStage::Adult }
    }

    fn observe_life(&self) -> PondShrimpLifeObservation {
        PondShrimpLifeObservation { id: self.view.id.clone(), stage: self.stage(), age_seconds: self.age_seconds }
    }

    fn valid_population(shrimp: &[Self], namespace: &str, next_id: u64) -> bool {
        if shrimp.len() > MAX_SHRIMP || next_id == 0 { return false; }
        if shrimp.is_empty() { return true; }
        let prefix = format!("{namespace}-shrimp-");
        let mut previous = 0;
        for individual in shrimp {
            if !individual.valid() { return false; }
            let Some(id) = individual.view.id.strip_prefix(&prefix).and_then(|suffix| suffix.parse::<u64>().ok()) else { return false; };
            if id <= previous || id >= next_id { return false; }
            previous = id;
        }
        true
    }

    fn valid(&self) -> bool {
        self.body.valid() && self.reserve.valid()
            && (0.0..=1.0).contains(&self.view.x) && (0.0..=1.0).contains(&self.view.y)
            && (0.0..TAU).contains(&self.view.heading)
            && [self.age_seconds, self.oxygen_stress_seconds, self.starvation_seconds].into_iter().all(nonnegative)
            && (0.0..=1.0).contains(&self.birth_credit)
    }
}

fn remove_shrimp(
    shrimp: &mut Vec<Shrimp>, stones: Option<&mut PondStoneLayout>, woods: Option<&mut PondWoodLayout>, id: &str,
) -> Result<Matter, CoreError> {
    let index = shrimp.iter().position(|individual| individual.view.id == id).ok_or(CoreError::PondShrimpMissing)?;
    let individual = shrimp.remove(index);
    if let Some(stones) = stones { stones.stone_attachments.retain(|attachment| attachment.shrimp_id != id); }
    if let Some(woods) = woods { woods.attachments.retain(|attachment| attachment.shrimp_id != id); }
    let mut outgoing = individual.body;
    outgoing.add(individual.reserve);
    Ok(outgoing)
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Daphnia {
    count: u32,
    body: Matter,
    reserve: Matter,
    birth_credit: f64,
    mortality_credit: f64,
}

impl Daphnia {
    fn new() -> Self {
        Self {
            count: 48,
            body: Matter::with_ratios(10.8, 0.14, 0.017),
            reserve: Matter::with_ratios(2.8, 0.11, 0.015),
            ..Self::default()
        }
    }

    fn valid(&self) -> bool {
        self.count <= MAX_DAPHNIA && self.body.valid() && self.reserve.valid()
            && (0.0..=1.0).contains(&self.birth_credit) && (0.0..1.0).contains(&self.mortality_credit)
            && (self.count != 0 || (self.body == Matter::default() && self.reserve == Matter::default()))
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Basin {
    id: String,
    surface_area_cm2: f64,
    water_liters: f64,
    temperature_celsius: f64,
    oxygen_mg: f64,
    minerals: Matter,
    algae: Matter,
    biofilm: Matter,
    plants: Matter,
    detritus: Matter,
    microbes: Matter,
    shrimp: Vec<Shrimp>,
    next_shrimp_id: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    shrimp_namespace: Option<String>,
    daphnia: Daphnia,
    random: RandomState,
    balance: Balance,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    stone_layout: Option<PondStoneLayout>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    stone_biofilm: Option<Vec<StoneBiofilm>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    wood_layout: Option<PondWoodLayout>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    moss_layout: Option<PondMossLayout>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    water_plant_layout: Option<PondPlantLayout>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    floating_plant_layout: Option<FloatingPlantLayout>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sedge_layout: Option<SedgeLayout>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    scenery_layout: Option<PondSceneryLayout>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    shape: Option<PondShape>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    observation_row: Option<u8>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    static_sections: Vec<PondStaticSection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    leaf_deposit_layout: Option<PondLeafLayout>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PondState {
    basins: Vec<Basin>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    next_basin_number: Option<u64>,
}

fn nonnegative(value: f64) -> bool { value.is_finite() && value >= 0.0 }

// Benson–Krause freshwater (0 salinity, 1 atmosphere), USGS WQ 2011.03, equation 7.
fn oxygen_saturation(celsius: f64) -> f64 {
    let t = celsius + 273.15;
    let inverse = 1.0 / t;
    let coefficient = 1.575_701e5 + inverse * (-6.642_308e7 + inverse * (1.243_800e10 - inverse * 8.621_949e11));
    (-139.34411 + inverse * coefficient).exp()
}

fn light_at(phase: f64) -> f64 { ((phase - 0.5) * TAU).cos().max(0.0) }

fn basin_id(index: usize) -> String {
    if index == 0 { POND_DEFAULT_BASIN_ID.to_owned() } else { format!("pond-{}", index + 1) }
}

fn stone_profiles(layout: Option<&PondStoneLayout>) -> impl Iterator<Item = PondBedProfile> + Clone + '_ {
    layout.into_iter().flat_map(PondStoneLayout::profiles)
}

fn wood_profiles(layout: Option<&PondWoodLayout>) -> impl Iterator<Item = PondBedProfile> + Clone + '_ {
    layout.into_iter().flat_map(PondWoodLayout::profiles)
}

fn valid_section_bed(stones: &PondStoneLayout, woods: Option<&PondWoodLayout>, shrimp: &[Shrimp], depth: f64) -> bool {
    let range = if stones.has_cave() { PondSwimRange::near_cave_floor(depth) }
        else if woods.is_some() { PondSwimRange::EXTENDED } else { PondSwimRange::STANDARD };
    let profiles = stones.profiles().chain(wood_profiles(woods));
    stones.valid()
        && shrimp.iter().all(|individual| {
            let point = animal_point(&individual.view);
            range.contains(point) && !profiles.clone().any(|profile| profile.blocks(point, depth))
        })
        && woods.is_none_or(|woods| {
            woods.valid() && !woods.profiles().any(|wood| stones.profiles().any(|stone| wood.overlaps(stone)))
                && woods.attachments.iter().all(|attachment| {
                    shrimp.iter().any(|individual| woods.attachment_matches(attachment, &individual.view, depth))
                })
        })
        && stones.stone_attachments.iter().all(|attachment| {
            shrimp.iter().any(|individual| attachment.matches(&stones.stones, &individual.view, depth))
                && woods.is_none_or(|woods| !woods.attachments.iter().any(|wood| wood.shrimp_id == attachment.shrimp_id))
        })
}

impl PondState {
    pub(super) fn has_wood_biofilm(&self) -> bool {
        self.basins.iter().any(Basin::has_wood_biofilm)
    }

    pub(super) fn new(seed: u64, basin_count: usize) -> Self {
        let basins = (0..basin_count).map(|index| {
            let mixed_seed = seed.wrapping_add((index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15));
            Basin::new(basin_id(index), mixed_seed)
        }).collect();
        Self { basins, next_basin_number: None }
    }

    pub(super) fn with_stones(seed: u64, basin_count: usize) -> Self {
        let mut pond = Self::new(seed, basin_count);
        for basin in &mut pond.basins {
            basin.stone_layout = Some(PondStoneLayout::new(false));
        }
        pond
    }

    pub(super) fn with_woods(seed: u64, basin_count: usize) -> Self {
        let mut pond = Self::with_stones(seed, basin_count);
        for basin in &mut pond.basins {
            basin.wood_layout = Some(PondWoodLayout::new());
        }
        pond
    }

    pub(super) fn with_moss(seed: u64, basin_count: usize) -> Self {
        let mut pond = Self::with_woods(seed, basin_count);
        for basin in &mut pond.basins {
            basin.moss_layout = Some(PondMossLayout::new());
        }
        pond
    }

    pub(super) fn with_scenery(seed: u64, basin_count: usize) -> Self {
        let mut pond = Self::with_moss(seed, basin_count);
        for basin in &mut pond.basins {
            let depth = basin.column_projection_px();
            let woods = basin.wood_layout.as_mut().expect("composed ponds have a wood layout");
            let wood_id = woods.seed_slope(0.235);
            basin.scenery_layout = Some(PondSceneryLayout::seeded(wood_id));
            let stones = basin.stone_layout.as_mut().expect("composed ponds have a stone layout");
            stones.stone_rest_enabled = true;
            stones.place(0.60, PondStonePreset::Main, depth, std::iter::empty(), woods.profiles())
                .expect("the composed pond's main stone clears its seeded log");
            woods.seed_wood_biofilm(Some(stones), &mut basin.biofilm);
            basin.clear_shrimp_after_depth_change();
        }
        pond
    }

    pub(super) fn clear_initial_basin(&mut self) {
        for basin in &mut self.basins {
            basin.clear_initial_stocks();
            basin.balance = Balance::default();
        }
    }

    pub(super) fn clear_initial_scenery(&mut self) {
        for basin in &mut self.basins {
            basin.stone_layout = Some(PondStoneLayout::new(true));
            if let Some(stocks) = &mut basin.stone_biofilm {
                for patch in stocks.drain(..) { basin.biofilm.add(patch.stock); }
            }
            if let Some(layout) = &mut basin.wood_layout {
                let enabled = layout.wood_biofilm.take().map(|stocks| {
                    for patch in stocks { basin.biofilm.add(patch.stock); }
                    Vec::new()
                });
                *layout = PondWoodLayout::new();
                layout.wood_biofilm = enabled;
            }
            basin.scenery_layout = Some(PondSceneryLayout::empty());
            basin.static_sections.clear();
        }
    }

    pub(super) fn fresh_habitat(&self, seed: u64, blank: bool) -> Result<Self, CoreError> {
        let mut preset = Self::new(seed, self.basins.len());
        preset.next_basin_number = self.next_basin_number;
        let climate = crate::season::observe(0);
        for (basin, source) in preset.basins.iter_mut().zip(&self.basins) {
            if basin.id != source.id {
                basin.id.clone_from(&source.id);
                for (index, shrimp) in basin.shrimp.iter_mut().enumerate() {
                    shrimp.view.id = format!("{}-shrimp-{}", basin.id, index + 1);
                }
            }
            basin.surface_area_cm2 = source.surface_area_cm2;
            basin.stone_layout = source.stone_layout.clone();
            if let Some(layout) = &mut basin.stone_layout {
                layout.stone_attachments.clear();
            }
            basin.wood_layout = source.wood_layout.clone();
            if let Some(layout) = &mut basin.wood_layout {
                layout.attachments.clear();
            }
            basin.moss_layout = source.moss_layout.clone();
            basin.scenery_layout = source.scenery_layout.clone();
            basin.shape = source.shape.clone();
            basin.observation_row = source.observation_row;
            basin.static_sections.clone_from(&source.static_sections);
            if let Some(layout) = &mut basin.moss_layout {
                layout.reset_stocks();
            }
            if blank || !source.wet() {
                basin.clear_initial_stocks();
                if source.wet() { basin.add_water(source.water_liters, climate)?; }
            } else {
                let volume_ratio = source.water_liters / basin.water_liters;
                basin.water_liters = source.water_liters;
                basin.oxygen_mg *= volume_ratio;
                basin.minerals = basin.minerals.scaled(volume_ratio);
            }
            basin.water_plant_layout = source.water_plant_layout.clone();
            if let Some(layout) = &mut basin.water_plant_layout {
                basin.plants.add(layout.reset_stocks(basin.plants.carbon()));
            }
            for plants in basin.static_sections.iter_mut().filter_map(|section| section.water_plants.as_mut()) {
                plants.stock = plants.layout.reset_stocks(0.0);
            }
            basin.floating_plant_layout = source.floating_plant_layout.clone();
            if let Some(layout) = &mut basin.floating_plant_layout {
                layout.reset_stocks();
            }
            basin.sedge_layout = source.sedge_layout.clone();
            if let Some(layout) = &mut basin.sedge_layout { layout.reset_stocks(); }
            if source.stone_biofilm.is_some() { basin.initialize_stone_biofilm(); }
            if let Some(woods) = basin.wood_layout.as_mut().filter(|layout| layout.wood_biofilm.is_some()) {
                woods.seed_wood_biofilm(basin.stone_layout.as_ref().filter(|_| basin.stone_biofilm.is_none()), &mut basin.biofilm);
            }
            for section in &mut basin.static_sections { section.reset_preset(&mut basin.biofilm); }
            basin.leaf_deposit_layout = source.leaf_deposit_layout.clone();
            for layout in basin.leaf_deposit_layout.iter_mut().chain(
                basin.static_sections.iter_mut().filter_map(|section| section.leaf_deposit_layout.as_mut()),
            ) {
                for dose in layout.reset_stocks() { basin.detritus.add(dose); }
            }
            basin.clear_shrimp_after_depth_change();
            basin.balance = Balance { initial: basin.inventory(), ..Balance::default() };
        }
        Ok(preset)
    }

    pub(super) fn starter_site_wet(&self) -> bool {
        self.basins.first().is_some_and(Basin::wet)
    }

    pub(super) fn starter_site_has_animals(&self) -> bool {
        self.basins.first().is_some_and(|basin| !basin.shrimp.is_empty() || basin.daphnia.count > 0
            || basin.static_sections.iter().any(|section| !section.shrimp.is_empty()))
    }

    pub(super) fn introduce_starter(&mut self) -> Result<(), CoreError> {
        if self.starter_site_has_animals() {
            return Err(CoreError::StarterCommunityUnavailable);
        }
        let basin = self.basins.first_mut().filter(|basin| basin.wet()).ok_or(CoreError::PondStarterNeedsWater)?;
        let before = basin.inventory().cn_p;
        basin.stock_starter();
        let after = basin.inventory().cn_p;
        basin.balance.inputs.cn_p.add(Matter(std::array::from_fn(|index| after.0[index] - before.0[index])));
        Ok(())
    }

    pub(super) fn reset_initial_balance(&mut self) {
        for basin in &mut self.basins {
            basin.balance = Balance { initial: basin.inventory(), ..Balance::default() };
        }
    }

    pub(super) fn preserve_bed_allocations(&mut self, current: &Self) -> Result<(), CoreError> {
        if self.next_basin_number.is_none() && current.next_basin_number.is_none()
            && self.basins.len() != current.basins.len() { return Err(CoreError::WrongWorldKind); }
        self.retain_topology_allocator(current);
        for prior in &mut self.basins {
            let Some(current) = current.basins.iter().find(|basin| basin.id == prior.id) else { continue; };
            match (&mut prior.stone_layout, &current.stone_layout) {
                (Some(prior), Some(current)) => prior.preserve_allocations(current),
                (None, None) => {},
                _ => return Err(CoreError::WrongWorldKind),
            }
            match (&mut prior.wood_layout, &current.wood_layout) {
                (Some(prior), Some(current)) => prior.preserve_allocations(current),
                (None, None) => {},
                _ => return Err(CoreError::WrongWorldKind),
            }
            match (&mut prior.moss_layout, &current.moss_layout) {
                (Some(prior), Some(current)) => prior.preserve_allocations(current),
                (None, None) => {},
                _ => return Err(CoreError::WrongWorldKind),
            }
            match (&mut prior.scenery_layout, &current.scenery_layout) {
                (Some(prior), Some(current)) => prior.preserve_allocations(current),
                (None, None) => {}
                _ => return Err(CoreError::WrongWorldKind),
            }
            if current.scenery_layout.is_some() {
                prior.next_shrimp_id = prior.next_shrimp_id.max(current.next_shrimp_id);
            }
            if let Some(current) = &current.water_plant_layout {
                let prior_carbon = prior.plants.carbon();
                prior.water_plant_layout.get_or_insert_with(|| PondPlantLayout::new(prior_carbon))
                    .preserve_allocations(current);
            }
            if let Some(current) = &current.floating_plant_layout {
                prior.floating_plant_layout.get_or_insert_with(FloatingPlantLayout::new).preserve_allocations(current);
            }
            if let Some(current) = &current.sedge_layout {
                prior.sedge_layout.get_or_insert_with(SedgeLayout::new).preserve_allocations(current);
            }
            if let Some(current) = &current.leaf_deposit_layout {
                prior.leaf_deposit_layout.get_or_insert_with(PondLeafLayout::new).preserve_allocations(current);
            }
        }
        Ok(())
    }

    pub(super) fn validate(&self, permits_stones: bool, permits_woods: bool, permits_moss: bool, permits_scenery: bool) -> Result<(), CoreError> {
        if !(1..=MAX_BASINS).contains(&self.basins.len()) || !self.valid_basin_identities(permits_scenery)
            || !self.valid_shrimp_namespaces(permits_scenery)
            || !self.basins.iter().all(|basin| {
            basin.stone_layout.is_some() == permits_stones
                && basin.wood_layout.is_some() == permits_woods && basin.moss_layout.is_some() == permits_moss
                && basin.scenery_layout.is_some() == permits_scenery
                && (permits_scenery || (basin.water_plant_layout.is_none() && basin.floating_plant_layout.is_none() && basin.stone_biofilm.is_none()
                    && !basin.has_wood_biofilm() && basin.shape.is_none() && basin.leaf_deposit_layout.is_none()
                    && basin.sedge_layout.is_none() && basin.static_sections.is_empty()))
                && basin.valid()
        }) {
            return Err(CoreError::InvalidPondState);
        }
        if self.basins.iter().any(|basin| basin.shape.is_some()) && !self.valid_shapes() {
            return Err(CoreError::InvalidPondState);
        }
        for element in 0..3 {
            let transfers = self.basins.iter().map(|basin| basin.balance.topology_matter_transfer.0[element]);
            let net = transfers.clone().sum::<f64>();
            let magnitude = transfers.map(f64::abs).sum::<f64>();
            if !magnitude.is_finite() || net.abs() > 1e-7 + 1e-9 * magnitude {
                return Err(CoreError::InvalidPondState);
            }
        }
        Ok(())
    }

    fn valid_shrimp_namespaces(&self, permits_sections: bool) -> bool {
        let mut used = std::collections::HashSet::new();
        for basin in &self.basins {
            if (!permits_sections && basin.shrimp_namespace.is_some()) || !used.insert(Cow::Borrowed(basin.shrimp_namespace())) { return false; }
            for section in &basin.static_sections {
                if !used.insert(section.shrimp_namespace_for(&basin.id)) { return false; }
            }
        }
        used.into_iter().all(|name| !name.is_empty() && name.len() <= 64 && name.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-'))
    }

    fn valid_shapes(&self) -> bool {
        let basin_count = self.basins.len();
        let mut shapes: Vec<Cow<'_, PondShape>> = Vec::with_capacity(basin_count);
        for (index, basin) in self.basins.iter().enumerate() {
            let row = basin.observation_row(index, basin_count);
            let shape = basin.effective_shape(row);
            if !shape.valid(row, basin.surface_area_cm2) || shapes.iter().any(|other| shape.touches(other)) {
                return false;
            }
            shapes.push(shape);
        }
        true
    }

    pub(super) fn advance(&mut self, phase: f64, climate: SeasonObservation, mut contact_feeding: Option<&mut PondContactFeeding>) {
        let light = light_at(phase);
        for basin in &mut self.basins {
            if let Some(feedback) = contact_feeding.as_deref_mut() {
                let mut entries = PondBasinContactFeeding::default();
                basin.advance(light, climate, Some(&mut entries));
                if !entries.is_empty() {
                    feedback.push((basin.id.clone(), entries));
                }
            } else {
                basin.advance(light, climate, None);
            }
        }
    }

    pub(super) fn edit(&mut self, edit: &PondEdit, climate: SeasonObservation) -> Result<(), CoreError> {
        let index = self.basins.iter().position(|basin| basin.id == edit.basin_id()).ok_or(CoreError::PondBasinMissing)?;
        let row = self.basins[index].observation_row(index, self.basins.len());
        if let PondEdit::AddSedge { anchor, .. } | PondEdit::MoveSedge { anchor, .. } = edit {
            if self.basins.iter().enumerate().any(|(other_index, basin)| {
                other_index != index && basin.sedge_layout.as_ref().is_some_and(|layout| layout.uses_shore(anchor.shore_cell))
            }) {
                return Err(CoreError::InvalidPondSedgeSite);
            }
        }
        let basin = &mut self.basins[index];
        match *edit {
            PondEdit::AddWater { liters, .. } => basin.add_water(liters, climate),
            PondEdit::DrainWater { liters, .. } => basin.drain(liters),
            PondEdit::Reshape { surface_area_cm2, .. } => basin.reshape(surface_area_cm2),
            PondEdit::PaintShape { mode, ref cells, .. } => self.paint_shape(index, mode, cells),
            PondEdit::CommitTopology { mode, ref cells, .. } => self.commit_topology(index, mode, cells),
            PondEdit::EditStaticSection { row, ref edit, .. } => basin.edit_static_section(row, edit, climate),
            PondEdit::AddDetritus { mg_c, .. } => basin.add_detritus(mg_c),
            PondEdit::AddLeafDeposit { x, preset, mg_c, .. } => basin.add_leaf_deposit(x, preset, mg_c),
            PondEdit::MoveLeafDeposit { id, x, .. } => basin.move_leaf_deposit(id, x),
            PondEdit::RemoveLeafDeposit { id, .. } => {
                if basin.scenery_layout.is_none() { return Err(CoreError::WrongWorldKind); }
                let layout = basin.leaf_deposit_layout.as_mut().ok_or(CoreError::PondLeafDepositMissing)?;
                let removed = layout.remove_deposit(id, &mut basin.detritus)?;
                basin.balance.outputs.cn_p.add(removed);
                Ok(())
            }
            PondEdit::StockShrimp { position, count, .. } => basin.stock_shrimp(position, count),
            PondEdit::RemoveShrimp { ref id, .. } => {
                if basin.scenery_layout.is_none() { return Err(CoreError::WrongWorldKind); }
                let outgoing = remove_shrimp(&mut basin.shrimp, basin.stone_layout.as_mut(), basin.wood_layout.as_mut(), id)?;
                basin.balance.outputs.cn_p.add(outgoing);
                Ok(())
            }
            PondEdit::StockDaphnia { count, .. } => basin.stock_daphnia(count),
            PondEdit::RemoveDaphnia { count, .. } => basin.remove_daphnia(count),
            PondEdit::AddStone { x, preset, .. } => {
                if preset == PondStonePreset::FlatStoneCave && basin.scenery_layout.is_none() {
                    return Err(CoreError::WrongWorldKind);
                }
                basin.check_water_plant_clearance(PondStone { id: 0, x, preset }.profile())?;
                let depth = basin.column_projection_px();
                let layout = basin.stone_layout.as_mut().ok_or(CoreError::WrongWorldKind)?;
                let animals = basin.shrimp.iter().map(|shrimp| animal_point(&shrimp.view));
                layout.place(x, preset, depth, animals, wood_profiles(basin.wood_layout.as_ref()))?;
                if let Some((stocks, stone)) = basin.stone_biofilm.as_mut().zip(layout.stones.last())
                    .filter(|(_, stone)| stone.preset == PondStonePreset::Main) {
                    stocks.push(StoneBiofilm::empty(stone.id));
                }
                Ok(())
            }
            PondEdit::MoveStone { id, x, .. } => {
                if let Some(stone) = basin.stone_layout.iter().flat_map(|layout| &layout.stones).find(|stone| stone.id == id) {
                    basin.check_water_plant_clearance(PondStone { x, ..*stone }.profile())?;
                }
                let depth = basin.column_projection_px();
                let layout = basin.stone_layout.as_mut().ok_or(CoreError::WrongWorldKind)?;
                let animals = basin.shrimp.iter().map(|shrimp| animal_point(&shrimp.view));
                layout.move_stone(id, x, depth, animals, wood_profiles(basin.wood_layout.as_ref()))
            }
            PondEdit::RemoveStone { id, .. } => {
                let layout = basin.stone_layout.as_mut().ok_or(CoreError::WrongWorldKind)?;
                let removing_cave = layout.stones.iter().any(|stone| stone.id == id && stone.preset == PondStonePreset::FlatStoneCave);
                layout.remove(id)?;
                if let Some(stocks) = &mut basin.stone_biofilm {
                    if let Some(index) = stocks.iter().position(|patch| patch.stone_id == id) {
                        basin.biofilm.add(stocks.remove(index).stock);
                    }
                }
                basin.drop_moss(PondMossParent::Stone { id });
                if removing_cave { basin.clear_shrimp_after_depth_change(); }
                Ok(())
            }
            PondEdit::EnableStoneRest { .. } => {
                if basin.scenery_layout.is_none() { return Err(CoreError::WrongWorldKind); }
                basin.stone_layout.as_mut().ok_or(CoreError::WrongWorldKind)?.stone_rest_enabled = true;
                Ok(())
            }
            PondEdit::EnableStoneBiofilm { .. } => {
                if basin.scenery_layout.is_none() { return Err(CoreError::WrongWorldKind); }
                basin.stone_layout.as_mut().ok_or(CoreError::WrongWorldKind)?.stone_rest_enabled = true;
                if basin.stone_biofilm.is_none() { basin.initialize_stone_biofilm(); }
                Ok(())
            }
            PondEdit::EnableWoodBiofilm { .. } => {
                if basin.scenery_layout.is_none() { return Err(CoreError::WrongWorldKind); }
                let layout = basin.wood_layout.as_mut().ok_or(CoreError::WrongWorldKind)?;
                if layout.wood_biofilm.is_none() {
                    layout.seed_wood_biofilm(basin.stone_layout.as_ref().filter(|_| basin.stone_biofilm.is_none()), &mut basin.biofilm);
                }
                Ok(())
            }
            PondEdit::AddWood { x, preset, .. } => {
                if preset.requires_composed_scene() && basin.scenery_layout.is_none() {
                    return Err(CoreError::WrongWorldKind);
                }
                basin.check_water_plant_clearance(PondWood { id: 0, x, preset }.profile())?;
                let depth = basin.column_projection_px();
                let layout = basin.wood_layout.as_mut().ok_or(CoreError::WrongWorldKind)?;
                let animals = basin.shrimp.iter().map(|shrimp| animal_point(&shrimp.view));
                layout.place(x, preset, depth, animals, stone_profiles(basin.stone_layout.as_ref()))
            }
            PondEdit::MoveWood { id, x, .. } => basin.move_wood(id, x),
            PondEdit::RemoveWood { id, .. } => {
                let returned = basin.wood_layout.as_mut().ok_or(CoreError::WrongWorldKind)?.remove(id)?;
                if returned != Matter::default() { basin.biofilm.add(returned); }
                basin.drop_moss(PondMossParent::Wood { id });
                if let Some(scenery) = &mut basin.scenery_layout {
                    scenery.remove_children(PondSceneryParent::Wood { id });
                }
                Ok(())
            }
            PondEdit::AddMoss { parent, offset, preset, .. } => basin.add_moss(parent, offset, preset),
            PondEdit::MoveMoss { id, offset, .. } => basin.moss_layout.as_mut().ok_or(CoreError::WrongWorldKind)?.move_patch(id, offset),
            PondEdit::TrimMoss { id, mg_c, .. } => {
                let trimmings = basin.moss_layout.as_mut().ok_or(CoreError::WrongWorldKind)?.trim(id, mg_c)?;
                basin.detritus.add(trimmings);
                Ok(())
            }
            PondEdit::RemoveMoss { id, .. } => {
                let removed = basin.moss_layout.as_mut().ok_or(CoreError::WrongWorldKind)?.remove(id)?;
                basin.detritus.add(removed);
                Ok(())
            }
            PondEdit::AddWaterPlant { x, .. } => basin.add_water_plant(x, None),
            PondEdit::PlaceRibbonPlant { x, preset, .. } => basin.add_water_plant(x, Some(preset)),
            PondEdit::MoveWaterPlant { id, x, .. } => basin.move_water_plant(id, x),
            PondEdit::TrimWaterPlant { id, mg_c, .. } => basin.transfer_water_plant(id, Some(mg_c)),
            PondEdit::RemoveWaterPlant { id, .. } => basin.transfer_water_plant(id, None),
            PondEdit::AddFloatingPlant { position, preset, .. } => basin.add_floating_plant(row, position, preset),
            PondEdit::MoveFloatingPlant { id, position, .. } => basin.move_floating_plant(id, position),
            PondEdit::RemoveFloatingPlant { id, .. } => basin.remove_floating_plant(id),
            PondEdit::AddSedge { anchor, preset, .. } => basin.add_sedge(row, anchor, preset),
            PondEdit::MoveSedge { id, anchor, .. } => basin.move_sedge(id, anchor),
            PondEdit::RemoveSedge { id, .. } => basin.remove_sedge(id),
            PondEdit::AddScenery { ref style, x, y, .. } => {
                basin.scenery_layout.as_mut().ok_or(CoreError::WrongWorldKind)?.add(style, x, y)
            }
            PondEdit::MoveScenery { id, x, y, .. } => {
                basin.scenery_layout.as_mut().ok_or(CoreError::WrongWorldKind)?.move_item(id, x, y)
            }
            PondEdit::RemoveScenery { id, .. } => {
                basin.scenery_layout.as_mut().ok_or(CoreError::WrongWorldKind)?.remove(id)
            }
        }
    }

    fn paint_shape(&mut self, index: usize, mode: PondShapeMode, cells: &[u16]) -> Result<(), CoreError> {
        let basin = &self.basins[index];
        if basin.scenery_layout.is_none() { return Err(CoreError::WrongWorldKind); }
        let basin_count = self.basins.len();
        let row = basin.observation_row(index, basin_count);
        let secondary_rows = basin.static_sections.iter().map(|section| section.row);
        let shape = basin.effective_shape(row).paint(mode, cells, row, secondary_rows).map_err(CoreError::PondShape)?;
        if basin.sedge_layout.as_ref().is_some_and(|layout| !layout.anchors_valid(&shape)) {
            return Err(CoreError::PondShape(PondShapeError::TopologySedgeAnchor));
        }
        if self.basins.iter().enumerate().any(|(other_index, other)| {
            other_index != index && shape.touches(&other.effective_shape(other.observation_row(other_index, basin_count)))
        }) {
            return Err(CoreError::PondShape(PondShapeError::TouchesOther));
        }
        // Changing the footprint uses the current volume and keeps every stock.
        // The fixed row keeps bed anchors legal; changing depth rechecks swimmers.
        let mut candidate = basin.clone();
        candidate.reshape(shape.area()).map_err(|error| CoreError::PondShape(match error {
            CoreError::PondCapacityExceeded => PondShapeError::Capacity,
            _ => PondShapeError::AreaOutOfRange,
        }))?;
        candidate.shape = Some(shape);
        if !candidate.valid() { return Err(CoreError::PondShape(PondShapeError::Occupied)); }
        self.basins[index] = candidate;
        Ok(())
    }

    pub(super) fn observe(&self) -> PondObservation {
        let basins = self.basins.iter().enumerate().map(|(index, basin)| {
            let mut observation = basin.observe(index);
            if self.next_basin_number.is_some() {
                observation.label = if basin.id == POND_DEFAULT_BASIN_ID { "主水域".to_owned() }
                    else { format!("水域 {}", basin.id.trim_start_matches("pond-")) };
            }
            if basin.scenery_layout.is_some() {
                let row = basin.observation_row(index, self.basins.len());
                observation.shape = Some(basin.effective_shape(row).observe(row));
            }
            observation
        }).collect();
        PondObservation { basins }
    }

    pub(super) fn ledger(&self) -> PondLedger {
        PondLedger { basins: self.basins.iter().map(Basin::ledger).collect() }
    }
}

impl Basin {
    fn new(id: String, seed: u64) -> Self {
        let mut basin = Self {
            id,
            surface_area_cm2: INITIAL_AREA_CM2,
            water_liters: 18.0,
            temperature_celsius: 21.0,
            oxygen_mg: 18.0 * 7.4,
            minerals: Matter([0.0, 6.3, 0.9]),
            algae: Matter::default(),
            biofilm: Matter::default(),
            plants: Matter::default(),
            detritus: Matter::default(),
            microbes: Matter::default(),
            shrimp: Vec::new(),
            next_shrimp_id: 1,
            shrimp_namespace: None,
            daphnia: Daphnia::default(),
            random: RandomState::new(seed),
            balance: Balance::default(),
            stone_layout: None,
            stone_biofilm: None,
            wood_layout: None,
            moss_layout: None,
            water_plant_layout: None,
            floating_plant_layout: None,
            sedge_layout: None,
            scenery_layout: None,
            shape: None,
            observation_row: None,
            static_sections: Vec::new(),
            leaf_deposit_layout: None,
        };
        basin.stock_starter();
        basin.balance.initial = basin.inventory();
        basin
    }

    fn stock_starter(&mut self) {
        self.algae.add(Matter::with_ratios(30.0, 0.088, 0.012));
        self.biofilm.add(Matter::with_ratios(24.0, 0.077, 0.010));
        let starter_plants = Matter::with_ratios(150.0, 0.06, 0.007);
        if let Some(layout) = &mut self.water_plant_layout {
            layout.add_unassigned_stock(self.plants.carbon(), starter_plants.carbon());
        }
        self.plants.add(starter_plants);
        self.detritus.add(pond_leaf::detritus_matter(120.0));
        self.microbes.add(Matter::with_ratios(10.0, 0.10, 0.012));
        self.daphnia = Daphnia::new();
        for index in 0..3 {
            let x = 0.24 + 0.24 * index as f64;
            let y = 0.55 + 0.22 * self.random.unit();
            let heading = self.random.unit() * TAU;
            let adult = index != 1;
            let (body, reserve) = Shrimp::starter_matter(adult);
            let id = self.allocate_shrimp_id();
            self.shrimp.push(Shrimp::new(id, x, y, heading, body, reserve, if adult { SHRIMP_MATURE_SECONDS } else { 0.0 }));
        }
        self.clear_shrimp_after_depth_change();
    }

    fn clear_initial_stocks(&mut self) {
        self.water_liters = 0.0;
        self.oxygen_mg = 0.0;
        self.minerals = Matter::default();
        self.algae = Matter::default();
        self.biofilm = Matter::default();
        if let Some(stocks) = &mut self.stone_biofilm {
            for patch in stocks { patch.stock = Matter::default(); }
        }
        self.plants = Matter::default();
        self.water_plant_layout = None;
        self.floating_plant_layout = None;
        self.sedge_layout = None;
        self.detritus = Matter::default();
        self.leaf_deposit_layout = None;
        self.microbes = Matter::default();
        self.shrimp.clear();
        if let Some(layout) = &mut self.stone_layout {
            layout.stone_attachments.clear();
        }
        if let Some(layout) = &mut self.wood_layout {
            layout.attachments.clear();
            for patch in layout.wood_biofilm.iter_mut().flatten() { patch.stock = Matter::default(); }
        }
        self.next_shrimp_id = 1;
        self.daphnia = Daphnia::default();
    }

    fn allocate_shrimp_id(&mut self) -> String {
        let next = self.next_shrimp_id;
        self.next_shrimp_id += 1;
        format!("{}-shrimp-{next}", self.shrimp_namespace())
    }

    fn shrimp_namespace(&self) -> &str { self.shrimp_namespace.as_deref().unwrap_or(&self.id) }

    fn wet(&self) -> bool { self.water_liters > 0.0 }

    fn capacity(&self) -> f64 { self.surface_area_cm2 * MAX_DEPTH_CM / 1000.0 }

    fn observation_row(&self, index: usize, basin_count: usize) -> usize {
        self.observation_row.map_or_else(|| pond_shape::protected_row(index, basin_count), usize::from)
    }

    fn effective_shape(&self, protected_row: usize) -> Cow<'_, PondShape> {
        self.shape.as_ref().map_or_else(
            || Cow::Owned(PondShape::seed(protected_row, self.surface_area_cm2)),
            Cow::Borrowed,
        )
    }

    fn depth_cm(&self) -> f64 { self.water_liters * 1000.0 / self.surface_area_cm2 }

    fn column_projection_px(&self) -> f64 { column_projection_px(self.depth_cm(), MAX_DEPTH_CM) }

    fn move_wood(&mut self, id: u32, x: f64) -> Result<(), CoreError> {
        if let Some(wood) = self.wood_layout.iter().flat_map(|layout| &layout.woods).find(|wood| wood.id == id) {
            self.check_water_plant_clearance(PondWood { x, ..*wood }.profile())?;
        }
        let depth = self.column_projection_px();
        let layout = self.wood_layout.as_mut().ok_or(CoreError::WrongWorldKind)?;
        let resting = self.shrimp.iter().filter(|shrimp| layout.attachments.iter().any(|attached| {
            attached.wood_id == id && attached.shrimp_id == shrimp.view.id
        })).map(|shrimp| shrimp.view.id.as_str()).collect::<Vec<_>>();
        let free_swimmers = self.shrimp.iter().filter(|shrimp| !resting.contains(&shrimp.view.id.as_str()))
            .map(|shrimp| animal_point(&shrimp.view));
        layout.move_wood(id, x, depth, free_swimmers, stone_profiles(self.stone_layout.as_ref()))?;
        self.sync_wood_attachments(depth);
        Ok(())
    }

    fn add_moss(&mut self, parent: PondMossParent, offset: f64, preset: PondMossPreset) -> Result<(), CoreError> {
        let depth = self.moss_column_px();
        let layout = self.moss_layout.as_mut().ok_or(CoreError::WrongWorldKind)?;
        let support = pond_moss::support(parent, offset, self.stone_layout.as_ref(), self.wood_layout.as_ref())?;
        if !support.submerged(depth) { return Err(CoreError::InvalidPondMossSite); }
        let stock = layout.place(parent, offset, preset)?;
        self.balance.inputs.cn_p.add(stock);
        Ok(())
    }

    fn drop_moss(&mut self, parent: PondMossParent) {
        if let Some(stock) = self.moss_layout.as_mut().and_then(|layout| layout.remove_with_parent(parent)) {
            self.detritus.add(stock);
        }
    }

    fn ground_profiles(&self) -> impl Iterator<Item = PondBedProfile> + Clone + '_ {
        stone_profiles(self.stone_layout.as_ref()).chain(wood_profiles(self.wood_layout.as_ref()))
    }

    fn check_water_plant_clearance(&self, profile: PondBedProfile) -> Result<(), CoreError> {
        if self.water_plant_layout.as_ref().is_some_and(|layout| layout.blocks_structure(profile)) {
            return Err(CoreError::PondWaterPlantSiteOccupied);
        }
        Ok(())
    }

    fn add_water_plant(&mut self, x: f64, ribbon_preset: Option<PondRibbonPlantPreset>) -> Result<(), CoreError> {
        if self.scenery_layout.is_none() { return Err(CoreError::WrongWorldKind); }
        if !self.wet() { return Err(CoreError::PondWaterPlantNeedsWater); }
        pond_plant::check_root(x, self.ground_profiles())?;
        let carbon = self.plants.carbon();
        let mut layout = self.water_plant_layout.clone().unwrap_or_else(|| PondPlantLayout::new(carbon));
        let stock = layout.place(x, carbon, ribbon_preset)?;
        self.water_plant_layout = Some(layout);
        self.plants.add(stock);
        self.balance.inputs.cn_p.add(stock);
        Ok(())
    }

    fn water_plants_mut(&mut self) -> Result<&mut PondPlantLayout, CoreError> {
        if self.scenery_layout.is_none() { return Err(CoreError::WrongWorldKind); }
        self.water_plant_layout.as_mut().ok_or(CoreError::PondWaterPlantMissing)
    }

    fn move_water_plant(&mut self, id: u32, x: f64) -> Result<(), CoreError> {
        if self.scenery_layout.is_none() { return Err(CoreError::WrongWorldKind); }
        pond_plant::check_root(x, self.ground_profiles())?;
        self.water_plants_mut()?.move_patch(id, x)
    }

    fn transfer_water_plant(&mut self, id: u32, mg_c: Option<f64>) -> Result<(), CoreError> {
        let carbon = self.plants.carbon();
        let layout = self.water_plants_mut()?;
        let fraction = match mg_c {
            Some(mg_c) => layout.trim(id, mg_c, carbon)?,
            None => layout.remove(id, carbon)?,
        };
        self.detritus.add(self.plants.take_fraction(fraction));
        Ok(())
    }

    fn add_floating_plant(&mut self, row: usize, position: Point, preset: PondFloatingPlantPreset) -> Result<(), CoreError> {
        if self.scenery_layout.is_none() { return Err(CoreError::WrongWorldKind); }
        if !self.wet() { return Err(CoreError::PondFloatingPlantNeedsWater); }
        pond_float::check_surface(position, &self.effective_shape(row))?;
        let stock = self.floating_plant_layout.get_or_insert_with(FloatingPlantLayout::new).place(position, preset)?;
        self.shape.get_or_insert_with(|| PondShape::seed(row, self.surface_area_cm2));
        self.balance.inputs.cn_p.add(stock);
        Ok(())
    }

    fn move_floating_plant(&mut self, id: u32, position: Point) -> Result<(), CoreError> {
        if self.scenery_layout.is_none() { return Err(CoreError::WrongWorldKind); }
        let layout = self.floating_plant_layout.as_mut().ok_or(CoreError::PondFloatingPlantMissing)?;
        pond_float::check_surface(position, self.shape.as_ref().ok_or(CoreError::PondFloatingPlantMissing)?)?;
        layout.move_patch(id, position)
    }

    fn remove_floating_plant(&mut self, id: u32) -> Result<(), CoreError> {
        if self.scenery_layout.is_none() { return Err(CoreError::WrongWorldKind); }
        let layout = self.floating_plant_layout.as_mut().ok_or(CoreError::PondFloatingPlantMissing)?;
        self.detritus.add(layout.remove(id)?);
        Ok(())
    }

    fn floating_canopy(&self) -> Option<PondFloatingCanopy> {
        let layout = self.floating_plant_layout.as_ref().filter(|layout| layout.has_patches())?;
        self.shape.as_ref().map(|shape| layout.canopy(shape, self.wet()))
    }

    fn add_sedge(&mut self, row: usize, anchor: PondShoreAnchor, preset: PondSedgePreset) -> Result<(), CoreError> {
        if self.scenery_layout.is_none() { return Err(CoreError::WrongWorldKind); }
        let shape = self.effective_shape(row).into_owned();
        if !anchor.valid(&shape) { return Err(CoreError::InvalidPondSedgeSite); }
        let mut layout = self.sedge_layout.clone().unwrap_or_else(SedgeLayout::new);
        let stock = layout.place(anchor, preset)?;
        self.balance.inputs.cn_p.add(stock);
        self.sedge_layout = Some(layout);
        self.shape.get_or_insert(shape);
        Ok(())
    }

    fn move_sedge(&mut self, id: u32, anchor: PondShoreAnchor) -> Result<(), CoreError> {
        if !self.shape.as_ref().is_some_and(|shape| anchor.valid(shape)) { return Err(CoreError::InvalidPondSedgeSite); }
        self.sedge_layout.as_mut().ok_or(CoreError::PondSedgeMissing)?.relocate(id, anchor)
    }

    fn remove_sedge(&mut self, id: u32) -> Result<(), CoreError> {
        let stock = self.sedge_layout.as_mut().ok_or(CoreError::PondSedgeMissing)?.remove(id)?;
        self.balance.outputs.cn_p.add(stock);
        Ok(())
    }

    fn moss_column_px(&self) -> f64 { if self.wet() { self.column_projection_px() } else { 0.0 } }

    fn submerged_moss(&self) -> [bool; MAX_MOSS_PATCHES] {
        let mut submerged = [false; MAX_MOSS_PATCHES];
        let Some(layout) = &self.moss_layout else { return submerged; };
        let depth = self.moss_column_px();
        for (patch, wet) in layout.patches.iter().zip(&mut submerged) {
            *wet = pond_moss::support(patch.parent, patch.offset, self.stone_layout.as_ref(), self.wood_layout.as_ref())
                .is_ok_and(|support| support.submerged(depth));
        }
        submerged
    }

    fn sync_wood_attachments(&mut self, depth: f64) {
        if let Some(woods) = &mut self.wood_layout { shrimp_rows::sync_wood_attachments(&mut self.shrimp, woods, depth); }
    }

    fn clear_shrimp_after_depth_change(&mut self) {
        if self.stone_layout.is_none() { return; }
        let depth = self.column_projection_px();
        shrimp_rows::project_after_depth_change(&mut self.shrimp, self.stone_layout.as_mut().expect("checked stone layout"), self.wood_layout.as_mut(), depth);
        for section in &mut self.static_sections {
            shrimp_rows::project_after_depth_change(&mut section.shrimp, &mut section.stones, Some(&mut section.woods), depth);
        }
    }

    fn concentration(&self, stock_mg: f64) -> Option<f64> {
        self.wet().then(|| stock_mg / self.water_liters)
    }

    fn turbidity(&self) -> f64 {
        let suspended_mg_c_per_liter = self.concentration(self.algae.carbon()).unwrap_or_default();
        suspended_mg_c_per_liter / (suspended_mg_c_per_liter + 5.0)
    }

    fn observe(&self, index: usize) -> PondBasinObservation {
        let turbidity = self.turbidity();
        let moss_patches = self.moss_layout.as_ref().map(|layout| {
            let depth = self.moss_column_px();
            layout.patches.iter().filter_map(|patch| {
                let support = pond_moss::support(patch.parent, patch.offset, self.stone_layout.as_ref(), self.wood_layout.as_ref()).ok()?;
                Some(patch.observe(support, depth))
            }).collect()
        });
        let (water_plant_patches, unassigned_plant_mg_cnp) = if self.scenery_layout.is_some() {
            let (patches, unassigned) = self.water_plant_layout.as_ref().map_or_else(
                || (Vec::new(), self.plants.0),
                |layout| layout.observe(self.plants, self.wet(), self.depth_cm(), turbidity),
            );
            (Some(patches), Some(unassigned))
        } else {
            (None, None)
        };
        PondBasinObservation {
            id: self.id.clone(),
            label: if index == 0 { "主水域".to_owned() } else { format!("水域 {}", index + 1) },
            wet: self.wet(),
            water_liters: self.water_liters,
            capacity_liters: self.capacity(),
            depth_cm: self.depth_cm(),
            max_depth_cm: MAX_DEPTH_CM,
            surface_area_cm2: self.surface_area_cm2,
            temperature_celsius: self.wet().then_some(self.temperature_celsius),
            oxygen_mg_per_liter: self.concentration(self.oxygen_mg),
            available_nitrogen_mg_per_liter: self.concentration(self.minerals.0[1]),
            available_phosphorus_mg_per_liter: self.concentration(self.minerals.0[2]),
            algae_mg_c: self.algae.carbon(),
            biofilm_mg_c: self.total_biofilm_carbon(),
            biofilm_distribution: self.scenery_layout.is_some().then(|| self.biofilm_distribution()),
            stone_biofilm_enabled: self.stone_biofilm.is_some().then_some(true),
            stone_biofilm: self.stone_biofilm_observation(),
            wood_biofilm_enabled: self.wood_layout.as_ref().and_then(|layout| layout.wood_biofilm.as_ref().map(|_| true)),
            wood_biofilm: self.wood_layout.as_ref().and_then(|layout| layout.wood_biofilm_observation(self.moss_column_px())),
            plant_mg_c: self.plants.carbon() + self.static_sections.iter().filter_map(|section| section.water_plants.as_ref())
                .map(|plants| plants.stock.carbon()).sum::<f64>(),
            detritus_mg_c: self.detritus.carbon(),
            daphnia_count: self.daphnia.count,
            daphnia_stock_mg_cnp: self.scenery_layout.is_some().then(|| {
                let mut stock = self.daphnia.body;
                stock.add(self.daphnia.reserve);
                stock.0
            }),
            shrimp: self.shrimp.iter().map(|shrimp| shrimp.view.clone()).collect(),
            shrimp_life: self.shrimp.iter().map(Shrimp::observe_life).collect(),
            turbidity_index: turbidity,
            stones: self.stone_layout.as_ref().map(|layout| layout.stones.clone()),
            woods: self.wood_layout.as_ref().map(|layout| layout.woods.clone()),
            static_sections: self.static_sections.iter().map(|section| section.observe(self, turbidity)).collect(),
            retain_observation_row: (self.scenery_layout.is_some() && self.shrimp.is_empty() && self.next_shrimp_id > 1).then_some(true),
            shrimp_attachments: self.wood_layout.as_ref().map(|layout| layout.attachments.clone()),
            wood_contact_feeding: None,
            stone_rest_enabled: self.stone_layout.as_ref().and_then(|layout| layout.stone_rest_enabled.then_some(true)),
            stone_attachments: self.stone_layout.as_ref().and_then(|layout| {
                (!layout.stone_attachments.is_empty()).then(|| layout.stone_attachments.clone())
            }),
            stone_contact_feeding: None,
            moss_presets: moss_patches.as_ref().map(|_| PondMossPresetObservation::choices()),
            moss_patches,
            water_plant_patches,
            unassigned_plant_mg_cnp,
            floating_plants: self.floating_plant_layout.as_ref()
                .filter(|layout| layout.has_patches()).map(|layout| layout.observe(self.wet())),
            floating_canopy: self.floating_canopy(),
            shore_sedges: self.sedge_layout.as_ref().and_then(|layout| layout.observe(self.depth_cm() * 10.0)),
            scenery: self.scenery_layout.as_ref().map(|layout| layout.items.clone()),
            shape: None,
            leaf_deposits: self.leaf_deposit_layout.as_ref().and_then(PondLeafLayout::observe),
        }
    }

    fn inventory(&self) -> Inventory {
        let mut cn_p = self.minerals;
        for matter in [self.algae, self.biofilm, self.plants, self.detritus, self.microbes, self.daphnia.body, self.daphnia.reserve] {
            cn_p.add(matter);
        }
        for section in &self.static_sections {
            if let Some(plants) = &section.water_plants { cn_p.add(plants.stock); }
            for patch in section.stone_biofilm.iter().flatten() { cn_p.add(patch.stock); }
            for stock in section.woods.wood_biofilm_matter() { cn_p.add(stock); }
            for patch in section.moss_layout.iter().flat_map(|layout| &layout.patches) { cn_p.add(patch.stock_mg_cnp); }
            for shrimp in &section.shrimp {
                cn_p.add(shrimp.body);
                cn_p.add(shrimp.reserve);
            }
        }
        for patch in self.stone_biofilm.iter().flatten() { cn_p.add(patch.stock); }
        for stock in self.wood_layout.iter().flat_map(PondWoodLayout::wood_biofilm_matter) { cn_p.add(stock); }
        for shrimp in &self.shrimp {
            cn_p.add(shrimp.body);
            cn_p.add(shrimp.reserve);
        }
        if let Some(layout) = &self.moss_layout {
            for patch in &layout.patches {
                cn_p.add(patch.stock_mg_cnp);
            }
        }
        if let Some(layout) = &self.floating_plant_layout {
            for patch in &layout.patches {
                cn_p.add(patch.stock_mg_cnp);
            }
        }
        if let Some(layout) = &self.sedge_layout {
            for patch in &layout.patches { cn_p.add(patch.stock); }
        }
        Inventory { water_liters: self.water_liters, oxygen_mg: self.oxygen_mg, cn_p }
    }

    fn located_leaf_carbon(&self) -> f64 {
        self.leaf_deposit_layout.as_ref().map_or(0.0, PondLeafLayout::located_carbon)
            + self.static_sections.iter().map(PondStaticSection::located_leaf_carbon).sum::<f64>()
    }

    fn topology_split_section_stocks(&mut self, fraction: f64, retain_section: bool, located_before: f64, located_here: f64) {
        self.plants = match &mut self.water_plant_layout {
            Some(layout) => layout.topology_split_stock(self.plants, fraction, retain_section),
            None => self.plants.scaled(fraction),
        };
        let carbon = self.detritus.carbon();
        let share = |located: f64| if carbon > 0.0 { (located / carbon).clamp(0.0, 1.0) } else { 0.0 };
        // The deposit markers track carbon within the mixed detritus pool. Their
        // share takes that same fraction of the pool's N and P with it.
        let detritus_fraction = share(located_here) + (1.0 - share(located_before)) * fraction;
        self.detritus = self.detritus.scaled(detritus_fraction);
        if !retain_section {
            self.water_plant_layout = None;
            self.leaf_deposit_layout = None;
        }
    }

    fn topology_add_unassigned_plants(&mut self, stock: Matter) {
        if let Some(layout) = &mut self.water_plant_layout {
            layout.add_unassigned_stock(self.plants.carbon(), stock.carbon());
        }
        self.plants.add(stock);
    }

    fn record_topology_transfer(&mut self, expected: Inventory) {
        let actual = self.inventory();
        let difference = Matter(std::array::from_fn(|element| {
            let observed = actual.cn_p.0[element];
            let projected = expected.cn_p.0[element];
            let difference = observed - projected;
            let roundoff = 1e-12 + 8.0 * f64::EPSILON * (observed.abs() + projected.abs());
            if difference.abs() <= roundoff { 0.0 } else { difference }
        }));
        self.balance.topology_matter_transfer.add(difference);
    }

    fn ledger(&self) -> PondBasinLedger {
        let current = self.inventory();
        let Balance { initial, inputs, outputs, topology_matter_transfer, .. } = self.balance;
        PondBasinLedger {
            id: self.id.clone(),
            initial_water_liters: initial.water_liters,
            current_water_liters: current.water_liters,
            water_input_liters: inputs.water_liters,
            water_output_liters: outputs.water_liters,
            water_residual_liters: initial.water_liters + inputs.water_liters - outputs.water_liters - current.water_liters,
            initial_matter_mg_cnp: initial.cn_p.0,
            current_matter_mg_cnp: current.cn_p.0,
            matter_input_mg_cnp: inputs.cn_p.0,
            matter_output_mg_cnp: outputs.cn_p.0,
            matter_topology_transfer_mg_cnp: (!topology_matter_transfer.is_zero()).then_some(topology_matter_transfer.0),
            matter_residual_mg_cnp: std::array::from_fn(|i| {
                initial.cn_p.0[i] + inputs.cn_p.0[i] - outputs.cn_p.0[i] + topology_matter_transfer.0[i] - current.cn_p.0[i]
            }),
            initial_oxygen_mg: initial.oxygen_mg,
            current_oxygen_mg: current.oxygen_mg,
            oxygen_input_mg: inputs.oxygen_mg,
            oxygen_output_mg: outputs.oxygen_mg,
            oxygen_residual_mg: initial.oxygen_mg + inputs.oxygen_mg - outputs.oxygen_mg - current.oxygen_mg,
            photosynthesis_oxygen_mg: self.balance.photosynthesis_oxygen_mg,
            producer_respiration_oxygen_mg: self.balance.producer_respiration_oxygen_mg,
            animal_respiration_oxygen_mg: self.balance.animal_respiration_oxygen_mg,
            microbial_respiration_oxygen_mg: self.balance.microbial_respiration_oxygen_mg,
            gas_import_oxygen_mg: self.balance.gas_import_oxygen_mg,
            gas_export_oxygen_mg: self.balance.gas_export_oxygen_mg,
            shrimp_food_mg_c: self.balance.shrimp_food_mg_c,
            daphnia_food_mg_c: self.balance.daphnia_food_mg_c,
            decomposed_mg_c: self.balance.decomposed_mg_c,
        }
    }

    fn valid(&self) -> bool {
        if !(MIN_AREA_CM2..=MAX_AREA_CM2).contains(&self.surface_area_cm2) || !(4.0..=34.0).contains(&self.temperature_celsius)
            || (self.water_liters != 0.0 && !(MIN_WATER_LITERS..=self.capacity()).contains(&self.water_liters))
            || !nonnegative(self.oxygen_mg) || self.minerals.carbon() != 0.0 || !self.daphnia.valid()
            || ![self.minerals, self.algae, self.biofilm, self.plants, self.detritus, self.microbes].into_iter().all(Matter::valid)
            || self.random.state == 0 || !Shrimp::valid_population(&self.shrimp, self.shrimp_namespace(), self.next_shrimp_id)
            || !self.valid_bed()
            || !self.static_sections_valid()
            || !self.valid_scenery()
            || self.stone_biofilm.as_ref().is_some_and(|stocks| !StoneBiofilm::validate(stocks, self.stone_layout.as_ref()))
            || self.water_plant_layout.as_ref().is_some_and(|layout| !layout.valid(self.plants, self.ground_profiles()))
            || self.floating_plant_layout.as_ref().is_some_and(|layout| !layout.valid(self.shape.as_ref()))
            || self.sedge_layout.as_ref().is_some_and(|layout| !layout.valid(self.shape.as_ref()))
            || self.leaf_deposit_layout.as_ref().is_some_and(|layout| !layout.valid(self.detritus.carbon()))
            || self.moss_layout.as_ref().is_some_and(|layout| !layout.valid(self.stone_layout.as_ref(), self.wood_layout.as_ref()))
            || (!self.wet() && (self.oxygen_mg != 0.0 || !self.shrimp.is_empty() || self.daphnia.count != 0 || self.algae.carbon() != 0.0))
            || !self.balance.initial.valid() || !self.balance.inputs.valid() || !self.balance.outputs.valid()
            || !self.balance.topology_matter_transfer.0.into_iter().all(f64::is_finite)
        {
            return false;
        }
        let ledger = self.ledger();
        fn within(residual: f64, stock: f64, input: f64, output: f64) -> bool {
            residual.is_finite() && residual.abs() <= 1e-7 + 1e-9 * (stock + input + output)
        }
        within(ledger.water_residual_liters, ledger.initial_water_liters, ledger.water_input_liters, ledger.water_output_liters)
            && within(ledger.oxygen_residual_mg, ledger.initial_oxygen_mg, ledger.oxygen_input_mg, ledger.oxygen_output_mg)
            && (0..3).all(|i| within(ledger.matter_residual_mg_cnp[i],
                ledger.initial_matter_mg_cnp[i] + self.balance.topology_matter_transfer.0[i].abs(),
                ledger.matter_input_mg_cnp[i], ledger.matter_output_mg_cnp[i]))
            && [self.balance.photosynthesis_oxygen_mg, self.balance.producer_respiration_oxygen_mg,
                self.balance.animal_respiration_oxygen_mg, self.balance.microbial_respiration_oxygen_mg,
                self.balance.gas_import_oxygen_mg, self.balance.gas_export_oxygen_mg,
                self.balance.shrimp_food_mg_c, self.balance.daphnia_food_mg_c, self.balance.decomposed_mg_c,
            ].into_iter().all(nonnegative)
    }

    fn valid_scenery(&self) -> bool {
        let woods = self.wood_layout.as_ref().map(|layout| layout.woods.as_slice()).unwrap_or_default();
        match &self.scenery_layout {
            Some(layout) => layout.valid(|id| woods.iter().any(|wood| wood.id == id)),
            None => !woods.iter().any(|wood| wood.preset.requires_composed_scene()),
        }
    }

    fn valid_bed(&self) -> bool {
        let Some(stones) = &self.stone_layout else { return self.wood_layout.is_none(); };
        ((!stones.stone_rest_enabled && !stones.has_cave()) || self.scenery_layout.is_some())
            && valid_section_bed(stones, self.wood_layout.as_ref(), &self.shrimp, self.column_projection_px())
    }

    fn add_water(&mut self, liters: f64, climate: SeasonObservation) -> Result<(), CoreError> {
        if !liters.is_finite() || liters <= 0.0 { return Err(CoreError::InvalidPondEdit); }
        let next = self.water_liters + liters;
        if next > self.capacity() { return Err(CoreError::PondCapacityExceeded); }
        if next < MIN_WATER_LITERS { return Err(CoreError::PondTooShallow); }
        let incoming_temp = climate.air_temperature_celsius.clamp(4.0, 34.0);
        self.temperature_celsius += (incoming_temp - self.temperature_celsius) * (liters / next);
        self.water_liters = next;
        let oxygen = 0.85 * oxygen_saturation(incoming_temp) * liters;
        let minerals = Matter([0.0, 0.12 * liters, 0.022 * liters]);
        self.oxygen_mg += oxygen;
        self.minerals.add(minerals);
        self.balance.inputs.water_liters += liters;
        self.balance.inputs.oxygen_mg += oxygen;
        self.balance.inputs.cn_p.add(minerals);
        self.clear_shrimp_after_depth_change();
        Ok(())
    }

    fn drain(&mut self, liters: f64) -> Result<(), CoreError> {
        if !liters.is_finite() || liters <= 0.0 { return Err(CoreError::InvalidPondEdit); }
        if liters > self.water_liters { return Err(CoreError::PondInsufficientWater); }
        let remaining = self.water_liters - liters;
        if remaining > 0.0 && remaining < MIN_WATER_LITERS { return Err(CoreError::PondTooShallow); }
        let fraction = liters / self.water_liters;
        let oxygen = self.oxygen_mg * fraction;
        self.oxygen_mg -= oxygen;
        self.water_liters = remaining;
        self.balance.outputs.oxygen_mg += oxygen;
        self.balance.outputs.water_liters += liters;
        self.balance.outputs.cn_p.add(self.minerals.take_fraction(fraction));
        self.balance.outputs.cn_p.add(self.algae.take_fraction(fraction));
        self.settle_dry();
        self.clear_shrimp_after_depth_change();
        Ok(())
    }

    fn reshape(&mut self, surface_area_cm2: f64) -> Result<(), CoreError> {
        if !(MIN_AREA_CM2..=MAX_AREA_CM2).contains(&surface_area_cm2) { return Err(CoreError::InvalidPondEdit); }
        if self.water_liters > surface_area_cm2 * MAX_DEPTH_CM / 1000.0 { return Err(CoreError::PondCapacityExceeded); }
        self.surface_area_cm2 = surface_area_cm2;
        if let Some(shape) = &mut self.shape { shape.rescale(surface_area_cm2); }
        self.clear_shrimp_after_depth_change();
        Ok(())
    }

    fn add_detritus(&mut self, mg_c: f64) -> Result<(), CoreError> {
        if !pond_leaf::valid_dose(mg_c) { return Err(CoreError::InvalidPondEdit); }
        self.record_detritus_input(pond_leaf::detritus_matter(mg_c));
        Ok(())
    }

    fn add_leaf_deposit(&mut self, x: f64, preset: PondLeafDepositPreset, mg_c: f64) -> Result<(), CoreError> {
        if self.scenery_layout.is_none() { return Err(CoreError::WrongWorldKind); }
        let dose = PondLeafLayout::add(&mut self.leaf_deposit_layout, x, preset, mg_c)?;
        self.record_detritus_input(dose);
        Ok(())
    }

    fn move_leaf_deposit(&mut self, id: u32, x: f64) -> Result<(), CoreError> {
        if self.scenery_layout.is_none() { return Err(CoreError::WrongWorldKind); }
        self.leaf_deposit_layout.as_mut().ok_or(CoreError::PondLeafDepositMissing)?.move_deposit(id, x)
    }

    fn record_detritus_input(&mut self, deposited: Matter) {
        self.detritus.add(deposited);
        self.balance.inputs.cn_p.add(deposited);
    }

    fn require_stocking_site(&self, count: u32) -> Result<(), CoreError> {
        if self.scenery_layout.is_none() { return Err(CoreError::WrongWorldKind); }
        if !self.wet() { return Err(CoreError::PondStockNeedsWater); }
        if count == 0 { return Err(CoreError::InvalidPondStockCount); }
        Ok(())
    }

    fn stock_shrimp(&mut self, position: Point, count: u32) -> Result<(), CoreError> {
        self.require_stocking_site(count)?;
        if count as usize > MAX_SHRIMP - self.shrimp.len() || self.next_shrimp_id.checked_add(u64::from(count)).is_none() {
            return Err(CoreError::PondShrimpLimit);
        }
        let depth = self.column_projection_px();
        let range = if self.stone_layout.as_ref().is_some_and(PondStoneLayout::has_cave) {
            PondSwimRange::near_cave_floor(depth)
        } else if self.wood_layout.as_ref().is_some_and(|layout| !layout.woods.is_empty()) {
            PondSwimRange::EXTENDED
        } else { PondSwimRange::STANDARD };
        let solids = stone_profiles(self.stone_layout.as_ref()).chain(wood_profiles(self.wood_layout.as_ref()));
        if !range.contains(position) || solids.into_iter().any(|profile| profile.blocks(position, depth)) {
            return Err(CoreError::InvalidPondShrimpSite);
        }

        let (body, reserve) = Shrimp::starter_matter(true);
        for _ in 0..count {
            let id = self.allocate_shrimp_id();
            let heading = self.random.unit() * TAU;
            self.shrimp.push(Shrimp::new(id, position.x, position.y, heading, body, reserve, SHRIMP_MATURE_SECONDS));
        }
        let mut introduced = body;
        introduced.add(reserve);
        self.balance.inputs.cn_p.add(introduced.scaled(f64::from(count)));
        Ok(())
    }

    fn stock_daphnia(&mut self, count: u32) -> Result<(), CoreError> {
        self.require_stocking_site(count)?;
        if count > MAX_DAPHNIA - self.daphnia.count { return Err(CoreError::PondDaphniaLimit); }

        let starter = Daphnia::new();
        let share = f64::from(count) / f64::from(starter.count);
        let body = starter.body.scaled(share);
        let reserve = starter.reserve.scaled(share);
        self.daphnia.count += count;
        self.daphnia.body.add(body);
        self.daphnia.reserve.add(reserve);
        self.balance.inputs.cn_p.add(body);
        self.balance.inputs.cn_p.add(reserve);
        Ok(())
    }

    fn remove_daphnia(&mut self, count: u32) -> Result<(), CoreError> {
        if self.scenery_layout.is_none() { return Err(CoreError::WrongWorldKind); }
        if count == 0 || count > self.daphnia.count { return Err(CoreError::InvalidDaphniaRemovalCount); }
        let share = f64::from(count) / f64::from(self.daphnia.count);
        self.balance.outputs.cn_p.add(self.daphnia.body.take_fraction(share));
        self.balance.outputs.cn_p.add(self.daphnia.reserve.take_fraction(share));
        self.daphnia.count -= count;
        self.daphnia.birth_credit *= 1.0 - share;
        self.daphnia.mortality_credit *= 1.0 - share;
        Ok(())
    }

    fn settle_dry(&mut self) {
        if self.wet() { return; }
        self.balance.outputs.oxygen_mg += self.oxygen_mg;
        self.balance.gas_export_oxygen_mg += self.oxygen_mg;
        self.oxygen_mg = 0.0;
        self.detritus.add(std::mem::take(&mut self.algae));
        let plankton = std::mem::take(&mut self.daphnia);
        self.detritus.add(plankton.body);
        self.detritus.add(plankton.reserve);
        for shrimp in self.shrimp.drain(..) {
            self.detritus.add(shrimp.body);
            self.detritus.add(shrimp.reserve);
        }
        if let Some(layout) = &mut self.wood_layout {
            layout.attachments.clear();
        }
        if let Some(layout) = &mut self.stone_layout {
            layout.stone_attachments.clear();
        }
        for section in &mut self.static_sections {
            for shrimp in section.shrimp.drain(..) {
                self.detritus.add(shrimp.body);
                self.detritus.add(shrimp.reserve);
            }
            section.woods.attachments.clear();
            section.stones.stone_attachments.clear();
        }
    }
}

impl Basin {
    fn advance(&mut self, light: f64, climate: SeasonObservation,
        mut contact_feeding: Option<&mut PondBasinContactFeeding>,
    ) {
        if !self.wet() { return; }
        self.evaporate(light, climate.relative_humidity);
        if !self.wet() { return; }
        self.clear_shrimp_after_depth_change();
        let thermal_target = (climate.air_temperature_celsius + 4.0 * light).clamp(4.0, 34.0);
        let depth_factor = (self.water_liters * 1000.0 / self.surface_area_cm2 / 7.5).clamp(0.25, 2.0);
        self.temperature_celsius += (thermal_target - self.temperature_celsius) * STEP_SECONDS / (7200.0 * depth_factor);
        let thermal_metabolism = 2.0_f64.powf((self.temperature_celsius - 20.0) / 10.0).clamp(0.35, 2.5);

        self.photosynthesize(light, thermal_metabolism);
        self.exchange_oxygen();
        let supply = self.respire(thermal_metabolism);
        let concentration = self.concentration(self.oxygen_mg).unwrap_or_default();
        let healthy = (concentration / 3.0).clamp(0.0, 1.0).min(supply);
        let feeding = if self.has_wood_biofilm() || self.static_sections.iter().any(|section| !section.shrimp.is_empty()) {
            self.feed_all_shrimp_rows(healthy, thermal_metabolism, contact_feeding.as_deref_mut())
        } else {
            let mut rows = [[false; MAX_SHRIMP]; section::MAX_OBSERVATION_SECTIONS];
            rows[0] = self.feed(healthy, thermal_metabolism, contact_feeding.as_deref_mut());
            rows
        };
        self.colonize_stone_biofilm(thermal_metabolism * STEP_HOURS);
        self.colonize_wood_biofilm(thermal_metabolism * STEP_HOURS);
        self.step_shrimp(healthy, thermal_metabolism, feeding[0]);
        self.step_secondary_shrimp(healthy, thermal_metabolism, &feeding[1..]);
        // Newly settled shrimp were not in contact at allocation. Drop observations for contacts lost this step.
        if let Some(entries) = contact_feeding {
            if let Some(layout) = &self.wood_layout {
                entries.wood.retain(|entry| layout.attachments.iter().any(|attachment| {
                    entry.shrimp_id == attachment.shrimp_id && entry.wood_id == attachment.wood_id && entry.offset == attachment.offset
                }));
            }
            if let Some(layout) = &self.stone_layout {
                entries.stone.retain(|entry| layout.stone_attachments.iter().any(|attachment| {
                    entry.shrimp_id == attachment.shrimp_id && entry.stone_id == attachment.stone_id && entry.offset == attachment.offset
                }));
            }
            entries.sections.retain_mut(|(row, entries)| {
                let Some(section) = self.static_sections.iter().find(|section| section.row == *row) else { return false; };
                shrimp_rows::retain_contacts(entries, &section.woods, &section.stones);
                !entries.is_empty()
            });
        }
        self.step_daphnia(healthy, thermal_metabolism);
    }

    fn evaporate(&mut self, light: f64, humidity: f64) {
        let temperature_factor = (self.temperature_celsius / 21.0).clamp(0.35, 2.0);
        let rate_cm_per_hour = 0.018 * (0.55 + 0.45 * light) * (1.1 - 0.65 * humidity) * temperature_factor;
        let estimated = rate_cm_per_hour * self.surface_area_cm2 / 1000.0 * STEP_HOURS;
        let remaining = (self.water_liters - estimated).max(0.0);
        let next = if remaining < MIN_WATER_LITERS { 0.0 } else { remaining };
        self.balance.outputs.water_liters += self.water_liters - next;
        self.water_liters = next;
        self.settle_dry();
    }

    fn photosynthesize(&mut self, light: f64, thermal: f64) {
        let water_depth_mm = self.depth_cm() * 10.0;
        if light <= 0.0 { return; }
        let incident_light = light;
        let light = self.floating_canopy().map_or(light, |canopy| light * canopy.light_transmission);
        let turbidity = self.turbidity();
        let light_at_depth = light * (1.0 - 0.65 * turbidity);
        let opportunity = light_at_depth * thermal * STEP_HOURS;
        let plant_carbon = self.plants.carbon();
        let plant_demands = self.water_plant_layout.as_ref().and_then(|layout| {
            layout.growth_demand(plant_carbon, light, turbidity, self.depth_cm(), thermal * STEP_HOURS)
        });
        let plant_growth = plant_demands.as_ref().map_or(plant_carbon * 0.020 * opportunity, |demands| demands.total());
        let requests = [
            Matter::with_ratios(self.algae.carbon() * 0.065 * opportunity, 0.088, 0.012),
            Matter::with_ratios(self.biofilm.carbon() * 0.037 * opportunity, 0.077, 0.010),
            Matter::with_ratios(plant_growth, 0.06, 0.007),
        ];
        let mut total = Matter::default();
        for request in requests { total.add(request); }
        let depth_cm = self.depth_cm();
        let column_px = self.moss_column_px();
        let mut section_growth = [const { None }; section::MAX_OBSERVATION_SECTIONS - 1];
        for (section, entry) in self.static_sections.iter().zip(&mut section_growth) {
            let Some(plants) = &section.water_plants else { continue; };
            let carbon = plants.stock.carbon();
            let demands = plants.layout.growth_demand(carbon, light, turbidity, depth_cm, thermal * STEP_HOURS);
            let growth = demands.as_ref().map_or(carbon * 0.020 * opportunity, |demands| demands.total());
            let request = Matter::with_ratios(growth, 0.06, 0.007);
            *entry = Some((request, demands));
            total.add(request);
        }
        let section_attached = std::array::from_fn::<_, { section::MAX_OBSERVATION_SECTIONS - 1 }, _>(|index| {
            self.static_sections.get(index).map(|section| {
                let demand = section.attached_growth_demand(column_px, opportunity);
                demand.accumulate(&mut total);
                demand
            })
        });
        let submerged = self.submerged_moss();
        let stone_wet = self.stone_biofilm_wet_fractions();
        let stone_film = self.stone_biofilm.as_deref_mut().unwrap_or_default();
        let mut stone_film_requests = [Matter::default(); MAX_STONES];
        for ((patch, wet), request) in stone_film.iter().zip(stone_wet).zip(&mut stone_film_requests) {
            *request = patch.growth_demand(wet, opportunity);
            total.add(*request);
        }
        let mut wood_film_requests = [Matter::default(); MAX_WOODS];
        if let Some(layout) = &self.wood_layout {
            let wet = layout.wood_biofilm_wet_fractions(column_px);
            for ((patch, wet), request) in layout.wood_biofilm.iter().flatten().zip(wet).zip(&mut wood_film_requests) {
                *request = patch.growth_demand(wet, opportunity);
                total.add(*request);
            }
        }
        let patches = self.moss_layout.as_mut().map(|layout| layout.patches.as_mut_slice()).unwrap_or_default();
        let mut moss_requests = [Matter::default(); MAX_MOSS_PATCHES];
        for ((patch, wet), request) in patches.iter().zip(submerged).zip(&mut moss_requests) {
            if wet {
                *request = patch.growth_demand(opportunity);
                total.add(*request);
            }
        }
        let floats = self.floating_plant_layout.as_mut().map(|layout| layout.patches.as_mut_slice()).unwrap_or_default();
        let mut floating_requests = [Matter::default(); MAX_FLOATING_PLANTS];
        for (patch, request) in floats.iter().zip(&mut floating_requests) {
            *request = patch.growth_demand(incident_light * thermal * STEP_HOURS);
            total.add(*request);
        }
        let sedges = self.sedge_layout.as_mut().map(|layout| layout.patches.as_mut_slice()).unwrap_or_default();
        let mut sedge_requests = [Matter::default(); MAX_SEDGE_PATCHES];
        for (patch, request) in sedges.iter().zip(&mut sedge_requests) {
            *request = patch.growth(water_depth_mm, incident_light * thermal * STEP_HOURS);
            total.add(*request);
        }
        if total.carbon() == 0.0 { return; }
        let fraction = (self.minerals.0[1] / total.0[1]).min(self.minerals.0[2] / total.0[2]).min(1.0);
        let actual = total.scaled(fraction);
        self.minerals.0[1] = (self.minerals.0[1] - actual.0[1]).max(0.0);
        self.minerals.0[2] = (self.minerals.0[2] - actual.0[2]).max(0.0);
        for (producer, request) in [&mut self.algae, &mut self.biofilm, &mut self.plants].into_iter().zip(requests) {
            producer.add(request.scaled(fraction));
        }
        if let (Some(layout), Some(demands)) = (&mut self.water_plant_layout, plant_demands) {
            layout.apply_flux(plant_carbon, demands, fraction);
        }
        for (section, entry) in self.static_sections.iter_mut().zip(section_growth) {
            if let (Some(plants), Some((request, demands))) = (&mut section.water_plants, entry) {
                if let Some(demands) = demands {
                    plants.layout.apply_flux(plants.stock.carbon(), demands, fraction);
                }
                plants.stock.add(request.scaled(fraction));
            }
        }
        for (section, demand) in self.static_sections.iter_mut().zip(section_attached) {
            if let Some(demand) = demand { section.apply_attached_growth(demand, fraction); }
        }
        for (patch, request) in patches.iter_mut().zip(moss_requests) {
            patch.stock_mg_cnp.add(request.scaled(fraction));
        }
        for (patch, request) in stone_film.iter_mut().zip(stone_film_requests) {
            patch.stock.add(request.scaled(fraction));
        }
        for (patch, request) in self.wood_layout.iter_mut().flat_map(|layout| layout.wood_biofilm.iter_mut().flatten()).zip(wood_film_requests) {
            patch.stock.add(request.scaled(fraction));
        }
        self.balance.inputs.cn_p.0[0] += actual.carbon();
        let mut aerial_carbon = 0.0;
        for (patch, request) in floats.iter_mut().zip(floating_requests) {
            let gained = request.scaled(fraction);
            patch.stock_mg_cnp.add(gained);
            aerial_carbon += gained.carbon();
        }
        for (patch, request) in sedges.iter_mut().zip(sedge_requests) {
            let gained = request.scaled(fraction);
            patch.stock.add(gained);
            aerial_carbon += gained.carbon();
        }
        let oxygen = (actual.carbon() - aerial_carbon).max(0.0) * OXYGEN_MG_PER_CARBON_MG;
        self.oxygen_mg += oxygen;
        self.balance.inputs.oxygen_mg += oxygen;
        self.balance.photosynthesis_oxygen_mg += oxygen;
    }

    fn exchange_oxygen(&mut self) {
        let concentration = self.oxygen_mg / self.water_liters;
        let area_m2 = self.surface_area_cm2 / 10_000.0;
        let exchange = 0.020 * area_m2 * (oxygen_saturation(self.temperature_celsius) - concentration) * 1000.0 * STEP_HOURS;
        if exchange >= 0.0 {
            self.oxygen_mg += exchange;
            self.balance.inputs.oxygen_mg += exchange;
            self.balance.gas_import_oxygen_mg += exchange;
        } else {
            let outflow = (-exchange).min(self.oxygen_mg);
            self.oxygen_mg -= outflow;
            self.balance.outputs.oxygen_mg += outflow;
            self.balance.gas_export_oxygen_mg += outflow;
        }
    }

    fn respire(&mut self, thermal: f64) -> f64 {
        let elapsed = STEP_HOURS * thermal;
        let water_depth_mm = self.depth_cm() * 10.0;
        for patch in self.sedge_layout.iter_mut().flat_map(|layout| &mut layout.patches) {
            let fuel = patch.maintenance(water_depth_mm, elapsed);
            self.minerals.add(fuel.minerals());
            self.balance.outputs.cn_p.0[0] += fuel.carbon();
        }
        let retention = 0.18;
        let algae = self.algae.carbon() * 0.005 * elapsed;
        let biofilm = self.biofilm.carbon() * 0.004 * elapsed;
        let plant_carbon = self.plants.carbon();
        let plant_demands = self.water_plant_layout.as_ref().and_then(|layout| layout.respiration_demand(plant_carbon, elapsed));
        let plants = plant_demands.as_ref().map_or(plant_carbon * 0.003 * elapsed, |demands| demands.total());
        let microbes = self.microbes.carbon() * 0.008 * elapsed;
        let decomposition = self.detritus.carbon() * 0.020 * elapsed;
        let daphnia = (self.daphnia.body.carbon() * 0.035 * elapsed).min(self.daphnia.reserve.carbon());
        let mut shrimp = [0.0; MAX_SHRIMP];
        for (individual, request) in self.shrimp.iter().zip(&mut shrimp) {
            *request = (individual.body.carbon() * 0.019 * elapsed).min(individual.reserve.carbon());
        }
        let mut demanded_carbon = algae + biofilm + plants + microbes + decomposition * (1.0 - retention)
            + daphnia + shrimp.iter().sum::<f64>();
        let mut section_respiration = [const { None }; section::MAX_OBSERVATION_SECTIONS - 1];
        for (section, entry) in self.static_sections.iter().zip(&mut section_respiration) {
            let Some(plants) = &section.water_plants else { continue; };
            let carbon = plants.stock.carbon();
            let demands = plants.layout.respiration_demand(carbon, elapsed);
            let request = demands.as_ref().map_or(carbon * 0.003 * elapsed, |demands| demands.total());
            *entry = Some((request, demands));
            demanded_carbon += request;
        }
        let column_px = self.moss_column_px();
        let section_attached = std::array::from_fn::<_, { section::MAX_OBSERVATION_SECTIONS - 1 }, _>(|index| {
            self.static_sections.get(index).map(|section| {
                let demand = section.extended_respiration_demand(column_px, elapsed);
                demanded_carbon += demand.total();
                demand
            })
        });
        let submerged = self.submerged_moss();
        let stone_wet = self.stone_biofilm_wet_fractions();
        let stone_film = self.stone_biofilm.as_deref_mut().unwrap_or_default();
        let mut stone_film_requests = [0.0; MAX_STONES];
        for ((patch, wet), request) in stone_film.iter().zip(stone_wet).zip(&mut stone_film_requests) {
            *request = patch.respiration_demand(wet, elapsed);
            demanded_carbon += *request;
        }
        let mut wood_film_requests = [0.0; MAX_WOODS];
        if let Some(layout) = &self.wood_layout {
            let wet = layout.wood_biofilm_wet_fractions(column_px);
            for ((patch, wet), request) in layout.wood_biofilm.iter().flatten().zip(wet).zip(&mut wood_film_requests) {
                *request = patch.respiration_demand(wet, elapsed);
                demanded_carbon += *request;
            }
        }
        let patches = self.moss_layout.as_mut().map(|layout| layout.patches.as_mut_slice()).unwrap_or_default();
        let mut moss_requests = [0.0; MAX_MOSS_PATCHES];
        for ((patch, wet), request) in patches.iter().zip(submerged).zip(&mut moss_requests) {
            if wet {
                *request = patch.respiration_demand(elapsed);
                demanded_carbon += *request;
            }
        }
        let floats = self.floating_plant_layout.as_mut().map(|layout| layout.patches.as_mut_slice()).unwrap_or_default();
        let mut floating_requests = [0.0; MAX_FLOATING_PLANTS];
        for (patch, request) in floats.iter().zip(&mut floating_requests) {
            *request = patch.respiration_demand(elapsed);
            demanded_carbon += *request;
        }
        let demand_oxygen = demanded_carbon * OXYGEN_MG_PER_CARBON_MG;
        let supply = if demand_oxygen == 0.0 { 1.0 } else { (self.oxygen_mg / demand_oxygen).min(1.0) };

        let mut producer_carbon = 0.0;
        for (pool, request) in [(&mut self.algae, algae), (&mut self.biofilm, biofilm), (&mut self.plants, plants)] {
            let respiratory_fuel = pool.take_carbon(request * supply);
            producer_carbon += respiratory_fuel.carbon();
            self.minerals.add(respiratory_fuel.minerals());
        }
        for (section, entry) in self.static_sections.iter_mut().zip(section_respiration) {
            if let (Some(plants), Some((request, demands))) = (&mut section.water_plants, entry) {
                if let Some(demands) = demands {
                    plants.layout.apply_flux(plants.stock.carbon(), demands, -supply);
                }
                let fuel = plants.stock.take_carbon(request * supply);
                producer_carbon += fuel.carbon();
                self.minerals.add(fuel.minerals());
            }
        }
        for (patch, request) in stone_film.iter_mut().zip(stone_film_requests) {
            let fuel = patch.stock.take_carbon(request * supply);
            producer_carbon += fuel.carbon();
            self.minerals.add(fuel.minerals());
        }
        for (patch, request) in self.wood_layout.iter_mut().flat_map(|layout| layout.wood_biofilm.iter_mut().flatten()).zip(wood_film_requests) {
            let fuel = patch.stock.take_carbon(request * supply);
            producer_carbon += fuel.carbon();
            self.minerals.add(fuel.minerals());
        }
        if let (Some(layout), Some(demands)) = (&mut self.water_plant_layout, plant_demands) {
            layout.apply_flux(plant_carbon, demands, -supply);
        }
        let mut shed = Matter::default();
        for ((patch, wet), request) in patches.iter_mut().zip(submerged).zip(moss_requests) {
            if wet {
                let fuel = patch.stock_mg_cnp.take_carbon(request * supply);
                producer_carbon += fuel.carbon();
                self.minerals.add(fuel.minerals());
                shed.add(patch.senesce(elapsed));
            }
        }
        let mut section_animal_carbon = 0.0;
        let mut section_animal_minerals = Matter::default();
        for (section, demand) in self.static_sections.iter_mut().zip(section_attached) {
            if let Some(demand) = demand {
                let realized = section.apply_extended_respiration(demand, supply, elapsed);
                producer_carbon += realized.producer_fuel.carbon();
                self.minerals.add(realized.producer_fuel.minerals());
                section_animal_carbon += realized.animal_fuel.carbon();
                section_animal_minerals.add(realized.animal_fuel.minerals());
                shed.add(realized.shed);
            }
        }
        for (patch, request) in floats.iter_mut().zip(floating_requests) {
            let fuel = patch.stock_mg_cnp.take_carbon(request * supply);
            producer_carbon += fuel.carbon();
            self.minerals.add(fuel.minerals());
        }
        let microbial_fuel = self.microbes.take_carbon(microbes * supply);
        self.minerals.add(microbial_fuel.minerals());
        let before_decomposition = self.detritus.carbon();
        let decomposed = self.detritus.take_carbon(decomposition * supply);
        let after_decomposition = self.detritus.carbon();
        for layout in self.leaf_deposit_layout.iter_mut().chain(
            self.static_sections.iter_mut().filter_map(|section| section.leaf_deposit_layout.as_mut()),
        ) {
            layout.decay(before_decomposition, after_decomposition);
        }
        if shed != Matter::default() { self.detritus.add(shed); }
        self.microbes.add(decomposed.scaled(retention));
        let mineralized = decomposed.scaled(1.0 - retention);
        self.minerals.add(mineralized.minerals());
        self.balance.decomposed_mg_c += decomposed.carbon();

        let daphnia_fuel = self.daphnia.reserve.take_carbon(daphnia * supply);
        let mut animal_carbon = daphnia_fuel.carbon();
        self.detritus.add(daphnia_fuel.minerals());
        for (individual, request) in self.shrimp.iter_mut().zip(shrimp) {
            let respiratory_fuel = individual.reserve.take_carbon(request * supply);
            animal_carbon += respiratory_fuel.carbon();
            self.detritus.add(respiratory_fuel.minerals());
        }
        if section_animal_carbon > 0.0 {
            animal_carbon += section_animal_carbon;
            self.detritus.add(section_animal_minerals);
        }

        let microbial_carbon = microbial_fuel.carbon() + mineralized.carbon();
        self.balance.outputs.cn_p.0[0] += producer_carbon + animal_carbon + microbial_carbon;
        let producer_oxygen = producer_carbon * OXYGEN_MG_PER_CARBON_MG;
        let animal_oxygen = animal_carbon * OXYGEN_MG_PER_CARBON_MG;
        let microbial_oxygen = microbial_carbon * OXYGEN_MG_PER_CARBON_MG;
        let spent = (producer_oxygen + animal_oxygen + microbial_oxygen).min(self.oxygen_mg);
        self.balance.producer_respiration_oxygen_mg += producer_oxygen;
        self.balance.animal_respiration_oxygen_mg += animal_oxygen;
        self.balance.microbial_respiration_oxygen_mg += microbial_oxygen;
        self.oxygen_mg -= spent;
        self.balance.outputs.oxygen_mg += spent;
        supply
    }

    fn feed(&mut self, oxygen: f64, thermal: f64,
        mut contact_feeding: Option<&mut PondBasinContactFeeding>,
    ) -> [bool; MAX_SHRIMP] {
        let depth = self.column_projection_px();
        let opportunity = oxygen * thermal * STEP_HOURS;
        self.feed_daphnia(opportunity);
        let woods = self.wood_layout.as_ref().filter(|layout| !layout.attachments.is_empty());
        let stones = self.stone_layout.as_ref().filter(|layout| !layout.stone_attachments.is_empty());

        let mut claims = [0.0; MAX_SHRIMP];
        for (individual, claim) in self.shrimp.iter().zip(&mut claims) {
            let hunger = (1.0 - individual.reserve.carbon() / 7.0).clamp(0.0, 1.0);
            *claim = individual.body.carbon() * 0.095 * hunger * opportunity;
        }
        let total = claims.iter().sum::<f64>();
        let mut feeding = [false; MAX_SHRIMP];
        if total == 0.0 { return feeding; }
        let food_sources = (contact_feeding.is_some() && (woods.is_some() || stones.is_some()) && total > 0.0)
            .then(|| self.biofilm_distribution());
        let mut stone_contacts = [None; MAX_SHRIMP];
        if let Some(layout) = stones {
            for (individual, contact) in self.shrimp.iter().zip(&mut stone_contacts) {
                *contact = layout.stone_attachments.iter()
                    .find(|attachment| attachment.matches(&layout.stones, &individual.view, depth));
            }
        }
        let wet = self.stone_biofilm_wet_fractions();
        let localized = self.stone_biofilm.is_some();
        let stone_food = self.stone_biofilm.as_deref_mut().map_or([Matter::default(); MAX_SHRIMP], |stocks| {
            Basin::allocate_stone_food(stocks, &stone_contacts, &mut claims, wet)
        });
        let shared_demand = if localized { claims.iter().sum::<f64>() } else { total };
        let shared = self.biofilm.take_carbon(shared_demand);
        let stone_eaten = stone_food.iter().map(|food| food.carbon()).sum::<f64>();
        self.balance.shrimp_food_mg_c += shared.carbon() + stone_eaten;
        for (index, ((individual, claim), observed)) in self.shrimp.iter_mut().zip(claims).zip(&mut feeding).enumerate() {
            let from_shared = if shared_demand > 0.0 { shared.scaled(claim / shared_demand) } else { Matter::default() };
            let from_stone = stone_food[index];
            let mut parcel = from_shared;
            if from_stone.carbon() > 0.0 { parcel.add(from_stone); }
            individual.reserve.add(parcel.scaled(0.57));
            self.detritus.add(parcel.scaled(0.43));
            let biofilm_mg_c = parcel.carbon();
            *observed = biofilm_mg_c > 0.0;
            let Some(entries) = contact_feeding.as_deref_mut().filter(|_| *observed) else { continue; };
            let wood_contact = woods.and_then(|layout| {
                layout.attachments.iter().find(|attachment| layout.attachment_matches(attachment, &individual.view, depth))
            });
            if let Some(attachment) = wood_contact {
                entries.wood.push(PondShrimpWoodFeedingObservation {
                    shrimp_id: individual.view.id.clone(), wood_id: attachment.wood_id,
                    offset: attachment.offset, biofilm_mg_c, wood_biofilm_mg_c: 0.0,
                    shared_biofilm_mg_c: biofilm_mg_c,
                });
            } else if let Some(attachment) = stone_contacts[index] {
                entries.stone.push(PondShrimpStoneFeedingObservation {
                    shrimp_id: individual.view.id.clone(), stone_id: attachment.stone_id,
                    offset: attachment.offset, biofilm_mg_c, stone_biofilm_mg_c: from_stone.carbon(),
                    shared_biofilm_mg_c: from_shared.carbon(),
                });
            }
        }
        if let Some((entries, sources)) = contact_feeding.zip(food_sources) {
            sources.attribute_contact_intake(entries, localized, false);
        }
        feeding
    }

    fn step_daphnia(&mut self, oxygen: f64, thermal: f64) {
        if self.daphnia.count == 0 { return; }
        let individuals = self.daphnia.count as f64;
        let mortality = if oxygen < 0.70 { (0.70 - oxygen) * individuals * STEP_SECONDS / 1800.0 } else { 0.0 };
        self.daphnia.mortality_credit += mortality;
        let deaths = (self.daphnia.mortality_credit.floor() as u32).min(self.daphnia.count);
        if deaths > 0 {
            let dead_share = f64::from(deaths) / individuals;
            self.detritus.add(self.daphnia.body.take_fraction(dead_share));
            self.detritus.add(self.daphnia.reserve.take_fraction(dead_share));
            self.daphnia.count -= deaths;
            self.daphnia.mortality_credit -= f64::from(deaths);
            if self.daphnia.count == 0 {
                self.daphnia = Daphnia::default();
                return;
            }
        }
        if oxygen >= 0.75 && self.daphnia.count < MAX_DAPHNIA && self.daphnia.reserve.carbon() >= 0.18 {
            self.daphnia.birth_credit = (self.daphnia.birth_credit + f64::from(self.daphnia.count) * thermal * 0.008 * STEP_HOURS).min(1.0);
            if self.daphnia.birth_credit >= 1.0 {
                self.daphnia.count += 1;
                let newborn = self.daphnia.reserve.take_carbon(0.18);
                self.daphnia.body.add(newborn);
                self.daphnia.birth_credit = 0.0;
            }
        }
    }
}

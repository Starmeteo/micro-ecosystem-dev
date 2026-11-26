use crate::forest_rocks::{self, ForestBarkShelter, ForestGroundPlant, ForestLeafPatch, ForestLog, ForestRockLayout,
    ForestStonework, LOG_HALF_SPAN};
use crate::season::{SeasonObservation, TICKS_PER_DAY};
use crate::{Activity, Animal, CoreError, Point, RandomState, ANIMAL_COUNT, STEP_MILLIS};
use serde::{Deserialize, Serialize};
use std::f64::consts::TAU;
use std::ops::RangeInclusive;

pub(super) const COLUMNS: usize = 12;
const ROWS: usize = 4;
pub(super) const CELL_COUNT: usize = COLUMNS * ROWS;
const X_MIN: f64 = 0.04;
const X_MAX: f64 = 0.96;
const Y_MIN: f64 = 0.58;
const Y_MAX: f64 = 0.94;
const CELL_WIDTH: f64 = (X_MAX - X_MIN) / COLUMNS as f64;
const CELL_HEIGHT: f64 = (Y_MAX - Y_MIN) / ROWS as f64;
const CELL_WATER_CAPACITY_ML: f64 = 30.0;
const STEP_S: f64 = STEP_MILLIS as f64 / 1000.0;
const STEP_H: f64 = STEP_S / 3600.0;
const RAIN_ML_PER_CELL_H: f64 = 0.4;
const EVAPORATION_ML_PER_CELL_H: f64 = 22.0;
const MOSS_STARTER_DOSE: Matter = Matter { mg_c: 8.0, mg_n: 0.32, mg_p: 0.048 };
const FERN_STARTER_DOSE: Matter = Matter { mg_c: 15.0, mg_n: 0.54, mg_p: 0.075 };
const OPEN_GROUND_SHADE: f64 = 0.12;
const FERN_MAX_ADDED_SHADE: f64 = 0.28;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ForestPlantKind { Moss, Fern }

impl ForestPlantKind {
    fn initial_stock(self) -> Matter {
        match self { Self::Moss => MOSS_STARTER_DOSE, Self::Fern => FERN_STARTER_DOSE }
    }

    pub(super) fn invalid_site(self) -> CoreError {
        match self { Self::Moss => CoreError::InvalidForestMossSite, Self::Fern => CoreError::InvalidForestFernSite }
    }

    pub(super) fn blocked_site(self) -> CoreError {
        match self { Self::Moss => CoreError::BlockedForestMossSite, Self::Fern => CoreError::BlockedForestFernSite }
    }

    pub(super) fn missing(self) -> CoreError {
        match self { Self::Moss => CoreError::ForestMossPatchMissing, Self::Fern => CoreError::ForestFernPatchMissing }
    }

    pub(super) fn limit(self) -> CoreError {
        match self { Self::Moss => CoreError::ForestMossPatchLimit, Self::Fern => CoreError::ForestFernPatchLimit }
    }

    fn invalid_trim(self) -> CoreError {
        match self { Self::Moss => CoreError::InvalidForestMossTrim, Self::Fern => CoreError::InvalidForestFernTrim }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ForestSubstrate {
    #[default]
    Native,
    FineSoil,
    CoarseHumus,
}

impl ForestSubstrate {
    fn is_native(&self) -> bool { *self == Self::Native }

    // Experimental presets for this model. The coefficients are gameplay defaults.
    fn profile(self) -> ForestSubstratePresetObservation {
        let (water_capacity_ml, evaporation_multiplier) = match self {
            Self::Native => (CELL_WATER_CAPACITY_ML, 1.0),
            Self::FineSoil => (36.0, 0.85),
            Self::CoarseHumus => (24.0, 1.15),
        };
        ForestSubstratePresetObservation { substrate: self, water_capacity_ml, evaporation_multiplier }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForestSubstratePresetObservation {
    pub substrate: ForestSubstrate,
    pub water_capacity_ml: f64,
    pub evaporation_multiplier: f64,
}

impl ForestSubstratePresetObservation {
    pub(super) fn all() -> [Self; 3] {
        [ForestSubstrate::Native, ForestSubstrate::FineSoil, ForestSubstrate::CoarseHumus].map(ForestSubstrate::profile)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForestSubstrateCellObservation {
    pub cell_index: usize,
    pub position: Point,
    pub water_ml: f64,
    /// Generic decomposer carbon held by this soil cell. This does not isolate fungal biomass.
    pub decomposer_mg_c: f64,
    #[serde(flatten)]
    pub profile: ForestSubstratePresetObservation,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForestGroundPlantObservation {
    pub id: u32,
    pub position: Point,
    pub cell_index: usize,
    pub stock_mg_cnp: [f64; 3],
    pub stress_fraction: f64,
}

pub type ForestMossPatchObservation = ForestGroundPlantObservation;

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForestFernPatchObservation {
    #[serde(flatten)]
    pub patch: ForestGroundPlantObservation,
    /// Extra shade potentially added on open ground before composition with other shade.
    pub canopy_shade_fraction: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForestFernPresetObservation {
    pub initial_mg_cnp: [f64; 3],
    pub height_mm: u32,
    pub width_mm: u32,
}

impl ForestFernPresetObservation {
    pub(super) fn ground() -> Self {
        Self { initial_mg_cnp: FERN_STARTER_DOSE.values(), height_mm: 140, width_mm: 130 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForestMossPresetObservation {
    pub initial_mg_cnp: [f64; 3],
    pub width_mm: u32,
}

impl ForestMossPresetObservation {
    pub(super) fn ground() -> Self { Self { initial_mg_cnp: MOSS_STARTER_DOSE.values(), width_mm: 40 } }
}
const FEED_MG_C_PER_S: f64 = 0.012;
const RESPIRATION_MG_C_PER_S: f64 = 0.002;
const TARGET_RESERVE_MG_C: f64 = 4.4;
const WALK_PER_S: f64 = 0.017;
const DECISION_TICKS: u8 = 48;
const SEARCH_CELLS: u8 = 5;
const MAX_WOODLICE: usize = 32;

// Experimental fixture rates. Sources motivate the direction of a response, not its rate.
const JUVENILE_MINIMUM_TICKS: u64 = 14 * TICKS_PER_DAY;
const ADULT_BODY_MG_C: f64 = 4.0;
const MAX_BODY_MG_C: f64 = 12.0;
const MAX_AGE_TICKS: u64 = 220 * TICKS_PER_DAY;
const BROOD_MG_C: f64 = 0.75;
const BROOD_TICKS: u64 = 6 * TICKS_PER_DAY;
const BROOD_COOLDOWN_TICKS: u64 = 12 * TICKS_PER_DAY;
const STARVATION_TICKS: u64 = TICKS_PER_DAY / 4;
const DRY_STRESS_TICKS: u64 = TICKS_PER_DAY / 2;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForestObservation {
    pub water_ml: [f64; 2],
    pub water_capacity_ml: [f64; 2],
    pub shade_fraction: [f64; 2],
    pub shelter_fraction: [f64; 2],
    pub evaporation_ml_per_hour: [f64; 2],
    pub litter_mg_c: f64,
    pub soil_nitrogen_mg: f64,
    pub soil_phosphorus_mg: f64,
    pub woodlice_by_side: [u32; 2],
    pub sheltered_woodlice_by_side: [u32; 2],
    pub feeding_woodlice_by_side: [u32; 2],
    pub walkable_cells: [u32; 2],
    pub rock_cell: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub life: Option<ForestLifeObservation>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForestLifeObservation {
    pub woodlice: Vec<WoodlouseLifeObservation>,
    pub woodlice_born: u64,
    pub woodlice_died: u64,
    pub springtail_mg_c: [f64; 2],
    pub fern_mg_c: [f64; 2],
    pub moss_mg_c: [f64; 2],
    pub fern_stress_fraction: [f64; 2],
    pub moss_stress_fraction: [f64; 2],
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WoodlouseLifeObservation {
    pub id: String,
    pub stage: WoodlouseStage,
    pub age_seconds: f64,
    pub biomass_mg_c: f64,
    pub brood_progress: Option<f64>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WoodlouseStage {
    Juvenile,
    Adult,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForestLedger {
    pub initial_water_ml: f64,
    pub current_water_ml: f64,
    pub water_input_ml: f64,
    pub water_output_ml: f64,
    pub water_residual_ml: f64,
    pub initial_matter_mg_cnp: [f64; 3],
    pub current_matter_mg_cnp: [f64; 3],
    pub matter_input_mg_cnp: [f64; 3],
    pub matter_output_mg_cnp: [f64; 3],
    pub matter_residual_mg_cnp: [f64; 3],
    pub fed_mg_c: f64,
    pub decomposed_mg_c: f64,
    pub photosynthesized_mg_c: f64,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Matter {
    mg_c: f64,
    mg_n: f64,
    mg_p: f64,
}

impl Matter {
    fn new(mg_c: f64, mg_n: f64, mg_p: f64) -> Self {
        Self { mg_c, mg_n, mg_p }
    }

    fn values(self) -> [f64; 3] {
        [self.mg_c, self.mg_n, self.mg_p]
    }

    fn from_values([mg_c, mg_n, mg_p]: [f64; 3]) -> Self {
        Self::new(mg_c, mg_n, mg_p)
    }

    fn scaled(self, ratio: f64) -> Self {
        Self::new(self.mg_c * ratio, self.mg_n * ratio, self.mg_p * ratio)
    }

    fn add(&mut self, addition: Self) {
        self.mg_c += addition.mg_c;
        self.mg_n += addition.mg_n;
        self.mg_p += addition.mg_p;
    }

    fn withdraw(&mut self, requested_mg_c: f64) -> Self {
        if self.mg_c <= 0.0 || requested_mg_c <= 0.0 {
            return Self::default();
        }
        self.withdraw_fraction((requested_mg_c / self.mg_c).min(1.0))
    }

    fn withdraw_fraction(&mut self, fraction: f64) -> Self {
        let withdrawn = self.scaled(fraction);
        *self = self.scaled(1.0 - fraction);
        withdrawn
    }

    fn valid(self) -> bool {
        self.values().into_iter().all(nonnegative)
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Inventory {
    water_ml: f64,
    matter: Matter,
}

impl Inventory {
    fn valid(self) -> bool {
        nonnegative(self.water_ml) && self.matter.valid()
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SoilCell {
    water_ml: f64,
    #[serde(default, skip_serializing_if = "ForestSubstrate::is_native")]
    substrate: ForestSubstrate,
    litter: Matter,
    waste: Matter,
    microbes: Matter,
    moss: Matter,
    mineral_n_mg: f64,
    mineral_p_mg: f64,
}

impl SoilCell {
    fn new(index: usize) -> Self {
        let row = index / COLUMNS;
        let col = index % COLUMNS;
        let litter_c = 42.0 + ((row * 3 + col * 7) % 11) as f64 * 2.1;
        Self {
            water_ml: CELL_WATER_CAPACITY_ML * (0.56 + ((row + col * 2) % 7) as f64 * 0.017),
            substrate: ForestSubstrate::Native,
            litter: Matter::new(litter_c, litter_c * 0.043, litter_c * 0.0045),
            waste: Matter::default(),
            microbes: Matter::new(2.4, 0.23, 0.035),
            moss: MOSS_STARTER_DOSE,
            mineral_n_mg: 3.5,
            mineral_p_mg: 0.58,
        }
    }

    fn water_capacity_ml(&self) -> f64 { self.substrate.profile().water_capacity_ml }

    fn saturation(&self) -> f64 {
        self.water_ml / self.water_capacity_ml()
    }

    fn valid(&self) -> bool {
        (0.0..=self.water_capacity_ml()).contains(&self.water_ml)
            && self.litter.valid() && self.waste.valid() && self.microbes.valid() && self.moss.valid()
            && nonnegative(self.mineral_n_mg) && nonnegative(self.mineral_p_mg)
    }

    fn organic_turnover(&mut self, decomposed: Matter, balance: &mut Balance) {
        let retained = 0.18;
        self.microbes.add(decomposed.scaled(retained));
        self.mineral_n_mg += decomposed.mg_n * (1.0 - retained);
        self.mineral_p_mg += decomposed.mg_p * (1.0 - retained);
        balance.outputs.matter.mg_c += decomposed.mg_c * (1.0 - retained);
        balance.decomposed_mg_c += decomposed.mg_c;
    }

    fn photosynthesis(&mut self, site: Site, light: f64, balance: &mut Balance) {
        let potential = 0.003 * light * (1.0 - site.shade) * self.saturation() * STEP_S;
        let uptake = self.plant_uptake(potential, 0.04, 0.006, balance);
        self.moss.add(uptake);
    }

    fn plant_uptake(&mut self, potential: f64, n_to_c: f64, p_to_c: f64, balance: &mut Balance) -> Matter {
        let carbon = potential.min(self.mineral_n_mg / n_to_c).min(self.mineral_p_mg / p_to_c);
        let uptake = Matter::new(carbon, (carbon * n_to_c).min(self.mineral_n_mg), (carbon * p_to_c).min(self.mineral_p_mg));
        self.mineral_n_mg -= uptake.mg_n;
        self.mineral_p_mg -= uptake.mg_p;
        balance.inputs.matter.mg_c += carbon;
        balance.photosynthesized_mg_c += carbon;
        uptake
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Woodlouse {
    body: Matter,
    reserve: Matter,
    next_cell: usize,
    decision_remaining: u8,
}

impl Woodlouse {
    fn new() -> Self {
        Self {
            body: Matter::new(12.0, 1.8, 0.18),
            reserve: Matter::new(2.6, 0.19, 0.024),
            next_cell: 0,
            decision_remaining: 0,
        }
    }

    fn hunger(&self) -> f64 {
        (1.0 - self.reserve.mg_c / TARGET_RESERVE_MG_C).clamp(0.0, 1.0)
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Balance {
    initial: Inventory,
    inputs: Inventory,
    outputs: Inventory,
    fed_mg_c: f64,
    decomposed_mg_c: f64,
    photosynthesized_mg_c: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SeasonalSoil {
    springtails: Matter,
    fern: Matter,
    fern_stress: f64,
    moss_stress: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WoodlouseLife {
    number: u64,
    stage: WoodlouseStage,
    age_ticks: u64,
    hungry_ticks: u64,
    dry_ticks: u64,
    brood: Matter,
    brood_progress: f64,
    cooldown_ticks: u64,
}

impl WoodlouseLife {
    fn juvenile(number: u64) -> Self {
        Self {
            number,
            stage: WoodlouseStage::Juvenile,
            age_ticks: 0,
            hungry_ticks: 0,
            dry_ticks: 0,
            brood: Matter::default(),
            brood_progress: 0.0,
            cooldown_ticks: 0,
        }
    }

    fn stocked_adult(number: u64) -> Self {
        Self { stage: WoodlouseStage::Adult, age_ticks: 35 * TICKS_PER_DAY, ..Self::juvenile(number) }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SeasonalForest {
    soil: Vec<SeasonalSoil>,
    woodlice: Vec<WoodlouseLife>,
    next_number: u64,
    born: u64,
    died: u64,
    #[serde(default = "typical_initial_woodlice", skip_serializing_if = "is_typical_initial_woodlice")]
    initial_woodlice: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    introduced_woodlice: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    stocked_woodlice: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    removed_woodlice: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    retired_woodlouse_numbers: u64,
}

fn typical_initial_woodlice() -> u64 { ANIMAL_COUNT as u64 }

fn is_typical_initial_woodlice(count: &u64) -> bool { *count == typical_initial_woodlice() }

fn is_zero(count: &u64) -> bool { *count == 0 }

impl SeasonalForest {
    fn new(individuals: usize) -> Self {
        let soil = (0..CELL_COUNT)
            .map(|index| SeasonalSoil {
                springtails: Matter::new(0.85, 0.085, 0.012),
                fern: if fern_patch(index) { FERN_STARTER_DOSE } else { Matter::default() },
                fern_stress: 0.0,
                moss_stress: 0.0,
            })
            .collect();
        let woodlice = (0..individuals)
            .map(|index| {
                let mut life = WoodlouseLife::juvenile(index as u64 + 1);
                if index + 1 < individuals {
                    life.stage = WoodlouseStage::Adult;
                    life.age_ticks = (35 + 17 * index as u64) * TICKS_PER_DAY;
                } else {
                    life.age_ticks = 5 * TICKS_PER_DAY;
                }
                life
            })
            .collect();
        Self {
            soil, woodlice, next_number: individuals as u64 + 1, born: 0, died: 0,
            initial_woodlice: individuals as u64, introduced_woodlice: 0,
            stocked_woodlice: 0, removed_woodlice: 0, retired_woodlouse_numbers: 0,
        }
    }

    fn valid(&self, animals: &[Animal], allows_stocking: bool) -> bool {
        let Some(initial) = self.initial_woodlice.checked_add(self.introduced_woodlice) else { return false };
        let Some(population) = initial.checked_add(self.stocked_woodlice).and_then(|total| total.checked_add(self.born)) else { return false };
        if (!allows_stocking && (self.stocked_woodlice > 0 || self.removed_woodlice > 0 || self.retired_woodlouse_numbers > 0))
            || (initial != 0 && initial != typical_initial_woodlice())
            || (self.initial_woodlice > 0 && self.introduced_woodlice > 0)
            || self.soil.len() != CELL_COUNT
            || !self.soil.iter().all(|cell| {
                cell.springtails.valid() && cell.fern.valid()
                    && fraction(cell.fern_stress) && fraction(cell.moss_stress)
            })
            || self.woodlice.len() != animals.len()
            || animals.len() > MAX_WOODLICE
            || population.checked_add(self.retired_woodlouse_numbers).and_then(|total| total.checked_add(1)) != Some(self.next_number)
            || population.checked_sub(self.died).and_then(|live| live.checked_sub(self.removed_woodlice)) != Some(animals.len() as u64)
        {
            return false;
        }
        let mut previous = 0;
        for (life, animal) in self.woodlice.iter().zip(animals) {
            if life.number <= previous || life.number >= self.next_number || animal.id != format!("isopod-{}", life.number)
                || life.age_ticks >= MAX_AGE_TICKS
                || life.hungry_ticks >= STARVATION_TICKS || life.dry_ticks >= DRY_STRESS_TICKS
                || !life.brood.valid() || !fraction(life.brood_progress)
                || (life.brood.mg_c == 0.0 && (life.brood_progress != 0.0 || life.brood != Matter::default()))
                || life.cooldown_ticks > BROOD_COOLDOWN_TICKS
                || (life.stage == WoodlouseStage::Juvenile && life.brood.mg_c != 0.0)
                || (life.stage == WoodlouseStage::Adult && life.age_ticks < JUVENILE_MINIMUM_TICKS)
            {
                return false;
            }
            previous = life.number;
        }
        true
    }

    fn observe(&self, soil: &[SoilCell], brains: &[Woodlouse], animals: &[Animal]) -> ForestLifeObservation {
        let woodlice = self.woodlice.iter().zip(brains).zip(animals).map(|((life, brain), animal)| {
            WoodlouseLifeObservation {
                id: animal.id.clone(),
                stage: life.stage,
                age_seconds: life.age_ticks as f64 * STEP_S,
                biomass_mg_c: brain.body.mg_c,
                brood_progress: (life.brood.mg_c > 0.0).then_some(life.brood_progress),
            }
        }).collect();
        let mut observation = ForestLifeObservation {
            woodlice,
            woodlice_born: self.born,
            woodlice_died: self.died,
            springtail_mg_c: [0.0; 2],
            fern_mg_c: [0.0; 2],
            moss_mg_c: [0.0; 2],
            fern_stress_fraction: [0.0; 2],
            moss_stress_fraction: [0.0; 2],
        };
        let mut fern_counts = [0; 2];
        for (index, (seasonal, cell)) in self.soil.iter().zip(soil).enumerate() {
            let side = side_of(index);
            observation.springtail_mg_c[side] += seasonal.springtails.mg_c;
            observation.fern_mg_c[side] += seasonal.fern.mg_c;
            observation.moss_mg_c[side] += cell.moss.mg_c;
            if fern_patch(index) || seasonal.fern.mg_c > 0.0 {
                observation.fern_stress_fraction[side] += seasonal.fern_stress;
                fern_counts[side] += 1;
            }
            observation.moss_stress_fraction[side] += seasonal.moss_stress / (CELL_COUNT / 2) as f64;
        }
        for (stress, count) in observation.fern_stress_fraction.iter_mut().zip(fern_counts) {
            *stress /= f64::from(count.max(1));
        }
        observation
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ForestState {
    soil: Vec<SoilCell>,
    woodlice: Vec<Woodlouse>,
    balance: Balance,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    seasonal: Option<SeasonalForest>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ForestFaunaEditState {
    pub animals: Vec<Animal>,
    brains: Vec<Woodlouse>,
    lives: Vec<WoodlouseLife>,
    next_number: u64,
    stocked_woodlice: u64,
    removed_woodlice: u64,
    retired_woodlouse_numbers: u64,
}

#[derive(Clone, Copy)]
pub(super) struct Site {
    shade: f64,
    shelter: f64,
    walkable: bool,
}

#[derive(Clone, Copy)]
struct SoilForcing {
    water_input_ml_per_hour: f64,
    evaporation_factor: f64,
    biological_activity: f64,
    decomposition_factor: f64,
    respiration_factor: f64,
}

impl SoilForcing {
    const LEGACY: Self = Self {
        water_input_ml_per_hour: RAIN_ML_PER_CELL_H,
        evaporation_factor: 1.0,
        biological_activity: 1.0,
        decomposition_factor: 1.0,
        respiration_factor: 1.0,
    };

    fn seasonal(now: SeasonObservation) -> Self {
        let warmth = ((now.air_temperature_celsius - 5.0) / 18.0).clamp(0.0, 1.0);
        Self {
            // Equivalent continuous external watering. Every added mL is in the water ledger.
            water_input_ml_per_hour: 3.5 + 8.0 * (now.relative_humidity - 0.72),
            evaporation_factor: (0.65 + 0.60 * warmth) * (0.28 + 1.20 * (1.0 - now.relative_humidity)),
            biological_activity: 0.30 + 0.70 * warmth,
            decomposition_factor: 0.045 + 0.045 * warmth,
            respiration_factor: 0.10 + 0.06 * warmth,
        }
    }
}

fn nonnegative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn fraction(value: f64) -> bool {
    (0.0..=1.0).contains(&value)
}

fn fern_patch(index: usize) -> bool {
    (index / COLUMNS + 2 * (index % COLUMNS)).is_multiple_of(4)
}

pub(super) fn cell_index(point: Point) -> usize {
    let column = (((point.x - X_MIN) / CELL_WIDTH).floor() as usize).min(COLUMNS - 1);
    let row = (((point.y - Y_MIN) / CELL_HEIGHT).floor() as usize).min(ROWS - 1);
    row * COLUMNS + column
}

pub(super) fn on_ground(point: Point) -> bool {
    (X_MIN..=X_MAX).contains(&point.x) && (Y_MIN..=Y_MAX).contains(&point.y)
}

fn cell_center(index: usize) -> Point {
    Point {
        x: X_MIN + (index % COLUMNS) as f64 * CELL_WIDTH + CELL_WIDTH / 2.0,
        y: Y_MIN + (index / COLUMNS) as f64 * CELL_HEIGHT + CELL_HEIGHT / 2.0,
    }
}

fn cardinal(index: usize) -> [Option<usize>; 4] {
    let col = index % COLUMNS;
    let row = index / COLUMNS;
    [
        (col > 0).then(|| index - 1),
        (col + 1 < COLUMNS).then(|| index + 1),
        (row > 0).then(|| index - COLUMNS),
        (row + 1 < ROWS).then(|| index + COLUMNS),
    ]
}

fn side_of(index: usize) -> usize {
    usize::from(index % COLUMNS >= COLUMNS / 2)
}

pub(super) fn log_cells(anchor: Point) -> RangeInclusive<usize> {
    let left = cell_index(Point { x: anchor.x - LOG_HALF_SPAN, y: anchor.y });
    let right = cell_index(Point { x: anchor.x + LOG_HALF_SPAN, y: anchor.y });
    left..=right
}

#[derive(Clone, Copy)]
enum GroundCover {
    Rock,
    Log,
    Stonework,
}

#[derive(Clone, Copy)]
pub(super) struct ForestTerrain<'a> {
    pub legacy_rock: Point,
    pub layout: Option<&'a ForestRockLayout>,
}

impl ForestTerrain<'_> {
    pub(super) fn geometry(self) -> [Site; CELL_COUNT] {
        let logs = self.layout.and_then(ForestRockLayout::logs).unwrap_or_default();
        let stonework = self.layout.and_then(ForestRockLayout::stonework).unwrap_or_default();
        let shelters = self.layout.and_then(ForestRockLayout::bark_shelters).unwrap_or_default();
        site_geometry(forest_rocks::positions(self.legacy_rock, self.layout), logs, stonework, shelters)
    }

    pub(super) fn walkable_at(self, point: Point) -> bool {
        self.geometry()[cell_index(point)].walkable
    }

    fn first_rock_cell(self) -> usize {
        forest_rocks::positions(self.legacy_rock, self.layout).next().map_or(0, cell_index)
    }
}

fn site_geometry(
    rocks: impl Iterator<Item = Point>, logs: &[ForestLog], stonework: &[ForestStonework], shelters: &[ForestBarkShelter],
) -> [Site; CELL_COUNT] {
    let mut geometry = [Site { shade: OPEN_GROUND_SHADE, shelter: 0.0, walkable: true }; CELL_COUNT];
    let stonework_contacts = stonework.iter().flat_map(|piece| piece.contact_positions());
    let structures = rocks.map(|position| (position, GroundCover::Rock))
        .chain(logs.iter().map(|log| (log.position, GroundCover::Log)))
        .chain(stonework_contacts.map(|position| (position, GroundCover::Stonework)));
    for (position, cover) in structures {
        let solid = if matches!(cover, GroundCover::Log) {
            log_cells(position)
        } else {
            let cell = cell_index(position);
            cell..=cell
        };
        let (solid_shade, nearby_strength) = match cover {
            GroundCover::Rock | GroundCover::Log => (0.91, 1.0),
            GroundCover::Stonework => (0.76, 0.65),
        };
        for (index, site) in geometry.iter_mut().enumerate() {
            if solid.contains(&index) {
                *site = Site { shade: site.shade.max(solid_shade), shelter: 0.0, walkable: false };
            }
            if !site.walkable {
                continue;
            }
            let centre = cell_center(index);
            let dx = centre.x - position.x;
            // A log shades the ground around its full horizontal span.
            let dx = if matches!(cover, GroundCover::Log) { (dx.abs() - LOG_HALF_SPAN).max(0.0) } else { dx };
            let dy = centre.y - position.y;
            let shade = (1.0 - (dx / 0.20).hypot(dy / 0.17)).max(0.0);
            let shelter = (1.0 - (dx / 0.25).hypot(dy / 0.19)).max(0.0);
            site.shade = site.shade.max(0.12 + 0.78 * (shade * nearby_strength));
            site.shelter = site.shelter.max(shelter * nearby_strength);
        }
    }
    for shelter in shelters {
        for foot in shelter.foot_cells() {
            let site = &mut geometry[foot];
            *site = Site { shade: site.shade.max(0.91), shelter: 0.0, walkable: false };
        }
        let center = &mut geometry[shelter.passage_cells()[1]];
        // Placement reserves both entrances. The roof adds cover without blocking ground.
        if center.walkable {
            center.shade = center.shade.max(0.93);
            center.shelter = 1.0;
        }
    }
    geometry
}

fn animal_point(animal: &Animal) -> Point {
    Point { x: animal.x, y: animal.y }
}

pub(super) fn validate_rock_change(rock: Point, animals: &[Animal]) -> Result<(), CoreError> {
    if !(0.085..=0.915).contains(&rock.x) || !(0.60..=0.91).contains(&rock.y) {
        return Err(CoreError::InvalidRockSite);
    }
    let blocked = cell_index(rock);
    if animals.iter().any(|animal| cell_index(animal_point(animal)) == blocked) {
        return Err(CoreError::OccupiedRockSite);
    }
    Ok(())
}

pub(super) fn clear_initial_rock_cell(rock: Point, animals: &mut [Animal]) {
    let blocked = cell_index(rock);
    let adjacent = cardinal(blocked).into_iter().flatten().next().expect("a ground cell has a neighbor");
    let destination = cell_center(adjacent);
    for animal in animals.iter_mut().filter(|animal| cell_index(animal_point(animal)) == blocked) {
        animal.x = destination.x;
        animal.y = destination.y;
    }
}

pub(super) fn clear_initial_structure_cells(
    rocks: impl Iterator<Item = Point>, logs: &[ForestLog], stonework: &[ForestStonework],
    shelters: &[ForestBarkShelter], animals: &mut [Animal],
) {
    let mut blocked = [false; CELL_COUNT];
    for rock in rocks { blocked[cell_index(rock)] = true; }
    for cell in logs.iter().flat_map(|log| log_cells(log.position)) {
        blocked[cell] = true;
    }
    for position in stonework.iter().flat_map(|piece| piece.contact_positions()) {
        blocked[cell_index(position)] = true;
    }
    for cell in shelters.iter().flat_map(|shelter| shelter.foot_cells()) {
        blocked[cell] = true;
    }
    for animal in animals {
        let cell = cell_index(animal_point(animal));
        if blocked[cell] {
            let target = cardinal(cell).into_iter().flatten().chain(0..CELL_COUNT)
                .find(|index| !blocked[*index]).expect("the forest retains unoccupied ground");
            let position = cell_center(target);
            animal.x = position.x;
            animal.y = position.y;
        }
    }
}

fn daylight(phase: f64) -> f64 {
    ((phase - 0.5) * TAU).cos().max(0.0)
}

fn evaporation_rate(cell: &SoilCell, site: Site, light: f64) -> f64 {
    EVAPORATION_ML_PER_CELL_H * (0.40 + 0.60 * light) * (1.0 - 0.83 * site.shade) * cell.saturation()
        * cell.substrate.profile().evaporation_multiplier
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ForestMatterFlows {
    inputs: Matter,
    outputs: Matter,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ForestGroundEditState(Vec<GroundCellEditState>);

#[derive(Clone, Debug, PartialEq)]
struct GroundCellEditState {
    substrate: ForestSubstrate,
    moss: Matter,
    litter: Matter,
    fern: Matter,
}

impl ForestState {
    pub(super) fn new(individuals: usize) -> Self {
        let mut forest = Self {
            soil: (0..CELL_COUNT).map(SoilCell::new).collect(),
            woodlice: (0..individuals).map(|_| Woodlouse::new()).collect(),
            balance: Balance::default(),
            seasonal: None,
        };
        forest.balance.initial = forest.inventory(&[]);
        forest
    }

    pub(super) fn new_seasonal(rock: Point, animals: &[Animal]) -> Self {
        let mut forest = Self::new(animals.len());
        forest.seasonal = Some(SeasonalForest::new(animals.len()));
        if let Some(juvenile) = forest.woodlice.last_mut() {
            juvenile.body = juvenile.body.scaled(0.24);
            juvenile.reserve = juvenile.reserve.scaled(0.65);
        }
        debug_assert!(validate_rock_change(rock, animals).is_ok());
        forest.balance.initial = forest.inventory(&[]);
        forest
    }

    pub(super) fn new_blank(rock: Point) -> Self {
        let mut forest = Self::new_seasonal(rock, &[]);
        for cell in &mut forest.soil {
            cell.litter = Matter::default();
            cell.microbes = Matter::default();
            cell.moss = Matter::default();
        }
        for cell in &mut forest.seasonal.as_mut().expect("blank forest is seasonal").soil {
            cell.springtails = Matter::default();
            cell.fern = Matter::default();
        }
        forest.balance.initial = forest.inventory(&[]);
        forest
    }

    pub(super) fn introduce_starter(&mut self, rock: Point, animals: &mut [Animal],
        ground_plants: (&mut [ForestGroundPlant], &mut [ForestGroundPlant])) -> Result<(), CoreError> {
        let first_number = self.seasonal.as_ref().expect("a blank forest has seasonal state").next_number;
        let next_number = first_number.checked_add(animals.len() as u64).ok_or(CoreError::ForestWoodlouseLimit)?;
        let before = self.inventory(&[]).matter;
        let starter = Self::new_seasonal(rock, animals);
        for (kind, patches) in [(ForestPlantKind::Moss, ground_plants.0), (ForestPlantKind::Fern, ground_plants.1)] {
            for patch in patches {
                let cell = patch.cell_index();
                let existing_c = self.plant_stock(kind, cell).mg_c;
                patch.share = plant_share(existing_c * patch.share, existing_c + starter.plant_stock(kind, cell).mg_c);
            }
        }
        for (existing, added) in self.soil.iter_mut().zip(starter.soil) {
            existing.litter.add(added.litter);
            existing.microbes.add(added.microbes);
            existing.moss.add(added.moss);
        }
        self.woodlice = starter.woodlice;
        let lifecycle = self.seasonal.as_mut().expect("a blank forest has seasonal state");
        let added = starter.seasonal.expect("the starter has seasonal state");
        for (existing, added) in lifecycle.soil.iter_mut().zip(added.soil) {
            existing.springtails.add(added.springtails);
            existing.fern.add(added.fern);
            existing.moss_stress = 0.0;
            existing.fern_stress = 0.0;
        }
        lifecycle.woodlice = added.woodlice;
        if first_number != 1 {
            for ((life, animal), number) in lifecycle.woodlice.iter_mut().zip(animals.iter_mut()).zip(first_number..next_number) {
                life.number = number;
                animal.id = format!("isopod-{number}");
            }
        }
        lifecycle.introduced_woodlice = animals.len() as u64;
        lifecycle.next_number = next_number;
        let after = self.inventory(&[]).matter;
        self.balance.inputs.matter.add(Matter::new(
            after.mg_c - before.mg_c, after.mg_n - before.mg_n, after.mg_p - before.mg_p,
        ));
        Ok(())
    }

    pub(super) fn has_stocked_woodlice(&self) -> bool {
        self.seasonal.as_ref().is_some_and(|seasonal| seasonal.stocked_woodlice > 0)
    }

    pub(super) fn stock_woodlice(&mut self, animals: &mut Vec<Animal>, position: Point, count: u32) -> Result<(), CoreError> {
        if count == 0 { return Err(CoreError::InvalidWoodlouseStockCount); }
        if count as usize > MAX_WOODLICE.saturating_sub(animals.len()) {
            return Err(CoreError::ForestWoodlouseLimit);
        }
        let seasonal = self.seasonal.as_mut().ok_or(CoreError::WrongWorldKind)?;
        let first = seasonal.next_number;
        let end = first.checked_add(u64::from(count)).ok_or(CoreError::ForestWoodlouseLimit)?;
        let stocked = seasonal.stocked_woodlice.checked_add(u64::from(count)).ok_or(CoreError::ForestWoodlouseLimit)?;
        let cell = cell_index(position);
        let brain = Woodlouse { next_cell: cell, ..Woodlouse::new() };
        for (index, number) in (first..end).enumerate() {
            let heading = index as f64 * TAU / f64::from(count);
            let at = if index == 0 { position } else {
                let center = cell_center(cell);
                let inset_x = CELL_WIDTH / 2.0 - 1e-9;
                let inset_y = CELL_HEIGHT / 2.0 - 1e-9;
                Point {
                    x: (position.x + 0.018 * heading.cos()).clamp(center.x - inset_x, center.x + inset_x),
                    y: (position.y + 0.014 * heading.sin()).clamp(center.y - inset_y, center.y + inset_y),
                }
            };
            animals.push(Animal { id: format!("isopod-{number}"), x: at.x, y: at.y, heading, activity: Activity::Resting });
            self.woodlice.push(brain.clone());
            seasonal.woodlice.push(WoodlouseLife::stocked_adult(number));
        }
        seasonal.next_number = end;
        seasonal.stocked_woodlice = stocked;
        self.balance.inputs.matter.add(brain.body.scaled(f64::from(count)));
        self.balance.inputs.matter.add(brain.reserve.scaled(f64::from(count)));
        Ok(())
    }

    pub(super) fn fauna_edit_state(&self, animals: &[Animal]) -> Option<ForestFaunaEditState> {
        let seasonal = self.seasonal.as_ref()?;
        Some(ForestFaunaEditState {
            animals: animals.to_vec(), brains: self.woodlice.clone(), lives: seasonal.woodlice.clone(),
            next_number: seasonal.next_number, stocked_woodlice: seasonal.stocked_woodlice,
            removed_woodlice: seasonal.removed_woodlice,
            retired_woodlouse_numbers: seasonal.retired_woodlouse_numbers,
        })
    }

    pub(super) fn restore_fauna_edit_state(&mut self, state: ForestFaunaEditState) -> Result<Vec<Animal>, CoreError> {
        let seasonal = self.seasonal.as_mut().ok_or(CoreError::WrongWorldKind)?;
        let next_number = seasonal.next_number.max(state.next_number);
        // Undo removes the introduced animals without making their numbers available again.
        let removed_allocations = next_number - state.next_number;
        seasonal.retired_woodlouse_numbers = state.retired_woodlouse_numbers.checked_add(removed_allocations)
            .ok_or(CoreError::InvalidForestState)?;
        seasonal.next_number = next_number;
        seasonal.stocked_woodlice = state.stocked_woodlice;
        seasonal.removed_woodlice = state.removed_woodlice;
        seasonal.woodlice = state.lives;
        self.woodlice = state.brains;
        Ok(state.animals)
    }

    pub(super) fn remove_woodlouse(&mut self, animals: &mut Vec<Animal>, id: &str) -> Result<(), CoreError> {
        let seasonal = self.seasonal.as_mut().ok_or(CoreError::WrongWorldKind)?;
        let index = animals.iter().position(|animal| animal.id == id).ok_or(CoreError::WoodlouseMissing)?;
        let removed = seasonal.removed_woodlice.checked_add(1).ok_or(CoreError::InvalidForestState)?;
        animals.remove(index);
        let brain = self.woodlice.remove(index);
        let life = seasonal.woodlice.remove(index);
        let mut outgoing = brain.body;
        outgoing.add(brain.reserve);
        outgoing.add(life.brood);
        self.balance.outputs.matter.add(outgoing);
        seasonal.removed_woodlice = removed;
        Ok(())
    }

    pub(super) fn reset_initial_balance(&mut self, patches: &[ForestLeafPatch]) {
        self.balance = Balance { initial: self.inventory(patches), ..Balance::default() };
    }

    pub(super) fn matter_flows(&self) -> ForestMatterFlows {
        ForestMatterFlows { inputs: self.balance.inputs.matter, outputs: self.balance.outputs.matter }
    }

    pub(super) fn restore_matter_flows(&mut self, flows: ForestMatterFlows) {
        self.balance.inputs.matter = flows.inputs;
        self.balance.outputs.matter = flows.outputs;
    }

    pub(super) fn record_leaf_edit(&mut self, incoming: [f64; 3], outgoing: [f64; 3]) {
        self.balance.inputs.matter.add(Matter::from_values(incoming));
        self.balance.outputs.matter.add(Matter::from_values(outgoing));
    }

    pub(super) fn ground_edit_state(&self) -> Option<ForestGroundEditState> {
        let seasonal = self.seasonal.as_ref()?;
        Some(ForestGroundEditState(self.soil.iter().zip(&seasonal.soil).map(|(cell, plant)| GroundCellEditState {
            substrate: cell.substrate,
            moss: cell.moss,
            litter: cell.litter,
            fern: plant.fern,
        }).collect()))
    }

    pub(super) fn restore_ground_edit_state(&mut self, state: ForestGroundEditState) -> Result<(), CoreError> {
        let seasonal = self.seasonal.as_mut().ok_or(CoreError::WrongWorldKind)?;
        for ((cell, plant), saved) in self.soil.iter_mut().zip(&mut seasonal.soil).zip(state.0) {
            cell.substrate = saved.substrate;
            cell.moss = saved.moss;
            cell.litter = saved.litter;
            plant.fern = saved.fern;
        }
        Ok(())
    }

    pub(super) fn copy_substrate_profiles(&mut self, source: &Self) {
        for (cell, saved) in self.soil.iter_mut().zip(&source.soil) { cell.substrate = saved.substrate; }
    }

    pub(super) fn observe_substrate(&self) -> Vec<ForestSubstrateCellObservation> {
        self.soil.iter().enumerate().map(|(cell_index, cell)| ForestSubstrateCellObservation {
            cell_index,
            position: cell_center(cell_index),
            water_ml: cell.water_ml,
            decomposer_mg_c: cell.microbes.mg_c,
            profile: cell.substrate.profile(),
        }).collect()
    }

    pub(super) fn set_substrate(&mut self, cells: &[usize], substrate: ForestSubstrate) -> Result<(), CoreError> {
        if cells.is_empty() || cells.len() > self.soil.len() { return Err(CoreError::InvalidForestSubstrateCells); }
        let mut visited = [false; CELL_COUNT];
        for &index in cells {
            let seen = visited.get_mut(index).ok_or(CoreError::InvalidForestSubstrateCells)?;
            if *seen { return Err(CoreError::InvalidForestSubstrateCells); }
            *seen = true;
        }
        let capacity = substrate.profile().water_capacity_ml;
        if cells.iter().any(|&index| self.soil[index].water_ml > capacity) {
            return Err(CoreError::ForestSubstrateTooWet);
        }
        if cells.iter().all(|&index| self.soil[index].substrate == substrate) {
            return Err(CoreError::ForestSubstrateUnchanged);
        }
        for &index in cells { self.soil[index].substrate = substrate; }
        Ok(())
    }

    pub(super) fn observe_moss_patch(&self, patch: ForestGroundPlant) -> ForestMossPatchObservation {
        self.observe_ground_plant(ForestPlantKind::Moss, patch)
    }

    pub(super) fn observe_fern_patch(&self, patch: ForestGroundPlant) -> ForestFernPatchObservation {
        let observation = self.observe_ground_plant(ForestPlantKind::Fern, patch);
        ForestFernPatchObservation { patch: observation, canopy_shade_fraction: fern_canopy_shade(observation.stock_mg_cnp[0]) }
    }

    fn observe_ground_plant(&self, kind: ForestPlantKind, patch: ForestGroundPlant) -> ForestGroundPlantObservation {
        let cell_index = patch.cell_index();
        ForestGroundPlantObservation {
            id: patch.id,
            position: patch.position,
            cell_index,
            stock_mg_cnp: self.plant_stock(kind, cell_index).scaled(patch.share).values(),
            stress_fraction: self.seasonal.as_ref().map_or(0.0, |seasonal| match kind {
                ForestPlantKind::Moss => seasonal.soil[cell_index].moss_stress,
                ForestPlantKind::Fern => seasonal.soil[cell_index].fern_stress,
            }),
        }
    }

    fn plant_stock(&self, kind: ForestPlantKind, index: usize) -> Matter {
        match kind {
            ForestPlantKind::Moss => self.soil[index].moss,
            ForestPlantKind::Fern => self.seasonal.as_ref().expect("fern requires seasonal soil").soil[index].fern,
        }
    }

    fn plant_stock_mut(&mut self, kind: ForestPlantKind, index: usize) -> &mut Matter {
        match kind {
            ForestPlantKind::Moss => &mut self.soil[index].moss,
            ForestPlantKind::Fern => &mut self.seasonal.as_mut().expect("fern requires seasonal soil").soil[index].fern,
        }
    }

    pub(super) fn add_ground_plant(&mut self, kind: ForestPlantKind, cell: usize, external_input: bool) -> f64 {
        let dose = kind.initial_stock();
        let stock = self.plant_stock_mut(kind, cell);
        stock.add(dose);
        let share = plant_share(dose.mg_c, stock.mg_c);
        if external_input { self.balance.inputs.matter.add(dose); }
        share
    }

    pub(super) fn move_ground_plant(&mut self, kind: ForestPlantKind, patch: ForestGroundPlant, target: usize) -> f64 {
        let source = patch.cell_index();
        if source == target { return patch.share; }
        let moved = self.plant_stock_mut(kind, source).withdraw_fraction(patch.share);
        let stock = self.plant_stock_mut(kind, target);
        stock.add(moved);
        plant_share(moved.mg_c, stock.mg_c)
    }

    pub(super) fn trim_ground_plant(&mut self, kind: ForestPlantKind, patch: ForestGroundPlant, mg_c: f64) -> Result<f64, CoreError> {
        let index = patch.cell_index();
        let stock = self.plant_stock_mut(kind, index);
        let patch_c = stock.mg_c * patch.share;
        if !mg_c.is_finite() || mg_c <= 0.0 || mg_c >= patch_c {
            return Err(kind.invalid_trim());
        }
        let litter = stock.withdraw(mg_c);
        let share = plant_share(patch_c - mg_c, stock.mg_c);
        self.soil[index].litter.add(litter);
        Ok(share)
    }

    pub(super) fn remove_ground_plant(&mut self, kind: ForestPlantKind, patch: ForestGroundPlant) {
        let outgoing = self.plant_stock_mut(kind, patch.cell_index()).withdraw_fraction(patch.share);
        self.balance.outputs.matter.add(outgoing);
    }

    pub(super) fn interactive_geometry(&self, terrain: ForestTerrain<'_>) -> [Site; CELL_COUNT] {
        let mut sites = terrain.geometry();
        if let Some(layout) = terrain.layout {
            for patch in layout.fern_patches() {
                let index = patch.cell_index();
                let carbon = self.plant_stock(ForestPlantKind::Fern, index).mg_c * patch.share;
                let shade = fern_canopy_shade(carbon);
                sites[index].shade = sites[index].shade.max(OPEN_GROUND_SHADE + shade);
                for neighbor in cardinal(index).into_iter().flatten() {
                    sites[neighbor].shade = sites[neighbor].shade.max(OPEN_GROUND_SHADE + shade * 0.5);
                }
            }
        }
        sites
    }

    pub(super) fn is_seasonal(&self) -> bool {
        self.seasonal.is_some()
    }

    pub(super) fn validate(&self, rock: Point, animals: &[Animal]) -> Result<(), CoreError> {
        validate_rock_change(rock, animals).map_err(|_| CoreError::InvalidForestState)?;
        self.validate_inventory(animals, &[], false)
    }

    pub(super) fn validate_inventory(&self, animals: &[Animal], patches: &[ForestLeafPatch], allows_composed_edits: bool) -> Result<(), CoreError> {
        if self.soil.len() != CELL_COUNT
            || !self.soil.iter().all(SoilCell::valid)
            || (!allows_composed_edits && self.soil.iter().any(|cell| !cell.substrate.is_native()))
            || self.woodlice.len() != animals.len()
            || !self.woodlice.iter().all(|brain| {
                brain.body.valid() && brain.reserve.valid() && brain.next_cell < CELL_COUNT
                    && brain.decision_remaining <= DECISION_TICKS
            })
            || !animals.iter().all(|animal| on_ground(animal_point(animal)))
            || !self.balance.initial.valid() || !self.balance.inputs.valid() || !self.balance.outputs.valid()
            || ![self.balance.fed_mg_c, self.balance.decomposed_mg_c, self.balance.photosynthesized_mg_c].into_iter().all(nonnegative)
            || self.seasonal.as_ref().is_some_and(|seasonal| !seasonal.valid(animals, allows_composed_edits))
        {
            return Err(CoreError::InvalidForestState);
        }
        let ledger = self.ledger(patches);
        let within_tolerance = |net: f64, initial: f64, incoming: f64, outgoing: f64| {
            net.is_finite() && net.abs() <= 1e-7 + 1e-9 * (initial + incoming + outgoing)
        };
        if !within_tolerance(ledger.water_residual_ml, ledger.initial_water_ml, ledger.water_input_ml, ledger.water_output_ml)
            || (0..3).any(|element| !within_tolerance(
                ledger.matter_residual_mg_cnp[element], ledger.initial_matter_mg_cnp[element],
                ledger.matter_input_mg_cnp[element], ledger.matter_output_mg_cnp[element],
            ))
        {
            return Err(CoreError::InvalidForestState);
        }
        Ok(())
    }

    fn inventory(&self, patches: &[ForestLeafPatch]) -> Inventory {
        let mut inventory = Inventory::default();
        for cell in &self.soil {
            inventory.water_ml += cell.water_ml;
            for parcel in [cell.litter, cell.waste, cell.microbes, cell.moss] {
                inventory.matter.add(parcel);
            }
            inventory.matter.mg_n += cell.mineral_n_mg;
            inventory.matter.mg_p += cell.mineral_p_mg;
        }
        for animal in &self.woodlice {
            inventory.matter.add(animal.body);
            inventory.matter.add(animal.reserve);
        }
        if let Some(seasonal) = &self.seasonal {
            for cell in &seasonal.soil {
                inventory.matter.add(cell.springtails);
                inventory.matter.add(cell.fern);
            }
            for life in &seasonal.woodlice {
                inventory.matter.add(life.brood);
            }
        }
        for patch in patches {
            inventory.matter.add(Matter::from_values(patch.remaining_mg_cnp));
        }
        inventory
    }

    pub(super) fn ledger(&self, patches: &[ForestLeafPatch]) -> ForestLedger {
        let current = self.inventory(patches);
        let initial = self.balance.initial;
        let inputs = self.balance.inputs;
        let outputs = self.balance.outputs;
        let initial_matter = initial.matter.values();
        let input_matter = inputs.matter.values();
        let output_matter = outputs.matter.values();
        let current_matter = current.matter.values();
        ForestLedger {
            initial_water_ml: initial.water_ml,
            current_water_ml: current.water_ml,
            water_input_ml: inputs.water_ml,
            water_output_ml: outputs.water_ml,
            water_residual_ml: initial.water_ml + inputs.water_ml - outputs.water_ml - current.water_ml,
            initial_matter_mg_cnp: initial_matter,
            current_matter_mg_cnp: current_matter,
            matter_input_mg_cnp: input_matter,
            matter_output_mg_cnp: output_matter,
            matter_residual_mg_cnp: std::array::from_fn(|element| {
                initial_matter[element] + input_matter[element] - output_matter[element] - current_matter[element]
            }),
            fed_mg_c: self.balance.fed_mg_c,
            decomposed_mg_c: self.balance.decomposed_mg_c,
            photosynthesized_mg_c: self.balance.photosynthesized_mg_c,
        }
    }

    fn water_and_capacity_by_side(&self) -> ([f64; 2], [f64; 2]) {
        let mut water = [0.0; 2];
        let mut capacities = [0.0; 2];
        for (index, cell) in self.soil.iter().enumerate() {
            let side = side_of(index);
            water[side] += cell.water_ml;
            capacities[side] += cell.water_capacity_ml();
        }
        (water, capacities)
    }

    pub(super) fn moisture(&self) -> [f64; 2] {
        let (water, capacity) = self.water_and_capacity_by_side();
        std::array::from_fn(|side| (water[side] / capacity[side]).clamp(0.0, 1.0))
    }

    pub(super) fn observe(&self, rock: Point, phase: f64, animals: &[Animal]) -> ForestObservation {
        let terrain = ForestTerrain { legacy_rock: rock, layout: None };
        self.observe_with_forcing(terrain, phase, animals, SoilForcing::LEGACY)
    }

    pub(super) fn observe_seasonal(
        &self, terrain: ForestTerrain<'_>, phase: f64, animals: &[Animal], now: SeasonObservation,
    ) -> ForestObservation {
        self.observe_with_forcing(terrain, phase, animals, SoilForcing::seasonal(now))
    }

    fn observe_with_forcing(
        &self, terrain: ForestTerrain<'_>, phase: f64, animals: &[Animal], forcing: SoilForcing,
    ) -> ForestObservation {
        let geometry = self.interactive_geometry(terrain);
        let count = (CELL_COUNT / 2) as f64;
        let (water_ml, water_capacity_ml) = self.water_and_capacity_by_side();
        let mut observation = ForestObservation {
            water_ml,
            water_capacity_ml,
            shade_fraction: [0.0; 2],
            shelter_fraction: [0.0; 2],
            evaporation_ml_per_hour: [0.0; 2],
            litter_mg_c: 0.0,
            soil_nitrogen_mg: 0.0,
            soil_phosphorus_mg: 0.0,
            woodlice_by_side: [0; 2],
            sheltered_woodlice_by_side: [0; 2],
            feeding_woodlice_by_side: [0; 2],
            walkable_cells: [0; 2],
            rock_cell: terrain.first_rock_cell(),
            life: self.seasonal.as_ref().map(|state| state.observe(&self.soil, &self.woodlice, animals)),
        };
        let light = daylight(phase);
        for (index, (cell, site)) in self.soil.iter().zip(geometry).enumerate() {
            let side = side_of(index);
            observation.shade_fraction[side] += site.shade / count;
            observation.shelter_fraction[side] += site.shelter / count;
            observation.evaporation_ml_per_hour[side] += evaporation_rate(cell, site, light) * forcing.evaporation_factor;
            observation.litter_mg_c += cell.litter.mg_c;
            observation.soil_nitrogen_mg += cell.mineral_n_mg;
            observation.soil_phosphorus_mg += cell.mineral_p_mg;
            observation.walkable_cells[side] += u32::from(site.walkable);
        }
        if let Some(patches) = terrain.layout.and_then(ForestRockLayout::leaf_patches) {
            observation.litter_mg_c += patches.iter().map(|patch| patch.remaining_mg_cnp[0]).sum::<f64>();
        }
        for animal in animals {
            let side = side_of(cell_index(animal_point(animal)));
            observation.woodlice_by_side[side] += 1;
            observation.sheltered_woodlice_by_side[side] += u32::from(animal.activity == Activity::Sheltering);
            observation.feeding_woodlice_by_side[side] += u32::from(animal.activity == Activity::Feeding);
        }
        observation
    }

    pub(super) fn advance(&mut self, rock: Point, phase: f64, animals: &mut [Animal], random: &mut RandomState) {
        let terrain = ForestTerrain { legacy_rock: rock, layout: None };
        self.advance_resources(terrain.geometry(), &mut [], phase, animals, random, SoilForcing::LEGACY);
    }

    pub(super) fn advance_seasonal(
        &mut self, geometry: [Site; CELL_COUNT], patches: &mut [ForestLeafPatch], phase: f64,
        animals: &mut Vec<Animal>, random: &mut RandomState, now: SeasonObservation,
    ) {
        let forcing = SoilForcing::seasonal(now);
        self.advance_resources(geometry, patches, phase, animals, random, forcing);
        self.update_life(animals, now);
    }

    fn advance_resources(
        &mut self, geometry: [Site; CELL_COUNT], patches: &mut [ForestLeafPatch], phase: f64,
        animals: &mut [Animal], random: &mut RandomState, forcing: SoilForcing,
    ) {
        let light = daylight(phase);
        let mut patch_indices = [None; CELL_COUNT];
        let mut patch_food = [0.0; CELL_COUNT];
        for (index, patch) in patches.iter().enumerate() {
            let cell = patch.cell_index();
            patch_indices[cell] = Some(index);
            patch_food[cell] = patch.remaining_mg_cnp[0];
        }
        let mut demand = [0.0; CELL_COUNT];
        let mut intentions = [(0, 0.0); MAX_WOODLICE];
        for ((animal, brain), intention) in animals.iter_mut().zip(&mut self.woodlice).zip(&mut intentions) {
            let request = step_animal(animal, brain, &self.soil, &patch_food, &geometry, light, random);
            let cell = cell_index(animal_point(animal));
            demand[cell] += request;
            *intention = (cell, request);
        }

        let mut consumption = [Matter::default(); CELL_COUNT];
        for (index, cell) in self.soil.iter_mut().enumerate() {
            let seasonal = self.seasonal.as_mut().map(|state| &mut state.soil[index]);
            let patch = patch_indices[index].map(|patch| &mut patches[patch]);
            let local = LocalSoil { cell, seasonal, patch };
            consumption[index] = step_cell(local, geometry[index], light, demand[index], forcing, &mut self.balance);
        }

        for ((animal, brain), (cell, request)) in animals.iter_mut().zip(&mut self.woodlice).zip(intentions) {
            let local = &mut self.soil[cell];
            if request > 0.0 && demand[cell] > 0.0 {
                let portion = consumption[cell].scaled(request / demand[cell]);
                if portion.mg_c > 0.0 {
                    brain.reserve.add(portion.scaled(0.39));
                    local.waste.add(portion.scaled(0.61));
                    self.balance.fed_mg_c += portion.mg_c;
                    animal.activity = Activity::Feeding;
                }
            }
            let spent = brain.reserve.withdraw(RESPIRATION_MG_C_PER_S * STEP_S * forcing.respiration_factor);
            self.balance.outputs.matter.mg_c += spent.mg_c;
            if self.seasonal.is_some() {
                local.mineral_n_mg += spent.mg_n;
                local.mineral_p_mg += spent.mg_p;
            } else {
                local.waste.add(Matter::new(0.0, spent.mg_n, spent.mg_p));
            }
        }
    }

    fn update_life(&mut self, animals: &mut Vec<Animal>, now: SeasonObservation) {
        let Some(seasonal) = &mut self.seasonal else { return };
        let mut deaths = Vec::new();
        let mut births = Vec::new();
        let parent_count = animals.len();
        for (index, ((animal, brain), life)) in animals.iter().zip(&mut self.woodlice).zip(&mut seasonal.woodlice).enumerate() {
            let cell_index = cell_index(animal_point(animal));
            let local = &self.soil[cell_index];
            if !life.advance_age_and_condition(brain, local.saturation()) {
                deaths.push((index, cell_index));
                continue;
            }
            let Some(next_number) = seasonal.next_number.checked_add(1) else { continue };
            let can_recruit = parent_count + births.len() < MAX_WOODLICE;
            if let Some(body) = life.reproduce(brain, local.saturation(), now.air_temperature_celsius, can_recruit) {
                births.push((seasonal.next_number, animal.clone(), body));
                seasonal.next_number = next_number;
                seasonal.born += 1;
            }
        }
        // Remove tail first to preserve the index pairing and stable order of surviving IDs.
        for (index, cell_index) in deaths.into_iter().rev() {
            animals.remove(index);
            let brain = self.woodlice.remove(index);
            let life = seasonal.woodlice.remove(index);
            let local = &mut self.soil[cell_index];
            local.litter.add(brain.body);
            local.litter.add(brain.reserve);
            local.litter.add(life.brood);
            seasonal.died += 1;
        }
        for (number, mut animal, body) in births {
            let position = animal_point(&animal);
            animal.id = format!("isopod-{number}");
            animal.activity = Activity::Resting;
            animals.push(animal);
            self.woodlice.push(Woodlouse {
                body: body.scaled(0.80),
                reserve: body.scaled(0.20),
                next_cell: cell_index(position),
                decision_remaining: 0,
            });
            seasonal.woodlice.push(WoodlouseLife::juvenile(number));
        }
    }
}

impl WoodlouseLife {
    fn advance_age_and_condition(&mut self, brain: &mut Woodlouse, wetness: f64) -> bool {
        self.age_ticks += 1;
        self.hungry_ticks = if brain.reserve.mg_c < 0.014 { self.hungry_ticks + 1 } else { 0 };
        self.dry_ticks = if wetness < 0.22 { self.dry_ticks + 1 } else { self.dry_ticks.saturating_sub(2) };
        if self.age_ticks >= MAX_AGE_TICKS || self.hungry_ticks >= STARVATION_TICKS || self.dry_ticks >= DRY_STRESS_TICKS {
            return false;
        }
        let available_to_grow = (brain.reserve.mg_c - 1.10).max(0.0);
        let growth = available_to_grow.min(0.000_10 * STEP_S).min((MAX_BODY_MG_C - brain.body.mg_c).max(0.0));
        brain.body.add(brain.reserve.withdraw(growth));
        if self.age_ticks >= JUVENILE_MINIMUM_TICKS && brain.body.mg_c >= ADULT_BODY_MG_C {
            self.stage = WoodlouseStage::Adult;
        }
        true
    }

    fn reproduce(&mut self, brain: &mut Woodlouse, wetness: f64, temperature: f64, can_recruit: bool) -> Option<Matter> {
        self.cooldown_ticks = self.cooldown_ticks.saturating_sub(1);
        if self.stage != WoodlouseStage::Adult || wetness < 0.42 || temperature < 12.0 {
            return None;
        }
        if self.brood.mg_c == 0.0 && self.cooldown_ticks == 0 && brain.reserve.mg_c >= 2.70 {
            self.brood = brain.reserve.withdraw(BROOD_MG_C);
        }
        if self.brood.mg_c == 0.0 {
            return None;
        }
        let development_rate = (temperature - 8.0) / 12.0;
        self.brood_progress = (self.brood_progress + development_rate / BROOD_TICKS as f64).min(1.0);
        if self.brood_progress < 1.0 || !can_recruit {
            return None;
        }
        self.brood_progress = 0.0;
        self.cooldown_ticks = BROOD_COOLDOWN_TICKS;
        Some(std::mem::take(&mut self.brood))
    }
}

impl SeasonalSoil {
    fn advance(&mut self, cell: &mut SoilCell, site: Site, light: f64, forcing: SoilForcing, balance: &mut Balance) {
        let wetness = cell.saturation();
        let forage = ((wetness - 0.18) / 0.34).clamp(0.0, 1.0);
        // Microbial turnover already took its share. Grazing takes only still-present microbes.
        let grazed = cell.microbes.withdraw(self.springtails.mg_c * 0.000_08 * forage * forcing.biological_activity * STEP_S);
        self.springtails.add(grazed.scaled(0.32));
        cell.waste.add(grazed.scaled(0.68));
        let springtail_detritus = self.springtails.withdraw(self.springtails.mg_c * (0.000_008 + 0.000_03 * (1.0 - forage)) * STEP_S);
        cell.litter.add(springtail_detritus);

        let moss_wetness = plant_stress(&mut self.moss_stress, wetness, 0.26);
        let moss_potential = 0.003 * light * (0.52 + 0.48 * site.shade) * moss_wetness * forcing.biological_activity
            * plant_cover(cell.moss.mg_c, 20.0) * STEP_S;
        let moss_uptake = cell.plant_uptake(moss_potential, 0.04, 0.006, balance);
        cell.moss.add(moss_uptake);
        let moss_litter = plant_litter(&mut cell.moss, self.moss_stress, 0.000_028);
        cell.litter.add(moss_litter);

        if self.fern.mg_c > 0.0 {
            let fern_wetness = plant_stress(&mut self.fern_stress, wetness, 0.39);
            let filtered_light = (1.0 - 1.7 * (site.shade - 0.40).abs()).max(0.0);
            let fern_potential = 0.003_8 * light * filtered_light * fern_wetness * forcing.biological_activity
                * plant_cover(self.fern.mg_c, 48.0) * STEP_S;
            self.fern.add(cell.plant_uptake(fern_potential, 0.036, 0.005, balance));
            let fern_litter = plant_litter(&mut self.fern, self.fern_stress, 0.000_047);
            cell.litter.add(fern_litter);
        }
    }
}

fn plant_stress(stress: &mut f64, wetness: f64, tolerance: f64) -> f64 {
    let available = ((wetness - tolerance) / 0.28).clamp(0.0, 1.0);
    let deficit = ((tolerance - wetness) / tolerance).clamp(0.0, 1.0);
    *stress = (*stress + (deficit - available * 0.55) * STEP_S / 2_400.0).clamp(0.0, 1.0);
    available * (1.0 - *stress)
}

fn plant_cover(biomass: f64, ceiling: f64) -> f64 {
    (biomass / 4.0).min(1.0) * (1.0 - biomass / ceiling).max(0.0)
}

fn plant_share(patch_c: f64, cell_c: f64) -> f64 {
    if cell_c > 0.0 { (patch_c / cell_c).clamp(0.0, 1.0) } else { 0.0 }
}

fn fern_canopy_shade(carbon: f64) -> f64 {
    FERN_MAX_ADDED_SHADE * (carbon / FERN_STARTER_DOSE.mg_c).clamp(0.0, 1.0)
}

fn plant_litter(biomass: &mut Matter, stress: f64, damage_rate: f64) -> Matter {
    biomass.withdraw(biomass.mg_c * (0.000_001 + damage_rate * (stress - 0.30).max(0.0)) * STEP_S)
}

struct LocalSoil<'a> {
    cell: &'a mut SoilCell,
    seasonal: Option<&'a mut SeasonalSoil>,
    patch: Option<&'a mut ForestLeafPatch>,
}

fn step_cell(local: LocalSoil<'_>, site: Site, light: f64, food_demand: f64, forcing: SoilForcing, balance: &mut Balance) -> Matter {
    let LocalSoil { cell, seasonal, patch } = local;
    let input = forcing.water_input_ml_per_hour * STEP_H;
    let filled = cell.water_ml + input;
    cell.water_ml = filled.min(cell.water_capacity_ml());
    let runoff = filled - cell.water_ml;
    let evaporation = (evaporation_rate(cell, site, light) * STEP_H * forcing.evaporation_factor).min(cell.water_ml);
    cell.water_ml -= evaporation;
    balance.inputs.water_ml += input;
    balance.outputs.water_ml += runoff + evaporation;

    let wetness = cell.saturation();
    // Feeding and decay both request the start-of-step litter, so shortages are shared.
    let patch_carbon = patch.as_ref().map_or(0.0, |patch| patch.remaining_mg_cnp[0]);
    let litter_carbon = cell.litter.mg_c + patch_carbon;
    let litter_decay_demand = litter_carbon * 0.000_045 * wetness * STEP_S * forcing.decomposition_factor;
    let total_demand = food_demand + litter_decay_demand;
    let taken = if let Some(patch) = patch.filter(|_| litter_carbon > 0.0 && total_demand > 0.0) {
        // One capped withdrawal supplies both processes. Apportion it by the two stocks' carbon.
        let fraction = (total_demand / litter_carbon).min(1.0);
        let mut residue = Matter::from_values(patch.remaining_mg_cnp);
        let mut taken = cell.litter.withdraw_fraction(fraction);
        taken.add(residue.withdraw_fraction(fraction));
        patch.remaining_mg_cnp = residue.values();
        taken
    } else {
        cell.litter.withdraw(total_demand)
    };
    let food_share = if total_demand > 0.0 { food_demand / total_demand } else { 0.0 };
    let decomposed_litter = taken.scaled(1.0 - food_share);

    let spent_waste = cell.waste.withdraw(cell.waste.mg_c * 0.000_24 * wetness * STEP_S * forcing.biological_activity);
    let turnover = cell.microbes.withdraw(cell.microbes.mg_c * 0.000_06 * STEP_S * forcing.biological_activity);
    cell.waste.add(turnover);
    let mineralization = Matter::new(
        decomposed_litter.mg_c + spent_waste.mg_c,
        decomposed_litter.mg_n + spent_waste.mg_n,
        decomposed_litter.mg_p + spent_waste.mg_p,
    );
    if let Some(seasonal) = seasonal {
        seasonal.advance(cell, site, light, forcing, balance);
    } else {
        cell.photosynthesis(site, light, balance);
    }
    cell.organic_turnover(mineralization, balance);
    taken.scaled(food_share)
}

fn step_animal(
    animal: &mut Animal, brain: &mut Woodlouse, cells: &[SoilCell], patch_food: &[f64; CELL_COUNT], geometry: &[Site; CELL_COUNT],
    light: f64, random: &mut RandomState,
) -> f64 {
    let current = cell_index(animal_point(animal));
    let hunger = brain.hunger();
    if brain.decision_remaining == 0 || !geometry[brain.next_cell].walkable
        || (brain.next_cell != current && !cardinal(current).contains(&Some(brain.next_cell)))
    {
        brain.next_cell = choose_step(current, hunger, cells, patch_food, geometry, light, random);
        brain.decision_remaining = DECISION_TICKS;
    }
    brain.decision_remaining -= 1;

    if current != brain.next_cell {
        let target = cell_center(brain.next_cell);
        let dx = target.x - animal.x;
        let dy = target.y - animal.y;
        let distance = dx.hypot(dy);
        let motion = (WALK_PER_S * STEP_S / distance).min(1.0);
        animal.x += dx * motion;
        animal.y += dy * motion;
        animal.heading = dy.atan2(dx).rem_euclid(TAU);
        animal.activity = if hunger > 0.33 { Activity::Foraging } else { Activity::Moving };
        return 0.0;
    }

    let site = geometry[current];
    if site.shelter >= 0.23 && (hunger < 0.22 || light >= 0.55) {
        animal.activity = Activity::Sheltering;
    } else {
        animal.activity = Activity::Resting;
    }
    if hunger >= 0.17 && cells[current].litter.mg_c + patch_food[current] > 0.0 {
        if animal.activity == Activity::Sheltering && brain.decision_remaining >= DECISION_TICKS / 4 {
            return 0.0;
        }
        // A short feeding bout can take place within a protected cell as well.
        return FEED_MG_C_PER_S * STEP_S;
    }
    0.0
}

fn choose_step(
    start: usize, hunger: f64, cells: &[SoilCell], patch_food: &[f64; CELL_COUNT], geometry: &[Site; CELL_COUNT],
    light: f64, random: &mut RandomState,
) -> usize {
    let mut distances = [u8::MAX; CELL_COUNT];
    let mut first_step = [start; CELL_COUNT];
    let mut queue = [start; CELL_COUNT];
    let mut head = 0;
    let mut tail = 1;
    distances[start] = 0;
    while head < tail {
        let here = queue[head];
        head += 1;
        if distances[here] == SEARCH_CELLS { continue; }
        for index in cardinal(here).into_iter().flatten() {
            if !geometry[index].walkable || distances[index] != u8::MAX { continue; }
            distances[index] = distances[here] + 1;
            first_step[index] = if here == start { index } else { first_step[here] };
            queue[tail] = index;
            tail += 1;
        }
    }

    let mut target = start;
    let mut best = f64::NEG_INFINITY;
    for index in 0..CELL_COUNT {
        let variation = random.unit() * 0.13;
        if distances[index] == u8::MAX { continue; }
        let cell = &cells[index];
        let site = geometry[index];
        let wetness = (1.0 - (cell.saturation() - 0.69).abs() / 0.69).max(0.0);
        let available_carbon = cell.litter.mg_c + patch_food[index];
        let food = available_carbon / (available_carbon + 24.0);
        let score = wetness * 1.65 + site.shelter * (0.56 + 0.60 * light) + site.shade * light * 0.30
            + food * (0.28 + hunger * 1.35) - f64::from(distances[index]) * 0.13 + variation;
        if score > best {
            best = score;
            target = index;
        }
    }
    first_step[target]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ForestBarkPreset, ForestRockEdit, WorldState};

    #[test]
    fn r5_take_out_woodlouse_exports_body_reserve_and_brood_without_death() {
        let mut world = WorldState::new_r5_composed_forest(19);
        let id = world.animals[1].id.clone();
        let forest = world.forest.as_mut().unwrap();
        let animal = &mut forest.woodlice[1];
        let brood = animal.reserve.withdraw(0.2);
        let life = &mut forest.seasonal.as_mut().unwrap().woodlice[1];
        life.brood = brood;
        life.brood_progress = 0.25;
        let mut expected = animal.body;
        expected.add(animal.reserve);
        expected.add(brood);
        let before = world.clone();
        let history = world.forest_layout_edit_state().unwrap();
        assert!(!serde_json::to_string(&world).unwrap().contains("removedWoodlice"));
        let remove = ForestRockEdit::RemoveWoodlouse { id: id.clone() };

        world.edit_forest_rocks(&remove).unwrap();
        let after = world.clone();
        let redo = world.forest_layout_edit_state().unwrap();
        let report = world.forest_ledger().unwrap();
        let observed = world.forest_observation().unwrap();
        assert_eq!(observed.life.unwrap().woodlice_died, 0);
        assert_eq!(observed.litter_mg_c, before.forest_observation().unwrap().litter_mg_c);
        assert_eq!(world.animals.len(), before.animals.len() - 1);
        assert!(world.animals.iter().all(|animal| animal.id != id));
        assert_eq!(world.edit_forest_rocks(&remove), Err(CoreError::WoodlouseMissing));
        assert_eq!(world, after);
        assert!(report.matter_output_mg_cnp.into_iter().zip(expected.values()).all(|(a, b)| (a - b).abs() < 1e-8));
        assert!(report.matter_residual_mg_cnp.into_iter().all(|amount| amount.abs() < 1e-8));
        world.validate().unwrap();

        world.restore_forest_layout_edit_state(history).unwrap();
        assert_eq!(world, before);
        world.restore_forest_layout_edit_state(redo).unwrap();
        assert_eq!(world, after);
        let saved: WorldState = serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
        saved.validate().unwrap();
        assert_eq!(saved, after);
    }

    fn relocated() -> Point {
        Point { x: 0.76, y: 0.70 }
    }

    #[test]
    fn r5_living_fern_moves_only_its_share_and_keeps_stock_in_the_existing_cell() {
        let read = |world: &WorldState| world.forest_fern_patches().unwrap()[0];
        let assert_matter = |actual: [f64; 3], expected: [f64; 3]| {
            assert!(actual.into_iter().zip(expected).all(|(a, b)| (a - b).abs() < 1e-8), "{actual:?} vs {expected:?}");
        };
        let source = 4;
        let destination = 31;
        assert!(fern_patch(source) && fern_patch(destination));
        let from = cell_center(source);
        let to = cell_center(destination);
        let mut world = WorldState::new_r5_composed_forest(1206);
        let baseline = world.clone();
        let encoded = serde_json::to_string(&world).unwrap();
        assert!(!encoded.contains("fernPatches"));
        let existing: WorldState = serde_json::from_str(&encoded).unwrap();
        assert_eq!(existing, world);
        assert_eq!(existing.forest_fern_patches(), Some(Vec::new()));
        assert!(WorldState::new_r5_leaf_forest(1206).forest_fern_preset().is_none());

        let cave = world.forest_rock_layout().unwrap().bark_shelters().unwrap()[0];
        for cell in cave.passage_cells() {
            let result = world.edit_forest_rocks(&ForestRockEdit::AddFernPatch { position: cell_center(cell) });
            assert_eq!(result, Err(CoreError::BlockedForestFernSite));
            assert_eq!(world, baseline);
        }
        let pose = ForestRockEdit::AddFernPatch { position: from };
        world.edit_forest_rocks(&pose).unwrap();
        let placed = read(&world);
        let stock = FERN_STARTER_DOSE.values();
        assert_eq!(placed.patch.cell_index, source);
        assert_matter(placed.patch.stock_mg_cnp, stock);
        assert_eq!(placed.canopy_shade_fraction, FERN_MAX_ADDED_SHADE);
        assert_matter(world.forest_ledger().unwrap().matter_input_mg_cnp, stock);
        assert_eq!(world.edit_forest_rocks(&pose), Err(CoreError::BlockedForestFernSite));
        assert_eq!(world.edit_forest_rocks(&ForestRockEdit::Add { position: from }), Err(CoreError::BlockedForestFernSite));
        let plants = &mut world.forest.as_mut().unwrap().seasonal.as_mut().unwrap().soil;
        plants[source].fern_stress = 0.7;
        plants[destination].fern_stress = 0.18;
        assert_eq!(read(&world).patch.stress_fraction, 0.7);

        let before_move = world.forest_ledger().unwrap();
        world.edit_forest_rocks(&ForestRockEdit::MoveFernPatch { id: placed.patch.id, position: to }).unwrap();
        let moved = read(&world);
        assert_eq!((moved.patch.id, moved.patch.cell_index, moved.patch.stress_fraction), (placed.patch.id, destination, 0.18));
        assert_matter(moved.patch.stock_mg_cnp, stock);
        let after_move = world.forest_ledger().unwrap();
        assert_matter(after_move.current_matter_mg_cnp, before_move.current_matter_mg_cnp);
        assert_eq!(after_move.matter_input_mg_cnp, before_move.matter_input_mg_cnp);
        assert_eq!(after_move.matter_output_mg_cnp, before_move.matter_output_mg_cnp);
        assert_eq!(after_move.current_water_ml, before_move.current_water_ml);
        let forest = world.forest.as_ref().unwrap();
        assert_matter(forest.plant_stock(ForestPlantKind::Fern, source).values(), stock);
        assert_matter(forest.plant_stock(ForestPlantKind::Fern, destination).values(), FERN_STARTER_DOSE.scaled(2.0).values());

        let before_trim = forest.soil[destination].litter.values();
        let bad_trim = ForestRockEdit::TrimFernPatch { id: placed.patch.id, mg_c: 15.0 };
        assert_eq!(world.edit_forest_rocks(&bad_trim), Err(CoreError::InvalidForestFernTrim));
        world.edit_forest_rocks(&ForestRockEdit::TrimFernPatch { id: placed.patch.id, mg_c: 5.0 }).unwrap();
        let cut = FERN_STARTER_DOSE.scaled(1.0 / 3.0).values();
        let remaining = FERN_STARTER_DOSE.scaled(2.0 / 3.0).values();
        assert_matter(read(&world).patch.stock_mg_cnp, remaining);
        assert_matter(world.forest.as_ref().unwrap().soil[destination].litter.values(), std::array::from_fn(|i| before_trim[i] + cut[i]));
        assert!(read(&world).canopy_shade_fraction < placed.canopy_shade_fraction);
        let initial = world.fresh_habitat(1207).unwrap();
        assert_eq!(read(&initial).patch.position, to);
        assert_matter(read(&initial).patch.stock_mg_cnp, stock);
        let before_removal = world.forest_layout_edit_state().unwrap();
        world.edit_forest_rocks(&ForestRockEdit::RemoveFernPatch { id: moved.patch.id }).unwrap();
        let removed = world.forest_ledger().unwrap();
        assert_matter(removed.matter_output_mg_cnp, remaining);
        assert_matter(removed.matter_residual_mg_cnp, [0.0; 3]);
        assert!(world.forest_fern_patches().unwrap().is_empty());
        world.restore_forest_layout_edit_state(before_removal).unwrap();
        assert_matter(read(&world).patch.stock_mg_cnp, remaining);
        world.validate().unwrap();

        let mut blank = WorldState::new_blank_r5_composed_forest(1206);
        blank.edit_forest_rocks(&pose).unwrap();
        blank.add_starter_community().unwrap();
        assert_matter(read(&blank).patch.stock_mg_cnp, stock);
        assert_matter(blank.forest.as_ref().unwrap().plant_stock(ForestPlantKind::Fern, source).values(),
            FERN_STARTER_DOSE.scaled(2.0).values());
    }

    #[test]
    fn r5_living_fern_canopy_composes_by_maximum_and_reaches_the_actual_water_cycle() {
        let mut planted = WorldState::new_blank_r5_composed_forest(1206);
        let mut unplanted = planted.clone();
        let center = 18;
        let adjacent = 19;
        let edge = 30;
        for cell in [center, adjacent] {
            planted.edit_forest_rocks(&ForestRockEdit::AddFernPatch { position: cell_center(cell) }).unwrap();
        }
        let terrain = ForestTerrain { legacy_rock: planted.rock, layout: planted.forest_rock_layout() };
        let forest = planted.forest.as_ref().unwrap();
        let baseline = terrain.geometry();
        let shaded = forest.interactive_geometry(terrain);
        assert_eq!(shaded[center].shade, OPEN_GROUND_SHADE + FERN_MAX_ADDED_SHADE);
        assert_eq!(shaded[adjacent].shade, shaded[center].shade);
        assert_eq!(shaded[edge].shade, OPEN_GROUND_SHADE + FERN_MAX_ADDED_SHADE * 0.5);
        assert!(shaded.iter().zip(baseline).all(|(site, native)| site.walkable == native.walkable && site.shelter == native.shelter));
        assert!(planted.forest_observation().unwrap().evaporation_ml_per_hour[1] < unplanted.forest_observation().unwrap().evaporation_ml_per_hour[1]);

        planted.advance_steps(48).unwrap();
        unplanted.advance_steps(48).unwrap();
        assert!(planted.forest.as_ref().unwrap().soil[center].water_ml > unplanted.forest.as_ref().unwrap().soil[center].water_ml);
        let live = planted.forest_fern_patches().unwrap()[0];
        assert_eq!(live.patch.stock_mg_cnp, planted.forest.as_ref().unwrap().plant_stock(ForestPlantKind::Fern, center).values());
        planted.validate().unwrap();
    }

    #[test]
    fn r5_decomposer_observation_tracks_local_stock_without_a_new_inventory() {
        let mut world = WorldState::new_blank_r5_composed_forest(1206);
        let position = cell_center(26);
        let empty = world.forest_substrate_cells().unwrap();
        assert!(empty.iter().all(|sample| sample.decomposer_mg_c == 0.0));

        world.edit_forest_rocks(&ForestRockEdit::SpreadLeafFragments { positions: vec![position], mg_c: 90.0 }).unwrap();
        assert_eq!(world.forest_substrate_cells().unwrap()[26].decomposer_mg_c, 0.0);
        world.advance_steps(240).unwrap();
        let active = world.forest_substrate_cells().unwrap();
        assert_eq!(active[26].position, position);
        assert!(active[26].decomposer_mg_c > 0.0);
        assert!(active.iter().filter(|sample| sample.cell_index != 26).all(|sample| sample.decomposer_mg_c == 0.0));

        let wire = serde_json::to_value(&world).unwrap();
        assert_eq!(wire["forest"]["soil"][26]["microbes"]["mgC"], active[26].decomposer_mg_c);
        let mut restored: WorldState = serde_json::from_value(wire).unwrap();
        assert_eq!(restored.forest_substrate_cells().unwrap(), active);
        let leaf_id = restored.forest_rocks.as_ref().unwrap().leaf_patches().unwrap()[0].id;
        restored.edit_forest_rocks(&ForestRockEdit::Remove { id: leaf_id }).unwrap();
        restored.advance_steps(240).unwrap();
        let after = restored.forest_substrate_cells().unwrap()[26].decomposer_mg_c;
        assert!(after > 0.0 && after < active[26].decomposer_mg_c);
        assert!(restored.forest_ledger().unwrap().matter_residual_mg_cnp.iter().all(|x| x.abs() < 1e-8));
        restored.validate().unwrap();
    }

    #[test]
    fn r5_substrate_changes_capacity_and_future_water_flow_without_changing_held_stock() {
        let mut world = WorldState::new_r5_composed_forest(1206);
        let edit = |cells, substrate| ForestRockEdit::SetSubstrate { cells, substrate };
        let baseline_wire = serde_json::to_string(&world).unwrap();
        assert!(!baseline_wire.contains("substrate"));
        let mut untouched: WorldState = serde_json::from_str(&baseline_wire).unwrap();
        assert_eq!(world, untouched);
        let before_ledger = world.forest_ledger().unwrap();
        let before = world.forest_observation().unwrap();
        let original_cells = world.forest_substrate_cells().unwrap();
        assert_eq!(original_cells.len(), CELL_COUNT);
        assert!(original_cells.iter().all(|cell| cell.profile.substrate == ForestSubstrate::Native));
        let left = [3, 15, 27];
        let right = [8, 20, 32];
        world.edit_forest_rocks(&edit(left.to_vec(), ForestSubstrate::FineSoil)).unwrap();
        world.edit_forest_rocks(&edit(right.to_vec(), ForestSubstrate::CoarseHumus)).unwrap();
        let changed = world.forest_observation().unwrap();
        assert_eq!(world.forest_ledger().unwrap(), before_ledger);
        assert_eq!(changed.water_ml, before.water_ml);
        assert_eq!(changed.water_capacity_ml, [before.water_capacity_ml[0] + 18.0, before.water_capacity_ml[1] - 18.0]);
        assert!(world.moisture()[0] < untouched.moisture()[0] && world.moisture()[1] > untouched.moisture()[1]);
        assert!(changed.evaporation_ml_per_hour[0] < before.evaporation_ml_per_hour[0]);
        assert!(changed.evaporation_ml_per_hour[1] > before.evaporation_ml_per_hour[1]);

        world.advance_steps(48).unwrap();
        untouched.advance_steps(48).unwrap();
        let evolved = world.forest_substrate_cells().unwrap();
        let unchanged = untouched.forest_substrate_cells().unwrap();
        assert!(left.iter().all(|&i| evolved[i].water_ml > unchanged[i].water_ml));
        assert!(right.iter().all(|&i| evolved[i].water_ml < unchanged[i].water_ml));
        assert!(world.forest_ledger().unwrap().water_residual_ml.abs() < 1e-8);
        world.validate().unwrap();
        let restored: WorldState = serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
        assert_eq!(world, restored);
        let blueprint = restored.fresh_habitat(1366).unwrap();
        for (index, cell) in blueprint.forest_substrate_cells().unwrap().iter().enumerate() {
            assert_eq!(cell.profile, evolved[index].profile);
            assert_eq!(cell.water_ml, original_cells[index].water_ml);
        }

        let mut old = WorldState::new_r5_leaf_forest(1206);
        assert!(old.forest_substrate_cells().is_none() && old.forest_substrate_presets().is_none());
        assert_eq!(old.edit_forest_rocks(&edit(vec![3], ForestSubstrate::FineSoil)), Err(CoreError::WrongWorldKind));
        let mut incompatible = serde_json::to_value(old).unwrap();
        incompatible["forest"]["soil"][0]["substrate"] = serde_json::json!("fineSoil");
        let incompatible: WorldState = serde_json::from_value(incompatible).unwrap();
        assert_eq!(incompatible.validate(), Err(CoreError::InvalidForestState));
    }

    #[test]
    fn r5_substrate_rejects_an_entire_invalid_stroke_and_applies_the_rainfall_capacity() {
        let mut world = WorldState::new_r5_composed_forest(1206);
        let forest = world.forest.as_mut().unwrap();
        forest.soil[9].water_ml = 28.0;
        forest.reset_initial_balance(world.forest_rocks.as_ref().unwrap().leaf_patches().unwrap());
        let captured = world.clone();
        for (cells, substrate, error) in [
            (vec![2, 9], ForestSubstrate::CoarseHumus, CoreError::ForestSubstrateTooWet),
            (vec![2, 2], ForestSubstrate::FineSoil, CoreError::InvalidForestSubstrateCells),
            (vec![2, CELL_COUNT], ForestSubstrate::FineSoil, CoreError::InvalidForestSubstrateCells),
            (vec![], ForestSubstrate::FineSoil, CoreError::InvalidForestSubstrateCells),
            (vec![2, 9], ForestSubstrate::Native, CoreError::ForestSubstrateUnchanged),
        ] {
            assert_eq!(world.edit_forest_rocks(&ForestRockEdit::SetSubstrate { cells, substrate }), Err(error));
            assert_eq!(world, captured);
        }

        for substrate in [ForestSubstrate::Native, ForestSubstrate::FineSoil, ForestSubstrate::CoarseHumus] {
            let capacity = substrate.profile().water_capacity_ml;
            let mut cell = SoilCell::new(0);
            cell.substrate = substrate;
            cell.water_ml = capacity - 0.25;
            let local = LocalSoil { cell: &mut cell, seasonal: None, patch: None };
            let forcing = SoilForcing { water_input_ml_per_hour: 0.5 / STEP_H, evaporation_factor: 0.0, ..SoilForcing::LEGACY };
            let mut balance = Balance::default();
            step_cell(local, Site { shade: 0.12, shelter: 0.0, walkable: true }, 0.0, 0.0, forcing, &mut balance);
            assert_eq!(cell.water_ml, capacity);
            assert_eq!(balance.inputs.water_ml, 0.5);
            assert_eq!(balance.outputs.water_ml, 0.25);
            assert!(cell.valid());
            cell.water_ml += 0.5;
            assert!(!cell.valid());
        }
    }

    #[test]
    fn r5_ground_moss_links_existing_cells_and_accounts_for_each_edit() {
        let mut world = WorldState::new_r5_composed_forest(1206);
        let from = Point { x: 0.45, y: 0.85 };
        let to = Point { x: 0.72, y: 0.64 };
        let source = cell_index(from);
        let target = cell_index(to);
        let stock = |world: &WorldState| world.forest_moss_patches().unwrap()[0];
        let same_matter = |actual: [f64; 3], expected: [f64; 3]| {
            assert!(actual.into_iter().zip(expected).all(|(left, right)| (left - right).abs() < 1e-9),
                "{actual:?} != {expected:?}");
        };
        let initial = world.forest_ledger().unwrap();
        let empty = serde_json::to_string(&world).unwrap();
        assert!(!empty.contains("mossPatches"));
        let existing: WorldState = serde_json::from_str(&empty).unwrap();
        assert_eq!(existing.forest_moss_patches(), Some(Vec::new()));
        assert!(existing.forest_moss_preset().is_some());
        let mut old = WorldState::new_r5_leaf_forest(1206);
        assert!(old.forest_moss_patches().is_none() && old.forest_moss_preset().is_none());
        assert_eq!(old.edit_forest_rocks(&ForestRockEdit::AddMossPatch { position: from }), Err(CoreError::WrongWorldKind));

        world.edit_forest_rocks(&ForestRockEdit::AddMossPatch { position: from }).unwrap();
        let patch = stock(&world);
        let dose = MOSS_STARTER_DOSE.values();
        same_matter(patch.stock_mg_cnp, dose);
        assert_eq!(patch.cell_index, source);
        same_matter(world.forest_ledger().unwrap().matter_input_mg_cnp, dose);
        same_matter(world.forest.as_ref().unwrap().soil[source].moss.values(), MOSS_STARTER_DOSE.scaled(2.0).values());
        let placed = world.clone();
        for (edit, error) in [
            (ForestRockEdit::AddMossPatch { position: Point { x: 0.47, y: 0.85 } }, CoreError::BlockedForestMossSite),
            (ForestRockEdit::AddMossPatch { position: world.rock }, CoreError::BlockedForestMossSite),
            (ForestRockEdit::MoveMossPatch { id: patch.id, position: Point { x: 0.5, y: 0.40 } }, CoreError::InvalidForestMossSite),
            (ForestRockEdit::Add { position: from }, CoreError::BlockedForestMossSite),
            (ForestRockEdit::TrimMossPatch { id: patch.id, mg_c: dose[0] }, CoreError::InvalidForestMossTrim),
            (ForestRockEdit::TrimMossPatch { id: patch.id, mg_c: -0.1 }, CoreError::InvalidForestMossTrim),
            (ForestRockEdit::RemoveMossPatch { id: 999 }, CoreError::ForestMossPatchMissing),
        ] {
            assert_eq!(world.edit_forest_rocks(&edit), Err(error), "{edit:?}");
            assert_eq!(world, placed);
        }

        let soil = &mut world.forest.as_mut().unwrap().seasonal.as_mut().unwrap().soil;
        soil[source].moss_stress = 0.79;
        soil[target].moss_stress = 0.23;
        assert_eq!(stock(&world).stress_fraction, 0.79);
        world.edit_forest_rocks(&ForestRockEdit::MoveMossPatch { id: patch.id, position: to }).unwrap();
        let moved = stock(&world);
        assert_eq!((moved.id, moved.cell_index, moved.stress_fraction), (patch.id, target, 0.23));
        same_matter(moved.stock_mg_cnp, dose);
        same_matter(world.forest.as_ref().unwrap().soil[source].moss.values(), dose);

        let trimmed = MOSS_STARTER_DOSE.scaled(0.25).values();
        let litter_before = world.forest.as_ref().unwrap().soil[target].litter.values();
        let before_trim = world.forest_layout_edit_state().unwrap();
        world.edit_forest_rocks(&ForestRockEdit::TrimMossPatch { id: patch.id, mg_c: 2.0 }).unwrap();
        let remaining = MOSS_STARTER_DOSE.scaled(0.75).values();
        same_matter(stock(&world).stock_mg_cnp, remaining);
        same_matter(world.forest.as_ref().unwrap().soil[target].litter.values(), std::array::from_fn(|i| litter_before[i] + trimmed[i]));
        let trimmed_state = world.forest_layout_edit_state().unwrap();
        world.restore_forest_layout_edit_state(before_trim).unwrap();
        assert_eq!(stock(&world), moved);
        same_matter(world.forest.as_ref().unwrap().soil[target].litter.values(), litter_before);
        world.restore_forest_layout_edit_state(trimmed_state).unwrap();

        let reopened: WorldState = serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
        assert_eq!(stock(&reopened), stock(&world));
        let fresh = reopened.fresh_habitat(2310).unwrap();
        assert_eq!((stock(&fresh).id, stock(&fresh).position), (patch.id, to));
        same_matter(stock(&fresh).stock_mg_cnp, dose);
        same_matter(fresh.forest_ledger().unwrap().matter_input_mg_cnp, [0.0; 3]);
        world.edit_forest_rocks(&ForestRockEdit::RemoveMossPatch { id: patch.id }).unwrap();
        assert!(world.forest_moss_patches().unwrap().is_empty());
        let removed = world.forest_ledger().unwrap();
        same_matter(removed.matter_output_mg_cnp, remaining);
        same_matter(removed.current_matter_mg_cnp, std::array::from_fn(|i| initial.current_matter_mg_cnp[i] + trimmed[i]));
        same_matter(removed.matter_residual_mg_cnp, [0.0; 3]);
        world.validate().unwrap();

        let mut blank = WorldState::new_blank_r5_composed_forest(1206);
        blank.edit_forest_rocks(&ForestRockEdit::AddMossPatch { position: from }).unwrap();
        blank.add_starter_community().unwrap();
        same_matter(stock(&blank).stock_mg_cnp, dose);
        same_matter(blank.forest.as_ref().unwrap().soil[source].moss.values(), MOSS_STARTER_DOSE.scaled(2.0).values());
        same_matter(stock(&blank.fresh_habitat(2310).unwrap()).stock_mg_cnp, dose);
    }

    #[test]
    fn r5_ground_moss_metadata_does_not_run_a_second_growth_process() {
        let mut planted = WorldState::new_r5_composed_forest(1206);
        let position = Point { x: 0.45, y: 0.85 };
        let cell = cell_index(position);
        let mut unlabelled = planted.clone();
        planted.edit_forest_rocks(&ForestRockEdit::AddMossPatch { position }).unwrap();
        unlabelled.forest.as_mut().unwrap().add_ground_plant(ForestPlantKind::Moss, cell, true);
        planted.advance_steps(48).unwrap();
        unlabelled.advance_steps(48).unwrap();

        assert_eq!(planted.forest, unlabelled.forest);
        assert_eq!(planted.forest_ledger(), unlabelled.forest_ledger());
        let total = planted.forest.as_ref().unwrap().soil[cell].moss;
        assert_ne!(total, MOSS_STARTER_DOSE.scaled(2.0));
        assert_eq!(planted.forest_moss_patches().unwrap()[0].stock_mg_cnp, total.scaled(0.5).values());
    }

    #[test]
    fn animal_walks_from_back_entrance_through_roof_and_out_the_front() {
        let mut world = WorldState::new_r5_bark_forest(1206);
        let position = Point { x: 0.38, y: 0.80 };
        world.edit_forest_rocks(&ForestRockEdit::AddBarkShelter { position, preset: ForestBarkPreset::Arch }).unwrap();
        let bark = world.forest_rock_layout().unwrap().bark_shelters().unwrap()[0];
        let [back, center, front] = bark.passage_cells();
        let start = cell_center(back);
        world.animals[0].x = start.x;
        world.animals[0].y = start.y;

        // Put a wet food source at the front so normal foraging must find a route
        // through the roof. It is beyond the search radius when going around a foot.
        let forest = world.forest.as_mut().unwrap();
        for cell in &mut forest.soil {
            cell.water_ml = 0.0;
            cell.litter = Matter::default();
        }
        forest.soil[front].water_ml = CELL_WATER_CAPACITY_ML * 0.69;
        forest.soil[front].litter = Matter::new(160.0, 16.0, 1.6);
        forest.reset_initial_balance(&[]);
        world.validate().unwrap();

        let terrain = ForestTerrain { legacy_rock: world.rock, layout: world.forest_rock_layout() };
        let geometry = terrain.geometry();
        assert!(bark.passage_cells().iter().all(|cell| geometry[*cell].walkable));
        assert!(bark.foot_cells().iter().all(|cell| !geometry[*cell].walkable));
        assert!(geometry[center].shade > geometry[front].shade && geometry[center].shelter > geometry[back].shelter);

        let mut route = vec![back];
        for _ in 0..280 {
            world.advance_steps(1).unwrap();
            let point = animal_point(&world.animals()[0]);
            let cell = cell_index(point);
            assert!(bark.in_passage(point));
            assert_eq!(bark.contains(point), cell == center);
            if route.last() != Some(&cell) {
                route.push(cell);
            }
            if cell == front {
                break;
            }
        }
        assert_eq!(route, [back, center, front]);
        world.validate().unwrap();
    }

    #[test]
    fn forest_bridge_feet_block_but_both_raised_spans_allow_animal_passage() {
        let position = Point { x: 0.51, y: 0.80 };
        for preset in [ForestBarkPreset::RootArch, ForestBarkPreset::BranchBridge] {
            let add = ForestRockEdit::AddBarkShelter { position, preset };
            let mut legacy = WorldState::new_r5_bark_forest(1206);
            assert_eq!(legacy.edit_forest_rocks(&add), Err(CoreError::WrongWorldKind));

            let mut world = WorldState::new_blank_r5_composed_forest(1206);
            world.edit_forest_rocks(&add).unwrap();
            let bridge = world.forest_rock_layout().unwrap().bark_shelters().unwrap()[0];
            let [back, center, front] = bridge.passage_cells();
            let terrain = ForestTerrain { legacy_rock: world.rock, layout: world.forest_rock_layout() };
            let geometry = terrain.geometry();
            assert!(bridge.foot_cells().into_iter().all(|cell| !geometry[cell].walkable));
            assert!([back, center, front].into_iter().all(|cell| geometry[cell].walkable));
            assert_eq!(world.edit_forest_rocks(&add), Err(CoreError::OverlappingBarkSite));
            world.edit_forest_rocks(&ForestRockEdit::StockWoodlice { position: cell_center(back), count: 1 }).unwrap();

            let forest = world.forest.as_mut().unwrap();
            for cell in &mut forest.soil {
                cell.water_ml = 0.0;
                cell.litter = Matter::default();
            }
            forest.soil[front].water_ml = CELL_WATER_CAPACITY_ML * 0.69;
            forest.soil[front].litter = Matter::new(160.0, 16.0, 1.6);
            forest.reset_initial_balance(&[]);
            world.validate().unwrap();

            let mut route = vec![back];
            for _ in 0..280 {
                world.advance_steps(1).unwrap();
                let cell = cell_index(animal_point(&world.animals()[0]));
                assert!(!bridge.foot_cells().contains(&cell));
                if route.last() != Some(&cell) {
                    route.push(cell);
                }
                if cell == front { break; }
            }
            assert_eq!(route, [back, center, front]);
            world.validate().unwrap();
        }
    }

    #[test]
    fn committed_rock_move_changes_geometry_and_rate_before_inventory() {
        let mut forest = WorldState::new_forest(42);
        let before = forest.forest_observation().unwrap();
        let resources = forest.forest_ledger();
        forest.edit_rock(relocated()).unwrap();
        let after = forest.forest_observation().unwrap();
        assert_ne!(after.rock_cell, before.rock_cell);
        assert_eq!(before.walkable_cells, [23, 24]);
        assert_eq!(after.walkable_cells, [24, 23]);
        assert!(after.shade_fraction[0] < before.shade_fraction[0]);
        assert!(after.shade_fraction[1] > before.shade_fraction[1]);
        assert!(after.shelter_fraction[0] < before.shelter_fraction[0]);
        assert!(after.shelter_fraction[1] > before.shelter_fraction[1]);
        assert!(after.evaporation_ml_per_hour[0] > before.evaporation_ml_per_hour[0]);
        assert!(after.evaporation_ml_per_hour[1] < before.evaporation_ml_per_hour[1]);
        assert_eq!(after.water_ml, before.water_ml);
        assert_eq!(forest.forest_ledger(), resources);
        assert_eq!(forest.tick(), 0);
        println!("geometry control {before:?}\ngeometry moved {after:?}");
    }

    #[test]
    fn paired_seeds_show_water_and_woodlouse_feedback_after_the_same_elapsed_time() {
        let mut left_control_samples = 0_u32;
        let mut right_control_samples = 0_u32;
        let mut left_moved_samples = 0_u32;
        let mut right_moved_samples = 0_u32;
        for seed in [7, 21, 42, 143, 509] {
            let mut control = WorldState::new_forest(seed);
            let mut moved = control.clone();
            moved.edit_rock(relocated()).unwrap();
            for second in 1..=120 {
                control.advance_steps(10).unwrap();
                moved.advance_steps(10).unwrap();
                if second >= 30 {
                    let base = control.forest_observation().unwrap();
                    let rock = moved.forest_observation().unwrap();
                    left_control_samples += base.sheltered_woodlice_by_side[0];
                    right_control_samples += base.sheltered_woodlice_by_side[1];
                    left_moved_samples += rock.sheltered_woodlice_by_side[0];
                    right_moved_samples += rock.sheltered_woodlice_by_side[1];
                }
            }
            let base = control.forest_observation().unwrap();
            let rock = moved.forest_observation().unwrap();
            assert!(rock.water_ml[0] < base.water_ml[0]);
            assert!(rock.water_ml[1] > base.water_ml[1]);
            control.validate().unwrap();
            moved.validate().unwrap();
            println!("seed {seed} at 120 s, control {:?} mL, moved {:?} mL", base.water_ml, rock.water_ml);
        }
        println!(
            "sheltered samples across paired seeds, control [{left_control_samples},{right_control_samples}], moved [{left_moved_samples},{right_moved_samples}]"
        );
        assert!(left_control_samples > left_moved_samples);
        assert!(right_moved_samples > right_control_samples);
    }

    #[test]
    fn soil_water_changes_a_route_with_fixed_geometry_and_food() {
        let mut sites = [Site { shade: 0.30, shelter: 0.4, walkable: true }; CELL_COUNT];
        sites[0].walkable = false;
        let start = COLUMNS + 5;
        let mut dry = vec![SoilCell::new(0); CELL_COUNT];
        for cell in &mut dry {
            cell.water_ml = 3.0;
        }
        let mut moist_left = dry.clone();
        moist_left[start - 1].water_ml = 20.7;
        let mut moist_right = dry;
        moist_right[start + 1].water_ml = 20.7;
        for seed in [9, 17, 38] {
            let mut rng = RandomState::new(seed);
            assert_eq!(choose_step(start, 0.35, &moist_left, &[0.0; CELL_COUNT], &sites, 0.0, &mut rng), start - 1);
            let mut rng = RandomState::new(seed);
            assert_eq!(choose_step(start, 0.35, &moist_right, &[0.0; CELL_COUNT], &sites, 0.0, &mut rng), start + 1);
        }
    }

    #[test]
    fn exact_replay_and_conservation_cover_water_carbon_nitrogen_and_phosphorus() {
        let mut single = WorldState::new_forest(42);
        let mut batched = single.clone();
        for world in [&mut single, &mut batched] {
            world.edit_rock(relocated()).unwrap();
        }
        single.advance_steps(18_000).unwrap();
        for _ in 0..45 { batched.advance_steps(400).unwrap(); }
        assert_eq!(single, batched);
        single.validate().unwrap();
        let ledger = single.forest_ledger().unwrap();
        println!("30 minute forest ledger {ledger:?}");
        assert!(ledger.water_input_ml > 0.0 && ledger.water_output_ml > 0.0);
        assert!(ledger.fed_mg_c > 0.0 && ledger.decomposed_mg_c > 0.0 && ledger.photosynthesized_mg_c > 0.0);
        assert!(ledger.water_residual_ml.abs() < 1e-7);
        assert!(ledger.matter_residual_mg_cnp.iter().all(|value| value.abs() < 1e-7));
    }

    #[test]
    fn scarce_litter_is_allocated_once_and_an_empty_cell_stays_finite() {
        let mut cell = SoilCell::new(0);
        cell.litter = Matter::new(0.001, 0.0004, 0.0001);
        let site = Site { shade: 0.2, shelter: 0.6, walkable: true };
        let mut balance = Balance::default();
        let soil = LocalSoil { cell: &mut cell, seasonal: None, patch: None };
        let food = step_cell(soil, site, 0.0, 0.0025, SoilForcing::LEGACY, &mut balance);
        assert!(food.mg_c > 0.0 && food.mg_c <= 0.001);
        assert!((food.mg_c + balance.decomposed_mg_c - 0.001).abs() < 1e-12);
        assert_eq!(cell.litter, Matter::default());
        let soil = LocalSoil { cell: &mut cell, seasonal: None, patch: None };
        assert_eq!(step_cell(soil, site, 0.0, 0.02, SoilForcing::LEGACY, &mut balance), Matter::default());
        assert!(cell.valid());
    }

    #[test]
    fn soil_and_patch_share_one_withdrawal_when_food_and_decay_compete() {
        use crate::ForestLeafPreset;

        let mut cell = SoilCell::new(0);
        cell.litter = Matter::new(0.001, 0.0004, 0.0001);
        cell.microbes = Matter::default();
        let mut patch = ForestLeafPatch {
            id: 2,
            position: Point { x: 0.3, y: 0.61 },
            mirrored: false,
            preset: ForestLeafPreset::Leaves,
            remaining_mg_cnp: [0.002, 0.0002, 0.00002],
            initial_dose_mg_cnp: None,
        };
        let site = Site { shade: 0.2, shelter: 0.6, walkable: true };
        let mut balance = Balance::default();
        let combined = [0.003, 0.0006, 0.00012];

        let soil = LocalSoil { cell: &mut cell, seasonal: None, patch: Some(&mut patch) };
        let food = step_cell(soil, site, 0.0, 0.0025, SoilForcing::LEGACY, &mut balance);
        assert!((food.mg_c - 0.0025).abs() < 1e-12);
        let left = cell.litter.values();
        let remaining = patch.remaining_mg_cnp;
        let residual_fraction = 1.0 - (food.mg_c + balance.decomposed_mg_c) / combined[0];
        for i in 0..3 {
            assert!((left[i] + remaining[i] - combined[i] * residual_fraction).abs() < 1e-12);
        }

        let soil = LocalSoil { cell: &mut cell, seasonal: None, patch: Some(&mut patch) };
        let second = step_cell(soil, site, 0.0, 0.0025, SoilForcing::LEGACY, &mut balance);
        assert!((food.mg_c + second.mg_c + balance.decomposed_mg_c - combined[0]).abs() < 1e-12);
        assert_eq!(cell.litter, Matter::default());
        assert_eq!(patch.remaining_mg_cnp, [0.0; 3]);
    }

    #[test]
    fn invalid_or_occupied_rock_destination_leaves_the_world_unchanged() {
        let mut world = WorldState::new_forest(42);
        let pristine = world.clone();
        let occupied = animal_point(&world.animals()[0]);
        assert_eq!(world.edit_rock(occupied), Err(CoreError::OccupiedRockSite));
        assert_eq!(world.edit_rock(Point { x: 0.60, y: 0.25 }), Err(CoreError::InvalidRockSite));
        assert_eq!(world, pristine);
    }

    #[test]
    fn seasonal_forest_replays_and_preserves_every_ledger_element() {
        let mut single = WorldState::new_seasonal_forest(42);
        let mut batched = single.clone();
        let initial = single.forest_observation().unwrap();
        let initial_life = initial.life.unwrap();
        assert_eq!(initial_life.woodlice.iter().filter(|life| life.stage == WoodlouseStage::Juvenile).count(), 1);
        assert!(initial_life.springtail_mg_c.iter().all(|&carbon| carbon > 0.0));
        assert!(initial_life.fern_mg_c.iter().all(|&carbon| carbon > 0.0));
        single.advance_steps(18_000).unwrap();
        for _ in 0..45 {
            batched.advance_steps(400).unwrap();
        }
        assert_eq!(batched, single);
        single.validate().unwrap();
        let observed = single.forest_observation().unwrap().life.unwrap();
        assert_eq!(observed.woodlice[0].age_seconds - initial_life.woodlice[0].age_seconds, 1800.0);
        let ledger = single.forest_ledger().unwrap();
        assert!(ledger.water_input_ml > 0.0 && ledger.water_output_ml > 0.0);
        assert!(ledger.fed_mg_c > 0.0 && ledger.decomposed_mg_c > 0.0 && ledger.photosynthesized_mg_c > 0.0);
        assert!(ledger.water_residual_ml.abs() < 1e-7);
        assert!(ledger.matter_residual_mg_cnp.iter().all(|value| value.abs() < 1e-7));
        assert!(WorldState::new_forest(42).forest_observation().unwrap().life.is_none());
    }

    #[test]
    fn recruitment_uses_reserved_brood_and_death_returns_stocks_to_the_soil() {
        let mut world = WorldState::new_seasonal_forest(42);
        let rock = world.rock;
        let forest = world.forest.as_mut().unwrap();
        let old_cell = cell_index(animal_point(&world.animals[1]));
        let seasonal = forest.seasonal.as_mut().unwrap();
        seasonal.woodlice[0].brood = forest.woodlice[0].reserve.withdraw(BROOD_MG_C);
        seasonal.woodlice[0].brood_progress = 1.0 - 1e-10;
        seasonal.woodlice[1].age_ticks = MAX_AGE_TICKS - 1;
        seasonal.woodlice[3].age_ticks = JUVENILE_MINIMUM_TICKS - 1;
        let juvenile = &mut forest.woodlice[3];
        juvenile.body.add(juvenile.reserve.withdraw(1.2));
        let soil_before = forest.soil[old_cell].litter;
        let body_before = forest.woodlice[1].body;
        let reserves_before = forest.woodlice[1].reserve;
        let brood_before = seasonal.woodlice[0].brood;

        let summer = crate::season::observe(TICKS_PER_DAY * 199);
        forest.update_life(&mut world.animals, summer);
        forest.validate(rock, &world.animals).unwrap();
        let terrain = ForestTerrain { legacy_rock: rock, layout: None };
        let after = forest.observe_seasonal(terrain, 0.5, &world.animals, summer).life.unwrap();
        assert_eq!((after.woodlice_born, after.woodlice_died), (1, 1));
        assert_eq!(after.woodlice.iter().map(|life| life.id.as_str()).collect::<Vec<_>>(), ["isopod-1", "isopod-3", "isopod-4", "isopod-5"]);
        assert_eq!(after.woodlice[2].stage, WoodlouseStage::Adult);
        assert_eq!(after.woodlice[3].stage, WoodlouseStage::Juvenile);
        assert_eq!(after.woodlice[3].age_seconds, 0.0);
        assert_eq!(after.woodlice[0].brood_progress, None);
        let newborn = forest.woodlice.last().unwrap();
        for (actual, expected) in newborn.body.values().into_iter().zip(brood_before.scaled(0.8).values()) {
            assert!((actual - expected).abs() < 1e-12);
        }
        let mut expected = soil_before;
        expected.add(body_before);
        expected.add(reserves_before);
        assert_eq!(forest.soil[old_cell].litter, expected);
    }

    #[test]
    fn season_and_wetness_drive_living_patches_and_vegetation_returns_to_litter() {
        let mut moist = WorldState::new_seasonal_forest(7);
        let mut dry = moist.clone();
        let summer = crate::season::observe(TICKS_PER_DAY * 199);
        let winter = crate::season::observe(0);
        let forest = moist.forest.as_ref().unwrap();
        let terrain = ForestTerrain { legacy_rock: moist.rock, layout: None };
        let summer_rate = forest.observe_seasonal(terrain, 0.5, &moist.animals, summer).evaporation_ml_per_hour;
        let winter_rate = forest.observe_seasonal(terrain, 0.5, &moist.animals, winter).evaporation_ml_per_hour;
        assert!(summer_rate[0] > winter_rate[0] && summer_rate[1] > winter_rate[1]);

        let index = 0;
        let source = moist.forest.as_mut().unwrap();
        let stressed = dry.forest.as_mut().unwrap();
        source.soil[index].water_ml = 24.0;
        stressed.soil[index].water_ml = 3.0;
        let patch = &mut stressed.seasonal.as_mut().unwrap().soil[index];
        patch.fern_stress = 0.80;
        patch.moss_stress = 0.80;
        let soil_before = stressed.soil[index].clone();
        let patch_before = patch.clone();
        let site = Site { shade: 0.40, shelter: 0.5, walkable: true };
        for forest in [source, stressed] {
            forest.balance.initial = forest.inventory(&[]);
            let patch = &mut forest.seasonal.as_mut().unwrap().soil[index];
            patch.advance(&mut forest.soil[index], site, 1.0, SoilForcing::seasonal(summer), &mut forest.balance);
        }
        let source = moist.forest.as_ref().unwrap();
        let stressed = dry.forest.as_ref().unwrap();
        let healthy_patch = &source.seasonal.as_ref().unwrap().soil[index];
        let stressed_patch = &stressed.seasonal.as_ref().unwrap().soil[index];
        assert!(healthy_patch.fern.mg_c > patch_before.fern.mg_c);
        assert!(healthy_patch.springtails.mg_c > patch_before.springtails.mg_c);
        assert!(source.soil[index].microbes.mg_c < soil_before.microbes.mg_c);
        assert!(stressed_patch.fern.mg_c < patch_before.fern.mg_c);
        assert!(stressed_patch.fern_stress > patch_before.fern_stress);
        assert!(stressed_patch.springtails.mg_c < patch_before.springtails.mg_c);
        assert!(stressed.soil[index].moss.mg_c < soil_before.moss.mg_c);
        assert!(stressed.soil[index].litter.mg_c > soil_before.litter.mg_c);
        moist.validate().unwrap();
        dry.validate().unwrap();
    }
}
    #[test]
    fn stonework_contacts_leave_real_paths_between_members() {
        use crate::ForestStoneworkKind;

        for (preset, blocked, open) in [
            (ForestStoneworkKind::Pair, &[4, 6][..], &[5][..]),
            (ForestStoneworkKind::Cluster, &[3, 5, 7][..], &[4, 6][..]),
        ] {
            let position = Point { x: 0.47, y: 0.715 };
            let piece = ForestStonework { id: 2, position, mirrored: false, preset };
            let geometry = site_geometry(std::iter::empty(), &[], &[piece], &[]);
            let row = cell_index(position) / COLUMNS;
            assert!(blocked.iter().all(|column| !geometry[row * COLUMNS + column].walkable));
            for column in open {
                let passage = row * COLUMNS + column;
                let before = passage - COLUMNS;
                let after = passage + COLUMNS;
                assert!(cardinal(before).contains(&Some(passage)));
                assert!(cardinal(passage).contains(&Some(after)));
                assert!(geometry[before].walkable && geometry[passage].walkable && geometry[after].walkable);
            }
        }
    }


use crate::pond::Matter;
use crate::pond_stones::{PondBedProfile, BASIN_WIDTH_PX};
use crate::CoreError;
use serde::{Deserialize, Serialize};

const MAX_WATER_PLANTS: usize = 8;
const MIN_ROOT_GAP_PX: f64 = 14.0;
const WHORLED_GROWTH_PER_HOUR: f64 = 0.020;
const RIBBON_GROWTH_PER_HOUR: f64 = 0.027;
const WHORLED_RESPIRATION_PER_HOUR: f64 = 0.003;
const RIBBON_RESPIRATION_PER_HOUR: f64 = 0.004;

/// Experimental dose for the established, pooled water plant functional type.
pub(super) fn dose() -> Matter {
    Matter::with_ratios(50.0, 0.06, 0.007)
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PondRibbonPlantPreset { Small, Medium, Large }

impl PondRibbonPlantPreset {
    fn dose(self) -> Matter {
        let carbon = match self { Self::Small => 30.0, Self::Medium => 50.0, Self::Large => 75.0 };
        Matter::with_ratios(carbon, 0.06, 0.007)
    }

    fn leaf_length_cm(self) -> f64 {
        match self { Self::Small => 12.0, Self::Medium => 18.0, Self::Large => 24.0 }
    }

    fn light_transmission(self, depth_cm: f64, turbidity: f64, carbon: f64) -> f64 {
        // Leaf arc length is not vertical height. Evaluate light halfway up the submerged blade.
        let relative_length = (carbon / self.dose().carbon()).sqrt().clamp(0.30, 1.25);
        let vertical_reach_cm = (self.leaf_length_cm() * relative_length * 0.65).min(depth_cm);
        let leaf_depth_cm = depth_cm - vertical_reach_cm * 0.5;
        (-(0.035 + 0.22 * turbidity) * leaf_depth_cm).exp()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PondWaterPlantObservation {
    pub id: u32,
    pub x: f64,
    pub initial_mg_cnp: [f64; 3],
    pub stock_mg_cnp: [f64; 3],
    /// Indicates whether the root at the basin floor is underwater.
    pub submerged: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ribbon_preset: Option<PondRibbonPlantPreset>,
    /// Fraction of light reaching the representative submerged ribbon leaf, excluding time of day.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ribbon_light_transmission: Option<f64>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PlantPatch {
    id: u32,
    x: f64,
    weight: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ribbon_preset: Option<PondRibbonPlantPreset>,
}

/// All cohorts share one C:N:P ratio and partition the basin's single plant inventory.
/// Old whorled-only layouts keep their original fixed allocation during ecology steps.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PondPlantLayout {
    next_id: u32,
    unassigned_weight: f64,
    patches: Vec<PlantPatch>,
}

pub(super) struct PlantDemands {
    unassigned: f64,
    patches: [f64; MAX_WATER_PLANTS],
}

impl PlantDemands {
    pub(super) fn total(&self) -> f64 { self.unassigned + self.patches.iter().sum::<f64>() }
}

pub(super) fn check_root(x: f64, mut profiles: impl Iterator<Item = PondBedProfile>) -> Result<(), CoreError> {
    if !(0.08..=0.92).contains(&x) {
        return Err(CoreError::InvalidPondWaterPlantSite);
    }
    if profiles.any(|profile| profile.blocks_root(x)) {
        return Err(CoreError::PondWaterPlantSiteOccupied);
    }
    Ok(())
}

fn roots_touch(a: f64, b: f64) -> bool { (a - b).abs() * BASIN_WIDTH_PX < MIN_ROOT_GAP_PX }

fn share(weight: f64, total: f64) -> f64 { if total > 0.0 { weight / total } else { 0.0 } }

impl PondPlantLayout {
    pub(super) fn new(unassigned_carbon: f64) -> Self {
        Self { next_id: 1, unassigned_weight: unassigned_carbon, patches: Vec::new() }
    }

    fn weight_total(&self) -> f64 { self.unassigned_weight + self.patches.iter().map(|patch| patch.weight).sum::<f64>() }

    pub(super) fn preserve_allocations(&mut self, current: &Self) {
        self.next_id = self.next_id.max(current.next_id);
    }

    pub(super) fn observe(&self, pool: Matter, wet: bool, depth_cm: f64, turbidity: f64) -> (Vec<PondWaterPlantObservation>, [f64; 3]) {
        let total = self.weight_total();
        let patches = self.patches.iter().map(|patch| {
            let stock = pool.scaled(share(patch.weight, total));
            PondWaterPlantObservation {
                id: patch.id, x: patch.x, initial_mg_cnp: patch.ribbon_preset.map_or_else(dose, PondRibbonPlantPreset::dose).0,
                stock_mg_cnp: stock.0, submerged: wet, ribbon_preset: patch.ribbon_preset,
                ribbon_light_transmission: patch.ribbon_preset.map(|preset| {
                    if wet { preset.light_transmission(depth_cm, turbidity, stock.carbon()) } else { 0.0 }
                }),
            }
        }).collect();
        let unassigned = if total > 0.0 { pool.scaled(share(self.unassigned_weight, total)) } else { pool };
        (patches, unassigned.0)
    }

    pub(super) fn has_patches(&self) -> bool { !self.patches.is_empty() }

    pub(super) fn rooted_section(&self, stock: Matter) -> (Self, Matter) {
        let mut layout = self.clone();
        let rooted_weight = self.patches.iter().map(|patch| patch.weight).sum::<f64>();
        layout.unassigned_weight = 0.0;
        (layout, stock.scaled(share(rooted_weight, self.weight_total())))
    }

    pub(super) fn valid_section(&self, stock: Matter, profiles: impl Iterator<Item = PondBedProfile> + Clone) -> bool {
        stock.valid() && self.unassigned_weight == 0.0 && self.valid(stock, profiles)
    }

    pub(super) fn topology_split_stock(&mut self, stock: Matter, fraction: f64, retain_section: bool) -> Matter {
        let total = self.weight_total();
        let unassigned = if total > 0.0 { share(self.unassigned_weight, total) } else { 1.0 };
        if retain_section {
            self.unassigned_weight *= fraction;
            stock.scaled(1.0 - unassigned * (1.0 - fraction))
        } else {
            stock.scaled(unassigned * fraction)
        }
    }

    fn demands(&self, carbon: f64, whorled_rate: f64,
        mut patch_rate: impl FnMut(Option<PondRibbonPlantPreset>, f64) -> f64,
    ) -> Option<PlantDemands> {
        if !self.patches.iter().any(|patch| patch.ribbon_preset.is_some()) { return None; }
        let total = self.weight_total();
        let mut result = PlantDemands { unassigned: carbon * share(self.unassigned_weight, total) * whorled_rate,
            patches: [0.0; MAX_WATER_PLANTS] };
        for (patch, demand) in self.patches.iter().zip(&mut result.patches) {
            let stock = carbon * share(patch.weight, total);
            *demand = stock * patch_rate(patch.ribbon_preset, stock);
        }
        Some(result)
    }

    pub(super) fn growth_demand(&self, carbon: f64, light: f64, turbidity: f64, depth_cm: f64,
        elapsed: f64,
    ) -> Option<PlantDemands> {
        let whorled_rate = WHORLED_GROWTH_PER_HOUR * light * (1.0 - 0.65 * turbidity) * elapsed;
        self.demands(carbon, whorled_rate, |preset, stock| {
            preset.map_or(whorled_rate, |ribbon| {
                RIBBON_GROWTH_PER_HOUR * light * ribbon.light_transmission(depth_cm, turbidity, stock) * elapsed
            })
        })
    }

    pub(super) fn respiration_demand(&self, carbon: f64, elapsed: f64) -> Option<PlantDemands> {
        let whorled_rate = WHORLED_RESPIRATION_PER_HOUR * elapsed;
        self.demands(carbon, whorled_rate, |preset, _| {
            if preset.is_some() { RIBBON_RESPIRATION_PER_HOUR * elapsed } else { whorled_rate }
        })
    }

    /// Match the allocation weights to the actual carbon credited or debited by the basin.
    pub(super) fn apply_flux(&mut self, previous_carbon: f64, demands: PlantDemands, allocation: f64) {
        if allocation == 0.0 { return; }
        self.rescale(previous_carbon);
        self.unassigned_weight = (self.unassigned_weight + demands.unassigned * allocation).max(0.0);
        for (patch, demand) in self.patches.iter_mut().zip(demands.patches) {
            patch.weight = (patch.weight + demand * allocation).max(0.0);
        }
    }

    pub(super) fn valid(&self, pool: Matter, profiles: impl Iterator<Item = PondBedProfile> + Clone) -> bool {
        let valid_weight = |weight: f64| weight.is_finite() && weight >= 0.0;
        let total = self.weight_total();
        self.next_id > 0 && self.patches.len() <= MAX_WATER_PLANTS
            && valid_weight(self.unassigned_weight) && total.is_finite() && (total > 0.0 || pool.carbon() == 0.0)
            && self.patches.iter().enumerate().all(|(index, patch)| {
                patch.id > 0 && patch.id < self.next_id && valid_weight(patch.weight)
                    && check_root(patch.x, profiles.clone()).is_ok()
                    && self.patches[..index].iter().all(|prior| prior.id < patch.id && !roots_touch(prior.x, patch.x))
            })
    }

    pub(super) fn blocks_structure(&self, profile: PondBedProfile) -> bool {
        self.patches.iter().any(|patch| profile.blocks_root(patch.x))
    }

    fn rescale(&mut self, current_carbon: f64) {
        let total = self.weight_total();
        self.unassigned_weight = share(self.unassigned_weight, total) * current_carbon;
        for patch in &mut self.patches {
            patch.weight = share(patch.weight, total) * current_carbon;
        }
    }

    pub(super) fn add_unassigned_stock(&mut self, current_carbon: f64, added_carbon: f64) {
        self.rescale(current_carbon);
        self.unassigned_weight += added_carbon;
    }

    pub(super) fn reset_stocks(&mut self, unassigned_carbon: f64) -> Matter {
        self.unassigned_weight = unassigned_carbon;
        let mut total = 0.0;
        let mut ribbon = false;
        for patch in &mut self.patches {
            ribbon |= patch.ribbon_preset.is_some();
            patch.weight = patch.ribbon_preset.map_or_else(dose, PondRibbonPlantPreset::dose).carbon();
            total += patch.weight;
        }
        if ribbon { Matter::with_ratios(total, 0.06, 0.007) } else { dose().scaled(self.patches.len() as f64) }
    }

    pub(super) fn place(&mut self, x: f64, current_carbon: f64, ribbon_preset: Option<PondRibbonPlantPreset>) -> Result<Matter, CoreError> {
        if self.patches.iter().any(|patch| roots_touch(patch.x, x)) {
            return Err(CoreError::PondWaterPlantSiteOccupied);
        }
        if self.patches.len() >= MAX_WATER_PLANTS { return Err(CoreError::PondWaterPlantLimit); }
        let next_id = self.next_id.checked_add(1).ok_or(CoreError::PondWaterPlantLimit)?;
        self.rescale(current_carbon);
        let stock = ribbon_preset.map_or_else(dose, PondRibbonPlantPreset::dose);
        self.patches.push(PlantPatch { id: self.next_id, x, weight: stock.carbon(), ribbon_preset });
        self.next_id = next_id;
        Ok(stock)
    }

    pub(super) fn move_patch(&mut self, id: u32, x: f64) -> Result<(), CoreError> {
        let index = self.index(id)?;
        if self.patches.iter().any(|patch| patch.id != id && roots_touch(patch.x, x)) {
            return Err(CoreError::PondWaterPlantSiteOccupied);
        }
        self.patches[index].x = x;
        Ok(())
    }

    pub(super) fn trim(&mut self, id: u32, mg_c: f64, current_carbon: f64) -> Result<f64, CoreError> {
        let index = self.index(id)?;
        let available = share(self.patches[index].weight, self.weight_total()) * current_carbon;
        let fraction = mg_c / current_carbon;
        if !(mg_c < available && fraction > 0.0 && fraction < 1.0) { return Err(CoreError::InvalidPondWaterPlantTrim); }
        self.rescale(current_carbon);
        self.patches[index].weight = available - mg_c;
        Ok(fraction)
    }

    pub(super) fn remove(&mut self, id: u32, current_carbon: f64) -> Result<f64, CoreError> {
        let index = self.index(id)?;
        let fraction = share(self.patches[index].weight, self.weight_total());
        self.rescale(current_carbon);
        self.patches.remove(index);
        Ok(fraction)
    }

    fn index(&self, id: u32) -> Result<usize, CoreError> {
        self.patches.iter().position(|patch| patch.id == id).ok_or(CoreError::PondWaterPlantMissing)
    }
}

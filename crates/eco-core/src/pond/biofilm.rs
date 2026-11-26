use super::{section::PondStaticSection, Basin, Matter, PondBasinContactFeeding, MAX_SHRIMP};
use crate::pond_stones::{PondShrimpStoneAttachment, PondStone, PondStoneLayout, PondStonePreset, BASIN_WIDTH_PX, MAX_STONES};
use crate::pond_wood::PondWoodLayout;
use serde::{Deserialize, Serialize};

// Scale from the composed pond's existing projected bed display.
const BED_WEIGHT: f64 = BASIN_WIDTH_PX * 36.0;
const STONE_CAPACITY_MG_C: f64 = 12.0;
const STONE_COLONIZATION_PER_HOUR: f64 = 0.02;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PondBiofilmSurfaceKind { Stone, Wood }

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PondBiofilmSurface {
    pub kind: PondBiofilmSurfaceKind,
    pub id: u32,
    pub mg_c: f64,
}

/// Visible and inactive portions of the basin's biofilm inventory.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PondBiofilmDistribution {
    pub bed_mg_c: f64,
    pub inactive_mg_c: f64,
    pub surfaces: Vec<PondBiofilmSurface>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PondStoneBiofilmObservation {
    pub stone_id: u32,
    pub stock_mg_c: f64,
    pub available_mg_c: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct StoneBiofilm {
    pub(super) stone_id: u32,
    pub(super) stock: Matter,
}

impl StoneBiofilm {
    pub(super) fn empty(stone_id: u32) -> Self { Self { stone_id, stock: Matter::default() } }

    fn observe(&self, wet: f64) -> PondStoneBiofilmObservation {
        PondStoneBiofilmObservation {
            stone_id: self.stone_id, stock_mg_c: self.stock.carbon(), available_mg_c: self.stock.carbon() * wet,
        }
    }

    pub(super) fn validate(stocks: &[Self], layout: Option<&PondStoneLayout>) -> bool {
        let Some(layout) = layout.filter(|layout| layout.stone_rest_enabled) else { return false; };
        layout.stones.iter().filter(|stone| stone.preset == PondStonePreset::Main)
            .map(|stone| stone.id).eq(stocks.iter().map(|patch| patch.stone_id))
            && stocks.iter().all(|patch| patch.stock.valid() && patch.stock.carbon() <= STONE_CAPACITY_MG_C + 1e-9)
    }

    fn vacant_mg_c(&self) -> f64 { (STONE_CAPACITY_MG_C - self.stock.carbon()).max(0.0) }

    pub(super) fn growth_demand(&self, wet: f64, opportunity: f64) -> Matter {
        let space = self.vacant_mg_c();
        let carbon = self.stock.carbon() * 0.037 * opportunity * wet * (space / STONE_CAPACITY_MG_C);
        Matter::with_ratios(carbon.min(space), 0.077, 0.010)
    }

    pub(super) fn respiration_demand(&self, wet: f64, elapsed: f64) -> f64 {
        self.stock.carbon() * 0.004 * elapsed * wet
    }
}

fn wet_fraction(stone: &PondStone, column: f64) -> f64 {
    let profile = stone.profile();
    profile.wetted_weight(column) / profile.wetted_weight(f64::INFINITY)
}

fn wet_fractions(stones: Option<&PondStoneLayout>, column: f64) -> [f64; MAX_STONES] {
    let mut fractions = [0.0; MAX_STONES];
    if column > 0.0 {
        let stones = stones.iter().flat_map(|layout| &layout.stones)
            .filter(|stone| stone.preset == PondStonePreset::Main);
        for (fraction, stone) in fractions.iter_mut().zip(stones) { *fraction = wet_fraction(stone, column); }
    }
    fractions
}

fn seed_stone_biofilm(stones: Option<&PondStoneLayout>, woods: Option<&PondWoodLayout>, shared: &mut Matter) -> Vec<StoneBiofilm> {
    let stones = stones.iter().flat_map(|layout| &layout.stones).filter(|stone| stone.preset == PondStonePreset::Main);
    let full_weight = |stone: &PondStone| stone.profile().wetted_weight(f64::INFINITY);
    let wood_weight = woods.iter().flat_map(|layout| &layout.woods)
        .map(|wood| wood.profile().wetted_weight(f64::INFINITY)).sum::<f64>();
    let total_weight = BED_WEIGHT + stones.clone().map(full_weight).sum::<f64>() + wood_weight;
    let start = shared.carbon();
    stones.map(|stone| {
        let seeded = (start * (full_weight(stone) / total_weight)).min(STONE_CAPACITY_MG_C);
        StoneBiofilm { stone_id: stone.id, stock: shared.take_carbon(seeded) }
    }).collect()
}

fn colonize_stocks(stocks: &mut [StoneBiofilm], wet: [f64; MAX_STONES], shared: &mut Matter, budget_per_stone: f64) {
    for (patch, wet) in stocks.iter_mut().zip(wet) {
        let space = patch.vacant_mg_c();
        let colonized = budget_per_stone * wet * (space / STONE_CAPACITY_MG_C);
        patch.stock.add(shared.take_carbon(colonized.min(space)));
    }
}

impl Basin {
    pub(super) fn initialize_stone_biofilm(&mut self) {
        self.stone_biofilm = Some(seed_stone_biofilm(self.stone_layout.as_ref(), self.wood_layout.as_ref(), &mut self.biofilm));
    }

    pub(super) fn stone_biofilm_wet_fractions(&self) -> [f64; MAX_STONES] {
        if self.stone_biofilm.is_none() { return [0.0; MAX_STONES]; }
        wet_fractions(self.stone_layout.as_ref(), self.moss_column_px())
    }

    pub(super) fn stone_biofilm_observation(&self) -> Option<Vec<PondStoneBiofilmObservation>> {
        let stocks = self.stone_biofilm.as_ref()?;
        Some(stocks.iter().zip(self.stone_biofilm_wet_fractions()).map(|(stock, wet)| stock.observe(wet)).collect())
    }

    pub(super) fn total_biofilm_carbon(&self) -> f64 {
        let primary = self.biofilm.carbon() + self.stone_biofilm.iter().flatten().map(|patch| patch.stock.carbon()).sum::<f64>();
        primary + self.static_sections.iter().flat_map(|section| section.stone_biofilm.iter().flatten())
            .map(|patch| patch.stock.carbon()).sum::<f64>()
    }

    pub(super) fn colonize_stone_biofilm(&mut self, elapsed: f64) {
        let count = self.stone_biofilm.as_ref().map_or(0, Vec::len)
            + self.static_sections.iter().filter_map(|section| section.stone_biofilm.as_ref()).map(Vec::len).sum::<usize>();
        if count == 0 { return; }
        let column = self.moss_column_px();
        let budget_per_stone = self.biofilm.carbon() * STONE_COLONIZATION_PER_HOUR * elapsed / count.max(MAX_STONES) as f64;
        let primary_wet = self.stone_biofilm_wet_fractions();
        if let Some(stocks) = &mut self.stone_biofilm {
            colonize_stocks(stocks, primary_wet, &mut self.biofilm, budget_per_stone);
        }
        for section in &mut self.static_sections {
            let wet = section.stone_biofilm_wet_fractions(column);
            if let Some(stocks) = &mut section.stone_biofilm {
                colonize_stocks(stocks, wet, &mut self.biofilm, budget_per_stone);
            }
        }
    }

    pub(super) fn allocate_stone_food(
        stocks: &mut [StoneBiofilm], contacts: &[Option<&PondShrimpStoneAttachment>; MAX_SHRIMP], claims: &mut [f64; MAX_SHRIMP],
        wet: [f64; MAX_STONES],
    ) -> [Matter; MAX_SHRIMP] {
        let mut local = [Matter::default(); MAX_SHRIMP];
        for (patch, wet) in stocks.iter_mut().zip(wet) {
            let members = contacts.iter().map(|contact| contact.is_some_and(|contact| contact.stone_id == patch.stone_id));
            let demand = members.clone().zip(claims.iter()).filter_map(|(member, claim)| member.then_some(claim)).sum::<f64>();
            if demand == 0.0 { continue; }
            let food = patch.stock.take_carbon(demand.min(patch.stock.carbon() * wet));
            for ((parcel, claim), member) in local.iter_mut().zip(claims.iter_mut()).zip(members) {
                if member {
                    *parcel = food.scaled(*claim / demand);
                    *claim = (*claim - parcel.carbon()).max(0.0);
                }
            }
        }
        local
    }

    pub(super) fn biofilm_distribution(&self) -> PondBiofilmDistribution {
        distribute_biofilm(self.stone_layout.as_ref(), self.wood_layout.as_ref(), self.stone_biofilm.as_deref(),
            self.biofilm, self.moss_column_px())
    }
}

impl PondStaticSection {
    pub(super) fn initialize_stone_biofilm(&mut self, shared: &mut Matter) {
        self.stone_biofilm = Some(seed_stone_biofilm(Some(&self.stones), Some(&self.woods), shared));
    }

    pub(super) fn stone_biofilm_wet_fractions(&self, column: f64) -> [f64; MAX_STONES] {
        if self.stone_biofilm.is_none() { return [0.0; MAX_STONES]; }
        wet_fractions(Some(&self.stones), column)
    }

    pub(super) fn stone_biofilm_observation(&self, column: f64) -> Option<Vec<PondStoneBiofilmObservation>> {
        let stocks = self.stone_biofilm.as_ref()?;
        Some(stocks.iter().zip(self.stone_biofilm_wet_fractions(column)).map(|(stock, wet)| stock.observe(wet)).collect())
    }

    pub(super) fn biofilm_distribution(&self, shared: Matter, column: f64) -> PondBiofilmDistribution {
        distribute_biofilm(Some(&self.stones), Some(&self.woods), self.stone_biofilm.as_deref(), shared, column)
    }
}

// Geometry and local stock describe only this view. Different views reuse the
// same shared stock projection, so distributions must not be summed across rows.
fn distribute_biofilm(stones: Option<&PondStoneLayout>, woods: Option<&PondWoodLayout>,
    localized: Option<&[StoneBiofilm]>, shared: Matter, column: f64,
) -> PondBiofilmDistribution {
    let total = shared.carbon();
    if column <= 0.0 {
        let local = localized.iter().flat_map(|stocks| stocks.iter()).map(|patch| patch.stock.carbon()).sum::<f64>();
        return PondBiofilmDistribution { inactive_mg_c: total + local, ..PondBiofilmDistribution::default() };
    }
    let woods = woods.iter().flat_map(|layout| &layout.woods);
    if let Some(localized) = localized {
        let mut inactive_mg_c = 0.0;
        let mut surfaces = Vec::new();
        for (patch, wet) in localized.iter().zip(wet_fractions(stones, column)) {
            let mg_c = patch.stock.carbon() * wet;
            inactive_mg_c += patch.stock.carbon() - mg_c;
            if mg_c > 0.0 { surfaces.push(PondBiofilmSurface { kind: PondBiofilmSurfaceKind::Stone, id: patch.stone_id, mg_c }); }
        }
        let available_weight = BED_WEIGHT + woods.clone().map(|wood| wood.profile().wetted_weight(f64::INFINITY)).sum::<f64>();
        let mut bed_mg_c = total;
        for wood in woods {
            let share = wood.profile().wetted_weight(column) / available_weight;
            let mg_c = (total * share).min(bed_mg_c);
            if mg_c > 0.0 {
                bed_mg_c -= mg_c;
                surfaces.push(PondBiofilmSurface { kind: PondBiofilmSurfaceKind::Wood, id: wood.id, mg_c });
            }
        }
        return PondBiofilmDistribution { bed_mg_c, inactive_mg_c, surfaces };
    }
    if total == 0.0 { return PondBiofilmDistribution::default(); }
    let stones = stones.iter().flat_map(|layout| &layout.stones).filter(|stone| stone.preset == PondStonePreset::Main)
        .map(|stone| (PondBiofilmSurfaceKind::Stone, stone.id, stone.profile()));
    let terrain = stones.chain(woods.map(|wood| (PondBiofilmSurfaceKind::Wood, wood.id, wood.profile())));
    let available_weight = terrain.clone().map(|(_, _, profile)| profile.wetted_weight(f64::INFINITY)).sum::<f64>() + BED_WEIGHT;
    let mut bed_mg_c = total;
    let surfaces = terrain.filter_map(|(kind, id, profile)| {
        let mg_c = (total * (profile.wetted_weight(column) / available_weight)).min(bed_mg_c);
        if mg_c == 0.0 { return None; }
        bed_mg_c -= mg_c;
        Some(PondBiofilmSurface { kind, id, mg_c })
    }).collect();
    PondBiofilmDistribution { bed_mg_c, inactive_mg_c: 0.0, surfaces }
}

impl PondBiofilmDistribution {
    /// Allocate each surface's pre-feed limit across its eligible actual intake parcels.
    /// Any unassigned part of the original debit is reported as shared food.
    pub(super) fn attribute_contact_intake(&self, entries: &mut PondBasinContactFeeding, localized_stones: bool) {
        self.attribute_surface_intake(
            PondBiofilmSurfaceKind::Wood, &mut entries.wood,
            |entry| (entry.wood_id, entry.biofilm_mg_c),
            |entry, local, shared| { entry.wood_biofilm_mg_c = local; entry.shared_biofilm_mg_c = shared; },
        );
        if !localized_stones {
            self.attribute_surface_intake(
                PondBiofilmSurfaceKind::Stone, &mut entries.stone,
                |entry| (entry.stone_id, entry.biofilm_mg_c),
                |entry, local, shared| { entry.stone_biofilm_mg_c = local; entry.shared_biofilm_mg_c = shared; },
            );
        }
    }

    fn attribute_surface_intake<T>(
        &self, kind: PondBiofilmSurfaceKind, entries: &mut [T], claim: impl Fn(&T) -> (u32, f64),
        mut assign: impl FnMut(&mut T, f64, f64),
    ) {
        if entries.is_empty() { return; }
        for surface in self.surfaces.iter().filter(|surface| surface.kind == kind) {
            let total_claim = entries.iter().map(&claim).filter(|(id, _)| *id == surface.id)
                .map(|(_, amount)| amount).sum::<f64>();
            if total_claim == 0.0 { continue; }
            let fraction = (surface.mg_c / total_claim).min(1.0);
            let mut remaining = surface.mg_c;
            for entry in entries.iter_mut() {
                let (id, amount) = claim(entry);
                if id != surface.id { continue; }
                let local = (amount * fraction).min(remaining);
                assign(entry, local, amount - local);
                remaining -= local;
            }
        }
    }
}

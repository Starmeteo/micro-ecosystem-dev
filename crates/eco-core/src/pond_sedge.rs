use crate::pond::Matter;
use crate::pond_shape::{CELL_COUNT, COLUMNS, PondShape, PondShapeError};
use crate::CoreError;
use serde::{Deserialize, Serialize};

pub(super) const MAX_SEDGE_PATCHES: usize = 8;
// Design reference, in millimetres above the section floor. The plant and roots share it.
const ROOT_COLLAR_MM: f64 = 36.0;
const N_PER_C: f64 = 0.035;
const P_PER_C: f64 = 0.005;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PondSedgePreset { Small, Medium, Large }

impl PondSedgePreset {
    fn dimensions_mm(self) -> (f64, f64) {
        match self {
            Self::Small => (150.0, 25.0),
            Self::Medium => (220.0, 40.0),
            Self::Large => (300.0, 60.0),
        }
    }

    fn dose(self) -> Matter {
        let carbon = match self { Self::Small => 18.0, Self::Medium => 32.0, Self::Large => 52.0 };
        Matter::with_ratios(carbon, N_PER_C, P_PER_C)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PondShoreAnchor {
    pub water_cell: u16,
    pub shore_cell: u16,
}

impl PondShoreAnchor {
    pub(super) fn valid(self, shape: &PondShape) -> bool {
        let water = usize::from(self.water_cell);
        let shore = usize::from(self.shore_cell);
        if water >= CELL_COUNT || shore >= CELL_COUNT { return false; }
        let distance = (water / COLUMNS).abs_diff(shore / COLUMNS) + (water % COLUMNS).abs_diff(shore % COLUMNS);
        distance == 1 && shape.cells().binary_search(&self.water_cell).is_ok()
            && shape.cells().binary_search(&self.shore_cell).is_err()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PondSedgeObservation {
    pub id: u32,
    pub anchor: PondShoreAnchor,
    pub preset: PondSedgePreset,
    pub shoot_height_mm: f64,
    pub root_diameter_mm: f64,
    pub root_collar_mm: f64,
    pub root_wet_fraction: f64,
    pub initial_mg_cnp: [f64; 3],
    pub stock_mg_cnp: [f64; 3],
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct SedgePatch {
    id: u32,
    anchor: PondShoreAnchor,
    preset: PondSedgePreset,
    pub(super) stock: Matter,
}

impl SedgePatch {
    fn root_wet_fraction(self, water_depth_mm: f64) -> f64 {
        let root_depth = self.preset.dimensions_mm().1 / 2.0;
        ((water_depth_mm - (ROOT_COLLAR_MM - root_depth)) / root_depth).clamp(0.0, 1.0)
    }

    pub(super) fn growth(self, water_depth_mm: f64, opportunity: f64) -> Matter {
        // Experimental coefficients. Wet roots use basin minerals and aerial leaves fix carbon.
        let carbon = self.stock.carbon();
        let room = (1.0 - carbon / (2.0 * self.preset.dose().carbon())).max(0.0);
        Matter::with_ratios(carbon * 0.016 * room * self.root_wet_fraction(water_depth_mm) * opportunity, N_PER_C, P_PER_C)
    }

    pub(super) fn maintenance(&mut self, water_depth_mm: f64, elapsed_hours: f64) -> Matter {
        let limit = self.stock.carbon() * 0.002 * self.root_wet_fraction(water_depth_mm) * elapsed_hours;
        self.stock.take_carbon(limit)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct SedgeLayout {
    next_id: u32,
    pub(super) patches: Vec<SedgePatch>,
}

impl SedgeLayout {
    pub(super) fn new() -> Self { Self { next_id: 1, patches: Vec::new() } }

    pub(super) fn place(&mut self, anchor: PondShoreAnchor, preset: PondSedgePreset) -> Result<Matter, CoreError> {
        if self.patches.len() >= MAX_SEDGE_PATCHES { return Err(CoreError::PondSedgeLimit); }
        self.check_occupancy(anchor, None)?;
        let next = self.next_id.checked_add(1).ok_or(CoreError::PondSedgeLimit)?;
        let stock = preset.dose();
        self.patches.push(SedgePatch { id: self.next_id, preset, anchor, stock });
        self.next_id = next;
        Ok(stock)
    }

    pub(super) fn relocate(&mut self, id: u32, anchor: PondShoreAnchor) -> Result<(), CoreError> {
        let index = self.index(id)?;
        self.check_occupancy(anchor, Some(id))?;
        self.patches[index].anchor = anchor;
        Ok(())
    }

    pub(super) fn remove(&mut self, id: u32) -> Result<Matter, CoreError> {
        let index = self.index(id)?;
        Ok(self.patches.remove(index).stock)
    }

    fn index(&self, id: u32) -> Result<usize, CoreError> {
        self.patches.iter().position(|patch| patch.id == id).ok_or(CoreError::PondSedgeMissing)
    }

    fn check_occupancy(&self, anchor: PondShoreAnchor, moving: Option<u32>) -> Result<(), CoreError> {
        if self.patches.iter().any(|patch| Some(patch.id) != moving && patch.anchor.shore_cell == anchor.shore_cell) {
            return Err(CoreError::InvalidPondSedgeSite);
        }
        Ok(())
    }

    pub(super) fn preserve_allocations(&mut self, current: &Self) { self.next_id = self.next_id.max(current.next_id); }

    pub(super) fn reset_stocks(&mut self) {
        for patch in &mut self.patches { patch.stock = patch.preset.dose(); }
    }

    pub(super) fn observe(&self, depth_mm: f64) -> Option<Vec<PondSedgeObservation>> {
        (!self.patches.is_empty()).then(|| self.patches.iter().map(|patch| {
            let (shoot_height_mm, root_diameter_mm) = patch.preset.dimensions_mm();
            PondSedgeObservation { id: patch.id, anchor: patch.anchor, preset: patch.preset,
                shoot_height_mm, root_diameter_mm, root_collar_mm: ROOT_COLLAR_MM,
                root_wet_fraction: patch.root_wet_fraction(depth_mm),
                initial_mg_cnp: patch.preset.dose().0, stock_mg_cnp: patch.stock.0 }
        }).collect())
    }

    pub(super) fn anchors_valid(&self, shape: &PondShape) -> bool {
        self.patches.iter().all(|patch| patch.anchor.valid(shape))
    }

    pub(super) fn uses_shore(&self, cell: u16) -> bool {
        self.patches.iter().any(|patch| patch.anchor.shore_cell == cell)
    }

    pub(super) fn anchors_in(&self, footprint: &[bool; CELL_COUNT]) -> bool {
        self.patches.iter().all(|patch| footprint[usize::from(patch.anchor.water_cell)])
    }

    pub(super) fn retain_in(&mut self, shape: &PondShape) {
        self.patches.retain(|patch| shape.cells().binary_search(&patch.anchor.water_cell).is_ok());
    }

    pub(super) fn merge_from(&mut self, source: &Self) -> Result<(), PondShapeError> {
        if self.patches.len() + source.patches.len() > MAX_SEDGE_PATCHES { return Err(PondShapeError::TopologyPopulation); }
        for patch in &source.patches {
            self.check_occupancy(patch.anchor, None).map_err(|_| PondShapeError::TopologySedgeAnchor)?;
            let id = self.next_id;
            self.next_id = id.checked_add(1).ok_or(PondShapeError::TopologyPopulation)?;
            self.patches.push(SedgePatch { id, ..*patch });
        }
        Ok(())
    }

    pub(super) fn valid(&self, shape: Option<&PondShape>) -> bool {
        self.next_id > 0 && self.patches.len() <= MAX_SEDGE_PATCHES
            && self.patches.iter().enumerate().all(|(index, patch)| {
                patch.id > 0 && patch.id < self.next_id && patch.stock.valid()
                    && (index == 0 || self.patches[index - 1].id < patch.id)
                    && shape.is_some_and(|shape| patch.anchor.valid(shape))
                    && !self.patches[..index].iter().any(|other| other.anchor.shore_cell == patch.anchor.shore_cell)
            })
    }
}

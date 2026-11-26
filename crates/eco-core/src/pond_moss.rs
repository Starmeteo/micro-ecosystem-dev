use crate::pond::Matter;
use crate::pond_stones::{PondStoneLayout, PondStonePreset, BASIN_WIDTH_PX};
use crate::pond_wood::PondWoodLayout;
use crate::CoreError;
use serde::{Deserialize, Serialize};

// Experimental fixture values for one virtual attached producer functional type.
const N_TO_C: f64 = 0.062;
const P_TO_C: f64 = 0.008;
const GROWTH_PER_HOUR: f64 = 0.055;
const RESPIRATION_PER_HOUR: f64 = 0.0045;
const SENESCENCE_PER_HOUR: f64 = 0.0015;
pub(super) const MAX_MOSS_PATCHES: usize = 6;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum PondMossParent {
    Stone { id: u32 },
    Wood { id: u32 },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PondMossPreset {
    Mat,
    Frond,
}

impl PondMossPreset {
    pub fn initial_mg_cnp(self) -> [f64; 3] {
        let carbon = match self {
            Self::Mat => 3.0,
            Self::Frond => 7.5,
        };
        Matter::with_ratios(carbon, N_TO_C, P_TO_C).0
    }

    pub fn capacity_mg_c(self) -> f64 {
        match self {
            Self::Mat => 12.0,
            Self::Frond => 24.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PondMossPresetObservation {
    pub preset: PondMossPreset,
    pub initial_mg_cnp: [f64; 3],
}

impl PondMossPresetObservation {
    pub(super) fn choices() -> [Self; 2] {
        [PondMossPreset::Mat, PondMossPreset::Frond].map(|preset| Self { preset, initial_mg_cnp: preset.initial_mg_cnp() })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PondMossPatchObservation {
    pub id: u32,
    pub parent: PondMossParent,
    pub offset: f64,
    pub preset: PondMossPreset,
    pub stock_mg_cnp: [f64; 3],
    pub x: f64,
    pub surface_height_px: f64,
    pub submerged: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PondMossPatch {
    pub id: u32,
    pub parent: PondMossParent,
    pub offset: f64,
    pub preset: PondMossPreset,
    pub stock_mg_cnp: Matter,
}

impl PondMossPatch {
    pub(super) fn observe(self, support: MossSupport, column_px: f64) -> PondMossPatchObservation {
        PondMossPatchObservation {
            id: self.id,
            parent: self.parent,
            offset: self.offset,
            preset: self.preset,
            stock_mg_cnp: self.stock_mg_cnp.0,
            x: support.x,
            surface_height_px: support.height_px,
            submerged: support.submerged(column_px),
        }
    }

    pub(super) fn growth_demand(self, opportunity: f64) -> Matter {
        let stock = self.stock_mg_cnp.carbon();
        let capacity = self.preset.capacity_mg_c();
        let space = (capacity - stock).max(0.0);
        let carbon = (stock * GROWTH_PER_HOUR * opportunity * space / capacity).min(space);
        Matter::with_ratios(carbon, N_TO_C, P_TO_C)
    }

    pub(super) fn respiration_demand(self, elapsed: f64) -> f64 {
        self.stock_mg_cnp.carbon() * RESPIRATION_PER_HOUR * elapsed
    }

    pub(super) fn senesce(&mut self, elapsed: f64) -> Matter {
        self.stock_mg_cnp.take_fraction((SENESCENCE_PER_HOUR * elapsed).min(1.0))
    }
}

#[derive(Clone, Copy)]
pub(super) struct MossSupport {
    x: f64,
    height_px: f64,
}

impl MossSupport {
    pub(super) fn submerged(self, column_px: f64) -> bool { column_px > self.height_px }
}

pub(super) fn support(
    parent: PondMossParent, offset: f64, stones: Option<&PondStoneLayout>, woods: Option<&PondWoodLayout>,
) -> Result<MossSupport, CoreError> {
    check_offset(offset)?;
    match parent {
        PondMossParent::Stone { id } => {
            let stone = stones.and_then(|layout| layout.stones.iter().find(|stone| stone.id == id))
                .ok_or(CoreError::PondMossParentMissing)?;
            if stone.preset != PondStonePreset::Main { return Err(CoreError::InvalidPondMossSite); }
            Ok(MossSupport { x: stone.x + offset * (10.0 / BASIN_WIDTH_PX), height_px: stone.preset.height_px() })
        }
        PondMossParent::Wood { id } => {
            let wood = woods.and_then(|layout| layout.woods.iter().find(|wood| wood.id == id))
                .ok_or(CoreError::PondMossParentMissing)?;
            let (x, height_px) = wood.moss_site(offset).ok_or(CoreError::InvalidPondMossSite)?;
            Ok(MossSupport { x, height_px })
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PondMossLayout {
    next_id: u32,
    pub(super) patches: Vec<PondMossPatch>,
}

impl PondMossLayout {
    pub(super) fn new() -> Self { Self { next_id: 1, patches: Vec::new() } }

    pub(super) fn preserve_allocations(&mut self, current: &Self) {
        self.next_id = self.next_id.max(current.next_id);
    }

    pub(super) fn reset_stocks(&mut self) {
        for patch in &mut self.patches {
            patch.stock_mg_cnp = Matter(patch.preset.initial_mg_cnp());
        }
    }

    pub(super) fn valid(&self, stones: Option<&PondStoneLayout>, woods: Option<&PondWoodLayout>) -> bool {
        self.next_id > 0 && self.patches.len() <= MAX_MOSS_PATCHES && self.patches.iter().enumerate().all(|(index, patch)| {
            patch.id > 0 && patch.id < self.next_id && (-1.0..=1.0).contains(&patch.offset)
                && patch.stock_mg_cnp.valid() && patch.stock_mg_cnp.carbon() > 0.0
                && patch.stock_mg_cnp.carbon() <= patch.preset.capacity_mg_c() + 1e-10
                && self.patches[..index].iter().all(|previous| previous.id < patch.id && previous.parent != patch.parent)
                && support(patch.parent, patch.offset, stones, woods).is_ok()
        })
    }

    pub(super) fn place(&mut self, parent: PondMossParent, offset: f64, preset: PondMossPreset) -> Result<Matter, CoreError> {
        check_offset(offset)?;
        if self.patches.iter().any(|patch| patch.parent == parent) { return Err(CoreError::PondMossSiteOccupied); }
        if self.patches.len() >= MAX_MOSS_PATCHES { return Err(CoreError::PondMossLimit); }
        let next = self.next_id.checked_add(1).ok_or(CoreError::PondMossLimit)?;
        let stock = Matter(preset.initial_mg_cnp());
        self.patches.push(PondMossPatch { id: self.next_id, parent, offset, preset, stock_mg_cnp: stock });
        self.next_id = next;
        Ok(stock)
    }

    pub(super) fn move_patch(&mut self, id: u32, offset: f64) -> Result<(), CoreError> {
        check_offset(offset)?;
        self.patch_mut(id)?.offset = offset;
        Ok(())
    }

    pub(super) fn trim(&mut self, id: u32, mg_c: f64) -> Result<Matter, CoreError> {
        let patch = self.patch_mut(id)?;
        let fraction = mg_c / patch.stock_mg_cnp.carbon();
        if !(fraction > 0.0 && fraction < 1.0) { return Err(CoreError::InvalidPondMossTrim); }
        Ok(patch.stock_mg_cnp.take_fraction(fraction))
    }

    pub(super) fn remove(&mut self, id: u32) -> Result<Matter, CoreError> {
        let index = self.patches.iter().position(|patch| patch.id == id).ok_or(CoreError::PondMossMissing)?;
        Ok(self.patches.remove(index).stock_mg_cnp)
    }

    pub(super) fn remove_with_parent(&mut self, parent: PondMossParent) -> Option<Matter> {
        let index = self.patches.iter().position(|patch| patch.parent == parent)?;
        Some(self.patches.remove(index).stock_mg_cnp)
    }

    fn patch_mut(&mut self, id: u32) -> Result<&mut PondMossPatch, CoreError> {
        self.patches.iter_mut().find(|patch| patch.id == id).ok_or(CoreError::PondMossMissing)
    }
}

fn check_offset(offset: f64) -> Result<(), CoreError> {
    if !(-1.0..=1.0).contains(&offset) { return Err(CoreError::InvalidPondMossSite); }
    Ok(())
}

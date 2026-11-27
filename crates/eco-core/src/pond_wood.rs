use crate::pond_stones::{slope_profile, PondBedProfile, PondSwimRange, StructurePreset, BASIN_WIDTH_PX};
use crate::pond::Matter;
use crate::{Activity, Animal, CoreError, Point};
use serde::{Deserialize, Serialize};
use std::f64::consts::FRAC_PI_2;

pub(super) const MAX_WOODS: usize = 2;
// Match the stone-film engineering default until material-specific rates are calibrated.
pub(super) const WOOD_BIOFILM_CAPACITY_MG_C: f64 = 12.0;
const REST_TICKS: u16 = 35;
const APPROACH_PX: f64 = 1.5;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PondWoodPreset {
    Branch,
    Root,
    Slope,
    HangingRoots,
    RootNetwork,
}

impl PondWoodPreset {
    pub(super) fn requires_composed_scene(self) -> bool {
        matches!(self, Self::Slope | Self::HangingRoots | Self::RootNetwork)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PondWood {
    pub id: u32,
    pub x: f64,
    pub preset: PondWoodPreset,
}

impl PondWood {
    fn valid_anchor(self) -> bool {
        match self.preset {
            PondWoodPreset::Branch => (0.37..=0.63).contains(&self.x),
            PondWoodPreset::Root => (0.25..=0.75).contains(&self.x),
            PondWoodPreset::Slope => {
                let [left, right] = slope_profile().anchor_range;
                (left..=right).contains(&self.x)
            }
            PondWoodPreset::HangingRoots => StructurePreset::HangingRoots.profile().accepts_anchor(self.x),
            PondWoodPreset::RootNetwork => StructurePreset::RootNetwork.profile().accepts_anchor(self.x),
        }
    }

    pub(super) fn profile(self) -> PondBedProfile {
        // Both lengths have the same fixed directional projection at the 600 mm reference plane.
        match self.preset {
            PondWoodPreset::Branch => PondBedProfile::flat(self.x, 96.0, 22.0),
            PondWoodPreset::Root => PondBedProfile::flat(self.x, 48.0, 15.0),
            PondWoodPreset::Slope => PondBedProfile::slope(self.x),
            PondWoodPreset::HangingRoots => PondBedProfile::structure(self.x, StructurePreset::HangingRoots),
            PondWoodPreset::RootNetwork => PondBedProfile::structure(self.x, StructurePreset::RootNetwork),
        }
    }

    fn support_span(self) -> Option<(f64, f64)> {
        // Center-only pads match the authored wet support and leave room for the shrimp's body.
        let (center, span) = match self.preset {
            PondWoodPreset::Branch => (0.0, 58.0),
            PondWoodPreset::Root => (0.0, 25.0),
            PondWoodPreset::Slope => (slope_profile().support_center, slope_profile().support_half_span),
            PondWoodPreset::HangingRoots | PondWoodPreset::RootNetwork => return None,
        };
        Some((self.x + center / BASIN_WIDTH_PX, span / BASIN_WIDTH_PX))
    }

    pub(super) fn moss_site(self, offset: f64) -> Option<(f64, f64)> {
        let (center, half_width) = self.support_span()?;
        let x = center + offset * half_width;
        let height = match self.preset {
            PondWoodPreset::Branch => 22.0,
            PondWoodPreset::Root => 15.0,
            PondWoodPreset::Slope => {
                let profile = slope_profile();
                PondBedProfile::slope_ridge_height(profile.support_center + offset * profile.support_half_span)
            }
            PondWoodPreset::HangingRoots | PondWoodPreset::RootNetwork => return None,
        };
        Some((x, height))
    }

    pub(super) fn surface(self, offset: f64, column_px: f64) -> Option<Point> {
        if !(-1.0..=1.0).contains(&offset) { return None; }
        let (x, height) = self.moss_site(offset)?;
        let point = Point { x, y: 1.0 - height / column_px - 0.25 / column_px };
        PondSwimRange::EXTENDED.contains(point).then_some(point)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PondShrimpAttachment {
    pub shrimp_id: String,
    pub wood_id: u32,
    pub offset: f64,
    pub rest_ticks: u16,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct WoodBiofilm {
    pub(super) wood_id: u32,
    pub(super) stock: Matter,
}

impl WoodBiofilm {
    fn empty(wood_id: u32) -> Self { Self { wood_id, stock: Matter::default() } }

    pub(super) fn vacant_mg_c(&self) -> f64 { (WOOD_BIOFILM_CAPACITY_MG_C - self.stock.carbon()).max(0.0) }

    pub(super) fn growth_demand(&self, wet: f64, opportunity: f64) -> Matter {
        let space = self.vacant_mg_c();
        let carbon = self.stock.carbon() * 0.037 * opportunity * wet * (space / WOOD_BIOFILM_CAPACITY_MG_C);
        Matter::with_ratios(carbon.min(space), 0.077, 0.010)
    }

    pub(super) fn respiration_demand(&self, wet: f64, elapsed: f64) -> f64 {
        self.stock.carbon() * 0.004 * elapsed * wet
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PondWoodLayout {
    next_id: u32,
    pub(super) woods: Vec<PondWood>,
    pub(super) attachments: Vec<PondShrimpAttachment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) wood_biofilm: Option<Vec<WoodBiofilm>>,
}

impl PondWoodLayout {
    pub(super) fn new() -> Self {
        Self { next_id: 1, woods: Vec::new(), attachments: Vec::new(), wood_biofilm: None }
    }

    pub(super) fn preserve_allocations(&mut self, current: &Self) {
        self.next_id = self.next_id.max(current.next_id);
    }

    /// Installs the authored slope before a new preset is populated and validated.
    pub(super) fn seed_slope(&mut self, x: f64) -> u32 {
        let id = self.next_id;
        let wood = PondWood { id, x, preset: PondWoodPreset::Slope };
        debug_assert!(wood.valid_anchor() && self.woods.len() < MAX_WOODS);
        self.next_id = id.checked_add(1).expect("new slope layout has an available ID");
        self.woods.push(wood);
        if let Some(stocks) = &mut self.wood_biofilm { stocks.push(WoodBiofilm::empty(id)); }
        id
    }

    pub(super) fn profiles(&self) -> impl Iterator<Item = PondBedProfile> + Clone + '_ {
        self.woods.iter().map(|wood| wood.profile())
    }

    pub(super) fn attachment_point(&self, attachment: &PondShrimpAttachment, column_px: f64) -> Option<Point> {
        self.woods.iter().find(|wood| wood.id == attachment.wood_id)?.surface(attachment.offset, column_px)
    }

    pub(super) fn attachment_matches(&self, attachment: &PondShrimpAttachment, animal: &Animal, column_px: f64) -> bool {
        animal.id == attachment.shrimp_id && animal.activity == Activity::Resting
            && self.attachment_point(attachment, column_px).is_some_and(|surface| {
                (animal.x - surface.x).abs() <= 1e-10 && (animal.y - surface.y).abs() <= 1e-10
            })
    }

    pub(super) fn valid(&self) -> bool {
        self.next_id > 0 && self.woods.len() <= MAX_WOODS && self.woods.iter().enumerate().all(|(index, wood)| {
            wood.id > 0 && wood.id < self.next_id && wood.valid_anchor()
                && !self.woods[..index].iter().any(|previous| previous.id >= wood.id || wood.profile().overlaps(previous.profile()))
        }) && self.wood_biofilm.as_ref().is_none_or(|stocks| {
            self.woods.iter().map(|wood| wood.id).eq(stocks.iter().map(|patch| patch.wood_id))
                && stocks.iter().all(|patch| patch.stock.valid() && patch.stock.carbon() <= WOOD_BIOFILM_CAPACITY_MG_C + 1e-9)
        }) && self.attachments.iter().enumerate().all(|(index, attachment)| {
            (1..=REST_TICKS).contains(&attachment.rest_ticks) && (-1.0..=1.0).contains(&attachment.offset)
                && !self.attachments[..index].iter().any(|other| other.shrimp_id == attachment.shrimp_id)
        })
    }

    pub(super) fn place(
        &mut self, x: f64, preset: PondWoodPreset, column_px: f64, animals: impl Iterator<Item = Point>,
        stone_profiles: impl Iterator<Item = PondBedProfile>,
    ) -> Result<(), CoreError> {
        if self.woods.len() >= MAX_WOODS { return Err(CoreError::PondWoodLimit); }
        let next_id = self.next_id.checked_add(1).ok_or(CoreError::PondWoodLimit)?;
        let wood = PondWood { id: self.next_id, x, preset };
        self.validate_site(wood, column_px, animals, stone_profiles)?;
        self.woods.push(wood);
        if let Some(stocks) = &mut self.wood_biofilm { stocks.push(WoodBiofilm::empty(wood.id)); }
        self.next_id = next_id;
        Ok(())
    }

    pub(super) fn move_wood(
        &mut self, id: u32, x: f64, column_px: f64, animals: impl Iterator<Item = Point>,
        stone_profiles: impl Iterator<Item = PondBedProfile>,
    ) -> Result<(), CoreError> {
        let index = self.woods.iter().position(|wood| wood.id == id).ok_or(CoreError::PondWoodMissing)?;
        let wood = PondWood { x, ..self.woods[index] };
        self.validate_site(wood, column_px, animals, stone_profiles)?;
        if self.attachments.iter().filter(|attachment| attachment.wood_id == id)
            .any(|attachment| wood.surface(attachment.offset, column_px).is_none())
        {
            return Err(CoreError::PondAttachedShrimpBlocked);
        }
        self.woods[index] = wood;
        Ok(())
    }

    pub(super) fn remove(&mut self, id: u32) -> Result<Matter, CoreError> {
        let index = self.woods.iter().position(|wood| wood.id == id).ok_or(CoreError::PondWoodMissing)?;
        self.woods.remove(index);
        self.attachments.retain(|attachment| attachment.wood_id != id);
        Ok(self.wood_biofilm.as_mut().map_or_else(Matter::default, |stocks| stocks.remove(index).stock))
    }

    fn validate_site(
        &self, wood: PondWood, column_px: f64, mut animals: impl Iterator<Item = Point>,
        stone_profiles: impl Iterator<Item = PondBedProfile>,
    ) -> Result<(), CoreError> {
        if !wood.valid_anchor() { return Err(CoreError::InvalidPondWoodSite); }
        let profile = wood.profile();
        if self.woods.iter().filter(|other| other.id != wood.id).map(|other| other.profile())
            .chain(stone_profiles).any(|other| profile.overlaps(other))
        {
            return Err(CoreError::OverlappingPondWoodSite);
        }
        if animals.any(|animal| profile.blocks(animal, column_px)) { return Err(CoreError::OccupiedPondWoodSite); }
        Ok(())
    }
}

pub(super) fn hold_or_release(attachments: &mut Vec<PondShrimpAttachment>, animal: &mut Animal, oxygen: f64) -> bool {
    let Some(index) = attachments.iter().position(|attachment| attachment.shrimp_id == animal.id) else { return false; };
    if attachments[index].rest_ticks <= 1 || oxygen < 0.30 {
        attachments.remove(index);
        animal.heading = 3.0 * FRAC_PI_2;
        return false;
    }
    attachments[index].rest_ticks -= 1;
    animal.activity = Activity::Resting;
    true
}

/// A downward-facing shrimp that rests close to an immersed surface can settle on it.
pub(super) fn settle(
    woods: &[PondWood], attachments: &mut Vec<PondShrimpAttachment>, animal: &mut Animal, column_px: f64,
) {
    if animal.heading.sin() <= 0.0 { return; }
    for wood in woods {
        let Some((center, half_width)) = wood.support_span() else { continue; };
        let offset = (animal.x - center) / half_width;
        if !(-1.0..=1.0).contains(&offset) { continue; }
        let Some(surface) = wood.surface(offset, column_px) else { continue; };
        let gap_px = (surface.y - animal.y) * column_px;
        if (0.0..=APPROACH_PX).contains(&gap_px) {
            animal.x = surface.x;
            animal.y = surface.y;
            animal.activity = Activity::Resting;
            attachments.push(PondShrimpAttachment {
                shrimp_id: animal.id.clone(), wood_id: wood.id, offset, rest_ticks: REST_TICKS,
            });
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slope_landing_height_depends_on_which_part_is_submerged() {
        let mut layout = PondWoodLayout::new();
        let id = layout.seed_slope(0.235);
        let wood = layout.woods[0];
        let (low_x, graze_height) = wood.moss_site(-0.75).unwrap();
        let (high_x, crown_height) = wood.moss_site(0.75).unwrap();
        assert!(low_x < high_x && graze_height < crown_height);
        assert!(wood.surface(-0.75, 65.0).is_some());
        assert!(wood.surface(0.75, 65.0).is_none());

        let offset = 0.55;
        let column = 130.0;
        let target = wood.surface(offset, column).unwrap();
        let mut animal = Animal {
            id: "slope-swimmer".to_owned(), x: target.x, y: target.y - 0.5 / column,
            heading: FRAC_PI_2, activity: Activity::Moving,
        };
        settle(&layout.woods, &mut layout.attachments, &mut animal, column);
        assert_eq!(animal.activity, Activity::Resting);
        let attachment = &layout.attachments[0];
        assert_eq!(attachment.wood_id, id);
        assert!((attachment.offset - offset).abs() < 1e-12);
        assert!(!wood.profile().blocks(Point { x: animal.x, y: animal.y }, column));
        assert!(layout.attachment_point(attachment, 65.0).is_none());
    }
}

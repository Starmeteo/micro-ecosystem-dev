use crate::pond::Matter;
use crate::pond_shape::{PondShape, PondShapeError, COLUMNS, ROWS};
use crate::{CoreError, Point};
use serde::{Deserialize, Serialize};

pub(super) const MAX_FLOATING_PLANTS: usize = 8;
const GROWTH_PER_HOUR: f64 = 0.028;
const RESPIRATION_PER_HOUR: f64 = 0.004;
const STRIP_HEIGHT_CM: f64 = 0.125;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PondFloatingPlantPreset { Small, Medium, Large }

impl PondFloatingPlantPreset {
    fn diameter_mm(self) -> f64 {
        match self { Self::Small => 30.0, Self::Medium => 45.0, Self::Large => 60.0 }
    }

    fn dose(self) -> Matter {
        let carbon = match self { Self::Small => 12.0, Self::Medium => 20.0, Self::Large => 32.0 };
        Matter::with_ratios(carbon, 0.06, 0.007)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PondFloatingPlantObservation {
    pub id: u32,
    pub position: Point,
    pub preset: PondFloatingPlantPreset,
    pub initial_mg_cnp: [f64; 3],
    pub stock_mg_cnp: [f64; 3],
    pub diameter_mm: f64,
    pub floating: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PondFloatingCanopy {
    pub covered_area_cm2: f64,
    pub coverage: f64,
    pub light_transmission: f64,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct FloatingPlant {
    id: u32,
    position: Point,
    preset: PondFloatingPlantPreset,
    pub(super) stock_mg_cnp: Matter,
}

impl FloatingPlant {
    fn diameter_mm(self) -> f64 {
        let relative_biomass = self.stock_mg_cnp.carbon() / self.preset.dose().carbon();
        self.preset.diameter_mm() * relative_biomass.sqrt().min(1.25)
    }

    pub(super) fn growth_demand(self, opportunity: f64) -> Matter {
        let carbon = self.stock_mg_cnp.carbon();
        let capacity = 3.0 * self.preset.dose().carbon();
        let growth = carbon * GROWTH_PER_HOUR * (1.0 - carbon / capacity).max(0.0) * opportunity;
        Matter::with_ratios(growth, 0.06, 0.007)
    }

    pub(super) fn respiration_demand(self, elapsed_hours: f64) -> f64 {
        self.stock_mg_cnp.carbon() * RESPIRATION_PER_HOUR * elapsed_hours
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct FloatingPlantLayout {
    next_id: u32,
    pub(super) patches: Vec<FloatingPlant>,
}

fn anchor_cell(position: Point) -> Option<u16> {
    if !(0.0..1.0).contains(&position.x) || !(0.0..1.0).contains(&position.y) {
        return None;
    }
    let x = (position.x * COLUMNS as f64) as usize;
    let y = (position.y * ROWS as f64) as usize;
    Some((y * COLUMNS + x) as u16)
}

fn center_in_cells(position: Point, cells: &[u16]) -> bool {
    anchor_cell(position).is_some_and(|cell| cells.contains(&cell))
}

pub(super) fn check_surface(position: Point, shape: &PondShape) -> Result<(), CoreError> {
    if center_in_cells(position, shape.cells()) { Ok(()) } else { Err(CoreError::InvalidPondFloatingPlantSite) }
}

impl FloatingPlantLayout {
    pub(super) fn new() -> Self { Self { next_id: 1, patches: Vec::new() } }

    pub(super) fn has_patches(&self) -> bool { !self.patches.is_empty() }

    pub(super) fn anchor_cells_in(&self, cells: &[u16]) -> bool {
        self.patches.iter().all(|patch| center_in_cells(patch.position, cells))
    }

    pub(super) fn retain_in(&mut self, shape: &PondShape) {
        self.patches.retain(|patch| center_in_cells(patch.position, shape.cells()));
    }

    pub(super) fn merge_from(&mut self, source: &Self) -> Result<(), PondShapeError> {
        if self.patches.len() + source.patches.len() > MAX_FLOATING_PLANTS {
            return Err(PondShapeError::TopologyPopulation);
        }
        let next = self.next_id.checked_add(source.patches.len() as u32).ok_or(PondShapeError::TopologyPopulation)?;
        self.patches.extend((self.next_id..next).zip(&source.patches).map(|(id, patch)| FloatingPlant { id, ..*patch }));
        self.next_id = next;
        Ok(())
    }

    pub(super) fn preserve_allocations(&mut self, current: &Self) { self.next_id = self.next_id.max(current.next_id); }

    pub(super) fn reset_stocks(&mut self) {
        for patch in &mut self.patches { patch.stock_mg_cnp = patch.preset.dose(); }
    }

    pub(super) fn place(&mut self, position: Point, preset: PondFloatingPlantPreset) -> Result<Matter, CoreError> {
        if self.patches.len() >= MAX_FLOATING_PLANTS { return Err(CoreError::PondFloatingPlantLimit); }
        let next = self.next_id.checked_add(1).ok_or(CoreError::PondFloatingPlantLimit)?;
        let stock = preset.dose();
        self.patches.push(FloatingPlant { id: self.next_id, position, preset, stock_mg_cnp: stock });
        self.next_id = next;
        Ok(stock)
    }

    pub(super) fn move_patch(&mut self, id: u32, position: Point) -> Result<(), CoreError> {
        let index = self.index(id)?;
        self.patches[index].position = position;
        Ok(())
    }

    pub(super) fn remove(&mut self, id: u32) -> Result<Matter, CoreError> {
        let index = self.index(id)?;
        Ok(self.patches.remove(index).stock_mg_cnp)
    }

    fn index(&self, id: u32) -> Result<usize, CoreError> {
        self.patches.iter().position(|patch| patch.id == id).ok_or(CoreError::PondFloatingPlantMissing)
    }

    pub(super) fn observe(&self, wet: bool) -> Vec<PondFloatingPlantObservation> {
        self.patches.iter().map(|patch| PondFloatingPlantObservation {
            id: patch.id, position: patch.position, preset: patch.preset,
            initial_mg_cnp: patch.preset.dose().0, stock_mg_cnp: patch.stock_mg_cnp.0,
            diameter_mm: patch.diameter_mm(), floating: wet && patch.stock_mg_cnp.carbon() > 0.0,
        }).collect()
    }

    pub(super) fn valid(&self, shape: Option<&PondShape>) -> bool {
        self.next_id > 0 && self.patches.len() <= MAX_FLOATING_PLANTS
            && self.patches.iter().enumerate().all(|(index, patch)| {
                patch.id > 0 && patch.id < self.next_id && patch.stock_mg_cnp.valid()
                    && (index == 0 || self.patches[index - 1].id < patch.id)
                    && shape.is_some_and(|shape| check_surface(patch.position, shape).is_ok())
            })
    }

    pub(super) fn canopy(&self, shape: &PondShape, wet: bool) -> PondFloatingCanopy {
        let covered_area_cm2 = if wet {
            let side = shape.cell_area().sqrt();
            let disks = self.patches.iter().map(|patch| Disk {
                x: patch.position.x * COLUMNS as f64 * side,
                y: patch.position.y * ROWS as f64 * side,
                radius: patch.diameter_mm() / 20.0,
            }).collect::<Vec<_>>();
            shape.cells().iter().map(|&cell| area_in_cell(usize::from(cell), side, &disks))
                .sum::<f64>().min(shape.area())
        } else { 0.0 };
        let coverage = covered_area_cm2 / shape.area();
        PondFloatingCanopy { covered_area_cm2, coverage, light_transmission: 1.0 - coverage }
    }
}

#[derive(Clone, Copy)]
struct Disk { x: f64, y: f64, radius: f64 }

impl Disk {
    fn span(self, y: f64, left: f64, right: f64) -> Option<(f64, f64)> {
        let reach_squared = self.radius * self.radius - (self.y - y).powi(2);
        if reach_squared <= 0.0 { return None; }
        let reach = reach_squared.sqrt();
        let start = (self.x - reach).max(left);
        let end = (self.x + reach).min(right);
        (end > start).then_some((start, end))
    }
}

// Midpoint integration over short vertical strips. Each horizontal interval is clipped
// to its water cell and merged with all other leaf intervals before adding width.
fn area_in_cell(cell: usize, side: f64, disks: &[Disk]) -> f64 {
    let left = (cell % COLUMNS) as f64 * side;
    let right = left + side;
    let top = (cell / COLUMNS) as f64 * side;
    let bottom = top + side;
    let mut nearby = [Disk { x: 0.0, y: 0.0, radius: 0.0 }; MAX_FLOATING_PLANTS];
    let mut count = 0;
    let (mut start, mut end) = (bottom, top);
    for &disk in disks {
        if disk.radius <= 0.0 || disk.x + disk.radius <= left || disk.x - disk.radius >= right
            || disk.y + disk.radius <= top || disk.y - disk.radius >= bottom { continue; }
        nearby[count] = disk;
        count += 1;
        start = start.min((disk.y - disk.radius).max(top));
        end = end.max((disk.y + disk.radius).min(bottom));
    }
    if end <= start { return 0.0; }

    let strips = ((end - start) / STRIP_HEIGHT_CM).ceil() as usize;
    let height = (end - start) / strips as f64;
    let mut widths = 0.0;
    for strip in 0..strips {
        let y = start + (strip as f64 + 0.5) * height;
        let mut spans = [(0.0, 0.0); MAX_FLOATING_PLANTS];
        let mut count_spans = 0;
        for disk in &nearby[..count] {
            if let Some(span) = disk.span(y, left, right) {
                spans[count_spans] = span;
                count_spans += 1;
            }
        }
        let spans = &mut spans[..count_spans];
        spans.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
        let mut frontier = left;
        for &(start, end) in spans.iter() {
            widths += (end - start.max(frontier)).max(0.0);
            frontier = frontier.max(end);
        }
    }
    widths * height
}

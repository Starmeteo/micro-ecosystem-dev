use crate::{Activity, Animal, CoreError, Point};
use serde::{Deserialize, Serialize};
use std::f64::consts::{FRAC_PI_2, PI, TAU};
use std::sync::OnceLock;

pub(super) const MAX_STONES: usize = 4;
// Fixed 480 × 270 projection. The basin-local swim range fits the narrowest bed.
pub(super) const BASIN_WIDTH_PX: f64 = 288.0;
const MAX_WATER_COLUMN_PX: f64 = 208.0;
pub(super) const SHRIMP_HALF_LENGTH_PX: f64 = 8.0;
const SWIM_X_MIN: f64 = 0.06;
const SWIM_X_MAX: f64 = 0.94;
const SWIM_TOP: f64 = 0.20;
const SWIM_BOTTOM: f64 = 0.92;
const WATER_GAP_PX: f64 = 4.0;
const CLEARANCE_PX: f64 = 0.25;
const STONE_REST_TICKS: u16 = 35;
const STONE_APPROACH_PX: f64 = 1.5;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PondStonePreset {
    Main,
    Pebble,
    FlatStoneCave,
}

impl PondStonePreset {
    fn half_width_px(self) -> f64 {
        match self {
            Self::Main => 32.0,
            Self::Pebble => 10.0,
            Self::FlatStoneCave => 26.0,
        }
    }

    pub(super) fn height_px(self) -> f64 {
        match self {
            Self::Main => 27.0,
            Self::Pebble => 9.0,
            Self::FlatStoneCave => 13.0,
        }
    }

    fn valid_anchor(self, x: f64) -> bool {
        match self {
            Self::Main => (0.26..=0.74).contains(&x),
            Self::Pebble => (0.19..=0.81).contains(&x),
            Self::FlatStoneCave => StructurePreset::FlatStoneCave.profile().accepts_anchor(x),
        }
    }

}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PondStone {
    pub id: u32,
    /// Horizontal bed anchor in basin coordinates. The projection fixes its vertical position.
    pub x: f64,
    pub preset: PondStonePreset,
}

impl PondStone {
    pub(super) fn profile(self) -> PondBedProfile {
        match self.preset {
            PondStonePreset::FlatStoneCave => PondBedProfile::structure(self.x, StructurePreset::FlatStoneCave),
            _ => PondBedProfile::flat(self.x, self.preset.half_width_px(), self.preset.height_px()),
        }
    }

    fn support_half_width(self) -> f64 {
        (self.preset.half_width_px() - SHRIMP_HALF_LENGTH_PX) / BASIN_WIDTH_PX
    }

    fn rest_surface(self, offset: f64, column_px: f64) -> Option<Point> {
        if self.preset != PondStonePreset::Main || !(-1.0..=1.0).contains(&offset) || column_px <= 0.0 {
            return None;
        }
        let point = Point {
            x: self.x + offset * self.support_half_width(),
            y: 1.0 - (self.preset.height_px() + CLEARANCE_PX) / column_px,
        };
        PondSwimRange::EXTENDED.contains(point).then_some(point)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PondShrimpStoneAttachment {
    pub shrimp_id: String,
    pub stone_id: u32,
    pub offset: f64,
    pub rest_ticks: u16,
}

impl PondShrimpStoneAttachment {
    pub(super) fn point(&self, stones: &[PondStone], column_px: f64) -> Option<Point> {
        stones.iter().find(|stone| stone.id == self.stone_id)?.rest_surface(self.offset, column_px)
    }

    pub(super) fn matches(&self, stones: &[PondStone], animal: &Animal, column_px: f64) -> bool {
        self.shrimp_id == animal.id && animal.activity == Activity::Resting
            && self.point(stones, column_px).is_some_and(|surface| {
                (animal.x - surface.x).abs() <= 1e-10 && (animal.y - surface.y).abs() <= 1e-10
            })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct SlopeProfile {
    // Local x, lower height and upper height in the 480 × 270 logical view.
    ridge: Vec<[f64; 3]>,
    foot: [[f64; 3]; 2],
    pub(super) anchor_range: [f64; 2],
    pub(super) support_center: f64,
    pub(super) support_half_span: f64,
}

pub(super) fn slope_profile() -> &'static SlopeProfile {
    const SOURCE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"),
        "/../../apps/desktop/src/render/assets/r5/pond-composed/slope-profile.json"));
    static VALUE: OnceLock<SlopeProfile> = OnceLock::new();
    VALUE.get_or_init(|| {
        let profile: SlopeProfile = serde_json::from_str(SOURCE).expect("valid bundled slope profile JSON");
        let [left, right] = profile.anchor_range;
        assert!(profile.ridge.len() >= 2 && profile.ridge.windows(2).all(|pair| pair[0][0] < pair[1][0])
            && profile.foot[0][0] < profile.foot[1][0]
            && profile.ridge.iter().chain(&profile.foot).all(|[_, lower, upper]| *lower >= 0.0 && *upper >= *lower)
            && (0.0..=1.0).contains(&left) && left < right && right <= 1.0
            && profile.support_half_span > 0.0
            && profile.support_center - profile.support_half_span >= profile.ridge[0][0]
            && profile.support_center + profile.support_half_span <= profile.ridge.last().expect("nonempty ridge")[0],
            "valid bundled slope sections, placement range and support span");
        profile
    })
}

#[derive(Clone, Copy)]
pub(super) enum StructurePreset { HangingRoots, RootNetwork, FlatStoneCave }

type BandVertices = [[f64; 3]; 2];

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct StructureProfile {
    anchor_range: [f64; 2],
    bands: Vec<BandVertices>,
    swim_clearance: [f64; 2],
}

impl StructureProfile {
    pub(super) fn accepts_anchor(&self, x: f64) -> bool {
        let [left, right] = self.anchor_range;
        (left..=right).contains(&x)
    }

    fn valid(&self) -> bool {
        let [left, right] = self.anchor_range;
        (0.0..1.0).contains(&left) && (left..=1.0).contains(&right)
            && self.swim_clearance.iter().all(|&clearance| (0.0..=2.0).contains(&clearance))
            && !self.bands.is_empty() && self.bands.iter().all(|[a, b]| {
                a[0] < b[0] && left * BASIN_WIDTH_PX + a[0] > 0.0
                    && right * BASIN_WIDTH_PX + b[0] < BASIN_WIDTH_PX
                    && [a, b].into_iter().all(|v| v[1] >= 0.0 && v[2] > v[1])
            })
    }
}

impl StructurePreset {
    pub(super) fn profile(self) -> &'static StructureProfile {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct Profiles {
            hanging_roots: StructureProfile,
            root_network: StructureProfile,
            flat_stone_cave: StructureProfile,
        }
        const SOURCE: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"),
            "/../../apps/desktop/src/render/assets/r5/pond-composed/structure-profiles.json"));
        static PROFILES: OnceLock<Profiles> = OnceLock::new();
        let profiles = PROFILES.get_or_init(|| {
            let profiles: Profiles = serde_json::from_str(SOURCE).expect("valid bundled structure profiles JSON");
            assert!([&profiles.hanging_roots, &profiles.root_network, &profiles.flat_stone_cave]
                .into_iter().all(StructureProfile::valid), "valid R5 structure solidity bands and placement ranges");
            profiles
        });
        match self {
            Self::HangingRoots => &profiles.hanging_roots,
            Self::RootNetwork => &profiles.root_network,
            Self::FlatStoneCave => &profiles.flat_stone_cave,
        }
    }
}

#[derive(Clone, Copy)]
enum BedShape {
    Flat { half_width_px: f64, height_px: f64 },
    Slope,
    Structure(StructurePreset),
}

#[derive(Clone, Copy)]
pub(super) struct PondBedProfile {
    x: f64,
    shape: BedShape,
}

impl PondBedProfile {
    pub(super) fn flat(x: f64, half_width_px: f64, height_px: f64) -> Self {
        Self { x, shape: BedShape::Flat { half_width_px, height_px } }
    }

    pub(super) fn slope(x: f64) -> Self { Self { x, shape: BedShape::Slope } }

    pub(super) fn structure(x: f64, preset: StructurePreset) -> Self {
        Self { x, shape: BedShape::Structure(preset) }
    }

    pub(super) fn slope_ridge_height(dx: f64) -> f64 {
        let band = slope_profile().ridge.windows(2).find(|pair| (pair[0][0]..=pair[1][0]).contains(&dx))
            .expect("supported attachment offsets are on the ridge");
        ProfileBand::new(0.0, band[0], band[1]).height_at(dx, 2)
    }

    pub(super) fn overlaps(self, other: Self) -> bool {
        if let (BedShape::Flat { half_width_px: a, .. }, BedShape::Flat { half_width_px: b, .. }) = (self.shape, other.shape) {
            let separation_px = (self.x - other.x).abs() * BASIN_WIDTH_PX;
            return separation_px < a + b + 2.0 * SHRIMP_HALF_LENGTH_PX + WATER_GAP_PX;
        }
        self.bands().any(|a| other.bands().any(|b| a.overlaps(b)))
    }

    fn flat_bounds(self, half_width_px: f64, height_px: f64, column_px: f64) -> (f64, f64, f64) {
        let half = (half_width_px + SHRIMP_HALF_LENGTH_PX) / BASIN_WIDTH_PX;
        let top = 1.0 - height_px / column_px;
        (self.x - half, self.x + half, top)
    }

    pub(super) fn blocks(self, point: Point, column_px: f64) -> bool {
        match self.shape {
            BedShape::Flat { half_width_px, height_px } => {
                let (left, right, top) = self.flat_bounds(half_width_px, height_px, column_px);
                (left..=right).contains(&point.x) && point.y >= top
            }
            BedShape::Slope | BedShape::Structure(_) => {
                let point = bed_point(point, column_px);
                self.swim_bands().any(|piece| piece.contains(point))
            }
        }
    }

    pub(super) fn blocks_root(self, x: f64) -> bool {
        self.bands().any(|band| band.contains(Point { x: x * BASIN_WIDTH_PX, y: 0.0 }))
    }

    pub(super) fn wetted_weight(self, column_px: f64) -> f64 {
        self.bands().map(|band| band.wetted_weight(column_px)).sum()
    }

    pub(super) fn blocks_segment(self, from: Point, to: Point, column_px: f64) -> bool {
        let BedShape::Flat { half_width_px, height_px } = self.shape else {
            let from = bed_point(from, column_px);
            let to = bed_point(to, column_px);
            return self.swim_bands().any(|piece| piece.blocks_segment(from, to));
        };
        let (left, right, top) = self.flat_bounds(half_width_px, height_px, column_px);
        let dx = to.x - from.x;
        if dx == 0.0 {
            return (left..=right).contains(&from.x) && from.y.max(to.y) >= top;
        }
        let enter = (left - from.x) / dx;
        let leave = (right - from.x) / dx;
        let first = enter.min(leave).max(0.0);
        let last = enter.max(leave).min(1.0);
        let dy = to.y - from.y;
        first <= last && (from.y + dy * first).max(from.y + dy * last) >= top
    }

    /// A depth change can put the old fractional position inside a solid profile.
    pub(super) fn clear_after_depth_change(self, point: Point, column_px: f64, can_swim: impl Fn(Point) -> bool) -> Point {
        if let BedShape::Flat { half_width_px, height_px } = self.shape {
            let (left, right, top) = self.flat_bounds(half_width_px, height_px, column_px);
            let above = top - CLEARANCE_PX / column_px;
            let candidate = if above >= SWIM_TOP {
                Point { y: above, ..point }
            } else {
                let side = CLEARANCE_PX / BASIN_WIDTH_PX;
                let x = if point.x <= self.x { left - side } else { right + side };
                Point { x, ..point }
            };
            if can_swim(candidate) { return candidate; }
        } else {
            let local = bed_point(point, column_px);
            for band in self.swim_bands().filter(|piece| piece.contains(local)) {
                for h in [band.height_at(local.x, 2) + CLEARANCE_PX, band.height_at(local.x, 1) - CLEARANCE_PX] {
                    let candidate = Point { y: 1.0 - h / column_px, ..point };
                    if (matches!(self.shape, BedShape::Structure(_)) || PondSwimRange::EXTENDED.contains(candidate))
                        && can_swim(candidate) { return candidate; }
                }
            }
        }

        // Sloped wood and a rock may share x, so test against the entire layout.
        for step in 1..=BASIN_WIDTH_PX as u32 {
            let distance = f64::from(step) / BASIN_WIDTH_PX;
            for x in [point.x - distance, point.x + distance] {
                let candidate = Point { x: x.clamp(0.0, 1.0), ..point };
                if can_swim(candidate) { return candidate; }
            }
        }
        // One of the right-hand water-column margins stays open for legal terrain layouts.
        let reserve = [Point { x: 1.0, y: SWIM_TOP }, Point { x: 1.0, y: PondSwimRange::EXTENDED.max_y }];
        reserve.into_iter().find(|candidate| can_swim(*candidate)).unwrap_or(point)
    }

    fn bands(self) -> impl Iterator<Item = ProfileBand> {
        let x = self.x * BASIN_WIDTH_PX;
        let (slope, ground, structure) = match self.shape {
            BedShape::Flat { half_width_px, height_px } => {
                (None, Some(ProfileBand::new(x, [-half_width_px, 0.0, height_px], [half_width_px, 0.0, height_px])), None)
            }
            BedShape::Slope => {
                let profile = slope_profile();
                (Some(profile.ridge.windows(2)), Some(ProfileBand::new(x, profile.foot[0], profile.foot[1])), None)
            }
            BedShape::Structure(preset) => (None, None, Some(preset.profile().bands.iter())),
        };
        slope.into_iter().flatten()
            .map(move |band| ProfileBand::new(x, band[0], band[1]))
            .chain(ground)
            .chain(structure.into_iter().flatten().map(move |band| ProfileBand::new(x, band[0], band[1])))
    }

    fn swim_bands(self) -> impl Iterator<Item = ProfileBand> {
        let clearance = match self.shape {
            BedShape::Structure(preset) => Some(preset.profile().swim_clearance),
            _ => None,
        };
        self.bands().map(move |band| clearance.map_or(band, |clearance| band.with_clearance(clearance)))
    }
}

fn bed_point(point: Point, column_px: f64) -> Point {
    Point { x: point.x * BASIN_WIDTH_PX, y: (1.0 - point.y) * column_px }
}

/// One trapezoid of a physical profile in the basin's projected water column.
#[derive(Clone, Copy)]
struct ProfileBand {
    left: [f64; 3],
    right: [f64; 3],
}

impl ProfileBand {
    fn new(origin: f64, mut left: [f64; 3], mut right: [f64; 3]) -> Self {
        left[0] += origin;
        right[0] += origin;
        Self { left, right }
    }

    fn with_clearance(self, [horizontal, vertical]: [f64; 2]) -> Self {
        let [lx, low_l, high_l] = self.left;
        let [rx, low_r, high_r] = self.right;
        Self { left: [lx - horizontal, low_l - vertical, high_l + vertical],
            right: [rx + horizontal, low_r - vertical, high_r + vertical] }
    }

    fn height_at(self, x: f64, edge: usize) -> f64 {
        let t = (x - self.left[0]) / (self.right[0] - self.left[0]);
        self.left[edge] + t * (self.right[edge] - self.left[edge])
    }

    /// Integrate projected band thickness within the basin and below the waterline.
    fn wetted_weight(self, column_px: f64) -> f64 {
        let left = self.left[0].max(0.0);
        let right = self.right[0].min(BASIN_WIDTH_PX);
        if left >= right { return 0.0; }
        let mut breaks = [left, right, right, right];
        for edge in [1, 2] {
            let rise = self.right[edge] - self.left[edge];
            if rise != 0.0 {
                let crossing = self.left[0] + (column_px - self.left[edge]) / rise * (self.right[0] - self.left[0]);
                if crossing > left && crossing < right { breaks[edge + 1] = crossing; }
            }
        }
        breaks.sort_by(f64::total_cmp);
        breaks.windows(2).map(|span| {
            let midpoint = (span[0] + span[1]) / 2.0;
            let thickness = column_px.min(self.height_at(midpoint, 2)) - self.height_at(midpoint, 1);
            (span[1] - span[0]) * thickness.max(0.0)
        }).sum()
    }

    fn contains(self, point: Point) -> bool {
        (self.left[0]..=self.right[0]).contains(&point.x)
            && (self.height_at(point.x, 1)..=self.height_at(point.x, 2)).contains(&point.y)
    }

    fn side_distances(self, point: Point) -> [f64; 4] {
        [point.x - self.left[0], self.right[0] - point.x,
            point.y - self.height_at(point.x, 1), self.height_at(point.x, 2) - point.y]
    }

    fn blocks_segment(self, from: Point, to: Point) -> bool {
        if from.x.max(to.x) < self.left[0] || from.x.min(to.x) > self.right[0] { return false; }
        let (mut first, mut last) = (0.0, 1.0);
        self.side_distances(from).into_iter().zip(self.side_distances(to))
            .all(|(start, end)| clip_half_plane(&mut first, &mut last, start, end))
    }

    fn overlaps(self, other: Self) -> bool {
        let half_gap = WATER_GAP_PX / 2.0;
        let left = (self.left[0].max(other.left[0]) - half_gap).max(0.0);
        let right = (self.right[0].min(other.right[0]) + half_gap).min(BASIN_WIDTH_PX);
        if left > right { return false; }
        let (mut first, mut last) = (0.0, 1.0);
        [(self, other), (other, self)].into_iter().all(|(above, below)| {
            let gap = |x| above.height_at(x, 2) + WATER_GAP_PX - below.height_at(x, 1);
            clip_half_plane(&mut first, &mut last, gap(left), gap(right))
        })
    }
}

fn clip_half_plane(first: &mut f64, last: &mut f64, start: f64, end: f64) -> bool {
    if start < 0.0 && end < 0.0 { return false; }
    if start < 0.0 { *first = first.max(start / (start - end)); }
    if end < 0.0 { *last = last.min(start / (start - end)); }
    *first <= *last
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PondStoneLayout {
    next_id: u32,
    pub(super) stones: Vec<PondStone>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub(super) stone_rest_enabled: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) stone_attachments: Vec<PondShrimpStoneAttachment>,
}

fn is_false(value: &bool) -> bool { !value }

impl PondStoneLayout {
    pub(super) fn new(stone_rest_enabled: bool) -> Self {
        Self { next_id: 1, stones: Vec::new(), stone_rest_enabled, stone_attachments: Vec::new() }
    }

    pub(super) fn preserve_allocations(&mut self, current: &Self) {
        self.next_id = self.next_id.max(current.next_id);
    }

    pub(super) fn has_cave(&self) -> bool {
        self.stones.iter().any(|stone| stone.preset == PondStonePreset::FlatStoneCave)
    }

    pub(super) fn valid(&self) -> bool {
        self.next_id > 0 && self.stones.len() <= MAX_STONES && self.stones.iter().enumerate().all(|(index, stone)| {
            stone.id > 0 && stone.id < self.next_id && stone.preset.valid_anchor(stone.x)
                && !self.stones[..index].iter().any(|previous| previous.id >= stone.id || stone.profile().overlaps(previous.profile()))
        }) && (self.stone_rest_enabled || self.stone_attachments.is_empty())
            && self.stone_attachments.iter().enumerate().all(|(index, attachment)| {
                (1..=STONE_REST_TICKS).contains(&attachment.rest_ticks) && (-1.0..=1.0).contains(&attachment.offset)
                    && !self.stone_attachments[..index].iter().any(|other| other.shrimp_id == attachment.shrimp_id)
            })
    }

    pub(super) fn place(
        &mut self, x: f64, preset: PondStonePreset, column_px: f64, animals: impl Iterator<Item = Point>,
        extra_profiles: impl Iterator<Item = PondBedProfile>,
    ) -> Result<(), CoreError> {
        if self.stones.len() >= MAX_STONES { return Err(CoreError::PondStoneLimit); }
        let next_id = self.next_id.checked_add(1).ok_or(CoreError::PondStoneLimit)?;
        let stone = PondStone { id: self.next_id, x, preset };
        self.validate_site(stone, column_px, animals, extra_profiles)?;
        self.stones.push(stone);
        self.next_id = next_id;
        Ok(())
    }

    pub(super) fn move_stone(
        &mut self, id: u32, x: f64, column_px: f64, animals: impl Iterator<Item = Point>,
        extra_profiles: impl Iterator<Item = PondBedProfile>,
    ) -> Result<(), CoreError> {
        let index = self.stones.iter().position(|stone| stone.id == id).ok_or(CoreError::PondStoneMissing)?;
        let stone = PondStone { x, ..self.stones[index] };
        self.validate_site(stone, column_px, animals, extra_profiles)?;
        if stone.x != self.stones[index].x {
            self.stone_attachments.retain(|attachment| attachment.stone_id != id);
        }
        self.stones[index] = stone;
        Ok(())
    }

    pub(super) fn remove(&mut self, id: u32) -> Result<(), CoreError> {
        let index = self.stones.iter().position(|stone| stone.id == id).ok_or(CoreError::PondStoneMissing)?;
        self.stones.remove(index);
        self.stone_attachments.retain(|attachment| attachment.stone_id != id);
        Ok(())
    }

    fn validate_site(
        &self, stone: PondStone, column_px: f64, mut animals: impl Iterator<Item = Point>,
        extra_profiles: impl Iterator<Item = PondBedProfile>,
    ) -> Result<(), CoreError> {
        if !stone.preset.valid_anchor(stone.x) { return Err(CoreError::InvalidPondStoneSite); }
        let profile = stone.profile();
        if self.stones.iter().filter(|other| other.id != stone.id).map(|other| other.profile())
            .chain(extra_profiles).any(|other| profile.overlaps(other))
        {
            return Err(CoreError::OverlappingPondStoneSite);
        }
        if animals.any(|animal| profile.blocks(animal, column_px)) { return Err(CoreError::OccupiedPondStoneSite); }
        Ok(())
    }

    pub(super) fn profiles(&self) -> impl Iterator<Item = PondBedProfile> + Clone + '_ {
        self.stones.iter().map(|stone| stone.profile())
    }
}

pub(super) fn hold_or_release(
    attachments: &mut Vec<PondShrimpStoneAttachment>, animal: &mut Animal, oxygen: f64,
) -> bool {
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

pub(super) fn settle(
    stones: &[PondStone], attachments: &mut Vec<PondShrimpStoneAttachment>, animal: &mut Animal, column_px: f64,
    blocked: impl Fn(Point, Point) -> bool,
) {
    if animal.heading.sin() <= 0.0 { return; }
    let from = animal_point(animal);
    for stone in stones {
        let offset = (animal.x - stone.x) / stone.support_half_width();
        let Some(surface) = stone.rest_surface(offset, column_px) else { continue; };
        let gap_px = (surface.y - animal.y) * column_px;
        if (0.0..=STONE_APPROACH_PX).contains(&gap_px) && !blocked(from, surface) {
            animal.x = surface.x;
            animal.y = surface.y;
            animal.activity = Activity::Resting;
            attachments.push(PondShrimpStoneAttachment {
                shrimp_id: animal.id.clone(), stone_id: stone.id, offset, rest_ticks: STONE_REST_TICKS,
            });
            return;
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct PondSwimRange {
    pub min_x: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl PondSwimRange {
    pub const STANDARD: Self = Self { min_x: SWIM_X_MIN, max_x: SWIM_X_MAX, max_y: SWIM_BOTTOM };
    pub const EXTENDED: Self = Self { min_x: 0.0, max_x: 1.0, max_y: 0.96 };
    pub const MIN_Y: f64 = SWIM_TOP;

    pub(super) fn near_cave_floor(column_px: f64) -> Self {
        let clearance = StructurePreset::FlatStoneCave.profile().swim_clearance[1];
        Self { max_y: Self::EXTENDED.max_y.max(1.0 - clearance / column_px.max(clearance)), ..Self::EXTENDED }
    }

    pub(super) fn contains(self, point: Point) -> bool {
        (self.min_x..=self.max_x).contains(&point.x) && (Self::MIN_Y..=self.max_y).contains(&point.y)
    }

    pub(super) fn clamp(self, point: Point) -> Point {
        Point { x: point.x.clamp(self.min_x, self.max_x), y: point.y.clamp(Self::MIN_Y, self.max_y) }
    }

    pub(super) fn turn_shrimp(self, animal: &mut Animal, speed: f64, blocked: impl Fn(Point, Point) -> bool) {
        let from = animal_point(animal);
        let heading = animal.heading;
        // Try to swim over the obstacle, then around it or back toward free water.
        let turns = [3.0 * FRAC_PI_2, heading + FRAC_PI_2, heading - FRAC_PI_2, heading + PI];
        for turn in turns {
            let to = self.clamp(Point { x: from.x + turn.cos() * speed, y: from.y + turn.sin() * speed * 0.60 });
            if to != from && !blocked(from, to) {
                animal.x = to.x;
                animal.y = to.y;
                animal.heading = turn.rem_euclid(TAU);
                return;
            }
        }
        animal.heading = (heading + PI).rem_euclid(TAU);
    }

}

pub(super) fn column_projection_px(depth_cm: f64, max_depth_cm: f64) -> f64 {
    (MAX_WATER_COLUMN_PX * (depth_cm / max_depth_cm).clamp(0.0, 1.0)).max(1.0)
}

pub(super) fn animal_point(animal: &Animal) -> Point {
    Point { x: animal.x, y: animal.y }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COLUMN: f64 = 130.0;

    fn at_height(x: f64, height: f64, water: f64) -> Point {
        Point { x, y: 1.0 - height / water }
    }

    #[test]
    fn slope_keeps_the_underpass_open_and_blocks_a_crossing_through_wood() {
        let slope = PondBedProfile::slope(0.235);
        let top = PondBedProfile::slope_ridge_height(120.0);
        let x = 0.235 + 120.0 / BASIN_WIDTH_PX;
        let below = at_height(x, 30.0, COLUMN);
        let above = at_height(x, top + 5.0, COLUMN);
        let inside = at_height(x, top - 6.0, COLUMN);
        assert!(!slope.blocks(below, COLUMN));
        assert!(!slope.blocks(above, COLUMN));
        assert!(slope.blocks(inside, COLUMN));
        assert!(slope.blocks_segment(below, above, COLUMN));
        let next = at_height(x + 32.0 / BASIN_WIDTH_PX, 30.0, COLUMN);
        assert!(!slope.blocks_segment(below, next, COLUMN));
    }

    #[test]
    fn authored_gaps_allow_crossing_without_allowing_solid_contacts() {
        for (preset, radius, open_height, solid_offset, solid_height) in [
            (StructurePreset::HangingRoots, 28.0, 8.0, -19.0, 39.0),
            (StructurePreset::RootNetwork, 42.0, 23.0, -22.0, 36.0),
            (StructurePreset::FlatStoneCave, 26.0, 6.0, 0.0, 11.0),
        ] {
            let profile = PondBedProfile::structure(0.5, preset);
            let x = |delta_px| 0.5 + delta_px / BASIN_WIDTH_PX;
            let mouth = at_height(x(radius + 2.0), open_height, COLUMN);
            let exit = at_height(x(-radius - 2.0), open_height, COLUMN);
            let open = at_height(x(solid_offset), open_height, COLUMN);
            let solid = at_height(x(solid_offset), solid_height, COLUMN);
            assert!(!profile.blocks(open, COLUMN));
            assert!(!profile.blocks_segment(mouth, exit, COLUMN));
            assert!(profile.blocks(solid, COLUMN));
            assert!(profile.blocks_segment(open, at_height(solid.x, solid_height + 2.0, COLUMN), COLUMN));
        }
        let cave = PondBedProfile::structure(0.5, StructurePreset::FlatStoneCave);
        assert!(cave.blocks_root(0.5 + 17.0 / BASIN_WIDTH_PX));
        assert!(!cave.blocks_root(0.5));
        assert!(PondSwimRange::near_cave_floor(MAX_WATER_COLUMN_PX).contains(at_height(0.5, 6.0, MAX_WATER_COLUMN_PX)));
        assert!(!PondSwimRange::EXTENDED.contains(at_height(0.5, 6.0, MAX_WATER_COLUMN_PX)));
    }

    #[test]
    fn ground_objects_fit_under_the_limb_but_cannot_overlap_the_wood_feet() {
        let slope = PondBedProfile::slope(0.235);
        let rock = PondBedProfile::flat(0.70, 32.0, 27.0);
        let foot = PondBedProfile::flat(0.26, 10.0, 9.0);
        assert!(!slope.overlaps(rock) && !rock.overlaps(slope));
        assert!(slope.overlaps(foot) && foot.overlaps(slope));
    }

    #[test]
    fn receding_water_checks_the_whole_layout_before_relocating_a_blocked_swimmer() {
        let slope = PondBedProfile::slope(0.235);
        let rock = PondBedProfile::flat(0.26, 32.0, 27.0);
        let water = 60.0;
        let point = at_height(0.235, 34.0, water);
        assert!(slope.blocks(point, water));
        assert!(rock.blocks(at_height(point.x, 24.75, water), water));
        let open = slope.clear_after_depth_change(point, water, |candidate| {
            !slope.blocks(candidate, water) && !rock.blocks(candidate, water)
        });
        assert_ne!(open, point);
        assert_ne!(open.x, point.x);
        assert!(PondSwimRange::EXTENDED.contains(open));
        assert!(!slope.blocks(open, water) && !rock.blocks(open, water));
    }
}

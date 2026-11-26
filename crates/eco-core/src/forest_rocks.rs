use crate::forest_scenery::{ForestScenery, ForestSceneryLayout, ForestSceneryParent};
use crate::{forest, Animal, CoreError, Point};
use serde::{Deserialize, Serialize};

const ROCK_LIMIT: usize = 8;
const LOG_LIMIT: usize = 4;
const STONEWORK_LIMIT: usize = 8;
const BARK_LIMIT: usize = 6;
const LEAF_PATCH_LIMIT: usize = 16;
// The authored F03 log spans 260 mm across the 450 mm reference view.
pub(crate) const LOG_HALF_SPAN: f64 = 260.0 / 450.0 / 2.0;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ForestRock {
    pub id: u32,
    pub position: Point,
    pub mirrored: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ForestLog {
    pub id: u32,
    pub position: Point,
    pub mirrored: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ForestStoneworkKind {
    Slab,
    Pair,
    Cluster,
}

impl ForestStoneworkKind {
    fn contact_offsets_mm(self) -> &'static [f64] {
        match self {
            Self::Slab => &[0.0],
            Self::Pair => &[-42.0, 42.0],
            Self::Cluster => &[-61.0, 0.0, 61.0],
        }
    }

    fn contains_anchor(self, position: Point) -> bool {
        let x_range = match self {
            Self::Slab => 0.11..=0.89,
            Self::Pair => 0.16..=0.84,
            Self::Cluster => 0.20..=0.80,
        };
        x_range.contains(&position.x) && (0.60..=0.91).contains(&position.y)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ForestStonework {
    pub id: u32,
    pub position: Point,
    pub mirrored: bool,
    pub preset: ForestStoneworkKind,
}

impl ForestStonework {
    pub(crate) fn contact_positions(self) -> impl Iterator<Item = Point> + Clone {
        let direction = if self.mirrored { -1.0 } else { 1.0 };
        self.preset.contact_offsets_mm().iter().map(move |offset| Point {
            x: self.position.x + direction * offset / 450.0,
            y: self.position.y,
        })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ForestBarkPreset {
    Arch,
    Split,
    RootArch,
    BranchBridge,
}

impl ForestBarkPreset {
    fn foot_offset_mm(self) -> f64 {
        match self {
            Self::Arch => 38.0,
            Self::Split => 51.0,
            Self::RootArch => 51.0,
            Self::BranchBridge => 78.0,
        }
    }

    /// Opening width and height in millimetres at the authored side-on reference.
    /// The ground grid reserves an explicit clear route through the opening.
    pub fn bridge_clearance_mm(self) -> Option<[u16; 2]> {
        match self {
            Self::Arch | Self::Split => None,
            Self::RootArch => Some([65, 12]),
            Self::BranchBridge => Some([90, 22]),
        }
    }

    fn contains_anchor(self, position: Point) -> bool {
        let x_range = match self {
            Self::Arch => 0.15..=0.85,
            Self::Split => 0.18..=0.82,
            Self::RootArch => 0.18..=0.82,
            Self::BranchBridge => 0.24..=0.76,
        };
        x_range.contains(&position.x) && (0.70..=0.81).contains(&position.y)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ForestBarkShelter {
    pub id: u32,
    pub position: Point,
    pub mirrored: bool,
    pub preset: ForestBarkPreset,
}

impl ForestBarkShelter {
    /// The two solid cells, in world left-to-right order.
    pub fn foot_cells(self) -> [usize; 2] {
        let offset = self.preset.foot_offset_mm() / 450.0;
        [-offset, offset].map(|dx| forest::cell_index(Point { x: self.position.x + dx, y: self.position.y }))
    }

    /// Back entrance, covered center, and front entrance in row-major order.
    pub fn passage_cells(self) -> [usize; 3] {
        let center = forest::cell_index(self.position);
        let back = center.checked_sub(forest::COLUMNS).unwrap_or(center);
        let front = center + forest::COLUMNS;
        [back, center, if front < forest::CELL_COUNT { front } else { center }]
    }

    /// True while the point lies under the roof in the covered center cell.
    pub fn contains(self, position: Point) -> bool {
        forest::on_ground(position) && forest::cell_index(self.position) == forest::cell_index(position)
    }

    pub fn in_passage(self, position: Point) -> bool {
        forest::on_ground(position) && self.passage_cells().contains(&forest::cell_index(position))
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ForestLeafPreset {
    Leaves,
    Heap,
    Fragments,
}

impl ForestLeafPreset {
    /// Fixture stock in mg C, N and P, independent of the depicted leaf area.
    pub fn initial_mg_cnp(self) -> [f64; 3] {
        let carbon = match self {
            Self::Leaves => 24.0,
            Self::Heap => 60.0,
            Self::Fragments => 12.0,
        };
        Self::dose_for_carbon(carbon)
    }

    pub(crate) fn dose_for_carbon(carbon: f64) -> [f64; 3] {
        [carbon, carbon * 0.043, carbon * 0.0045]
    }

    fn contains_anchor(self, position: Point) -> bool {
        let x_range = match self {
            Self::Leaves | Self::Fragments => 0.16..=0.84,
            Self::Heap => 0.24..=0.76,
        };
        x_range.contains(&position.x) && (0.60..=0.91).contains(&position.y)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ForestLeafPatch {
    pub id: u32,
    pub position: Point,
    pub mirrored: bool,
    pub preset: ForestLeafPreset,
    pub remaining_mg_cnp: [f64; 3],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) initial_dose_mg_cnp: Option<[f64; 3]>,
}

impl ForestLeafPatch {
    pub fn initial_mg_cnp(self) -> [f64; 3] {
        self.initial_dose_mg_cnp.unwrap_or_else(|| self.preset.initial_mg_cnp())
    }

    pub fn cell_index(self) -> usize {
        forest::cell_index(self.position)
    }

    fn valid_stock(self) -> bool {
        if let Some(dose) = self.initial_dose_mg_cnp {
            if self.preset != ForestLeafPreset::Fragments || !(dose[0] > 0.0 && dose[0] <= 240.0)
                || !dose.into_iter().zip(ForestLeafPreset::dose_for_carbon(dose[0]))
                    .all(|(actual, expected)| actual.is_finite() && (actual - expected).abs() < 1e-10)
            { return false; }
        }
        self.remaining_mg_cnp.into_iter().zip(self.initial_mg_cnp())
            .all(|(remaining, initial)| (0.0..=initial).contains(&remaining))
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase", deny_unknown_fields)]
pub enum ForestRockEdit {
    Add { position: Point },
    AddLog { position: Point },
    AddStonework { position: Point, preset: ForestStoneworkKind },
    AddBarkShelter { position: Point, preset: ForestBarkPreset },
    AddLeafPatch { position: Point, preset: ForestLeafPreset },
    PaintLeafPatches { preset: ForestLeafPreset, positions: Vec<Point> },
    SpreadLeafFragments { positions: Vec<Point>, mg_c: f64 },
    AddMossPatch { position: Point },
    MoveMossPatch { id: u32, position: Point },
    TrimMossPatch { id: u32, mg_c: f64 },
    RemoveMossPatch { id: u32 },
    AddFernPatch { position: Point },
    MoveFernPatch { id: u32, position: Point },
    TrimFernPatch { id: u32, mg_c: f64 },
    RemoveFernPatch { id: u32 },
    SetSubstrate { cells: Vec<usize>, substrate: forest::ForestSubstrate },
    StockWoodlice { position: Point, count: u32 },
    RemoveWoodlouse { id: String },
    AddScenery { style: String, x: f64, y: f64, #[serde(default)] parent: Option<ForestSceneryParent> },
    MoveScenery { id: u32, x: f64, y: f64 },
    RemoveScenery { id: u32 },
    Move { id: u32, position: Point },
    Mirror { id: u32 },
    Remove { id: u32 },
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ForestGroundPlant {
    pub(crate) id: u32,
    pub(crate) position: Point,
    /// Attributed fraction of the existing functional plant stock in the occupied soil cell.
    pub(crate) share: f64,
}

impl ForestGroundPlant {
    pub(crate) fn cell_index(self) -> usize { forest::cell_index(self.position) }
}

#[derive(Clone, Copy)]
enum GroundPlantEdit {
    Add(Point),
    Move(u32, Point),
    Trim(u32, f64),
    Remove(u32),
}

impl ForestRockEdit {
    fn ground_plant_edit(&self) -> Option<(forest::ForestPlantKind, GroundPlantEdit)> {
        use forest::ForestPlantKind::{Fern, Moss};
        use GroundPlantEdit::{Add, Move, Remove, Trim};
        Some(match *self {
            Self::AddMossPatch { position } => (Moss, Add(position)),
            Self::MoveMossPatch { id, position } => (Moss, Move(id, position)),
            Self::TrimMossPatch { id, mg_c } => (Moss, Trim(id, mg_c)),
            Self::RemoveMossPatch { id } => (Moss, Remove(id)),
            Self::AddFernPatch { position } => (Fern, Add(position)),
            Self::MoveFernPatch { id, position } => (Fern, Move(id, position)),
            Self::TrimFernPatch { id, mg_c } => (Fern, Trim(id, mg_c)),
            Self::RemoveFernPatch { id } => (Fern, Remove(id)),
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ForestRockLayout {
    next_id: u32,
    rocks: Vec<ForestRock>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    logs: Option<Vec<ForestLog>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    stonework: Option<Vec<ForestStonework>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bark_shelters: Option<Vec<ForestBarkShelter>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    leaf_patches: Option<Vec<ForestLeafPatch>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    moss_patches: Vec<ForestGroundPlant>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    fern_patches: Vec<ForestGroundPlant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    scenery: Option<ForestSceneryLayout>,
}

impl ForestRockLayout {
    pub(crate) fn new(position: Point) -> Self {
        Self {
            next_id: 2,
            rocks: vec![ForestRock { id: 1, position, mirrored: false }],
            logs: None,
            stonework: None,
            bark_shelters: None,
            leaf_patches: None,
            moss_patches: Vec::new(),
            fern_patches: Vec::new(),
            scenery: None,
        }
    }

    pub(crate) fn with_logs(position: Point) -> Self {
        Self { logs: Some(Vec::new()), ..Self::new(position) }
    }

    pub(crate) fn with_stonework(position: Point) -> Self {
        Self { stonework: Some(Vec::new()), ..Self::with_logs(position) }
    }

    pub(crate) fn with_bark_shelters(position: Point) -> Self {
        Self { bark_shelters: Some(Vec::new()), ..Self::with_stonework(position) }
    }

    pub(crate) fn with_leaf_patches(position: Point) -> Self {
        Self { leaf_patches: Some(Vec::new()), ..Self::with_bark_shelters(position) }
    }

    pub(crate) fn with_scenery(rock: Point, arch: Point, leaf: Point) -> Self {
        let leaf_preset = ForestLeafPreset::Heap;
        Self {
            next_id: 4,
            bark_shelters: Some(vec![ForestBarkShelter { id: 2, position: arch, mirrored: false, preset: ForestBarkPreset::Arch }]),
            leaf_patches: Some(vec![ForestLeafPatch { id: 3, position: leaf, mirrored: false, preset: leaf_preset,
                remaining_mg_cnp: leaf_preset.initial_mg_cnp(), initial_dose_mg_cnp: None }]),
            scenery: Some(ForestSceneryLayout::seeded()),
            ..Self::with_leaf_patches(rock)
        }
    }

    pub(crate) fn blank_scenery() -> Self {
        Self {
            next_id: 2,
            rocks: Vec::new(),
            logs: Some(Vec::new()),
            stonework: Some(Vec::new()),
            bark_shelters: Some(Vec::new()),
            leaf_patches: Some(Vec::new()),
            moss_patches: Vec::new(),
            fern_patches: Vec::new(),
            scenery: Some(ForestSceneryLayout::empty()),
        }
    }

    pub fn rocks(&self) -> &[ForestRock] {
        &self.rocks
    }

    pub fn logs(&self) -> Option<&[ForestLog]> {
        self.logs.as_deref()
    }

    pub fn stonework(&self) -> Option<&[ForestStonework]> {
        self.stonework.as_deref()
    }

    pub fn bark_shelters(&self) -> Option<&[ForestBarkShelter]> {
        self.bark_shelters.as_deref()
    }

    pub fn leaf_patches(&self) -> Option<&[ForestLeafPatch]> {
        self.leaf_patches.as_deref()
    }

    pub fn scenery(&self) -> Option<&[ForestScenery]> {
        self.scenery.as_ref().map(|layout| layout.items.as_slice())
    }

    fn contains_terrain(&self, id: u32) -> bool {
        self.rocks.iter().any(|entry| entry.id == id)
            || self.logs.iter().flatten().any(|entry| entry.id == id)
            || self.stonework.iter().flatten().any(|entry| entry.id == id)
            || self.bark_shelters.iter().flatten().any(|entry| entry.id == id)
            || self.leaf_patches.iter().flatten().any(|entry| entry.id == id)
    }

    pub(crate) fn leaf_patches_mut(&mut self) -> Option<&mut [ForestLeafPatch]> {
        self.leaf_patches.as_deref_mut()
    }

    pub(crate) fn moss_patches(&self) -> &[ForestGroundPlant] { &self.moss_patches }

    pub(crate) fn fern_patches(&self) -> &[ForestGroundPlant] { &self.fern_patches }

    pub(crate) fn ground_plants_mut(&mut self) -> (&mut [ForestGroundPlant], &mut [ForestGroundPlant]) {
        (&mut self.moss_patches, &mut self.fern_patches)
    }

    fn ground_plants_mut_of(&mut self, kind: forest::ForestPlantKind) -> &mut Vec<ForestGroundPlant> {
        match kind {
            forest::ForestPlantKind::Moss => &mut self.moss_patches,
            forest::ForestPlantKind::Fern => &mut self.fern_patches,
        }
    }

    pub(crate) fn validate(&self, animals: &[Animal]) -> Result<(), CoreError> {
        if self.next_id < 2 || self.rocks.len() > ROCK_LIMIT {
            return Err(CoreError::InvalidForestState);
        }
        for (index, rock) in self.rocks.iter().enumerate() {
            forest::validate_rock_change(rock.position, animals)?;
            if rock.id == 0 || rock.id >= self.next_id || self.rocks[..index].iter().any(|other| other.id == rock.id) {
                return Err(CoreError::InvalidForestState);
            }
            if self.rocks[..index].iter().any(|other| forest::cell_index(other.position) == forest::cell_index(rock.position)) {
                return Err(CoreError::OverlappingRockSite);
            }
        }
        let mut occupied = [false; forest::CELL_COUNT];
        for rock in &self.rocks {
            occupied[forest::cell_index(rock.position)] = true;
        }
        let logs = self.logs().unwrap_or_default();
        self.validate_logs(logs, animals, &mut occupied)?;
        if let Some(stonework) = self.stonework() {
            self.validate_stonework(stonework, logs, animals, &mut occupied)?;
        }
        if let Some(shelters) = self.bark_shelters() {
            self.validate_bark_shelters(shelters, animals, &mut occupied)?;
        }
        self.validate_plant_patches(&occupied, forest::ForestPlantKind::Moss, &self.moss_patches)?;
        self.validate_plant_patches(&occupied, forest::ForestPlantKind::Fern, &self.fern_patches)?;
        if let Some(patches) = self.leaf_patches() {
            self.validate_leaf_patches(patches, &mut occupied)?;
        }
        if self.scenery.as_ref().is_some_and(|layout| !layout.valid(|id| self.contains_terrain(id))) {
            return Err(CoreError::InvalidForestState);
        }
        Ok(())
    }

    fn validate_plant_patches(&self, solids: &[bool; forest::CELL_COUNT], kind: forest::ForestPlantKind,
        patches: &[ForestGroundPlant]) -> Result<(), CoreError> {
        if patches.is_empty() { return Ok(()); }
        if self.scenery.is_none() || patches.len() > forest::CELL_COUNT {
            return Err(CoreError::InvalidForestState);
        }
        let mut planted = [false; forest::CELL_COUNT];
        let fern = kind == forest::ForestPlantKind::Fern;
        for (index, patch) in patches.iter().enumerate() {
            if patch.id == 0 || patch.id >= self.next_id || !(0.0..=1.0).contains(&patch.share)
                || self.contains_terrain(patch.id) || patches[..index].iter().any(|other| other.id == patch.id)
                || (fern && self.moss_patches.iter().any(|other| other.id == patch.id))
            {
                return Err(CoreError::InvalidForestState);
            }
            if !forest::on_ground(patch.position) { return Err(kind.invalid_site()); }
            let cell = patch.cell_index();
            if solids[cell] || planted[cell]
                || (fern && self.bark_shelters.iter().flatten().any(|shelter| shelter.in_passage(patch.position)))
            {
                return Err(kind.blocked_site());
            }
            planted[cell] = true;
        }
        Ok(())
    }

    fn validate_logs(
        &self, logs: &[ForestLog], animals: &[Animal], occupied: &mut [bool; forest::CELL_COUNT],
    ) -> Result<(), CoreError> {
        if logs.len() > LOG_LIMIT {
            return Err(CoreError::InvalidForestState);
        }
        for (index, log) in logs.iter().enumerate() {
            if log.id == 0 || log.id >= self.next_id
                || self.rocks.iter().any(|other| other.id == log.id)
                || logs[..index].iter().any(|other| other.id == log.id)
            {
                return Err(CoreError::InvalidForestState);
            }
            if !(0.33..=0.67).contains(&log.position.x) || !(0.60..=0.91).contains(&log.position.y) {
                return Err(CoreError::InvalidLogSite);
            }
            let footprint = forest::log_cells(log.position);
            if footprint.clone().any(|cell| occupied[cell]) {
                return Err(CoreError::OverlappingLogSite);
            }
            if animals.iter().any(|animal| {
                let position = Point { x: animal.x, y: animal.y };
                footprint.contains(&forest::cell_index(position))
            }) {
                return Err(CoreError::OccupiedLogSite);
            }
            for cell in footprint {
                occupied[cell] = true;
            }
        }
        Ok(())
    }

    fn validate_stonework(
        &self, stonework: &[ForestStonework], logs: &[ForestLog], animals: &[Animal],
        occupied: &mut [bool; forest::CELL_COUNT],
    ) -> Result<(), CoreError> {
        if stonework.len() > STONEWORK_LIMIT {
            return Err(CoreError::InvalidForestState);
        }
        let mut animal_cells = [false; forest::CELL_COUNT];
        for animal in animals {
            let position = Point { x: animal.x, y: animal.y };
            animal_cells[forest::cell_index(position)] = true;
        }
        for (index, piece) in stonework.iter().enumerate() {
            if piece.id == 0 || piece.id >= self.next_id
                || self.rocks.iter().any(|other| other.id == piece.id)
                || logs.iter().any(|other| other.id == piece.id)
                || stonework[..index].iter().any(|other| other.id == piece.id)
            {
                return Err(CoreError::InvalidForestState);
            }
            if !piece.preset.contains_anchor(piece.position) {
                return Err(CoreError::InvalidStoneworkSite);
            }
            let contacts = piece.contact_positions().map(forest::cell_index);
            if contacts.clone().any(|cell| occupied[cell]) {
                return Err(CoreError::OverlappingStoneworkSite);
            }
            if contacts.clone().any(|cell| animal_cells[cell]) {
                return Err(CoreError::OccupiedStoneworkSite);
            }
            for cell in contacts {
                occupied[cell] = true;
            }
        }
        if occupied.iter().all(|cell| *cell) {
            return Err(CoreError::ForestNoOpenGround);
        }
        Ok(())
    }

    fn validate_bark_shelters(
        &self, shelters: &[ForestBarkShelter], animals: &[Animal], occupied: &mut [bool; forest::CELL_COUNT],
    ) -> Result<(), CoreError> {
        if shelters.len() > BARK_LIMIT {
            return Err(CoreError::InvalidForestState);
        }
        let mut reserved_passages = [false; forest::CELL_COUNT];
        let mut animal_cells = [false; forest::CELL_COUNT];
        for animal in animals {
            let position = Point { x: animal.x, y: animal.y };
            animal_cells[forest::cell_index(position)] = true;
        }
        for (index, shelter) in shelters.iter().enumerate() {
            if shelter.preset.bridge_clearance_mm().is_some() && self.scenery.is_none() {
                return Err(CoreError::WrongWorldKind);
            }
            if shelter.id == 0 || shelter.id >= self.next_id
                || self.rocks.iter().any(|other| other.id == shelter.id)
                || self.logs.iter().flatten().any(|other| other.id == shelter.id)
                || self.stonework.iter().flatten().any(|other| other.id == shelter.id)
                || shelters[..index].iter().any(|other| other.id == shelter.id)
            {
                return Err(CoreError::InvalidForestState);
            }
            if !shelter.preset.contains_anchor(shelter.position) {
                return Err(CoreError::InvalidBarkSite);
            }
            let feet = shelter.foot_cells();
            let passage = shelter.passage_cells();
            if feet.iter().any(|cell| occupied[*cell] || reserved_passages[*cell]) {
                return Err(CoreError::OverlappingBarkSite);
            }
            if feet.iter().any(|cell| animal_cells[*cell]) {
                return Err(CoreError::OccupiedBarkFeet);
            }
            if passage.iter().any(|cell| occupied[*cell] || reserved_passages[*cell]) {
                return Err(CoreError::BlockedBarkPassage);
            }
            for cell in feet {
                occupied[cell] = true;
            }
            for cell in passage {
                reserved_passages[cell] = true;
            }
        }
        Ok(())
    }

    fn validate_leaf_patches(
        &self, patches: &[ForestLeafPatch], occupied: &mut [bool; forest::CELL_COUNT],
    ) -> Result<(), CoreError> {
        if patches.len() > LEAF_PATCH_LIMIT {
            return Err(CoreError::InvalidForestState);
        }
        for (index, patch) in patches.iter().enumerate() {
            if patch.id == 0 || patch.id >= self.next_id || !patch.valid_stock()
                || self.rocks.iter().any(|other| other.id == patch.id)
                || self.logs.iter().flatten().any(|other| other.id == patch.id)
                || self.stonework.iter().flatten().any(|other| other.id == patch.id)
                || self.bark_shelters.iter().flatten().any(|other| other.id == patch.id)
                || patches[..index].iter().any(|other| other.id == patch.id)
            {
                return Err(CoreError::InvalidForestState);
            }
            if !patch.preset.contains_anchor(patch.position) {
                return Err(CoreError::InvalidLeafPatchSite);
            }
            let cell = patch.cell_index();
            if occupied[cell] || self.bark_shelters.iter().flatten().any(|shelter| shelter.in_passage(patch.position)) {
                return Err(CoreError::OverlappingLeafPatchSite);
            }
            occupied[cell] = true;
        }
        Ok(())
    }

    fn add_leaf_patches(&mut self, preset: ForestLeafPreset, positions: &[Point], spread_mg_c: Option<f64>) -> Result<(), CoreError> {
        let patches = self.leaf_patches.as_mut().ok_or(CoreError::WrongWorldKind)?;
        if positions.is_empty() {
            return Err(CoreError::InvalidLeafPatchBatchSize);
        }
        if positions.len() > LEAF_PATCH_LIMIT.saturating_sub(patches.len()) {
            return Err(CoreError::ForestLeafPatchLimit);
        }
        let first_id = self.next_id;
        let next_id = first_id.checked_add(positions.len() as u32).ok_or(CoreError::ForestLeafPatchLimit)?;
        let mut unallocated = spread_mg_c.map(ForestLeafPreset::dose_for_carbon);
        let per_cell = unallocated.map(|dose| dose.map(|amount| amount / positions.len() as f64));
        for (index, (id, position)) in (first_id..next_id).zip(positions.iter().copied()).enumerate() {
            let initial_dose_mg_cnp = if index + 1 == positions.len() { unallocated } else { per_cell };
            let remaining_mg_cnp = initial_dose_mg_cnp.unwrap_or_else(|| preset.initial_mg_cnp());
            if let Some(remaining) = &mut unallocated {
                for (amount, allocated) in remaining.iter_mut().zip(remaining_mg_cnp) { *amount -= allocated; }
            }
            patches.push(ForestLeafPatch { id, position, mirrored: false, preset, remaining_mg_cnp, initial_dose_mg_cnp });
        }
        self.next_id = next_id;
        Ok(())
    }

    pub(crate) fn edit(&mut self, edit: &ForestRockEdit, animals: &[Animal], forest: &mut forest::ForestState) -> Result<(), CoreError> {
        if let Some((kind, action)) = edit.ground_plant_edit() {
            return self.edit_ground_plant(kind, action, animals, forest);
        }
        let mut next = self.clone();
        match *edit {
            ForestRockEdit::Add { position } => {
                if next.rocks.len() >= ROCK_LIMIT { return Err(CoreError::ForestRockLimit); }
                let id = next.next_id;
                next.next_id = id.checked_add(1).ok_or(CoreError::ForestRockLimit)?;
                next.rocks.push(ForestRock { id, position, mirrored: false });
            }
            ForestRockEdit::AddLog { position } => {
                let logs = next.logs.as_mut().ok_or(CoreError::WrongWorldKind)?;
                if logs.len() >= LOG_LIMIT {
                    return Err(CoreError::ForestLogLimit);
                }
                let id = next.next_id;
                next.next_id = id.checked_add(1).ok_or(CoreError::ForestLogLimit)?;
                logs.push(ForestLog { id, position, mirrored: false });
            }
            ForestRockEdit::AddStonework { position, preset } => {
                let stonework = next.stonework.as_mut().ok_or(CoreError::WrongWorldKind)?;
                if stonework.len() >= STONEWORK_LIMIT {
                    return Err(CoreError::ForestStoneworkLimit);
                }
                let id = next.next_id;
                next.next_id = id.checked_add(1).ok_or(CoreError::ForestStoneworkLimit)?;
                stonework.push(ForestStonework { id, position, mirrored: false, preset });
            }
            ForestRockEdit::AddBarkShelter { position, preset } => {
                let shelters = next.bark_shelters.as_mut().ok_or(CoreError::WrongWorldKind)?;
                if shelters.len() >= BARK_LIMIT {
                    return Err(CoreError::ForestBarkLimit);
                }
                let id = next.next_id;
                next.next_id = id.checked_add(1).ok_or(CoreError::ForestBarkLimit)?;
                shelters.push(ForestBarkShelter { id, position, mirrored: false, preset });
            }
            ForestRockEdit::AddLeafPatch { position, preset } => {
                next.add_leaf_patches(preset, &[position], None)?;
            }
            ForestRockEdit::PaintLeafPatches { preset, ref positions } => {
                next.add_leaf_patches(preset, positions, None)?;
            }
            ForestRockEdit::SpreadLeafFragments { ref positions, mg_c } => {
                if !(0.1..=240.0).contains(&mg_c) { return Err(CoreError::InvalidLeafFragmentDose); }
                next.add_leaf_patches(ForestLeafPreset::Fragments, positions, Some(mg_c))?;
            }
            ForestRockEdit::AddMossPatch { .. } | ForestRockEdit::MoveMossPatch { .. }
            | ForestRockEdit::TrimMossPatch { .. } | ForestRockEdit::RemoveMossPatch { .. }
            | ForestRockEdit::AddFernPatch { .. } | ForestRockEdit::MoveFernPatch { .. }
            | ForestRockEdit::TrimFernPatch { .. } | ForestRockEdit::RemoveFernPatch { .. } => {
                unreachable!("ground plant operations are handled before terrain edits");
            }
            ForestRockEdit::StockWoodlice { .. } | ForestRockEdit::RemoveWoodlouse { .. } | ForestRockEdit::SetSubstrate { .. } => {
                return Err(CoreError::WrongWorldKind);
            }
            ForestRockEdit::AddScenery { ref style, x, y, parent } => {
                let terrain_exists = match parent {
                    Some(ForestSceneryParent::Terrain { id }) => next.contains_terrain(id),
                    _ => true,
                };
                let scenery = next.scenery.as_mut().ok_or(CoreError::WrongWorldKind)?;
                if !terrain_exists || matches!(parent, Some(ForestSceneryParent::Scenery { id })
                    if !scenery.items.iter().any(|entry| entry.id == id)) {
                    return Err(CoreError::ForestSceneryParentMissing);
                }
                scenery.add(style, x, y, parent)?;
            }
            ForestRockEdit::MoveScenery { id, x, y } => {
                next.scenery.as_mut().ok_or(CoreError::WrongWorldKind)?.move_item(id, x, y)?;
            }
            ForestRockEdit::RemoveScenery { id } => {
                next.scenery.as_mut().ok_or(CoreError::WrongWorldKind)?.remove(id)?;
            }
            ForestRockEdit::Move { id, position } => *next.pose_mut(id)?.0 = position,
            ForestRockEdit::Mirror { id } => {
                let mirrored = next.pose_mut(id)?.1;
                *mirrored = !*mirrored;
            }
            ForestRockEdit::Remove { id } => {
                next.remove(id)?;
                if let Some(scenery) = &mut next.scenery {
                    scenery.remove_children(ForestSceneryParent::Terrain { id });
                }
            }
        }
        next.validate(animals)?;
        *self = next;
        Ok(())
    }

    fn edit_ground_plant(&mut self, kind: forest::ForestPlantKind, action: GroundPlantEdit,
        animals: &[Animal], forest: &mut forest::ForestState) -> Result<(), CoreError> {
        if self.scenery.is_none() { return Err(CoreError::WrongWorldKind); }
        let mut next = self.clone();
        let find = |patches: &[ForestGroundPlant], id| {
            patches.iter().position(|patch| patch.id == id).ok_or(kind.missing())
        };
        match action {
            GroundPlantEdit::Add(position) => {
                if next.ground_plants_mut_of(kind).len() >= forest::CELL_COUNT { return Err(kind.limit()); }
                let id = next.next_id;
                next.next_id = id.checked_add(1).ok_or(kind.limit())?;
                let patch = ForestGroundPlant { id, position, share: 1.0 };
                next.ground_plants_mut_of(kind).push(patch);
                next.validate(animals)?;
                next.ground_plants_mut_of(kind).last_mut().expect("appended plant").share =
                    forest.add_ground_plant(kind, patch.cell_index(), true);
            }
            GroundPlantEdit::Move(id, position) => {
                let patches = next.ground_plants_mut_of(kind);
                let index = find(patches, id)?;
                let previous = patches[index];
                patches[index].position = position;
                next.validate(animals)?;
                next.ground_plants_mut_of(kind)[index].share = forest.move_ground_plant(kind, previous, forest::cell_index(position));
            }
            GroundPlantEdit::Trim(id, mg_c) => {
                let patches = next.ground_plants_mut_of(kind);
                let index = find(patches, id)?;
                patches[index].share = forest.trim_ground_plant(kind, patches[index], mg_c)?;
            }
            GroundPlantEdit::Remove(id) => {
                let patches = next.ground_plants_mut_of(kind);
                let index = find(patches, id)?;
                forest.remove_ground_plant(kind, patches.remove(index));
            }
        }
        *self = next;
        Ok(())
    }

    pub(crate) fn restore(&mut self, mut previous: Self, animals: &[Animal]) -> Result<(), CoreError> {
        if self.logs.is_some() != previous.logs.is_some()
            || self.stonework.is_some() != previous.stonework.is_some()
            || self.bark_shelters.is_some() != previous.bark_shelters.is_some()
            || self.leaf_patches.is_some() != previous.leaf_patches.is_some()
            || self.scenery.is_some() != previous.scenery.is_some()
        {
            return Err(CoreError::WrongWorldKind);
        }
        previous.next_id = previous.next_id.max(self.next_id);
        if let (Some(prior), Some(current)) = (&mut previous.scenery, &self.scenery) {
            prior.preserve_allocations(current);
        }
        previous.validate(animals)?;
        *self = previous;
        Ok(())
    }

    fn pose_mut(&mut self, id: u32) -> Result<(&mut Point, &mut bool), CoreError> {
        if let Some(rock) = self.rocks.iter_mut().find(|rock| rock.id == id) {
            return Ok((&mut rock.position, &mut rock.mirrored));
        }
        if let Some(log) = self.logs.iter_mut().flatten().find(|log| log.id == id) {
            return Ok((&mut log.position, &mut log.mirrored));
        }
        if let Some(piece) = self.stonework.iter_mut().flatten().find(|piece| piece.id == id) {
            return Ok((&mut piece.position, &mut piece.mirrored));
        }
        if let Some(shelter) = self.bark_shelters.iter_mut().flatten().find(|shelter| shelter.id == id) {
            return Ok((&mut shelter.position, &mut shelter.mirrored));
        }
        let patch = self.leaf_patches.iter_mut().flatten().find(|patch| patch.id == id)
            .ok_or(CoreError::ForestRockMissing)?;
        Ok((&mut patch.position, &mut patch.mirrored))
    }

    fn remove(&mut self, id: u32) -> Result<(), CoreError> {
        if let Some(index) = self.rocks.iter().position(|rock| rock.id == id) {
            self.rocks.remove(index);
            return Ok(());
        }
        if let Some(logs) = &mut self.logs {
            if let Some(index) = logs.iter().position(|log| log.id == id) {
                logs.remove(index);
                return Ok(());
            }
        }
        if let Some(stonework) = &mut self.stonework {
            if let Some(index) = stonework.iter().position(|piece| piece.id == id) {
                stonework.remove(index);
                return Ok(());
            }
        }
        if let Some(shelters) = &mut self.bark_shelters {
            if let Some(index) = shelters.iter().position(|shelter| shelter.id == id) {
                shelters.remove(index);
                return Ok(());
            }
        }
        let patches = self.leaf_patches.as_mut().ok_or(CoreError::ForestRockMissing)?;
        let index = patches.iter().position(|patch| patch.id == id).ok_or(CoreError::ForestRockMissing)?;
        patches.remove(index);
        Ok(())
    }
}

pub(crate) fn positions(legacy: Point, layout: Option<&ForestRockLayout>) -> impl Iterator<Item = Point> + Clone + '_ {
    layout.is_none().then_some(legacy).into_iter()
        .chain(layout.into_iter().flat_map(|layout| layout.rocks().iter().map(|rock| rock.position)))
}

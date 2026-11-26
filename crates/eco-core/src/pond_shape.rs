use serde::{Deserialize, Serialize};

pub(super) const COLUMNS: usize = 12;
pub(super) const ROWS: usize = 12;
pub(super) const CELL_COUNT: usize = COLUMNS * ROWS;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PondShapeMode {
    Add,
    Remove,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PondShapeError {
    InvalidCells,
    ProtectedRow,
    Unchanged,
    Disconnected,
    TouchesOther,
    AreaOutOfRange,
    Capacity,
    Occupied,
    TopologyOperation,
    TopologyOccupied,
    TopologySection,
    TopologySectionLimit,
    TopologyScale,
    TopologyLimit,
    TopologyShallow,
    TopologyLostAnchor,
    TopologySedgeAnchor,
    TopologyPopulation,
}

impl PondShapeError {
    pub fn code(self) -> &'static str {
        match self {
            Self::InvalidCells => "pond_shape_invalid_cells",
            Self::ProtectedRow => "pond_shape_protected_row",
            Self::Unchanged => "pond_shape_unchanged",
            Self::Disconnected => "pond_shape_disconnected",
            Self::TouchesOther => "pond_shape_touches_other",
            Self::AreaOutOfRange => "pond_shape_area_out_of_range",
            Self::Capacity => "pond_shape_capacity",
            Self::Occupied => "pond_shape_occupied",
            Self::TopologyOperation => "pond_shape_topology_operation",
            Self::TopologyOccupied => "pond_shape_topology_occupied",
            Self::TopologySection => "pond_shape_topology_section",
            Self::TopologySectionLimit => "pond_shape_topology_section_limit",
            Self::TopologyScale => "pond_shape_topology_scale",
            Self::TopologyLimit => "pond_shape_topology_limit",
            Self::TopologyShallow => "pond_shape_topology_shallow",
            Self::TopologyLostAnchor => "pond_shape_topology_lost_anchor",
            Self::TopologySedgeAnchor => "pond_shape_topology_sedge_anchor",
            Self::TopologyPopulation => "pond_shape_topology_population",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PondShapeObservation {
    pub columns: u8,
    pub rows: u8,
    pub cells: Vec<u16>,
    pub protected_row: u8,
    pub cell_area_cm2: f64,
}

/// A horizontal footprint contributes area to one well-mixed basin. Its fixed
/// side-view row is the observation plane used by the existing bed and shrimp.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PondShape {
    cells: Vec<u16>,
    cell_area_cm2: f64,
}

pub(super) fn protected_row(index: usize, basin_count: usize) -> usize {
    if basin_count == 1 { 6 } else { index * 3 + 1 }
}

impl PondShape {
    pub(super) fn seed(protected_row: usize, surface_area_cm2: f64) -> Self {
        let cells = ((protected_row - 1) * COLUMNS..(protected_row + 1) * COLUMNS)
            .map(|index| index as u16).collect::<Vec<_>>();
        Self { cell_area_cm2: surface_area_cm2 / cells.len() as f64, cells }
    }

    pub(super) fn observe(&self, protected_row: usize) -> PondShapeObservation {
        PondShapeObservation {
            columns: COLUMNS as u8,
            rows: ROWS as u8,
            cells: self.cells.clone(),
            protected_row: protected_row as u8,
            cell_area_cm2: self.cell_area_cm2,
        }
    }

    pub(super) fn area(&self) -> f64 { self.cell_area_cm2 * self.cells.len() as f64 }

    pub(super) fn cell_area(&self) -> f64 { self.cell_area_cm2 }

    pub(super) fn cells(&self) -> &[u16] { &self.cells }

    pub(super) fn topology_component(&self, cells: Vec<u16>) -> Self {
        Self { cells, cell_area_cm2: self.cell_area_cm2 }
    }

    pub(super) fn rescale(&mut self, surface_area_cm2: f64) {
        self.cell_area_cm2 = surface_area_cm2 / self.cells.len() as f64;
    }

    pub(super) fn paint(&self, mode: PondShapeMode, cells: &[u16], protected_row: usize, secondary_rows: impl Iterator<Item = u8>)
        -> Result<Self, PondShapeError> {
        use PondShapeError as Error;

        let footprint = self.edited_footprint(mode, cells, protected_row, secondary_rows)?;
        if footprint == self.footprint() { return Err(Error::Unchanged); }
        let cells = footprint.iter().enumerate().filter_map(|(index, &filled)| filled.then_some(index as u16)).collect::<Vec<_>>();
        if !connected(&footprint, protected_row, cells.len()) { return Err(Error::Disconnected); }
        Ok(Self { cells, cell_area_cm2: self.cell_area_cm2 })
    }

    pub(super) fn edited_footprint(&self, mode: PondShapeMode, cells: &[u16], protected_row: usize, mut secondary_rows: impl Iterator<Item = u8>)
        -> Result<[bool; CELL_COUNT], PondShapeError> {
        use PondShapeError as Error;
        if cells.is_empty() || cells.len() > CELL_COUNT || cells.iter().any(|&cell| usize::from(cell) >= CELL_COUNT) {
            return Err(Error::InvalidCells);
        }
        if mode == PondShapeMode::Remove && cells.iter().any(|&cell| usize::from(cell) / COLUMNS == protected_row) {
            return Err(Error::ProtectedRow);
        }
        if mode == PondShapeMode::Remove && secondary_rows.any(|row| {
            cells.iter().any(|&cell| usize::from(cell) / COLUMNS == usize::from(row))
        }) {
            return Err(Error::TopologySection);
        }
        let mut footprint = self.footprint();
        for &cell in cells { footprint[usize::from(cell)] = mode == PondShapeMode::Add; }
        Ok(footprint)
    }

    pub(super) fn touches(&self, other: &Self) -> bool {
        let other = other.footprint();
        self.cells.iter().copied().map(usize::from).any(|cell| {
            other[cell] || neighbors(cell).any(|adjacent| other[adjacent])
        })
    }

    pub(super) fn valid(&self, protected_row: usize, surface_area_cm2: f64) -> bool {
        if protected_row >= ROWS || !(COLUMNS..=CELL_COUNT).contains(&self.cells.len()) || !self.cell_area_cm2.is_finite() || self.cell_area_cm2 <= 0.0
            || (self.area() - surface_area_cm2).abs() > 1e-8 || self.cells.iter().any(|&cell| usize::from(cell) >= CELL_COUNT)
            || !self.cells.windows(2).all(|pair| pair[0] < pair[1])
        {
            return false;
        }
        let footprint = self.footprint();
        let protected = protected_row * COLUMNS;
        footprint[protected..protected + COLUMNS].iter().all(|&filled| filled)
            && connected(&footprint, protected_row, self.cells.len())
    }

    pub(super) fn footprint(&self) -> [bool; CELL_COUNT] {
        let mut footprint = [false; CELL_COUNT];
        for &cell in &self.cells { footprint[usize::from(cell)] = true; }
        footprint
    }
}

pub(super) fn components(mut footprint: [bool; CELL_COUNT]) -> Vec<Vec<u16>> {
    let mut components = Vec::new();
    for first in 0..CELL_COUNT {
        if !footprint[first] { continue; }
        footprint[first] = false;
        let mut component = vec![first];
        let mut head = 0;
        while head < component.len() {
            let cell = component[head];
            head += 1;
            for neighbor in neighbors(cell) {
                if footprint[neighbor] {
                    footprint[neighbor] = false;
                    component.push(neighbor);
                }
            }
        }
        component.sort_unstable();
        components.push(component.into_iter().map(|cell| cell as u16).collect());
    }
    components
}

pub(super) fn contains_row(cells: &[u16], row: usize) -> bool {
    let start = row * COLUMNS;
    (start..start + COLUMNS).all(|cell| cells.binary_search(&(cell as u16)).is_ok())
}

fn neighbors(cell: usize) -> impl Iterator<Item = usize> {
    [
        (cell % COLUMNS > 0).then(|| cell - 1),
        (cell % COLUMNS < COLUMNS - 1).then_some(cell + 1),
        (cell >= COLUMNS).then(|| cell - COLUMNS),
        (cell + COLUMNS < CELL_COUNT).then_some(cell + COLUMNS),
    ].into_iter().flatten()
}

fn connected(footprint: &[bool; CELL_COUNT], protected_row: usize, count: usize) -> bool {
    let start = protected_row * COLUMNS;
    let mut visited = [false; CELL_COUNT];
    visited[start] = true;
    let mut pending = [0; CELL_COUNT];
    pending[0] = start;
    let mut tail = 1;
    let mut head = 0;
    while head < tail {
        for cell in neighbors(pending[head]) {
            if footprint[cell] && !visited[cell] {
                visited[cell] = true;
                pending[tail] = cell;
                tail += 1;
            }
        }
        head += 1;
    }
    tail == count
}

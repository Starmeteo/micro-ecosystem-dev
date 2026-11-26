use super::*;
use super::section::MAX_OBSERVATION_SECTIONS;
use pond_shape::{CELL_COUNT, ROWS, components, contains_row};

type Error = PondShapeError;

impl PondState {
    pub(super) fn valid_basin_identities(&self, permits_topology: bool) -> bool {
        let Some(next) = self.next_basin_number else {
            return self.basins.iter().enumerate().all(|(index, basin)| {
                basin.id == basin_id(index) && basin.observation_row.is_none()
            });
        };
        permits_topology && next >= 2 && self.basins.iter().enumerate().all(|(index, basin)| {
            let number = if basin.id == POND_DEFAULT_BASIN_ID { Some(1) } else {
                basin.id.strip_prefix("pond-").and_then(|value| value.parse::<u64>().ok())
                    .filter(|&value| value >= 2 && basin.id == format!("pond-{value}"))
            };
            number.is_some_and(|number| number < next) && basin.shape.is_some()
                && basin.observation_row.is_some_and(|row| usize::from(row) < ROWS)
                && !self.basins[..index].iter().any(|previous| previous.id == basin.id)
        })
    }

    pub(crate) fn retain_topology_allocator(&mut self, current: &Self) {
        if let Some(next) = current.next_basin_number {
            self.freeze_identities();
            self.next_basin_number = self.next_basin_number.map(|prior| prior.max(next));
        }
        self.preserve_static_section_allocations(current);
    }

    fn freeze_identities(&mut self) {
        if self.next_basin_number.is_some() { return; }
        let count = self.basins.len();
        for (index, basin) in self.basins.iter_mut().enumerate() {
            let row = basin.observation_row(index, count);
            basin.observation_row = Some(row as u8);
            basin.shape.get_or_insert_with(|| PondShape::seed(row, basin.surface_area_cm2));
        }
        self.next_basin_number = Some(count as u64 + 1);
    }

    pub(super) fn commit_topology(&mut self, index: usize, mode: PondShapeMode, cells: &[u16]) -> Result<(), CoreError> {
        if self.basins[index].scenery_layout.is_none() { return Err(CoreError::WrongWorldKind); }
        let mut staged = self.clone();
        staged.freeze_identities();
        let selected = &staged.basins[index];
        let row = selected.observation_row(index, staged.basins.len());
        let shape = selected.effective_shape(row);
        let secondary_rows = selected.static_sections.iter().map(|section| section.row);
        let footprint = shape.edited_footprint(mode, cells, row, secondary_rows).map_err(CoreError::PondShape)?;
        match mode {
            PondShapeMode::Add => staged.merge(index, row, footprint),
            PondShapeMode::Remove => staged.split(index, row, footprint),
        }.map_err(CoreError::PondShape)?;
        staged.validate(true, true, true, true)?;
        *self = staged;
        Ok(())
    }

    fn merge(&mut self, index: usize, row: usize, mut footprint: [bool; CELL_COUNT]) -> Result<(), Error> {
        let old_footprints = self.basins.iter().map(|basin| basin.shape.as_ref().unwrap().footprint()).collect::<Vec<_>>();
        let added = std::array::from_fn::<_, CELL_COUNT, _>(|cell| {
            footprint[cell] && !old_footprints.iter().any(|old| old[cell])
        });
        for old in &old_footprints {
            for (occupied, previous) in footprint.iter_mut().zip(old) { *occupied |= *previous; }
        }
        let combined_cells = components(footprint).into_iter().find(|cells| contains_row(cells, row))
            .ok_or(Error::TopologyOperation)?;
        if added.iter().enumerate().any(|(cell, &new)| new && combined_cells.binary_search(&(cell as u16)).is_err()) {
            return Err(Error::Disconnected);
        }
        let participants = old_footprints.iter().enumerate().filter_map(|(number, old)| {
            combined_cells.iter().any(|&cell| old[usize::from(cell)]).then_some(number)
        }).collect::<Vec<_>>();
        if participants.len() < 2 { return Err(Error::TopologyOperation); }
        let sources = participants.iter().map(|&number| &self.basins[number]).collect::<Vec<_>>();
        let keep = &self.basins[index];
        let cell_area = keep.shape.as_ref().unwrap().cell_area();
        if sources.iter().any(|source| (source.shape.as_ref().unwrap().cell_area() - cell_area).abs() > 1e-8) {
            return Err(Error::TopologyScale);
        }
        let shape = keep.shape.as_ref().unwrap().topology_component(combined_cells);
        let mut merged = keep.clone();
        for &number in &participants {
            if number == index { continue; }
            let source = &self.basins[number];
            let section = PondStaticSection::from_basin(source);
            let rooted = section.as_ref().and_then(|section| section.water_plants.as_ref())
                .map_or(Matter::default(), |plants| plants.stock);
            let unassigned = Matter(std::array::from_fn(|element| source.plants.0[element] - rooted.0[element]));
            merged.static_sections.extend(section);
            merged.static_sections.extend_from_slice(&source.static_sections);
            merged.add_free_stocks(source, 1.0);
            merged.topology_add_unassigned_plants(unassigned);
            merged.daphnia.merge_population(&source.daphnia)?;
            if let Some(layout) = source.floating_plant_layout.as_ref().filter(|layout| layout.has_patches()) {
                merged.floating_plant_layout.get_or_insert_with(FloatingPlantLayout::new).merge_from(layout)?;
            }
            if let Some(layout) = &source.sedge_layout {
                merged.sedge_layout.get_or_insert_with(SedgeLayout::new).merge_from(layout)?;
            }
        }
        if merged.static_sections.len() >= MAX_OBSERVATION_SECTIONS { return Err(Error::TopologySectionLimit); }
        merged.static_sections.sort_unstable_by_key(|section| section.row);
        let area_weighted = merged.water_liters == 0.0;
        let weight = |basin: &Basin| if area_weighted { basin.surface_area_cm2 } else { basin.water_liters };
        let total_weight = sources.iter().map(|source| weight(source)).sum::<f64>();
        merged.temperature_celsius = sources.iter().map(|source| {
            source.temperature_celsius * (weight(source) / total_weight)
        }).sum::<f64>().clamp(4.0, 34.0);
        merged.install_footprint(shape, row)?;
        merged.clear_shrimp_after_depth_change();
        if !merged.valid() { return Err(Error::Occupied); }
        self.basins[index] = merged;
        for &number in participants.iter().rev() {
            if number != index { self.basins.remove(number); }
        }
        Ok(())
    }

    fn split(&mut self, index: usize, row: usize, footprint: [bool; CELL_COUNT]) -> Result<(), Error> {
        let regions = components(footprint);
        if regions.len() < 2 { return Err(Error::TopologyOperation); }
        if self.basins.len() + regions.len() - 1 > MAX_BASINS { return Err(Error::TopologyLimit); }
        let source = &self.basins[index];
        if let Some(layout) = &source.floating_plant_layout {
            let mut remaining_cells = regions.iter().flatten().copied().collect::<Vec<_>>();
            remaining_cells.sort_unstable();
            if !layout.anchor_cells_in(&remaining_cells) { return Err(Error::TopologyLostAnchor); }
        }
        if source.sedge_layout.as_ref().is_some_and(|layout| !layout.anchors_in(&footprint)) {
            return Err(Error::TopologySedgeAnchor);
        }
        let total_cells = regions.iter().map(Vec::len).sum::<usize>();
        let populations = apportion_population(source.daphnia.count, &regions, total_cells);
        let original_inventory = source.inventory();
        let located_before = source.located_leaf_carbon();
        let mut next = self.next_basin_number.unwrap();
        let mut children = Vec::new();
        let mut retained = None;
        for (region, population) in regions.into_iter().zip(populations) {
            let keep = contains_row(&region, row);
            let mut sections = source.static_sections.iter().filter(|section| {
                contains_row(&region, usize::from(section.row))
            }).cloned().collect::<Vec<_>>();
            let promoted = (!keep && !sections.is_empty()).then(|| sections.remove(0));
            let primary_leaf_carbon = if keep {
                source.leaf_deposit_layout.as_ref().map_or(0.0, PondLeafLayout::located_carbon)
            } else {
                promoted.as_ref().map_or(0.0, PondStaticSection::located_leaf_carbon)
            };
            let located_here = primary_leaf_carbon + sections.iter().map(PondStaticSection::located_leaf_carbon).sum::<f64>();
            let section = if keep {
                row
            } else if let Some(section) = &promoted {
                usize::from(section.row)
            } else {
                (0..ROWS).find(|&row| contains_row(&region, row)).ok_or(Error::TopologySection)?
            };
            let fraction = region.len() as f64 / total_cells as f64;
            let shape = source.shape.as_ref().unwrap().topology_component(region);
            let mut child = source.clone();
            child.static_sections = sections;
            child.scale_free_stocks(fraction, keep, located_before, located_here);
            child.daphnia = source.daphnia.population_part(population);
            if let Some(layout) = &mut child.floating_plant_layout { layout.retain_in(&shape); }
            if let Some(layout) = &mut child.sedge_layout { layout.retain_in(&shape); }
            child.install_footprint(shape, section)?;
            if !keep {
                child.id = format!("pond-{next}");
                child.random = RandomState::new(source.random.state ^ next.wrapping_mul(0x9e37_79b9_7f4a_7c15));
                child.clear_section_layout();
                if let Some(section) = promoted { section.install_primary(&mut child); }
                next = next.checked_add(1).ok_or(Error::TopologyLimit)?;
            }
            child.clear_shrimp_after_depth_change();
            let mut expected = Inventory::default();
            expected.add_scaled(original_inventory, fraction);
            child.record_topology_transfer(expected);
            if !child.valid() { return Err(Error::Occupied); }
            if keep {
                retained = Some(child);
            } else {
                children.push(child);
            }
        }
        self.basins[index] = retained.ok_or(Error::ProtectedRow)?;
        self.basins.extend(children);
        self.next_basin_number = Some(next);
        Ok(())
    }
}

fn apportion_population(count: u32, regions: &[Vec<u16>], total_cells: usize) -> Vec<u32> {
    let mut shares = Vec::with_capacity(regions.len());
    let mut priorities = Vec::with_capacity(regions.len());
    for (index, region) in regions.iter().enumerate() {
        let exact_numerator = count as usize * region.len();
        shares.push((exact_numerator / total_cells) as u32);
        priorities.push((std::cmp::Reverse(exact_numerator % total_cells), index));
    }
    priorities.sort_unstable();
    let remaining = count - shares.iter().sum::<u32>();
    for (_, index) in priorities.into_iter().take(remaining as usize) {
        shares[index] += 1;
    }
    shares
}

impl Daphnia {
    fn population_part(&self, count: u32) -> Self {
        if count == 0 { return Self::default(); }
        let fraction = f64::from(count) / f64::from(self.count);
        Self {
            count,
            body: self.body.scaled(fraction),
            reserve: self.reserve.scaled(fraction),
            birth_credit: self.birth_credit * fraction,
            mortality_credit: self.mortality_credit * fraction,
        }
    }

    fn merge_population(&mut self, source: &Self) -> Result<(), Error> {
        let count = self.count + source.count;
        if count > MAX_DAPHNIA { return Err(Error::TopologyPopulation); }
        if source.count == 0 { return Ok(()); }
        self.count = count;
        self.body.add(source.body);
        self.reserve.add(source.reserve);
        // Carry accumulated fractions up to the existing limits. Geometry edits never
        // trigger births or deaths; the next normal ecology step applies those rules.
        self.birth_credit = (self.birth_credit + source.birth_credit).min(1.0);
        self.mortality_credit = (self.mortality_credit + source.mortality_credit).min(1.0 - f64::EPSILON);
        Ok(())
    }
}

impl Basin {
    fn clear_section_layout(&mut self) {
        self.shrimp.clear();
        self.shrimp_namespace = None;
        self.next_shrimp_id = 1;
        let stone_rest_enabled = self.stone_layout.as_ref().is_some_and(|layout| layout.stone_rest_enabled);
        self.stone_layout = Some(PondStoneLayout::new(stone_rest_enabled));
        if let Some(layout) = &mut self.stone_biofilm { layout.clear(); }
        self.wood_layout = Some(PondWoodLayout::new());
        self.moss_layout = Some(PondMossLayout::new());
        self.scenery_layout = Some(PondSceneryLayout::empty());
    }

    fn install_footprint(&mut self, shape: PondShape, row: usize) -> Result<(), Error> {
        if self.sedge_layout.as_ref().is_some_and(|layout| !layout.anchors_valid(&shape)) {
            return Err(Error::TopologySedgeAnchor);
        }
        self.surface_area_cm2 = shape.area();
        if !(MIN_AREA_CM2..=MAX_AREA_CM2).contains(&self.surface_area_cm2) { return Err(Error::AreaOutOfRange); }
        if self.water_liters > self.capacity() { return Err(Error::Capacity); }
        if self.water_liters > 0.0 && self.water_liters < MIN_WATER_LITERS { return Err(Error::TopologyShallow); }
        self.observation_row = Some(row as u8);
        self.shape = Some(shape);
        Ok(())
    }

    pub(super) fn scale_free_stocks(&mut self, fraction: f64, retain_section: bool, located_before: f64, located_here: f64) {
        self.water_liters *= fraction;
        self.oxygen_mg *= fraction;
        for stock in [&mut self.minerals, &mut self.algae, &mut self.biofilm, &mut self.microbes] {
            *stock = stock.scaled(fraction);
        }
        self.topology_split_section_stocks(fraction, retain_section, located_before, located_here);
        let mut balance = Balance::default();
        balance.add_scaled(&self.balance, fraction);
        self.balance = balance;
    }

    pub(super) fn add_free_stocks(&mut self, source: &Self, fraction: f64) {
        self.water_liters += source.water_liters * fraction;
        self.oxygen_mg += source.oxygen_mg * fraction;
        for (stock, addition) in [(&mut self.minerals, source.minerals), (&mut self.algae, source.algae),
            (&mut self.biofilm, source.biofilm), (&mut self.detritus, source.detritus), (&mut self.microbes, source.microbes)] {
            stock.add(addition.scaled(fraction));
        }
        self.balance.add_scaled(&source.balance, fraction);
    }
}

#[cfg(test)]
mod tests;

impl Inventory {
    fn add_scaled(&mut self, source: Self, fraction: f64) {
        self.water_liters += source.water_liters * fraction;
        self.oxygen_mg += source.oxygen_mg * fraction;
        self.cn_p.add(source.cn_p.scaled(fraction));
    }
}

impl Balance {
    fn add_scaled(&mut self, source: &Self, fraction: f64) {
        self.initial.add_scaled(source.initial, fraction);
        self.inputs.add_scaled(source.inputs, fraction);
        self.outputs.add_scaled(source.outputs, fraction);
        self.topology_matter_transfer.add(source.topology_matter_transfer.scaled(fraction));
        for (total, addition) in [
            (&mut self.photosynthesis_oxygen_mg, source.photosynthesis_oxygen_mg),
            (&mut self.producer_respiration_oxygen_mg, source.producer_respiration_oxygen_mg),
            (&mut self.animal_respiration_oxygen_mg, source.animal_respiration_oxygen_mg),
            (&mut self.microbial_respiration_oxygen_mg, source.microbial_respiration_oxygen_mg),
            (&mut self.gas_import_oxygen_mg, source.gas_import_oxygen_mg),
            (&mut self.gas_export_oxygen_mg, source.gas_export_oxygen_mg),
            (&mut self.shrimp_food_mg_c, source.shrimp_food_mg_c),
            (&mut self.daphnia_food_mg_c, source.daphnia_food_mg_c),
            (&mut self.decomposed_mg_c, source.decomposed_mg_c),
        ] { *total += addition * fraction; }
    }
}

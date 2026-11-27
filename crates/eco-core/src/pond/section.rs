use super::*;

pub(super) const MAX_OBSERVATION_SECTIONS: usize = 4;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PondStaticSectionObservation {
    pub row: u8,
    pub stones: Vec<PondStone>,
    pub woods: Vec<PondWood>,
    pub scenery: Vec<PondScenery>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leaf_deposits: Option<Vec<PondLeafDepositObservation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub water_plant_patches: Option<Vec<PondWaterPlantObservation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moss_patches: Option<Vec<PondMossPatchObservation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moss_presets: Option<[PondMossPresetObservation; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub biofilm_distribution: Option<PondBiofilmDistribution>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stone_rest_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stone_biofilm_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stone_biofilm: Option<Vec<PondStoneBiofilmObservation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wood_biofilm_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wood_biofilm: Option<Vec<PondWoodBiofilmObservation>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub shrimp: Vec<Animal>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub shrimp_life: Vec<PondShrimpLifeObservation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shrimp_attachments: Option<Vec<PondShrimpAttachment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stone_attachments: Option<Vec<PondShrimpStoneAttachment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wood_contact_feeding: Option<Vec<PondShrimpWoodFeedingObservation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stone_contact_feeding: Option<Vec<PondShrimpStoneFeedingObservation>>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct SectionPlants {
    pub(super) layout: PondPlantLayout,
    pub(super) stock: Matter,
}

/// Localized objects anchored to an additional full observation row. Water,
/// oxygen, mineral nutrients and detritus reactions remain basin-wide.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PondStaticSection {
    pub(super) row: u8,
    pub(super) stones: PondStoneLayout,
    pub(super) woods: PondWoodLayout,
    scenery: PondSceneryLayout,
    pub(super) stone_biofilm_enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) leaf_deposit_layout: Option<PondLeafLayout>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) water_plants: Option<SectionPlants>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) moss_layout: Option<PondMossLayout>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) stone_biofilm: Option<Vec<StoneBiofilm>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) shrimp: Vec<Shrimp>,
    #[serde(default = "first_shrimp_id", skip_serializing_if = "is_first_shrimp_id")]
    pub(super) next_shrimp_id: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) random: Option<RandomState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) shrimp_namespace: Option<String>,
}

fn first_shrimp_id() -> u64 { 1 }

fn is_first_shrimp_id(value: &u64) -> bool { *value == first_shrimp_id() }

impl PondStaticSection {
    pub(super) fn shrimp_namespace_for<'a>(&'a self, basin_id: &str) -> Cow<'a, str> {
        match &self.shrimp_namespace {
            Some(namespace) => Cow::Borrowed(namespace),
            None => Cow::Owned(format!("{basin_id}-row-{}", self.row)),
        }
    }

    pub(super) fn from_basin(basin: &Basin) -> Option<Self> {
        let stones = basin.stone_layout.as_ref()?;
        let woods = basin.wood_layout.as_ref()?;
        let scenery = basin.scenery_layout.as_ref()?;
        let has_shrimp_history = !basin.shrimp.is_empty() || basin.next_shrimp_id > 1;
        if stones.stones.is_empty() && woods.woods.is_empty() && scenery.items.is_empty() && !has_shrimp_history
            && !basin.moss_layout.as_ref().is_some_and(|layout| !layout.patches.is_empty())
            && !basin.leaf_deposit_layout.as_ref().is_some_and(PondLeafLayout::has_deposits)
            && !basin.water_plant_layout.as_ref().is_some_and(PondPlantLayout::has_patches)
        {
            return None;
        }
        Some(Self {
            row: basin.observation_row?,
            stones: stones.clone(),
            woods: woods.clone(),
            scenery: scenery.clone(),
            stone_biofilm_enabled: basin.stone_biofilm.is_some(),
            leaf_deposit_layout: basin.leaf_deposit_layout.clone(),
            water_plants: basin.water_plant_layout.as_ref().map(|plants| {
                let (layout, stock) = plants.rooted_section(basin.plants);
                SectionPlants { layout, stock }
            }),
            moss_layout: basin.moss_layout.clone(),
            stone_biofilm: basin.stone_biofilm.clone(),
            shrimp: basin.shrimp.clone(),
            next_shrimp_id: basin.next_shrimp_id,
            random: has_shrimp_history.then(|| basin.random.clone()),
            shrimp_namespace: has_shrimp_history.then(|| basin.shrimp_namespace().to_owned()),
        })
    }

    pub(super) fn observe(&self, basin: &Basin, turbidity: f64) -> PondStaticSectionObservation {
        let column = basin.moss_column_px();
        let moss_patches = self.moss_layout.as_ref().map(|layout| {
            layout.patches.iter().filter_map(|patch| {
                let support = pond_moss::support(patch.parent, patch.offset, Some(&self.stones), Some(&self.woods)).ok()?;
                Some(patch.observe(support, column))
            }).collect()
        });
        let enabled = self.moss_layout.is_some() || self.stone_biofilm.is_some()
            || self.woods.wood_biofilm.is_some() || !self.shrimp.is_empty();
        PondStaticSectionObservation {
            row: self.row,
            stones: self.stones.stones.clone(),
            woods: self.woods.woods.clone(),
            scenery: self.scenery.items.clone(),
            leaf_deposits: self.leaf_deposit_layout.as_ref().and_then(PondLeafLayout::observe),
            water_plant_patches: self.water_plants.as_ref().filter(|plants| plants.layout.has_patches())
                .map(|plants| plants.layout.observe(plants.stock, basin.wet(), basin.depth_cm(), turbidity).0),
            moss_presets: moss_patches.as_ref().map(|_| PondMossPresetObservation::choices()),
            moss_patches,
            biofilm_distribution: enabled.then(|| self.biofilm_distribution(basin.biofilm, column)),
            stone_rest_enabled: (enabled && self.stones.stone_rest_enabled).then_some(true),
            stone_biofilm_enabled: self.stone_biofilm.is_some().then_some(true),
            stone_biofilm: self.stone_biofilm_observation(column),
            wood_biofilm_enabled: self.woods.wood_biofilm.is_some().then_some(true),
            wood_biofilm: self.woods.wood_biofilm_observation(column),
            shrimp: self.shrimp.iter().map(|shrimp| shrimp.view.clone()).collect(),
            shrimp_life: self.shrimp.iter().map(Shrimp::observe_life).collect(),
            shrimp_attachments: (!self.woods.attachments.is_empty()).then(|| self.woods.attachments.clone()),
            stone_attachments: (!self.stones.stone_attachments.is_empty()).then(|| self.stones.stone_attachments.clone()),
            wood_contact_feeding: None,
            stone_contact_feeding: None,
        }
    }

    pub(super) fn located_leaf_carbon(&self) -> f64 {
        self.leaf_deposit_layout.as_ref().map_or(0.0, PondLeafLayout::located_carbon)
    }

    pub(super) fn reset_preset(&mut self, shared_biofilm: &mut Matter) {
        self.shrimp.clear();
        self.shrimp_namespace = None;
        self.next_shrimp_id = 1;
        self.random = None;
        self.stones.stone_attachments.clear();
        self.woods.attachments.clear();
        if let Some(moss) = &mut self.moss_layout { moss.reset_stocks(); }
        if self.stone_biofilm.is_some() { self.initialize_stone_biofilm(shared_biofilm); }
        if self.woods.wood_biofilm.is_some() {
            self.woods.seed_wood_biofilm((self.stone_biofilm.is_none()).then_some(&self.stones), shared_biofilm);
        }
    }

    pub(super) fn install_primary(self, basin: &mut Basin) {
        basin.stone_biofilm = self.stone_biofilm.or_else(|| self.stone_biofilm_enabled.then(|| {
            self.stones.stones.iter().filter(|stone| stone.preset == PondStonePreset::Main)
                .map(|stone| StoneBiofilm::empty(stone.id)).collect()
        }));
        basin.stone_layout = Some(self.stones);
        basin.wood_layout = Some(self.woods);
        basin.moss_layout = Some(self.moss_layout.unwrap_or_else(PondMossLayout::new));
        basin.scenery_layout = Some(self.scenery);
        basin.leaf_deposit_layout = self.leaf_deposit_layout;
        basin.shrimp = self.shrimp;
        basin.next_shrimp_id = self.next_shrimp_id;
        basin.shrimp_namespace = self.shrimp_namespace;
        if let Some(random) = self.random { basin.random = random; }
        if let Some(mut plants) = self.water_plants {
            plants.layout.add_unassigned_stock(plants.stock.carbon(), basin.plants.carbon());
            basin.water_plant_layout = Some(plants.layout);
            basin.plants.add(plants.stock);
        }
    }

    fn valid(&self, basin: &Basin) -> bool {
        let wet = basin.wet();
        (wet || self.shrimp.is_empty())
            && self.random.as_ref().is_none_or(|random| random.state != 0)
            && (self.shrimp_namespace.is_some() || (self.shrimp.is_empty() && self.next_shrimp_id == 1))
            && (self.shrimp.is_empty() || self.random.is_some())
            && Shrimp::valid_population(&self.shrimp, self.shrimp_namespace.as_deref().unwrap_or_default(), self.next_shrimp_id)
            && valid_section_bed(&self.stones, Some(&self.woods), &self.shrimp, basin.column_projection_px())
            && (!self.stone_biofilm_enabled || self.stones.stone_rest_enabled)
            && self.stone_biofilm.as_ref().is_none_or(|stocks| self.stone_biofilm_enabled && StoneBiofilm::validate(stocks, Some(&self.stones)))
            && self.moss_layout.as_ref().is_none_or(|layout| layout.valid(Some(&self.stones), Some(&self.woods)))
            && self.scenery.valid(|id| self.woods.woods.iter().any(|wood| wood.id == id))
            && self.leaf_deposit_layout.as_ref().is_none_or(|layout| layout.valid(basin.detritus.carbon()))
            && self.water_plants.as_ref().is_none_or(|plants| {
                plants.layout.valid_section(plants.stock, self.stones.profiles().chain(self.woods.profiles()))
            })
    }
}

#[derive(Clone, Copy)]
struct SectionAllocations<'a> {
    stones: &'a PondStoneLayout,
    woods: &'a PondWoodLayout,
    scenery: &'a PondSceneryLayout,
    leaves: Option<&'a PondLeafLayout>,
    plants: Option<&'a PondPlantLayout>,
    moss: Option<&'a PondMossLayout>,
    shrimp_namespace: Option<&'a str>,
    next_shrimp_id: u64,
}

impl PondState {
    pub(super) fn preserve_static_section_allocations(&mut self, current: &Self) {
        if !self.basins.iter().chain(&current.basins).any(|basin| !basin.static_sections.is_empty()) {
            return;
        }
        let mut allocated = [None; pond_shape::ROWS];
        for (index, basin) in current.basins.iter().enumerate() {
            let row = basin.observation_row(index, current.basins.len());
            if let (Some(slot), Some(stones), Some(woods), Some(scenery)) =
                (allocated.get_mut(row), &basin.stone_layout, &basin.wood_layout, &basin.scenery_layout)
            {
                *slot = Some(SectionAllocations {
                    stones, woods, scenery, leaves: basin.leaf_deposit_layout.as_ref(), plants: basin.water_plant_layout.as_ref(),
                    moss: basin.moss_layout.as_ref(), shrimp_namespace: Some(basin.shrimp_namespace()), next_shrimp_id: basin.next_shrimp_id,
                });
            }
            for section in &basin.static_sections {
                if let Some(slot) = allocated.get_mut(usize::from(section.row)) {
                    *slot = Some(SectionAllocations {
                        stones: &section.stones, woods: &section.woods, scenery: &section.scenery,
                        leaves: section.leaf_deposit_layout.as_ref(), plants: section.water_plants.as_ref().map(|plants| &plants.layout),
                        moss: section.moss_layout.as_ref(), shrimp_namespace: section.shrimp_namespace.as_deref(), next_shrimp_id: section.next_shrimp_id,
                    });
                }
            }
        }
        let count = self.basins.len();
        for (index, basin) in self.basins.iter_mut().enumerate() {
            let row = basin.observation_row(index, count);
            let same_namespace = allocated.get(row).and_then(|entry| entry.as_ref()).and_then(|old| old.shrimp_namespace)
                .is_some_and(|namespace| namespace == basin.shrimp_namespace());
            if let (Some(Some(old)), Some(stones), Some(woods), Some(scenery)) =
                (allocated.get(row).copied(), &mut basin.stone_layout, &mut basin.wood_layout, &mut basin.scenery_layout)
            {
                stones.preserve_allocations(old.stones);
                woods.preserve_allocations(old.woods);
                scenery.preserve_allocations(old.scenery);
                PondLeafLayout::preserve_optional_allocations(&mut basin.leaf_deposit_layout, old.leaves);
                if let Some(old_plants) = old.plants {
                    basin.water_plant_layout.get_or_insert_with(|| PondPlantLayout::new(basin.plants.carbon()))
                        .preserve_allocations(old_plants);
                }
                if let Some(moss) = old.moss {
                    basin.moss_layout.get_or_insert_with(PondMossLayout::new).preserve_allocations(moss);
                }
                if same_namespace { basin.next_shrimp_id = basin.next_shrimp_id.max(old.next_shrimp_id); }
            }
            for section in &mut basin.static_sections {
                if let Some(Some(old)) = allocated.get(usize::from(section.row)).copied() {
                    section.stones.preserve_allocations(old.stones);
                    section.woods.preserve_allocations(old.woods);
                    section.scenery.preserve_allocations(old.scenery);
                    PondLeafLayout::preserve_optional_allocations(&mut section.leaf_deposit_layout, old.leaves);
                    if let Some(plants) = old.plants {
                        section.water_plants.get_or_insert_with(|| SectionPlants { layout: PondPlantLayout::new(0.0), stock: Matter::default() })
                            .layout.preserve_allocations(plants);
                    }
                    if let Some(moss) = old.moss {
                        section.moss_layout.get_or_insert_with(PondMossLayout::new).preserve_allocations(moss);
                    }
                    if let Some(namespace) = old.shrimp_namespace.filter(|_| old.next_shrimp_id > first_shrimp_id()) {
                        let previous = section.shrimp_namespace.get_or_insert_with(|| namespace.to_owned());
                        if previous == namespace { section.next_shrimp_id = section.next_shrimp_id.max(old.next_shrimp_id); }
                    }
                }
            }
        }
    }
}

impl Basin {
    pub(super) fn static_sections_valid(&self) -> bool {
        if self.static_sections.is_empty() { return true; }
        let (Some(main), Some(shape)) = (self.observation_row, &self.shape) else { return false; };
        let detritus_carbon = self.detritus.carbon();
        self.static_sections.len() < MAX_OBSERVATION_SECTIONS
            && self.static_sections.windows(2).all(|pair| pair[0].row < pair[1].row)
            && self.static_sections.iter().all(|section| {
                section.row != main && usize::from(section.row) < pond_shape::ROWS
                    && pond_shape::contains_row(shape.cells(), usize::from(section.row)) && section.valid(self)
            })
            && PondLeafLayout::fits_pool(self.located_leaf_carbon(), detritus_carbon)
    }

    pub(super) fn edit_static_section(&mut self, row: u8, edit: &PondEdit, climate: SeasonObservation) -> Result<(), CoreError> {
        if edit.basin_id() != self.id || !matches!(edit,
            PondEdit::AddStone { .. } | PondEdit::MoveStone { .. } | PondEdit::RemoveStone { .. }
                | PondEdit::AddWood { .. } | PondEdit::MoveWood { .. } | PondEdit::RemoveWood { .. }
                | PondEdit::AddScenery { .. } | PondEdit::MoveScenery { .. } | PondEdit::RemoveScenery { .. }
                | PondEdit::AddLeafDeposit { .. } | PondEdit::MoveLeafDeposit { .. } | PondEdit::RemoveLeafDeposit { .. }
                | PondEdit::AddWaterPlant { .. } | PondEdit::PlaceRibbonPlant { .. } | PondEdit::MoveWaterPlant { .. }
                | PondEdit::TrimWaterPlant { .. } | PondEdit::RemoveWaterPlant { .. }
                | PondEdit::AddMoss { .. } | PondEdit::MoveMoss { .. } | PondEdit::TrimMoss { .. } | PondEdit::RemoveMoss { .. }
                | PondEdit::EnableStoneRest { .. } | PondEdit::EnableStoneBiofilm { .. } | PondEdit::EnableWoodBiofilm { .. }
                | PondEdit::StockShrimp { .. } | PondEdit::RemoveShrimp { .. }
        ) {
            return Err(CoreError::InvalidPondStaticSectionEdit);
        }
        let section = self.static_sections.iter_mut().find(|section| section.row == row)
            .ok_or(CoreError::PondStaticSectionMissing)?;

        match *edit {
            PondEdit::AddLeafDeposit { x, preset, mg_c, .. } => {
                let dose = PondLeafLayout::add(&mut section.leaf_deposit_layout, x, preset, mg_c)?;
                self.record_detritus_input(dose);
                return Ok(());
            }
            PondEdit::MoveLeafDeposit { id, x, .. } => {
                return section.leaf_deposit_layout.as_mut().ok_or(CoreError::PondLeafDepositMissing)?.move_deposit(id, x);
            }
            PondEdit::RemoveLeafDeposit { id, .. } => {
                let layout = section.leaf_deposit_layout.as_mut().ok_or(CoreError::PondLeafDepositMissing)?;
                let removed = layout.remove_deposit(id, &mut self.detritus)?;
                self.balance.outputs.cn_p.add(removed);
                return Ok(());
            }
            PondEdit::RemoveShrimp { ref id, .. } => {
                let outgoing = remove_shrimp(&mut section.shrimp, Some(&mut section.stones), Some(&mut section.woods), id)?;
                self.balance.outputs.cn_p.add(outgoing);
                return Ok(());
            }
            _ => {}
        }

        // Project the layout at the shared basin depth to reuse ordinary geometry
        // checks. This temporary basin never advances or contributes to inventory.
        let mut scratch = Basin::new(self.id.clone(), 1);
        scratch.clear_initial_stocks();
        scratch.surface_area_cm2 = self.surface_area_cm2;
        scratch.water_liters = self.water_liters;
        scratch.stone_layout = Some(section.stones.clone());
        scratch.wood_layout = Some(section.woods.clone());
        scratch.scenery_layout = Some(section.scenery.clone());
        scratch.moss_layout = section.moss_layout.clone().or_else(|| Some(PondMossLayout::new()));
        scratch.stone_biofilm = section.stone_biofilm.clone();
        scratch.biofilm = self.biofilm;
        scratch.shrimp.clone_from(&section.shrimp);
        scratch.next_shrimp_id = section.next_shrimp_id;
        let stock_shrimp = matches!(edit, PondEdit::StockShrimp { .. });
        scratch.shrimp_namespace = section.shrimp_namespace.clone().or_else(|| {
            stock_shrimp.then(|| section.shrimp_namespace_for(&self.id).into_owned())
        });
        scratch.random = section.random.clone().unwrap_or_else(|| {
            RandomState::new(self.random.state ^ (u64::from(section.row) + 1).wrapping_mul(0x9e37_79b9_7f4a_7c15))
        });
        if let Some(plants) = &section.water_plants {
            scratch.water_plant_layout = Some(plants.layout.clone());
            scratch.plants = plants.stock;
        }
        let mut draft = PondState { basins: vec![scratch], next_basin_number: None };
        draft.edit(edit, climate)?;
        let updated = draft.basins.pop().unwrap();
        section.stones = updated.stone_layout.unwrap();
        section.woods = updated.wood_layout.unwrap();
        section.scenery = updated.scenery_layout.unwrap();
        section.water_plants = updated.water_plant_layout.map(|layout| SectionPlants { layout, stock: updated.plants });
        if section.moss_layout.is_some() || matches!(edit, PondEdit::AddMoss { .. } | PondEdit::EnableStoneRest { .. }) {
            section.moss_layout = updated.moss_layout;
        }
        section.stone_biofilm = updated.stone_biofilm;
        section.stone_biofilm_enabled |= section.stone_biofilm.is_some();
        section.shrimp = updated.shrimp;
        section.next_shrimp_id = updated.next_shrimp_id;
        section.shrimp_namespace = updated.shrimp_namespace;
        if stock_shrimp || section.random.is_some() { section.random = Some(updated.random); }
        self.biofilm = updated.biofilm;
        self.balance.inputs.cn_p.add(updated.balance.inputs.cn_p);
        self.detritus.add(updated.detritus);
        debug_assert!(self.static_sections_valid());
        Ok(())
    }
}

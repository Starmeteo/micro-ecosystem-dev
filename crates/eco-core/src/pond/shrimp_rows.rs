use super::*;

type FeedingFlags = [bool; MAX_SHRIMP];

pub(super) fn sync_wood_attachments(shrimp: &mut [Shrimp], layout: &mut PondWoodLayout, column: f64) {
    let woods = &layout.woods;
    layout.attachments.retain(|attachment| {
        let Some(individual) = shrimp.iter_mut().find(|shrimp| shrimp.view.id == attachment.shrimp_id) else { return false; };
        let Some(surface) = woods.iter().find(|wood| wood.id == attachment.wood_id)
            .and_then(|wood| wood.surface(attachment.offset, column)) else { return false; };
        individual.view.x = surface.x;
        individual.view.y = surface.y;
        true
    });
}

pub(super) fn project_after_depth_change(shrimp: &mut [Shrimp], stones: &mut PondStoneLayout, mut woods: Option<&mut PondWoodLayout>, column: f64) {
    if let Some(woods) = woods.as_deref_mut() { sync_wood_attachments(shrimp, woods, column); }
    let range = if stones.has_cave() { PondSwimRange::near_cave_floor(column) } else { PondSwimRange::EXTENDED };
    let stone_items = &stones.stones;
    let profiles = stone_items.iter().map(|stone| stone.profile()).chain(wood_profiles(woods.as_deref()));
    stones.stone_attachments.retain(|attachment| {
        let Some(individual) = shrimp.iter_mut().find(|shrimp| shrimp.view.id == attachment.shrimp_id) else { return false; };
        let Some(surface) = attachment.point(stone_items, column) else { return false; };
        if profiles.clone().any(|profile| profile.blocks(surface, column)) { return false; }
        individual.view.x = surface.x;
        individual.view.y = surface.y;
        true
    });
    for individual in shrimp {
        let mut point = animal_point(&individual.view);
        if point.y > range.max_y {
            point.y = range.max_y;
            individual.view.y = point.y;
        }
        if let Some(profile) = profiles.clone().find(|profile| profile.blocks(point, column)) {
            let open = profile.clear_after_depth_change(point, column,
                |candidate| range.contains(candidate) && !profiles.clone().any(|obstacle| obstacle.blocks(candidate, column)));
            individual.view.x = open.x;
            individual.view.y = open.y;
        }
    }
}

pub(super) fn retain_contacts(entries: &mut PondBasinContactFeeding, woods: &PondWoodLayout, stones: &PondStoneLayout) {
    entries.wood.retain(|entry| woods.attachments.iter().any(|attachment| {
        entry.shrimp_id == attachment.shrimp_id && entry.wood_id == attachment.wood_id && entry.offset == attachment.offset
    }));
    entries.stone.retain(|entry| stones.stone_attachments.iter().any(|attachment| {
        entry.shrimp_id == attachment.shrimp_id && entry.stone_id == attachment.stone_id && entry.offset == attachment.offset
    }));
}

#[derive(Clone, Copy, Default)]
struct RowFeeding {
    claims: [f64; MAX_SHRIMP],
    stone_food: [Matter; MAX_SHRIMP],
    wood_food: [Matter; MAX_SHRIMP],
}

impl RowFeeding {
    fn prepare(shrimp: &[Shrimp], woods: Option<&mut PondWoodLayout>, stones: Option<&PondStoneLayout>, stocks: Option<&mut [StoneBiofilm]>,
        wet: [f64; MAX_STONES], opportunity: f64, column: f64,
    ) -> Self {
        let mut result = Self::default();
        for (individual, claim) in shrimp.iter().zip(&mut result.claims) {
            let hunger = (1.0 - individual.reserve.carbon() / 7.0).clamp(0.0, 1.0);
            *claim = individual.body.carbon() * 0.095 * hunger * opportunity;
        }
        if let (Some(stones), Some(stocks)) = (stones, stocks) {
            let mut contacts = [None; MAX_SHRIMP];
            for (individual, contact) in shrimp.iter().zip(&mut contacts) {
                *contact = stones.stone_attachments.iter().find(|attachment| attachment.matches(&stones.stones, &individual.view, column));
            }
            result.stone_food = Basin::allocate_stone_food(stocks, &contacts, &mut result.claims, wet);
        }
        if let Some(woods) = woods {
            result.wood_food = woods.allocate_wood_food(shrimp, &mut result.claims, column);
        }
        result
    }
}

struct RowFeedingTargets<'a> {
    shrimp: &'a mut [Shrimp],
    woods: Option<&'a PondWoodLayout>,
    stones: Option<&'a PondStoneLayout>,
    localized: bool,
}

impl RowFeedingTargets<'_> {
    fn receive(self, plan: RowFeeding, shared: Matter, shared_demand: f64, column: f64, detritus: &mut Matter,
        feedback: Option<(&mut PondBasinContactFeeding, PondBiofilmDistribution)>,
    ) -> FeedingFlags {
        let mut observed = [false; MAX_SHRIMP];
        let mut feedback = feedback;
        for (index, (individual, fed)) in self.shrimp.iter_mut().zip(&mut observed).enumerate() {
            let from_shared = if shared_demand > 0.0 { shared.scaled(plan.claims[index] / shared_demand) } else { Matter::default() };
            let from_stone = plan.stone_food[index];
            let from_wood = plan.wood_food[index];
            let mut parcel = from_shared;
            if from_stone.carbon() > 0.0 { parcel.add(from_stone); }
            if from_wood.carbon() > 0.0 { parcel.add(from_wood); }
            individual.reserve.add(parcel.scaled(0.57));
            detritus.add(parcel.scaled(0.43));
            let biofilm_mg_c = parcel.carbon();
            *fed = biofilm_mg_c > 0.0;
            let Some((entries, _)) = feedback.as_mut().filter(|_| *fed) else { continue; };
            let wood_contact = self.woods.and_then(|layout| layout.attachments.iter()
                .find(|attachment| layout.attachment_matches(attachment, &individual.view, column)));
            if let Some(attachment) = wood_contact {
                entries.wood.push(PondShrimpWoodFeedingObservation {
                    shrimp_id: individual.view.id.clone(), wood_id: attachment.wood_id,
                    offset: attachment.offset, biofilm_mg_c, wood_biofilm_mg_c: from_wood.carbon(), shared_biofilm_mg_c: from_shared.carbon(),
                });
            } else if let Some(attachment) = self.stones.and_then(|layout| layout.stone_attachments.iter()
                .find(|attachment| attachment.matches(&layout.stones, &individual.view, column))) {
                entries.stone.push(PondShrimpStoneFeedingObservation {
                    shrimp_id: individual.view.id.clone(), stone_id: attachment.stone_id,
                    offset: attachment.offset, biofilm_mg_c, stone_biofilm_mg_c: from_stone.carbon(),
                    shared_biofilm_mg_c: from_shared.carbon(),
                });
            }
        }
        if let Some((entries, distribution)) = feedback {
            distribution.attribute_contact_intake(entries, self.localized, self.woods.is_some_and(|layout| layout.wood_biofilm.is_some()));
        }
        observed
    }
}

impl Basin {
    pub(super) fn feed_daphnia(&mut self, opportunity: f64) {
        let demand = self.daphnia.count as f64 * 0.024 * opportunity
            * (1.0 - self.daphnia.reserve.carbon() / (self.daphnia.count.max(1) as f64 * 0.15)).clamp(0.0, 1.0);
        let received = self.algae.take_carbon(demand);
        self.daphnia.reserve.add(received.scaled(0.58));
        self.detritus.add(received.scaled(0.42));
        self.balance.daphnia_food_mg_c += received.carbon();
    }

    pub(super) fn feed_all_shrimp_rows(&mut self, oxygen: f64, thermal: f64, mut feedback: Option<&mut PondBasinContactFeeding>)
        -> [FeedingFlags; section::MAX_OBSERVATION_SECTIONS]
    {
        let column = self.column_projection_px();
        let opportunity = oxygen * thermal * STEP_HOURS;
        self.feed_daphnia(opportunity);
        let has_main_contacts = self.wood_layout.as_ref().is_some_and(|layout| !layout.attachments.is_empty())
            || self.stone_layout.as_ref().is_some_and(|layout| !layout.stone_attachments.is_empty());
        let main_distribution = (feedback.is_some() && has_main_contacts).then(|| self.biofilm_distribution());
        let common_before = self.biofilm;
        let mut rows = [RowFeeding::default(); section::MAX_OBSERVATION_SECTIONS];
        let wet = self.stone_biofilm_wet_fractions();
        rows[0] = RowFeeding::prepare(&self.shrimp, self.wood_layout.as_mut(), self.stone_layout.as_ref(),
            self.stone_biofilm.as_deref_mut(), wet, opportunity, column);
        let mut distributions: [_; section::MAX_OBSERVATION_SECTIONS - 1] = std::array::from_fn(|_| None);
        for ((section, row), distribution) in self.static_sections.iter_mut().zip(&mut rows[1..]).zip(&mut distributions) {
            if section.shrimp.is_empty() { continue; }
            if feedback.is_some() && (!section.woods.attachments.is_empty() || !section.stones.stone_attachments.is_empty()) {
                *distribution = Some(section.biofilm_distribution(common_before, column));
            }
            let wet = section.stone_biofilm_wet_fractions(column);
            *row = RowFeeding::prepare(&section.shrimp, Some(&mut section.woods), Some(&section.stones),
                section.stone_biofilm.as_deref_mut(), wet, opportunity, column);
        }
        let shared_demand = rows.iter().flat_map(|row| row.claims).sum::<f64>();
        let shared = self.biofilm.take_carbon(shared_demand);
        let stone = rows.iter().flat_map(|row| row.stone_food).map(Matter::carbon).sum::<f64>();
        let wood = rows.iter().flat_map(|row| row.wood_food).map(Matter::carbon).sum::<f64>();
        self.balance.shrimp_food_mg_c += shared.carbon() + stone + wood;
        let mut fed = [[false; MAX_SHRIMP]; section::MAX_OBSERVATION_SECTIONS];
        fed[0] = RowFeedingTargets { shrimp: &mut self.shrimp, woods: self.wood_layout.as_ref(),
            stones: self.stone_layout.as_ref(), localized: self.stone_biofilm.is_some() }
            .receive(rows[0], shared, shared_demand, column, &mut self.detritus, feedback.as_deref_mut().zip(main_distribution));
        for (((section, plan), observed), distribution) in self.static_sections.iter_mut().zip(&rows[1..]).zip(&mut fed[1..]).zip(distributions) {
            let mut entries = PondBasinContactFeeding::default();
            *observed = RowFeedingTargets { shrimp: &mut section.shrimp, woods: Some(&section.woods),
                stones: Some(&section.stones), localized: section.stone_biofilm.is_some() }
                .receive(*plan, shared, shared_demand, column, &mut self.detritus, distribution.map(|distribution| (&mut entries, distribution)));
            if let Some(feedback) = feedback.as_deref_mut().filter(|_| !entries.is_empty()) {
                feedback.sections.push((section.row, entries));
            }
        }
        fed
    }
}

struct ShrimpHabitat<'a> {
    shrimp: &'a mut Vec<Shrimp>,
    next_id: &'a mut u64,
    namespace: &'a str,
    random: &'a mut RandomState,
    woods: Option<&'a mut PondWoodLayout>,
    stones: Option<&'a mut PondStoneLayout>,
}

impl ShrimpHabitat<'_> {
    fn advance(self, column_px: f64, oxygen: f64, thermal: f64, detritus: &mut Matter, feeding: FeedingFlags) {
        let Self { shrimp, next_id, namespace, random, woods, stones } = self;
        let mut births = Vec::new();
        let id_capacity = (u64::MAX - *next_id).min(MAX_SHRIMP as u64) as usize;
        let capacity_for_births = (MAX_SHRIMP - shrimp.len()).min(id_capacity);
        let mut snapshot_feeding = feeding.into_iter();
        let (woods, mut attachments) = match woods {
            Some(layout) => (layout.woods.as_slice(), Some(&mut layout.attachments)),
            None => (&[][..], None),
        };
        let (stones, mut stone_attachments) = match stones {
            Some(layout) => (layout.stones.as_slice(), layout.stone_rest_enabled.then_some(&mut layout.stone_attachments)),
            None => (&[][..], None),
        };
        let profiles = stones.iter().map(|stone| stone.profile()).chain(woods.iter().map(|wood| wood.profile()));
        let cave = stones.iter().any(|stone| stone.preset == PondStonePreset::FlatStoneCave);
        let blocked = |from, to| profiles.clone().any(|profile| profile.blocks_segment(from, to, column_px));
        shrimp.retain_mut(|individual| {
            let fed = snapshot_feeding.next().unwrap_or(false);
            individual.age_seconds += STEP_SECONDS;
            if oxygen < 0.77 {
                individual.oxygen_stress_seconds += (1.0 - oxygen) * STEP_SECONDS;
            } else {
                individual.oxygen_stress_seconds = (individual.oxygen_stress_seconds - 2.0 * STEP_SECONDS).max(0.0);
            }
            if individual.reserve.carbon() < 0.025 {
                individual.starvation_seconds += STEP_SECONDS;
            } else {
                individual.starvation_seconds = 0.0;
            }
            if individual.oxygen_stress_seconds >= 1200.0 || individual.starvation_seconds >= 18.0 * 3600.0 {
                detritus.add(individual.body);
                detritus.add(individual.reserve);
                return false;
            }
            if individual.stage() == PondShrimpStage::Juvenile {
                let growth = individual.reserve.take_carbon(0.12 * thermal * oxygen * STEP_HOURS);
                individual.body.add(growth);
            } else if oxygen >= 0.8 && individual.reserve.carbon() >= 4.0 {
                individual.birth_credit = (individual.birth_credit + thermal * STEP_HOURS / 36.0).min(1.0);
                if individual.birth_credit >= 1.0 && births.len() < capacity_for_births {
                    let parental = individual.reserve.take_carbon(1.6);
                    births.push((individual.view.x, individual.view.y, individual.view.heading, parental));
                    individual.birth_credit = 0.0;
                }
            }

            let sample = random.unit();
            if attachments.as_deref_mut().is_some_and(|attachments| hold_or_release(attachments, &mut individual.view, oxygen)) {
                return true;
            }
            if stone_attachments.as_deref_mut().is_some_and(|attachments| pond_stones::hold_or_release(attachments, &mut individual.view, oxygen)) {
                return true;
            }
            if oxygen < 0.30 {
                individual.view.activity = Activity::Resting;
            } else if sample < 0.14 {
                individual.view.activity = if fed { Activity::Feeding } else { Activity::Resting };
                if let Some(attachments) = attachments.as_deref_mut() {
                    let before = attachments.len();
                    settle(woods, attachments, &mut individual.view, column_px);
                    if attachments.len() != before { return true; }
                }
                if let Some(attachments) = stone_attachments.as_deref_mut() {
                    pond_stones::settle(stones, attachments, &mut individual.view, column_px, blocked);
                }
            } else {
                let speed = 0.016 * (0.35 + thermal * 0.65) * oxygen * STEP_SECONDS;
                let drift = (random.unit() - 0.5) * 0.44;
                let animal = &mut individual.view;
                let from = animal_point(animal);
                let standard = PondSwimRange::STANDARD;
                // Keep using the bank or bed area until a shrimp returns from it, even if the last wood was removed.
                let range = if cave {
                    PondSwimRange::near_cave_floor(column_px)
                } else if !woods.is_empty() || (attachments.is_some() && !standard.contains(from)) {
                    PondSwimRange::EXTENDED
                } else { standard };
                animal.heading = (animal.heading + drift).rem_euclid(TAU);
                let proposed = Point { x: from.x + animal.heading.cos() * speed, y: from.y + animal.heading.sin() * speed * 0.60 };
                if !range.contains(proposed) {
                    animal.heading = (animal.heading + TAU / 2.0).rem_euclid(TAU);
                }
                let to = range.clamp(proposed);
                if blocked(from, to) {
                    range.turn_shrimp(animal, speed, blocked);
                } else {
                    animal.x = to.x;
                    animal.y = to.y;
                }
                animal.activity = Activity::Foraging;
            }
            true
        });
        if let Some(attachments) = attachments {
            attachments.retain(|attached| shrimp.iter().any(|shrimp| shrimp.view.id == attached.shrimp_id));
        }
        if let Some(attachments) = stone_attachments {
            attachments.retain(|attached| shrimp.iter().any(|shrimp| shrimp.view.id == attached.shrimp_id));
        }
        for (x, y, heading, parental) in births {
            let id = format!("{namespace}-shrimp-{}", *next_id);
            *next_id += 1;
            shrimp.push(Shrimp::new(id, x, y, heading, parental.scaled(0.80), parental.scaled(0.20), 0.0));
        }
    }
}

impl Basin {
    pub(super) fn step_shrimp(&mut self, oxygen: f64, thermal: f64, feeding: FeedingFlags) {
        let column = self.column_projection_px();
        ShrimpHabitat {
            shrimp: &mut self.shrimp, next_id: &mut self.next_shrimp_id,
            namespace: self.shrimp_namespace.as_deref().unwrap_or(&self.id),
            random: &mut self.random, woods: self.wood_layout.as_mut(), stones: self.stone_layout.as_mut(),
        }.advance(column, oxygen, thermal, &mut self.detritus, feeding);
    }

    pub(super) fn step_secondary_shrimp(&mut self, oxygen: f64, thermal: f64, feeding: &[FeedingFlags]) {
        let column = self.column_projection_px();
        for (section, feeding) in self.static_sections.iter_mut().zip(feeding) {
            if section.shrimp.is_empty() { continue; }
            let namespace = section.shrimp_namespace.as_deref().expect("populated shrimp section has a namespace");
            let random = section.random.as_mut().expect("populated shrimp section has a random state");
            ShrimpHabitat {
                shrimp: &mut section.shrimp, next_id: &mut section.next_shrimp_id, namespace,
                random, woods: Some(&mut section.woods), stones: Some(&mut section.stones),
            }.advance(column, oxygen, thermal, &mut self.detritus, *feeding);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wood_film_feeding_spends_only_contacted_stocks_across_sections() {
        let mut world = WorldState::new_blank_r5_composed_dual_pond(2729);
        let main = POND_DEFAULT_BASIN_ID.to_owned();
        for basin_id in [main.clone(), "pond-2".to_owned()] {
            for edit in [
                PondEdit::AddWater { basin_id: basin_id.clone(), liters: 12.0 },
                PondEdit::AddWood { basin_id: basin_id.clone(), x: 0.35, preset: PondWoodPreset::Root },
                PondEdit::StockShrimp { basin_id, position: Point { x: 0.8, y: 0.45 }, count: 2 },
            ] { world.edit_pond(&edit).unwrap(); }
        }
        let merge = |mode| PondEdit::CommitTopology { basin_id: main.clone(), mode, cells: vec![29] };
        world.edit_pond(&merge(PondShapeMode::Add)).unwrap();
        let basin = &mut world.pond.as_mut().unwrap().basins[0];
        let column = basin.column_projection_px();
        let mount = |shrimp: &mut Shrimp, layout: &mut PondWoodLayout, offset| {
            let attachment = PondShrimpAttachment { shrimp_id: shrimp.view.id.clone(), wood_id: 1, offset, rest_ticks: 20 };
            let point = layout.attachment_point(&attachment, column).unwrap();
            shrimp.view.x = point.x;
            shrimp.view.y = point.y;
            shrimp.view.activity = Activity::Resting;
            layout.attachments.push(attachment);
        };
        let woods = basin.wood_layout.as_mut().unwrap();
        mount(&mut basin.shrimp[0], woods, -0.8);
        mount(&mut basin.shrimp[1], woods, 0.8);
        let section = &mut basin.static_sections[0];
        mount(&mut section.shrimp[0], &mut section.woods, 0.0);
        let row = section.row;
        let (main_food, side_food, common) = (0.000_004, 0.000_007, 0.000_010);
        woods.wood_biofilm.as_mut().unwrap()[0].stock = Matter::with_ratios(main_food, 0.077, 0.010);
        section.woods.wood_biofilm.as_mut().unwrap()[0].stock = Matter::with_ratios(side_food, 0.077, 0.010);
        basin.biofilm = Matter::with_ratios(common, 0.077, 0.010);
        basin.balance = Balance { initial: basin.inventory(), ..Balance::default() };
        let mut split = world.clone();

        let basin = &mut world.pond.as_mut().unwrap().basins[0];
        let before = basin.inventory();
        let mut control = basin.clone();
        let mut trace = PondBasinContactFeeding::default();
        let fed = basin.feed_all_shrimp_rows(1.0, 1.0, Some(&mut trace));
        assert_eq!(fed, control.feed_all_shrimp_rows(1.0, 1.0, None));
        assert_eq!(*basin, control);
        assert!(fed[0][0] && fed[0][1] && fed[1][0] && fed[1][1]);
        let near = |got: f64, want: f64| assert!((got - want).abs() < 1e-10, "{got} != {want}");
        let [first, second] = trace.wood.as_slice() else { panic!("expected two primary wood contacts") };
        let [(section_row, incoming)] = trace.sections.as_slice() else { panic!("expected one secondary row") };
        let [third] = incoming.wood.as_slice() else { panic!("unexpected contact for free swimmer") };
        assert_eq!(*section_row, row);
        assert_eq!([first.wood_id, second.wood_id, third.wood_id], [1, 1, 1]);
        near(first.wood_biofilm_mg_c + second.wood_biofilm_mg_c, main_food);
        near(third.wood_biofilm_mg_c, side_food);
        near(basin.balance.shrimp_food_mg_c, main_food + side_food + common);
        for (before, after) in before.cn_p.0.into_iter().zip(basin.inventory().cn_p.0) { near(before, after); }
        assert_eq!(basin.biofilm.carbon(), 0.0);
        world.validate().unwrap();

        split.edit_pond(&merge(PondShapeMode::Remove)).unwrap();
        let parts = &split.pond_observation().unwrap().basins;
        near(parts[0].wood_biofilm.as_ref().unwrap()[0].stock_mg_c, main_food);
        near(parts[1].wood_biofilm.as_ref().unwrap()[0].stock_mg_c, side_food);
        split.validate().unwrap();
    }
    use crate::WorldState;

    #[test]
    fn r5_take_out_shrimp_is_section_scoped_and_clears_its_contact() {
        let mut world = WorldState::new_blank_r5_composed_dual_pond(362);
        let primary = POND_DEFAULT_BASIN_ID.to_owned();
        for id in [primary.as_str(), "pond-2"] {
            let basin_id = id.to_owned();
            for edit in [
                PondEdit::AddWater { basin_id: basin_id.clone(), liters: 12.0 },
                PondEdit::AddStone { basin_id: basin_id.clone(), x: 0.35, preset: PondStonePreset::Main },
                PondEdit::EnableStoneRest { basin_id: basin_id.clone() },
                PondEdit::StockShrimp { basin_id, position: Point { x: 0.78, y: 0.4 }, count: 1 },
            ] { world.edit_pond(&edit).unwrap(); }
        }
        world.edit_pond(&PondEdit::CommitTopology { basin_id: primary.clone(), mode: PondShapeMode::Add, cells: vec![29] }).unwrap();
        let basin = &mut world.pond.as_mut().unwrap().basins[0];
        let column = basin.column_projection_px();
        let section = &mut basin.static_sections[0];
        let shrimp = &mut section.shrimp[0];
        let shrimp_id = shrimp.view.id.clone();
        let contact = PondShrimpStoneAttachment { shrimp_id: shrimp_id.clone(), stone_id: 1, offset: 0.0, rest_ticks: 20 };
        let point = contact.point(&section.stones.stones, column).unwrap();
        shrimp.view.x = point.x;
        shrimp.view.y = point.y;
        section.stones.stone_attachments.push(contact);
        let mut exported = shrimp.body;
        exported.add(shrimp.reserve);
        let row = section.row;
        let before = world.clone();
        let remove = PondEdit::RemoveShrimp { basin_id: primary.clone(), id: shrimp_id };
        assert_eq!(world.edit_pond(&remove), Err(CoreError::PondShrimpMissing));
        assert_eq!(world, before);

        world.edit_pond(&PondEdit::EditStaticSection { basin_id: primary, row, edit: Box::new(remove) }).unwrap();
        let after = world.clone();
        let original = &before.pond.as_ref().unwrap().basins[0];
        let edited = &after.pond.as_ref().unwrap().basins[0];
        assert_eq!(edited.shrimp, original.shrimp);
        assert_eq!(edited.detritus, original.detritus);
        assert!(edited.static_sections[0].shrimp.is_empty());
        assert!(edited.static_sections[0].stones.stone_attachments.is_empty());
        for element in 0..3 {
            assert!((edited.balance.outputs.cn_p.0[element] - original.balance.outputs.cn_p.0[element] - exported.0[element]).abs() < 1e-8);
        }
        world.validate().unwrap();
        world.restore_pond_edit_state(before.clone()).unwrap();
        assert_eq!(world, before);
        world.restore_pond_edit_state(after.clone()).unwrap();
        let loaded: WorldState = serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
        assert_eq!(loaded, after);
        loaded.validate().unwrap();
    }

    #[test]
    fn r5_shrimp_contacts_share_food_once_across_observation_rows() {
        let mut world = WorldState::new_blank_r5_composed_dual_pond(2729);
        let climate = world.season_observation();
        for id in [POND_DEFAULT_BASIN_ID, "pond-2"] {
            let basin_id = id.to_owned();
            for edit in [
                PondEdit::AddWater { basin_id: basin_id.clone(), liters: 12.0 },
                PondEdit::AddStone { basin_id: basin_id.clone(), x: 0.35, preset: PondStonePreset::Main },
                PondEdit::EnableStoneBiofilm { basin_id: basin_id.clone() },
                PondEdit::StockShrimp { basin_id, position: Point { x: 0.78, y: 0.4 }, count: 1 },
            ] { world.edit_pond(&edit).unwrap(); }
        }
        let main = POND_DEFAULT_BASIN_ID.to_owned();
        world.edit_pond(&PondEdit::CommitTopology { basin_id: main.clone(), mode: PondShapeMode::Add, cells: vec![29] }).unwrap();
        let basin = &mut world.pond.as_mut().unwrap().basins[0];
        let depth = basin.column_projection_px();
        let mount = |individual: &mut Shrimp, layout: &mut PondStoneLayout| {
            let attachment = PondShrimpStoneAttachment { shrimp_id: individual.view.id.clone(), stone_id: 1, offset: 0.0, rest_ticks: 20 };
            let point = attachment.point(&layout.stones, depth).unwrap();
            individual.view.x = point.x;
            individual.view.y = point.y;
            layout.stone_attachments.push(attachment);
        };
        let section = &mut basin.static_sections[0];
        mount(&mut basin.shrimp[0], basin.stone_layout.as_mut().unwrap());
        mount(&mut section.shrimp[0], &mut section.stones);
        let (main_food, side_food, common_food) = (0.000_004, 0.000_007, 0.000_010);
        basin.stone_biofilm.as_mut().unwrap()[0].stock = Matter::with_ratios(main_food, 0.077, 0.010);
        section.stone_biofilm.as_mut().unwrap()[0].stock = Matter::with_ratios(side_food, 0.077, 0.010);
        basin.biofilm = Matter::with_ratios(common_food, 0.077, 0.010);
        basin.balance = Balance { initial: basin.inventory(), ..Balance::default() };

        let row = basin.static_sections[0].row;
        let mut secondary_contact = basin.clone();
        secondary_contact.stone_layout.as_mut().unwrap().stone_attachments.clear();
        let mut observed = PondState { basins: vec![secondary_contact], next_basin_number: Some(3) };
        let mut last_step = Vec::new();
        observed.advance(0.5, climate, Some(&mut last_step));
        assert_eq!(last_step.len(), 1);
        assert!(last_step[0].1.stone.is_empty());
        assert_eq!(last_step[0].1.sections[0].0, row);
        assert_eq!(last_step[0].1.sections[0].1.stone.len(), 1);

        let mut expected = basin.clone();
        let before_feed = basin.inventory();
        let mut trace = PondBasinContactFeeding::default();
        let feeding = basin.feed_all_shrimp_rows(1.0, 1.0, Some(&mut trace));
        assert_eq!(feeding, expected.feed_all_shrimp_rows(1.0, 1.0, None));
        assert_eq!(*basin, expected);
        assert!(feeding[0][0] && feeding[1][0]);
        assert_eq!(trace.sections.len(), 1);
        assert_eq!(trace.sections[0].0, row);
        let first = &trace.stone[0];
        let second = &trace.sections[0].1.stone[0];
        assert_eq!([first.stone_id, second.stone_id], [1, 1]);
        assert_ne!(first.shrimp_id, second.shrimp_id);
        let near = |got: f64, want: f64| assert!((got - want).abs() < 1e-10, "{got} != {want}");
        near(first.stone_biofilm_mg_c, main_food);
        near(second.stone_biofilm_mg_c, side_food);
        near(first.shared_biofilm_mg_c + second.shared_biofilm_mg_c, common_food);
        near(basin.balance.shrimp_food_mg_c, main_food + side_food + common_food);
        for (before, after) in before_feed.cn_p.0.into_iter().zip(basin.inventory().cn_p.0) { near(before, after); }
        assert_eq!(basin.biofilm.carbon(), 0.0);
        let age = basin.shrimp[0].age_seconds;
        let side_age = basin.static_sections[0].shrimp[0].age_seconds;
        basin.step_shrimp(1.0, 1.0, feeding[0]);
        basin.step_secondary_shrimp(1.0, 1.0, &feeding[1..]);
        near(basin.shrimp[0].age_seconds, age + STEP_SECONDS);
        near(basin.static_sections[0].shrimp[0].age_seconds, side_age + STEP_SECONDS);
        let main_id = basin.shrimp[0].view.id.clone();
        let side_id = basin.static_sections[0].shrimp[0].view.id.clone();
        world.validate().unwrap();

        let mut dried = world.clone();
        dried.edit_pond(&PondEdit::DrainWater { basin_id: main.clone(), liters: 24.0 }).unwrap();
        let dry = &dried.pond.as_ref().unwrap().basins[0];
        assert!(dry.shrimp.is_empty() && dry.static_sections[0].shrimp.is_empty());
        assert!(dry.static_sections[0].stones.stone_attachments.is_empty());
        assert!(dry.detritus.carbon() > 0.0);
        dried.validate().unwrap();

        world.edit_pond(&PondEdit::CommitTopology { basin_id: main, mode: PondShapeMode::Remove, cells: vec![29] }).unwrap();
        let parts = &world.pond.as_ref().unwrap().basins;
        assert_eq!([&parts[0].shrimp[0].view.id, &parts[1].shrimp[0].view.id], [&main_id, &side_id]);
        world.validate().unwrap();
    }
}

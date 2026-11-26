use super::*;

#[test]
fn r5_take_out_daphnia_exports_the_population_share() {
    let mut world = WorldState::new_r5_composed_pond(44);
    let before = world.clone();
    let original = basin(&before);
    let remove = |count| PondEdit::RemoveDaphnia { basin_id: basin_id(0), count };
    assert_eq!(world.edit_pond(&remove(0)), Err(CoreError::InvalidDaphniaRemovalCount));
    assert_eq!(world.edit_pond(&remove(original.daphnia.count + 1)), Err(CoreError::InvalidDaphniaRemovalCount));
    assert_eq!(world, before);

    let number = original.daphnia.count / 3;
    let fraction = f64::from(number) / f64::from(original.daphnia.count);
    world.edit_pond(&remove(number)).unwrap();
    let current = basin(&world);
    assert_eq!(current.daphnia.count, original.daphnia.count - number);
    assert_eq!(current.detritus, original.detritus);
    for element in 0..3 {
        let total = original.daphnia.body.0[element] + original.daphnia.reserve.0[element];
        assert_close(current.balance.outputs.cn_p.0[element], original.balance.outputs.cn_p.0[element] + total * fraction);
    }
    assert_balanced(current);
    world.edit_pond(&remove(original.daphnia.count - number)).unwrap();
    assert_eq!(basin(&world).daphnia, Daphnia::default());
    assert_balanced(basin(&world));
    world.restore_pond_edit_state(before.clone()).unwrap();
    assert_eq!(world, before);
}

#[test]
fn r5_shore_sedge_tracks_root_waterline_and_external_stocks() {
    let id = basin_id(0);
    let mut world = crate::WorldState::new_blank_r5_composed_pond(93);
    let anchor = PondShoreAnchor { water_cell: 63, shore_cell: 51 };
    let moved = PondShoreAnchor { water_cell: 64, shore_cell: 52 };
    let add = |anchor| PondEdit::AddSedge { basin_id: id.clone(), anchor, preset: PondSedgePreset::Medium };
    let read = |world: &crate::WorldState| world.pond.as_ref().unwrap().observe().basins[0].shore_sedges.clone();
    assert!(read(&world).is_none());
    let old: crate::WorldState = serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
    assert!(read(&old).is_none());
    let inland = PondShoreAnchor { water_cell: 63, shore_cell: 75 };
    assert_eq!(world.edit_pond(&add(inland)), Err(CoreError::InvalidPondSedgeSite));
    world.edit_pond(&add(anchor)).unwrap();
    assert_eq!(world.edit_pond(&add(anchor)), Err(CoreError::InvalidPondSedgeSite));
    let initial = read(&world).unwrap()[0];
    assert_eq!((initial.root_wet_fraction, initial.shoot_height_mm, initial.root_diameter_mm), (0.0, 220.0, 40.0));
    let before = world.clone();
    let bury = PondEdit::PaintShape { basin_id: id.clone(), mode: PondShapeMode::Add, cells: vec![51] };
    assert_eq!(world.edit_pond(&bury), Err(CoreError::PondShape(PondShapeError::TopologySedgeAnchor)));
    assert_eq!(read(&world), read(&before));
    let refill = |liters| PondEdit::AddWater { basin_id: id.clone(), liters };
    world.edit_pond(&refill(6.0)).unwrap();
    assert!((read(&world).unwrap()[0].root_wet_fraction - 0.45).abs() < 1e-10);
    world.edit_pond(&refill(12.0)).unwrap();
    assert_eq!(read(&world).unwrap()[0].root_wet_fraction, 1.0);
    let basin = &mut world.pond.as_mut().unwrap().basins[0];
    let need = basin.minerals;
    let oxygen = basin.oxygen_mg;
    basin.photosynthesize(1.0, 1.0);
    assert!(basin.sedge_layout.as_ref().unwrap().patches[0].stock.carbon() > initial.stock_mg_cnp[0]);
    assert!(basin.minerals.0[1] < need.0[1] && basin.minerals.0[2] < need.0[2]);
    assert_eq!(basin.oxygen_mg, oxygen);
    world.edit_pond(&PondEdit::MoveSedge { basin_id: id.clone(), id: initial.id, anchor: moved }).unwrap();
    let reloaded: crate::WorldState = serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
    reloaded.validate().unwrap();
    assert_eq!(read(&reloaded), read(&world));
    assert_eq!(read(&reloaded).unwrap()[0].anchor, moved);
    world.edit_pond(&PondEdit::DrainWater { basin_id: id.clone(), liters: 18.0 }).unwrap();
    let dry = read(&world).unwrap()[0];
    assert_eq!(dry.root_wet_fraction, 0.0);
    assert_eq!(dry.stock_mg_cnp, read(&reloaded).unwrap()[0].stock_mg_cnp);
    world.edit_pond(&PondEdit::RemoveSedge { basin_id: id, id: initial.id }).unwrap();
    assert!(read(&world).is_none());
    world.validate().unwrap();
}

#[test]
fn r5_pond_structures_keep_real_routes_across_deep_and_shallow_water() {
    for (label, edit, radius, travel_height) in [
        ("hanging roots", PondEdit::AddWood { basin_id: basin_id(0), x: 0.5, preset: PondWoodPreset::HangingRoots }, 28.0, 7.6),
        ("root network", PondEdit::AddWood { basin_id: basin_id(0), x: 0.5, preset: PondWoodPreset::RootNetwork }, 42.0, 23.0),
        ("stone cave", PondEdit::AddStone { basin_id: basin_id(0), x: 0.5, preset: PondStonePreset::FlatStoneCave }, 26.0, 6.0),
    ] {
        let mut world = WorldState::new_blank_r5_composed_pond(29);
        world.edit_pond(&PondEdit::AddWater { basin_id: basin_id(0), liters: 18.0 }).unwrap();
        world.edit_pond(&edit).unwrap();
        let column = basin(&world).column_projection_px();
        let [start, end] = [0.5 + (radius + 3.0) / pond_stones::BASIN_WIDTH_PX, 0.5 - (radius + 3.0) / pond_stones::BASIN_WIDTH_PX];
        let position = Point { x: start, y: 1.0 - travel_height / column };
        world.edit_pond(&PondEdit::StockShrimp { basin_id: basin_id(0), position, count: 1 }).unwrap();
        let b = &mut world.pond.as_mut().unwrap().basins[0];
        let mut crossed = false;
        for _ in 0..280 {
            // Control the horizontal intent, retaining the production stepping, drift and collision code.
            let before = animal_point(&b.shrimp[0].view);
            b.shrimp[0].view.heading = std::f64::consts::PI;
            b.step_shrimp(1.0, 1.0, [true; MAX_SHRIMP]);
            let after = animal_point(&b.shrimp[0].view);
            assert!(!b.ground_profiles().any(|pole| pole.blocks_segment(before, after, column)), "{label}");
            assert!(((1.0 - after.y) * column - travel_height).abs() < 1.8, "{label}: {after:?}");
            if after.x <= end { crossed = true; break; }
        }
        assert!(crossed, "could not pass {label}");
        world.validate().unwrap();
        world.edit_pond(&PondEdit::DrainWater { basin_id: basin_id(0), liters: 17.0 }).unwrap();
        world.validate().unwrap();
        world.edit_pond(&PondEdit::DrainWater { basin_id: basin_id(0), liters: 1.0 }).unwrap();
        assert!(basin(&world).shrimp.is_empty());
        assert_balanced(basin(&world));
        world.edit_pond(&PondEdit::AddWater { basin_id: basin_id(0), liters: 18.0 }).unwrap();
        world.validate().unwrap();
        assert_balanced(basin(&world));
    }
}

#[test]
fn r5_floating_leaf_canopy_unions_overlaps_and_clips_the_real_footprint() {
    let mut world = WorldState::new_r5_composed_pond(2729);
    let observation = |world: &WorldState| world.pond_observation().unwrap().basins[0].clone();
    let no_plants = world.clone();
    let shape = observation(&world).shape.unwrap();
    assert!(basin(&world).shape.is_none());
    assert!(!serde_json::to_string(&world).unwrap().contains("floatingPlantLayout"));
    assert!(!serde_json::to_string(&observation(&world)).unwrap().contains("floatingCanopy"));
    let row = f64::from(shape.protected_row) - 1.0;
    let center = Point { x: 6.5 / 12.0, y: (row + 0.5) / 12.0 };
    let outside = Point { x: center.x, y: (row - 0.5) / 12.0 };
    let add = |position| PondEdit::AddFloatingPlant { basin_id: basin_id(0), position, preset: PondFloatingPlantPreset::Large };
    assert_eq!(world.edit_pond(&add(outside)), Err(CoreError::InvalidPondFloatingPlantSite));
    assert_eq!(world, no_plants);
    world.edit_pond(&add(center)).unwrap();
    assert_eq!(observation(&world).shape, Some(shape.clone()));
    let first = observation(&world).floating_canopy.unwrap();
    let disk_area = std::f64::consts::PI * 3.0_f64.powi(2);
    assert!((first.covered_area_cm2 - disk_area).abs() < 0.06);
    assert_close(first.coverage, first.covered_area_cm2 / 2400.0);
    assert_close(first.light_transmission + first.coverage, 1.0);
    world.edit_pond(&add(center)).unwrap();
    assert_eq!(observation(&world).floating_canopy, Some(first));
    assert_eq!(observation(&world).floating_plants.unwrap().iter().map(|plant| plant.id).collect::<Vec<_>>(), [1, 2]);

    let other_center = Point { x: 2.5 / 12.0, y: center.y };
    let move_second = |position| PondEdit::MoveFloatingPlant { basin_id: basin_id(0), id: 2, position };
    world.edit_pond(&move_second(other_center)).unwrap();
    let two = observation(&world).floating_canopy.unwrap();
    assert_close(two.covered_area_cm2, 2.0 * first.covered_area_cm2);
    world.edit_pond(&move_second(Point { x: other_center.x, y: row / 12.0 })).unwrap();
    let bank = observation(&world).floating_canopy.unwrap();
    assert!((bank.covered_area_cm2 - 1.5 * disk_area).abs() < 0.09);
    let before_cut = world.clone();
    let cut = PondEdit::PaintShape { basin_id: basin_id(0), mode: PondShapeMode::Remove, cells: vec![(row as u16) * 12 + 2] };
    assert_eq!(world.edit_pond(&cut), Err(CoreError::PondShape(PondShapeError::Occupied)));
    assert_eq!(world, before_cut);
    assert_balanced(basin(&world));
}

#[test]
fn r5_floating_leaf_shades_water_producers_without_aerial_oxygen_credit() {
    let add = PondEdit::AddFloatingPlant {
        basin_id: basin_id(0), position: Point { x: 0.50, y: 0.50 }, preset: PondFloatingPlantPreset::Large,
    };
    let mut world = WorldState::new_r5_composed_pond(2729);
    world.edit_pond(&PondEdit::PlaceRibbonPlant { basin_id: basin_id(0), x: 0.36, preset: PondRibbonPlantPreset::Medium }).unwrap();
    let mut unshaded = basin(&world).clone();
    world.edit_pond(&add).unwrap();
    let shaded = &mut world.pond.as_mut().unwrap().basins[0];
    let baseline = shaded.clone();
    let transmission = shaded.floating_canopy().unwrap().light_transmission;
    assert!(transmission < 1.0);
    unshaded.photosynthesize(1.0, 1.0);
    shaded.photosynthesize(1.0, 1.0);
    for (total, shaded) in [(&unshaded.algae, &shaded.algae), (&unshaded.biofilm, &shaded.biofilm)] {
        assert!(shaded.carbon() < total.carbon());
    }
    assert_close(shaded.plants.carbon() - baseline.plants.carbon(),
        (unshaded.plants.carbon() - baseline.plants.carbon()) * transmission);
    let floating = shaded.floating_plant_layout.as_ref().unwrap().patches[0];
    let floating_gain = floating.stock_mg_cnp.carbon() - 32.0;
    assert!(floating_gain > 0.0);
    assert_close(shaded.balance.photosynthesis_oxygen_mg, unshaded.balance.photosynthesis_oxygen_mg * transmission);
    assert_close(shaded.balance.inputs.cn_p.carbon() - baseline.balance.inputs.cn_p.carbon(),
        shaded.balance.photosynthesis_oxygen_mg / OXYGEN_MG_PER_CARBON_MG + floating_gain);
    assert_balanced(shaded);

    let mut floating_only = WorldState::new_blank_r5_composed_pond(2729);
    floating_only.edit_pond(&PondEdit::AddWater { basin_id: basin_id(0), liters: 18.0 }).unwrap();
    floating_only.edit_pond(&add).unwrap();
    let pond = &mut floating_only.pond.as_mut().unwrap().basins[0];
    let before = pond.balance.inputs.cn_p.carbon();
    let oxygen = pond.oxygen_mg;
    let nitrogen = pond.minerals.0[1];
    pond.photosynthesize(1.0, 1.0);
    assert!(pond.balance.inputs.cn_p.carbon() > before);
    assert!(pond.minerals.0[1] < nitrogen);
    assert_eq!(pond.oxygen_mg, oxygen);
    assert_eq!(pond.balance.photosynthesis_oxygen_mg, 0.0);
    assert_balanced(pond);

    let mut limited = baseline;
    limited.minerals.0[2] = 1e-6;
    limited.oxygen_mg = 1e-7;
    limited.balance = Balance { initial: limited.inventory(), ..Balance::default() };
    let starting_carbon = limited.floating_plant_layout.as_ref().unwrap().patches[0].stock_mg_cnp.carbon();
    limited.photosynthesize(1.0, 1.0);
    let acquired = limited.floating_plant_layout.as_ref().unwrap().patches[0].stock_mg_cnp.carbon() - starting_carbon;
    assert!(0.0 < acquired && acquired < floating_gain);
    assert_close(limited.minerals.0[2], 0.0);
    let available_oxygen = limited.oxygen_mg;
    assert!(limited.respire(1.0) < 1.0);
    let respired = limited.floating_plant_layout.as_ref().unwrap().patches[0].stock_mg_cnp.carbon();
    assert!(respired < starting_carbon + acquired);
    assert_close(limited.balance.outputs.oxygen_mg, available_oxygen);
    assert_balanced(&limited);
}

#[test]
fn r5_rooted_plants_in_two_sections_share_minerals_and_oxygen() {
    let mut world = WorldState::new_blank_r5_composed_dual_pond(2729);
    for (basin_id, liters) in [(basin_id(0), 5.0), (basin_id(1), 7.0)] {
        world.edit_pond(&PondEdit::AddWater { basin_id: basin_id.clone(), liters }).unwrap();
        world.edit_pond(&PondEdit::AddWaterPlant { basin_id, x: 0.32 }).unwrap();
    }
    world.edit_pond(&PondEdit::PlaceRibbonPlant { basin_id: basin_id(1), x: 0.72, preset: PondRibbonPlantPreset::Large }).unwrap();
    world.edit_pond(&PondEdit::CommitTopology { basin_id: basin_id(0), mode: PondShapeMode::Add, cells: vec![24] }).unwrap();
    let mut fixture = basin(&world).clone();
    fixture.algae = Matter::with_ratios(18.0, 0.088, 0.012);
    fixture.minerals = Matter([0.0, 2.0, 2.0]);
    fixture.balance = Balance { initial: fixture.inventory(), ..Balance::default() };

    let producer_stocks = |pond: &Basin| {
        let side = pond.static_sections[0].water_plants.as_ref().unwrap();
        let (patches, _) = side.layout.observe(side.stock, pond.wet(), pond.depth_cm(), pond.turbidity());
        [pond.algae.carbon(), pond.plants.carbon(), patches[0].stock_mg_cnp[0], patches[1].stock_mg_cnp[0]]
    };
    let initial = producer_stocks(&fixture);
    assert_eq!(initial, [18.0, 50.0, 50.0, 75.0]);

    let mut ample_light = fixture.clone();
    ample_light.photosynthesize(1.0, 1.0);
    let ample_growth = producer_stocks(&ample_light);
    assert!(ample_growth.iter().zip(initial).all(|(after, before)| *after > before));
    assert_close(ample_growth[1], ample_growth[2]);
    assert!((ample_growth[3] - 75.0) / 75.0 > (ample_growth[2] - 50.0) / 50.0);
    assert_balanced(&ample_light);

    for nutrient in [1, 2] {
        let mut limited = fixture.clone();
        let supplied = 0.375;
        limited.minerals.0[nutrient] = (fixture.minerals.0[nutrient] - ample_light.minerals.0[nutrient]) * supplied;
        limited.balance = Balance { initial: limited.inventory(), ..Balance::default() };
        limited.photosynthesize(1.0, 1.0);
        for ((before, ample), after) in initial.into_iter().zip(ample_growth).zip(producer_stocks(&limited)) {
            assert_close(after - before, (ample - before) * supplied);
        }
        assert_close(limited.minerals.0[nutrient], 0.0);
        assert_close(limited.balance.photosynthesis_oxygen_mg, ample_light.balance.photosynthesis_oxygen_mg * supplied);
        assert_balanced(&limited);
    }

    let mut ample_oxygen = fixture.clone();
    assert_eq!(ample_oxygen.respire(1.0), 1.0);
    let ample_respiration = producer_stocks(&ample_oxygen);
    assert!(ample_respiration.iter().zip(initial).all(|(after, before)| *after < before));
    assert_close(ample_respiration[1], ample_respiration[2]);
    assert!((75.0 - ample_respiration[3]) / 75.0 > (50.0 - ample_respiration[2]) / 50.0);
    assert_balanced(&ample_oxygen);

    let mut limited = fixture;
    limited.oxygen_mg = ample_oxygen.balance.outputs.oxygen_mg * 0.25;
    limited.balance = Balance { initial: limited.inventory(), ..Balance::default() };
    assert_close(limited.respire(1.0), 0.25);
    for ((before, ample), after) in initial.into_iter().zip(ample_respiration).zip(producer_stocks(&limited)) {
        assert_close(before - after, (before - ample) * 0.25);
    }
    assert_close(limited.oxygen_mg, 0.0);
    assert_close(limited.balance.producer_respiration_oxygen_mg, limited.balance.initial.oxygen_mg);
    assert_balanced(&limited);
}

#[test]
fn r5_attached_sections_share_growth_respiration_and_colonization() {
    let mut world = WorldState::new_blank_r5_composed_pond_with_basins(2731, 3);
    for (index, id) in [basin_id(0), basin_id(1), basin_id(2)].into_iter().enumerate() {
        world.edit_pond(&PondEdit::AddWater { basin_id: id.clone(), liters: 18.0 }).unwrap();
        for x in [0.32, 0.68] {
            world.edit_pond(&PondEdit::AddStone { basin_id: id.clone(), x, preset: PondStonePreset::Main }).unwrap();
        }
        world.edit_pond(&PondEdit::EnableStoneBiofilm { basin_id: id.clone() }).unwrap();
        world.edit_pond(&PondEdit::AddMoss { basin_id: id.clone(), parent: PondMossParent::Stone { id: 1 }, offset: 0.0,
            preset: if index == 0 { PondMossPreset::Mat } else { PondMossPreset::Frond } }).unwrap();
        world.edit_pond(&PondEdit::StockShrimp { basin_id: id, position: Point { x: 0.5, y: 0.25 }, count: 1 }).unwrap();
    }
    world.edit_pond(&PondEdit::CommitTopology { basin_id: basin_id(1), mode: PondShapeMode::Add, cells: vec![60] }).unwrap();
    world.edit_pond(&PondEdit::CommitTopology { basin_id: basin_id(0), mode: PondShapeMode::Add, cells: vec![24] }).unwrap();
    let mut fixture = basin(&world).clone();
    assert_eq!(fixture.static_sections.len(), 2);
    let side = &fixture.static_sections[0];
    let column = fixture.moss_column_px();
    assert_eq!(side.stone_biofilm_wet_fractions(column)[..2], [1.0; 2]);
    assert_eq!(side.stone_biofilm_wet_fractions(0.0), [0.0; MAX_STONES]);
    assert_eq!(side.shrimp.len(), 1);
    fixture.biofilm = Matter::with_ratios(10.0, 0.077, 0.010);
    fixture.minerals = Matter([0.0, 2.0, 2.0]);
    fixture.balance = Balance { initial: fixture.inventory(), ..Balance::default() };

    let before_colonization = fixture.inventory();
    fixture.colonize_stone_biofilm(1.0);
    assert_close(fixture.biofilm.carbon(), 9.8);
    assert_close(fixture.total_biofilm_carbon(), 10.0);
    assert_close(fixture.stone_biofilm.as_ref().unwrap()[0].stock.carbon(), 0.2 / 6.0);
    assert_close(fixture.static_sections[0].stone_biofilm.as_ref().unwrap()[0].stock.carbon(), 0.2 / 6.0);
    for element in 0..3 { assert_close(fixture.inventory().cn_p.0[element], before_colonization.cn_p.0[element]); }
    assert_balanced(&fixture);

    let stocks = |pond: &Basin| {
        let side = &pond.static_sections[0];
        [pond.biofilm.carbon(), pond.stone_biofilm.as_ref().unwrap()[0].stock.carbon(),
            side.stone_biofilm.as_ref().unwrap()[0].stock.carbon(), pond.moss_layout.as_ref().unwrap().patches[0].stock_mg_cnp.carbon(),
            side.moss_layout.as_ref().unwrap().patches[0].stock_mg_cnp.carbon()]
    };
    let initial = stocks(&fixture);
    let mut plentiful = fixture.clone();
    plentiful.photosynthesize(1.0, 1.0);
    let expected = stocks(&plentiful);
    assert!(expected.iter().zip(initial).all(|(after, before)| *after > before));
    for mineral in [1, 2] {
        let mut limited = fixture.clone();
        limited.minerals.0[mineral] = (fixture.minerals.0[mineral] - plentiful.minerals.0[mineral]) * 0.375;
        limited.balance = Balance { initial: limited.inventory(), ..Balance::default() };
        limited.photosynthesize(1.0, 1.0);
        for ((before, full), actual) in initial.into_iter().zip(expected).zip(stocks(&limited)) {
            assert_close(actual - before, (full - before) * 0.375);
        }
        assert_close(limited.balance.photosynthesis_oxygen_mg, plentiful.balance.photosynthesis_oxygen_mg * 0.375);
        assert_balanced(&limited);
    }

    let side_reserve = |pond: &Basin| pond.static_sections[0].shrimp[0].reserve.carbon();
    let mut plentiful = fixture.clone();
    assert_eq!(plentiful.respire(1.0), 1.0);
    assert!(side_reserve(&plentiful) < side_reserve(&fixture));
    let mut limited = fixture.clone();
    limited.oxygen_mg = plentiful.balance.outputs.oxygen_mg * 0.25;
    limited.balance = Balance { initial: limited.inventory(), ..Balance::default() };
    assert_close(limited.respire(1.0), 0.25);
    assert_close(side_reserve(&fixture) - side_reserve(&limited), (side_reserve(&fixture) - side_reserve(&plentiful)) * 0.25);
    assert_close(initial[2] - stocks(&limited)[2], (initial[2] - stocks(&plentiful)[2]) * 0.25);
    assert_close(limited.balance.producer_respiration_oxygen_mg, plentiful.balance.producer_respiration_oxygen_mg * 0.25);
    assert_close(limited.balance.animal_respiration_oxygen_mg, plentiful.balance.animal_respiration_oxygen_mg * 0.25);
    assert_close(limited.oxygen_mg, 0.0);
    assert_balanced(&limited);
}

#[test]
fn r5_floating_leaf_save_dry_refill_and_history_keep_basin_boundaries() {
    let mut world = WorldState::new_blank_r5_composed_pond_with_basins(2729, 2);
    let basin_id = basin_id(0);
    let position = Point { x: 0.42, y: 1.5 / 12.0 };
    let add = PondEdit::AddFloatingPlant { basin_id: basin_id.clone(), position, preset: PondFloatingPlantPreset::Medium };
    let water = PondEdit::AddWater { basin_id: basin_id.clone(), liters: 18.0 };
    assert_eq!(world.edit_pond(&add), Err(CoreError::PondFloatingPlantNeedsWater));
    world.edit_pond(&water).unwrap();
    let before_add = world.clone();
    let other = world.pond_observation().unwrap().basins[1].clone();
    world.edit_pond(&add).unwrap();
    let first = basin(&world).observe(0);
    assert_eq!(first.floating_plants.as_ref().unwrap()[0].stock_mg_cnp[0], 20.0);
    assert_eq!(world.pond_observation().unwrap().basins[1], other);
    let merge = PondEdit::CommitTopology { basin_id: basin_id.clone(), mode: PondShapeMode::Add, cells: vec![24] };
    let mut joined = world.clone();
    joined.edit_pond(&merge).unwrap();
    let merged = joined.pond_observation().unwrap();
    assert_eq!(merged.basins.len(), 1);
    assert_eq!(merged.basins[0].floating_plants, first.floating_plants);

    world.edit_pond(&PondEdit::DrainWater { basin_id: basin_id.clone(), liters: 18.0 }).unwrap();
    let dry = basin(&world).observe(0);
    let dry_stock = dry.floating_plants.unwrap()[0];
    assert!(!dry_stock.floating);
    assert_eq!(dry_stock.stock_mg_cnp, first.floating_plants.unwrap()[0].stock_mg_cnp);
    assert_eq!(dry.floating_canopy.unwrap().covered_area_cm2, 0.0);
    assert_eq!(dry.floating_canopy.unwrap().light_transmission, 1.0);
    assert_eq!(world.edit_pond(&add), Err(CoreError::PondFloatingPlantNeedsWater));
    world.edit_pond(&water).unwrap();
    assert_eq!(basin(&world).observe(0).floating_canopy, first.floating_canopy);
    world.validate().unwrap();
    let reopened: WorldState = serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
    reopened.validate().unwrap();
    assert_eq!(reopened, world);
    let before_remove = basin(&world).ledger();
    let removed_id = dry_stock.id;
    world.edit_pond(&PondEdit::RemoveFloatingPlant { basin_id, id: removed_id }).unwrap();
    let removed = basin(&world).observe(0);
    assert!(removed.floating_canopy.is_none() && removed.floating_plants.is_none());
    assert_close(removed.detritus_mg_c, 20.0);
    for i in 0..3 {
        assert_close(basin(&world).ledger().current_matter_mg_cnp[i], before_remove.current_matter_mg_cnp[i]);
    }
    world.restore_pond_edit_state(before_add).unwrap();
    world.edit_pond(&add).unwrap();
    assert!(basin(&world).observe(0).floating_plants.unwrap()[0].id > removed_id);
    let fresh = world.fresh_habitat(42).unwrap();
    fresh.validate().unwrap();
    assert_balanced(basin(&fresh));
    assert_balanced(basin(&world));
}

#[test]
fn r5_topology_conserves_stocks_and_keeps_independent_identifiers() {
    let mut world = WorldState::new_blank_r5_composed_pond_with_basins(892, 3);
    let main = POND_DEFAULT_BASIN_ID.to_owned();
    let second = "pond-2".to_owned();
    for (id, volume, detritus) in [(&main, 3.0, 35.0), (&second, 5.0, 45.0)] {
        world.edit_pond(&PondEdit::AddWater { basin_id: id.clone(), liters: volume }).unwrap();
        world.edit_pond(&PondEdit::AddDetritus { basin_id: id.clone(), mg_c: detritus }).unwrap();
    }
    let pond = world.pond.as_mut().unwrap();
    pond.basins[0].temperature_celsius = 12.0;
    pond.basins[1].temperature_celsius = 28.0;
    let untouched = pond.basins[2].clone();
    let mut old_stocks = pond.basins[1].clone();
    old_stocks.add_free_stocks(&pond.basins[0], 1.0);
    let original = world.clone();
    let original_json = serde_json::to_string(&original).unwrap();
    assert!(!original_json.contains("nextBasinNumber") && !original_json.contains("observationRow"));

    let topology = |mode| PondEdit::CommitTopology { basin_id: second.clone(), mode, cells: vec![24] };
    world.edit_pond(&topology(PondShapeMode::Add)).unwrap();
    world.validate().unwrap();
    let merged = world.clone();
    let pond = world.pond.as_ref().unwrap();
    assert_eq!(pond.basins.iter().map(|basin| basin.id.as_str()).collect::<Vec<_>>(), ["pond-2", "pond-3"]);
    assert_eq!(pond.basins[0].observation_row, Some(4));
    assert_eq!(pond.basins[1].observation_row, Some(7));
    assert_eq!(pond.basins[1].inventory(), untouched.inventory());
    assert_eq!(pond.basins[1].random, untouched.random);
    assert_eq!(pond.basins[0].surface_area_cm2, 4900.0);
    assert_eq!(pond.basins[0].inventory(), old_stocks.inventory());
    assert_eq!(pond.basins[0].balance, old_stocks.balance);
    assert!((pond.basins[0].temperature_celsius - 22.0).abs() < 1e-9);

    world.edit_pond(&topology(PondShapeMode::Remove)).unwrap();
    world.validate().unwrap();
    let pond = world.pond.as_ref().unwrap();
    assert_eq!(pond.basins.iter().map(|basin| basin.id.as_str()).collect::<Vec<_>>(), ["pond-2", "pond-3", "pond-4"]);
    assert_eq!(pond.basins[2].observation_row, Some(0));
    assert_ne!(pond.basins[0].random, pond.basins[2].random);
    for index in [0, 2] {
        let mut expected = old_stocks.clone();
        expected.scale_free_stocks(0.5, false, 0.0, 0.0);
        assert_eq!(pond.basins[index].inventory(), expected.inventory());
        assert_eq!(pond.basins[index].balance, expected.balance);
        assert_eq!(pond.basins[index].water_liters, 4.0);
    }
    let saved = serde_json::to_string(&world).unwrap();
    let round_trip: WorldState = serde_json::from_str(&saved).unwrap();
    round_trip.validate().unwrap();
    assert_eq!(round_trip, world);
    let fresh = world.fresh_habitat(42).unwrap();
    fresh.validate().unwrap();
    assert_eq!(fresh.pond.as_ref().unwrap().basins[2].id, "pond-4");
    assert_eq!(fresh.pond.as_ref().unwrap().basins[2].observation_row, Some(0));
    world.restore_pond_edit_state(merged).unwrap();
    world.edit_pond(&topology(PondShapeMode::Remove)).unwrap();
    assert_eq!(world.pond.as_ref().unwrap().basins[2].id, "pond-5");
    let mut restored = original;
    restored.retain_pond_topology_allocator(&world);
    restored.validate().unwrap();
    assert_eq!(restored.pond.as_ref().unwrap().next_basin_number, Some(6));
}

#[test]
fn r5_topology_rejects_unrepresentable_results_atomically() {
    let main = POND_DEFAULT_BASIN_ID.to_owned();
    let bridge = |mode| PondEdit::CommitTopology { basin_id: main.clone(), mode, cells: vec![24] };
    let rejected = |world: &mut WorldState, edit: PondEdit, why: PondShapeError| {
        let original = world.clone();
        assert_eq!(world.edit_pond(&edit), Err(CoreError::PondShape(why)));
        assert_eq!(*world, original);
    };

    let blank = WorldState::new_blank_r5_composed_dual_pond(5);
    let mut unscaled = blank.clone();
    unscaled.edit_pond(&PondEdit::Reshape { basin_id: "pond-2".into(), surface_area_cm2: 2500.0 }).unwrap();
    rejected(&mut unscaled, bridge(PondShapeMode::Add), PondShapeError::TopologyScale);

    let mut shallow = blank.clone();
    shallow.edit_pond(&PondEdit::AddWater { basin_id: main.clone(), liters: 0.075 }).unwrap();
    shallow.edit_pond(&bridge(PondShapeMode::Add)).unwrap();
    rejected(&mut shallow, bridge(PondShapeMode::Remove), PondShapeError::TopologyShallow);

    let mut full = blank;
    full.edit_pond(&bridge(PondShapeMode::Add)).unwrap();
    full.edit_pond(&PondEdit::AddWater { basin_id: main.clone(), liters: 58.8 }).unwrap();
    rejected(&mut full, bridge(PondShapeMode::Remove), PondShapeError::Capacity);

    let mut narrow = WorldState::new_blank_r5_composed_pond(5);
    narrow.edit_pond(&PondEdit::PaintShape { basin_id: main.clone(), mode: PondShapeMode::Add,
        cells: vec![51, 39, 38, 37, 36] }).unwrap();
    rejected(&mut narrow, PondEdit::CommitTopology { basin_id: main, mode: PondShapeMode::Remove,
        cells: vec![51] }, PondShapeError::TopologySection);
}
use crate::{season, WorldState};

fn basin(world: &WorldState) -> &Basin {
    &world.pond.as_ref().unwrap().basins[0]
}

fn assert_close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-7, "{actual:.12} vs {expected:.12}");
}

fn assert_balanced(basin: &Basin) {
    let ledger = basin.ledger();
    assert_close(ledger.water_residual_liters, 0.0);
    assert_close(ledger.oxygen_residual_mg, 0.0);
    for residual in ledger.matter_residual_mg_cnp { assert_close(residual, 0.0); }
    assert!(basin.valid());
}

fn leaf_deposit(x: f64, preset: PondLeafDepositPreset, mg_c: f64) -> PondEdit {
    PondEdit::AddLeafDeposit { basin_id: basin_id(0), x, preset, mg_c }
}

#[test]
fn r5_pond_leaf_deposit_uses_the_original_detritus_and_oxygen_processes_once() {
    use PondLeafDepositPreset::{Foreground, Midground};
    let mut world = WorldState::new_r5_composed_pond(2729);
    let mut pool_only = world.clone();
    assert!(basin(&world).observe(0).leaf_deposits.is_none());
    assert!(serde_json::to_value(&world).unwrap()["pond"]["basins"][0].get("leafDepositLayout").is_none());
    let before = basin(&world).ledger();
    let mut deposition = Matter::default();
    for (x, preset, mg_c) in [(0.235, Midground, 24.0), (0.62, Foreground, 40.0)] {
        world.edit_pond(&leaf_deposit(x, preset, mg_c)).unwrap();
        pool_only.edit_pond(&PondEdit::AddDetritus { basin_id: basin_id(0), mg_c }).unwrap();
        deposition.add(pond_leaf::detritus_matter(mg_c));
    }
    let after = basin(&world).ledger();
    assert_eq!(after, basin(&pool_only).ledger());
    assert_eq!(after.matter_input_mg_cnp, deposition.0);
    assert_eq!(before.current_oxygen_mg, after.current_oxygen_mg);
    assert_close(basin(&world).detritus.carbon(), 184.0);
    let placed = basin(&world).observe(0).leaf_deposits.unwrap();
    assert_eq!(placed[0].initial_mg_c, 24.0);
    assert_eq!(placed[1].remaining_mg_c, 40.0);
    let move_deposit = PondEdit::MoveLeafDeposit { basin_id: basin_id(0), id: placed[0].id, x: 0.48 };
    world.edit_pond(&move_deposit).unwrap();
    let moved = basin(&world).observe(0).leaf_deposits.unwrap();
    assert_eq!(moved[0], PondLeafDepositObservation { x: 0.48, ..placed[0] });
    assert_eq!(basin(&world).ledger(), after);

    world.advance_steps(32).unwrap();
    pool_only.advance_steps(32).unwrap();
    let decayed = basin(&world).observe(0).leaf_deposits.unwrap();
    assert!(decayed.iter().all(|deposit| deposit.remaining_mg_c < deposit.initial_mg_c));
    assert_close(decayed[0].remaining_mg_c / 24.0, decayed[1].remaining_mg_c / 40.0);
    assert_eq!(basin(&world).ledger(), basin(&pool_only).ledger());
    let generic_input = PondEdit::AddDetritus { basin_id: basin_id(0), mg_c: 9.0 };
    world.edit_pond(&generic_input).unwrap();
    pool_only.edit_pond(&generic_input).unwrap();
    assert_eq!(basin(&world).observe(0).leaf_deposits.unwrap(), decayed);
    let mut without_markers = world.clone();
    without_markers.pond.as_mut().unwrap().basins[0].leaf_deposit_layout = None;
    assert_eq!(without_markers, pool_only);
    assert_balanced(basin(&world));

    let fresh = world.fresh_habitat(2729).unwrap();
    assert_eq!(basin(&fresh).observe(0).leaf_deposits.unwrap(), moved);
    assert_close(basin(&fresh).detritus.carbon(), 184.0);
    assert_eq!(basin(&fresh).ledger().matter_input_mg_cnp, [0.0; 3]);
    assert_balanced(basin(&fresh));
}

#[test]
fn r5_pond_leaf_deposit_checks_bounds_and_supports_a_dry_bed() {
    use PondLeafDepositPreset::Midground;
    let mut old = WorldState::new_r5_moss_pond(2729);
    let saved = old.clone();
    assert_eq!(old.edit_pond(&leaf_deposit(0.5, Midground, 10.0)), Err(CoreError::WrongWorldKind));
    assert_eq!(old, saved);

    let mut world = WorldState::new_blank_r5_composed_pond(2729);
    let before = world.clone();
    for (x, mg_c, error) in [
        (f64::NAN, 10.0, CoreError::InvalidPondLeafDepositSite),
        (0.079, 10.0, CoreError::InvalidPondLeafDepositSite),
        (0.921, 10.0, CoreError::InvalidPondLeafDepositSite),
        (0.5, 0.0, CoreError::InvalidPondEdit),
        (0.5, 50_001.0, CoreError::InvalidPondEdit),
    ] {
        assert_eq!(world.edit_pond(&leaf_deposit(x, Midground, mg_c)), Err(error));
        assert_eq!(world, before);
    }
    for _ in 0..16 { world.edit_pond(&leaf_deposit(0.08, Midground, 10.0)).unwrap(); }
    let full = world.clone();
    assert_eq!(world.edit_pond(&leaf_deposit(0.92, Midground, 10.0)), Err(CoreError::PondLeafDepositLimit));
    assert_eq!(world, full);
    assert_eq!(world.edit_pond(&PondEdit::MoveLeafDeposit { basin_id: basin_id(0), id: 17, x: 0.92 }), Err(CoreError::PondLeafDepositMissing));
    assert_eq!(world, full);
    let dry = basin(&world).observe(0);
    world.advance_steps(10).unwrap();
    assert_eq!(basin(&world).observe(0), dry);
    assert_balanced(basin(&world));
    let mut malformed = serde_json::to_value(&world).unwrap();
    malformed["pond"]["basins"][0]["leafDepositLayout"]["deposits"][0]["remainingMgC"] = 999.0.into();
    let malformed: WorldState = serde_json::from_value(malformed).unwrap();
    assert_eq!(malformed.validate(), Err(CoreError::InvalidPondState));
}

fn paint(index: usize, mode: PondShapeMode, cells: Vec<u16>) -> PondEdit {
    PondEdit::PaintShape { basin_id: basin_id(index), mode, cells }
}

#[test]
fn r5_pond_shape_changes_area_and_depth_without_moving_stock() {
    use PondShapeMode::{Add, Remove};

    let mut world = WorldState::new_r5_composed_pond(2729);
    let unpainted = serde_json::to_value(&world).unwrap();
    let view = |state: &WorldState| state.pond_observation().unwrap().basins.remove(0);
    let initial = view(&world);
    let shape = initial.shape.as_ref().unwrap();
    assert_eq!((shape.columns, shape.rows, shape.protected_row), (12, 12, 6));
    assert_eq!(shape.cells, (60..84).collect::<Vec<_>>());
    assert_eq!(shape.cell_area_cm2, 100.0);
    assert!(unpainted["pond"]["basins"][0].get("shape").is_none());
    let restored: WorldState = serde_json::from_value(unpainted).unwrap();
    assert_eq!(view(&restored), initial);
    assert_eq!(restored, world);

    let ledger = world.pond_ledger().unwrap();
    world.edit_pond(&paint(0, Add, vec![53, 60, 53])).unwrap();
    let expanded = view(&world);
    let expanded_shape = expanded.shape.as_ref().unwrap();
    assert_eq!(expanded_shape.cells, std::iter::once(53).chain(60..84).collect::<Vec<_>>());
    assert_eq!(expanded.surface_area_cm2, 2500.0);
    assert_eq!(expanded.capacity_liters, 30.0);
    assert_eq!(expanded.depth_cm, 7.2);
    assert_eq!(expanded.water_liters, initial.water_liters);
    assert_eq!(expanded.oxygen_mg_per_liter, initial.oxygen_mg_per_liter);
    assert_eq!(expanded.available_nitrogen_mg_per_liter, initial.available_nitrogen_mg_per_liter);
    assert_eq!(expanded.available_phosphorus_mg_per_liter, initial.available_phosphorus_mg_per_liter);
    assert_eq!(expanded.stones, initial.stones);
    assert_eq!(expanded.woods, initial.woods);
    assert_eq!(expanded.scenery, initial.scenery);
    assert_eq!(world.pond_ledger().unwrap(), ledger);
    world.validate().unwrap();

    let fresh = world.fresh_habitat(2729).unwrap();
    assert_eq!(view(&fresh).shape, expanded.shape);
    assert_eq!(view(&fresh).depth_cm, expanded.depth_cm);
    let saved = serde_json::to_string(&world).unwrap();
    let reopened: WorldState = serde_json::from_str(&saved).unwrap();
    reopened.validate().unwrap();
    assert_eq!(reopened, world);

    let area_edit = PondEdit::Reshape { basin_id: basin_id(0), surface_area_cm2: 2872.5 };
    world.edit_pond(&area_edit).unwrap();
    let numeric_shape = view(&world).shape.unwrap();
    assert_eq!(numeric_shape.cells, expanded_shape.cells);
    assert_eq!(numeric_shape.cell_area_cm2, 114.9);
    world.edit_pond(&paint(0, Remove, vec![53])).unwrap();
    let reduced = view(&world);
    assert_eq!(reduced.shape.as_ref().unwrap().cells, shape.cells);
    assert_close(reduced.surface_area_cm2, 24.0 * 114.9);
    assert_eq!(world.pond_ledger().unwrap(), ledger);
    world.validate().unwrap();
}

#[test]
fn r5_pond_shape_rejects_splits_merges_and_capacity_atomically() {
    use PondShapeError::{AreaOutOfRange, Capacity, Disconnected, InvalidCells, ProtectedRow, TouchesOther, Unchanged};
    use PondShapeMode::{Add, Remove};

    let mut world = WorldState::new_r5_composed_pond(2729);
    let initial = world.clone();
    for (operation, issue) in [
        (paint(0, Add, Vec::new()), InvalidCells),
        (paint(0, Add, vec![53, 144]), InvalidCells),
        (paint(0, Add, vec![53; 145]), InvalidCells),
        (paint(0, Remove, vec![63, 75]), ProtectedRow),
        (paint(0, Add, vec![60, 70]), Unchanged),
        (paint(0, Add, vec![29]), Disconnected),
        (paint(0, Remove, (60..70).collect()), Capacity),
        (paint(0, Add, (0..144).collect()), AreaOutOfRange),
    ] {
        assert_eq!(world.edit_pond(&operation), Err(CoreError::PondShape(issue)));
        assert_eq!(world, initial);
    }
    world.edit_pond(&paint(0, Add, vec![53, 41])).unwrap();
    let joined = world.clone();
    assert_eq!(world.edit_pond(&paint(0, Remove, vec![53])), Err(CoreError::PondShape(Disconnected)));
    assert_eq!(world, joined);

    let mut ponds = PondState::with_scenery(2729, MAX_BASINS);
    let original = ponds.clone();
    assert_eq!(ponds.observe().basins.iter().map(|basin| basin.shape.as_ref().unwrap().protected_row).collect::<Vec<_>>(), [1, 4, 7, 10]);
    for operation in [paint(0, Add, vec![29]), paint(0, Add, vec![29, 41]), paint(1, Add, vec![29])] {
        assert_eq!(ponds.edit(&operation, season::observe(0)), Err(CoreError::PondShape(TouchesOther)));
        assert_eq!(ponds, original);
    }
    ponds.edit(&paint(3, Add, vec![138]), season::observe(0)).unwrap();
    ponds.validate(true, true, true, true).unwrap();
    assert_eq!(&ponds.observe().basins[..3], &original.observe().basins[..3]);

    let mut dry = WorldState::new_blank_r5_composed_pond(2729);
    dry.edit_pond(&paint(0, Remove, (60..72).collect())).unwrap();
    let dry_view = dry.pond_observation().unwrap().basins.remove(0);
    assert_eq!(dry_view.capacity_liters, 14.4);
    assert_eq!(dry_view.depth_cm, 0.0);
    assert_eq!(dry_view.oxygen_mg_per_liter, None);
    let dry_before = dry.clone();
    assert_eq!(dry.edit_pond(&PondEdit::AddWater { basin_id: basin_id(0), liters: 15.0 }), Err(CoreError::PondCapacityExceeded));
    assert_eq!(dry, dry_before);
}

#[test]
fn r5_pond_shape_validates_checkpoints_and_rejects_old_model_edits() {
    let mut legacy = WorldState::new_r5_moss_pond(2729);
    let old = legacy.clone();
    let add = paint(0, PondShapeMode::Add, vec![53]);
    assert_eq!(legacy.edit_pond(&add), Err(CoreError::WrongWorldKind));
    assert_eq!(legacy, old);
    assert_eq!(legacy.pond_observation().unwrap().basins[0].shape, None);
    let mut current = WorldState::new_r5_composed_pond(2729);
    current.edit_pond(&add).unwrap();
    let current = serde_json::to_value(current).unwrap();
    let mut legacy_save = serde_json::to_value(legacy).unwrap();
    legacy_save["pond"]["basins"][0]["shape"] = current["pond"]["basins"][0]["shape"].clone();
    let invalid_version: WorldState = serde_json::from_value(legacy_save).unwrap();
    assert_eq!(invalid_version.validate(), Err(CoreError::InvalidPondState));
    for (index, bad_cell) in [(1, 53), (0, 41), (1, 144), (13, 71)] {
        let mut bad_save = current.clone();
        bad_save["pond"]["basins"][0]["shape"]["cells"][index] = bad_cell.into();
        let invalid_shape: WorldState = serde_json::from_value(bad_save).unwrap();
        assert_eq!(invalid_shape.validate(), Err(CoreError::InvalidPondState));
    }
}

#[test]
fn r5_water_plant_roots_share_one_pool_and_return_trimmings_to_detritus() {
    let add = |x| PondEdit::AddWaterPlant { basin_id: basin_id(0), x };
    let mut world = WorldState::new_r5_composed_pond(2729);
    let initial = world.clone();
    let initial_view = basin(&world).observe(0);
    assert_eq!(initial_view.water_plant_patches, Some(Vec::new()));
    assert_eq!(initial_view.unassigned_plant_mg_cnp, Some(basin(&world).plants.0));
    assert!(!serde_json::to_string(&world).unwrap().contains("waterPlantLayout"));
    for (edit, error) in [
        (add(f64::NAN), CoreError::InvalidPondWaterPlantSite),
        (add(0.02), CoreError::InvalidPondWaterPlantSite),
        (add(0.235), CoreError::PondWaterPlantSiteOccupied), // Sloped wood foot.
        (add(0.60), CoreError::PondWaterPlantSiteOccupied), // Main stone.
    ] {
        assert_eq!(world.edit_pond(&edit), Err(error));
        assert_eq!(world, initial);
    }
    let mut old = WorldState::new_r5_moss_pond(2729);
    assert_eq!(basin(&old).observe(0).water_plant_patches, None);
    assert_eq!(old.edit_pond(&add(0.36)), Err(CoreError::WrongWorldKind));

    // One root is under the raised trunk where the modeled floor remains open.
    world.edit_pond(&add(0.36)).unwrap();
    world.edit_pond(&add(0.80)).unwrap();
    let planted = world.clone();
    for edit in [
        add(0.40),
        PondEdit::MoveStone { basin_id: basin_id(0), id: 1, x: 0.74 },
        PondEdit::MoveWood { basin_id: basin_id(0), id: 1, x: 0.36 },
    ] {
        assert_eq!(world.edit_pond(&edit), Err(CoreError::PondWaterPlantSiteOccupied));
        assert_eq!(world, planted);
    }
    let plants = basin(&world).plants;
    let ledger = basin(&world).ledger();
    let views = basin(&world).observe(0);
    let mut allocated = Matter(views.unassigned_plant_mg_cnp.unwrap());
    for patch in views.water_plant_patches.unwrap() {
        assert!(patch.submerged);
        allocated.add(Matter(patch.stock_mg_cnp));
        for (actual, expected) in patch.stock_mg_cnp.into_iter().zip(pond_plant::dose().0) { assert_close(actual, expected); }
    }
    for index in 0..3 {
        assert_close(allocated.0[index], plants.0[index]);
        assert_close(ledger.matter_input_mg_cnp[index], 2.0 * pond_plant::dose().0[index]);
    }

    world.edit_pond(&PondEdit::MoveWaterPlant { basin_id: basin_id(0), id: 1, x: 0.42 }).unwrap();
    let moved = basin(&world).observe(0).water_plant_patches.unwrap();
    assert_eq!(moved[0].x, 0.42);
    assert_eq!(basin(&world).ledger(), ledger);
    let mut shared_pool_only = world.clone();
    shared_pool_only.pond.as_mut().unwrap().basins[0].water_plant_layout = None;
    world.advance_steps(600).unwrap();
    shared_pool_only.advance_steps(600).unwrap();
    assert_eq!(basin(&world).plants, basin(&shared_pool_only).plants);
    assert_eq!(basin(&world).ledger(), basin(&shared_pool_only).ledger());
    assert_eq!(basin(&world).shrimp, basin(&shared_pool_only).shrimp);

    let before = basin(&world).ledger();
    let shortly_before_trim = basin(&world).detritus;
    let observation = basin(&world).observe(0);
    let [first, second] = <[_; 2]>::try_from(observation.water_plant_patches.unwrap()).unwrap();
    let trim = |mg_c| PondEdit::TrimWaterPlant { basin_id: basin_id(0), id: first.id, mg_c };
    let not_trimmed = world.clone();
    assert_eq!(world.edit_pond(&trim(first.stock_mg_cnp[0])), Err(CoreError::InvalidPondWaterPlantTrim));
    assert_eq!(world, not_trimmed);
    world.edit_pond(&trim(20.0)).unwrap();
    world.edit_pond(&PondEdit::RemoveWaterPlant { basin_id: basin_id(0), id: second.id }).unwrap();
    let observed = basin(&world).observe(0);
    let remaining = &observed.water_plant_patches.unwrap()[0];
    assert_eq!(remaining.id, first.id);
    assert_eq!(remaining.x, 0.42);
    for element in 0..3 {
        let taken = first.stock_mg_cnp[element] * 20.0 / first.stock_mg_cnp[0];
        assert_close(remaining.stock_mg_cnp[element], first.stock_mg_cnp[element] - taken);
        assert_close(basin(&world).detritus.0[element] - shortly_before_trim.0[element], taken + second.stock_mg_cnp[element]);
        assert_close(observed.unassigned_plant_mg_cnp.unwrap()[element], observation.unassigned_plant_mg_cnp.unwrap()[element]);
    }
    assert_eq!(basin(&world).ledger().matter_input_mg_cnp, before.matter_input_mg_cnp);
    assert_eq!(basin(&world).ledger().matter_output_mg_cnp, before.matter_output_mg_cnp);
    assert_balanced(basin(&world));
}

#[test]
fn r5_water_plant_exposure_and_initial_presets_keep_their_stock_boundaries() {
    let add = |x| PondEdit::AddWaterPlant { basin_id: basin_id(0), x };
    let mut world = WorldState::new_blank_r5_composed_pond(2729);
    assert_eq!(world.edit_pond(&add(0.10)), Err(CoreError::PondWaterPlantNeedsWater));
    let add_water = PondEdit::AddWater { basin_id: basin_id(0), liters: 12.0 };
    world.edit_pond(&add_water).unwrap();
    for root in 1..=8 { world.edit_pond(&add(f64::from(root) / 10.0)).unwrap(); }
    let full = world.clone();
    assert_eq!(world.edit_pond(&add(0.90)), Err(CoreError::PondWaterPlantLimit));
    assert_eq!(world, full);
    world.add_starter_community().unwrap();
    let after = basin(&world).observe(0);
    assert_close(after.unassigned_plant_mg_cnp.unwrap()[0], 150.0);
    assert_close(after.water_plant_patches.unwrap()[0].stock_mg_cnp[0], 50.0);

    world.edit_pond(&PondEdit::TrimWaterPlant { basin_id: basin_id(0), id: 1, mg_c: 9.0 }).unwrap();
    world.edit_pond(&PondEdit::DrainWater { basin_id: basin_id(0), liters: 12.0 }).unwrap();
    let dry = basin(&world).observe(0).water_plant_patches.unwrap();
    assert!(dry.iter().all(|patch| !patch.submerged));
    assert_close(dry[0].stock_mg_cnp[0], 41.0);
    world.advance_steps(8).unwrap();
    assert_eq!(basin(&world).observe(0).water_plant_patches.unwrap(), dry);
    world.edit_pond(&add_water).unwrap();
    assert!(basin(&world).observe(0).water_plant_patches.unwrap().iter().all(|patch| patch.submerged));
    let fresh = world.fresh_habitat(2729).unwrap();
    let initial = basin(&fresh).observe(0);
    assert_close(initial.unassigned_plant_mg_cnp.unwrap()[0], 150.0);
    for patch in initial.water_plant_patches.unwrap() {
        for (mass, dose) in patch.stock_mg_cnp.into_iter().zip(pond_plant::dose().0) { assert_close(mass, dose); }
    }
    assert_eq!(basin(&fresh).ledger().matter_input_mg_cnp, [0.0; 3]);
    assert_balanced(basin(&world));
    assert_balanced(basin(&fresh));
}

#[test]
fn r5_ribbon_uses_shared_nutrients_and_oxygen_but_applies_growth_to_its_own_stock() {
    let ribbon = |x, preset| PondEdit::PlaceRibbonPlant { basin_id: basin_id(0), x, preset };
    let mut world = WorldState::new_blank_r5_composed_pond(2729);
    world.edit_pond(&PondEdit::AddWater { basin_id: basin_id(0), liters: 24.0 }).unwrap();
    world.edit_pond(&PondEdit::AddWaterPlant { basin_id: basin_id(0), x: 0.15 }).unwrap();
    world.edit_pond(&ribbon(0.40, PondRibbonPlantPreset::Small)).unwrap();
    world.edit_pond(&ribbon(0.75, PondRibbonPlantPreset::Large)).unwrap();
    world.add_starter_community().unwrap();
    let pond = &mut world.pond.as_mut().unwrap().basins[0];
    let view = |pond: &Basin| pond.observe(0).water_plant_patches.unwrap();
    let original = view(pond);
    let transmission = |patches: &[PondWaterPlantObservation], index: usize| patches[index].ribbon_light_transmission.unwrap();
    assert_eq!(original[0].ribbon_preset, None);
    assert_eq!(original[0].ribbon_light_transmission, None);
    assert!(transmission(&original, 1) < transmission(&original, 2));
    assert!((0.0..1.0).contains(&transmission(&original, 1)));
    assert_close(original[1].stock_mg_cnp[0], 30.0);
    assert_close(original[2].stock_mg_cnp[0], 75.0);

    let mut clear = pond.clone();
    clear.algae = Matter::default();
    assert!(transmission(&view(&clear), 1) > transmission(&original, 1));
    clear.water_liters *= 0.6;
    assert!(transmission(&view(&clear), 1) > transmission(&original, 2));

    // All producers, including the original algae and film, share one phosphorus limit.
    pond.minerals = Matter([0.0, 1.0, 0.0000001]);
    pond.balance = Balance { initial: pond.inventory(), ..Balance::default() };
    let old_algae = pond.algae.carbon();
    pond.photosynthesize(1.0, 1.0);
    let grown = view(pond);
    assert_close(pond.minerals.0[2], 0.0);
    assert!(pond.algae.carbon() > old_algae);
    let relative_credit = |index: usize| (grown[index].stock_mg_cnp[0] - original[index].stock_mg_cnp[0]) / original[index].stock_mg_cnp[0];
    assert!(0.0 < relative_credit(1) && relative_credit(1) < relative_credit(2));
    assert_close(relative_credit(1) / relative_credit(2), transmission(&original, 1) / transmission(&original, 2));
    assert_close(pond.balance.photosynthesis_oxygen_mg, pond.balance.inputs.cn_p.carbon() * OXYGEN_MG_PER_CARBON_MG);

    // Hold rates and other processes fixed. A night step debits proportional ribbon and whorled respiration.
    pond.oxygen_mg = 0.00001;
    pond.balance = Balance { initial: pond.inventory(), ..Balance::default() };
    pond.photosynthesize(0.0, 1.0);
    assert_eq!(view(pond), grown);
    let supply = pond.respire(1.0);
    assert!(supply > 0.0 && supply < 1.0);
    assert_close(pond.oxygen_mg, 0.0);
    let respired = view(pond);
    for (before, after) in grown.iter().zip(&respired) {
        let rate = if before.ribbon_preset.is_some() { 0.004 } else { 0.003 };
        let expected_loss = before.stock_mg_cnp[0] * rate * STEP_HOURS * supply;
        assert_close(before.stock_mg_cnp[0] - after.stock_mg_cnp[0], expected_loss);
    }
    let observation = pond.observe(0);
    let mut allocated = Matter(observation.unassigned_plant_mg_cnp.unwrap());
    for patch in respired { allocated.add(Matter(patch.stock_mg_cnp)); }
    for (observed, actual) in allocated.0.into_iter().zip(pond.plants.0) { assert_close(observed, actual); }
    assert_balanced(pond);
}

#[test]
fn r5_ribbon_edit_save_and_dry_restore_keep_stock_and_type() {
    let ribbon = |x| PondEdit::PlaceRibbonPlant { basin_id: basin_id(0), x, preset: PondRibbonPlantPreset::Medium };
    let mut world = WorldState::new_r5_composed_pond(2729);
    assert_eq!(serde_json::to_value(ribbon(0.8)).unwrap(), serde_json::json!({
        "type":"placeRibbonPlant", "basinId":basin_id(0), "x":0.8, "preset":"medium",
    }));
    world.edit_pond(&PondEdit::AddWaterPlant { basin_id: basin_id(0), x: 0.36 }).unwrap();
    assert!(!serde_json::to_string(&world).unwrap().contains("ribbonPreset"));
    assert!(!serde_json::to_string(&world.pond_observation().unwrap()).unwrap().contains("ribbonLightTransmission"));
    let original = world.clone();
    for x in [0.38, 0.60] {
        assert_eq!(world.edit_pond(&ribbon(x)), Err(CoreError::PondWaterPlantSiteOccupied));
        assert_eq!(world, original);
    }
    world.edit_pond(&ribbon(0.80)).unwrap();
    let planted = world.clone();
    assert_eq!(world.edit_pond(&PondEdit::MoveWaterPlant { basin_id: basin_id(0), id: 2, x: 0.40 }), Err(CoreError::PondWaterPlantSiteOccupied));
    assert_eq!(world, planted);
    world.edit_pond(&PondEdit::MoveWaterPlant { basin_id: basin_id(0), id: 2, x: 0.87 }).unwrap();
    world.edit_pond(&PondEdit::TrimWaterPlant { basin_id: basin_id(0), id: 2, mg_c: 20.0 }).unwrap();
    let remaining = basin(&world).observe(0).water_plant_patches.unwrap()[1];
    assert_eq!(remaining.ribbon_preset, Some(PondRibbonPlantPreset::Medium));
    assert_eq!(remaining.x, 0.87);
    assert_close(remaining.stock_mg_cnp[0], 30.0);
    assert_close(basin(&world).detritus.carbon(), basin(&original).detritus.carbon() + 20.0);
    world.edit_pond(&PondEdit::DrainWater { basin_id: basin_id(0), liters: 18.0 }).unwrap();
    let dry = basin(&world).observe(0).water_plant_patches.unwrap();
    assert!(!dry[1].submerged);
    assert_eq!(dry[1].ribbon_light_transmission, Some(0.0));
    world.advance_steps(20).unwrap();
    assert_eq!(basin(&world).observe(0).water_plant_patches.unwrap(), dry);
    world.edit_pond(&PondEdit::AddWater { basin_id: basin_id(0), liters: 18.0 }).unwrap();
    assert_eq!(basin(&world).observe(0).water_plant_patches.unwrap()[1].stock_mg_cnp, dry[1].stock_mg_cnp);
    let bytes = serde_json::to_string(&world).unwrap();
    assert!(bytes.contains("\"ribbonPreset\":\"medium\""));
    assert!(!bytes.contains("ribbonLightTransmission"));
    let restored: WorldState = serde_json::from_str(&bytes).unwrap();
    restored.validate().unwrap();
    assert_eq!(restored, world);
    world.edit_pond(&PondEdit::RemoveWaterPlant { basin_id: basin_id(0), id: 2 }).unwrap();
    assert_eq!(basin(&world).observe(0).water_plant_patches.unwrap().len(), 1);
    assert!(!serde_json::to_string(&world).unwrap().contains("ribbonPreset"));
    assert_balanced(basin(&world));
}

#[test]
fn r5_shrimp_stage_reads_existing_state_across_maturity_and_drainage() {
    for mut world in [WorldState::new_pond(2729), WorldState::new_r5_composed_pond(2729)] {
        let initial = basin(&world).observe(0);
        assert_eq!(initial.shrimp_life.iter().map(|shrimp| shrimp.stage).collect::<Vec<_>>(),
            [PondShrimpStage::Adult, PondShrimpStage::Juvenile, PondShrimpStage::Adult]);
        let juvenile_id = initial.shrimp[1].id.clone();
        world.pond.as_mut().unwrap().basins[0].shrimp[1].age_seconds = SHRIMP_MATURE_SECONDS - STEP_SECONDS * 0.5;
        let serialized_state = serde_json::to_string(&world).unwrap();
        assert!(!serialized_state.contains("shrimpLife"));
        let mut restored: WorldState = serde_json::from_str(&serialized_state).unwrap();
        let before = basin(&restored).observe(0);
        assert_eq!(before, basin(&world).observe(0));
        assert_eq!(before.shrimp_life[1].id, juvenile_id);
        assert_eq!(before.shrimp_life[1].stage, PondShrimpStage::Juvenile);
        let json = serde_json::to_value(&before).unwrap();
        assert_eq!(json["shrimp"][1]["id"], json["shrimpLife"][1]["id"]);
        assert_eq!(json["shrimpLife"][1]["stage"], "juvenile");
        assert_eq!(json["shrimpLife"][1]["ageSeconds"], SHRIMP_MATURE_SECONDS - STEP_SECONDS * 0.5);

        restored.advance_steps(1).unwrap();
        let after = basin(&restored).observe(0);
        assert_eq!(after.shrimp[1].id, juvenile_id);
        assert_eq!(after.shrimp_life[1].id, juvenile_id);
        assert_eq!(after.shrimp_life[1].stage, PondShrimpStage::Adult);
        assert_close(after.shrimp_life[1].age_seconds, before.shrimp_life[1].age_seconds + STEP_SECONDS);
        assert_balanced(basin(&restored));

        restored.edit_pond(&PondEdit::DrainWater { basin_id: basin_id(0), liters: after.water_liters }).unwrap();
        let dry = basin(&restored).observe(0);
        assert!(dry.shrimp.is_empty() && dry.shrimp_life.is_empty());
        assert_balanced(basin(&restored));
    }
}

#[test]
fn r5_pond_stocking_checks_sites_counts_and_elemental_inputs_atomically() {
    let shrimp = |count, position| PondEdit::StockShrimp { basin_id: basin_id(0), position, count };
    let daphnia = |count| PondEdit::StockDaphnia { basin_id: basin_id(0), count };
    let open = Point { x: 0.48, y: 0.30 };
    let mut world = WorldState::new_r5_composed_pond(2729);
    let initial = world.clone();
    let shrimp_at_cap = MAX_SHRIMP as u32 - basin(&world).shrimp.len() as u32;
    let daphnia_at_cap = MAX_DAPHNIA - basin(&world).daphnia.count;
    for (edit, expected) in [
        (shrimp(0, open), CoreError::InvalidPondStockCount),
        (shrimp(shrimp_at_cap + 1, open), CoreError::PondShrimpLimit),
        (shrimp(1, Point { x: 0.60, y: 0.95 }), CoreError::InvalidPondShrimpSite),
        (shrimp(1, Point { x: 0.48, y: 0.1 }), CoreError::InvalidPondShrimpSite),
        (shrimp(1, Point { x: f64::NAN, y: 0.3 }), CoreError::InvalidPondShrimpSite),
        (daphnia(0), CoreError::InvalidPondStockCount),
        (daphnia(daphnia_at_cap + 1), CoreError::PondDaphniaLimit),
    ] {
        assert_eq!(world.edit_pond(&edit), Err(expected));
        assert_eq!(world, initial);
    }
    let mut blank = WorldState::new_blank_r5_composed_pond(2729);
    assert_eq!(blank.edit_pond(&shrimp(1, open)), Err(CoreError::PondStockNeedsWater));
    assert_eq!(blank.edit_pond(&daphnia(1)), Err(CoreError::PondStockNeedsWater));
    blank.edit_pond(&PondEdit::AddWater { basin_id: basin_id(0), liters: 12.0 }).unwrap();
    assert!(blank.starter_community_available());
    let empty = blank.clone();
    blank.edit_pond(&daphnia(4)).unwrap();
    assert!(!blank.starter_community_available());
    assert_eq!(blank.add_starter_community(), Err(CoreError::StarterCommunityUnavailable));
    blank.restore_pond_edit_state(empty.clone()).unwrap();
    assert!(blank.starter_community_available());
    blank.edit_pond(&shrimp(1, open)).unwrap();
    assert!(!blank.starter_community_available());
    blank.restore_pond_edit_state(empty).unwrap();
    assert!(blank.starter_community_available());
    let mut legacy = WorldState::new_r5_moss_pond(2729);
    assert_eq!(legacy.edit_pond(&shrimp(1, open)), Err(CoreError::WrongWorldKind));
    assert_eq!(legacy.edit_pond(&daphnia(1)), Err(CoreError::WrongWorldKind));

    let stock_before = basin(&world).ledger();
    world.edit_pond(&shrimp(2, open)).unwrap();
    world.edit_pond(&daphnia(12)).unwrap();
    let stocked = basin(&world);
    assert_eq!(stocked.daphnia.count, 60);
    assert_eq!(stocked.shrimp.len(), 5);
    assert!(stocked.shrimp[3..].iter().all(|shrimp| shrimp.stage() == PondShrimpStage::Adult
        && animal_point(&shrimp.view) == open));
    let old_ids: Vec<_> = stocked.shrimp[3..].iter().map(|shrimp| shrimp.view.id.clone()).collect();
    assert_ne!(old_ids[0], old_ids[1]);
    let (body, reserve) = Shrimp::starter_matter(true);
    let starter = Daphnia::new();
    let expected_input = std::array::from_fn::<_, 3, _>(|index|
        2.0 * (body.0[index] + reserve.0[index]) + 0.25 * (starter.body.0[index] + starter.reserve.0[index]));
    let stock_after = stocked.ledger();
    assert_close(stock_before.current_water_liters, stock_after.current_water_liters);
    assert_close(stock_before.current_oxygen_mg, stock_after.current_oxygen_mg);
    for (index, expected) in expected_input.into_iter().enumerate() {
        assert_close(stock_after.matter_input_mg_cnp[index] - stock_before.matter_input_mg_cnp[index], expected);
        assert_close(stock_after.current_matter_mg_cnp[index] - stock_before.current_matter_mg_cnp[index], expected);
    }
    assert_balanced(stocked);

    world.restore_pond_edit_state(initial.clone()).unwrap();
    assert_eq!(basin(&world).ledger(), stock_before);
    assert_eq!(basin(&world).observe(0), basin(&initial).observe(0));
    world.edit_pond(&shrimp(2, open)).unwrap();
    assert!(basin(&world).shrimp[3..].iter().all(|shrimp| !old_ids.contains(&shrimp.view.id)));
    let read_back: WorldState = serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
    assert_eq!(read_back.pond_observation(), world.pond_observation());
    assert_balanced(basin(&read_back));
}

#[test]
fn r5_moss_add_move_trim_and_both_removal_paths_conserve_each_element() {
    let mut world = WorldState::new_r5_moss_pond(2729);
    let add_stone = PondEdit::AddStone { basin_id: basin_id(0), x: 0.37, preset: PondStonePreset::Main };
    let add_wood = PondEdit::AddWood { basin_id: basin_id(0), x: 0.74, preset: PondWoodPreset::Root };
    world.edit_pond(&add_stone).unwrap();
    world.edit_pond(&add_wood).unwrap();
    let rock = PondMossParent::Stone { id: 1 };
    let root = PondMossParent::Wood { id: 1 };
    let add = |parent, offset, preset| PondEdit::AddMoss { basin_id: basin_id(0), parent, offset, preset };
    let pristine = world.clone();
    assert_eq!(world.edit_pond(&add(root, 1.1, PondMossPreset::Mat)), Err(CoreError::InvalidPondMossSite));
    assert_eq!(world.edit_pond(&add(PondMossParent::Stone { id: 99 }, 0.0, PondMossPreset::Mat)), Err(CoreError::PondMossParentMissing));
    assert_eq!(world, pristine);

    let mat = PondMossPreset::Mat.initial_mg_cnp();
    let frond = PondMossPreset::Frond.initial_mg_cnp();
    world.edit_pond(&add(root, 0.20, PondMossPreset::Mat)).unwrap();
    world.edit_pond(&add(rock, -0.40, PondMossPreset::Frond)).unwrap();
    assert_eq!(world.edit_pond(&add(root, 0.40, PondMossPreset::Frond)), Err(CoreError::PondMossSiteOccupied));
    assert_eq!(basin(&world).ledger().matter_input_mg_cnp, std::array::from_fn(|index| mat[index] + frond[index]));
    let patches = basin(&world).observe(0).moss_patches.unwrap();
    assert_eq!(patches[0].stock_mg_cnp, mat);
    assert_eq!(patches[1].stock_mg_cnp, frond);
    assert!(patches.iter().all(|patch| patch.submerged));
    assert_close(patches[0].x, 0.74 + 0.20 * 25.0 / 288.0);
    assert_eq!(patches[0].surface_height_px, 15.0);

    let before = basin(&world).ledger();
    world.edit_pond(&PondEdit::MoveWood { basin_id: basin_id(0), id: 1, x: 0.72 }).unwrap();
    world.edit_pond(&PondEdit::MoveMoss { basin_id: basin_id(0), id: patches[0].id, offset: 0.50 }).unwrap();
    let moved = basin(&world).observe(0).moss_patches.unwrap();
    assert_close(moved[0].x, 0.72 + 0.50 * 25.0 / 288.0);
    assert_eq!(moved[0].stock_mg_cnp, mat);
    assert_eq!(moved[1], patches[1]);
    assert_eq!(basin(&world).ledger(), before);

    let trim = |mg_c| PondEdit::TrimMoss { basin_id: basin_id(0), id: moved[0].id, mg_c };
    let before_trim = world.clone();
    assert_eq!(world.edit_pond(&trim(mat[0])), Err(CoreError::InvalidPondMossTrim));
    assert_eq!(world, before_trim);
    let detritus = basin(&world).detritus;
    world.edit_pond(&trim(1.0)).unwrap();
    let half_edited = basin(&world).observe(0).moss_patches.unwrap();
    assert_eq!(half_edited[0].stock_mg_cnp[0], 2.0);
    for i in 0..3 { assert_close(basin(&world).detritus.0[i] - detritus.0[i], mat[i] / 3.0); }
    let fresh = world.fresh_habitat(172).unwrap();
    let fresh_base = pristine.fresh_habitat(172).unwrap();
    assert_eq!(basin(&fresh).observe(0).moss_patches.unwrap()[0].stock_mg_cnp, mat);
    assert_eq!(basin(&fresh).ledger().matter_input_mg_cnp, [0.0; 3]);
    for i in 0..3 {
        assert_close(basin(&fresh).ledger().initial_matter_mg_cnp[i] - basin(&fresh_base).ledger().initial_matter_mg_cnp[i], mat[i] + frond[i]);
    }

    let edited = world.clone();
    world.edit_pond(&PondEdit::RemoveWood { basin_id: basin_id(0), id: 1 }).unwrap();
    world.edit_pond(&PondEdit::RemoveMoss { basin_id: basin_id(0), id: moved[1].id }).unwrap();
    assert!(basin(&world).observe(0).moss_patches.unwrap().is_empty());
    for i in 0..3 {
        assert_close(basin(&world).detritus.0[i] - detritus.0[i], mat[i] + frond[i]);
        assert_close(basin(&world).ledger().current_matter_mg_cnp[i], before.current_matter_mg_cnp[i]);
    }
    assert_eq!(basin(&world).ledger().matter_output_mg_cnp, before.matter_output_mg_cnp);
    assert_balanced(basin(&world));
    world.restore_pond_edit_state(edited).unwrap();
    assert_eq!(basin(&world).observe(0).moss_patches.as_ref(), Some(&half_edited));
    world.restore_pond_edit_state(pristine).unwrap();
    world.edit_pond(&add(root, 0.20, PondMossPreset::Mat)).unwrap();
    assert!(basin(&world).observe(0).moss_patches.unwrap()[0].id > moved[1].id);
    world.validate().unwrap();
}

#[test]
fn r5_submerged_moss_competes_for_minerals_and_shares_limited_oxygen() {
    let mut world = WorldState::new_r5_moss_pond(2729);
    world.edit_pond(&PondEdit::AddStone { basin_id: basin_id(0), x: 0.37, preset: PondStonePreset::Main }).unwrap();
    world.edit_pond(&PondEdit::AddMoss {
        basin_id: basin_id(0), parent: PondMossParent::Stone { id: 1 }, offset: 0.0, preset: PondMossPreset::Frond,
    }).unwrap();
    let pond = &mut world.pond.as_mut().unwrap().basins[0];
    pond.minerals = Matter([0.0, 0.00000001, 0.00000001]);
    pond.balance = Balance { initial: pond.inventory(), ..Balance::default() };
    let mat_before = pond.moss_layout.as_ref().unwrap().patches[0].stock_mg_cnp;
    let plant_before = pond.plants;
    let carbon_before = pond.inventory().cn_p.carbon();
    pond.photosynthesize(1.0, 1.0);
    let mat_after = pond.moss_layout.as_ref().unwrap().patches[0].stock_mg_cnp;
    assert!(mat_after.carbon() > mat_before.carbon() && pond.plants.carbon() > plant_before.carbon());
    assert!(pond.minerals.0[1] < 1e-15);
    let carbon_input = pond.balance.inputs.cn_p.carbon();
    assert!((pond.inventory().cn_p.carbon() - carbon_before - carbon_input).abs() < 1e-12);
    assert!((carbon_input * OXYGEN_MG_PER_CARBON_MG - pond.balance.photosynthesis_oxygen_mg).abs() < 1e-15);
    assert_balanced(pond);

    pond.oxygen_mg = 0.0000002;
    pond.balance = Balance { initial: pond.inventory(), ..Balance::default() };
    let before_o2 = pond.oxygen_mg;
    let mut expected_patch = pond.moss_layout.as_ref().unwrap().patches[0];
    let moss_demand = expected_patch.respiration_demand(STEP_HOURS);
    let mut without_moss = pond.clone();
    without_moss.moss_layout.as_mut().unwrap().patches.clear();
    let other_demand = before_o2 / without_moss.respire(1.0);
    let supply = pond.respire(1.0);
    let patch = pond.moss_layout.as_ref().unwrap().patches[0];
    assert!((0.0..1.0).contains(&supply));
    let expected_supply = before_o2 / (other_demand + moss_demand * OXYGEN_MG_PER_CARBON_MG);
    assert!((supply - expected_supply).abs() < 1e-12);
    let fuel = expected_patch.stock_mg_cnp.take_carbon(moss_demand * expected_supply);
    let senescence = expected_patch.senesce(STEP_HOURS);
    assert!(fuel.carbon() > 0.0 && senescence.carbon() > 0.0);
    assert!((patch.stock_mg_cnp.carbon() - expected_patch.stock_mg_cnp.carbon()).abs() < 1e-12);
    assert!(pond.balance.producer_respiration_oxygen_mg > 0.0 && pond.balance.animal_respiration_oxygen_mg > 0.0);
    assert!((pond.balance.outputs.oxygen_mg - before_o2).abs() < 1e-15 && pond.oxygen_mg < 1e-15);
    assert_balanced(pond);
}

#[test]
fn r5_moss_survives_emergence_without_treating_it_as_submerged_growth() {
    let mut world = WorldState::new_r5_moss_pond(2729);
    let place_stone = |x, preset| PondEdit::AddStone { basin_id: basin_id(0), x, preset };
    world.edit_pond(&place_stone(0.37, PondStonePreset::Main)).unwrap();
    world.edit_pond(&place_stone(0.65, PondStonePreset::Pebble)).unwrap();
    let add = |id| PondEdit::AddMoss {
        basin_id: basin_id(0), parent: PondMossParent::Stone { id }, offset: 0.0, preset: PondMossPreset::Mat,
    };
    assert_eq!(world.edit_pond(&add(2)), Err(CoreError::InvalidPondMossSite));
    world.edit_pond(&add(1)).unwrap();
    let initial = basin(&world).observe(0).moss_patches.unwrap()[0];
    let drain = |liters| PondEdit::DrainWater { basin_id: basin_id(0), liters };
    world.edit_pond(&drain(17.9)).unwrap();
    world.advance_steps(300).unwrap();
    let exposed = basin(&world).observe(0).moss_patches.unwrap()[0];
    assert_eq!(exposed, PondMossPatchObservation { submerged: false, ..initial });
    world.edit_pond(&drain(basin(&world).water_liters)).unwrap();
    assert_eq!(basin(&world).observe(0).moss_patches.unwrap()[0], exposed);
    world.edit_pond(&PondEdit::AddWater { basin_id: basin_id(0), liters: 18.0 }).unwrap();
    world.advance_steps(1800).unwrap();
    let restored = basin(&world).observe(0).moss_patches.unwrap()[0];
    assert!(restored.submerged && restored.stock_mg_cnp[0] > initial.stock_mg_cnp[0]);
    assert_balanced(basin(&world));
}

#[test]
fn r5_pond_with_empty_moss_sites_replays_the_previous_wood_definition() {
    let mut old = WorldState::new_r5_wood_pond(2729);
    let mut new = WorldState::new_r5_moss_pond(2729);
    let plank = PondEdit::AddWood { basin_id: basin_id(0), x: 0.50, preset: PondWoodPreset::Branch };
    for world in [&mut old, &mut new] {
        world.edit_pond(&plank).unwrap();
        world.advance_steps(2500).unwrap();
        world.validate().unwrap();
    }
    let mut observed = basin(&new).observe(0);
    assert_eq!(observed.moss_patches.take(), Some(Vec::new()));
    assert!(observed.moss_presets.take().is_some());
    assert_eq!(observed, basin(&old).observe(0));
    assert_eq!(basin(&old).random, basin(&new).random);
    assert_eq!(old.pond_ledger(), new.pond_ledger());
}

#[test]
fn empty_r5_stone_basin_preserves_the_legacy_trajectory() {
    let mut legacy = WorldState::new_pond(2729);
    let mut r5 = WorldState::new_r5_stone_pond(2729);
    let solid = PondEdit::AddStone { basin_id: basin_id(0), x: 0.37, preset: PondStonePreset::Main };
    assert_eq!(legacy.edit_pond(&solid), Err(CoreError::WrongWorldKind));
    legacy.advance_steps(2500).unwrap();
    r5.advance_steps(2500).unwrap();
    let old = basin(&legacy).observe(0);
    let mut new = basin(&r5).observe(0);
    assert_eq!(new.stones.take(), Some(Vec::new()));
    assert_eq!(old, new);
    assert_eq!(basin(&r5).random, basin(&legacy).random);
    assert_eq!(r5.pond_ledger(), legacy.pond_ledger());
    legacy.validate().unwrap();
    r5.validate().unwrap();
}

#[test]
fn r5_shrimp_swims_clear_of_a_main_stone_and_passes_after_removal() {
    let mut pond = WorldState::new_r5_stone_pond(2729);
    let edit = PondEdit::AddStone { basin_id: basin_id(0), x: 0.50, preset: PondStonePreset::Main };
    pond.edit_pond(&edit).unwrap();
    let basin = &mut pond.pond.as_mut().unwrap().basins[0];
    let column = basin.column_projection_px();
    let left = 0.50 - 40.0 / 288.0;
    let right = 0.50 + 40.0 / 288.0;
    basin.shrimp[0].view.x = left - 0.0005;
    basin.shrimp[0].view.y = 0.80;
    basin.shrimp[0].view.heading = 0.0;
    basin.random = RandomState::new(0);
    assert!(!basin.stone_layout.as_ref().unwrap().profiles().any(|profile| profile.blocks(animal_point(&basin.shrimp[0].view), column)));
    let stone_id = basin.stone_layout.as_ref().unwrap().stones[0].id;
    let mut removed = pond.clone();
    removed.edit_pond(&PondEdit::RemoveStone { basin_id: basin_id(0), id: stone_id }).unwrap();
    pond.advance_steps(1).unwrap();
    removed.advance_steps(1).unwrap();
    assert!(self::basin(&pond).shrimp[0].view.y < 0.80);
    assert!(self::basin(&pond).shrimp[0].view.x < left);
    assert!(self::basin(&removed).shrimp[0].view.x > left);
    let mut over_roof = false;
    let mut crossed = false;
    for _ in 0..2500 {
        let from = animal_point(&self::basin(&pond).shrimp[0].view);
        pond.advance_steps(1).unwrap();
        let basin = self::basin(&pond);
        let to = animal_point(&basin.shrimp[0].view);
        let layout = basin.stone_layout.as_ref().unwrap();
        assert!(!layout.profiles().any(|profile| profile.blocks_segment(from, to, basin.column_projection_px())));
        if (left..=right).contains(&to.x) {
            over_roof = true;
            assert!(to.y < 1.0 - 27.0 / basin.column_projection_px());
        }
        if over_roof && to.x > right {
            crossed = true;
            break;
        }
    }
    assert!(over_roof && crossed, "the shrimp should take a route over the stone to the other side");
    removed.advance_steps(u32::try_from(pond.tick() - removed.tick()).unwrap()).unwrap();
    assert_eq!(pond.pond_ledger(), removed.pond_ledger());
    pond.validate().unwrap();
}

#[test]
fn r5_stone_edits_are_atomic_and_inert_while_water_level_changes() {
    let mut pond = WorldState::new_r5_stone_pond(2729);
    let add_main = PondEdit::AddStone { basin_id: basin_id(0), x: 0.37, preset: PondStonePreset::Main };
    let add_pebble = PondEdit::AddStone { basin_id: basin_id(0), x: 0.65, preset: PondStonePreset::Pebble };
    let with_animals = pond.pond_observation().unwrap().basins[0].shrimp.clone();
    let initial_balance = pond.pond_ledger();
    pond.edit_pond(&add_main).unwrap();
    pond.edit_pond(&add_pebble).unwrap();
    let edited = pond.clone();
    let placed = basin(&pond).observe(0).stones.unwrap();
    assert_eq!(basin(&pond).observe(0).shrimp, with_animals);
    assert_eq!(pond.pond_ledger(), initial_balance);
    let move_main = |x| PondEdit::MoveStone { basin_id: basin_id(0), id: placed[0].id, x };
    assert_eq!(pond.edit_pond(&move_main(0.25)), Err(CoreError::InvalidPondStoneSite));
    assert_eq!(pond.edit_pond(&move_main(0.59)), Err(CoreError::OverlappingPondStoneSite));
    assert_eq!(pond, edited);

    let drain = |liters| PondEdit::DrainWater { basin_id: basin_id(0), liters };
    let mut no_structures = WorldState::new_pond(2729);
    for world in [&mut pond, &mut no_structures] {
        world.edit_pond(&drain(17.9)).unwrap();
        world.validate().unwrap();
    }
    let shallow = basin(&pond).observe(0);
    assert!(shallow.wet);
    assert_ne!(shallow.shrimp, with_animals);
    assert_eq!(shallow.stones.as_ref(), Some(&placed));
    let animal_x = shallow.shrimp[2].x;
    let move_to_animal = PondEdit::MoveStone { basin_id: basin_id(0), id: placed[1].id, x: animal_x };
    assert_eq!(pond.edit_pond(&move_to_animal), Err(CoreError::OccupiedPondStoneSite));
    assert_eq!(basin(&pond).observe(0), shallow);
    let remaining = shallow.water_liters;
    for world in [&mut pond, &mut no_structures] {
        world.edit_pond(&drain(remaining)).unwrap();
        world.validate().unwrap();
    }
    let dry = basin(&pond).observe(0);
    assert!(!dry.wet && dry.shrimp.is_empty());
    assert_eq!(dry.stones.as_ref(), Some(&placed));
    let preset = pond.fresh_habitat(190).unwrap();
    assert_eq!(basin(&preset).observe(0).stones, dry.stones);
    assert!(!basin(&preset).wet());
    for world in [&mut pond, &mut no_structures] {
        world.edit_pond(&PondEdit::AddWater { basin_id: basin_id(0), liters: 18.0 }).unwrap();
        world.validate().unwrap();
    }
    assert_eq!(basin(&pond).observe(0).stones, dry.stones);
    assert!(basin(&pond).observe(0).shrimp.is_empty());
    assert_eq!(pond.pond_ledger(), no_structures.pond_ledger());
}

#[test]
fn r5_pond_edit_history_preserves_daylight_and_does_not_reuse_stone_ids() {
    let mut pond = WorldState::new_r5_stone_pond_with_basins(2729, 2).unwrap();
    let second_before = pond.pond_observation().unwrap().basins[1].clone();
    let old = pond.clone();
    let add = PondEdit::AddStone { basin_id: basin_id(0), x: 0.37, preset: PondStonePreset::Main };
    pond.edit_pond(&add).unwrap();
    let stocked = pond.clone();
    let first = basin(&pond).observe(0).stones.unwrap()[0];
    pond.set_daylight_target(0.66).unwrap();
    pond.restore_pond_edit_state(old.clone()).unwrap();
    assert!(basin(&pond).observe(0).stones.unwrap().is_empty());
    pond.restore_pond_edit_state(stocked.clone()).unwrap();
    assert_eq!(basin(&pond).observe(0).stones, Some(vec![first]));
    pond.restore_pond_edit_state(old).unwrap();
    pond.edit_pond(&add).unwrap();
    assert!(basin(&pond).observe(0).stones.unwrap()[0].id > first.id);
    assert_eq!(pond.daylight_target(), Some(0.66));
    assert_eq!(pond.pond_observation().unwrap().basins[1], second_before);
    pond.advance_steps(1).unwrap();
    let advanced = pond.clone();
    assert_eq!(pond.restore_pond_edit_state(stocked), Err(CoreError::InvalidPondState));
    assert_eq!(pond, advanced);
}

#[test]
fn r5_empty_wood_layout_retains_the_complete_stone_pond_trajectory() {
    let mut stones = WorldState::new_r5_stone_pond(2729);
    let mut woods = WorldState::new_r5_wood_pond(2729);
    let add = PondEdit::AddWood { basin_id: basin_id(0), x: 0.50, preset: PondWoodPreset::Root };
    assert_eq!(stones.edit_pond(&add), Err(CoreError::WrongWorldKind));
    for world in [&mut stones, &mut woods] {
        world.edit_pond(&PondEdit::AddStone { basin_id: basin_id(0), x: 0.37, preset: PondStonePreset::Main }).unwrap();
        world.advance_steps(2500).unwrap();
        world.validate().unwrap();
    }
    let mut current = basin(&woods).observe(0);
    assert_eq!(current.woods.take(), Some(Vec::new()));
    assert_eq!(current.shrimp_attachments.take(), Some(Vec::new()));
    assert_eq!(current, basin(&stones).observe(0));
    assert_eq!(basin(&woods).random, basin(&stones).random);
    assert_eq!(woods.pond_ledger(), stones.pond_ledger());
}

#[test]
fn r5_wood_contact_reports_consumed_parcels_and_excludes_new_or_released_contacts() {
    let mut world = WorldState::new_r5_composed_dual_pond(2729);
    let local = &mut world.pond.as_mut().unwrap().basins[0];
    let wood = local.wood_layout.as_ref().unwrap().woods[0];
    let depth = local.column_projection_px();
    let (offset, site) = [-0.75, -0.5, 0.0, 0.5, 0.75].into_iter()
        .find_map(|offset| wood.surface(offset, depth).map(|site| (offset, site))).unwrap();
    let animal = &mut local.shrimp[0].view;
    animal.x = site.x;
    animal.y = site.y - 1.0 / depth;
    animal.heading = std::f64::consts::FRAC_PI_2;
    let shrimp_id = animal.id.clone();
    local.random = RandomState::new(42);

    // This step settles the shrimp after allocating food, so it produces no contact-fed record.
    assert!(world.advance_steps_with_pond_contact_feeding(1).unwrap().is_empty());
    let current = basin(&world);
    let attachment = &current.wood_layout.as_ref().unwrap().attachments[0];
    assert_eq!(attachment.shrimp_id, shrimp_id);
    assert_close(attachment.offset, offset);
    let previous_food = current.balance.shrimp_food_mg_c;
    let mut control = world.clone();
    let mut zero = world.clone();
    let local = &mut zero.pond.as_mut().unwrap().basins[0];
    local.biofilm = Matter::default();
    assert!(zero.advance_steps_with_pond_contact_feeding(1).unwrap().is_empty());

    let observed = world.advance_steps_with_pond_contact_feeding(1).unwrap();
    let [(id, entries)] = observed.as_slice() else { panic!("expected one active basin") };
    let [event] = entries.wood.as_slice() else { panic!("expected only the already attached shrimp") };
    assert!(entries.stone.is_empty());
    assert_eq!(id, &basin_id(0));
    assert_eq!((&event.shrimp_id, event.wood_id), (&shrimp_id, wood.id));
    assert_close(event.offset, offset);
    assert!(event.biofilm_mg_c > 0.0);
    assert!(event.wood_biofilm_mg_c > 0.0);
    assert_eq!(event.wood_biofilm_mg_c + event.shared_biofilm_mg_c, event.biofilm_mg_c);
    assert!(event.biofilm_mg_c < basin(&world).balance.shrimp_food_mg_c - previous_food);
    control.advance_steps(1).unwrap();
    assert_eq!(world, control);
    assert_eq!(world.pond_ledger(), control.pond_ledger());
    assert_eq!(serde_json::to_vec(&world).unwrap(), serde_json::to_vec(&control).unwrap());
    assert_balanced(basin(&world));

    // A positive prior event cannot be replayed when the current tick loses the contact.
    world.pond.as_mut().unwrap().basins[0].wood_layout.as_mut().unwrap().attachments[0].rest_ticks = 1;
    assert!(world.advance_steps_with_pond_contact_feeding(1).unwrap().is_empty());
    assert!(basin(&world).wood_layout.as_ref().unwrap().attachments.is_empty());
}

fn approach_main_stone(world: &mut WorldState, offset: f64) -> PondShrimpStoneAttachment {
    let basin = &mut world.pond.as_mut().unwrap().basins[0];
    let layout = basin.stone_layout.as_ref().unwrap();
    let depth = basin.column_projection_px();
    let shrimp = &mut basin.shrimp[0].view;
    let attachment = PondShrimpStoneAttachment {
        shrimp_id: shrimp.id.clone(), stone_id: layout.stones[0].id, offset, rest_ticks: 35,
    };
    let site = attachment.point(&layout.stones, depth).unwrap();
    shrimp.x = site.x;
    shrimp.y = site.y - 1.0 / depth;
    shrimp.heading = std::f64::consts::FRAC_PI_2;
    basin.random = RandomState::new(42);
    attachment
}

#[test]
fn r5_main_stone_contact_persists_without_retrospectively_attributing_food() {
    let mut world = WorldState::new_r5_composed_dual_pond(2729);
    let expected = approach_main_stone(&mut world, 0.6);
    assert!(world.advance_steps_with_pond_contact_feeding(1).unwrap().is_empty());
    let perched = basin(&world).observe(0);
    let [attached] = perched.stone_attachments.as_deref().unwrap() else { panic!("expected one main stone contact") };
    assert_eq!(attached.shrimp_id, expected.shrimp_id);
    assert_eq!(attached.stone_id, expected.stone_id);
    assert_close(attached.offset, expected.offset);
    let local = basin(&world);
    let layout = local.stone_layout.as_ref().unwrap();
    let depth = local.column_projection_px();
    let shrimp = &local.shrimp[0].view;
    assert!(attached.matches(&layout.stones, shrimp, depth));
    assert!(!local.ground_profiles().any(|profile| profile.blocks(animal_point(shrimp), depth)));
    world.validate().unwrap();
    assert!(perched.stone_contact_feeding.is_none());
    let checkpoint = serde_json::to_vec(&world).unwrap();
    let restored: WorldState = serde_json::from_slice(&checkpoint).unwrap();
    restored.validate().unwrap();
    assert_eq!(world, restored);
    assert!(basin(&world.fresh_habitat(591).unwrap()).observe(0).stone_attachments.is_none());

    let previous_food = local.balance.shrimp_food_mg_c;
    let mut control = restored;
    let mut empty = world.clone();
    empty.pond.as_mut().unwrap().basins[0].biofilm = Matter::default();
    assert!(empty.advance_steps_with_pond_contact_feeding(1).unwrap().is_empty());
    let observed = world.advance_steps_with_pond_contact_feeding(1).unwrap();
    let [(id, entries)] = observed.as_slice() else { panic!("expected only the active basin") };
    let [entry] = entries.stone.as_slice() else { panic!("expected one actual stone parcel") };
    assert!(entries.wood.is_empty());
    assert_eq!(id, &basin_id(0));
    assert_eq!((&entry.shrimp_id, entry.stone_id), (&expected.shrimp_id, expected.stone_id));
    assert!(entry.stone_biofilm_mg_c > 0.0);
    assert_close(entry.stone_biofilm_mg_c + entry.shared_biofilm_mg_c, entry.biofilm_mg_c);
    assert!(entry.biofilm_mg_c < basin(&world).balance.shrimp_food_mg_c - previous_food);
    control.advance_steps(1).unwrap();
    assert_eq!(world, control);
    assert_balanced(basin(&world));

    let mut released = world.clone();
    released.pond.as_mut().unwrap().basins[0].stone_layout.as_mut().unwrap().stone_attachments[0].rest_ticks = 1;
    assert!(released.advance_steps_with_pond_contact_feeding(1).unwrap().is_empty());
    assert!(basin(&released).observe(0).stone_attachments.is_none());
}

#[test]
fn r5_stone_contact_handles_moved_removed_and_shallow_surfaces() {
    let mut world = WorldState::new_r5_composed_pond(2729);
    world.edit_pond(&PondEdit::RemoveWood { basin_id: basin_id(0), id: 1 }).unwrap();
    approach_main_stone(&mut world, 0.0);
    world.advance_steps(1).unwrap();
    let local = basin(&world);
    let attached = local.observe(0).stone_attachments.unwrap().remove(0);
    let original_pose = animal_point(&local.shrimp[0].view);
    let stone_id = attached.stone_id;

    let mut moved = world.clone();
    assert!(moved.edit_pond(&PondEdit::MoveStone { basin_id: basin_id(0), id: stone_id, x: 0.95 }).is_err());
    assert_eq!(moved, world);
    moved.edit_pond(&PondEdit::MoveStone { basin_id: basin_id(0), id: stone_id, x: 0.30 }).unwrap();
    moved.validate().unwrap();
    assert!(basin(&moved).observe(0).stone_attachments.is_none());
    assert_eq!(animal_point(&basin(&moved).shrimp[0].view), original_pose);
    assert!(moved.advance_steps_with_pond_contact_feeding(1).unwrap().is_empty());

    let mut removed = world.clone();
    removed.edit_pond(&PondEdit::RemoveStone { basin_id: basin_id(0), id: stone_id }).unwrap();
    removed.validate().unwrap();
    assert!(basin(&removed).observe(0).stone_attachments.is_none());
    assert!(removed.advance_steps_with_pond_contact_feeding(1).unwrap().is_empty());

    world.edit_pond(&PondEdit::DrainWater { basin_id: basin_id(0), liters: 1.0 }).unwrap();
    let local = basin(&world);
    let layout = local.stone_layout.as_ref().unwrap();
    assert_eq!(layout.stone_attachments, [attached.clone()]);
    assert!(attached.matches(&layout.stones, &local.shrimp[0].view, local.column_projection_px()));
    assert_ne!(animal_point(&local.shrimp[0].view), original_pose);
    let shallow_liters = 2.0;
    world.edit_pond(&PondEdit::DrainWater { basin_id: basin_id(0), liters: local.water_liters - shallow_liters }).unwrap();
    world.validate().unwrap();
    assert!(basin(&world).observe(0).stone_attachments.is_none());
    assert!(world.advance_steps_with_pond_contact_feeding(1).unwrap().is_empty());
    let remaining = basin(&world).water_liters;
    world.edit_pond(&PondEdit::DrainWater { basin_id: basin_id(0), liters: remaining }).unwrap();
    assert!(basin(&world).observe(0).stone_attachments.is_none());
    world.validate().unwrap();
    assert_balanced(basin(&world));
}

#[test]
fn r5_legacy_composed_stone_rest_is_explicit_and_basin_local() {
    let enable = PondEdit::EnableStoneRest { basin_id: basin_id(0) };
    let mut older = WorldState::new_r5_wood_pond(2729);
    assert_eq!(older.edit_pond(&enable), Err(CoreError::WrongWorldKind));

    let mut composed = WorldState::new_r5_composed_dual_pond(2729);
    approach_main_stone(&mut composed, 0.6);
    let mut checkpoint = serde_json::to_value(composed).unwrap();
    for basin in checkpoint["pond"]["basins"].as_array_mut().unwrap() {
        basin["stoneLayout"].as_object_mut().unwrap().remove("stoneRestEnabled");
    }
    let mut legacy: WorldState = serde_json::from_value(checkpoint.clone()).unwrap();
    legacy.validate().unwrap();
    assert_eq!(serde_json::to_value(&legacy).unwrap(), checkpoint);
    assert!(legacy.pond_observation().unwrap().basins.iter().all(|basin| basin.stone_rest_enabled.is_none()));
    let mut enabled = legacy.clone();
    enabled.edit_pond(&enable).unwrap();
    enabled.edit_pond(&enable).unwrap();
    let capabilities = enabled.pond_observation().unwrap().basins.iter()
        .map(|basin| basin.stone_rest_enabled).collect::<Vec<_>>();
    assert_eq!(capabilities, [Some(true), None]);
    legacy.advance_steps(1).unwrap();
    enabled.advance_steps(1).unwrap();
    assert!(basin(&legacy).observe(0).stone_attachments.is_none());
    assert!(basin(&enabled).observe(0).stone_attachments.is_some());
    assert_eq!(legacy.pond.as_ref().unwrap().basins[1], enabled.pond.as_ref().unwrap().basins[1]);
}

#[test]
fn r5_persistent_stone_film_spends_only_accessible_stock_and_shares_the_remainder() {
    let mut world = WorldState::new_blank_r5_composed_pond(2729);
    world.edit_pond(&PondEdit::AddWater { basin_id: basin_id(0), liters: 18.0 }).unwrap();
    for x in [0.30, 0.65] {
        world.edit_pond(&PondEdit::AddStone { basin_id: basin_id(0), x, preset: PondStonePreset::Main }).unwrap();
    }
    world.edit_pond(&PondEdit::EnableStoneBiofilm { basin_id: basin_id(0) }).unwrap();
    let local = &mut world.pond.as_mut().unwrap().basins[0];
    let depth = local.column_projection_px();
    for (offset, adult) in [(-0.7, true), (0.7, false)] {
        let id = local.allocate_shrimp_id();
        let contact = PondShrimpStoneAttachment { shrimp_id: id.clone(), stone_id: 1, offset, rest_ticks: 20 };
        let stone_layout = local.stone_layout.as_mut().unwrap();
        let site = contact.point(&stone_layout.stones, depth).unwrap();
        let (body, reserve) = Shrimp::starter_matter(adult);
        local.shrimp.push(Shrimp::new(id, site.x, site.y, 0.0, body, reserve, if adult { SHRIMP_MATURE_SECONDS } else { 0.0 }));
        stone_layout.stone_attachments.push(contact);
    }
    let id = local.allocate_shrimp_id();
    let (body, reserve) = Shrimp::starter_matter(true);
    local.shrimp.push(Shrimp::new(id, 0.50, 0.25, 0.0, body, reserve, SHRIMP_MATURE_SECONDS));
    let (local_food, shared_food, inaccessible) = (0.000_005, 0.000_01, 0.2);
    local.biofilm = Matter::with_ratios(shared_food, 0.077, 0.010);
    let patches = local.stone_biofilm.as_mut().unwrap();
    patches[0].stock = Matter::with_ratios(local_food, 0.077, 0.010);
    patches[1].stock = Matter::with_ratios(inaccessible, 0.077, 0.010);
    local.balance = Balance { initial: local.inventory(), ..Balance::default() };
    let mut headless = local.clone();
    let mut traced = PondBasinContactFeeding::default();
    let observed = local.feed(1.0, 1.0, Some(&mut traced));
    assert_eq!(observed, headless.feed(1.0, 1.0, None));
    assert!(observed[..3].iter().all(|fed| *fed));
    assert_eq!(*local, headless);
    let [first, second] = traced.stone.as_slice() else { panic!("expected two consumers on stone 1") };
    assert_eq!([first.stone_id, second.stone_id], [1, 1]);
    assert_close(first.stone_biofilm_mg_c + second.stone_biofilm_mg_c, local_food);
    assert_close(first.stone_biofilm_mg_c / second.stone_biofilm_mg_c, 1.5);
    for entry in [first, second] {
        assert!(entry.stone_biofilm_mg_c > 0.0 && entry.shared_biofilm_mg_c > 0.0);
        assert_close(entry.stone_biofilm_mg_c + entry.shared_biofilm_mg_c, entry.biofilm_mg_c);
    }
    assert_eq!(local.biofilm.carbon(), 0.0);
    assert_eq!(local.stone_biofilm.as_ref().unwrap()[0].stock.carbon(), 0.0);
    assert_eq!(local.stone_biofilm.as_ref().unwrap()[1].stock.carbon(), inaccessible);
    assert_close(local.balance.shrimp_food_mg_c, local_food + shared_food);
    assert_balanced(local);
    world.validate().unwrap();
}

#[test]
fn r5_persistent_stone_film_recolonizes_from_real_stock_and_grows_under_shared_limits() {
    let mut world = WorldState::new_r5_composed_pond(2729);
    world.edit_pond(&PondEdit::EnableStoneBiofilm { basin_id: basin_id(0) }).unwrap();
    let local = &mut world.pond.as_mut().unwrap().basins[0];
    let patch = &mut local.stone_biofilm.as_mut().unwrap()[0];
    local.biofilm.add(std::mem::take(&mut patch.stock));
    let total = local.total_biofilm_carbon();
    local.colonize_stone_biofilm(0.25);
    let amount = |basin: &Basin| basin.stone_biofilm.as_ref().unwrap()[0].stock.carbon();
    assert!(amount(local) > 0.0);
    assert_close(local.total_biofilm_carbon(), total);
    assert_close(local.biofilm.carbon() + amount(local), total);
    assert_balanced(local);

    let before = local.clone();
    let mut dark = before.clone();
    dark.photosynthesize(0.0, 1.0);
    assert_eq!(dark, before);
    let mut phosphorus_limited = before.clone();
    phosphorus_limited.minerals.0[2] = 0.0;
    let limited_before = phosphorus_limited.clone();
    phosphorus_limited.photosynthesize(1.0, 1.0);
    assert_eq!(phosphorus_limited, limited_before);
    local.photosynthesize(1.0, 1.0);
    assert!(amount(local) > amount(&before));
    assert!(local.minerals.0[1] < before.minerals.0[1] && local.minerals.0[2] < before.minerals.0[2]);
    let fixed = local.balance.inputs.cn_p.carbon() - before.balance.inputs.cn_p.carbon();
    assert_close(local.balance.photosynthesis_oxygen_mg - before.balance.photosynthesis_oxygen_mg, fixed * OXYGEN_MG_PER_CARBON_MG);
    assert_balanced(local);

    let before = local.clone();
    local.respire(1.0);
    assert!(amount(local) < amount(&before));
    assert!(local.balance.producer_respiration_oxygen_mg > before.balance.producer_respiration_oxygen_mg);
    assert_balanced(local);
}

#[test]
fn r5_persistent_stone_film_survives_edits_dry_water_and_checkpoint_replay() {
    let mut world = WorldState::new_r5_composed_dual_pond(2729);
    let legacy_snapshot = world.pond_observation().unwrap();
    let legacy_durable = serde_json::to_string(&world).unwrap();
    assert!(!legacy_durable.contains("stoneBiofilm"));
    assert!(!serde_json::to_string(&legacy_snapshot).unwrap().contains("stoneBiofilm"));
    let mut older = WorldState::new_r5_wood_pond(2729);
    let enable = PondEdit::EnableStoneBiofilm { basin_id: basin_id(0) };
    assert_eq!(older.edit_pond(&enable), Err(CoreError::WrongWorldKind));
    world.edit_pond(&enable).unwrap();
    world.edit_pond(&enable).unwrap();
    let observed = |world: &WorldState| basin(world).observe(0);
    let wet = observed(&world);
    let [patch] = wet.stone_biofilm.as_ref().unwrap().as_slice() else { panic!("expected main-stone stock") };
    assert_eq!(wet.stone_biofilm_enabled, Some(true));
    assert_eq!(patch.stock_mg_c, patch.available_mg_c);
    assert!(patch.stock_mg_c > 0.0);
    assert_close(wet.biofilm_mg_c, legacy_snapshot.basins[0].biofilm_mg_c);
    assert_eq!(world.pond_observation().unwrap().basins[1], legacy_snapshot.basins[1]);
    let visible = wet.biofilm_distribution.as_ref().unwrap();
    assert_close(visible.bed_mg_c + visible.inactive_mg_c + visible.surfaces.iter().map(|surface| surface.mg_c).sum::<f64>(), wet.biofilm_mg_c);
    assert_balanced(basin(&world));

    world.edit_pond(&PondEdit::DrainWater { basin_id: basin_id(0), liters: wet.water_liters - 0.8 }).unwrap();
    let shallow = observed(&world);
    let partial = &shallow.stone_biofilm.as_ref().unwrap()[0];
    assert!(partial.available_mg_c > 0.0 && partial.available_mg_c < patch.available_mg_c);
    assert_eq!(partial.stock_mg_c, patch.stock_mg_c);
    assert_close(shallow.biofilm_distribution.unwrap().inactive_mg_c, partial.stock_mg_c - partial.available_mg_c);
    world.edit_pond(&PondEdit::DrainWater { basin_id: basin_id(0), liters: shallow.water_liters }).unwrap();
    let dry = observed(&world);
    assert_eq!(dry.stone_biofilm.as_ref().unwrap()[0].available_mg_c, 0.0);
    assert_close(dry.biofilm_distribution.unwrap().inactive_mg_c, wet.biofilm_mg_c);
    world.advance_steps(30).unwrap();
    assert_eq!(observed(&world).stone_biofilm, dry.stone_biofilm);
    world.edit_pond(&PondEdit::AddWater { basin_id: basin_id(0), liters: wet.water_liters }).unwrap();
    assert_eq!(observed(&world).stone_biofilm, wet.stone_biofilm);
    let seeded = world.fresh_habitat(123).unwrap();
    assert_eq!(observed(&seeded).stone_biofilm_enabled, Some(true));
    assert_eq!(observed(&seeded).stone_biofilm.as_ref().unwrap()[0].stone_id, patch.stone_id);
    seeded.validate().unwrap();
    let mut moved = world.clone();
    moved.edit_pond(&PondEdit::RemoveWood { basin_id: basin_id(0), id: 1 }).unwrap();
    moved.edit_pond(&PondEdit::MoveStone { basin_id: basin_id(0), id: patch.stone_id, x: 0.30 }).unwrap();
    assert_eq!(observed(&moved).stone_biofilm, wet.stone_biofilm);
    moved.edit_pond(&PondEdit::AddStone { basin_id: basin_id(0), x: 0.68, preset: PondStonePreset::Main }).unwrap();
    assert_eq!(observed(&moved).stone_biofilm.as_ref().unwrap()[1].stock_mg_c, 0.0);
    moved.validate().unwrap();
    let before_remove = world.clone();
    world.edit_pond(&PondEdit::RemoveStone { basin_id: basin_id(0), id: patch.stone_id }).unwrap();
    let removed = observed(&world);
    assert_eq!(removed.stone_biofilm, Some(vec![]));
    assert_close(removed.biofilm_mg_c, wet.biofilm_mg_c);
    world.restore_pond_edit_state(before_remove).unwrap();
    assert_eq!(observed(&world).stone_biofilm, wet.stone_biofilm);
    let bytes = serde_json::to_vec(&world).unwrap();
    let mut reread: WorldState = serde_json::from_slice(&bytes).unwrap();
    reread.validate().unwrap();
    assert_eq!(reread, world);
    reread.advance_steps(120).unwrap();
    world.advance_steps(120).unwrap();
    assert_eq!(reread, world);
    assert_balanced(basin(&world));
}

#[test]
fn r5_biofilm_projection_uses_the_existing_visual_shares_and_keeps_one_stock() {
    let mut world = WorldState::new_r5_composed_pond(2729);
    let durable_before = serde_json::to_value(&world).unwrap();
    let observed = world.pond_observation().unwrap();
    let wet = observed.basins[0].biofilm_distribution.as_ref().unwrap();
    assert_eq!(wet.inactive_mg_c, 0.0);
    assert_eq!(wet.surfaces.iter().map(|surface| (surface.kind, surface.id)).collect::<Vec<_>>(),
        [(PondBiofilmSurfaceKind::Stone, 1), (PondBiofilmSurfaceKind::Wood, 1)]);
    assert_close(wet.bed_mg_c, 13.036_683_729_197_701);
    assert_close(wet.surfaces[0].mg_c, 2.172_780_621_532_95);
    assert_close(wet.surfaces[1].mg_c, 8.790_535_649_269_348);
    assert_close(wet.bed_mg_c + wet.surfaces.iter().map(|surface| surface.mg_c).sum::<f64>(), 24.0);
    assert_eq!(serde_json::to_value(&world).unwrap(), durable_before);
    assert!(durable_before["pond"]["basins"][0].get("biofilmDistribution").is_none());
    assert!(serde_json::to_value(&observed).unwrap()["basins"][0].get("biofilmDistribution").is_some());
    assert!(WorldState::new_r5_wood_pond(2729).pond_observation().unwrap().basins[0].biofilm_distribution.is_none());

    world.edit_pond(&PondEdit::DrainWater { basin_id: basin_id(0), liters: 17.2 }).unwrap();
    let shallow = basin(&world).biofilm_distribution();
    assert_close(shallow.bed_mg_c, 23.193_577_607_757_69);
    assert!(shallow.surfaces[1].mg_c < wet.surfaces[1].mg_c);
    let remaining_water = basin(&world).water_liters;
    world.edit_pond(&PondEdit::DrainWater { basin_id: basin_id(0), liters: remaining_water }).unwrap();
    assert_eq!(basin(&world).biofilm_distribution(), PondBiofilmDistribution {
        bed_mg_c: 0.0, inactive_mg_c: 24.0, surfaces: Vec::new(),
    });
    world.edit_pond(&PondEdit::AddWater { basin_id: basin_id(0), liters: 18.0 }).unwrap();
    assert_eq!(basin(&world).biofilm_distribution(), *wet);
    world.edit_pond(&PondEdit::RemoveWood { basin_id: basin_id(0), id: 1 }).unwrap();
    let removed = basin(&world).biofilm_distribution();
    assert_eq!(removed.surfaces.len(), 1);
    assert_eq!(removed.surfaces[0].kind, PondBiofilmSurfaceKind::Stone);
    assert_close(removed.bed_mg_c + removed.surfaces[0].mg_c, 24.0);
}

#[test]
fn r5_contact_food_caps_each_typed_source_and_shares_scarcity_proportionally() {
    let mut world = WorldState::new_blank_r5_composed_pond(2729);
    world.edit_pond(&PondEdit::AddWater { basin_id: basin_id(0), liters: 18.0 }).unwrap();
    world.edit_pond(&PondEdit::AddWood { basin_id: basin_id(0), x: 0.27, preset: PondWoodPreset::Root }).unwrap();
    world.edit_pond(&PondEdit::AddStone { basin_id: basin_id(0), x: 0.64, preset: PondStonePreset::Main }).unwrap();
    let local = &mut world.pond.as_mut().unwrap().basins[0];
    let wood = local.wood_layout.as_ref().unwrap().woods[0];
    let column = local.column_projection_px();
    for (offset, adult) in [(-0.7, true), (0.7, false)] {
        let position = wood.surface(offset, column).unwrap();
        let id = local.allocate_shrimp_id();
        let (body, reserve) = Shrimp::starter_matter(adult);
        local.shrimp.push(Shrimp::new(id.clone(), position.x, position.y, 0.0, body, reserve,
            if adult { SHRIMP_MATURE_SECONDS } else { 0.0 }));
        local.wood_layout.as_mut().unwrap().attachments.push(PondShrimpAttachment {
            shrimp_id: id, wood_id: wood.id, offset, rest_ticks: 20,
        });
    }
    let (body, reserve) = Shrimp::starter_matter(true);
    let id = local.allocate_shrimp_id();
    local.shrimp.push(Shrimp::new(id, 0.5, 0.25, 0.0, body, reserve, SHRIMP_MATURE_SECONDS));
    let id = local.allocate_shrimp_id();
    let layout = local.stone_layout.as_mut().unwrap();
    let stone = layout.stones[0];
    let attachment = PondShrimpStoneAttachment { shrimp_id: id.clone(), stone_id: stone.id, offset: 0.0, rest_ticks: 20 };
    let site = attachment.point(&layout.stones, column).unwrap();
    layout.stone_attachments.push(attachment);
    local.shrimp.push(Shrimp::new(id, site.x, site.y, 0.0, body, reserve, SHRIMP_MATURE_SECONDS));
    let total_stock = 0.000_01;
    local.biofilm = Matter::with_ratios(total_stock, 0.077, 0.010);
    local.balance.initial = local.inventory();
    local.balance.inputs = Inventory::default();
    let source_budget = local.biofilm_distribution().surfaces;
    let mut control = local.clone();
    let mut traced = PondBasinContactFeeding::default();
    let observed = local.feed(1.0, 1.0, Some(&mut traced));
    assert_eq!(observed, control.feed(1.0, 1.0, None));
    assert_eq!(*local, control);
    assert_balanced(local);
    assert_eq!(local.biofilm.carbon(), 0.0);
    assert_eq!(local.balance.shrimp_food_mg_c, total_stock);

    let [stone_entry] = traced.stone.as_slice() else { panic!("expected the attached stone shrimp") };
    let [stone_budget, wood_budget] = source_budget.as_slice() else { panic!("expected separate stone and wood sources") };
    assert_eq!(stone_budget.kind, PondBiofilmSurfaceKind::Stone);
    assert_eq!(stone_entry.stone_id, stone_budget.id);
    assert_eq!(wood_budget.id, stone_entry.stone_id); // Stone and wood IDs may overlap.
    assert!((stone_entry.stone_biofilm_mg_c - stone_budget.mg_c).abs() < 1e-18);
    assert!(stone_entry.stone_biofilm_mg_c > 0.0 && stone_entry.shared_biofilm_mg_c > 0.0);
    assert!((stone_entry.stone_biofilm_mg_c + stone_entry.shared_biofilm_mg_c - stone_entry.biofilm_mg_c).abs() < 1e-18);
    let [a, b] = traced.wood.as_slice() else { panic!("only attached shrimp should contribute to source estimates") };
    assert_eq!(wood_budget.kind, PondBiofilmSurfaceKind::Wood);
    assert!((a.wood_biofilm_mg_c + b.wood_biofilm_mg_c - wood_budget.mg_c).abs() < 1e-18);
    for entry in [a, b] {
        assert_eq!(entry.wood_id, wood_budget.id);
        assert!(entry.wood_biofilm_mg_c > 0.0 && entry.shared_biofilm_mg_c > 0.0);
        assert!((entry.wood_biofilm_mg_c + entry.shared_biofilm_mg_c - entry.biofilm_mg_c).abs() < 1e-18);
    }
    assert_close(a.wood_biofilm_mg_c / b.wood_biofilm_mg_c, a.biofilm_mg_c / b.biofilm_mg_c);
    let unlocated = total_stock - stone_entry.biofilm_mg_c - traced.wood.iter().map(|entry| entry.biofilm_mg_c).sum::<f64>();
    assert!(unlocated > 0.0);
    let shared = unlocated + stone_entry.shared_biofilm_mg_c + traced.wood.iter().map(|entry| entry.shared_biofilm_mg_c).sum::<f64>();
    let unassigned_budget = total_stock - source_budget.iter().map(|surface| surface.mg_c).sum::<f64>();
    assert!((shared - unassigned_budget).abs() < 1e-18);
}

#[test]
fn r5_wood_supports_autonomous_rest_and_carried_or_released_shrimp() {
    let mut world = WorldState::new_r5_wood_pond(2729);
    world.edit_pond(&PondEdit::AddWood { basin_id: basin_id(0), x: 0.50, preset: PondWoodPreset::Root }).unwrap();
    let local = &mut world.pond.as_mut().unwrap().basins[0];
    let root = local.wood_layout.as_ref().unwrap().woods[0];
    let depth = local.column_projection_px();
    let site = root.surface(0.75, depth).unwrap();
    let animal = &mut local.shrimp[0].view;
    animal.x = site.x;
    animal.y = site.y - 1.0 / depth;
    animal.heading = std::f64::consts::FRAC_PI_2;
    let shrimp_id = animal.id.clone();
    local.random = RandomState::new(42);

    world.advance_steps(1).unwrap();
    let perched = basin(&world).observe(0);
    let attachment = perched.shrimp_attachments.as_ref().unwrap()[0].clone();
    assert_eq!(attachment.shrimp_id, shrimp_id);
    assert_eq!(attachment.wood_id, root.id);
    assert_close(attachment.offset, 0.75);
    assert_eq!(perched.shrimp[0].activity, Activity::Resting);
    let mut natural_release = world.clone();
    natural_release.advance_steps(u32::from(attachment.rest_ticks)).unwrap();
    let released = basin(&natural_release).observe(0);
    assert!(!released.shrimp_attachments.unwrap().iter().any(|attachment| attachment.shrimp_id == shrimp_id));
    assert_eq!(released.shrimp[0].id, shrimp_id);
    natural_release.validate().unwrap();

    let baseline = world.pond_ledger();
    world.edit_pond(&PondEdit::MoveWood { basin_id: basin_id(0), id: root.id, x: 0.58 }).unwrap();
    let moved = basin(&world).observe(0);
    assert_eq!(moved.shrimp_attachments, perched.shrimp_attachments);
    assert_close(moved.shrimp[0].x - perched.shrimp[0].x, 0.08);
    assert_eq!(moved.shrimp[0].y, perched.shrimp[0].y);
    assert_eq!(moved.shrimp[1..], perched.shrimp[1..]);
    assert_eq!(world.pond_ledger(), baseline);
    world.validate().unwrap();

    let preset = world.fresh_habitat(158).unwrap();
    assert_eq!(basin(&preset).observe(0).woods, moved.woods);
    assert!(basin(&preset).observe(0).shrimp_attachments.unwrap().is_empty());
    let mut exposed = world.clone();
    exposed.edit_pond(&PondEdit::DrainWater { basin_id: basin_id(0), liters: 17.9 }).unwrap();
    let shallow = basin(&exposed).observe(0);
    assert_eq!(shallow.woods, moved.woods);
    assert!(shallow.shrimp_attachments.unwrap().is_empty());
    assert_eq!(shallow.shrimp.len(), moved.shrimp.len());
    assert_balanced(basin(&exposed));
    exposed.edit_pond(&PondEdit::DrainWater { basin_id: basin_id(0), liters: shallow.water_liters }).unwrap();
    assert_eq!(basin(&exposed).observe(0).woods, moved.woods);
    exposed.validate().unwrap();

    world.edit_pond(&PondEdit::RemoveWood { basin_id: basin_id(0), id: root.id }).unwrap();
    let removed = basin(&world).observe(0);
    assert!(removed.woods.unwrap().is_empty());
    assert!(removed.shrimp_attachments.unwrap().is_empty());
    assert_eq!(removed.shrimp, moved.shrimp);
    assert_eq!(world.pond_ledger(), baseline);
    world.validate().unwrap();
}

#[test]
fn r5_wood_and_stone_reserve_common_routes_and_wood_ids_survive_history() {
    let mut world = WorldState::new_blank_r5_wood_pond(42);
    let initial = world.clone();
    let add_root = |x| PondEdit::AddWood { basin_id: basin_id(0), x, preset: PondWoodPreset::Root };
    let add_stone = |x| PondEdit::AddStone { basin_id: basin_id(0), x, preset: PondStonePreset::Main };
    world.edit_pond(&add_root(0.30)).unwrap();
    let first = basin(&world).observe(0).woods.unwrap()[0];
    assert_eq!(world.edit_pond(&add_stone(0.47)), Err(CoreError::OverlappingPondStoneSite));
    world.edit_pond(&add_stone(0.68)).unwrap();
    let before_move = world.clone();
    let move_root = PondEdit::MoveWood { basin_id: basin_id(0), x: 0.51, id: first.id };
    assert_eq!(world.edit_pond(&move_root), Err(CoreError::OverlappingPondWoodSite));
    assert_eq!(world, before_move);
    world.restore_pond_edit_state(initial).unwrap();
    world.edit_pond(&add_root(0.27)).unwrap();
    world.edit_pond(&add_root(0.73)).unwrap();
    assert_eq!(world.edit_pond(&add_root(0.50)), Err(CoreError::PondWoodLimit));
    let woods = basin(&world).observe(0).woods.unwrap();
    assert_eq!(woods.len(), 2);
    assert!(woods[0].id > first.id);
    world.validate().unwrap();
}

#[test]
fn pond_geometry_reshapes_without_changing_volume_or_stocks_and_rejects_overflow() {
    let mut pond = WorldState::new_pond(42);
    let id = POND_DEFAULT_BASIN_ID.to_owned();
    let before = pond.clone();
    assert_eq!(pond.edit_pond(&PondEdit::Reshape { basin_id: id.clone(), surface_area_cm2: 800.0 }), Err(CoreError::PondCapacityExceeded));
    assert_eq!(pond.edit_pond(&PondEdit::AddWater { basin_id: id.clone(), liters: 11.0 }), Err(CoreError::PondCapacityExceeded));
    assert_eq!(pond.edit_pond(&PondEdit::DrainWater { basin_id: id.clone(), liters: 18.01 }), Err(CoreError::PondInsufficientWater));
    assert_eq!(pond.edit_pond(&PondEdit::DrainWater { basin_id: id.clone(), liters: 17.975 }), Err(CoreError::PondTooShallow));
    assert_eq!(pond.edit_pond(&PondEdit::AddDetritus { basin_id: "unknown".to_owned(), mg_c: 10.0 }), Err(CoreError::PondBasinMissing));
    assert_eq!(pond, before);

    pond.edit_pond(&PondEdit::Reshape { basin_id: id, surface_area_cm2: 2000.0 }).unwrap();
    let snapshot = pond.pond_observation().unwrap();
    assert_eq!(snapshot.basins[0].capacity_liters, 24.0);
    assert_eq!(snapshot.basins[0].depth_cm, 9.0);
    assert_eq!(basin(&pond).inventory(), basin(&before).inventory());
    pond.validate().unwrap();
}

#[test]
fn evaporation_drainage_and_refill_have_distinct_water_and_matter_flows() {
    let mut pond = Basin::new(POND_DEFAULT_BASIN_ID.to_owned(), 42);
    let initial = pond.observe(0);
    let starting_minerals = pond.minerals;
    let starting_oxygen = pond.oxygen_mg;
    for _ in 0..12000 { pond.evaporate(1.0, 0.50); }
    assert!(pond.water_liters < initial.water_liters);
    assert_eq!(pond.minerals, starting_minerals);
    assert_eq!(pond.oxygen_mg, starting_oxygen);
    let concentrated = pond.observe(0);
    assert!(concentrated.available_nitrogen_mg_per_liter > initial.available_nitrogen_mg_per_liter);

    let volume_before_drain = pond.water_liters;
    let algae_before_drain = pond.algae;
    pond.drain(2.0).unwrap();
    let drained = pond.observe(0);
    let fraction = 2.0 / volume_before_drain;
    assert_close(drained.oxygen_mg_per_liter.unwrap(), concentrated.oxygen_mg_per_liter.unwrap());
    assert_close(drained.available_nitrogen_mg_per_liter.unwrap(), concentrated.available_nitrogen_mg_per_liter.unwrap());
    assert_close(pond.algae.carbon(), algae_before_drain.carbon() * (1.0 - fraction));
    assert_eq!(pond.daphnia.count, initial.daphnia_count);
    assert_eq!(drained.detritus_mg_c, initial.detritus_mg_c);

    let incoming = season::observe(0);
    let minerals_before_fill = pond.minerals;
    let oxygen_before_fill = pond.oxygen_mg;
    let temperature_before_fill = pond.temperature_celsius;
    pond.add_water(1.4, incoming).unwrap();
    assert_close(pond.minerals.0[1] - minerals_before_fill.0[1], 1.4 * 0.12);
    assert_close(pond.minerals.0[2] - minerals_before_fill.0[2], 1.4 * 0.022);
    assert_close(pond.oxygen_mg - oxygen_before_fill, 1.4 * 0.85 * oxygen_saturation(incoming.air_temperature_celsius));
    assert_close(pond.temperature_celsius, (temperature_before_fill * (pond.water_liters - 1.4) + incoming.air_temperature_celsius * 1.4) / pond.water_liters);
    println!("evaporation {:.6} L with unchanged mineral mass, then drain/refill {:?}", initial.water_liters - concentrated.water_liters, pond.ledger());
    assert_balanced(&pond);
}

#[test]
fn drying_retains_nonvolatile_residue_accounts_for_oxygen_and_never_spawns_replacements() {
    let mut pond = Basin::new(POND_DEFAULT_BASIN_ID.to_owned(), 77);
    pond.water_liters = MIN_WATER_LITERS;
    pond.balance.initial = pond.inventory();
    let previous = pond.clone();
    pond.evaporate(1.0, 0.40);
    let dry = pond.observe(0);
    assert!(!dry.wet);
    assert_eq!(dry.water_liters, 0.0);
    assert_eq!(dry.temperature_celsius, None);
    assert_eq!(dry.oxygen_mg_per_liter, None);
    assert_eq!(dry.available_nitrogen_mg_per_liter, None);
    assert_eq!(dry.available_phosphorus_mg_per_liter, None);
    assert!(dry.shrimp.is_empty());
    assert_eq!(dry.daphnia_count, 0);
    assert_eq!(pond.minerals, previous.minerals);
    assert_eq!(pond.plants, previous.plants);
    assert!(pond.detritus.carbon() > previous.detritus.carbon() + previous.algae.carbon());
    assert_eq!(pond.balance.gas_export_oxygen_mg, previous.oxygen_mg);
    assert_balanced(&pond);

    pond.advance(1.0, season::observe(0), None);
    assert_eq!(pond.observe(0), dry);
    pond.add_water(7.0, season::observe(0)).unwrap();
    let wet = pond.observe(0);
    assert_close(wet.available_nitrogen_mg_per_liter.unwrap(), (previous.minerals.0[1] + 0.84) / 7.0);
    assert_eq!(wet.daphnia_count, 0);
    assert!(wet.shrimp.is_empty());
    assert_balanced(&pond);

    let mut drained = Basin::new(POND_DEFAULT_BASIN_ID.to_owned(), 77);
    let old_cnp = drained.inventory().cn_p;
    let transported = drained.algae;
    drained.drain(18.0).unwrap();
    assert_eq!(drained.minerals, Matter::default());
    assert_close(drained.inventory().cn_p.carbon(), old_cnp.carbon() - transported.carbon());
    assert_balanced(&drained);
}

#[test]
fn single_aerobic_budget_is_shared_under_oxygen_scarcity() {
    let mut pond = Basin::new(POND_DEFAULT_BASIN_ID.to_owned(), 7);
    pond.detritus.add(Matter::with_ratios(4000.0, 0.034, 0.005));
    pond.oxygen_mg = 0.000_04;
    pond.balance.initial = pond.inventory();
    let had_oxygen = pond.oxygen_mg;
    let initial_carbon = pond.inventory().cn_p.carbon();
    let supply = pond.respire(1.0);
    let ledger = pond.ledger();
    assert!((0.0..1.0).contains(&supply));
    assert_close(pond.oxygen_mg, 0.0);
    assert!(ledger.producer_respiration_oxygen_mg > 0.0 && ledger.animal_respiration_oxygen_mg > 0.0 && ledger.microbial_respiration_oxygen_mg > 0.0);
    let measured_demand = ledger.producer_respiration_oxygen_mg + ledger.animal_respiration_oxygen_mg + ledger.microbial_respiration_oxygen_mg;
    assert_close(measured_demand, had_oxygen);
    assert_close(ledger.oxygen_output_mg, had_oxygen);
    assert_close(initial_carbon - pond.inventory().cn_p.carbon(), had_oxygen / OXYGEN_MG_PER_CARBON_MG);
    let frozen = pond.clone();
    assert_eq!(pond.respire(1.0), 0.0);
    assert_eq!(pond, frozen, "oxygen-zero day-independent respiration cannot deduct again");
    println!("scarce oxygen {had_oxygen:.8} mg, producer/animal/microbe {:.8}/{:.8}/{:.8} mg", ledger.producer_respiration_oxygen_mg, ledger.animal_respiration_oxygen_mg, ledger.microbial_respiration_oxygen_mg);
    assert_balanced(&pond);
}

#[test]
fn light_organic_load_temperature_and_supersaturation_have_the_expected_directions() {
    assert!(oxygen_saturation(5.0) > oxygen_saturation(20.0));
    assert!(oxygen_saturation(20.0) > oxygen_saturation(30.0));
    assert!((oxygen_saturation(20.0) - 9.09).abs() < 0.03);

    let mut gas = Basin::new(POND_DEFAULT_BASIN_ID.to_owned(), 42);
    gas.oxygen_mg = oxygen_saturation(gas.temperature_celsius) * gas.water_liters * 1.20;
    gas.balance.initial = gas.inventory();
    gas.exchange_oxygen();
    assert!(gas.balance.gas_export_oxygen_mg > 0.0);
    assert!(gas.oxygen_mg > oxygen_saturation(gas.temperature_celsius) * gas.water_liters);
    assert_balanced(&gas);

    let mut daytime = WorldState::new_pond(42);
    let mut nighttime = daytime.clone();
    nighttime.daylight.phase = 0.0;
    let mut loaded = nighttime.clone();
    loaded.edit_pond(&PondEdit::AddDetritus { basin_id: POND_DEFAULT_BASIN_ID.to_owned(), mg_c: 650.0 }).unwrap();
    for world in [&mut daytime, &mut nighttime, &mut loaded] {
        world.advance_steps(18_000).unwrap();
        world.validate().unwrap();
    }
    let sunlight = basin(&daytime);
    let dark = basin(&nighttime);
    let high_organic = basin(&loaded);
    assert!(sunlight.balance.photosynthesis_oxygen_mg > 0.0);
    assert_eq!(dark.balance.photosynthesis_oxygen_mg, 0.0);
    assert!(dark.balance.producer_respiration_oxygen_mg > 0.0);
    assert!(dark.balance.animal_respiration_oxygen_mg > 0.0);
    assert!(dark.balance.microbial_respiration_oxygen_mg > 0.0);
    assert!(sunlight.observe(0).oxygen_mg_per_liter > dark.observe(0).oxygen_mg_per_liter);
    assert!(high_organic.observe(0).oxygen_mg_per_liter < dark.observe(0).oxygen_mg_per_liter);
    assert!(sunlight.temperature_celsius > dark.temperature_celsius);
    assert!(high_organic.balance.microbial_respiration_oxygen_mg > dark.balance.microbial_respiration_oxygen_mg);
    println!("30-min day/dark/loaded DO {:.6}/{:.6}/{:.6} mg/L, day/dark temperatures {:.6}/{:.6} °C", sunlight.observe(0).oxygen_mg_per_liter.unwrap(), dark.observe(0).oxygen_mg_per_liter.unwrap(), high_organic.observe(0).oxygen_mg_per_liter.unwrap(), sunlight.temperature_celsius, dark.temperature_celsius);
    println!("30-min loaded ledger {:?}", high_organic.ledger());
    assert_balanced(sunlight);
    assert_balanced(dark);
    assert_balanced(high_organic);
}

#[test]
fn lower_oxygen_and_parental_investment_change_pond_populations_without_creating_matter() {
    let mut pond = Basin::new(POND_DEFAULT_BASIN_ID.to_owned(), 9);
    pond.shrimp[0].reserve.add(Matter::with_ratios(3.0, 0.11, 0.012));
    pond.shrimp[0].birth_credit = 1.0;
    pond.daphnia.birth_credit = 1.0;
    pond.balance.initial = pond.inventory();
    let juvenile = pond.shrimp[1].body;
    let before_life = pond.inventory();
    pond.step_shrimp(1.0, 1.0, [false; MAX_SHRIMP]);
    pond.step_daphnia(1.0, 1.0);
    assert_eq!(pond.shrimp.len(), 4);
    assert_eq!(pond.shrimp[3].view.id, "pond-main-shrimp-4");
    assert!(pond.shrimp[1].body.carbon() > juvenile.carbon());
    assert_eq!(pond.daphnia.count, 49);
    assert_close(pond.inventory().cn_p.carbon(), before_life.cn_p.carbon());
    pond.shrimp[0].oxygen_stress_seconds = 1199.99;
    pond.daphnia.mortality_credit = 0.999_99;
    let detritus_before_death = pond.detritus.carbon();
    pond.step_shrimp(0.0, 1.0, [false; MAX_SHRIMP]);
    pond.step_daphnia(0.0, 1.0);
    assert_eq!(pond.shrimp.len(), 3);
    assert!(!pond.shrimp.iter().any(|shrimp| shrimp.view.id == "pond-main-shrimp-1"));
    assert_eq!(pond.daphnia.count, 48);
    assert!(pond.detritus.carbon() > detritus_before_death);
    assert_balanced(&pond);
}

#[test]
fn edible_resources_are_disjoint_and_the_same_biofilm_is_allocated_once() {
    let mut pond = Basin::new(POND_DEFAULT_BASIN_ID.to_owned(), 42);
    pond.biofilm = Matter::with_ratios(0.000_01, 0.077, 0.010);
    pond.algae = Matter::default();
    pond.balance.initial = pond.inventory();
    let old_reserve = pond.daphnia.reserve;
    let fed = pond.feed(1.0, 1.0, None);
    assert_close(pond.balance.shrimp_food_mg_c, 0.000_01);
    assert_eq!(pond.daphnia.reserve, old_reserve);
    assert!(fed[..3].iter().all(|fed| *fed));
    assert_eq!(pond.biofilm, Matter::default());

    pond.algae = Matter::with_ratios(0.000_01, 0.088, 0.012);
    pond.balance.inputs.cn_p.add(pond.algae);
    let reserves = pond.shrimp.iter().map(|animal| animal.reserve).collect::<Vec<_>>();
    pond.feed(1.0, 1.0, None);
    assert_eq!(pond.shrimp.iter().map(|animal| animal.reserve).collect::<Vec<_>>(), reserves);
    assert_close(pond.balance.daphnia_food_mg_c, 0.000_01);
    assert_eq!(pond.algae, Matter::default());
    assert_balanced(&pond);
}

#[test]
fn pond_replay_batches_match_and_other_basins_keep_their_own_random_history() {
    let initial = WorldState::new_pond_with_basins(7401, 2).unwrap();
    let mut control = initial.clone();
    let mut exposed = initial.clone();
    let mut batched = initial;
    let intervention = PondEdit::DrainWater { basin_id: POND_DEFAULT_BASIN_ID.to_owned(), liters: 18.0 };
    exposed.edit_pond(&intervention).unwrap();
    batched.edit_pond(&intervention).unwrap();
    control.advance_steps(1200).unwrap();
    exposed.advance_steps(1200).unwrap();
    for _ in 0..24 { batched.advance_steps(50).unwrap(); }
    assert_eq!(exposed, batched);
    let reference = &control.pond.as_ref().unwrap().basins[1];
    let unrelated = &exposed.pond.as_ref().unwrap().basins[1];
    assert_eq!(reference, unrelated);
    assert!(!basin(&exposed).wet());
    assert_balanced(basin(&exposed));
    assert_balanced(unrelated);
}

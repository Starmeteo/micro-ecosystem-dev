use super::*;
use crate::WorldState;

fn bridge(mode: PondShapeMode) -> PondEdit {
    PondEdit::CommitTopology { basin_id: POND_DEFAULT_BASIN_ID.to_owned(), mode, cells: vec![24] }
}

#[test]
fn r5_pond_authored_structures_remain_editable_on_a_promoted_section_after_save() {
    let mut world = WorldState::new_blank_r5_composed_dual_pond(5);
    let main = || POND_DEFAULT_BASIN_ID.to_owned();
    for basin_id in [main(), "pond-2".into()] {
        world.edit_pond(&PondEdit::AddWater { basin_id, liters: 18.0 }).unwrap();
    }
    for (x, preset) in [(0.19, PondWoodPreset::HangingRoots), (0.5, PondWoodPreset::RootNetwork)] {
        world.edit_pond(&PondEdit::AddWood { basin_id: "pond-2".into(), x, preset }).unwrap();
    }
    world.edit_pond(&PondEdit::AddStone { basin_id: "pond-2".into(), x: 0.81, preset: PondStonePreset::FlatStoneCave }).unwrap();
    let source = world.pond.as_ref().unwrap().observe();
    let retained_row = source.basins[1].shape.as_ref().unwrap().protected_row;
    world.edit_pond(&bridge(PondShapeMode::Add)).unwrap();
    let compound = world.pond.as_ref().unwrap().observe();
    let side = &compound.basins[0].static_sections[0];
    assert_eq!(side.row, retained_row);
    assert_eq!(Some(&side.woods), source.basins[1].woods.as_ref());
    assert_eq!(Some(&side.stones), source.basins[1].stones.as_ref());
    let moved = PondEdit::EditStaticSection { basin_id: main(), row: retained_row,
        edit: Box::new(PondEdit::MoveStone { basin_id: main(), id: side.stones[0].id, x: 0.79 }) };
    world.edit_pond(&moved).unwrap();
    let saved = serde_json::to_string(&world).unwrap();
    assert!(saved.contains("hangingRoots") && saved.contains("rootNetwork") && saved.contains("flatStoneCave"));
    let mut restored: WorldState = serde_json::from_str(&saved).unwrap();
    assert_eq!(restored, world);
    restored.validate().unwrap();
    restored.edit_pond(&bridge(PondShapeMode::Remove)).unwrap();
    let split = restored.pond.as_ref().unwrap().observe();
    assert_eq!(split.basins[1].shape.as_ref().unwrap().protected_row, retained_row);
    assert_eq!(split.basins[1].woods, source.basins[1].woods);
    let cave = split.basins[1].stones.as_ref().unwrap()[0];
    assert_eq!(cave.x, 0.79);
    assert_eq!(cave.preset, PondStonePreset::FlatStoneCave);
    restored.edit_pond(&PondEdit::RemoveStone { basin_id: split.basins[1].id.clone(), id: cave.id }).unwrap();
    assert!(restored.pond.as_ref().unwrap().observe().basins[1].stones.as_ref().unwrap().is_empty());
    restored.validate().unwrap();
}

#[test]
fn r5_topology_retains_static_layout_on_the_original_section_only() {
    let mut world = WorldState::new_blank_r5_composed_dual_pond(5);
    let source = &mut world.pond.as_mut().unwrap().basins[0];
    let wood_id = source.wood_layout.as_mut().unwrap().seed_slope(0.235);
    source.scenery_layout = Some(PondSceneryLayout::seeded(wood_id));
    world.edit_pond(&PondEdit::AddStone {
        basin_id: POND_DEFAULT_BASIN_ID.to_owned(), x: 0.60, preset: PondStonePreset::Main,
    }).unwrap();
    for (id, liters, mg_c) in [(POND_DEFAULT_BASIN_ID, 5.0, 20.0), ("pond-2", 7.0, 60.0)] {
        world.edit_pond(&PondEdit::AddWater { basin_id: id.to_owned(), liters }).unwrap();
        world.edit_pond(&PondEdit::AddDetritus { basin_id: id.to_owned(), mg_c }).unwrap();
    }
    let basins = &world.pond.as_ref().unwrap().basins;
    let original = basins[0].clone();
    let oxygen_mg = basins[0].oxygen_mg + basins[1].oxygen_mg;
    let original_row = original.observation_row(0, basins.len());
    let assert_layout = |retained: &Basin| {
        assert_eq!(retained.id, original.id);
        assert_eq!(retained.observation_row, Some(original_row as u8));
        assert_eq!(retained.stone_layout, original.stone_layout);
        assert_eq!(retained.wood_layout, original.wood_layout);
        assert_eq!(retained.scenery_layout, original.scenery_layout);
    };

    world.edit_pond(&bridge(PondShapeMode::Add)).unwrap();
    world.validate().unwrap();
    let merged = world.clone();
    let basins = &world.pond.as_ref().unwrap().basins;
    assert_eq!(basins.len(), 1);
    assert_layout(&basins[0]);
    assert_eq!(basins[0].water_liters, 12.0);
    assert_eq!(basins[0].detritus.carbon(), 80.0);
    assert_eq!(basins[0].oxygen_mg, oxygen_mg);

    world.edit_pond(&bridge(PondShapeMode::Remove)).unwrap();
    world.validate().unwrap();
    let basins = &world.pond.as_ref().unwrap().basins;
    assert_eq!(basins.len(), 2);
    assert_layout(&basins[0]);
    assert_eq!(basins[1].id, "pond-3");
    assert_ne!(basins[1].observation_row, basins[0].observation_row);
    assert_eq!(basins[1].stone_layout, Some(PondStoneLayout::new(true)));
    let empty = basins[1].wood_layout.as_ref().unwrap();
    assert!(empty.woods.is_empty());
    assert_eq!(empty.wood_biofilm, Some(Vec::new()));
    assert_eq!(basins[1].scenery_layout, Some(PondSceneryLayout::empty()));
    for basin in basins {
        assert_eq!(basin.water_liters, 6.0);
        assert_eq!(basin.detritus.carbon(), 40.0);
        assert_eq!(basin.oxygen_mg, oxygen_mg * 0.5);
    }
    let checkpoint = serde_json::to_string(&world).unwrap();
    let loaded: WorldState = serde_json::from_str(&checkpoint).unwrap();
    loaded.validate().unwrap();
    assert_eq!(loaded, world);
    world.restore_pond_edit_state(merged).unwrap();
    assert_layout(&world.pond.as_ref().unwrap().basins[0]);
    world.edit_pond(&bridge(PondShapeMode::Remove)).unwrap();
    assert_eq!(world.pond.as_ref().unwrap().basins[1].id, "pond-4");
}

#[test]
fn r5_topology_preserves_incoming_shrimp_and_rejects_a_cut_through_a_floating_anchor_atomically() {
    let mut absorbed = WorldState::new_blank_r5_composed_dual_pond(5);
    absorbed.edit_pond(&PondEdit::AddWater { basin_id: "pond-2".into(), liters: 12.0 }).unwrap();
    absorbed.edit_pond(&PondEdit::StockShrimp {
        basin_id: "pond-2".into(), position: Point { x: 0.66, y: 0.45 }, count: 1,
    }).unwrap();
    let incoming = absorbed.pond.as_ref().unwrap().basins[1].shrimp[0].view.id.clone();
    absorbed.edit_pond(&bridge(PondShapeMode::Add)).unwrap();
    absorbed.validate().unwrap();
    assert_eq!(absorbed.pond.as_ref().unwrap().basins[0].static_sections[0].shrimp[0].view.id, incoming);
    absorbed.edit_pond(&bridge(PondShapeMode::Remove)).unwrap();
    let promoted = &absorbed.pond.as_ref().unwrap().basins[1];
    assert_eq!(promoted.id, "pond-3");
    assert_eq!(promoted.shrimp[0].view.id, incoming);

    let mut split = WorldState::new_blank_r5_composed_dual_pond(5);
    split.edit_pond(&bridge(PondShapeMode::Add)).unwrap();
    split.edit_pond(&PondEdit::AddWater { basin_id: POND_DEFAULT_BASIN_ID.to_owned(), liters: 10.0 }).unwrap();
    split.edit_pond(&PondEdit::AddFloatingPlant {
        basin_id: POND_DEFAULT_BASIN_ID.to_owned(), position: Point { x: 0.5 / 12.0, y: 2.5 / 12.0 },
        preset: PondFloatingPlantPreset::Medium,
    }).unwrap();
    let untouched = split.clone();
    assert_eq!(split.edit_pond(&bridge(PondShapeMode::Remove)), Err(CoreError::PondShape(Error::TopologyLostAnchor)));
    assert_eq!(split, untouched);
}

#[test]
fn r5_topology_retains_empty_row_once_its_shrimp_ids_have_been_used() {
    let mut world = WorldState::new_blank_r5_composed_dual_pond(19);
    let main = || POND_DEFAULT_BASIN_ID.to_owned();
    for basin_id in [main(), "pond-2".into()] {
        world.edit_pond(&PondEdit::AddWater { basin_id, liters: 5.0 }).unwrap();
    }
    world.edit_pond(&PondEdit::StockShrimp {
        basin_id: "pond-2".into(), position: Point { x: 0.66, y: 0.45 }, count: 1,
    }).unwrap();
    world.edit_pond(&PondEdit::DrainWater { basin_id: "pond-2".into(), liters: 5.0 }).unwrap();
    let before = world.pond.as_ref().unwrap().observe();
    let row = before.basins[1].shape.as_ref().unwrap().protected_row;
    assert!(before.basins[1].shrimp.is_empty());
    assert_eq!(before.basins[1].retain_observation_row, Some(true));
    world.edit_pond(&bridge(PondShapeMode::Add)).unwrap();
    world.edit_pond(&PondEdit::EditStaticSection {
        basin_id: main(), row, edit: Box::new(PondEdit::StockShrimp {
            basin_id: main(), position: Point { x: 0.66, y: 0.45 }, count: 1,
        }),
    }).unwrap();
    world.edit_pond(&bridge(PondShapeMode::Remove)).unwrap();
    let promoted = &world.pond.as_ref().unwrap().basins[1];
    assert_eq!(promoted.shrimp[0].view.id, "pond-2-shrimp-2");
    world.validate().unwrap();
}

#[test]
fn r5_topology_preserves_local_moss_film_and_shrimp_across_edit_and_promotion() {
    let mut world = WorldState::new_blank_r5_composed_dual_pond(29);
    let main = || POND_DEFAULT_BASIN_ID.to_owned();
    for basin_id in [main(), "pond-2".into()] {
        world.edit_pond(&PondEdit::AddWater { basin_id: basin_id.clone(), liters: 10.0 }).unwrap();
        world.edit_pond(&PondEdit::AddStone { basin_id: basin_id.clone(), x: 0.65, preset: PondStonePreset::Main }).unwrap();
        world.edit_pond(&PondEdit::AddMoss {
            basin_id: basin_id.clone(), parent: PondMossParent::Stone { id: 1 }, offset: 0.0, preset: PondMossPreset::Mat,
        }).unwrap();
        world.edit_pond(&PondEdit::StockShrimp {
            basin_id, position: Point { x: 0.86, y: 0.43 }, count: 1,
        }).unwrap();
    }
    let pond = world.pond.as_mut().unwrap();
    for basin in &mut pond.basins { basin.biofilm = Matter::with_ratios(20.0, 0.077, 0.010); }
    pond.reset_initial_balance();
    for basin_id in [main(), "pond-2".into()] {
        world.edit_pond(&PondEdit::EnableStoneBiofilm { basin_id }).unwrap();
    }
    let [first, incoming]: [Basin; 2] = world.pond.as_ref().unwrap().basins.clone().try_into().unwrap();
    let incoming_row = incoming.observation_row(1, 2) as u8;
    assert!(incoming.stone_biofilm.as_ref().unwrap()[0].stock.carbon() > 0.0);
    world.edit_pond(&bridge(PondShapeMode::Add)).unwrap();
    let combined = &world.pond.as_ref().unwrap().basins[0];
    let secondary = &combined.static_sections[0];
    assert_eq!(secondary.row, incoming_row);
    assert_eq!(secondary.moss_layout, incoming.moss_layout);
    assert_eq!(secondary.stone_biofilm, incoming.stone_biofilm);
    assert_eq!(secondary.shrimp[0].view.id, incoming.shrimp[0].view.id);
    assert_eq!(secondary.shrimp[0].age_seconds, incoming.shrimp[0].age_seconds);
    assert_eq!(combined.shrimp[0].view.id, first.shrimp[0].view.id);
    assert_eq!(combined.stone_biofilm, first.stone_biofilm);
    let mut expected = first.inventory();
    expected.add_scaled(incoming.inventory(), 1.0);
    for element in 0..3 { assert!((combined.inventory().cn_p.0[element] - expected.cn_p.0[element]).abs() < 1e-8); }

    let local = |edit| PondEdit::EditStaticSection { basin_id: main(), row: incoming_row, edit: Box::new(edit) };
    world.edit_pond(&local(PondEdit::TrimMoss { basin_id: main(), id: 1, mg_c: 1.0 })).unwrap();
    let before_stock = world.clone();
    let stock = local(PondEdit::StockShrimp { basin_id: main(), position: Point { x: 0.86, y: 0.43 }, count: 1 });
    world.edit_pond(&stock).unwrap();
    let issued = world.pond.as_ref().unwrap().basins[0].static_sections[0].shrimp[1].view.id.clone();
    world.restore_pond_edit_state(before_stock).unwrap();
    world.edit_pond(&stock).unwrap();
    let combined = &world.pond.as_ref().unwrap().basins[0];
    let secondary = &combined.static_sections[0];
    assert_ne!(secondary.shrimp[1].view.id, issued);
    assert_eq!(combined.moss_layout, first.moss_layout);
    assert_eq!(combined.detritus.carbon(), 1.0);
    let retained_moss = secondary.moss_layout.clone();
    let retained_film = secondary.stone_biofilm.clone();
    let retained_ids = secondary.shrimp.iter().map(|shrimp| shrimp.view.id.clone()).collect::<Vec<_>>();
    let checkpoint = serde_json::to_string(&world).unwrap();
    let mut restored: WorldState = serde_json::from_str(&checkpoint).unwrap();
    restored.validate().unwrap();
    assert_eq!(world, restored);
    restored.edit_pond(&bridge(PondShapeMode::Remove)).unwrap();
    restored.validate().unwrap();
    let promoted = &restored.pond.as_ref().unwrap().basins[1];
    assert_eq!(promoted.id, "pond-3");
    assert_eq!(promoted.shrimp_namespace(), "pond-2");
    assert_eq!(promoted.shrimp.iter().map(|shrimp| shrimp.view.id.clone()).collect::<Vec<_>>(), retained_ids);
    assert_eq!(promoted.moss_layout, retained_moss);
    assert_eq!(promoted.stone_biofilm, retained_film);
    let preset = world.fresh_habitat(37).unwrap();
    preset.validate().unwrap();
    let reset = &preset.pond.as_ref().unwrap().basins[0].static_sections[0];
    assert!(reset.shrimp.is_empty());
    assert!(reset.shrimp_namespace.is_none());
    assert!(reset.stone_biofilm.is_some());
    assert_eq!(reset.moss_layout, incoming.moss_layout);
}

#[test]
fn r5_static_section_survives_merge_edit_reload_and_split_without_extra_stocks() {
    let mut world = WorldState::new_blank_r5_composed_dual_pond(41);
    let main = || POND_DEFAULT_BASIN_ID.to_owned();
    let other = || "pond-2".to_owned();
    {
        let source = &mut world.pond.as_mut().unwrap().basins[1];
        let wood_id = source.wood_layout.as_mut().unwrap().seed_slope(0.235);
        source.scenery_layout = Some(PondSceneryLayout::seeded(wood_id));
    }
    for basin_id in [main(), other()] {
        world.edit_pond(&PondEdit::AddStone { basin_id, x: 0.68, preset: PondStonePreset::Main }).unwrap();
    }
    world.edit_pond(&PondEdit::EnableStoneBiofilm { basin_id: other() }).unwrap();
    for basin_id in [main(), other()] {
        world.edit_pond(&PondEdit::AddWater { basin_id, liters: 7.0 }).unwrap();
    }
    world.edit_pond(&PondEdit::StockShrimp {
        basin_id: main(), position: Point { x: 0.5, y: 0.45 }, count: 2,
    }).unwrap();

    let original = world.pond.as_ref().unwrap().basins.clone();
    let row = original[1].observation_row(1, original.len()) as u8;
    let local = |edit| PondEdit::EditStaticSection { basin_id: main(), row, edit: Box::new(edit) };
    let section = |world: &WorldState| world.pond.as_ref().unwrap().observe().basins[0].static_sections[0].clone();
    let primary = |world: &WorldState| world.pond.as_ref().unwrap().basins[0].observe(0);
    let before = serde_json::to_value(&world).unwrap();
    assert!(!before["pond"]["basins"][0].as_object().unwrap().contains_key("staticSections"));
    world.edit_pond(&bridge(PondShapeMode::Add)).unwrap();
    let merged = world.pond.as_ref().unwrap().basins[0].inventory();
    assert_eq!(world.pond.as_ref().unwrap().basins.len(), 1);
    assert_eq!(merged.water_liters, 14.0);
    assert_eq!(primary(&world).shrimp.len(), 2);
    assert_eq!(section(&world).row, row);
    assert_eq!(section(&world).stones, original[1].observe(1).stones.unwrap());
    assert_eq!(section(&world).woods, original[1].observe(1).woods.unwrap());
    let scenery = original[1].observe(1).scenery.unwrap();
    assert!(scenery.iter().any(|item| matches!(item.parent, Some(PondSceneryParent::Wood { .. }))));
    assert_eq!(section(&world).scenery, scenery);

    let preserved = world.clone();
    assert_eq!(world.edit_pond(&local(PondEdit::MoveStone { basin_id: other(), id: 1, x: 0.73 })),
        Err(CoreError::InvalidPondStaticSectionEdit));
    assert_eq!(world.edit_pond(&local(PondEdit::DrainWater { basin_id: main(), liters: 1.0 })),
        Err(CoreError::InvalidPondStaticSectionEdit));
    assert_eq!(world.edit_pond(&local(PondEdit::MoveStone { basin_id: main(), id: 1, x: 0.01 })),
        Err(CoreError::InvalidPondStoneSite));
    let cell = u16::from(row) * pond_shape::COLUMNS as u16;
    let topology = Err(CoreError::PondShape(Error::TopologySection));
    assert_eq!(world.edit_pond(&PondEdit::PaintShape { basin_id: main(), mode: PondShapeMode::Remove, cells: vec![cell] }), topology);
    assert_eq!(world.edit_pond(&PondEdit::CommitTopology { basin_id: main(), mode: PondShapeMode::Remove, cells: vec![cell, 24] }), topology);
    assert_eq!(world, preserved);

    world.edit_pond(&local(PondEdit::MoveStone { basin_id: main(), id: 1, x: 0.73 })).unwrap();
    let moved_wood = section(&world).woods[0];
    world.edit_pond(&local(PondEdit::MoveWood { basin_id: main(), id: moved_wood.id, x: 0.27 })).unwrap();
    assert_eq!(section(&world).scenery, scenery);
    assert_eq!(section(&world).stones[0].x, 0.73);
    assert_eq!(primary(&world).stones.unwrap()[0].x, 0.68);
    let new_wood = section(&world).woods[0];
    assert_ne!(moved_wood, new_wood);

    let carved = world.clone();
    world.edit_pond(&local(PondEdit::RemoveWood { basin_id: main(), id: new_wood.id })).unwrap();
    assert!(!section(&world).scenery.iter().any(|item| matches!(item.parent, Some(PondSceneryParent::Wood { .. }))));
    world.restore_pond_edit_state(carved).unwrap();

    let expected = section(&world);
    let habitat = world.fresh_habitat(42).unwrap();
    assert_eq!(section(&habitat), expected);
    let encoded = serde_json::to_string(&world).unwrap();
    assert!(serde_json::to_value(&world).unwrap()["pond"]["basins"][0]["staticSections"][0].get("leafDepositLayout").is_none());
    let mut loaded: WorldState = serde_json::from_str(&encoded).unwrap();
    loaded.validate().unwrap();
    assert_eq!(loaded, world);
    let saved = loaded.pond.as_ref().unwrap().basins[0].inventory();
    assert_eq!(saved.water_liters, merged.water_liters);
    assert_eq!(saved.oxygen_mg, merged.oxygen_mg);
    assert_eq!(saved.cn_p.0, merged.cn_p.0);

    loaded.edit_pond(&bridge(PondShapeMode::Remove)).unwrap();
    let pond = loaded.pond.as_ref().unwrap().observe();
    let [primary, split] = pond.basins.as_slice() else { panic!("two split basins") };
    assert_eq!(primary.shrimp.len(), 2);
    assert!(primary.static_sections.is_empty());
    assert_eq!(split.id, "pond-3");
    assert_eq!(split.shape.as_ref().unwrap().protected_row, row);
    assert_eq!(split.stones.as_deref(), Some(expected.stones.as_slice()));
    assert_eq!(split.woods.as_deref(), Some(expected.woods.as_slice()));
    assert_eq!(split.scenery.as_deref(), Some(expected.scenery.as_slice()));
    assert_eq!(split.stone_biofilm_enabled, Some(true));
    assert_eq!(split.stone_biofilm.as_ref().unwrap()[0].stock_mg_c, 0.0);
    assert_eq!(primary.water_liters + split.water_liters, merged.water_liters);
    loaded.validate().unwrap();
}

#[test]
fn r5_extracting_a_decayed_leaf_only_removes_its_remainder_in_the_selected_section() {
    let mut world = WorldState::new_blank_r5_composed_dual_pond(24);
    let main = || POND_DEFAULT_BASIN_ID.to_owned();
    for (basin_id, x, mg_c) in [(main(), 0.30, 24.0), ("pond-2".into(), 0.60, 40.0)] {
        world.edit_pond(&PondEdit::AddWater { basin_id: basin_id.clone(), liters: 8.0 }).unwrap();
        world.edit_pond(&PondEdit::AddLeafDeposit { basin_id, x, preset: PondLeafDepositPreset::Midground, mg_c }).unwrap();
    }
    world.edit_pond(&bridge(PondShapeMode::Add)).unwrap();
    world.advance_steps(32).unwrap();
    let prior = &world.pond.as_ref().unwrap().basins[0];
    let snapshot = prior.observe(0);
    let side = &snapshot.static_sections[0];
    let selected = side.leaf_deposits.as_ref().unwrap()[0];
    assert!(selected.remaining_mg_c > 0.0 && selected.remaining_mg_c < selected.initial_mg_c);
    let prior_pool = prior.detritus;
    let before = prior.ledger();
    let expected = prior_pool.scaled(selected.remaining_mg_c / prior_pool.carbon());
    let take = |id| PondEdit::EditStaticSection {
        basin_id: main(), row: side.row, edit: Box::new(PondEdit::RemoveLeafDeposit { basin_id: main(), id }),
    };
    let unchanged = world.clone();
    assert_eq!(world.edit_pond(&take(99)), Err(CoreError::PondLeafDepositMissing));
    assert_eq!(world, unchanged);
    world.edit_pond(&take(selected.id)).unwrap();
    let basin = &world.pond.as_ref().unwrap().basins[0];
    let after = basin.ledger();
    for element in 0..3 {
        assert!((prior_pool.0[element] - basin.detritus.0[element] - expected.0[element]).abs() < 1e-9);
        assert!((after.matter_output_mg_cnp[element] - before.matter_output_mg_cnp[element] - expected.0[element]).abs() < 1e-9);
    }
    assert_eq!(after.decomposed_mg_c, before.decomposed_mg_c);
    let observed = basin.observe(0);
    assert!(observed.static_sections[0].leaf_deposits.is_none());
    assert_eq!(observed.leaf_deposits, snapshot.leaf_deposits);
    world.validate().unwrap();
    let loaded: WorldState = serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
    assert_eq!(loaded, world);
}

#[test]
fn r5_deposits_keep_both_rows_and_share_one_detritus_pool_and_decomposition() {
    let mut world = WorldState::new_blank_r5_composed_dual_pond(24);
    let main = || POND_DEFAULT_BASIN_ID.to_owned();
    let second = || "pond-2".to_owned();
    let near = |actual: f64, expected: f64| assert!((actual - expected).abs() < 1e-8, "{actual} != {expected}");
    for basin_id in [main(), second()] {
        world.edit_pond(&PondEdit::AddWater { basin_id, liters: 12.0 }).unwrap();
    }
    let mut pool_only = world.clone();
    for (basin_id, x, mg_c) in [(main(), 0.3, 24.0), (second(), 0.6, 40.0)] {
        world.edit_pond(&PondEdit::AddLeafDeposit {
            basin_id: basin_id.clone(), x, preset: PondLeafDepositPreset::Midground, mg_c,
        }).unwrap();
        pool_only.edit_pond(&PondEdit::AddDetritus { basin_id, mg_c }).unwrap();
    }
    for candidate in [&mut world, &mut pool_only] {
        candidate.edit_pond(&PondEdit::AddDetritus { basin_id: second(), mg_c: 20.0 }).unwrap();
        candidate.edit_pond(&bridge(PondShapeMode::Add)).unwrap();
    }
    let primary = |world: &WorldState| world.pond.as_ref().unwrap().basins[0].observe(0);
    let merged = primary(&world);
    let row = merged.static_sections[0].row;
    assert!(merged.static_sections[0].stones.is_empty());
    assert!(merged.static_sections[0].woods.is_empty());
    assert!(merged.static_sections[0].scenery.is_empty());
    assert_eq!(merged.leaf_deposits.as_ref().unwrap()[0].id, 1);
    assert_eq!(merged.static_sections[0].leaf_deposits.as_ref().unwrap()[0].id, 1);
    assert_eq!(merged.static_sections[0].leaf_deposits.as_ref().unwrap()[0].remaining_mg_c, 40.0);
    let wrapped = |edit| PondEdit::EditStaticSection { basin_id: main(), row, edit: Box::new(edit) };

    let ledger = |world: &WorldState| world.pond.as_ref().unwrap().basins[0].ledger();
    let untouched = world.clone();
    assert_eq!(world.edit_pond(&wrapped(PondEdit::AddLeafDeposit {
        basin_id: main(), x: 0.07, preset: PondLeafDepositPreset::Foreground, mg_c: 12.0,
    })), Err(CoreError::InvalidPondLeafDepositSite));
    assert_eq!(world, untouched);
    world.edit_pond(&wrapped(PondEdit::MoveLeafDeposit { basin_id: main(), id: 1, x: 0.72 })).unwrap();
    assert_eq!(ledger(&world), ledger(&untouched));
    assert_eq!(primary(&world).leaf_deposits, merged.leaf_deposits);
    assert_eq!(primary(&world).static_sections[0].leaf_deposits.as_ref().unwrap()[0].x, 0.72);
    world.edit_pond(&wrapped(PondEdit::AddLeafDeposit {
        basin_id: main(), x: 0.48, preset: PondLeafDepositPreset::Foreground, mg_c: 12.0,
    })).unwrap();
    pool_only.edit_pond(&PondEdit::AddDetritus { basin_id: main(), mg_c: 12.0 }).unwrap();
    assert_eq!(ledger(&world), ledger(&pool_only));

    let mut over_attributed = world.clone();
    let invalid_basin = &mut over_attributed.pond.as_mut().unwrap().basins[0];
    invalid_basin.detritus = pond_leaf::detritus_matter(60.0);
    // Each row fits 60 mg C by itself, but the sum of their markers does not.
    assert!(!invalid_basin.static_sections_valid());

    world.advance_steps(32).unwrap();
    pool_only.advance_steps(32).unwrap();
    assert_eq!(ledger(&world), ledger(&pool_only));
    assert!(ledger(&world).decomposed_mg_c > 0.0);
    let after = primary(&world);
    let main_leaves = after.leaf_deposits.unwrap();
    let other_leaves = after.static_sections[0].leaf_deposits.as_ref().unwrap();
    let retained = main_leaves[0].remaining_mg_c / 24.0;
    assert!(retained < 1.0);
    near(other_leaves[0].remaining_mg_c / 40.0, retained);
    near(other_leaves[1].remaining_mg_c / 12.0, retained);
    let preset = primary(&world.fresh_habitat(27).unwrap());
    for leaf in preset.leaf_deposits.iter().flatten().chain(preset.static_sections[0].leaf_deposits.iter().flatten()) {
        assert_eq!(leaf.remaining_mg_c, leaf.initial_mg_c);
    }

    let saved = serde_json::to_string(&world).unwrap();
    let mut reopened: WorldState = serde_json::from_str(&saved).unwrap();
    reopened.validate().unwrap();
    assert_eq!(reopened, world);
    reopened.edit_pond(&bridge(PondShapeMode::Remove)).unwrap();
    let parts = &reopened.pond.as_ref().unwrap().basins;
    assert_eq!(parts[0].observe(0).leaf_deposits.as_deref(), Some(main_leaves.as_slice()));
    assert_eq!(parts[1].observe(1).leaf_deposits.as_deref(), Some(other_leaves.as_slice()));
    assert_eq!(parts[1].observation_row, Some(row));
    for (part, located) in parts.iter().zip([24.0, 52.0]) {
        near(part.detritus.carbon(), (located + 10.0) * retained);
        near(part.ledger().water_residual_liters, 0.0);
        near(part.ledger().oxygen_residual_mg, 0.0);
        for e in 0..3 {
            near(part.ledger().matter_residual_mg_cnp[e], 0.0);
            assert!(part.balance.topology_matter_transfer.0[e].abs() > 1e-8);
        }
    }
    let total = ledger(&world);
    for e in 0..3 {
        near(parts.iter().map(|part| part.inventory().cn_p.0[e]).sum(), total.current_matter_mg_cnp[e]);
        near(parts.iter().map(|part| part.balance.topology_matter_transfer.0[e]).sum(), 0.0);
    }
    reopened.validate().unwrap();
}

#[test]
fn r5_section_routing_keeps_secondary_layers_and_never_reuses_allocated_ids() {
    let mut world = WorldState::new_blank_r5_composed_pond_with_basins(24, 3);
    let topology = |basin_id: &str, mode, cell| PondEdit::CommitTopology {
        basin_id: basin_id.into(), mode, cells: vec![cell],
    };
    for (basin_id, x) in [(POND_DEFAULT_BASIN_ID, 0.53), ("pond-2", 0.60), ("pond-3", 0.67)] {
        world.edit_pond(&PondEdit::AddStone {
            basin_id: basin_id.into(), x, preset: PondStonePreset::Main,
        }).unwrap();
    }
    let add_leaf = |basin_id: &str, x| PondEdit::AddLeafDeposit {
        basin_id: basin_id.into(), x, preset: PondLeafDepositPreset::Midground, mg_c: 12.0,
    };
    world.edit_pond(&add_leaf("pond-2", 0.48)).unwrap();
    world.edit_pond(&add_leaf("pond-3", 0.63)).unwrap();
    world.edit_pond(&topology("pond-2", PondShapeMode::Add, 60)).unwrap();
    let earlier = world.clone();
    world.edit_pond(&bridge(PondShapeMode::Add)).unwrap();
    let add_pebble = |basin_id: &str| PondEdit::AddStone {
        basin_id: basin_id.into(), x: 0.25, preset: PondStonePreset::Pebble,
    };
    world.edit_pond(&PondEdit::EditStaticSection {
        basin_id: POND_DEFAULT_BASIN_ID.into(), row: 4,
        edit: Box::new(add_pebble(POND_DEFAULT_BASIN_ID)),
    }).unwrap();
    world.edit_pond(&PondEdit::EditStaticSection {
        basin_id: POND_DEFAULT_BASIN_ID.into(), row: 4,
        edit: Box::new(add_leaf(POND_DEFAULT_BASIN_ID, 0.56)),
    }).unwrap();
    let snapshot = |world: &WorldState| world.pond.as_ref().unwrap().observe().basins;
    assert_eq!(snapshot(&world)[0].static_sections.iter().map(|section| section.row).collect::<Vec<_>>(), [4, 7]);
    assert_eq!(snapshot(&world)[0].static_sections[0].stones[1].id, 2);

    let mut checkpoint = earlier.clone();
    checkpoint.retain_pond_topology_allocator(&world);
    for mut restored in [checkpoint, {
        let mut restored = world.clone();
        restored.restore_pond_edit_state(earlier).unwrap();
        restored
    }] {
        restored.edit_pond(&add_pebble("pond-2")).unwrap();
        restored.edit_pond(&add_leaf("pond-2", 0.65)).unwrap();
        assert_eq!(snapshot(&restored)[1].stones.as_ref().unwrap()[1].id, 3);
        assert_eq!(snapshot(&restored)[1].leaf_deposits.as_ref().unwrap()[1].id, 3);
        restored.validate().unwrap();
    }

    world.edit_pond(&bridge(PondShapeMode::Remove)).unwrap();
    let basins = snapshot(&world);
    assert_eq!(basins[1].id, "pond-4");
    assert_eq!(basins[1].shape.as_ref().unwrap().protected_row, 4);
    assert_eq!(basins[1].stones.as_ref().unwrap()[1].id, 2);
    assert_eq!(basins[1].static_sections[0].row, 7);
    assert_eq!(basins[1].static_sections[0].stones[0].x, 0.67);
    assert_eq!(basins[1].leaf_deposits.as_ref().unwrap()[1].x, 0.56);
    assert_eq!(basins[1].static_sections[0].leaf_deposits.as_ref().unwrap()[0].x, 0.63);
    world.edit_pond(&topology("pond-4", PondShapeMode::Remove, 60)).unwrap();
    let basins = snapshot(&world);
    assert_eq!(basins[2].id, "pond-5");
    assert_eq!(basins[2].shape.as_ref().unwrap().protected_row, 7);
    assert_eq!(basins[2].stones.as_ref().unwrap()[0].x, 0.67);
    assert_eq!(basins[2].leaf_deposits.as_ref().unwrap()[0].x, 0.63);
    world.validate().unwrap();
}

#[test]
fn r5_full_leaf_layouts_merge_without_increasing_the_per_section_limit() {
    let mut world = WorldState::new_blank_r5_composed_dual_pond(3);
    let add = |basin_id: &str| PondEdit::AddLeafDeposit {
        basin_id: basin_id.into(), x: 0.30, preset: PondLeafDepositPreset::Midground, mg_c: 10.0,
    };
    for id in [POND_DEFAULT_BASIN_ID, "pond-2"] {
        for _ in 0..16 { world.edit_pond(&add(id)).unwrap(); }
    }
    world.edit_pond(&bridge(PondShapeMode::Add)).unwrap();
    let basin = &world.pond.as_ref().unwrap().basins[0];
    let row = basin.static_sections[0].row;
    assert_eq!(basin.detritus.carbon(), 320.0);
    assert_eq!(basin.observe(0).leaf_deposits.unwrap().len(), 16);
    assert_eq!(basin.static_sections[0].observe(basin, basin.turbidity()).leaf_deposits.unwrap().len(), 16);
    let merged = world.clone();
    for edit in [add(POND_DEFAULT_BASIN_ID), PondEdit::EditStaticSection {
        basin_id: POND_DEFAULT_BASIN_ID.into(), row, edit: Box::new(add(POND_DEFAULT_BASIN_ID)),
    }] {
        assert_eq!(world.edit_pond(&edit), Err(CoreError::PondLeafDepositLimit));
        assert_eq!(world, merged);
    }
}

#[test]
fn r5_topology_rejects_a_fifth_observation_section_atomically() {
    let mut world = WorldState::new_blank_r5_composed_dual_pond(5);
    for basin_id in [POND_DEFAULT_BASIN_ID, "pond-2"] {
        world.edit_pond(&PondEdit::AddStone {
            basin_id: basin_id.into(), x: 0.50, preset: PondStonePreset::Main,
        }).unwrap();
    }
    {
        let pond = world.pond.as_mut().unwrap();
        pond.freeze_identities();
        let mut template = pond.basins[1].clone();
        let main = &mut pond.basins[0];
        let cells_per_row = pond_shape::COLUMNS as u16;
        let row_cells = |row: u16| row * cells_per_row..(row + 1) * cells_per_row;
        let main_rows: [u8; 4] = [0, 2, 4, 6];
        let mut cells = main_rows.iter().flat_map(|&row| row_cells(u16::from(row))).collect::<Vec<_>>();
        cells.extend([12, 36, 60]);
        cells.sort_unstable();
        let main_shape = main.shape.as_ref().unwrap().topology_component(cells);
        main.install_footprint(main_shape, usize::from(main_rows[0])).unwrap();
        for &row in &main_rows[1..] {
            template.observation_row = Some(row);
            main.static_sections.push(PondStaticSection::from_basin(&template).unwrap());
        }
        let second = &mut pond.basins[1];
        let new_shape = second.shape.as_ref().unwrap().topology_component(row_cells(8).collect());
        second.install_footprint(new_shape, 8).unwrap();
    }
    world.validate().unwrap();
    let original = world.clone();
    assert_eq!(world.edit_pond(&PondEdit::CommitTopology {
        basin_id: POND_DEFAULT_BASIN_ID.into(), mode: PondShapeMode::Add, cells: vec![84],
    }), Err(CoreError::PondShape(Error::TopologySectionLimit)));
    assert_eq!(world, original);
}

#[test]
fn r5_topology_moves_floating_plants_and_daphnia_while_retaining_the_selected_living_section() {
    let mut world = WorldState::new_blank_r5_composed_dual_pond(24);
    let main = || POND_DEFAULT_BASIN_ID.to_owned();
    let second = || "pond-2".to_owned();
    for (id, daphnia) in [(main(), 5), (second(), 2)] {
        let basin_id = || id.clone();
        world.edit_pond(&PondEdit::AddWater { basin_id: basin_id(), liters: 12.0 }).unwrap();
        world.edit_pond(&PondEdit::StockDaphnia { basin_id: basin_id(), count: daphnia }).unwrap();
    }
    world.edit_pond(&PondEdit::StockShrimp {
        basin_id: main(), position: Point { x: 0.5, y: 0.45 }, count: 2,
    }).unwrap();
    world.edit_pond(&PondEdit::AddStone { basin_id: main(), x: 0.68, preset: PondStonePreset::Main }).unwrap();
    world.edit_pond(&PondEdit::AddMoss {
        basin_id: main(), parent: PondMossParent::Stone { id: 1 }, offset: 0.0, preset: PondMossPreset::Mat,
    }).unwrap();
    world.edit_pond(&PondEdit::PlaceRibbonPlant { basin_id: main(), x: 0.30, preset: PondRibbonPlantPreset::Medium }).unwrap();
    world.edit_pond(&PondEdit::AddLeafDeposit {
        basin_id: main(), x: 0.5, preset: PondLeafDepositPreset::Midground, mg_c: 20.0,
    }).unwrap();
    world.edit_pond(&PondEdit::AddDetritus { basin_id: second(), mg_c: 10.0 }).unwrap();
    let free_plant = Matter::with_ratios(40.0, 0.06, 0.007);
    {
        let pond = world.pond.as_mut().unwrap();
        let film = Matter::with_ratios(12.0, 0.077, 0.01);
        pond.basins[0].biofilm.add(film);
        pond.basins[0].balance.inputs.cn_p.add(film);
        pond.basins[1].plants.add(free_plant);
        pond.basins[1].balance.inputs.cn_p.add(free_plant);
    }
    world.edit_pond(&PondEdit::EnableStoneBiofilm { basin_id: main() }).unwrap();
    let attachment = {
        let basin = &mut world.pond.as_mut().unwrap().basins[0];
        let depth = basin.column_projection_px();
        let stone_layout = basin.stone_layout.as_mut().unwrap();
        let shrimp = &mut basin.shrimp[0];
        let attachment = PondShrimpStoneAttachment {
            shrimp_id: shrimp.view.id.clone(), stone_id: 1, offset: 0.0, rest_ticks: 30,
        };
        let seat = attachment.point(&stone_layout.stones, depth).unwrap();
        shrimp.view.x = seat.x;
        shrimp.view.y = seat.y;
        shrimp.view.activity = Activity::Resting;
        stone_layout.stone_attachments.push(attachment.clone());
        attachment
    };
    for (id, y, preset) in [(main(), 1.5 / 12.0, PondFloatingPlantPreset::Small),
                            (second(), 4.5 / 12.0, PondFloatingPlantPreset::Large)] {
        world.edit_pond(&PondEdit::AddFloatingPlant { basin_id: id, position: Point { x: 0.4, y }, preset }).unwrap();
    }

    let total_inventory = |basins: &[Basin]| {
        let mut total = Inventory::default();
        for basin in basins { total.add_scaled(basin.inventory(), 1.0); }
        total
    };
    let before = world.pond.as_ref().unwrap().basins.clone();
    let expected = total_inventory(&before);
    let expected_root = before[0].observe(0).water_plant_patches;
    let expected_floats = before.iter().map(|basin| basin.observe(0).floating_plants.unwrap()[0]).collect::<Vec<_>>();
    assert_eq!((expected_floats[0].id, expected_floats[1].id), (1, 1));
    let near = |actual: f64, expected: f64| assert!((actual - expected).abs() < 1e-8, "{actual} != {expected}");
    let check_world_total = |basins: &[Basin]| {
        let actual = total_inventory(basins);
        near(actual.water_liters, expected.water_liters);
        near(actual.oxygen_mg, expected.oxygen_mg);
        for element in 0..3 {
            near(actual.cn_p.0[element], expected.cn_p.0[element]);
            let transfers = basins.iter().map(|basin| basin.balance.topology_matter_transfer.0[element]).sum();
            near(transfers, 0.0);
            let external_inputs = |sources: &[Basin]| sources.iter().map(|basin| basin.balance.inputs.cn_p.0[element]).sum();
            near(external_inputs(basins), external_inputs(&before));
        }
    };

    world.edit_pond(&bridge(PondShapeMode::Add)).unwrap();
    world.validate().unwrap();
    let merged = world.pond.as_ref().unwrap().basins[0].clone();
    assert_eq!(merged.daphnia.count, 7);
    assert_eq!(merged.stone_layout.as_ref().unwrap().stone_attachments, [attachment.clone()]);
    assert_ne!(merged.shrimp[0].view.y, before[0].shrimp[0].view.y);
    near(merged.plants.carbon(), 50.0 + free_plant.carbon());
    let merged_view = merged.observe(0);
    let merged_floats = merged_view.floating_plants.unwrap();
    assert_eq!(merged_floats[0].id, expected_floats[0].id);
    assert_ne!(merged_floats[1].id, expected_floats[1].id);
    assert_eq!(merged_floats[1].stock_mg_cnp, expected_floats[1].stock_mg_cnp);
    check_world_total(&world.pond.as_ref().unwrap().basins);

    world.edit_pond(&bridge(PondShapeMode::Remove)).unwrap();
    world.validate().unwrap();
    let basins = &world.pond.as_ref().unwrap().basins;
    let main_view = basins[0].observe(0);
    let child_view = basins[1].observe(1);
    assert_eq!(basins[1].id, "pond-3");
    assert_eq!((basins[0].daphnia.count, basins[1].daphnia.count), (4, 3));
    near(basins[0].daphnia.body.carbon(), merged.daphnia.body.carbon() * 4.0 / 7.0);
    assert_eq!(basins[0].shrimp, before[0].shrimp);
    assert_eq!(basins[0].stone_layout.as_ref().unwrap().stone_attachments, [attachment]);
    assert!(basins[1].shrimp.is_empty());
    assert_eq!(basins[0].moss_layout, before[0].moss_layout);
    assert!(basins[1].moss_layout.as_ref().unwrap().patches.is_empty());
    near(main_view.water_plant_patches.unwrap()[0].stock_mg_cnp[0], expected_root.unwrap()[0].stock_mg_cnp[0]);
    near(basins[0].plants.carbon(), 70.0);
    near(basins[1].plants.carbon(), 20.0);
    near(basins[0].detritus.carbon(), 25.0);
    near(basins[1].detritus.carbon(), 5.0);
    assert_eq!(basins[0].stone_biofilm, before[0].stone_biofilm);
    assert_eq!(basins[1].stone_biofilm, Some(Vec::new()));
    assert_eq!(main_view.floating_plants.unwrap(), [merged_floats[0]]);
    assert_eq!(child_view.floating_plants.unwrap(), [merged_floats[1]]);
    assert!(basins[0].balance.topology_matter_transfer.carbon() > 0.0);
    assert!(basins[1].balance.topology_matter_transfer.carbon() < 0.0);
    check_world_total(basins);
    let mut reopened: WorldState = serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
    reopened.validate().unwrap();
    assert_eq!(reopened, world);

    reopened.edit_pond(&bridge(PondShapeMode::Add)).unwrap();
    world.edit_pond(&bridge(PondShapeMode::Add)).unwrap();
    assert_eq!(reopened, world);
    let daphnia = &world.pond.as_ref().unwrap().basins[0].daphnia;
    assert_eq!(daphnia.count, merged.daphnia.count);
    for element in 0..3 {
        near(daphnia.body.0[element], merged.daphnia.body.0[element]);
        near(daphnia.reserve.0[element], merged.daphnia.reserve.0[element]);
    }
    check_world_total(&world.pond.as_ref().unwrap().basins);
}

#[test]
fn r5_topology_rejects_excess_population_without_changing_either_basin() {
    let mut world = WorldState::new_blank_r5_composed_dual_pond(5);
    for id in [POND_DEFAULT_BASIN_ID, "pond-2"] {
        world.edit_pond(&PondEdit::AddWater { basin_id: id.into(), liters: 10.0 }).unwrap();
        world.edit_pond(&PondEdit::StockDaphnia { basin_id: id.into(), count: 1200 }).unwrap();
    }
    let before = world.clone();
    assert_eq!(world.edit_pond(&bridge(PondShapeMode::Add)), Err(CoreError::PondShape(Error::TopologyPopulation)));
    assert_eq!(world, before);

    let mut world = WorldState::new_blank_r5_composed_dual_pond(5);
    for (id, count, y) in [(POND_DEFAULT_BASIN_ID, 8, 1.5 / 12.0), ("pond-2", 1, 4.5 / 12.0)] {
        world.edit_pond(&PondEdit::AddWater { basin_id: id.into(), liters: 10.0 }).unwrap();
        for _ in 0..count {
            world.edit_pond(&PondEdit::AddFloatingPlant {
                basin_id: id.into(), position: Point { x: 0.4, y }, preset: PondFloatingPlantPreset::Small,
            }).unwrap();
        }
    }
    let before = world.clone();
    assert_eq!(world.edit_pond(&bridge(PondShapeMode::Add)), Err(CoreError::PondShape(Error::TopologyPopulation)));
    assert_eq!(world, before);
}

#[test]
fn r5_rooted_plant_rows_keep_stocks_anchors_and_shared_inputs_across_fusion_and_split() {
    let mut world = WorldState::new_blank_r5_composed_dual_pond(74);
    let main = || POND_DEFAULT_BASIN_ID.to_owned();
    let second = || "pond-2".to_owned();
    for (id, liters) in [(main(), 5.0), (second(), 7.0)] {
        world.edit_pond(&PondEdit::AddWater { basin_id: id, liters }).unwrap();
    }
    for (basin, carbon) in world.pond.as_mut().unwrap().basins.iter_mut().zip([20.0, 40.0]) {
        let stock = Matter::with_ratios(carbon, 0.06, 0.007);
        basin.plants.add(stock);
        basin.balance.inputs.cn_p.add(stock);
    }
    world.edit_pond(&PondEdit::AddWaterPlant { basin_id: main(), x: 0.46 }).unwrap();
    world.edit_pond(&PondEdit::PlaceRibbonPlant { basin_id: second(), x: 0.46, preset: PondRibbonPlantPreset::Large }).unwrap();
    let snapshots = |state: &WorldState| state.pond.as_ref().unwrap().observe().basins;
    let row = snapshots(&world)[1].shape.as_ref().unwrap().protected_row;
    let localized = |edit| PondEdit::EditStaticSection { basin_id: main(), row, edit: Box::new(edit) };
    let near = |value: f64, expected: f64| assert!((value - expected).abs() < 1e-8, "{value} != {expected}");

    world.edit_pond(&bridge(PondShapeMode::Add)).unwrap();
    world.validate().unwrap();
    let merged = snapshots(&world).remove(0);
    near(merged.plant_mg_c, 185.0);
    near(merged.unassigned_plant_mg_cnp.unwrap()[0], 60.0);
    let first = merged.water_plant_patches.unwrap()[0];
    let ribbon = merged.static_sections[0].water_plant_patches.as_ref().unwrap()[0];
    assert_eq!((first.id, ribbon.id, first.x, ribbon.x), (1, 1, 0.46, 0.46));
    near(first.stock_mg_cnp[0], 50.0);
    near(ribbon.stock_mg_cnp[0], 75.0);
    assert!(ribbon.ribbon_light_transmission.unwrap() > 0.0);

    let unchanged = world.clone();
    assert_eq!(world.edit_pond(&localized(PondEdit::AddStone { basin_id: main(), x: 0.46, preset: PondStonePreset::Main })),
        Err(CoreError::PondWaterPlantSiteOccupied));
    assert_eq!(world, unchanged);
    world.edit_pond(&localized(PondEdit::MoveWaterPlant { basin_id: main(), id: 1, x: 0.55 })).unwrap();
    world.edit_pond(&localized(PondEdit::TrimWaterPlant { basin_id: main(), id: 1, mg_c: 5.0 })).unwrap();
    let trimmed = world.clone();
    let add = localized(PondEdit::AddWaterPlant { basin_id: main(), x: 0.78 });
    world.edit_pond(&add).unwrap();
    world.restore_pond_edit_state(trimmed).unwrap();
    world.edit_pond(&add).unwrap();
    let changed = snapshots(&world).remove(0);
    assert_eq!(changed.water_plant_patches.unwrap()[0], first);
    let side = changed.static_sections[0].water_plant_patches.as_ref().unwrap();
    assert_eq!((side[0].id, side[0].x, side[1].id), (1, 0.55, 3));
    near(side[0].stock_mg_cnp[0], 70.0);
    near(changed.detritus_mg_c, 5.0);
    world.edit_pond(&localized(PondEdit::RemoveWaterPlant { basin_id: main(), id: 3 })).unwrap();
    near(snapshots(&world)[0].detritus_mg_c, 55.0);

    let saved = serde_json::to_string(&world).unwrap();
    let mut loaded: WorldState = serde_json::from_str(&saved).unwrap();
    assert_eq!(loaded, world);
    loaded.validate().unwrap();
    let preset = loaded.fresh_habitat(18).unwrap();
    near(snapshots(&preset)[0].static_sections[0].water_plant_patches.as_ref().unwrap()[0].stock_mg_cnp[0], 75.0);
    let before = loaded.pond.as_ref().unwrap().basins[0].inventory();
    loaded.edit_pond(&bridge(PondShapeMode::Remove)).unwrap();
    loaded.validate().unwrap();
    let parts = snapshots(&loaded);
    assert_eq!(parts[1].shape.as_ref().unwrap().protected_row, row);
    for (part, expected_root) in parts.iter().zip([50.0, 70.0]) {
        near(part.unassigned_plant_mg_cnp.unwrap()[0], 30.0);
        near(part.water_plant_patches.as_ref().unwrap()[0].stock_mg_cnp[0], expected_root);
    }
    let basins = &loaded.pond.as_ref().unwrap().basins;
    for element in 0..3 {
        near(basins.iter().map(|basin| basin.inventory().cn_p.0[element]).sum(), before.cn_p.0[element]);
        near(basins.iter().map(|basin| basin.ledger().matter_residual_mg_cnp[element].abs()).sum(), 0.0);
    }
}

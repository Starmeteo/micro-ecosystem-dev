use super::*;

#[test]
fn r5_local_biofilm_grazing_survives_checkpoint_and_recovers_on_a_wet_stone() {
    let directory = std::env::temp_dir().join(format!("micro-ecosystem-stone-film-{}", Uuid::new_v4()));
    let store = SaveStore::new(&directory);
    let mut runtime = WorldRuntime::new();
    let world_id = "pond-local-film";
    let basin_id = || eco_core::POND_DEFAULT_BASIN_ID.to_owned();
    let mut state = WorldState::new_r5_composed_pond(2729);
    state.edit_pond(&PondEdit::RemoveWood { basin_id: basin_id(), id: 1 }).unwrap();
    state.edit_pond(&PondEdit::StockShrimp { basin_id: basin_id(), position: Point { x: 0.60, y: 0.785 }, count: 4 }).unwrap();
    let legacy = runtime.create_with_solar(world_id, state).unwrap();
    let edit = |runtime: &mut WorldRuntime, snapshot: &Snapshot, command, action| {
        runtime.edit_pond_command(&snapshot.world_id, command, &snapshot.run_generation, &snapshot.edit_revision, action)
    };
    let view = |snapshot: &Snapshot| snapshot.pond.as_ref().unwrap().basins[0].clone();
    let stock = |snapshot: &Snapshot| view(snapshot).stone_biofilm.unwrap()[0].stock_mg_c;
    assert!(view(&legacy).stone_biofilm.is_none());
    let paused = runtime.set_edit_paused(world_id, true).unwrap();
    let enabled = edit(&mut runtime, &paused, "local-film", PondEdit::EnableStoneBiofilm { basin_id: basin_id() }).unwrap();
    assert_eq!(view(&enabled).stone_biofilm_enabled, Some(true));
    assert!((view(&enabled).biofilm_mg_c - view(&legacy).biofilm_mg_c).abs() < 1e-12);
    assert!(stock(&enabled) > 0.0);

    let mut prior = runtime.start_trial(world_id).unwrap();
    let grazed = (0..400).find_map(|_| {
        let next = runtime.advance_trial(&prior.world_id, 1).unwrap();
        let ate_local = view(&next).stone_contact_feeding.iter().flatten().any(|event| event.stone_biofilm_mg_c > 0.0);
        let depleted = stock(&next) < stock(&prior);
        prior = next.clone();
        (ate_local && depleted).then_some(next)
    }).expect("the locally grazed stock declines during a confirmed contact");
    assert_eq!(runtime.snapshot(world_id).unwrap(), enabled);
    let adopted = runtime.apply_trial(&store, &grazed.world_id).unwrap();
    assert_eq!(view(&adopted).stone_biofilm, view(&grazed).stone_biofilm);
    assert!(view(&adopted).stone_contact_feeding.is_none());
    let durable = runtime.world(world_id).unwrap().saved.clone();
    let mut cold = WorldRuntime::new();
    let loaded = cold.load(&store, world_id).unwrap().snapshot;
    assert_eq!(cold.world(world_id).unwrap().saved, durable);
    assert_eq!(view(&loaded).stone_biofilm, view(&adopted).stone_biofilm);

    let recovery = cold.start_trial(world_id).unwrap();
    let liters = view(&loaded).water_liters;
    let dry = edit(&mut cold, &recovery, "dry-film", PondEdit::DrainWater { basin_id: basin_id(), liters }).unwrap();
    assert_eq!(view(&dry).stone_biofilm.as_ref().unwrap()[0].available_mg_c, 0.0);
    assert_eq!(stock(&dry), stock(&loaded));
    let wet = edit(&mut cold, &dry, "wet-film", PondEdit::AddWater { basin_id: basin_id(), liters }).unwrap();
    assert!(view(&wet).shrimp.is_empty());
    assert_eq!(stock(&wet), stock(&dry));
    let recovered = cold.advance_trial(&recovery.world_id, 1200).unwrap();
    assert!(stock(&recovered) > stock(&wet));
    let removed = edit(&mut cold, &recovered, "move-to-shared", PondEdit::RemoveStone { basin_id: basin_id(), id: 1 }).unwrap();
    assert!(view(&removed).stone_biofilm.unwrap_or_default().is_empty());
    assert!((view(&removed).biofilm_mg_c - view(&recovered).biofilm_mg_c).abs() < 1e-12);
    assert_eq!(cold.snapshot(world_id).unwrap(), loaded);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn r5_main_stone_contact_survives_cold_load_but_intake_is_live_only() {
    let directory = std::env::temp_dir().join(format!("micro-ecosystem-stone-contact-{}", Uuid::new_v4()));
    let store = SaveStore::new(&directory);
    let mut runtime = WorldRuntime::new();
    let world_id = "pond-stone-contact";
    let basin_id = eco_core::POND_DEFAULT_BASIN_ID.to_owned();
    let mut state = WorldState::new_r5_composed_pond(2729);
    state.edit_pond(&PondEdit::RemoveWood { basin_id: basin_id.clone(), id: 1 }).unwrap();
    state.edit_pond(&PondEdit::StockShrimp { basin_id: basin_id.clone(), position: Point { x: 0.60, y: 0.785 }, count: 4 }).unwrap();
    runtime.create_with_solar(world_id, state).unwrap();
    let basin = |snapshot: &Snapshot| snapshot.pond.as_ref().unwrap().basins[0].clone();
    let observed = (0..400).find_map(|_| {
        let snapshot = runtime.advance_ticks(world_id, 1).unwrap();
        let view = basin(&snapshot);
        let retained = view.stone_contact_feeding.iter().flatten().any(|event| {
            event.stone_biofilm_mg_c > 0.0 && view.stone_attachments.iter().flatten()
                .any(|link| link.shrimp_id == event.shrimp_id && link.rest_ticks > 3)
        });
        retained.then_some(snapshot)
    }).expect("nearby V13 shrimp lands and feeds at the main stone");
    let observed_basin = basin(&observed);
    assert!(observed_basin.wood_contact_feeding.is_none());
    for event in observed_basin.stone_contact_feeding.as_ref().unwrap() {
        assert!((event.stone_biofilm_mg_c + event.shared_biofilm_mg_c - event.biofilm_mg_c).abs() < 1e-12);
    }
    assert_eq!(runtime.advance_ticks(world_id, 0).unwrap(), observed);
    let durable = runtime.world(world_id).unwrap().saved.clone();
    assert!(!serde_json::to_string(&durable).unwrap().contains("stoneContactFeeding"));
    runtime.save(&store, world_id).unwrap();
    let mut reopened = WorldRuntime::new();
    let cold = reopened.load(&store, world_id).unwrap().snapshot;
    assert_eq!(reopened.world(world_id).unwrap().saved, durable);
    assert_eq!(basin(&cold).stone_attachments, observed_basin.stone_attachments);
    assert!(basin(&cold).stone_contact_feeding.is_none());

    let paused = runtime.set_edit_paused(world_id, true).unwrap();
    assert_eq!(basin(&paused).stone_attachments, observed_basin.stone_attachments);
    assert!(basin(&paused).stone_contact_feeding.is_none());
    let trial = runtime.start_trial(world_id).unwrap();
    let edited = runtime.edit_pond_command(&trial.world_id, "remove-stone", &trial.run_generation, &trial.edit_revision,
        PondEdit::RemoveStone { basin_id, id: 1 }).unwrap();
    assert!(basin(&edited).stone_attachments.is_none());
    assert!(basin(&edited).stone_contact_feeding.is_none());
    let undone = runtime.edit_pond_history_command(&trial.world_id, "undo-stone", &edited.run_generation, &edited.edit_revision, false).unwrap();
    assert_eq!(basin(&undone).stone_attachments, observed_basin.stone_attachments);
    assert!(basin(&undone).stone_contact_feeding.is_none());
    assert_eq!(runtime.snapshot(world_id).unwrap(), paused);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn r5_wood_contact_feeding_is_live_only_through_checkpoint_pause_and_edit() {
    let directory = std::env::temp_dir().join(format!("micro-ecosystem-pond-contact-{}", Uuid::new_v4()));
    let store = SaveStore::new(&directory);
    let mut runtime = WorldRuntime::new();
    let world_id = "pond-contact";
    runtime.create_with_solar(world_id, WorldState::new_r5_composed_dual_pond(2729)).unwrap();
    let has_contact_feeding = |snapshot: &Snapshot| {
        snapshot.pond.as_ref().unwrap().basins.iter().any(|basin| basin.wood_contact_feeding.is_some())
    };
    let observed = (0..400).find_map(|_| {
        let snapshot = runtime.advance_ticks(world_id, 20).unwrap();
        let contact_continues = snapshot.pond.as_ref().unwrap().basins.iter().any(|basin| {
            basin.wood_contact_feeding.iter().flatten().any(|entry| basin.shrimp_attachments.iter().flatten()
                .any(|attachment| entry.shrimp_id == attachment.shrimp_id && attachment.rest_ticks > 3))
        });
        contact_continues.then_some(snapshot)
    }).expect("ordinary V13 stepping establishes wood-contact feeding");
    assert!(has_contact_feeding(&observed));
    assert_eq!(runtime.advance_ticks(world_id, 0).unwrap(), observed);
    let durable = runtime.world(world_id).unwrap().saved.clone();
    let serialized = serde_json::to_string(&durable).unwrap();
    assert!(!serialized.contains("woodContactFeeding"));
    assert!(serde_json::to_string(&observed).unwrap().contains("woodContactFeeding"));
    runtime.save(&store, world_id).unwrap();
    let mut reopened = WorldRuntime::new();
    let cold = reopened.load(&store, world_id).unwrap().snapshot;
    assert!(!has_contact_feeding(&cold));
    assert_eq!(reopened.world(world_id).unwrap().saved, durable);

    let paused = runtime.set_edit_paused(world_id, true).unwrap();
    assert!(!has_contact_feeding(&paused));
    let trial = runtime.start_trial(world_id).unwrap();
    assert!(!has_contact_feeding(&trial));
    let advanced = runtime.advance_trial(&trial.world_id, 1).unwrap();
    let basin = advanced.pond.as_ref().unwrap().basins.iter().find(|basin| basin.wood_contact_feeding.is_some()).unwrap();
    let drained = runtime.edit_pond_command(&trial.world_id, "drain", &advanced.run_generation, &advanced.edit_revision,
        PondEdit::DrainWater { basin_id: basin.id.clone(), liters: basin.water_liters }).unwrap();
    assert!(!has_contact_feeding(&drained));
    let undone = runtime.edit_pond_history_command(&trial.world_id, "undo-drain", &drained.run_generation, &drained.edit_revision, false).unwrap();
    assert!(!has_contact_feeding(&undone));
    assert_eq!(runtime.snapshot(world_id).unwrap(), paused);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn forest_r5_composed_scopes_moves_and_attachment_history_to_the_same_save() {
    let path = std::env::temp_dir().join(format!("micro-ecosystem-forest-r5-composed-{}", Uuid::new_v4()));
    let store = SaveStore::new(&path);
    let mut runtime = WorldRuntime::new();
    let start = runtime.create_library_world(&store, "forest-r5-composed", 1206).unwrap();
    let blank = runtime.create_library_world(&store, "blank-forest-r5-composed", 1206).unwrap();
    let legacy = runtime.create_library_world(&store, "forest-r5-litter", 1206).unwrap();
    let scene = |snapshot: &Snapshot| snapshot.forest_rocks.as_ref().unwrap().scenery.clone().unwrap();
    let edit = |runtime: &mut WorldRuntime, snap: &Snapshot, command_id, edit| {
        runtime.edit_forest_rock_command(&snap.world_id, command_id, &snap.run_generation, &snap.edit_revision,
            ForestRockCommand::Change { edit })
    };
    let history = |runtime: &mut WorldRuntime, snap: &Snapshot, command_id, action| {
        runtime.edit_forest_rock_command(&snap.world_id, command_id, &snap.run_generation, &snap.edit_revision, action).unwrap()
    };

    assert_eq!(start.model_version, 14);
    assert_eq!(start.definition, "forest-r5-composed/1");
    assert!(start.season.is_some());
    assert!(scene(&blank).is_empty() && blank.animals.is_empty() && blank.starter_community_available);
    let empty_layout = blank.forest_rocks.as_ref().unwrap();
    assert!(empty_layout.rocks.is_empty());
    assert_eq!(empty_layout.bark_shelters, Some(vec![]));
    assert_eq!(empty_layout.leaf_patches, Some(vec![]));
    let seeded = start.forest_rocks.as_ref().unwrap();
    assert_eq!(seeded.rocks[0].position, Point { x: 497.0 / 576.0, y: 242.0 / 324.0 });
    let arch = &seeded.bark_shelters.as_ref().unwrap()[0];
    assert_eq!(arch.shelter.position, Point { x: 88.0 / 576.0, y: 232.0 / 324.0 });
    assert_eq!(arch.foot_cells, [12, 14]);
    assert_eq!(arch.passage_cells, [1, 13, 25]);
    let leaf = &seeded.leaf_patches.as_ref().unwrap()[0];
    assert_eq!(leaf.patch.id, 3);
    assert_eq!(leaf.patch.position, Point { x: 361.0 / 576.0, y: 254.0 / 324.0 });
    assert_eq!(leaf.patch.remaining_mg_cnp[0], 60.0);
    let carrier = Some(ForestSceneryParent::Terrain { id: arch.shelter.id });
    assert!(scene(&start).iter().any(|item| item.parent == carrier));

    let manifest: serde_json::Value = serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),
        "/../../apps/desktop/src/render/assets/r5/forest-composed/definition.json"))).unwrap();
    let attachable = manifest["styles"].as_array().unwrap().iter().find(|item| item["placeable"] == true
        && item["attachedOnly"] == true && item["parentKinds"].as_array().is_some_and(|kinds| kinds.iter().any(|kind| kind == "terrain"))).unwrap();
    let style = attachable["id"].as_str().unwrap().to_owned();
    let attached = ForestRockEdit::AddScenery { style: style.clone(), x: -0.12, y: -0.10, parent: carrier };
    assert!(legacy.forest_rocks.as_ref().unwrap().scenery.is_none());
    assert!(edit(&mut runtime, &legacy, "old", attached.clone()).is_err());
    assert!(edit(&mut runtime, &start, "no-parent", ForestRockEdit::AddScenery {
        style: style.clone(), x: 0.5, y: 0.5, parent: None,
    }).is_err());
    let invalid_parent = ForestRockEdit::AddScenery { style, x: 0., y: 0., parent: Some(ForestSceneryParent::Terrain { id: 90 }) };
    assert!(edit(&mut runtime, &start, "bad-parent", invalid_parent).is_err());
    assert_eq!(runtime.snapshot(&start.world_id).unwrap(), start);

    let trial = runtime.start_trial(&start.world_id).unwrap();
    let stock = runtime.forest_ledger(&trial.world_id).unwrap();
    let placed = edit(&mut runtime, &trial, "place", attached.clone()).unwrap();
    assert_eq!(edit(&mut runtime, &trial, "place", attached.clone()).unwrap(), placed);
    let id = scene(&placed).last().unwrap().id;
    let undone = history(&mut runtime, &placed, "undo-place", ForestRockCommand::Undo);
    assert_eq!(scene(&undone), scene(&trial));
    let replaced = edit(&mut runtime, &undone, "replace", attached).unwrap();
    let moved_id = scene(&replaced).last().unwrap().id;
    assert!(moved_id > id);
    let moved = edit(&mut runtime, &replaced, "move", ForestRockEdit::MoveScenery { id: moved_id, x: -0.21, y: 0.11 }).unwrap();
    assert_eq!(scene(&moved).last().unwrap().parent, carrier);
    let removed = edit(&mut runtime, &moved, "remove-carrier", ForestRockEdit::Remove { id: arch.shelter.id }).unwrap();
    assert!(scene(&removed).iter().all(|item| item.parent != carrier));
    assert!(scene(&removed).iter().all(|item| item.id != moved_id));
    assert_eq!(runtime.forest_ledger(&trial.world_id).unwrap(), stock);
    let restored = history(&mut runtime, &removed, "undo-carrier", ForestRockCommand::Undo);
    assert_eq!(scene(&restored), scene(&moved));
    assert_eq!(runtime.snapshot(&start.world_id).unwrap(), start);
    let adopted = runtime.apply_trial(&store, &trial.world_id).unwrap();
    let reopened = WorldRuntime::new().load(&store, &start.world_id).unwrap().snapshot;
    assert_eq!(scene(&reopened), scene(&adopted));
    let preset = runtime.save_custom_preset(&store, &start.world_id, "林下分层造景回归", 1206).unwrap();
    let fresh = runtime.create_library_world(&store, &preset.preset_id, 1206).unwrap();
    assert_eq!(scene(&fresh), scene(&adopted));
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn r5_composed_scenery_survives_trial_history_and_save_without_changing_stocks() {
    let path = std::env::temp_dir().join(format!("micro-ecosystem-r5-composed-{}", Uuid::new_v4()));
    let store = SaveStore::new(&path);
    let mut runtime = WorldRuntime::new();
    let start = runtime.create_library_world(&store, "pond-r5-composed", 2729).unwrap();
    let blank = runtime.create_library_world(&store, "blank-pond-r5-composed", 2729).unwrap();
    let old = runtime.create_library_world(&store, "pond-r5-moss", 2729).unwrap();
    let basin_id = || eco_core::POND_DEFAULT_BASIN_ID.to_owned();
    let basin = |snap: &Snapshot| snap.pond.as_ref().unwrap().basins[0].clone();
    let edit = |runtime: &mut WorldRuntime, snap: &Snapshot, command_id: &str, edit| {
        runtime.edit_pond_command(&snap.world_id, command_id, &snap.run_generation, &snap.edit_revision, edit)
    };
    let history = |runtime: &mut WorldRuntime, snap: &Snapshot, command_id: &str, redo| {
        runtime.edit_pond_history_command(&snap.world_id, command_id, &snap.run_generation, &snap.edit_revision, redo).unwrap()
    };
    let initial = basin(&start);
    let scenery = initial.scenery.unwrap();
    let wood_id = initial.woods.unwrap()[0].id;
    let style = "flat-stone".to_owned();
    assert_eq!(basin(&blank).scenery, Some(Vec::new()));
    assert_eq!(basin(&blank).woods, Some(Vec::new()));
    assert_eq!(basin(&blank).stones, Some(Vec::new()));
    assert!(basin(&blank).shrimp.is_empty() && !basin(&blank).wet);
    assert!(basin(&old).scenery.is_none());
    assert!(edit(&mut runtime, &old, "old-add", PondEdit::AddScenery {
        basin_id: basin_id(), style: style.clone(), x: 0.5, y: 0.5,
    }).is_err());
    assert!(edit(&mut runtime, &old, "old-slope", PondEdit::AddWood {
        basin_id: basin_id(), x: 0.235, preset: PondWoodPreset::Slope,
    }).is_err());

    let trial = runtime.start_trial(&start.world_id).unwrap();
    let ledger = runtime.pond_ledger(&trial.world_id).unwrap().unwrap();
    let add = PondEdit::AddScenery { basin_id: basin_id(), style: style.clone(), x: -0.12, y: 0.75 };
    let placed = edit(&mut runtime, &trial, "place", add.clone()).unwrap();
    assert_eq!(edit(&mut runtime, &trial, "place", add.clone()).unwrap(), placed);
    let id = basin(&placed).scenery.unwrap().last().unwrap().id;
    assert!(edit(&mut runtime, &placed, "range", PondEdit::MoveScenery {
        basin_id: basin_id(), id, x: 1.13, y: 0.5,
    }).is_err());
    let undone = history(&mut runtime, &placed, "undo-place", false);
    assert_eq!(basin(&undone).scenery.as_ref(), Some(&scenery));
    let replaced = edit(&mut runtime, &undone, "replace", add).unwrap();
    let new_id = basin(&replaced).scenery.unwrap().last().unwrap().id;
    assert!(new_id > id);
    let moved = edit(&mut runtime, &replaced, "move", PondEdit::MoveScenery {
        basin_id: basin_id(), id: new_id, x: 0.45, y: 0.9,
    }).unwrap();
    let removed = edit(&mut runtime, &moved, "remove-carrier", PondEdit::RemoveWood {
        basin_id: basin_id(), id: wood_id,
    }).unwrap();
    assert!(basin(&removed).scenery.unwrap().len() < basin(&moved).scenery.unwrap().len());
    assert_eq!(runtime.pond_ledger(&trial.world_id).unwrap().unwrap(), ledger);
    assert_eq!(runtime.snapshot(&start.world_id).unwrap(), start);
    let restored = history(&mut runtime, &removed, "restore-carrier", false);
    assert_eq!(basin(&restored).scenery, basin(&moved).scenery);
    let adopted = runtime.apply_trial(&store, &trial.world_id).unwrap();
    let reopened = WorldRuntime::new().load(&store, &start.world_id).unwrap().snapshot;
    assert_eq!(reopened.pond, adopted.pond);
    let preset = runtime.save_custom_preset(&store, &start.world_id, "水下造景回归", 2729).unwrap();
    let fresh = runtime.create_library_world(&store, &preset.preset_id, 2729).unwrap();
    assert_eq!(basin(&fresh).scenery, basin(&adopted).scenery);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn r5_pond_moss_history_preserves_patch_identity_and_elemental_transfers() {
    let path = std::env::temp_dir().join(format!("micro-ecosystem-r5-pond-moss-{}", Uuid::new_v4()));
    let store = SaveStore::new(&path);
    let mut runtime = WorldRuntime::new();
    let old = runtime.create_library_world(&store, "pond-r5-wood", 2729).unwrap();
    let start = runtime.create_library_world(&store, "pond-r5-moss", 2729).unwrap();
    let id = &start.world_id;
    let basin_id = || eco_core::POND_DEFAULT_BASIN_ID.to_owned();
    let edit = |runtime: &mut WorldRuntime, from: &Snapshot, command_id: &str, action| {
        runtime.edit_pond_command(&from.world_id, command_id, &from.run_generation, &from.edit_revision, action)
    };
    let history = |runtime: &mut WorldRuntime, from: &Snapshot, command_id: &str, redo| {
        runtime.edit_pond_history_command(&from.world_id, command_id, &from.run_generation, &from.edit_revision, redo)
    };
    let basin = |snapshot: &Snapshot| snapshot.pond.as_ref().unwrap().basins[0].clone();
    assert!(id.starts_with("pond-") && start.season.is_some());
    assert!(basin(&old).moss_patches.is_none());
    assert_eq!(basin(&start).moss_patches, Some(Vec::new()));
    let initial_ledger = runtime.pond_ledger(id).unwrap().unwrap();
    let stone = edit(&mut runtime, &start, "stone", PondEdit::AddStone { basin_id: basin_id(), x: 0.37, preset: PondStonePreset::Main }).unwrap();
    let supports = edit(&mut runtime, &stone, "wood", PondEdit::AddWood { basin_id: basin_id(), x: 0.74, preset: PondWoodPreset::Root }).unwrap();
    let stone_id = basin(&supports).stones.unwrap()[0].id;
    let wood_id = basin(&supports).woods.unwrap()[0].id;
    assert_eq!(stone_id, wood_id);
    let mat_parent = PondMossParent::Stone { id: stone_id };
    let frond_parent = PondMossParent::Wood { id: wood_id };
    let mat = PondEdit::AddMoss { basin_id: basin_id(), parent: mat_parent, offset: 0.0, preset: PondMossPreset::Mat };
    assert!(edit(&mut runtime, &old, "legacy-plant", mat.clone()).is_err());
    let planted = edit(&mut runtime, &supports, "plant", mat.clone()).unwrap();
    assert_eq!(edit(&mut runtime, &supports, "plant", mat.clone()).unwrap(), planted);
    assert!(edit(&mut runtime, &planted, "occupied", mat).is_err());
    let all = edit(&mut runtime, &planted, "plant-wood", PondEdit::AddMoss {
        basin_id: basin_id(), parent: frond_parent, offset: 0.2, preset: PondMossPreset::Frond,
    }).unwrap();
    let patches = basin(&all).moss_patches.unwrap();
    assert_eq!(patches.len(), 2);
    assert!(patches.iter().all(|patch| patch.submerged));
    assert_eq!([patches[0].stock_mg_cnp[0], patches[1].stock_mg_cnp[0]], [3.0, 7.5]);
    let planted_ledger = runtime.pond_ledger(id).unwrap().unwrap();
    for index in 0..3 {
        let imported = planted_ledger.basins[0].matter_input_mg_cnp[index] - initial_ledger.basins[0].matter_input_mg_cnp[index];
        assert!((imported - patches[0].stock_mg_cnp[index] - patches[1].stock_mg_cnp[index]).abs() < 1e-9);
    }
    assert_eq!(basin(&all).biofilm_mg_c, basin(&supports).biofilm_mg_c);

    let shifted = edit(&mut runtime, &all, "shift-patch", PondEdit::MoveMoss { basin_id: basin_id(), id: patches[0].id, offset: 0.4 }).unwrap();
    let dragged = edit(&mut runtime, &shifted, "shift-parent", PondEdit::MoveWood { basin_id: basin_id(), id: wood_id, x: 0.73 }).unwrap();
    let moved = basin(&dragged).moss_patches.unwrap();
    assert_eq!(moved[1].offset, patches[1].offset);
    assert!((moved[1].x - patches[1].x + 0.01).abs() < 1e-12);
    assert_eq!(moved[1].stock_mg_cnp, patches[1].stock_mg_cnp);
    assert_eq!(moved[0], basin(&shifted).moss_patches.unwrap()[0]);
    assert_eq!(runtime.pond_ledger(id).unwrap().unwrap(), planted_ledger);

    let fork = runtime.start_trial(id).unwrap();
    let aged = runtime.advance_trial(&fork.world_id, 1200).unwrap();
    let live = basin(&aged).moss_patches.unwrap();
    assert!(live[0].stock_mg_cnp[0] > moved[0].stock_mg_cnp[0]);
    let branch_ledger = runtime.pond_ledger(&fork.world_id).unwrap().unwrap();
    let trimmed = edit(&mut runtime, &aged, "trim", PondEdit::TrimMoss { basin_id: basin_id(), id: live[0].id, mg_c: 1.0 }).unwrap();
    let after_trim = basin(&trimmed);
    assert!((after_trim.detritus_mg_c - basin(&aged).detritus_mg_c - 1.0).abs() < 1e-9);
    assert!((after_trim.moss_patches.as_ref().unwrap()[0].stock_mg_cnp[0] - live[0].stock_mg_cnp[0] + 1.0).abs() < 1e-9);
    let removed = edit(&mut runtime, &trimmed, "remove-wood", PondEdit::RemoveWood { basin_id: basin_id(), id: wood_id }).unwrap();
    let dropped = basin(&removed);
    assert_eq!(dropped.moss_patches.as_ref().unwrap().len(), 1);
    assert!((dropped.detritus_mg_c - after_trim.detritus_mg_c - live[1].stock_mg_cnp[0]).abs() < 1e-9);
    let ledger_after = runtime.pond_ledger(&fork.world_id).unwrap().unwrap();
    assert_eq!(ledger_after.basins[0].matter_input_mg_cnp, branch_ledger.basins[0].matter_input_mg_cnp);
    assert_eq!(ledger_after.basins[0].matter_output_mg_cnp, branch_ledger.basins[0].matter_output_mg_cnp);
    for index in 0..3 {
        assert!((ledger_after.basins[0].current_matter_mg_cnp[index] - branch_ledger.basins[0].current_matter_mg_cnp[index]).abs() < 1e-8);
    }
    assert_eq!(runtime.snapshot(id).unwrap(), dragged);
    let restored = history(&mut runtime, &removed, "restore-wood", false).unwrap();
    assert_eq!(restored.pond, trimmed.pond);
    let redo = history(&mut runtime, &restored, "drop-again", true).unwrap();
    assert_eq!(redo.pond, removed.pond);
    let chosen = history(&mut runtime, &redo, "restore-again", false).unwrap();
    let adopted = runtime.apply_trial(&store, &fork.world_id).unwrap();
    assert_eq!(adopted.pond, chosen.pond);
    let reopened = WorldRuntime::new().load(&store, id).unwrap().snapshot;
    assert_eq!(reopened.pond, adopted.pond);
    assert_ne!(reopened.run_generation, adopted.run_generation);
    let copy = runtime.copy_world(&store, id).unwrap();
    assert_eq!(copy.pond, adopted.pond);
    let preset = runtime.save_custom_preset(&store, id, "附着斑块回归", 2729).unwrap();
    let fresh = runtime.create_library_world(&store, &preset.preset_id, 2729).unwrap();
    let fresh = basin(&fresh).moss_patches.unwrap();
    assert_eq!(fresh[0].parent, moved[0].parent);
    assert_eq!(fresh[0].offset, moved[0].offset);
    assert_eq!(fresh[0].stock_mg_cnp, moved[0].stock_mg_cnp);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn r5_pond_leaf_deposit_survives_history_trial_presets_and_readback() {
    let path = std::env::temp_dir().join(format!("micro-ecosystem-r5-pond-leaf-deposit-{}", Uuid::new_v4()));
    let store = SaveStore::new(&path);
    let mut runtime = WorldRuntime::new();
    let start = runtime.create_library_world(&store, "pond-r5-composed", 2729).unwrap();
    let id = &start.world_id;
    let view = |snapshot: &Snapshot| snapshot.pond.as_ref().unwrap().basins[0].clone();
    let edit = |runtime: &mut WorldRuntime, from: &Snapshot, command, action| {
        runtime.edit_pond_command(&from.world_id, command, &from.run_generation, &from.edit_revision, action)
    };
    let history = |runtime: &mut WorldRuntime, from: &Snapshot, command, redo| {
        runtime.edit_pond_history_command(&from.world_id, command, &from.run_generation, &from.edit_revision, redo)
    };
    let wire = serde_json::json!({"type":"addLeafDeposit","basinId":"pond-main","x":0.43,"preset":"foreground","mgC":24.0});
    let add: PondEdit = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(&add).unwrap(), wire);
    let original = runtime.pond_ledger(id).unwrap().unwrap();
    let placed = edit(&mut runtime, &start, "place", add.clone()).unwrap();
    assert_eq!(edit(&mut runtime, &start, "place", add.clone()).unwrap(), placed);
    let deposit = view(&placed).leaf_deposits.unwrap()[0];
    assert_eq!((deposit.id, deposit.initial_mg_c, deposit.remaining_mg_c), (1, 24.0, 24.0));
    let placed_ledger = runtime.pond_ledger(id).unwrap().unwrap();

    let slide = PondEdit::MoveLeafDeposit { basin_id: eco_core::POND_DEFAULT_BASIN_ID.into(), id: 1, x: 0.56 };
    let moved = edit(&mut runtime, &placed, "slide", slide).unwrap();
    assert_eq!(view(&moved).leaf_deposits.unwrap()[0].x, 0.56);
    assert_eq!(runtime.pond_ledger(id).unwrap().unwrap(), placed_ledger);
    let unmoved = history(&mut runtime, &moved, "unmove", false).unwrap();
    assert_eq!(view(&unmoved), view(&placed));
    let undone = history(&mut runtime, &unmoved, "unplace", false).unwrap();
    assert_eq!(view(&undone), view(&start));
    assert_eq!(runtime.pond_ledger(id).unwrap().unwrap(), original);
    let redone = history(&mut runtime, &undone, "redo", true).unwrap();
    assert_eq!(view(&redone), view(&placed));
    let undone = history(&mut runtime, &redone, "undo-save", false).unwrap();
    runtime.save(&store, id).unwrap();

    let mut reopened = WorldRuntime::new();
    let restored = reopened.load(&store, id).unwrap().snapshot;
    assert_eq!(view(&restored), view(&undone));
    let placed = edit(&mut reopened, &restored, "place-new-id", add).unwrap();
    assert_eq!(view(&placed).leaf_deposits.unwrap()[0].id, 2);
    let trial = reopened.start_trial(id).unwrap();
    let advanced = reopened.advance_trial(&trial.world_id, 32).unwrap();
    assert_eq!(reopened.snapshot(id).unwrap(), placed);
    let aged = view(&advanced).leaf_deposits.unwrap()[0];
    assert!(aged.remaining_mg_c < aged.initial_mg_c);
    let adopted = reopened.apply_trial(&store, &trial.world_id).unwrap();
    assert_eq!(view(&adopted), view(&advanced));
    let taken = edit(&mut reopened, &adopted, "take-leaf", PondEdit::RemoveLeafDeposit {
        basin_id: eco_core::POND_DEFAULT_BASIN_ID.into(), id: aged.id,
    }).unwrap();
    assert!(view(&taken).leaf_deposits.is_none());
    reopened.save(&store, id).unwrap();
    let persisted_without_leaf = WorldRuntime::new().load(&store, id).unwrap().snapshot;
    assert_eq!(view(&persisted_without_leaf), view(&taken));
    let adopted = history(&mut reopened, &taken, "undo-take", false).unwrap();
    assert_eq!(view(&adopted), view(&advanced));
    reopened.save(&store, id).unwrap();
    let copy = reopened.copy_world(&store, id).unwrap();
    assert_eq!(view(&copy), view(&adopted));
    let persisted = WorldRuntime::new().load(&store, id).unwrap().snapshot;
    assert_eq!(view(&persisted), view(&adopted));
    let preset = reopened.save_custom_preset(&store, id, "沉叶池底", 2729).unwrap();
    let fresh = reopened.create_library_world(&store, &preset.preset_id, 2729).unwrap();
    let fresh_leaf = view(&fresh).leaf_deposits.unwrap()[0];
    assert_eq!((fresh_leaf.id, fresh_leaf.x, fresh_leaf.remaining_mg_c), (aged.id, aged.x, aged.initial_mg_c));
    assert_eq!(reopened.pond_ledger(&fresh.world_id).unwrap().unwrap().basins[0].matter_input_mg_cnp, [0.0; 3]);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn r5_pond_shape_stroke_is_one_retriable_edit_and_survives_history_and_restore() {
    let path = std::env::temp_dir().join(format!("micro-ecosystem-r5-pond-shape-{}", Uuid::new_v4()));
    let store = SaveStore::new(&path);
    let mut runtime = WorldRuntime::new();
    let start = runtime.create_library_world(&store, "pond-r5-composed", 2729).unwrap();
    let id = &start.world_id;
    let view = |snapshot: &Snapshot| snapshot.pond.as_ref().unwrap().basins[0].clone();
    let edit = |runtime: &mut WorldRuntime, from: &Snapshot, command, action| {
        runtime.edit_pond_command(&from.world_id, command, &from.run_generation, &from.edit_revision, action)
    };
    let history = |runtime: &mut WorldRuntime, from: &Snapshot, command, redo| {
        runtime.edit_pond_history_command(&from.world_id, command, &from.run_generation, &from.edit_revision, redo)
    };
    let ledger = runtime.pond_ledger(id).unwrap().unwrap();

    let wire = serde_json::json!({"type":"paintShape", "basinId":"pond-main", "mode":"add", "cells":[41,53]});
    let stroke: PondEdit = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(&stroke).unwrap(), wire);
    let expanded = edit(&mut runtime, &start, "expand", stroke.clone()).unwrap();
    assert_eq!(expanded.edit_revision, (start.edit_revision.parse::<u64>().unwrap() + 1).to_string());
    assert_eq!(edit(&mut runtime, &start, "expand", stroke).unwrap(), expanded);
    assert_eq!(view(&expanded).surface_area_cm2, 2600.0);
    assert_eq!(view(&expanded).shape.as_ref().unwrap().cells[..2], [41, 53]);
    assert_eq!(runtime.pond_ledger(id).unwrap().unwrap(), ledger);

    let remove = |cells| PondEdit::PaintShape { basin_id: eco_core::POND_DEFAULT_BASIN_ID.into(), mode: PondShapeMode::Remove, cells };
    let error = edit(&mut runtime, &expanded, "cut", remove(vec![53])).unwrap_err();
    assert_eq!(error, RuntimeError::PondShape { issue: PondShapeError::Disconnected });
    assert_eq!(PondShapeError::Disconnected.code(), "pond_shape_disconnected");
    assert_eq!(runtime.snapshot(id).unwrap(), expanded);
    let undone = history(&mut runtime, &expanded, "undo", false).unwrap();
    assert_eq!(view(&undone), view(&start));
    let redone = history(&mut runtime, &undone, "redo", true).unwrap();
    assert_eq!(view(&redone), view(&expanded));
    assert_eq!(runtime.pond_ledger(id).unwrap().unwrap(), ledger);

    let trial = runtime.start_trial(id).unwrap();
    let shaved = edit(&mut runtime, &trial, "trim-shape", remove(vec![41])).unwrap();
    assert_eq!(runtime.snapshot(id).unwrap(), redone);
    let adopted = runtime.apply_trial(&store, &trial.world_id).unwrap();
    assert_eq!(view(&adopted), view(&shaved));
    assert_eq!(runtime.pond_ledger(id).unwrap().unwrap(), ledger);
    let copied = runtime.copy_world(&store, id).unwrap();
    assert_eq!(view(&copied), view(&adopted));
    let reopened = WorldRuntime::new().load(&store, id).unwrap().snapshot;
    assert_eq!(view(&reopened), view(&adopted));
    let preset = runtime.save_custom_preset(&store, id, "水域形状", 2729).unwrap();
    let fresh = runtime.create_library_world(&store, &preset.preset_id, 2729).unwrap();
    assert_eq!(view(&fresh).shape, view(&adopted).shape);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn dual_pond_preset_keeps_water_and_shoreline_edits_independent() {
    let path = std::env::temp_dir().join(format!("micro-ecosystem-r5-dual-{}", Uuid::new_v4()));
    let store = SaveStore::new(&path);
    let mut runtime = WorldRuntime::new();
    let start = runtime.create_library_world(&store, "pond-r5-dual", 2729).unwrap();
    let initial = start.pond.as_ref().unwrap();
    assert_eq!(initial.basins.len(), 2);
    let first = &initial.basins[0];
    let second = &initial.basins[1];
    assert_ne!(first.id, second.id);
    assert_ne!(first.shape.as_ref().unwrap().cells, second.shape.as_ref().unwrap().cells);

    let apply = |runtime: &mut WorldRuntime, state: &Snapshot, command, edit| {
        runtime.edit_pond_command(&state.world_id, command, &state.run_generation, &state.edit_revision, edit)
    };
    let joined = PondEdit::PaintShape { basin_id: first.id.clone(), mode: PondShapeMode::Add, cells: vec![24] };
    assert_eq!(apply(&mut runtime, &start, "join", joined).unwrap_err(),
        RuntimeError::PondShape { issue: PondShapeError::TouchesOther });
    assert_eq!(runtime.snapshot(&start.world_id).unwrap(), start);

    let filled = apply(&mut runtime, &start, "fill-second", PondEdit::AddWater {
        basin_id: second.id.clone(), liters: 0.6,
    }).unwrap();
    let adjusted = filled.pond.as_ref().unwrap();
    assert_eq!(adjusted.basins[0], *first);
    assert_eq!(adjusted.basins[1].water_liters, second.water_liters + 0.6);
    let copy = runtime.copy_world(&store, &filled.world_id).unwrap();
    assert_eq!(copy.pond, filled.pond);
    let reopened = WorldRuntime::new().load(&store, &copy.world_id).unwrap().snapshot;
    assert_eq!(reopened.pond, filled.pond);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn empty_r5_pond_topology_survives_history_trials_and_checkpoint_restore() {
    let path = std::env::temp_dir().join(format!("micro-ecosystem-r5-topology-{}", Uuid::new_v4()));
    let store = SaveStore::new(&path);
    let mut runtime = WorldRuntime::new();
    let blank = runtime.create_library_world(&store, "blank-pond-r5-dual", 2729).unwrap();
    let initial = blank.pond.as_ref().unwrap();
    assert!(initial.basins.iter().all(|basin| !basin.wet && basin.scenery.as_ref().unwrap().is_empty()));
    let first = initial.basins[0].id.clone();
    let second = initial.basins[1].id.clone();
    let first_row = initial.basins[0].shape.as_ref().unwrap().protected_row;
    let apply = |runtime: &mut WorldRuntime, from: &Snapshot, command, edit| {
        runtime.edit_pond_command(&from.world_id, command, &from.run_generation, &from.edit_revision, edit)
    };
    let replay = |runtime: &mut WorldRuntime, from: &Snapshot, command, redo| {
        runtime.edit_pond_history_command(&from.world_id, command, &from.run_generation, &from.edit_revision, redo).unwrap()
    };
    let topology = |mode, cells| PondEdit::CommitTopology { basin_id: first.clone(), mode, cells };

    let watered = apply(&mut runtime, &blank, "first-water", PondEdit::AddWater { basin_id: first.clone(), liters: 3.0 }).unwrap();
    let watered = apply(&mut runtime, &watered, "second-water", PondEdit::AddWater { basin_id: second.clone(), liters: 5.0 }).unwrap();
    let stocked = apply(&mut runtime, &watered, "first-detritus", PondEdit::AddDetritus { basin_id: first.clone(), mg_c: 30.0 }).unwrap();
    let stocked = apply(&mut runtime, &stocked, "second-detritus", PondEdit::AddDetritus { basin_id: second.clone(), mg_c: 70.0 }).unwrap();
    let oxygen_mg = |state: &Snapshot| state.pond.as_ref().unwrap().basins.iter()
        .map(|basin| basin.oxygen_mg_per_liter.unwrap_or_default() * basin.water_liters).sum::<f64>();
    let combined = apply(&mut runtime, &stocked, "merge", topology(PondShapeMode::Add, vec![24])).unwrap();
    assert_eq!(apply(&mut runtime, &stocked, "merge", topology(PondShapeMode::Add, vec![24])).unwrap(), combined);
    assert_eq!(combined.edit_revision.parse::<u64>().unwrap(), stocked.edit_revision.parse::<u64>().unwrap() + 1);
    let view = combined.pond.as_ref().unwrap();
    assert_eq!(view.basins.len(), 1);
    assert_eq!(view.basins[0].id, first);
    assert_eq!(view.basins[0].shape.as_ref().unwrap().protected_row, first_row);
    assert_eq!(view.basins[0].water_liters, 8.0);
    assert_eq!(view.basins[0].surface_area_cm2, 4_900.0);
    assert_eq!(view.basins[0].detritus_mg_c, 100.0);
    assert!((oxygen_mg(&combined) - oxygen_mg(&stocked)).abs() < 1e-8);
    let unmerged = replay(&mut runtime, &combined, "undo-merge", false);
    assert_eq!(unmerged.pond, stocked.pond);
    let merged = replay(&mut runtime, &unmerged, "redo-merge", true);
    assert_eq!(merged.pond, combined.pond);
    let combined = merged;
    runtime.save(&store, &combined.world_id).unwrap();

    assert!(apply(&mut runtime, &combined, "break-section", topology(PondShapeMode::Remove, vec![12, 24])).is_err());
    assert_eq!(runtime.snapshot(&combined.world_id).unwrap(), combined);
    let split = apply(&mut runtime, &combined, "split", topology(PondShapeMode::Remove, vec![24])).unwrap();
    let parts = &split.pond.as_ref().unwrap().basins;
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0].id, first);
    assert_eq!(parts[0].shape.as_ref().unwrap().protected_row, first_row);
    assert_eq!(parts.iter().map(|basin| basin.water_liters).collect::<Vec<_>>(), [4.0, 4.0]);
    assert_eq!(parts.iter().map(|basin| basin.detritus_mg_c).collect::<Vec<_>>(), [50.0, 50.0]);
    assert!((oxygen_mg(&split) - oxygen_mg(&stocked)).abs() < 1e-8);
    let prior_split_id = parts[1].id.clone();
    assert_ne!(prior_split_id, second);
    assert_eq!(replay(&mut runtime, &split, "undo-split", false).pond, combined.pond);
    let undone = runtime.snapshot(&combined.world_id).unwrap();
    let replayed = replay(&mut runtime, &undone, "redo-split", true);
    assert_eq!(replayed.pond, split.pond);

    let restored = runtime.load(&store, &combined.world_id).unwrap().snapshot;
    assert_eq!(restored.pond, combined.pond);
    assert_ne!(restored.run_generation, combined.run_generation);
    assert!(matches!(apply(&mut runtime, &combined, "old-world", topology(PondShapeMode::Remove, vec![24])),
        Err(RuntimeError::StaleGeneration { .. })));
    let new_split = apply(&mut runtime, &restored, "split-after-restore", topology(PondShapeMode::Remove, vec![24])).unwrap();
    assert_ne!(new_split.pond.as_ref().unwrap().basins[1].id, prior_split_id);
    let merged_again = replay(&mut runtime, &new_split, "undo-before-trial", false);
    runtime.save(&store, &merged_again.world_id).unwrap();

    let mut cold = WorldRuntime::new();
    let cold_merged = cold.load(&store, &merged_again.world_id).unwrap().snapshot;
    let trial = cold.start_trial(&cold_merged.world_id).unwrap();
    let trial_split = apply(&mut cold, &trial, "split-trial", topology(PondShapeMode::Remove, vec![24])).unwrap();
    assert_eq!(cold.snapshot(&cold_merged.world_id).unwrap(), cold_merged);
    assert_ne!(trial_split.pond.as_ref().unwrap().basins[1].id, new_split.pond.as_ref().unwrap().basins[1].id);
    let adopted = cold.apply_trial(&store, &trial.world_id).unwrap();
    assert_eq!(adopted.pond, trial_split.pond);
    let copy = cold.copy_world(&store, &adopted.world_id).unwrap();
    let reread = WorldRuntime::new().load(&store, &copy.world_id).unwrap().snapshot;
    assert_eq!(reread.pond, adopted.pond);
    let preset = cold.save_custom_preset(&store, &adopted.world_id, "分池版图", 18).unwrap();
    let layout = cold.create_library_world(&store, &preset.preset_id, 21).unwrap();
    let adopted_basins = &adopted.pond.as_ref().unwrap().basins;
    let layout_basins = &layout.pond.as_ref().unwrap().basins;
    assert_eq!(layout_basins.len(), adopted_basins.len());
    for (before, after) in adopted_basins.iter().zip(layout_basins) {
        assert_eq!(after.id, before.id);
        assert_eq!(after.shape, before.shape);
        assert_eq!(after.water_liters, before.water_liters);
        assert_eq!(after.detritus_mg_c, 0.0);
    }
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn r5_static_sections_remain_independent_when_shared_water_merges_and_splits() {
    let path = std::env::temp_dir().join(format!("micro-ecosystem-pond-sections-{}", Uuid::new_v4()));
    let store = SaveStore::new(&path);
    let mut runtime = WorldRuntime::new();
    let start = runtime.create_library_world(&store, "blank-pond-r5-dual", 2729).unwrap();
    let world_id = &start.world_id;
    let a = eco_core::POND_DEFAULT_BASIN_ID.to_owned();
    let b = "pond-2".to_owned();
    let apply = |runtime: &mut WorldRuntime, state: &Snapshot, name: &str, edit| {
        runtime.edit_pond_command(&state.world_id, name, &state.run_generation, &state.edit_revision, edit)
    };
    let topology = |mode, cells| PondEdit::CommitTopology { basin_id: a.clone(), mode, cells };
    let mut setup = start.clone();
    for (name, basin_id, liters) in [("water-a", &a, 5.0), ("water-b", &b, 7.0)] {
        setup = apply(&mut runtime, &setup, name, PondEdit::AddWater { basin_id: basin_id.clone(), liters }).unwrap();
        setup = apply(&mut runtime, &setup, &format!("{name}-stone"), PondEdit::AddStone {
            basin_id: basin_id.clone(), x: 0.65, preset: PondStonePreset::Main,
        }).unwrap();
    }
    setup = apply(&mut runtime, &setup, "section-b-root", PondEdit::AddWood { basin_id: b.clone(), x: 0.28, preset: PondWoodPreset::Root }).unwrap();
    setup = apply(&mut runtime, &setup, "section-a-shrimp", PondEdit::StockShrimp {
        basin_id: a.clone(), position: Point { x: 0.86, y: 0.4 }, count: 1,
    }).unwrap();
    let original = setup.pond.as_ref().unwrap().basins.clone();
    let second_row = original[1].shape.as_ref().unwrap().protected_row;
    let trial = runtime.start_trial(world_id).unwrap();
    let merged = apply(&mut runtime, &trial, "merge-sections", topology(PondShapeMode::Add, vec![29])).unwrap();
    let basins = &merged.pond.as_ref().unwrap().basins;
    assert_eq!(basins.len(), 1);
    let main = &basins[0];
    let second = &main.static_sections[0];
    assert_eq!(main.water_liters, 12.0);
    assert_eq!(main.stones, original[0].stones);
    assert_eq!(main.shrimp[0].id, original[0].shrimp[0].id);
    assert_eq!(second.row, second_row);
    assert_eq!(second.stones, *original[1].stones.as_ref().unwrap());
    assert_eq!(second.woods, *original[1].woods.as_ref().unwrap());
    let inner = |edit| PondEdit::EditStaticSection { basin_id: a.clone(), row: second_row, edit: Box::new(edit) };
    assert!(apply(&mut runtime, &merged, "reject-side-water", inner(PondEdit::AddWater { basin_id: a.clone(), liters: 0.5 })).is_err());
    assert!(apply(&mut runtime, &merged, "reject-section-row-cut", topology(PondShapeMode::Remove, vec![u16::from(second_row) * 12 + 1])).is_err());
    assert_eq!(runtime.snapshot(&trial.world_id).unwrap(), merged);
    let edit = apply(&mut runtime, &merged, "move-side-stone", inner(PondEdit::MoveStone {
        basin_id: a.clone(), id: second.stones[0].id, x: 0.70,
    })).unwrap();
    let edited = &edit.pond.as_ref().unwrap().basins[0];
    assert_eq!(edited.stones, main.stones);
    assert_eq!(edited.water_liters, main.water_liters);
    assert_eq!(edited.static_sections[0].stones[0].x, 0.70);
    let undone = runtime.edit_pond_history_command(&trial.world_id, "undo-side", &edit.run_generation, &edit.edit_revision, false).unwrap();
    assert_eq!(undone.pond, merged.pond);
    let redone = runtime.edit_pond_history_command(&trial.world_id, "redo-side", &undone.run_generation, &undone.edit_revision, true).unwrap();
    assert_eq!(redone.pond, edit.pond);
    assert_eq!(runtime.snapshot(world_id).unwrap(), setup);
    let adopted = runtime.apply_trial(&store, &trial.world_id).unwrap();
    let mut cold = WorldRuntime::new();
    let loaded = cold.load(&store, world_id).unwrap().snapshot;
    assert_eq!(loaded.pond, adopted.pond);
    let preset = cold.save_custom_preset(&store, world_id, "双剖面", 2729).unwrap();
    let seeded = cold.create_library_world(&store, &preset.preset_id, 2729).unwrap();
    assert_eq!(seeded.pond.unwrap().basins[0].static_sections, edited.static_sections);
    let split = apply(&mut cold, &loaded, "split-sections", topology(PondShapeMode::Remove, vec![29])).unwrap();
    let parts = &split.pond.as_ref().unwrap().basins;
    assert_eq!(parts.iter().map(|basin| basin.water_liters).collect::<Vec<_>>(), [6.0, 6.0]);
    assert_eq!(parts[0].stones, original[0].stones);
    assert_eq!(parts[1].shape.as_ref().unwrap().protected_row, second_row);
    assert_eq!(parts[1].stones.as_ref().unwrap()[0].x, 0.70);
    assert_eq!(parts[1].woods, original[1].woods);
    assert!(parts.iter().all(|basin| basin.static_sections.is_empty()));
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn r5_shared_detritus_preserves_deposits_in_both_sections_through_trial_and_load() {
    fn basins(snapshot: &Snapshot) -> &[eco_core::PondBasinObservation] {
        &snapshot.pond.as_ref().unwrap().basins
    }

    let path = std::env::temp_dir().join(format!("micro-ecosystem-pond-section-leaves-{}", Uuid::new_v4()));
    let store = SaveStore::new(&path);
    let mut runtime = WorldRuntime::new();
    let start = runtime.create_library_world(&store, "blank-pond-r5-dual", 2729).unwrap();
    let world_id = &start.world_id;
    let primary = eco_core::POND_DEFAULT_BASIN_ID.to_owned();
    let secondary = "pond-2".to_owned();
    let apply = |runtime: &mut WorldRuntime, state: &Snapshot, name: &str, edit| {
        runtime.edit_pond_command(&state.world_id, name, &state.run_generation, &state.edit_revision, edit).unwrap()
    };
    let history = |runtime: &mut WorldRuntime, state: &Snapshot, name: &str, redo| {
        runtime.edit_pond_history_command(&state.world_id, name, &state.run_generation, &state.edit_revision, redo).unwrap()
    };
    let topology = |mode| PondEdit::CommitTopology { basin_id: primary.clone(), mode, cells: vec![29] };
    let deposit = |basin_id, x, mg_c| PondEdit::AddLeafDeposit {
        basin_id, x, mg_c, preset: PondLeafDepositPreset::Midground,
    };
    let near = |actual: f64, expected: f64| assert!((actual - expected).abs() < 1e-8, "{actual} != {expected}");

    let mut setup = start.clone();
    for (index, (basin_id, liters, x, mg_c)) in [(&primary, 5.0, 0.31, 20.0), (&secondary, 7.0, 0.61, 40.0)].into_iter().enumerate() {
        setup = apply(&mut runtime, &setup, &format!("water-{index}"), PondEdit::AddWater { basin_id: basin_id.clone(), liters });
        setup = apply(&mut runtime, &setup, &format!("leaf-{index}"), deposit(basin_id.clone(), x, mg_c));
    }
    setup = apply(&mut runtime, &setup, "unlocated-detritus", PondEdit::AddDetritus { basin_id: primary.clone(), mg_c: 30.0 });
    let original = basins(&setup);
    let row = original[1].shape.as_ref().unwrap().protected_row;
    let side_edit = |edit| PondEdit::EditStaticSection { basin_id: primary.clone(), row, edit: Box::new(edit) };
    let trial = runtime.start_trial(world_id).unwrap();
    let merged = apply(&mut runtime, &trial, "merge-leaf-sections", topology(PondShapeMode::Add));
    let combined = &basins(&merged)[0];
    let side = &combined.static_sections[0];
    assert_eq!((combined.water_liters, combined.detritus_mg_c, side.row), (12.0, 90.0, row));
    assert_eq!(combined.leaf_deposits, original[0].leaf_deposits);
    assert_eq!(side.leaf_deposits, original[1].leaf_deposits);
    assert!(side.stones.is_empty() && side.woods.is_empty() && side.scenery.is_empty());
    assert_eq!(combined.leaf_deposits.as_ref().unwrap()[0].id, side.leaf_deposits.as_ref().unwrap()[0].id);

    let moved = apply(&mut runtime, &merged, "move-side-leaf", side_edit(PondEdit::MoveLeafDeposit {
        basin_id: primary.clone(), id: 1, x: 0.72,
    }));
    let edited = &basins(&moved)[0];
    assert_eq!(edited.leaf_deposits, combined.leaf_deposits);
    assert_eq!(edited.detritus_mg_c, 90.0);
    let moved_leaf = &edited.static_sections[0].leaf_deposits.as_ref().unwrap()[0];
    assert_eq!((moved_leaf.x, moved_leaf.remaining_mg_c), (0.72, 40.0));
    let undone = history(&mut runtime, &moved, "undo-side-leaf", false);
    assert_eq!(undone.pond, merged.pond);
    let redone = history(&mut runtime, &undone, "redo-side-leaf", true);
    assert_eq!(redone.pond, moved.pond);
    let added = apply(&mut runtime, &redone, "add-side-leaf", side_edit(deposit(primary.clone(), 0.79, 10.0)));
    assert_eq!(basins(&added)[0].detritus_mg_c, 100.0);
    assert_eq!(runtime.pond_ledger(&trial.world_id).unwrap().unwrap().basins[0].matter_input_mg_cnp[0], 100.0);

    let advanced = runtime.advance_trial(&trial.world_id, 120).unwrap();
    let after = &basins(&advanced)[0];
    let after_primary = after.leaf_deposits.as_ref().unwrap()[0].remaining_mg_c;
    let after_side = after.static_sections[0].leaf_deposits.as_ref().unwrap();
    assert!(after.detritus_mg_c < 100.0 && after_primary < 20.0);
    near(after_side[0].remaining_mg_c, after_primary * 2.0);
    near(after_side[1].remaining_mg_c, after_primary * 0.5);
    assert_eq!(runtime.snapshot(world_id).unwrap(), setup);
    let adopted = runtime.apply_trial(&store, &trial.world_id).unwrap();
    let mut cold = WorldRuntime::new();
    let loaded = cold.load(&store, world_id).unwrap().snapshot;
    assert_eq!(loaded.pond, adopted.pond);
    assert_eq!(loaded.pond, advanced.pond);

    let preset = cold.save_custom_preset(&store, world_id, "双剖面沉叶", 2729).unwrap();
    let seeded = cold.create_library_world(&store, &preset.preset_id, 2729).unwrap();
    let seeded_basin = &basins(&seeded)[0];
    assert_eq!(seeded_basin.leaf_deposits.as_ref().unwrap()[0].remaining_mg_c, 20.0);
    assert_eq!(seeded_basin.static_sections[0].leaf_deposits.as_ref().unwrap().iter()
        .map(|leaf| (leaf.x, leaf.remaining_mg_c)).collect::<Vec<_>>(), [(0.72, 40.0), (0.79, 10.0)]);
    assert_eq!(seeded_basin.detritus_mg_c, 70.0);

    let before_split = cold.pond_ledger(world_id).unwrap().unwrap();
    let split = apply(&mut cold, &loaded, "split-leaf-sections", topology(PondShapeMode::Remove));
    let children = basins(&split);
    let side_located = after_side.iter().map(|leaf| leaf.remaining_mg_c).sum::<f64>();
    let unlocated_share = (after.detritus_mg_c - after_primary - side_located) / 2.0;
    assert_eq!(children[0].leaf_deposits, after.leaf_deposits);
    assert_eq!(children[1].shape.as_ref().unwrap().protected_row, row);
    assert_eq!(children[1].leaf_deposits.as_deref().unwrap(), after_side);
    near(children[0].detritus_mg_c, after_primary + unlocated_share);
    near(children[1].detritus_mg_c, side_located + unlocated_share);
    let ledger = cold.pond_ledger(world_id).unwrap().unwrap();
    for element in 0..3 {
        near(ledger.basins.iter().map(|basin| basin.current_matter_mg_cnp[element]).sum(),
            before_split.basins[0].current_matter_mg_cnp[element]);
        near(ledger.basins.iter().map(|basin| basin.matter_topology_transfer_mg_cnp.unwrap_or_default()[element]).sum(), 0.0);
        assert!(ledger.basins.iter().all(|basin| basin.matter_residual_mg_cnp[element].abs() < 1e-8));
    }
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn r5_living_sections_keep_animals_carriers_and_inventory_through_simulation_and_split() {
    use eco_core::{PondMossParent, PondMossPreset};

    fn basins(snapshot: &Snapshot) -> &[eco_core::PondBasinObservation] {
        &snapshot.pond.as_ref().unwrap().basins
    }
    fn secondary_json(snapshot: &Snapshot) -> serde_json::Value {
        serde_json::to_value(&basins(snapshot)[0].static_sections[0]).unwrap()
    }
    fn shrimp_ids(basin: &eco_core::PondBasinObservation) -> Vec<&str> {
        basin.shrimp.iter().map(|shrimp| shrimp.id.as_str()).collect()
    }

    let path = std::env::temp_dir().join(format!("micro-ecosystem-living-sections-{}", Uuid::new_v4()));
    let store = SaveStore::new(&path);
    let mut runtime = WorldRuntime::new();
    let start = runtime.create_library_world(&store, "pond-r5-dual", 2731).unwrap();
    let a = basins(&start)[0].id.clone();
    let b = basins(&start)[1].id.clone();
    let row = basins(&start)[1].shape.as_ref().unwrap().protected_row;
    let edit = |runtime: &mut WorldRuntime, state: &Snapshot, name: &str, action| {
        runtime.edit_pond_command(&state.world_id, name, &state.run_generation, &state.edit_revision, action).unwrap()
    };
    let topology = |mode| PondEdit::CommitTopology { basin_id: a.clone(), mode, cells: vec![29] };
    let local = |action| PondEdit::EditStaticSection { basin_id: a.clone(), row, edit: Box::new(action) };
    let mut ready = start.clone();
    for (index, basin_id) in [&a, &b].into_iter().enumerate() {
        let main_stone = basins(&ready)[index].stones.as_ref().unwrap()[0].id;
        ready = edit(&mut runtime, &ready, &format!("life-film-{index}"), PondEdit::EnableStoneBiofilm { basin_id: basin_id.clone() });
        ready = edit(&mut runtime, &ready, &format!("life-moss-{index}"), PondEdit::AddMoss {
            basin_id: basin_id.clone(), parent: PondMossParent::Stone { id: main_stone }, offset: 0.0, preset: PondMossPreset::Mat,
        });
    }
    let prior = basins(&ready);
    let initial_water = prior.iter().map(|basin| basin.water_liters).sum::<f64>();
    let trial = runtime.start_trial(&start.world_id).unwrap();
    let merged = edit(&mut runtime, &trial, "merge-all-life", topology(PondShapeMode::Add));
    let joined = &basins(&merged)[0];
    let second = secondary_json(&merged);
    assert_eq!(joined.water_liters, initial_water);
    assert_eq!(shrimp_ids(joined), shrimp_ids(&prior[0]));
    assert_eq!(second["shrimp"].as_array().unwrap().iter().map(|shrimp| shrimp["id"].as_str().unwrap()).collect::<Vec<_>>(), shrimp_ids(&prior[1]));
    assert_eq!(second["mossPatches"][0]["stockMgCnp"], serde_json::json!(prior[1].moss_patches.as_ref().unwrap()[0].stock_mg_cnp));
    assert!(second["stoneBiofilm"][0]["stockMgC"].as_f64().unwrap() > 0.0);

    let revised = edit(&mut runtime, &merged, "trim-secondary-moss", local(PondEdit::TrimMoss {
        basin_id: a.clone(), id: prior[1].moss_patches.as_ref().unwrap()[0].id, mg_c: 1.0,
    }));
    assert_eq!(basins(&revised)[0].moss_patches, joined.moss_patches);
    assert!((basins(&revised)[0].detritus_mg_c - joined.detritus_mg_c - 1.0).abs() < 1e-8);
    let stocked = edit(&mut runtime, &revised, "stock-secondary-shrimp", local(PondEdit::StockShrimp {
        basin_id: a.clone(), position: Point { x: 0.86, y: 0.35 }, count: 1,
    }));
    let before_secondary = secondary_json(&stocked);
    assert_eq!(before_secondary["shrimp"].as_array().unwrap().len(), prior[1].shrimp.len() + 1);
    assert_eq!(basins(&stocked)[0].shrimp, joined.shrimp);
    let moved = runtime.advance_trial(&trial.world_id, 32).unwrap();
    let advanced = secondary_json(&moved);
    assert!(advanced["shrimpLife"][0]["ageSeconds"].as_f64().unwrap() > before_secondary["shrimpLife"][0]["ageSeconds"].as_f64().unwrap());
    assert_ne!(advanced["shrimp"], before_secondary["shrimp"]);
    assert_eq!(runtime.snapshot(&start.world_id).unwrap(), ready);
    let adopted = runtime.apply_trial(&store, &trial.world_id).unwrap();
    let mut cold = WorldRuntime::new();
    let loaded = cold.load(&store, &start.world_id).unwrap().snapshot;
    assert_eq!(basins(&loaded)[0].shrimp, basins(&adopted)[0].shrimp);
    assert_eq!(secondary_json(&loaded)["shrimp"], advanced["shrimp"]);
    assert_eq!(secondary_json(&loaded)["stoneBiofilm"], advanced["stoneBiofilm"]);

    let before = cold.pond_ledger(&loaded.world_id).unwrap().unwrap();
    let split = edit(&mut cold, &loaded, "split-all-life", topology(PondShapeMode::Remove));
    let children = basins(&split);
    assert_eq!(children[1].shape.as_ref().unwrap().protected_row, row);
    assert_eq!(shrimp_ids(&children[0]), shrimp_ids(&basins(&loaded)[0]));
    assert_eq!(shrimp_ids(&children[1]), advanced["shrimp"].as_array().unwrap().iter().map(|shrimp| shrimp["id"].as_str().unwrap()).collect::<Vec<_>>());
    assert_eq!(children[1].moss_patches.as_ref().unwrap()[0].stock_mg_cnp, serde_json::from_value::<[f64; 3]>(advanced["mossPatches"][0]["stockMgCnp"].clone()).unwrap());
    let after = cold.pond_ledger(&loaded.world_id).unwrap().unwrap();
    for element in 0..3 {
        let sum = after.basins.iter().map(|basin| basin.current_matter_mg_cnp[element]).sum::<f64>();
        assert!((sum - before.basins[0].current_matter_mg_cnp[element]).abs() < 1e-8);
        assert!(after.basins.iter().all(|basin| basin.matter_residual_mg_cnp[element].abs() < 1e-8));
    }
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn r5_rooted_plants_keep_their_section_through_trial_save_preset_and_split() {
    use eco_core::PondRibbonPlantPreset;

    fn basins(snapshot: &Snapshot) -> &[eco_core::PondBasinObservation] {
        &snapshot.pond.as_ref().unwrap().basins
    }

    let path = std::env::temp_dir().join(format!("micro-ecosystem-pond-section-plants-{}", Uuid::new_v4()));
    let store = SaveStore::new(&path);
    let mut runtime = WorldRuntime::new();
    let start = runtime.create_library_world(&store, "blank-pond-r5-dual", 2730).unwrap();
    let primary = eco_core::POND_DEFAULT_BASIN_ID.to_owned();
    let side = "pond-2".to_owned();
    let edit = |runtime: &mut WorldRuntime, state: &Snapshot, name: &str, action| {
        runtime.edit_pond_command(&state.world_id, name, &state.run_generation, &state.edit_revision, action).unwrap()
    };
    let mut setup = start.clone();
    for (name, id, liters) in [("plants-water-a", &primary, 5.0), ("plants-water-b", &side, 7.0)] {
        setup = edit(&mut runtime, &setup, name, PondEdit::AddWater { basin_id: id.clone(), liters });
    }
    setup = edit(&mut runtime, &setup, "plants-a", PondEdit::AddWaterPlant { basin_id: primary.clone(), x: 0.31 });
    setup = edit(&mut runtime, &setup, "plants-b", PondEdit::PlaceRibbonPlant {
        basin_id: side, x: 0.62, preset: PondRibbonPlantPreset::Large,
    });
    let row = basins(&setup)[1].shape.as_ref().unwrap().protected_row;
    let topology = |mode| PondEdit::CommitTopology { basin_id: primary.clone(), mode, cells: vec![29] };
    let local = |action| PondEdit::EditStaticSection { basin_id: primary.clone(), row, edit: Box::new(action) };
    let trial = runtime.start_trial(&setup.world_id).unwrap();
    let merged = edit(&mut runtime, &trial, "plants-join", topology(PondShapeMode::Add));
    let joined = &basins(&merged)[0];
    assert_eq!(basins(&merged).len(), 1);
    assert_eq!((joined.water_liters, joined.plant_mg_c), (12.0, 125.0));
    let main_before = joined.water_plant_patches.as_ref().unwrap();
    let side_before = joined.static_sections[0].water_plant_patches.as_ref().unwrap();
    assert_eq!((main_before[0].id, side_before[0].id, joined.static_sections[0].row), (1, 1, row));
    assert_eq!((main_before[0].stock_mg_cnp[0], side_before[0].stock_mg_cnp[0]), (50.0, 75.0));

    let moved = edit(&mut runtime, &merged, "plants-move-side", local(PondEdit::MoveWaterPlant {
        basin_id: primary.clone(), id: 1, x: 0.71,
    }));
    let trimmed = edit(&mut runtime, &moved, "plants-trim-side", local(PondEdit::TrimWaterPlant {
        basin_id: primary.clone(), id: 1, mg_c: 15.0,
    }));
    let trimmed_basin = &basins(&trimmed)[0];
    assert_eq!(trimmed_basin.water_plant_patches.as_ref().unwrap(), main_before);
    assert_eq!((trimmed_basin.plant_mg_c, trimmed_basin.detritus_mg_c), (110.0, 15.0));
    let trimmed_side = trimmed_basin.static_sections[0].water_plant_patches.as_ref().unwrap();
    assert_eq!((trimmed_side[0].x, trimmed_side[0].stock_mg_cnp[0]), (0.71, 60.0));
    let removed = edit(&mut runtime, &trimmed, "plants-remove-side", local(PondEdit::RemoveWaterPlant {
        basin_id: primary.clone(), id: 1,
    }));
    assert_eq!((basins(&removed)[0].plant_mg_c, basins(&removed)[0].detritus_mg_c), (50.0, 75.0));
    let restored = runtime.edit_pond_history_command(
        &removed.world_id, "plants-undo-remove", &removed.run_generation, &removed.edit_revision, false,
    ).unwrap();
    assert_eq!(restored.pond, trimmed.pond);
    assert_eq!(runtime.snapshot(&setup.world_id).unwrap(), setup);

    let adopted = runtime.apply_trial(&store, &trial.world_id).unwrap();
    let mut cold = WorldRuntime::new();
    let loaded = cold.load(&store, &setup.world_id).unwrap().snapshot;
    assert_eq!(loaded.pond, adopted.pond);
    let preset = cold.save_custom_preset(&store, &loaded.world_id, "双剖面活体水草", 2730).unwrap();
    let seeded = cold.create_library_world(&store, &preset.preset_id, 2730).unwrap();
    let replanted = &basins(&seeded)[0].static_sections[0].water_plant_patches.as_ref().unwrap()[0];
    assert_eq!((replanted.x, replanted.stock_mg_cnp[0]), (0.71, 75.0));

    let before = cold.pond_ledger(&loaded.world_id).unwrap().unwrap();
    let split = edit(&mut cold, &loaded, "plants-split", topology(PondShapeMode::Remove));
    let children = basins(&split);
    assert_eq!(children.iter().map(|basin| basin.plant_mg_c).collect::<Vec<_>>(), [50.0, 60.0]);
    assert_eq!(children[0].water_plant_patches.as_ref().unwrap(), main_before);
    assert_eq!(children[1].shape.as_ref().unwrap().protected_row, row);
    let promoted = &children[1].water_plant_patches.as_ref().unwrap()[0];
    assert_eq!((promoted.id, promoted.x, promoted.stock_mg_cnp), (1, trimmed_side[0].x, trimmed_side[0].stock_mg_cnp));
    let after = cold.pond_ledger(&loaded.world_id).unwrap().unwrap();
    for element in 0..3 {
        let combined = after.basins.iter().map(|basin| basin.current_matter_mg_cnp[element]).sum::<f64>();
        assert!((combined - before.basins[0].current_matter_mg_cnp[element]).abs() < 1e-8);
        assert!(after.basins.iter().all(|basin| basin.matter_residual_mg_cnp[element].abs() < 1e-8));
    }
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn r5_live_topology_preserves_section_float_and_population_through_adoption() {
    use eco_core::PondFloatingPlantPreset;

    let path = std::env::temp_dir().join(format!("micro-ecosystem-live-pond-{}", Uuid::new_v4()));
    let store = SaveStore::new(&path);
    let mut runtime = WorldRuntime::new();
    let start = runtime.create_library_world(&store, "blank-pond-r5-dual", 2729).unwrap();
    let world_id = &start.world_id;
    let a = eco_core::POND_DEFAULT_BASIN_ID.to_owned();
    let b = "pond-2".to_owned();
    let edit = |runtime: &mut WorldRuntime, state: &Snapshot, command: &str, action| {
        runtime.edit_pond_command(&state.world_id, command, &state.run_generation, &state.edit_revision, action).unwrap()
    };
    let topology = |mode| PondEdit::CommitTopology { basin_id: a.clone(), mode, cells: vec![29] };
    let first_leaf = Point { x: 3.5 / 12.0, y: 1.5 / 12.0 };
    let second_leaf = Point { x: 8.5 / 12.0, y: 4.5 / 12.0 };
    let mut stocked = start.clone();
    for (i, (id, liters, count, position)) in [(&a, 5.0, 5, first_leaf), (&b, 7.0, 2, second_leaf)].into_iter().enumerate() {
        stocked = edit(&mut runtime, &stocked, &format!("live-water-{i}"), PondEdit::AddWater { basin_id: id.clone(), liters });
        stocked = edit(&mut runtime, &stocked, &format!("live-daphnia-{i}"), PondEdit::StockDaphnia { basin_id: id.clone(), count });
        stocked = edit(&mut runtime, &stocked, &format!("live-leaf-{i}"), PondEdit::AddFloatingPlant {
            basin_id: id.clone(), position, preset: PondFloatingPlantPreset::Large,
        });
    }
    stocked = edit(&mut runtime, &stocked, "section-shrimp", PondEdit::StockShrimp {
        basin_id: a.clone(), position: Point { x: 0.76, y: 0.40 }, count: 1,
    });
    stocked = edit(&mut runtime, &stocked, "section-root", PondEdit::AddWaterPlant { basin_id: a.clone(), x: 0.36 });
    stocked = edit(&mut runtime, &stocked, "section-deposit", PondEdit::AddLeafDeposit {
        basin_id: a.clone(), x: 0.48, preset: PondLeafDepositPreset::Midground, mg_c: 20.0,
    });
    let before = runtime.pond_ledger(world_id).unwrap().unwrap();
    let trial = runtime.start_trial(world_id).unwrap();
    let merged = edit(&mut runtime, &trial, "merge-live", topology(PondShapeMode::Add));
    let combined = &merged.pond.as_ref().unwrap().basins[0];
    assert_eq!((combined.water_liters, combined.daphnia_count), (12.0, 7));
    let floats = combined.floating_plants.as_ref().unwrap();
    assert_eq!(floats.iter().map(|patch| patch.position).collect::<Vec<_>>(), [first_leaf, second_leaf]);
    assert!(floats[1].id > floats[0].id);
    let split = edit(&mut runtime, &merged, "split-live", topology(PondShapeMode::Remove));
    let parts = &split.pond.as_ref().unwrap().basins;
    assert_eq!(parts.iter().map(|basin| (basin.water_liters, basin.daphnia_count)).collect::<Vec<_>>(), [(6.0, 4), (6.0, 3)]);
    assert_eq!(parts[0].shrimp[0].id, combined.shrimp[0].id);
    assert!(parts[1].shrimp.is_empty());
    assert_eq!(parts[0].water_plant_patches, combined.water_plant_patches);
    assert_eq!(parts[0].leaf_deposits, combined.leaf_deposits);
    assert_eq!(parts.iter().map(|basin| basin.detritus_mg_c).collect::<Vec<_>>(), [20.0, 0.0]);
    assert_eq!(parts[0].floating_plants.as_deref().unwrap(), &floats[..1]);
    assert_eq!(parts[1].floating_plants.as_deref().unwrap(), &floats[1..]);
    let assert_ledger = |ledger: &PondLedger| {
        let oxygen = |ledger: &PondLedger| ledger.basins.iter().map(|basin| basin.current_oxygen_mg).sum::<f64>();
        assert!((oxygen(ledger) - oxygen(&before)).abs() < 1e-8);
        for element in 0..3 {
            let net_transfer = ledger.basins.iter().map(|b| b.matter_topology_transfer_mg_cnp.unwrap_or_default()[element]).sum::<f64>();
            assert!(net_transfer.abs() < 1e-8);
            for (actual, expected) in [
                (ledger.basins.iter().map(|b| b.current_matter_mg_cnp[element]).sum::<f64>(), before.basins.iter().map(|b| b.current_matter_mg_cnp[element]).sum::<f64>()),
                (ledger.basins.iter().map(|b| b.matter_input_mg_cnp[element]).sum::<f64>(), before.basins.iter().map(|b| b.matter_input_mg_cnp[element]).sum::<f64>()),
                (ledger.basins.iter().map(|b| b.matter_output_mg_cnp[element]).sum::<f64>(), before.basins.iter().map(|b| b.matter_output_mg_cnp[element]).sum::<f64>()),
            ] {
                assert!((actual - expected).abs() < 1e-8);
            }
            assert!(ledger.basins.iter().all(|b| b.matter_residual_mg_cnp[element].abs() < 1e-8));
        }
    };
    let divided = runtime.pond_ledger(&trial.world_id).unwrap().unwrap();
    assert!(divided.basins.iter().all(|basin| basin.matter_topology_transfer_mg_cnp.is_some()));
    assert_ledger(&divided);
    let undo = runtime.edit_pond_history_command(&trial.world_id,"live-undo",&split.run_generation,&split.edit_revision,false).unwrap();
    assert_eq!(undo.pond, merged.pond);
    let redo = runtime.edit_pond_history_command(&trial.world_id,"live-redo",&undo.run_generation,&undo.edit_revision,true).unwrap();
    assert_eq!(redo.pond, split.pond);
    assert_eq!(runtime.snapshot(world_id).unwrap(), stocked);
    let adopted = runtime.apply_trial(&store, &trial.world_id).unwrap();
    assert_eq!(adopted.pond, split.pond);
    let mut reopened = WorldRuntime::new();
    let cold = reopened.load(&store, world_id).unwrap().snapshot;
    assert_eq!(cold.pond, adopted.pond);
    assert_ledger(&reopened.pond_ledger(world_id).unwrap().unwrap());
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn r5_water_plant_history_and_save_preserve_allocation_and_id_floor() {
    let path = std::env::temp_dir().join(format!("micro-ecosystem-r5-water-plant-{}", Uuid::new_v4()));
    let store = SaveStore::new(&path);
    let mut runtime = WorldRuntime::new();
    let started = runtime.create_library_world(&store, "pond-r5-composed", 2729).unwrap();
    let id = &started.world_id;
    let add = |x| PondEdit::AddWaterPlant { basin_id: eco_core::POND_DEFAULT_BASIN_ID.into(), x };
    let edit = |runtime: &mut WorldRuntime, state: &Snapshot, command, action| {
        runtime.edit_pond_command(&state.world_id, command, &state.run_generation, &state.edit_revision, action)
    };
    let history = |runtime: &mut WorldRuntime, state: &Snapshot, command, redo| {
        runtime.edit_pond_history_command(&state.world_id, command, &state.run_generation, &state.edit_revision, redo)
    };
    let view = |snapshot: &Snapshot| snapshot.pond.as_ref().unwrap().basins[0].clone();
    let original = runtime.pond_ledger(id).unwrap().unwrap();
    let planted = edit(&mut runtime, &started, "add", add(0.36)).unwrap();
    assert_eq!(edit(&mut runtime, &started, "add", add(0.36)).unwrap(), planted);
    assert_eq!(view(&planted).water_plant_patches.unwrap()[0].id, 1);
    let planted_ledger = runtime.pond_ledger(id).unwrap().unwrap();
    let undone = history(&mut runtime, &planted, "undo", false).unwrap();
    assert_eq!(view(&undone), view(&started));
    assert_eq!(runtime.pond_ledger(id).unwrap().unwrap(), original);
    let redone = history(&mut runtime, &undone, "redo", true).unwrap();
    assert_eq!(view(&redone), view(&planted));
    assert_eq!(runtime.pond_ledger(id).unwrap().unwrap(), planted_ledger);
    history(&mut runtime, &redone, "undo-again", false).unwrap();
    runtime.save(&store, id).unwrap();
    let mut reopened = WorldRuntime::new();
    let loaded = reopened.load(&store, id).unwrap().snapshot;
    let next = edit(&mut reopened, &loaded, "add", add(0.36)).unwrap();
    assert_eq!(view(&next).water_plant_patches.unwrap()[0].id, 2);

    let trial = reopened.start_trial(id).unwrap();
    let trial_trim = PondEdit::TrimWaterPlant { basin_id: eco_core::POND_DEFAULT_BASIN_ID.into(), id: 2, mg_c: 10.0 };
    let trimmed = edit(&mut reopened, &trial, "trim", trial_trim).unwrap();
    assert_eq!(reopened.snapshot(id).unwrap(), next);
    assert!((view(&trimmed).water_plant_patches.unwrap()[0].stock_mg_cnp[0] - 40.0).abs() < 1e-9);
    let adopted = reopened.apply_trial(&store, &trial.world_id).unwrap();
    assert_eq!(view(&adopted), view(&trimmed));
    let persisted = WorldRuntime::new().load(&store, id).unwrap().snapshot;
    assert_eq!(view(&persisted), view(&adopted));
    let copied = reopened.copy_world(&store, id).unwrap();
    assert_eq!(view(&copied), view(&adopted));
    let blueprint = reopened.save_custom_preset(&store, id, "水草布局", 2729).unwrap();
    let fresh = reopened.create_library_world(&store, &blueprint.preset_id, 2729).unwrap();
    let fresh = view(&fresh).water_plant_patches.unwrap()[0];
    assert_eq!(fresh.id, 2);
    assert_eq!(fresh.x, 0.36);
    assert!(fresh.stock_mg_cnp.into_iter().zip(fresh.initial_mg_cnp).all(|(actual, expected)| (actual - expected).abs() < 1e-9));
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn r5_floating_canopy_reopens_and_trial_changes_remain_basin_local() {
    use eco_core::PondFloatingPlantPreset;

    let path = std::env::temp_dir().join(format!("micro-ecosystem-r5-floating-{}", Uuid::new_v4()));
    let store = SaveStore::new(&path);
    let mut runtime = WorldRuntime::new();
    let started = runtime.create_library_world(&store, "blank-pond-r5-dual", 2729).unwrap();
    let world_id = &started.world_id;
    let basin_id = || eco_core::POND_DEFAULT_BASIN_ID.to_owned();
    let view = |snapshot: &Snapshot| snapshot.pond.as_ref().unwrap().basins[0].clone();
    let row = f64::from(view(&started).shape.unwrap().protected_row);
    let left = Point { x: 3.5 / 12.0, y: (row + 0.5) / 12.0 };
    let right = Point { x: 8.5 / 12.0, ..left };
    let edit = |runtime: &mut WorldRuntime, state: &Snapshot, command, action| {
        runtime.edit_pond_command(&state.world_id, command, &state.run_generation, &state.edit_revision, action)
    };
    let leaf = || PondEdit::AddFloatingPlant { basin_id: basin_id(), position: left, preset: PondFloatingPlantPreset::Large };
    let wet = edit(&mut runtime, &started, "water-floats", PondEdit::AddWater { basin_id: basin_id(), liters: 18.0 }).unwrap();
    let one = edit(&mut runtime, &wet, "first-float", leaf()).unwrap();
    let two = edit(&mut runtime, &one, "second-float", leaf()).unwrap();
    assert_eq!(edit(&mut runtime, &one, "second-float", leaf()).unwrap(), two);
    assert_eq!(view(&one).floating_canopy, view(&two).floating_canopy);
    let second = view(&two).floating_plants.unwrap()[1].id;
    let moved = edit(&mut runtime, &two, "separate-floats", PondEdit::MoveFloatingPlant {
        basin_id: basin_id(), id: second, position: right,
    }).unwrap();
    let coverage = view(&moved).floating_canopy.unwrap();
    assert!(coverage.covered_area_cm2 > view(&two).floating_canopy.unwrap().covered_area_cm2);
    assert!((coverage.light_transmission + coverage.coverage - 1.0).abs() < 1e-12);
    assert_eq!(moved.pond.as_ref().unwrap().basins[1], started.pond.as_ref().unwrap().basins[1]);

    let trial = runtime.start_trial(world_id).unwrap();
    let dried = edit(&mut runtime, &trial, "dry-floats", PondEdit::DrainWater {
        basin_id: basin_id(), liters: view(&trial).water_liters,
    }).unwrap();
    assert_eq!(view(&dried).floating_canopy.unwrap().coverage, 0.0);
    assert!(view(&dried).floating_plants.unwrap().iter().all(|plant| !plant.floating));
    let rewetted = edit(&mut runtime, &dried, "reflood-floats", PondEdit::AddWater {
        basin_id: basin_id(), liters: view(&trial).water_liters,
    }).unwrap();
    assert_eq!(view(&rewetted).floating_canopy, view(&trial).floating_canopy);
    let advanced = runtime.advance_trial(&trial.world_id, 600).unwrap();
    let progressed = view(&advanced).floating_plants.unwrap();
    assert_ne!(progressed[1].stock_mg_cnp, view(&rewetted).floating_plants.unwrap()[1].stock_mg_cnp);
    assert_eq!(runtime.snapshot(world_id).unwrap(), moved);
    let adopted = runtime.apply_trial(&store, &trial.world_id).unwrap();
    assert_eq!(adopted.pond, advanced.pond);

    let mut cold = WorldRuntime::new();
    let loaded = cold.load(&store, world_id).unwrap().snapshot;
    assert_eq!(loaded.pond, adopted.pond);
    let persisted = cold.world(world_id).unwrap().saved.clone();
    let removed = edit(&mut cold, &loaded, "remove-float", PondEdit::RemoveFloatingPlant { basin_id: basin_id(), id: second }).unwrap();
    let delta = view(&removed).detritus_mg_c - view(&loaded).detritus_mg_c;
    assert!((delta - progressed[1].stock_mg_cnp[0]).abs() < 1e-8);
    let undone = cold.edit_pond_history_command(world_id, "undo-float", &removed.run_generation, &removed.edit_revision, false).unwrap();
    assert_eq!(undone.pond, loaded.pond);
    assert_eq!(cold.world(world_id).unwrap().saved.state, persisted.state);
    let blueprint = cold.save_custom_preset(&store, world_id, "双浮叶方案", 2729).unwrap();
    let reset = cold.create_library_world(&store, &blueprint.preset_id, 2729).unwrap();
    let plants = view(&reset).floating_plants.unwrap();
    assert_eq!(plants[1].position, right);
    assert_eq!(plants[1].preset, PondFloatingPlantPreset::Large);
    assert_eq!(plants[1].stock_mg_cnp, plants[1].initial_mg_cnp);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn r5_ribbon_plant_reopens_with_its_live_stock_and_reseeds_a_preset() {
    use eco_core::PondRibbonPlantPreset;

    let path = std::env::temp_dir().join(format!("micro-ecosystem-r5-ribbon-{}", Uuid::new_v4()));
    let store = SaveStore::new(&path);
    let mut runtime = WorldRuntime::new();
    let started = runtime.create_library_world(&store, "blank-pond-r5-dual", 2729).unwrap();
    let id = &started.world_id;
    let basin_id = || eco_core::POND_DEFAULT_BASIN_ID.to_owned();
    let edit = |runtime: &mut WorldRuntime, state: &Snapshot, command, action| {
        runtime.edit_pond_command(&state.world_id, command, &state.run_generation, &state.edit_revision, action)
    };
    let plants = |snapshot: &Snapshot| snapshot.pond.as_ref().unwrap().basins[0].water_plant_patches.clone().unwrap();

    let wet = edit(&mut runtime, &started, "water", PondEdit::AddWater { basin_id: basin_id(), liters: 18.0 }).unwrap();
    let whorled = edit(&mut runtime, &wet, "whorled", PondEdit::AddWaterPlant { basin_id: basin_id(), x: 0.30 }).unwrap();
    let ribbon = PondEdit::PlaceRibbonPlant { basin_id: basin_id(), x: 0.62, preset: PondRibbonPlantPreset::Large };
    let planted = edit(&mut runtime, &whorled, "ribbon", ribbon.clone()).unwrap();
    assert_eq!(edit(&mut runtime, &whorled, "ribbon", ribbon).unwrap(), planted);
    assert_eq!(plants(&planted)[0], plants(&whorled)[0]);
    let individual = plants(&planted)[1];
    assert_eq!(individual.ribbon_preset, Some(PondRibbonPlantPreset::Large));
    assert_eq!(individual.stock_mg_cnp[0], 75.0);
    let neighbor = planted.pond.as_ref().unwrap().basins[1].clone();
    let moved = edit(&mut runtime, &planted, "move", PondEdit::MoveWaterPlant {
        basin_id: basin_id(), id: individual.id, x: 0.70,
    }).unwrap();
    assert_eq!(moved.pond.as_ref().unwrap().basins[1], neighbor);
    assert_eq!(plants(&moved)[1].stock_mg_cnp, individual.stock_mg_cnp);
    let reverted = runtime.edit_pond_history_command(id, "undo-move", &moved.run_generation, &moved.edit_revision, false).unwrap();
    assert_eq!(reverted.pond, planted.pond);

    let trial = runtime.start_trial(id).unwrap();
    let trimmed = edit(&mut runtime, &trial, "trim", PondEdit::TrimWaterPlant {
        basin_id: basin_id(), id: individual.id, mg_c: 12.0,
    }).unwrap();
    assert!((plants(&trimmed)[1].stock_mg_cnp[0] - 63.0).abs() < 1e-8);
    let advanced = runtime.advance_trial(&trial.world_id, 600).unwrap();
    assert_ne!(plants(&advanced)[1].stock_mg_cnp, plants(&trimmed)[1].stock_mg_cnp);
    assert_eq!(runtime.snapshot(id).unwrap(), reverted);
    assert_eq!(runtime.apply_trial(&store, &trial.world_id).unwrap().pond, advanced.pond);
    let settled = runtime.world(id).unwrap().saved.clone();

    let mut cold = WorldRuntime::new();
    let loaded = cold.load(&store, id).unwrap().snapshot;
    assert_eq!(cold.world(id).unwrap().saved, settled);
    assert_eq!(loaded.pond, advanced.pond);
    for branch in [&mut runtime, &mut cold] {
        branch.set_paused(id, false).unwrap();
        branch.set_edit_paused(id, false).unwrap();
    }
    let replay = cold.advance_ticks(id, 100).unwrap();
    assert_ne!(replay.simulation_tick, loaded.simulation_tick);
    assert_eq!(runtime.advance_ticks(id, 100).unwrap().pond, replay.pond);
    let blueprint = cold.save_custom_preset(&store, id, "带状叶水草布局", 2729).unwrap();
    let fresh = cold.create_library_world(&store, &blueprint.preset_id, 2729).unwrap();
    let starter = plants(&fresh)[1];
    assert_eq!(starter.id, individual.id);
    assert_eq!(starter.x, individual.x);
    assert_eq!(starter.ribbon_preset, individual.ribbon_preset);
    assert_eq!(starter.initial_mg_cnp, individual.initial_mg_cnp);
    for (stock, initial) in starter.stock_mg_cnp.into_iter().zip(starter.initial_mg_cnp) {
        assert!((stock - initial).abs() < 1e-8);
    }
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn r5_pond_wood_keeps_real_attachments_through_history_trial_and_checkpoint() {
    let path = std::env::temp_dir().join(format!("micro-ecosystem-r5-pond-wood-{}", Uuid::new_v4()));
    let store = SaveStore::new(&path);
    let mut runtime = WorldRuntime::new();
    let old = runtime.create_library_world(&store, "pond-r5-stones", 2729).unwrap();
    let start = runtime.create_library_world(&store, "pond-r5-wood", 2729).unwrap();
    let id = &start.world_id;
    let basin_id = eco_core::POND_DEFAULT_BASIN_ID;
    let add = PondEdit::AddWood { basin_id: basin_id.to_owned(), x: 0.50, preset: PondWoodPreset::Branch };
    let edit = |runtime: &mut WorldRuntime, from: &Snapshot, command_id: &str, action| {
        runtime.edit_pond_command(&from.world_id, command_id, &from.run_generation, &from.edit_revision, action)
    };
    let undo = |runtime: &mut WorldRuntime, from: &Snapshot, command_id| {
        runtime.edit_pond_history_command(&from.world_id, command_id, &from.run_generation, &from.edit_revision, false)
    };
    let basin = |snapshot: &Snapshot| snapshot.pond.as_ref().unwrap().basins[0].clone();
    assert!(id.starts_with("pond-") && start.season.is_some());
    assert!(basin(&old).woods.is_none());
    assert_eq!(basin(&start).woods, Some(Vec::new()));
    assert!(edit(&mut runtime, &old, "legacy-wood", add.clone()).is_err());

    let ledger = runtime.pond_ledger(id).unwrap().unwrap();
    let placed = edit(&mut runtime, &start, "place", add.clone()).unwrap();
    assert_eq!(edit(&mut runtime, &start, "place", add).unwrap(), placed);
    assert_eq!(runtime.pond_ledger(id).unwrap().unwrap(), ledger);
    let wood = basin(&placed).woods.unwrap()[0];
    assert_eq!(basin(&placed).shrimp, basin(&start).shrimp);
    runtime.set_daylight_target(id, 0.22).unwrap();
    let trial = runtime.start_trial(id).unwrap();
    let resting = (0..300).find_map(|_| {
        let shot = runtime.advance_trial(&trial.world_id, 20).unwrap();
        basin(&shot).shrimp_attachments.is_some_and(|riders| !riders.is_empty()).then_some(shot)
    }).expect("the deterministic trial must reach a genuine wood rest");
    let local_ledger = runtime.pond_ledger(&trial.world_id).unwrap().unwrap();
    let resting_basin = basin(&resting);
    let riders = resting_basin.shrimp_attachments.as_ref().unwrap();
    assert!(riders.iter().all(|attachment| attachment.wood_id == wood.id));
    let moved = [0.52, 0.48].into_iter().enumerate().find_map(|(index, x)| {
        let command = format!("move-{index}");
        edit(&mut runtime, &resting, &command, PondEdit::MoveWood { basin_id: basin_id.to_owned(), id: wood.id, x }).ok()
    }).expect("one nearby site must accept the parent and its resting shrimp");
    let moved_basin = basin(&moved);
    let displacement = moved_basin.woods.as_ref().unwrap()[0].x - wood.x;
    assert_eq!(moved_basin.shrimp_attachments, resting_basin.shrimp_attachments);
    for (before, after) in resting_basin.shrimp.iter().zip(&moved_basin.shrimp) {
        if riders.iter().any(|entry| entry.shrimp_id == before.id) {
            assert!((after.x - before.x - displacement).abs() < 1e-12);
            assert_eq!(after.y, before.y);
        } else {
            assert_eq!(after, before);
        }
    }
    assert_eq!(runtime.pond_ledger(&trial.world_id).unwrap().unwrap(), local_ledger);
    let remove = PondEdit::RemoveWood { basin_id: basin_id.to_owned(), id: wood.id };
    let released = edit(&mut runtime, &moved, "remove", remove).unwrap();
    let released_basin = basin(&released);
    assert_eq!(released_basin.shrimp_attachments, Some(Vec::new()));
    assert_eq!(released_basin.shrimp.len(), moved_basin.shrimp.len());
    for (rest, free) in moved_basin.shrimp.iter().zip(&released_basin.shrimp) {
        assert_eq!((&rest.id, rest.x, rest.y), (&free.id, free.x, free.y));
    }
    assert_eq!(runtime.pond_ledger(&trial.world_id).unwrap().unwrap(), local_ledger);
    assert_eq!(runtime.snapshot(id).unwrap(), placed);
    let restored = undo(&mut runtime, &released, "undo-remove").unwrap();
    assert_eq!(basin(&restored), moved_basin);

    let adopted = runtime.apply_trial(&store, &trial.world_id).unwrap();
    assert_eq!(basin(&adopted), moved_basin);
    assert_eq!(runtime.world(id).unwrap().saved.state.daylight_target(), Some(0.22));
    assert_eq!(runtime.pond_ledger(id).unwrap().unwrap(), local_ledger);
    let reopened = WorldRuntime::new().load(&store, id).unwrap().snapshot;
    assert_eq!(basin(&reopened), moved_basin);
    assert_ne!(reopened.run_generation, adopted.run_generation);
    let copied = runtime.copy_world(&store, id).unwrap();
    assert_eq!(basin(&copied), moved_basin);
    let preset = runtime.save_custom_preset(&store, id, "沉木布置回归", 2729).unwrap();
    assert_eq!(preset.theme, WorldTheme::Pond);
    let fresh = runtime.create_library_world(&store, &preset.preset_id, 2729).unwrap();
    assert_eq!(basin(&fresh).woods, moved_basin.woods);
    assert_eq!(basin(&fresh).shrimp_attachments, Some(Vec::new()));
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn r5_pond_stones_keep_world_and_basin_state_separate_through_edits_and_restart() {
    let path = std::env::temp_dir().join(format!("micro-ecosystem-r5-pond-stones-{}", Uuid::new_v4()));
    let store = SaveStore::new(&path);
    let mut runtime = WorldRuntime::new();
    let old = runtime.create_library_world(&store, "pond", 2729).unwrap();
    let start = runtime.create_library_world(&store, "pond-r5-stones", 2729).unwrap();
    let id = &start.world_id;
    let basin_id = eco_core::POND_DEFAULT_BASIN_ID;
    let main = PondEdit::AddStone { basin_id: basin_id.to_owned(), x: 0.37, preset: PondStonePreset::Main };
    let pebble = PondEdit::AddStone { basin_id: basin_id.to_owned(), x: 0.65, preset: PondStonePreset::Pebble };
    let edit = |runtime: &mut WorldRuntime, from: &Snapshot, command_id, action| {
        runtime.edit_pond_command(&from.world_id, command_id, &from.run_generation, &from.edit_revision, action)
    };
    let history = |runtime: &mut WorldRuntime, from: &Snapshot, command_id, redo| {
        runtime.edit_pond_history_command(&from.world_id, command_id, &from.run_generation, &from.edit_revision, redo)
    };
    let stones = |snapshot: &Snapshot| snapshot.pond.as_ref().unwrap().basins[0].stones.clone();

    assert!(id.starts_with("pond-"));
    assert!(start.season.is_some());
    assert_eq!(stones(&start), Some(Vec::new()));
    assert_eq!(stones(&old), None);
    assert!(edit(&mut runtime, &old, "legacy-stone", main.clone()).is_err());
    let legacy_trial = runtime.start_trial(&old.world_id).unwrap();
    assert_eq!(edit(&mut runtime, &legacy_trial, "legacy-trial-water", PondEdit::AddWater { basin_id: basin_id.to_owned(), liters: 0.5 }),
        Err(RuntimeError::TrialOperationInvalid));
    runtime.discard_trial(&legacy_trial.world_id).unwrap();
    let ledger = runtime.pond_ledger(id).unwrap().unwrap();
    let first = edit(&mut runtime, &start, "main", main.clone()).unwrap();
    assert_eq!(edit(&mut runtime, &start, "main", main).unwrap(), first);
    let pair = edit(&mut runtime, &first, "pebble", pebble.clone()).unwrap();
    assert_eq!(runtime.pond_ledger(id).unwrap().unwrap(), ledger);
    let pairs = stones(&pair).unwrap();
    assert_eq!(pairs.len(), 2);
    assert_eq!(pair.pond.as_ref().unwrap().basins[0].shrimp, start.pond.as_ref().unwrap().basins[0].shrimp);
    let invalid = PondEdit::MoveStone { basin_id: basin_id.to_owned(), id: pairs[1].id, x: 0.37 };
    assert!(edit(&mut runtime, &pair, "overlap", invalid).is_err());
    assert_eq!(pair, runtime.snapshot(id).unwrap());
    assert!(edit(&mut runtime, &pair, "unknown-basin", PondEdit::RemoveStone { basin_id: "other".into(), id: pairs[0].id }).is_err());

    runtime.set_daylight_target(id, 0.81).unwrap();
    let undone = history(&mut runtime, &pair, "undo-pebble", false).unwrap();
    assert_eq!(stones(&undone), stones(&first));
    assert_eq!(runtime.world(id).unwrap().saved.state.daylight_target(), Some(0.81));
    let redone = history(&mut runtime, &undone, "redo-pebble", true).unwrap();
    assert_eq!(stones(&redone), Some(pairs.clone()));
    let removed_again = history(&mut runtime, &redone, "undo-again", false).unwrap();
    let readded = edit(&mut runtime, &removed_again, "new-pebble", pebble).unwrap();
    assert!(stones(&readded).unwrap()[1].id > pairs[1].id);
    assert!(!readded.pond_redo_available);
    let fixed = stones(&readded).unwrap();

    let trial = runtime.start_trial(id).unwrap();
    let shallow = edit(&mut runtime, &trial, "part-drain", PondEdit::DrainWater { basin_id: basin_id.to_owned(), liters: 17.9 }).unwrap();
    assert!(shallow.pond.as_ref().unwrap().basins[0].wet);
    assert_eq!(stones(&shallow), Some(fixed.clone()));
    let remaining = shallow.pond.as_ref().unwrap().basins[0].water_liters;
    let dry = edit(&mut runtime, &shallow, "full-drain", PondEdit::DrainWater { basin_id: basin_id.to_owned(), liters: remaining }).unwrap();
    assert!(!dry.pond.as_ref().unwrap().basins[0].wet);
    assert!(dry.pond.as_ref().unwrap().basins[0].shrimp.is_empty());
    assert_eq!(stones(&dry), Some(fixed.clone()));
    let wet_again = edit(&mut runtime, &dry, "refill", PondEdit::AddWater { basin_id: basin_id.to_owned(), liters: 18.0 }).unwrap();
    assert!(wet_again.pond.as_ref().unwrap().basins[0].wet);
    assert!(wet_again.pond.as_ref().unwrap().basins[0].shrimp.is_empty());
    assert_eq!(stones(&wet_again), Some(fixed));
    assert_eq!(runtime.snapshot(id).unwrap(), readded);
    let adopted = runtime.apply_trial(&store, &trial.world_id).unwrap();
    assert_eq!(adopted.pond, wet_again.pond);
    assert_eq!(runtime.world(id).unwrap().saved.state.daylight_target(), Some(0.81));
    let reopened = WorldRuntime::new().load(&store, id).unwrap().snapshot;
    assert_eq!(reopened.pond, adopted.pond);
    assert_ne!(reopened.run_generation, adopted.run_generation);
    let copied = runtime.copy_world(&store, id).unwrap();
    assert_eq!(copied.pond, adopted.pond);
    let preset = runtime.save_custom_preset(&store, id, "浅塘底石回归", 2729).unwrap();
    assert_eq!(preset.theme, WorldTheme::Pond);
    let fresh = runtime.create_library_world(&store, &preset.preset_id, 2729).unwrap();
    assert_eq!(stones(&fresh), stones(&adopted));
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn r5_litter_history_and_saves_preserve_remaining_stock_and_external_flows() {
    let path = std::env::temp_dir().join(format!("micro-ecosystem-r5-litter-{}", Uuid::new_v4()));
    let store = SaveStore::new(&path);
    let mut runtime = WorldRuntime::new();
    let old = runtime.create_library_world(&store, "forest-r5-bark", 1206).unwrap();
    let start = runtime.create_library_world(&store, "forest-r5-litter", 1206).unwrap();
    let id = &start.world_id;
    let edit = |runtime: &mut WorldRuntime, previous: &Snapshot, command_id, action| {
        runtime.edit_forest_rock_command(&previous.world_id, command_id, &previous.run_generation, &previous.edit_revision, action)
    };
    let change = |edit| ForestRockCommand::Change { edit };
    let initial = runtime.forest_ledger(id).unwrap().unwrap();
    let add = change(ForestRockEdit::AddLeafPatch { position: Point { x: 0.38, y: 0.80 }, preset: ForestLeafPreset::Leaves });
    assert!(old.forest_rocks.as_ref().unwrap().leaf_patches.is_none());
    assert!(edit(&mut runtime, &old, "old-add", add.clone()).is_err());
    let added = edit(&mut runtime, &start, "add", add.clone()).unwrap();
    assert_eq!(edit(&mut runtime, &start, "add", add.clone()).unwrap(), added);
    let patch_id = added.forest_rocks.as_ref().unwrap().leaf_patches.as_ref().unwrap()[0].patch.id;
    let added_ledger = runtime.forest_ledger(id).unwrap().unwrap();
    assert!((added_ledger.matter_input_mg_cnp[0] - initial.matter_input_mg_cnp[0] - 24.0).abs() < 1e-9);
    assert_eq!(added_ledger.matter_output_mg_cnp, initial.matter_output_mg_cnp);
    assert_eq!(added.forest.as_ref().unwrap().walkable_cells, start.forest.as_ref().unwrap().walkable_cells);
    runtime.set_daylight_target(id, 0.42).unwrap();
    let undone = edit(&mut runtime, &added, "undo-add", ForestRockCommand::Undo).unwrap();
    assert!(undone.forest_rocks.as_ref().unwrap().leaf_patches.as_ref().unwrap().is_empty());
    assert_eq!(runtime.forest_ledger(id).unwrap().unwrap(), initial);
    assert_eq!(runtime.world(id).unwrap().saved.state.daylight_target(), Some(0.42));
    let redone = edit(&mut runtime, &undone, "redo-add", ForestRockCommand::Redo).unwrap();
    assert_eq!(redone.forest_rocks.as_ref().unwrap().leaf_patches, added.forest_rocks.as_ref().unwrap().leaf_patches);
    assert_eq!(runtime.forest_ledger(id).unwrap().unwrap(), added_ledger);

    let trial = runtime.start_trial(id).unwrap();
    let advanced = runtime.advance_trial(&trial.world_id, 1200).unwrap();
    let stock = advanced.forest_rocks.as_ref().unwrap().leaf_patches.as_ref().unwrap()[0].patch.remaining_mg_cnp;
    assert!(stock[0] > 0.0 && stock[0] < 24.0);
    let before_move = runtime.forest_ledger(&trial.world_id).unwrap().unwrap();
    let moved = edit(&mut runtime, &advanced, "move", change(ForestRockEdit::Move {
        id: patch_id, position: Point { x: 0.63, y: 0.87 },
    })).unwrap();
    assert_eq!(moved.forest_rocks.as_ref().unwrap().leaf_patches.as_ref().unwrap()[0].patch.remaining_mg_cnp, stock);
    assert_eq!(runtime.forest_ledger(&trial.world_id).unwrap().unwrap(), before_move);
    let removed = edit(&mut runtime, &moved, "remove", change(ForestRockEdit::Remove { id: patch_id })).unwrap();
    let after_remove = runtime.forest_ledger(&trial.world_id).unwrap().unwrap();
    for (index, mass) in stock.into_iter().enumerate() {
        assert!((after_remove.matter_output_mg_cnp[index] - before_move.matter_output_mg_cnp[index] - mass).abs() < 1e-9);
        assert!(after_remove.matter_residual_mg_cnp[index].abs() < 1e-6);
    }
    assert_eq!(runtime.snapshot(id).unwrap(), redone);
    let restored = edit(&mut runtime, &removed, "undo-remove", ForestRockCommand::Undo).unwrap();
    assert_eq!(restored.forest_rocks.as_ref().unwrap().leaf_patches, moved.forest_rocks.as_ref().unwrap().leaf_patches);
    assert_eq!(runtime.forest_ledger(&trial.world_id).unwrap().unwrap(), before_move);
    let adopted = runtime.apply_trial(&store, &trial.world_id).unwrap();
    assert_eq!(adopted.simulation_tick, advanced.simulation_tick);
    assert_eq!(adopted.forest_rocks.as_ref().unwrap().leaf_patches, restored.forest_rocks.as_ref().unwrap().leaf_patches);
    let copy = runtime.copy_world(&store, id).unwrap();
    assert_eq!(copy.forest_rocks.unwrap().leaf_patches, adopted.forest_rocks.as_ref().unwrap().leaf_patches);
    let preset = runtime.save_custom_preset(&store, id, "落叶布置回归", 1206).unwrap();
    let fresh = runtime.create_library_world(&store, &preset.preset_id, 1206).unwrap();
    let fresh_patch = &fresh.forest_rocks.as_ref().unwrap().leaf_patches.as_ref().unwrap()[0];
    let adopted_patch = &adopted.forest_rocks.as_ref().unwrap().leaf_patches.as_ref().unwrap()[0];
    assert_eq!(fresh.simulation_tick, "0");
    assert_eq!(fresh_patch.patch.position, adopted_patch.patch.position);
    assert_eq!(fresh_patch.patch.remaining_mg_cnp, fresh_patch.initial_mg_cnp);
    assert_eq!(runtime.forest_ledger(&fresh.world_id).unwrap().unwrap().matter_input_mg_cnp, [0.0; 3]);
    let persisted = WorldRuntime::new().load(&store, id).unwrap().snapshot;
    assert_eq!(persisted.forest_rocks.unwrap().leaf_patches, adopted.forest_rocks.unwrap().leaf_patches);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn r5_leaf_brush_is_one_replayable_edit_with_durable_distinct_stocks() {
    let path = std::env::temp_dir().join(format!("micro-ecosystem-r5-leaf-brush-{}", Uuid::new_v4()));
    let store = SaveStore::new(&path);
    let mut runtime = WorldRuntime::new();
    let start = runtime.create_library_world(&store, "forest-r5-composed", 1206).unwrap();
    let id = &start.world_id;
    let edit = |runtime: &mut WorldRuntime, previous: &Snapshot, command_id, action| {
        runtime.edit_forest_rock_command(id, command_id, &previous.run_generation, &previous.edit_revision, action)
    };
    let preset = ForestLeafPreset::Leaves;
    let paint = |positions| ForestRockCommand::Change { edit: ForestRockEdit::PaintLeafPatches { preset, positions } };
    let positions = vec![Point { x: 0.38, y: 0.62 }, Point { x: 0.50, y: 0.75 }, Point { x: 0.73, y: 0.88 }];
    let stocks = |snapshot: &Snapshot| snapshot.forest_rocks.as_ref().unwrap().leaf_patches.as_ref().unwrap().clone();
    let initial_stocks = stocks(&start);
    assert_eq!(initial_stocks.len(), 1);
    assert_eq!(initial_stocks[0].patch.remaining_mg_cnp[0], 60.0);
    let initial_ledger = runtime.forest_ledger(id).unwrap().unwrap();
    let occupied = paint(vec![positions[0], initial_stocks[0].patch.position]);
    let rejected = Err(RuntimeError::from(CoreError::OverlappingLeafPatchSite));
    assert_eq!(edit(&mut runtime, &start, "blocked", occupied.clone()), rejected);
    assert_eq!(edit(&mut runtime, &start, "blocked", occupied), rejected);
    assert_eq!(runtime.snapshot(id).unwrap(), start);
    assert_eq!(runtime.forest_ledger(id).unwrap().unwrap(), initial_ledger);

    let command = paint(positions.clone());
    let added = edit(&mut runtime, &start, "paint", command.clone()).unwrap();
    assert_eq!(added.edit_revision, "1");
    assert_eq!(edit(&mut runtime, &start, "paint", command.clone()).unwrap(), added);
    assert_eq!(edit(&mut runtime, &start, "paint", paint(vec![positions[0]])), Err(RuntimeError::CommandReuse));
    let added_stocks = stocks(&added);
    assert_eq!(added_stocks[0], initial_stocks[0]);
    assert_eq!(added_stocks[1..].iter().map(|item| item.patch.id).collect::<Vec<_>>(), [4, 5, 6]);
    assert_eq!(added_stocks[1..].iter().map(|item| item.patch.position).collect::<Vec<_>>(), positions);
    let added_ledger = runtime.forest_ledger(id).unwrap().unwrap();
    assert_eq!(added_ledger.matter_input_mg_cnp, preset.initial_mg_cnp().map(|mass| mass * 3.0));
    assert_eq!(added_ledger.matter_output_mg_cnp, initial_ledger.matter_output_mg_cnp);
    assert!(added_ledger.matter_residual_mg_cnp.into_iter().all(|mass| mass.abs() < 1e-6));

    let undone = edit(&mut runtime, &added, "undo", ForestRockCommand::Undo).unwrap();
    assert_eq!(stocks(&undone), initial_stocks);
    assert_eq!(runtime.forest_ledger(id).unwrap().unwrap(), initial_ledger);
    let redone = edit(&mut runtime, &undone, "redo", ForestRockCommand::Redo).unwrap();
    assert_eq!(stocks(&redone), added_stocks);
    assert_eq!(runtime.forest_ledger(id).unwrap().unwrap(), added_ledger);
    let undone_again = edit(&mut runtime, &redone, "undo-again", ForestRockCommand::Undo).unwrap();
    let repainted = edit(&mut runtime, &undone_again, "repaint", command).unwrap();
    assert_eq!(stocks(&repainted)[1..].iter().map(|item| item.patch.id).collect::<Vec<_>>(), [7, 8, 9]);

    let trial = runtime.start_trial(id).unwrap();
    let advanced = runtime.advance_trial(&trial.world_id, 1200).unwrap();
    let consumed_stocks = stocks(&advanced);
    assert!(consumed_stocks[1..].iter().any(|item| item.patch.remaining_mg_cnp[0] < item.initial_mg_cnp[0]));
    assert_eq!(runtime.snapshot(id).unwrap(), repainted);
    let adopted = runtime.apply_trial(&store, &trial.world_id).unwrap();
    assert_eq!(stocks(&adopted), consumed_stocks);

    let persisted = WorldRuntime::new().load(&store, id).unwrap().snapshot;
    assert_eq!(stocks(&persisted), consumed_stocks);
    assert_eq!(persisted.simulation_tick, adopted.simulation_tick);
    let copy = runtime.copy_world(&store, id).unwrap();
    assert_eq!(stocks(&copy), consumed_stocks);
    let blueprint = runtime.save_custom_preset(&store, id, "落叶刷预设", 1206).unwrap();
    let fresh = runtime.create_library_world(&store, &blueprint.preset_id, 1206).unwrap();
    assert_eq!(fresh.simulation_tick, "0");
    assert_eq!(runtime.forest_ledger(&fresh.world_id).unwrap().unwrap().matter_input_mg_cnp, [0.0; 3]);
    for (new, consumed) in stocks(&fresh).iter().zip(consumed_stocks) {
        assert_eq!(new.patch.id, consumed.patch.id);
        assert_eq!(new.patch.position, consumed.patch.position);
        assert_eq!(new.patch.remaining_mg_cnp, new.initial_mg_cnp);
    }
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn r5_take_out_fauna_replays_inside_trials_and_persists_only_on_adoption() {
    let path = std::env::temp_dir().join(format!("micro-ecosystem-r5-take-out-fauna-{}", Uuid::new_v4()));
    let store = SaveStore::new(&path);
    let mut runtime = WorldRuntime::new();
    let formal = runtime.create_library_world(&store, "forest-r5-composed", 1206).unwrap();
    let id = formal.animals[0].id.clone();
    let remove = ForestRockCommand::Change { edit: ForestRockEdit::RemoveWoodlouse { id } };
    let edit = |runtime: &mut WorldRuntime, world: &Snapshot, command, action| {
        runtime.edit_forest_rock_command(&world.world_id, command, &world.run_generation, &world.edit_revision, action)
    };
    let baseline_ledger = runtime.forest_ledger(&formal.world_id).unwrap().unwrap();
    let abandoned = runtime.start_trial(&formal.world_id).unwrap();
    let preview = edit(&mut runtime, &abandoned, "take-out", remove.clone()).unwrap();
    assert_eq!(edit(&mut runtime, &abandoned, "take-out", remove.clone()).unwrap(), preview);
    assert_eq!(preview.animals, formal.animals[1..]);
    runtime.discard_trial(&abandoned.world_id).unwrap();
    assert_eq!(runtime.snapshot(&formal.world_id).unwrap(), formal);
    assert_eq!(runtime.forest_ledger(&formal.world_id).unwrap().unwrap(), baseline_ledger);

    let trial = runtime.start_trial(&formal.world_id).unwrap();
    let removed = edit(&mut runtime, &trial, "take-out", remove).unwrap();
    let exported = runtime.forest_ledger(&trial.world_id).unwrap().unwrap();
    let adopted = runtime.apply_trial(&store, &trial.world_id).unwrap();
    let persisted = WorldRuntime::new().load(&store, &formal.world_id).unwrap().snapshot;
    assert_eq!(adopted.world_id, formal.world_id);
    assert_eq!(adopted.animals, removed.animals);
    assert_eq!(persisted.animals, removed.animals);
    assert_eq!(runtime.forest_ledger(&formal.world_id).unwrap().unwrap(), exported);
    assert!(exported.matter_output_mg_cnp[0] > baseline_ledger.matter_output_mg_cnp[0]);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn r5_woodlouse_stock_replays_and_restores_brains_ledger_and_id_floor() {
    let path = std::env::temp_dir().join(format!("micro-ecosystem-r5-woodlouse-stock-{}", Uuid::new_v4()));
    let store = SaveStore::new(&path);
    let mut runtime = WorldRuntime::new();
    let start = runtime.create_library_world(&store, "blank-forest-r5-composed", 1206).unwrap();
    let id = &start.world_id;
    let edit = |runtime: &mut WorldRuntime, from: &Snapshot, command, action| {
        runtime.edit_forest_rock_command(id, command, &from.run_generation, &from.edit_revision, action)
    };
    let action = ForestRockCommand::Change { edit: ForestRockEdit::StockWoodlice { position: Point { x: 0.32, y: 0.73 }, count: 3 } };
    let original_ledger = runtime.forest_ledger(id).unwrap().unwrap();
    let stocked = edit(&mut runtime, &start, "stock", action.clone()).unwrap();
    assert_eq!(stocked.edit_revision, "1");
    assert!(!stocked.starter_community_available);
    assert_eq!(edit(&mut runtime, &start, "stock", action.clone()).unwrap(), stocked);
    let stocked_state = runtime.world(id).unwrap().saved.state.clone();
    let stocked_ledger = runtime.forest_ledger(id).unwrap().unwrap();
    let undone = edit(&mut runtime, &stocked, "undo", ForestRockCommand::Undo).unwrap();
    assert!(undone.animals.is_empty() && undone.starter_community_available);
    assert_eq!(runtime.forest_ledger(id).unwrap().unwrap(), original_ledger);
    let redone = edit(&mut runtime, &undone, "redo", ForestRockCommand::Redo).unwrap();
    assert_eq!(redone.animals, stocked.animals);
    assert_eq!(runtime.world(id).unwrap().saved.state, stocked_state);
    assert_eq!(runtime.forest_ledger(id).unwrap().unwrap(), stocked_ledger);

    edit(&mut runtime, &redone, "second-undo", ForestRockCommand::Undo).unwrap();
    runtime.save(&store, id).unwrap();
    let mut loaded = WorldRuntime::new();
    let restored = loaded.load(&store, id).unwrap().snapshot;
    let restocked = edit(&mut loaded, &restored, "restock", action).unwrap();
    assert_eq!(restocked.animals.iter().map(|animal| animal.id.as_str()).collect::<Vec<_>>(), ["isopod-4", "isopod-5", "isopod-6"]);
    let copy = loaded.copy_world(&store, id).unwrap();
    assert_eq!(copy.animals, restocked.animals);
    assert_eq!(loaded.world(&copy.world_id).unwrap().saved.state, loaded.world(id).unwrap().saved.state);
    let saved_as_preset = loaded.save_custom_preset(&store, id, "空白投放布局", 1206).unwrap();
    let fresh = loaded.create_library_world(&store, &saved_as_preset.preset_id, 1206).unwrap();
    assert!(fresh.animals.is_empty() && fresh.starter_community_available);
    assert_eq!(loaded.forest_ledger(&fresh.world_id).unwrap().unwrap().matter_input_mg_cnp, [0.0; 3]);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn r5_bark_shelter_passage_and_occupants_survive_edit_history_and_reopen() {
    let path = std::env::temp_dir().join(format!("micro-ecosystem-r5-bark-{}", Uuid::new_v4()));
    let store = SaveStore::new(path.clone());
    let mut runtime = WorldRuntime::new();
    let previous = runtime.create_library_world(&store, "forest-r5-stonework", 1206).unwrap();
    let initial = runtime.create_library_world(&store, "forest-r5-bark", 1206).unwrap();
    let id = &initial.world_id;
    let edit = |runtime: &mut WorldRuntime, from: &Snapshot, command_id, action| {
        runtime.edit_forest_rock_command(&from.world_id, command_id, &from.run_generation, &from.edit_revision, action)
    };
    let change = |edit| ForestRockCommand::Change { edit };
    let arch = change(ForestRockEdit::AddBarkShelter { position: Point { x: 0.38, y: 0.80 }, preset: ForestBarkPreset::Arch });
    let split = change(ForestRockEdit::AddBarkShelter { position: Point { x: 0.74, y: 0.73 }, preset: ForestBarkPreset::Split });
    assert!(previous.forest_rocks.as_ref().unwrap().bark_shelters.is_none());
    assert_eq!(initial.forest_rocks.as_ref().unwrap().bark_shelters, Some(Vec::new()));
    assert!(edit(&mut runtime, &previous, "unsupported", arch.clone()).is_err());
    let ledger = runtime.forest_ledger(id).unwrap().unwrap();
    let installed = edit(&mut runtime, &initial, "arch", arch.clone()).unwrap();
    assert_eq!(edit(&mut runtime, &initial, "arch", arch).unwrap(), installed);
    let mixed = edit(&mut runtime, &installed, "split", split).unwrap();
    let shelters = mixed.forest_rocks.as_ref().unwrap().bark_shelters.as_ref().unwrap();
    assert_eq!(shelters[0].shelter.id, 2);
    assert_eq!(shelters[0].passage_cells, [16, 28, 40]);
    assert_eq!(shelters[0].foot_cells, [27, 29]);
    assert_eq!(shelters[0].occupant_ids, ["isopod-2"]);
    assert_eq!(mixed.forest.as_ref().unwrap().walkable_cells.iter().sum::<u32>(), 43);
    assert!(edit(&mut runtime, &mixed, "block-entrance", change(ForestRockEdit::Add { position: Point { x: 0.38, y: 0.90 } })).is_err());
    assert_eq!(runtime.snapshot(id).unwrap(), mixed);

    let moved = edit(&mut runtime, &mixed, "move-arch", change(ForestRockEdit::Move { id: 2, position: Point { x: 0.22, y: 0.805 } })).unwrap();
    assert_eq!(moved.animals, mixed.animals);
    assert!(moved.forest_rocks.as_ref().unwrap().bark_shelters.as_ref().unwrap()[0].occupant_ids.is_empty());
    let restored_position = edit(&mut runtime, &moved, "undo-move", ForestRockCommand::Undo).unwrap();
    assert_eq!(restored_position.forest_rocks.as_ref().unwrap().bark_shelters.as_ref().unwrap(), shelters);
    let flipped = edit(&mut runtime, &restored_position, "mirror", change(ForestRockEdit::Mirror { id: 2 })).unwrap();
    assert_eq!(flipped.forest, mixed.forest);
    let removed = edit(&mut runtime, &flipped, "remove", change(ForestRockEdit::Remove { id: 2 })).unwrap();
    assert_eq!(removed.animals, mixed.animals);
    assert_eq!(removed.forest.as_ref().unwrap().walkable_cells.iter().sum::<u32>(), 45);
    let restored = edit(&mut runtime, &removed, "undo-remove", ForestRockCommand::Undo).unwrap();
    assert_eq!(restored.forest_rocks.as_ref().unwrap().bark_shelters, flipped.forest_rocks.as_ref().unwrap().bark_shelters);
    assert_eq!(runtime.forest_ledger(id).unwrap().unwrap(), ledger);

    let trial = runtime.start_trial(id).unwrap();
    let edited = edit(&mut runtime, &trial, "trial-remove", change(ForestRockEdit::Remove { id: 2 })).unwrap();
    assert_eq!(edited.forest_rocks.unwrap().bark_shelters.unwrap().len(), 1);
    runtime.advance_trial(&trial.world_id, 100).unwrap();
    assert_eq!(runtime.snapshot(id).unwrap(), restored);
    runtime.discard_trial(&trial.world_id).unwrap();
    runtime.save(&store, id).unwrap();
    runtime.save(&store, &previous.world_id).unwrap();
    let mut restarted = WorldRuntime::new();
    let reopened = restarted.load(&store, id).unwrap().snapshot;
    assert_eq!(reopened.forest_rocks.as_ref().unwrap().bark_shelters, restored.forest_rocks.as_ref().unwrap().bark_shelters);
    assert_eq!(reopened.animals, restored.animals);
    assert!(restarted.load(&store, &previous.world_id).unwrap().snapshot.forest_rocks.unwrap().bark_shelters.is_none());
    assert_eq!(restarted.world(id).unwrap().saved.state, runtime.world(id).unwrap().saved.state);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn r5_forest_bridges_move_mirror_round_trip_and_seed_a_custom_preset() {
    let path = std::env::temp_dir().join(format!("micro-ecosystem-r5-bridges-{}", Uuid::new_v4()));
    let store = SaveStore::new(&path);
    let mut runtime = WorldRuntime::new();
    let start = runtime.create_library_world(&store, "blank-forest-r5-composed", 1206).unwrap();
    let edit = |runtime: &mut WorldRuntime, previous: &Snapshot, command_id, action| {
        runtime.edit_forest_rock_command(&previous.world_id, command_id, &previous.run_generation, &previous.edit_revision, action).unwrap()
    };
    let change = |edit| ForestRockCommand::Change { edit };
    let root = edit(&mut runtime, &start, "f05-root", change(ForestRockEdit::AddBarkShelter {
        position: Point { x: 0.29, y: 0.78 }, preset: ForestBarkPreset::RootArch,
    }));
    let both = edit(&mut runtime, &root, "f05-branch", change(ForestRockEdit::AddBarkShelter {
        position: Point { x: 0.66, y: 0.78 }, preset: ForestBarkPreset::BranchBridge,
    }));
    let before = both.forest_rocks.as_ref().unwrap().bark_shelters.as_ref().unwrap();
    let branch = &before[1];
    assert_eq!(before[0].bridge_clearance_mm, Some([65, 12]));
    assert_eq!(branch.bridge_clearance_mm, Some([90, 22]));
    let moved = edit(&mut runtime, &both, "f05-move", change(ForestRockEdit::Move {
        id: branch.shelter.id, position: Point { x: 0.74, y: 0.79 },
    }));
    let mirrored = edit(&mut runtime, &moved, "f05-mirror", change(ForestRockEdit::Mirror { id: branch.shelter.id }));
    let retained = mirrored.forest_rocks.as_ref().unwrap().bark_shelters.as_ref().unwrap();
    assert_eq!(&retained[0], &before[0]);
    assert!(retained[1].shelter.mirrored);
    assert_ne!(retained[1].passage_cells, branch.passage_cells);
    let removed = edit(&mut runtime, &mirrored, "f05-remove", change(ForestRockEdit::Remove { id: branch.shelter.id }));
    let undone = edit(&mut runtime, &removed, "f05-undo", ForestRockCommand::Undo);
    assert_eq!(undone.forest_rocks.as_ref().unwrap().bark_shelters.as_deref(), Some(retained.as_slice()));
    runtime.save(&store, &start.world_id).unwrap();
    let custom = runtime.save_custom_preset(&store, &start.world_id, "两种枝桥", 1206).unwrap();
    let mut cold = WorldRuntime::new();
    let reopened = cold.load(&store, &start.world_id).unwrap().snapshot;
    assert_eq!(reopened.forest_rocks.unwrap().bark_shelters.as_deref(), Some(retained.as_slice()));
    let seeded = cold.create_library_world(&store, &custom.preset_id, 1206).unwrap();
    assert_eq!(seeded.forest_rocks.unwrap().bark_shelters.as_deref(), Some(retained.as_slice()));
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn r5_stonework_edits_keep_each_group_and_older_world_intact() {
    let path = std::env::temp_dir().join(format!("micro-ecosystem-r5-stonework-{}", Uuid::new_v4()));
    let store = SaveStore::new(path.clone());
    let mut runtime = WorldRuntime::new();
    let old = runtime.create_library_world(&store, "forest-r5-terrain", 1206).unwrap();
    let start = runtime.create_library_world(&store, "forest-r5-stonework", 1206).unwrap();
    let id = &start.world_id;
    let edit = |runtime: &mut WorldRuntime, previous: &Snapshot, command_id, action| {
        runtime.edit_forest_rock_command(&previous.world_id, command_id, &previous.run_generation, &previous.edit_revision, action)
    };
    let change = |edit| ForestRockCommand::Change { edit };
    let add = |x, y, preset| change(ForestRockEdit::AddStonework { position: Point { x, y }, preset });
    let pair = add(0.50, 0.615, ForestStoneworkKind::Pair);
    let ledger = runtime.forest_ledger(id).unwrap().unwrap();
    assert!(old.forest_rocks.as_ref().unwrap().stonework.is_none());
    assert_eq!(start.forest_rocks.as_ref().unwrap().stonework, Some(Vec::new()));
    assert!(edit(&mut runtime, &old, "unsupported", pair.clone()).is_err());
    let paired = edit(&mut runtime, &start, "pair", pair.clone()).unwrap();
    assert_eq!(edit(&mut runtime, &start, "pair", pair.clone()).unwrap(), paired);
    assert!(edit(&mut runtime, &paired, "overlap", pair).is_err());
    assert_eq!(runtime.snapshot(id).unwrap(), paired);

    let clustered = edit(&mut runtime, &paired, "cluster", add(0.47, 0.88, ForestStoneworkKind::Cluster)).unwrap();
    let combined = edit(&mut runtime, &clustered, "slab", add(0.82, 0.615, ForestStoneworkKind::Slab)).unwrap();
    let structures = combined.forest_rocks.as_ref().unwrap();
    assert_eq!(structures.rocks[0].id, 1);
    assert!(structures.logs.as_ref().unwrap().is_empty());
    let stonework = structures.stonework.as_ref().unwrap();
    assert_eq!(stonework.iter().map(|group| group.id).collect::<Vec<_>>(), [2, 3, 4]);
    assert_eq!(combined.forest.as_ref().unwrap().walkable_cells.iter().sum::<u32>(), 41);

    let moved = edit(&mut runtime, &combined, "move", change(ForestRockEdit::Move { id: 3, position: Point { x: 0.56, y: 0.887 } })).unwrap();
    let mirrored = edit(&mut runtime, &moved, "mirror", change(ForestRockEdit::Mirror { id: 3 })).unwrap();
    assert_eq!(moved.forest, mirrored.forest);
    let removed = edit(&mut runtime, &mirrored, "remove", change(ForestRockEdit::Remove { id: 3 })).unwrap();
    let undo = edit(&mut runtime, &removed, "undo", ForestRockCommand::Undo).unwrap();
    assert_eq!(undo.forest_rocks.as_ref().unwrap().stonework, mirrored.forest_rocks.as_ref().unwrap().stonework);
    let redo = edit(&mut runtime, &undo, "redo", ForestRockCommand::Redo).unwrap();
    assert_eq!(redo.forest_rocks.as_ref().unwrap().stonework, removed.forest_rocks.as_ref().unwrap().stonework);
    let restored = edit(&mut runtime, &redo, "restore", ForestRockCommand::Undo).unwrap();
    assert_eq!(runtime.forest_ledger(id).unwrap().unwrap(), ledger);

    let trial = runtime.start_trial(id).unwrap();
    let changed = edit(&mut runtime, &trial, "trial-remove", change(ForestRockEdit::Remove { id: 3 })).unwrap();
    assert_eq!(changed.forest_rocks.unwrap().stonework.unwrap().len(), 2);
    runtime.advance_trial(&trial.world_id, 100).unwrap();
    assert_eq!(runtime.snapshot(id).unwrap(), restored);
    runtime.discard_trial(&trial.world_id).unwrap();
    assert_eq!(runtime.snapshot(&old.world_id).unwrap(), old);

    runtime.save(&store, id).unwrap();
    runtime.save(&store, &old.world_id).unwrap();
    let mut restarted = WorldRuntime::new();
    let reopened = restarted.load(&store, id).unwrap().snapshot;
    assert_eq!(reopened.forest_rocks.as_ref().unwrap().stonework, restored.forest_rocks.as_ref().unwrap().stonework);
    assert_eq!(restarted.world(id).unwrap().saved.state, runtime.world(id).unwrap().saved.state);
    assert_ne!(reopened.run_generation, restored.run_generation);
    assert!(restarted.load(&store, &old.world_id).unwrap().snapshot.forest_rocks.unwrap().stonework.is_none());
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn r5_ground_logs_share_edit_history_and_survive_reopen() {
    let path = std::env::temp_dir().join(format!("micro-ecosystem-r5-terrain-{}", Uuid::new_v4()));
    let store = SaveStore::new(path.clone());
    let mut runtime = WorldRuntime::new();
    let old = runtime.create_library_world(&store, "forest-r5", 1206).unwrap();
    let start = runtime.create_library_world(&store, "forest-r5-terrain", 1206).unwrap();
    let id = &start.world_id;
    let terrain_edit = |runtime: &mut WorldRuntime, previous: &Snapshot, command_id, action| {
        runtime.edit_forest_rock_command(&previous.world_id, command_id, &previous.run_generation, &previous.edit_revision, action)
    };
    let change = |edit| ForestRockCommand::Change { edit };
    let log_back = change(ForestRockEdit::AddLog { position: Point { x: 0.50, y: 0.615 } });
    let log_front = change(ForestRockEdit::AddLog { position: Point { x: 0.50, y: 0.88 } });

    assert!(old.forest_rocks.as_ref().unwrap().logs.is_none());
    assert_eq!(start.forest_rocks.as_ref().unwrap().logs, Some(Vec::new()));
    assert!(matches!(terrain_edit(&mut runtime, &old, "unsupported", log_back.clone()), Err(RuntimeError::InvalidFixtureInput { .. })));
    let initial_ledger = runtime.forest_ledger(id).unwrap().unwrap();
    let first = terrain_edit(&mut runtime, &start, "first-log", log_back.clone()).unwrap();
    assert_eq!(terrain_edit(&mut runtime, &start, "first-log", log_back.clone()).unwrap(), first);
    assert!(matches!(terrain_edit(&mut runtime, &first, "overlap", log_back), Err(RuntimeError::InvalidFixtureInput { .. })));
    assert_eq!(runtime.snapshot(id).unwrap(), first);
    let second = terrain_edit(&mut runtime, &first, "second-log", log_front).unwrap();
    let placement = second.forest_rocks.as_ref().unwrap();
    assert_eq!(placement.rocks[0].id, 1);
    assert_eq!(placement.logs.as_ref().unwrap().iter().map(|log| log.id).collect::<Vec<_>>(), [2, 3]);
    let geometry = second.forest.as_ref().unwrap();
    assert!(geometry.walkable_cells.iter().sum::<u32>() < start.forest.as_ref().unwrap().walkable_cells.iter().sum());
    assert_ne!(geometry.shade_fraction, start.forest.as_ref().unwrap().shade_fraction);

    let moved = terrain_edit(&mut runtime, &second, "move-log", change(ForestRockEdit::Move {
        id: 3, position: Point { x: 0.56, y: 0.89 },
    })).unwrap();
    let mirrored = terrain_edit(&mut runtime, &moved, "mirror-log", change(ForestRockEdit::Mirror { id: 3 })).unwrap();
    let removed = terrain_edit(&mut runtime, &mirrored, "remove-log", change(ForestRockEdit::Remove { id: 3 })).unwrap();
    let undone = terrain_edit(&mut runtime, &removed, "undo-remove", ForestRockCommand::Undo).unwrap();
    assert_eq!(undone.forest_rocks.as_ref().unwrap().logs, mirrored.forest_rocks.as_ref().unwrap().logs);
    let redone = terrain_edit(&mut runtime, &undone, "redo-remove", ForestRockCommand::Redo).unwrap();
    assert_eq!(redone.forest_rocks.as_ref().unwrap().logs, removed.forest_rocks.as_ref().unwrap().logs);
    let saved = terrain_edit(&mut runtime, &redone, "restore-two-logs", ForestRockCommand::Undo).unwrap();
    assert_eq!(runtime.forest_ledger(id).unwrap().unwrap(), initial_ledger);
    assert_eq!(runtime.snapshot(&old.world_id).unwrap(), old);

    let trial = runtime.start_trial(id).unwrap();
    let trial_change = terrain_edit(&mut runtime, &trial, "trial-remove", change(ForestRockEdit::Remove { id: 2 })).unwrap();
    assert_eq!(trial_change.forest_rocks.unwrap().logs.unwrap().len(), 1);
    runtime.advance_trial(&trial.world_id, 100).unwrap();
    assert_eq!(runtime.snapshot(id).unwrap(), saved);
    runtime.discard_trial(&trial.world_id).unwrap();

    runtime.save(&store, id).unwrap();
    runtime.save(&store, &old.world_id).unwrap();
    let mut restarted = WorldRuntime::new();
    let restored = restarted.load(&store, id).unwrap().snapshot;
    assert_ne!(restored.run_generation, saved.run_generation);
    assert_eq!(restored.forest_rocks.as_ref().unwrap().logs, saved.forest_rocks.as_ref().unwrap().logs);
    assert_eq!(restarted.world(id).unwrap().saved.state, runtime.world(id).unwrap().saved.state);
    assert!(restarted.load(&store, &old.world_id).unwrap().snapshot.forest_rocks.unwrap().logs.is_none());
    for run in [&mut runtime, &mut restarted] {
        run.set_edit_paused(id, false).unwrap();
        run.advance_ticks(id, 2000).unwrap();
        run.world(id).unwrap().saved.state.validate().unwrap();
    }
    assert_eq!(restarted.world(id).unwrap().saved.state, runtime.world(id).unwrap().saved.state);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn r5_stones_replay_restore_and_keep_trial_changes_isolated() {
    let path = std::env::temp_dir().join(format!("micro-ecosystem-r5-{}", Uuid::new_v4()));
    let save = SaveStore::new(path.clone());
    let mut runtime = WorldRuntime::new();
    let start = runtime.create_library_world(&save, "forest-r5", 1206).unwrap();
    let id = &start.world_id;
    let stone_b = ForestRockCommand::Change { edit: ForestRockEdit::Add { position: Point { x: 0.81, y: 0.61 } } };
    let command = |runtime: &mut WorldRuntime, previous: &Snapshot, command_id, action| {
        runtime.edit_forest_rock_command(&previous.world_id, command_id, &previous.run_generation, &previous.edit_revision, action)
    };
    let before = runtime.forest_ledger(id).unwrap().unwrap();
    let added = command(&mut runtime, &start, "add", stone_b.clone()).unwrap();
    assert_eq!(command(&mut runtime, &start, "add", stone_b.clone()).unwrap(), added);
    assert!(matches!(command(&mut runtime, &start, "old-revision", ForestRockCommand::Undo), Err(RuntimeError::StaleRevision { .. })));
    assert!(matches!(command(&mut runtime, &added, "overlap", stone_b), Err(RuntimeError::InvalidFixtureInput { .. })));
    assert_eq!(runtime.snapshot(id).unwrap(), added);
    let change = ForestRockCommand::Change { edit: ForestRockEdit::Move { id: 1, position: Point { x: 0.13, y: 0.62 } } };
    let moved = command(&mut runtime, &added, "move", change).unwrap();
    let flashed = command(&mut runtime, &moved, "mirror", ForestRockCommand::Change { edit: ForestRockEdit::Mirror { id: 1 } }).unwrap();
    let removed = command(&mut runtime, &flashed, "remove", ForestRockCommand::Change { edit: ForestRockEdit::Remove { id: 2 } }).unwrap();
    assert_eq!(removed.forest_rocks.as_ref().unwrap().rocks.len(), 1);
    let undone = command(&mut runtime, &removed, "undo", ForestRockCommand::Undo).unwrap();
    assert_eq!(undone.forest_rocks.as_ref().unwrap().rocks, flashed.forest_rocks.as_ref().unwrap().rocks);
    let redone = command(&mut runtime, &undone, "redo", ForestRockCommand::Redo).unwrap();
    assert_eq!(redone.forest_rocks.as_ref().unwrap().rocks, removed.forest_rocks.as_ref().unwrap().rocks);
    let final_state = command(&mut runtime, &redone, "undo-again", ForestRockCommand::Undo).unwrap();
    let terrain = final_state.forest_rocks.as_ref().unwrap().rocks.clone();
    assert_eq!(terrain.iter().map(|stone| stone.id).collect::<Vec<_>>(), [1, 2]);
    assert_ne!(start.forest.as_ref().unwrap().shade_fraction, final_state.forest.as_ref().unwrap().shade_fraction);
    assert_eq!(runtime.forest_ledger(id).unwrap().unwrap(), before);

    let trial = runtime.start_trial(id).unwrap();
    let trial_edited = command(&mut runtime, &trial, "trial-edit", ForestRockCommand::Change { edit: ForestRockEdit::Remove { id: 2 } }).unwrap();
    assert_eq!(trial_edited.forest_rocks.unwrap().rocks.len(), 1);
    runtime.advance_trial(&trial.world_id, 30).unwrap();
    assert_eq!(runtime.snapshot(id).unwrap(), final_state);
    runtime.discard_trial(&trial.world_id).unwrap();

    runtime.save(&save, id).unwrap();
    let source = runtime.world(id).unwrap().saved.state.clone();
    let mut restarted = WorldRuntime::new();
    let loaded = restarted.load(&save, id).unwrap().snapshot;
    assert_eq!(loaded.forest_rocks.as_ref().unwrap().rocks, terrain);
    assert_ne!(loaded.run_generation, start.run_generation);
    assert_eq!(restarted.world(id).unwrap().saved.state, source);
    for run in [&mut runtime, &mut restarted] {
        run.set_edit_paused(id, false).unwrap();
        run.advance_ticks(id, 2000).unwrap();
        run.world(id).unwrap().saved.state.validate().unwrap();
    }
    assert_eq!(restarted.world(id).unwrap().saved.state, runtime.world(id).unwrap().saved.state);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn zero_one_and_two_observers_do_not_change_completed_ticks() {
    let mut runtime = WorldRuntime::new();
    for world_id in ["headless", "one-view", "two-views"] {
        runtime.create_fixture(world_id, 71).unwrap();
    }

    let untouched = runtime.snapshot("one-view").unwrap();
    for _ in 0..50 {
        runtime.advance_ticks("headless", 5).unwrap();
        runtime.advance_ticks("one-view", 5).unwrap();
        runtime.snapshot("one-view").unwrap();
        runtime.advance_ticks("two-views", 5).unwrap();
        runtime.snapshot("two-views").unwrap();
        runtime.snapshot("two-views").unwrap();
    }

    assert_ne!(runtime.snapshot("one-view").unwrap().simulation_tick, untouched.simulation_tick);
    let headless = &runtime.world("headless").unwrap().saved.state;
    assert_eq!(&runtime.world("one-view").unwrap().saved.state, headless);
    assert_eq!(&runtime.world("two-views").unwrap().saved.state, headless);
    assert_eq!(headless.tick(), 250);
}

#[test]
fn fixture_worlds_have_distinct_runs_and_isolated_mutable_state() {
    let mut runtime = WorldRuntime::new();
    let a = runtime.create_fixture("a", 71).unwrap();
    let b = runtime.create_fixture("b", 71).unwrap();
    assert_ne!(a.run_generation, b.run_generation);
    assert_eq!(a.animals, b.animals);
    runtime.advance_ticks("a", 73).unwrap();
    assert_eq!(runtime.snapshot("b").unwrap(), b);
    assert_ne!(runtime.snapshot("a").unwrap().animals, b.animals);
    assert!(matches!(runtime.create_fixture("b", 11), Err(RuntimeError::WorldAlreadyLoaded { .. })));
}

#[test]
fn manual_and_system_pauses_do_not_add_ticks() {
    let mut runtime = WorldRuntime::new();
    runtime.create_fixture("forest", 7).unwrap();
    let paused = runtime.set_paused("forest", true).unwrap();
    assert_eq!(runtime.advance_ticks("forest", 999_999).unwrap(), paused);
    runtime.set_suspended(true);
    assert_eq!(runtime.advance_ticks("forest", 10).unwrap(), paused);
    runtime.set_suspended(false);
    assert_eq!(runtime.advance_ticks("forest", 10).unwrap(), paused);

    let active = runtime.set_paused("forest", false).unwrap();
    runtime.set_suspended(true);
    assert_eq!(runtime.advance_ticks("forest", 10).unwrap(), active);
    runtime.set_suspended(false);
    assert_eq!(runtime.advance_ticks("forest", 1).unwrap().simulation_tick, "1");
}

#[test]
fn edits_retry_once_and_mismatching_revision_or_generation_is_rejected() {
    let mut runtime = WorldRuntime::new();
    let start = runtime.create_fixture("forest", 7).unwrap();
    let generation = &start.run_generation;
    assert_eq!(runtime.edit_rock("forest", generation, "0", 0.5, 0.6), Err(RuntimeError::EditRequiresPause));
    runtime.set_paused("forest", true).unwrap();
    let accepted = runtime.edit_rock_command("forest", "move-1", generation, "0", 0.5, 0.6).unwrap();
    let second = runtime.edit_rock_command("forest", "move-2", generation, "1", 0.4, 0.6).unwrap();
    assert_eq!(runtime.edit_rock_command("forest", "move-1", generation, "0", 0.5, 0.6).unwrap(), accepted);
    assert_eq!(runtime.snapshot("forest").unwrap(), second);
    assert_eq!(runtime.edit_rock_command("forest", "move-1", generation, "0", 0.2, 0.6), Err(RuntimeError::CommandReuse));
    assert!(matches!(runtime.edit_rock("forest", generation, "0", 0.8, 0.6), Err(RuntimeError::StaleRevision { .. })));
    assert!(matches!(runtime.edit_rock("forest", "generation-from-another-run", "2", 0.8, 0.6), Err(RuntimeError::StaleGeneration { .. })));
    assert!(matches!(runtime.edit_rock("forest", generation, "2", 1.8, 0.6), Err(RuntimeError::InvalidFixtureInput { .. })));
    assert_eq!(runtime.snapshot("forest").unwrap(), second);
}

#[test]
fn snapshot_wire_fields_are_camel_case_and_identifiers_are_strings() {
    let mut runtime = WorldRuntime::new();
    runtime.create_fixture("forest", 7).unwrap();
    let snapshot = runtime.advance_ticks("forest", 3).unwrap();
    let wire = serde_json::to_value(&snapshot).unwrap();
    assert_eq!(wire["worldId"], "forest");
    assert_eq!(wire["runGeneration"], snapshot.run_generation);
    assert_eq!(wire["editRevision"], "0");
    assert_eq!(wire["simulationTick"], "3");
    assert_eq!(wire["simulatedMillis"], 300);
    assert_eq!(wire["simulatedSeconds"], 0.3);
    assert_eq!(wire["animals"][0]["id"], "isopod-1");
    assert_eq!(wire["moisture"].as_array().unwrap().len(), 2);
}

#[test]
fn forest_trial_advances_the_complete_copy_and_leaves_formal_world_exact() {
    let mut runtime = WorldRuntime::new();
    let formal = runtime.create_forest("forest", 1206).unwrap();
    assert!(formal.forest.is_some());
    assert_eq!(runtime.start_trial("forest"), Err(RuntimeError::EditRequiresPause));
    runtime.set_edit_paused("forest", true).unwrap();
    let before = runtime.world("forest").unwrap().saved.clone();
    let trial = runtime.start_trial("forest").unwrap();
    assert_ne!(trial.world_id, "forest");
    assert_ne!(trial.run_generation, formal.run_generation);
    assert_eq!(runtime.world(&trial.world_id).unwrap().saved.state, before.state);
    let mut expected = before.state.clone();
    expected.advance_steps(600).unwrap();
    assert_eq!(runtime.advance_trial(&trial.world_id, 0), Err(RuntimeError::InvalidTrialTicks));
    runtime.set_suspended(true);
    assert_eq!(runtime.advance_trial(&trial.world_id, 30), Err(RuntimeError::Suspended));
    runtime.set_suspended(false);
    runtime.advance_trial(&trial.world_id, 600).unwrap();
    assert_eq!(runtime.world(&trial.world_id).unwrap().saved.state, expected);
    runtime.discard_trial(&trial.world_id).unwrap();
    assert_eq!(runtime.world("forest").unwrap().saved, before);
    assert_eq!(runtime.advance_trial(&trial.world_id, 1), Err(RuntimeError::TrialMissing));
}

#[test]
fn forest_reopens_with_identical_resources_rng_phase_location_and_pause_reasons() {
    let directory = std::env::temp_dir().join(format!("micro-ecosystem-forest-resume-{}", Uuid::new_v4()));
    let store = SaveStore::new(&directory);
    let mut continuous = WorldRuntime::new();
    continuous.create_forest("forest", 1206).unwrap();
    continuous.set_daylight_target("forest", 0.91).unwrap();
    continuous.advance_ticks("forest", 270).unwrap();
    continuous.set_paused("forest", true).unwrap();
    continuous.set_edit_paused("forest", true).unwrap();
    let before = continuous.world("forest").unwrap().saved.clone();
    let mut restored = WorldRuntime::new();
    continuous.save(&store, "forest").unwrap();
    store.select_active_world("forest").unwrap();
    assert_eq!(store.active_world().unwrap().as_deref(), Some("forest"));
    let first = restored.load(&store, "forest").unwrap().snapshot;
    assert_ne!(first.run_generation, continuous.snapshot("forest").unwrap().run_generation);
    assert_eq!(restored.world("forest").unwrap().saved, before);
    assert!(first.paused && first.manual_paused && first.edit_paused);
    assert!(restored.set_edit_paused("forest", false).unwrap().paused);
    assert_eq!(restored.advance_ticks("forest", 12_000).unwrap().simulation_tick, first.simulation_tick);
    continuous.set_edit_paused("forest", false).unwrap();
    for runtime in [&mut continuous, &mut restored] {
        runtime.set_paused("forest", false).unwrap();
        runtime.advance_ticks("forest", 420).unwrap();
    }
    assert_eq!(restored.world("forest").unwrap().saved, continuous.world("forest").unwrap().saved);
    // The directory belongs to this uniquely named test only.
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn r5_decorative_bed_materials_preserve_wet_and_dry_state_through_history_and_copy() {
    for style in ["bottom-fine-sand-patch", "bottom-coarse-sand-patch", "bottom-gravel-patch"] {
        assert_decorative_bed_material(style);
    }
}

fn assert_decorative_bed_material(style: &str) {
    let path = std::env::temp_dir().join(format!("micro-ecosystem-pond-bed-material-{}", Uuid::new_v4()));
    let store = SaveStore::new(&path);
    let mut runtime = WorldRuntime::new();
    let source = runtime.create_library_world(&store, "pond-r5-composed", 2729).unwrap();
    let trial = runtime.start_trial(&source.world_id).unwrap();
    let basin_id = eco_core::POND_DEFAULT_BASIN_ID;
    let find = |snapshot: &Snapshot| snapshot.pond.as_ref().unwrap().basins[0].scenery.as_ref().unwrap().iter()
        .filter(|item| item.style == style).cloned().collect::<Vec<_>>();
    let edit = |runtime: &mut WorldRuntime, from: &Snapshot, command, change| {
        runtime.edit_pond_command(&from.world_id, command, &from.run_generation, &from.edit_revision, change)
    };
    let place = |x, y| PondEdit::AddScenery { basin_id: basin_id.into(), style: style.into(), x, y };
    assert!(find(&source).is_empty());
    let wet_ledger = runtime.pond_ledger(&trial.world_id).unwrap();
    for (name, x, y) in [("side", 0.149, 0.85), ("floating", 0.5, 0.769), ("front", 0.5, 0.941)] {
        assert!(matches!(edit(&mut runtime, &trial, name, place(x, y)), Err(RuntimeError::InvalidFixtureInput { .. })));
        assert_eq!(runtime.snapshot(&trial.world_id).unwrap(), trial);
    }

    let edge = edit(&mut runtime, &trial, "place-edge", place(0.15, 0.77)).unwrap();
    assert_eq!(runtime.pond_ledger(&trial.world_id).unwrap(), wet_ledger);
    let id = find(&edge)[0].id;
    let move_patch = |x, y| PondEdit::MoveScenery { basin_id: basin_id.into(), id, x, y };
    assert!(edit(&mut runtime, &edge, "move-side", move_patch(0.851, 0.85)).is_err());
    assert_eq!(runtime.snapshot(&trial.world_id).unwrap(), edge);
    let moved = edit(&mut runtime, &edge, "move", move_patch(0.85, 0.94)).unwrap();
    assert_eq!(runtime.pond_ledger(&trial.world_id).unwrap(), wet_ledger);
    let undo = runtime.edit_pond_history_command(&moved.world_id, "undo", &moved.run_generation, &moved.edit_revision, false).unwrap();
    assert_eq!(find(&undo), find(&edge));
    let redo = runtime.edit_pond_history_command(&undo.world_id, "redo", &undo.run_generation, &undo.edit_revision, true).unwrap();
    assert_eq!(find(&redo), find(&moved));
    let liters = redo.pond.as_ref().unwrap().basins[0].water_liters;
    let dry = edit(&mut runtime, &redo, "drain", PondEdit::DrainWater { basin_id: basin_id.into(), liters }).unwrap();
    assert!(!dry.pond.as_ref().unwrap().basins[0].wet);
    assert_eq!(find(&dry), find(&redo));
    let dry_ledger = runtime.pond_ledger(&trial.world_id).unwrap();
    let dry_added = edit(&mut runtime, &dry, "place-dry", place(0.52, 0.86)).unwrap();
    assert_eq!(find(&dry_added).len(), 2);
    assert_eq!(runtime.pond_ledger(&trial.world_id).unwrap(), dry_ledger);
    assert_eq!(runtime.snapshot(&source.world_id).unwrap(), source);
    let adopted = runtime.apply_trial(&store, &trial.world_id).unwrap();
    let copied = runtime.copy_world(&store, &source.world_id).unwrap();
    let loaded = WorldRuntime::new().load(&store, &source.world_id).unwrap().snapshot;
    assert_eq!(find(&adopted), find(&copied));
    assert_eq!(find(&adopted), find(&loaded));
    assert_eq!(runtime.pond_ledger(&copied.world_id).unwrap(), dry_ledger);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn r5_living_fern_edit_history_retains_shade_and_stock_on_checkpoint() {
    let path = std::env::temp_dir().join(format!("micro-ecosystem-fern-{}", Uuid::new_v4()));
    let store = SaveStore::new(&path);
    let mut runtime = WorldRuntime::new();
    let original = runtime.create_library_world(&store, "forest-r5-composed", 1206).unwrap();
    let old = runtime.create_library_world(&store, "forest-r5-litter", 1206).unwrap();
    let read = |snapshot: &Snapshot| snapshot.forest_rocks.as_ref().unwrap().fern_patches.clone();
    assert_eq!(read(&original), Some(vec![]));
    assert!(read(&old).is_none());
    let preset = &serde_json::to_value(&original).unwrap()["forestRocks"]["fernPreset"];
    assert_eq!(preset, &serde_json::json!({"initialMgCnp":[15.0,0.54,0.075],"heightMm":140,"widthMm":130}));
    let issue = |runtime: &mut WorldRuntime, from: &Snapshot, id: &str, action| {
        runtime.edit_forest_rock_command(&from.world_id, id, &from.run_generation, &from.edit_revision, action)
    };
    let change = |edit| ForestRockCommand::Change { edit: serde_json::from_value(edit).unwrap() };
    let trial = runtime.start_trial(&original.world_id).unwrap();
    let add = change(serde_json::json!({"kind":"addFernPatch","position":{"x":0.45,"y":0.85}}));
    let added = issue(&mut runtime, &trial, "add-fern", add.clone()).unwrap();
    assert_eq!(issue(&mut runtime, &trial, "add-fern", add.clone()).unwrap(), added);
    let old_id = read(&added).unwrap()[0].patch.id;
    let undone = issue(&mut runtime, &added, "undo-add-fern", ForestRockCommand::Undo).unwrap();
    let replaced = issue(&mut runtime, &undone, "replace-fern", add).unwrap();
    let id = read(&replaced).unwrap()[0].patch.id;
    assert!(id > old_id);
    let moved = issue(&mut runtime, &replaced, "move-fern", change(serde_json::json!({
        "kind":"moveFernPatch","id":id,"position":{"x":0.72,"y":0.64},
    }))).unwrap();
    let trimmed = issue(&mut runtime, &moved, "trim-fern", change(serde_json::json!({
        "kind":"trimFernPatch","id":id,"mgC":5.0,
    }))).unwrap();
    let canopy = serde_json::to_value(&trimmed).unwrap()["forestRocks"]["fernPatches"][0].clone();
    assert_eq!(canopy["stockMgCnp"][0], 10.0);
    assert!(canopy["canopyShadeFraction"].as_f64().unwrap() < 0.28);
    let ledger = runtime.forest_ledger(&trimmed.world_id).unwrap();
    let removed = issue(&mut runtime, &trimmed, "remove-fern", change(serde_json::json!({"kind":"removeFernPatch","id":id}))).unwrap();
    let restored = issue(&mut runtime, &removed, "undo-remove-fern", ForestRockCommand::Undo).unwrap();
    assert_eq!(read(&restored), read(&trimmed));
    assert_eq!(restored.forest, trimmed.forest);
    assert_eq!(runtime.snapshot(&original.world_id).unwrap(), original);
    assert_eq!(runtime.snapshot(&old.world_id).unwrap(), old);
    let adopted = runtime.apply_trial(&store, &trial.world_id).unwrap();
    let mut reopened = WorldRuntime::new();
    let reload = reopened.load(&store, &original.world_id).unwrap().snapshot;
    assert_eq!(read(&adopted), read(&reload));
    assert_eq!(reopened.forest_ledger(&reload.world_id).unwrap(), ledger);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn r5_substrate_stroke_history_trial_copy_and_blueprint_keep_their_own_state() {
    let path = std::env::temp_dir().join(format!("micro-ecosystem-forest-substrate-{}", Uuid::new_v4()));
    let store = SaveStore::new(&path);
    let mut runtime = WorldRuntime::new();
    let source = runtime.create_library_world(&store, "forest-r5-composed", 1206).unwrap();
    let legacy = runtime.create_library_world(&store, "forest-r5-litter", 1206).unwrap();
    let read = |snapshot: &Snapshot| snapshot.forest_rocks.as_ref().unwrap().substrate_cells.clone();
    let starting = read(&source).unwrap();
    assert!(read(&legacy).is_none());
    let wire = serde_json::to_value(&source).unwrap();
    assert_eq!(wire["forestRocks"]["substrateCells"][0]["substrate"], "native");
    assert_eq!(wire["forestRocks"]["substrateCells"][0]["waterCapacityMl"], 30.0);
    assert_eq!(wire["forestRocks"]["substratePresets"][1]["substrate"], "fineSoil");
    assert_eq!(wire["forestRocks"]["substratePresets"][1]["evaporationMultiplier"], 0.85);

    let submit = |runtime: &mut WorldRuntime, snapshot: &Snapshot, command_id, change| {
        runtime.edit_forest_rock_command(&snapshot.world_id, command_id, &snapshot.run_generation, &snapshot.edit_revision, change)
    };
    let operation = |cells: Vec<usize>, name: &str| ForestRockCommand::Change {
        edit: serde_json::from_value(serde_json::json!({ "kind":"setSubstrate", "cells": cells, "substrate": name })).unwrap(),
    };

    let trial = runtime.start_trial(&source.world_id).unwrap();
    let initial_inventory = runtime.forest_ledger(&trial.world_id).unwrap();
    let exact_stroke = operation(vec![3, 15, 27], "fineSoil");
    let applied = submit(&mut runtime, &trial, "stroke", exact_stroke.clone()).unwrap();
    assert_eq!(submit(&mut runtime, &trial, "stroke", exact_stroke.clone()).unwrap(), applied);
    assert!(matches!(submit(&mut runtime, &applied, "no-change", exact_stroke), Err(RuntimeError::InvalidFixtureInput { .. })));
    assert_eq!(runtime.snapshot(&trial.world_id).unwrap(), applied);
    assert_eq!(runtime.forest_ledger(&trial.world_id).unwrap(), initial_inventory);
    let undone = submit(&mut runtime, &applied, "undo", ForestRockCommand::Undo).unwrap();
    assert_eq!(read(&undone), read(&source));
    let redone = submit(&mut runtime, &undone, "redo", ForestRockCommand::Redo).unwrap();
    assert_eq!(read(&redone), read(&applied));
    let advance = runtime.advance_trial(&trial.world_id, 48).unwrap();
    assert_eq!(runtime.snapshot(&source.world_id).unwrap(), source);
    assert_eq!(runtime.snapshot(&legacy.world_id).unwrap(), legacy);
    assert_ne!(read(&advance), read(&applied));

    let adopted = runtime.apply_trial(&store, &trial.world_id).unwrap();
    let copied = runtime.copy_world(&store, &source.world_id).unwrap();
    let mut reopened = WorldRuntime::new();
    assert_eq!(read(&copied), read(&adopted));
    assert_eq!(read(&reopened.load(&store, &source.world_id).unwrap().snapshot), read(&adopted));
    let blueprint = runtime.save_custom_preset(&store, &source.world_id, "地表回归", 1207).unwrap();
    let recreated = runtime.create_library_world(&store, &blueprint.preset_id, 1207).unwrap();
    for ((cell, initial), saved) in read(&recreated).unwrap().iter().zip(&starting).zip(read(&adopted).unwrap()) {
        assert_eq!(cell.water_ml, initial.water_ml);
        assert_eq!(cell.profile, saved.profile);
    }
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn r5_ground_moss_history_retry_trial_and_checkpoint_preserve_local_stocks() {
    let path = std::env::temp_dir().join(format!("micro-ecosystem-forest-moss-{}", Uuid::new_v4()));
    let store = SaveStore::new(&path);
    let mut runtime = WorldRuntime::new();
    let source = runtime.create_library_world(&store, "forest-r5-composed", 1206).unwrap();
    let old = runtime.create_library_world(&store, "forest-r5-litter", 1206).unwrap();
    let read = |snapshot: &Snapshot| snapshot.forest_rocks.as_ref().unwrap().moss_patches.clone();
    assert_eq!(read(&source), Some(Vec::new()));
    assert!(read(&old).is_none());
    let source_json = serde_json::to_value(&source).unwrap();
    let controls = &source_json["forestRocks"]["mossPreset"];
    assert_eq!(controls["widthMm"], 40);
    assert_eq!(controls["initialMgCnp"], serde_json::json!([8.0, 0.32, 0.048]));

    let submit = |runtime: &mut WorldRuntime, snapshot: &Snapshot, command_id, change| {
        runtime.edit_forest_rock_command(&snapshot.world_id, command_id, &snapshot.run_generation, &snapshot.edit_revision, change)
    };
    let from_json = |edit: serde_json::Value| ForestRockCommand::Change { edit: serde_json::from_value(edit).unwrap() };
    let place = from_json(serde_json::json!({ "kind":"addMossPatch", "position":{"x":0.45,"y":0.85} }));
    let trial = runtime.start_trial(&source.world_id).unwrap();
    let initial = runtime.forest_ledger(&trial.world_id).unwrap();
    let added = submit(&mut runtime, &trial, "add", place.clone()).unwrap();
    assert_eq!(submit(&mut runtime, &trial, "add", place.clone()).unwrap(), added);
    let added_id = read(&added).unwrap()[0].id;
    let undone = submit(&mut runtime, &added, "undo-add", ForestRockCommand::Undo).unwrap();
    assert_eq!(read(&undone), read(&trial));
    assert_eq!(runtime.forest_ledger(&trial.world_id).unwrap(), initial);
    let replaced = submit(&mut runtime, &undone, "replace", place).unwrap();
    let id = read(&replaced).unwrap()[0].id;
    assert!(id > added_id);
    let moved = submit(&mut runtime, &replaced, "move", from_json(serde_json::json!({
        "kind":"moveMossPatch", "id":id, "position":{"x":0.72,"y":0.64},
    }))).unwrap();
    let trimmed = submit(&mut runtime, &moved, "trim", from_json(serde_json::json!({
        "kind":"trimMossPatch", "id":id, "mgC":2.0,
    }))).unwrap();
    assert_eq!(read(&trimmed).unwrap()[0].stock_mg_cnp[0], 6.0);
    let trimmed_balance = runtime.forest_ledger(&trial.world_id).unwrap();
    let removed = submit(&mut runtime, &trimmed, "remove", from_json(serde_json::json!({
        "kind":"removeMossPatch", "id":id,
    }))).unwrap();
    assert!(read(&removed).unwrap().is_empty());
    assert_eq!(runtime.forest_ledger(&trial.world_id).unwrap().unwrap().matter_output_mg_cnp[0], 6.0);
    let recovered = submit(&mut runtime, &removed, "undo-remove", ForestRockCommand::Undo).unwrap();
    assert_eq!(read(&recovered), read(&trimmed));
    assert_eq!(runtime.forest_ledger(&trial.world_id).unwrap(), trimmed_balance);
    assert_eq!(runtime.snapshot(&source.world_id).unwrap(), source);
    assert_eq!(runtime.snapshot(&old.world_id).unwrap(), old);

    let adopted = runtime.apply_trial(&store, &trial.world_id).unwrap();
    let mut reopened = WorldRuntime::new();
    assert_eq!(read(&adopted), read(&reopened.load(&store, &source.world_id).unwrap().snapshot));
    assert_eq!(reopened.forest_ledger(&source.world_id).unwrap(), trimmed_balance);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn r5_pond_stocking_retries_and_trial_history_keep_ids_and_inventories_scoped() {
    let directory = std::env::temp_dir().join(format!("micro-ecosystem-pond-stock-{}", Uuid::new_v4()));
    let store = SaveStore::new(&directory);
    let mut runtime = WorldRuntime::new();
    let formal = runtime.create_library_world(&store, "pond-r5-composed", 2729).unwrap();
    let other = runtime.create_library_world(&store, "pond-r5-composed", 2730).unwrap();
    let trial = runtime.start_trial(&formal.world_id).unwrap();
    let initial_inventory = runtime.pond_ledger(&trial.world_id).unwrap();
    let edit = |runtime: &mut WorldRuntime, snapshot: &Snapshot, command_id, change| {
        runtime.edit_pond_command(&snapshot.world_id, command_id, &snapshot.run_generation, &snapshot.edit_revision, change)
    };
    let history = |runtime: &mut WorldRuntime, snapshot: &Snapshot, command_id, redo| {
        runtime.edit_pond_history_command(&snapshot.world_id, command_id, &snapshot.run_generation, &snapshot.edit_revision, redo)
    };
    let shrimp: PondEdit = serde_json::from_value(serde_json::json!({
        "type":"stockShrimp","basinId":eco_core::POND_DEFAULT_BASIN_ID,"position":{"x":0.48,"y":0.30},"count":2,
    })).unwrap();
    let daphnia: PondEdit = serde_json::from_value(serde_json::json!({
        "type":"stockDaphnia","basinId":eco_core::POND_DEFAULT_BASIN_ID,"count":12,
    })).unwrap();

    let stocked_shrimp = edit(&mut runtime, &trial, "shrimp", shrimp.clone()).unwrap();
    let stocked = edit(&mut runtime, &stocked_shrimp, "daphnia", daphnia.clone()).unwrap();
    assert_eq!(edit(&mut runtime, &stocked_shrimp, "daphnia", daphnia).unwrap(), stocked);
    assert_eq!(runtime.snapshot(&formal.world_id).unwrap(), formal);
    assert_eq!(runtime.snapshot(&other.world_id).unwrap(), other);
    let stocked_inventory = runtime.pond_ledger(&trial.world_id).unwrap();
    assert_ne!(stocked_inventory, initial_inventory);
    let undone_daphnia = history(&mut runtime, &stocked, "undo-daphnia", false).unwrap();
    let undone_shrimp = history(&mut runtime, &undone_daphnia, "undo-shrimp", false).unwrap();
    assert_eq!(undone_shrimp.pond, trial.pond);
    assert_eq!(runtime.pond_ledger(&trial.world_id).unwrap(), initial_inventory);
    let redone_shrimp = history(&mut runtime, &undone_shrimp, "redo-shrimp", true).unwrap();
    let redone = history(&mut runtime, &redone_shrimp, "redo-daphnia", true).unwrap();
    assert_eq!(redone.pond, stocked.pond);
    assert_eq!(runtime.pond_ledger(&trial.world_id).unwrap(), stocked_inventory);

    let adopted = runtime.apply_trial(&store, &trial.world_id).unwrap();
    assert_eq!(adopted.pond, stocked.pond);
    assert_eq!(runtime.snapshot(&other.world_id).unwrap(), other);
    let mut reopened = WorldRuntime::new();
    assert_eq!(reopened.load(&store, &formal.world_id).unwrap().snapshot.pond, stocked.pond);
    assert_eq!(reopened.pond_ledger(&formal.world_id).unwrap(), stocked_inventory);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn pond_edit_retries_history_and_invalid_capacity_preserve_exact_inventory() {
    let mut runtime = WorldRuntime::new();
    let initial = runtime.create_pond("pond", 2729).unwrap();
    let generation = &initial.run_generation;
    let add = PondEdit::AddWater { basin_id: "pond-main".to_owned(), liters: 0.8 };
    assert_eq!(runtime.edit_pond_command("pond", "running", generation, "0", add.clone()), Err(RuntimeError::EditRequiresPause));
    runtime.set_edit_paused("pond", true).unwrap();
    let initial_inventory = runtime.pond_ledger("pond").unwrap();
    let filled = runtime.edit_pond_command("pond", "add", generation, "0", add.clone()).unwrap();
    let filled_inventory = runtime.pond_ledger("pond").unwrap();
    assert_eq!(filled.pond.as_ref().unwrap().basins[0].water_liters, 18.8);
    assert!(filled.pond_undo_available && !filled.pond_redo_available);
    let impossible = PondEdit::Reshape { basin_id: "pond-main".to_owned(), surface_area_cm2: 400.0 };
    assert!(matches!(runtime.edit_pond_command("pond", "narrow", generation, "1", impossible),
        Err(RuntimeError::InvalidFixtureInput { .. })));
    assert_eq!(runtime.snapshot("pond").unwrap(), filled);

    let undone = runtime.edit_pond_history_command("pond", "undo", generation, "1", false).unwrap();
    assert_eq!(undone.pond, initial.pond);
    assert_eq!(runtime.pond_ledger("pond").unwrap(), initial_inventory);
    assert!(!undone.pond_undo_available && undone.pond_redo_available);
    assert_eq!(runtime.edit_pond_command("pond", "add", generation, "0", add).unwrap(), filled);
    assert_eq!(runtime.snapshot("pond").unwrap(), undone);
    let redone = runtime.edit_pond_history_command("pond", "redo", generation, "2", true).unwrap();
    assert_eq!(redone.pond, filled.pond);
    assert_eq!(runtime.pond_ledger("pond").unwrap(), filled_inventory);
    assert!(matches!(runtime.edit_pond_history_command("pond", "redo", generation, "2", false), Err(RuntimeError::CommandReuse)));
    runtime.set_edit_paused("pond", false).unwrap();
    let after = runtime.advance_ticks("pond", 1).unwrap();
    assert!(!after.pond_undo_available && !after.pond_redo_available);
}

#[test]
fn mixed_world_checkpoints_and_pond_trial_keep_state_and_season_separate() {
    let directory = std::env::temp_dir().join(format!("micro-ecosystem-pond-resume-{}", Uuid::new_v4()));
    let store = SaveStore::new(&directory);
    let mut runtime = WorldRuntime::new();
    let forest = runtime.create_seasonal_forest("forest", 1206).unwrap();
    runtime.save(&store, "forest").unwrap();
    let pond = runtime.create_pond("pond", 2729).unwrap();
    runtime.set_daylight_target("pond", 0.91).unwrap();
    runtime.advance_ticks("pond", 650).unwrap();
    runtime.set_edit_paused("pond", true).unwrap();
    runtime.edit_pond_command("pond", "detritus", &pond.run_generation, "0",
        PondEdit::AddDetritus { basin_id: "pond-main".to_owned(), mg_c: 50.0 }).unwrap();
    let saved = runtime.world("pond").unwrap().saved.clone();
    let trial = runtime.start_trial("pond").unwrap();
    let advanced = runtime.advance_trial(&trial.world_id, 600).unwrap();
    assert_ne!(advanced.pond, trial.pond);
    assert_eq!(runtime.save(&store, &trial.world_id), Err(RuntimeError::TrialMissing));
    runtime.discard_trial(&trial.world_id).unwrap();
    assert_eq!(runtime.world("pond").unwrap().saved, saved);
    assert_eq!(runtime.snapshot("forest").unwrap(), forest);
    runtime.save(&store, "pond").unwrap();
    store.select_active_world("pond").unwrap();

    let mut reopened = WorldRuntime::new();
    assert_eq!(store.active_world().unwrap().as_deref(), Some("pond"));
    let loaded = reopened.load(&store, "pond").unwrap().snapshot;
    assert_ne!(loaded.run_generation, pond.run_generation);
    assert!(loaded.edit_paused && !loaded.pond_undo_available);
    assert_eq!(reopened.world("pond").unwrap().saved, saved);
    assert_eq!(reopened.load(&store, "forest").unwrap().snapshot.forest, forest.forest);
    for instance in [&mut runtime, &mut reopened] {
        instance.set_edit_paused("pond", false).unwrap();
        instance.advance_ticks("pond", 900).unwrap();
    }
    assert_eq!(reopened.world("pond").unwrap().saved, runtime.world("pond").unwrap().saved);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn automatic_checkpoint_rotation_keeps_manual_and_damaged_versions_and_recovers_previous() {
    let directory = std::env::temp_dir().join(format!("micro-ecosystem-automatic-{}", Uuid::new_v4()));
    let store = SaveStore::new(&directory);
    let mut runtime = WorldRuntime::new();
    runtime.create_pond("pond", 2729).unwrap();
    assert_eq!(runtime.save(&store, "pond").unwrap().checkpoint_serial, "1");
    for serial in 2..=4 {
        runtime.advance_ticks("pond", 3).unwrap();
        assert_eq!(runtime.autosave(&store, "pond").unwrap().checkpoint_serial, serial.to_string());
    }
    let world = directory.join("world-pond");
    let automatic = |serial: u64| world.join(format!("autosave-{serial:020}.json"));
    assert!(!automatic(2).exists());
    assert!(world.join("checkpoint-00000000000000000001.json").is_file());
    assert!(automatic(3).is_file() && automatic(4).is_file());
    std::fs::write(automatic(3), b"damaged prior automatic checkpoint").unwrap();
    runtime.advance_ticks("pond", 7).unwrap();
    assert_eq!(runtime.autosave(&store, "pond").unwrap().checkpoint_serial, "5");
    assert_eq!(std::fs::read(automatic(3)).unwrap(), b"damaged prior automatic checkpoint");
    let mut restored = WorldRuntime::new();
    let latest = restored.load(&store, "pond").unwrap();
    assert!(latest.automatic && !latest.recovered_previous);
    assert_eq!(latest.snapshot.simulation_tick, "16");
    std::fs::write(automatic(5), b"incomplete checkpoint").unwrap();
    let fallback = restored.load(&store, "pond").unwrap();
    assert!(fallback.automatic && fallback.recovered_previous);
    assert_eq!(fallback.snapshot.simulation_tick, "9");
    std::fs::remove_dir_all(directory).unwrap();
}

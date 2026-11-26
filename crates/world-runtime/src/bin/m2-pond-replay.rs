use eco_core::{POND_DEFAULT_BASIN_ID, STEP_MILLIS, WorldState};
use serde_json::json;
use world_runtime::{PondEdit, WorldRuntime};

const SEED: u64 = 2729;
const TICKS: u32 = 72_000;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().nth(1).as_deref() == Some("--baseline") {
        return baseline();
    }
    let mut runtime = WorldRuntime::new();
    let initial = runtime.create_pond("day", SEED)?;
    for id in ["night", "loaded-night", "edited"] {
        runtime.create_pond(id, SEED)?;
    }
    runtime.set_daylight_target("day", 0.5)?;
    runtime.set_daylight_target("night", 0.0)?;
    runtime.set_daylight_target("loaded-night", 0.0)?;
    runtime.set_daylight_target("edited", 0.5)?;

    let basin_id = POND_DEFAULT_BASIN_ID.to_owned();
    runtime.set_edit_paused("loaded-night", true)?;
    let loaded = runtime.snapshot("loaded-night")?;
    runtime.edit_pond_command("loaded-night", "load", &loaded.run_generation, &loaded.edit_revision,
        PondEdit::AddDetritus { basin_id: basin_id.clone(), mg_c: 2500.0 })?;
    runtime.set_edit_paused("loaded-night", false)?;

    let mut edited = runtime.set_edit_paused("edited", true)?;
    for (id, edit) in [
        ("add", PondEdit::AddWater { basin_id: basin_id.clone(), liters: 0.8 }),
        ("drain", PondEdit::DrainWater { basin_id: basin_id.clone(), liters: 0.8 }),
        ("reshape", PondEdit::Reshape { basin_id: basin_id.clone(), surface_area_cm2: 2000.0 }),
    ] {
        edited = runtime.edit_pond_command("edited", id, &edited.run_generation, &edited.edit_revision, edit)?;
    }
    let edited_ledger = runtime.pond_ledger("edited")?;

    let mut dry = WorldState::new_pond(SEED);
    let start_water = dry.pond_observation().unwrap().basins[0].water_liters;
    dry.edit_pond(&PondEdit::DrainWater { basin_id: basin_id.clone(), liters: start_water })?;
    let dry_observation = dry.pond_observation();
    dry.advance_steps(600)?;
    dry.edit_pond(&PondEdit::AddWater { basin_id: basin_id.clone(), liters: 0.8 })?;
    let rewetted = dry.pond_observation();

    let mut isolated = WorldState::new_pond_with_basins(SEED, 2)?;
    let mut twin = isolated.clone();
    isolated.edit_pond(&PondEdit::AddDetritus { basin_id, mg_c: 2500.0 })?;
    isolated.advance_steps(TICKS)?;
    twin.advance_steps(TICKS)?;
    let isolation = isolated.pond_observation().unwrap().basins[1] == twin.pond_observation().unwrap().basins[1];
    assert!(isolation);

    let mut scenarios = Vec::new();
    for id in ["day", "night", "loaded-night"] {
        let snapshot = runtime.advance_ticks(id, TICKS)?;
        let ledger = runtime.pond_ledger(id)?.unwrap();
        for balance in &ledger.basins {
            assert!(balance.water_residual_liters.abs() < 1e-8);
            assert!(balance.oxygen_residual_mg.abs() < 1e-7);
            assert!(balance.matter_residual_mg_cnp.iter().all(|residual| residual.abs() < 1e-6));
        }
        scenarios.push(json!({ "name": id, "daylightPhase": snapshot.daylight_phase, "pond": snapshot.pond, "ledger": ledger }));
    }
    println!("{}", serde_json::to_string_pretty(&json!({
        "scenario": "POND-REPLAY",
        "seed": SEED,
        "elapsedSimulatedSeconds": TICKS as f64 * STEP_MILLIS as f64 / 1000.0,
        "initial": initial.pond,
        "editGeometry": { "pond": edited.pond, "ledger": edited_ledger },
        "lightAndOrganicLoad": scenarios,
        "dryObservation": dry_observation,
        "rewettedObservation": rewetted,
        "undisturbedBasinMatchesTwin": isolation,
    }))?);
    Ok(())
}

fn baseline() -> Result<(), Box<dyn std::error::Error>> {
    const TICKS_PER_HALF_DAY: u32 = (12 * 60 * 60 * 1000 / STEP_MILLIS) as u32;
    let mut pond = WorldState::new_pond(SEED);
    let mut forest = WorldState::new_seasonal_forest(1206);
    let mut samples = Vec::new();

    for interval in 0..=6 {
        if interval > 0 {
            pond.advance_steps(TICKS_PER_HALF_DAY)?;
            forest.advance_steps(TICKS_PER_HALF_DAY)?;
        }
        let basin = &pond.pond_observation().unwrap().basins[0];
        let pond_ledger = &pond.pond_ledger().unwrap().basins[0];
        let forest_readings = forest.forest_observation().unwrap();
        let forest_ledger = forest.forest_ledger().unwrap();
        pond.validate()?;
        forest.validate()?;
        samples.push(json!({
            "simulatedHours": interval * 12,
            "season": pond.season_observation(),
            "pond": {
                "waterLiters": basin.water_liters,
                "oxygenMgPerLiter": basin.oxygen_mg_per_liter,
                "shrimpCount": basin.shrimp.len(),
                "daphniaCount": basin.daphnia_count,
                "waterResidualLiters": pond_ledger.water_residual_liters,
                "oxygenResidualMg": pond_ledger.oxygen_residual_mg,
                "matterResidualMgCnp": pond_ledger.matter_residual_mg_cnp,
            },
            "forest": {
                "waterMl": forest_readings.water_ml,
                "woodliceBySide": forest_readings.woodlice_by_side,
                "litterMgC": forest_readings.litter_mg_c,
                "ledger": forest_ledger,
            }
        }));
    }
    println!("{}", serde_json::to_string_pretty(&json!({
        "scenario": "M2-THREE-DAY-EXPERIMENTAL-BASELINE",
        "pondSeed": SEED,
        "forestSeed": 1206,
        "conditions": "No edit, built-in seasonal forcing, simulated natural daylight, starting at noon.",
        "samples": samples,
    }))?);
    Ok(())
}

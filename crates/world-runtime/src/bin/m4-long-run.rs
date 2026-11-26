use eco_core::{WorldState, POND_DEFAULT_BASIN_ID, STEP_MILLIS};
use serde_json::{json, Value};
use std::{error::Error, time::Instant};
use world_runtime::{PondEdit, WorldRuntime};

const FOREST_SEED: u64 = 1206;
const POND_SEED: u64 = 2729;
const DAY: u32 = (86_400_000 / STEP_MILLIS) as u32;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn main() -> Result<()> {
    let days = std::env::args().nth(1).map_or(Ok(60), |value| value.parse::<u32>())?;
    if !(1..=365).contains(&days) {
        return Err("simulated days must be between 1 and 365".into());
    }
    let timer = Instant::now();
    let forest = run_baseline(WorldState::new_seasonal_forest(FOREST_SEED), FOREST_SEED, days)?;
    let pond = run_baseline(WorldState::new_pond(POND_SEED), POND_SEED, days)?;
    println!("{}", serde_json::to_string_pretty(&json!({
        "scenario": "M4-FIXED-LONG-RUN",
        "simulatedDaysPerBaseline": days,
        "stepMillis": STEP_MILLIS,
        "forest": forest,
        "pond": pond,
        "pondIntervention": pond_intervention()?,
        "worldIsolation": mixed_worlds()?,
        "wallSeconds": timer.elapsed().as_secs_f64(),
    }))?);
    Ok(())
}

fn require(condition: bool, message: &str) -> Result<()> {
    if condition { Ok(()) } else { Err(message.to_owned().into()) }
}

fn run_baseline(mut world: WorldState, seed: u64, days: u32) -> Result<Value> {
    let timer = Instant::now();
    let definition = world.definition().to_owned();
    let initial = serde_json::to_value(&world)?;
    let mut samples = Vec::new();
    let mut maximum = [0.0_f64; 5];
    let mut first_woodlice_zero_hour = None;
    let mut first_shrimp_zero_hour = None;
    let mut first_daphnia_zero_hour = None;
    let mut first_dry_hour = None;
    let mut replay_verified = Vec::new();
    for hour in (0..=days * 24).step_by(6) {
        if hour > 0 {
            // Replay one interval near the initial community and one at the end.
            // The saved state includes the current RNG and every inventory.
            let replay = hour == 6 || hour == days * 24;
            let copy = replay.then(|| serde_json::to_vec(&world)).transpose()?;
            world.advance_steps(DAY / 4)?;
            if let Some(copy) = copy {
                let mut restored: WorldState = serde_json::from_slice(&copy)?;
                for _ in 0..6 { restored.advance_steps(DAY / 24)?; }
                require(restored == world, &format!("state differs after replay in {definition} at {hour}h"))?;
                replay_verified.push(hour);
            }
        }
        world.validate().map_err(|error| format!("{definition} invalid at hour {hour}: {error}"))?;
        require(world.tick() == u64::from(hour) * u64::from(DAY / 24), "unexpected baseline tick")?;
        let observations = if let Some(forest) = world.forest_observation() {
            let ledger = world.forest_ledger().ok_or("forest ledger missing")?;
            let animals = forest.woodlice_by_side.into_iter().sum::<u32>();
            let life = forest.life.ok_or("seasonal life missing")?;
            if animals == 0 { first_woodlice_zero_hour.get_or_insert(hour); }
            for (slot, value) in maximum.iter_mut().zip(std::iter::once(ledger.water_residual_ml).chain(ledger.matter_residual_mg_cnp)) {
                *slot = slot.max(value.abs());
            }
            json!({
                "waterMl": forest.water_ml,
                "woodlice": animals,
                "woodliceBorn": life.woodlice_born,
                "woodliceDied": life.woodlice_died,
                "litterMgC": forest.litter_mg_c,
                "fernMgC": life.fern_mg_c.into_iter().sum::<f64>(),
                "mossMgC": life.moss_mg_c.into_iter().sum::<f64>(),
                "springtailMgC": life.springtail_mg_c.into_iter().sum::<f64>(),
                "ledger": ledger,
            })
        } else {
            let pond = world.pond_observation().ok_or("pond observation missing")?;
            let basin = &pond.basins[0];
            let ledger = world.pond_ledger().ok_or("pond ledger missing")?;
            let balance = &ledger.basins[0];
            if basin.shrimp.is_empty() { first_shrimp_zero_hour.get_or_insert(hour); }
            if basin.daphnia_count == 0 { first_daphnia_zero_hour.get_or_insert(hour); }
            if !basin.wet { first_dry_hour.get_or_insert(hour); }
            let residuals = std::iter::once(balance.water_residual_liters)
                .chain(balance.matter_residual_mg_cnp)
                .chain(std::iter::once(balance.oxygen_residual_mg));
            for (slot, value) in maximum.iter_mut().zip(residuals) { *slot = slot.max(value.abs()); }
            json!({
                "waterLiters": basin.water_liters,
                "shrimp": basin.shrimp.len(),
                "daphnia": basin.daphnia_count,
                "dissolvedOxygenMgPerLiter": basin.oxygen_mg_per_liter,
                "detritusMgC": basin.detritus_mg_c,
                "ledger": balance,
            })
        };
        if [0, 6, 12, 24, 72, 168, 336, 720, 1392, 1416].contains(&hour) || hour == days * 24 {
            samples.push(json!({ "simulatedHour": hour, "tick": world.tick(), "season": world.season_observation(), "observations": observations }));
        }
        if hour != 0 && hour % 240 == 0 { eprintln!("{definition}: verified through day {}", hour / 24); }
    }
    if let Some(forest) = world.forest_ledger() {
        require(forest.water_input_ml > 0.0 && forest.water_output_ml > 0.0, "forest boundary water flux missing")?;
        require(forest.current_matter_mg_cnp[0] > 0.0, "forest matter inventory unexpectedly empty")?;
    } else if let Some(pond) = world.pond_ledger() {
        require(pond.basins[0].water_input_liters == 0.0 && pond.basins[0].water_output_liters > 0.0, "baseline pond water boundary incorrect")?;
    }
    Ok(json!({
        "seed": seed,
        "definition": definition,
        "modelVersion": world.model_version(),
        "initialState": initial,
        "stepCount": world.tick(),
        "validationSamples": days * 4 + 1,
        "replayedFromSerializedStateEndingAtHours": replay_verified,
        "maximumAbsoluteWaterResidual": maximum[0],
        "maximumAbsoluteMatterResidualMgCnp": &maximum[1..4],
        "maximumAbsoluteOxygenResidualMg": world.pond_observation().map(|_| maximum[4]),
        "firstObservedNoWoodliceHour": first_woodlice_zero_hour,
        "firstObservedNoShrimpHour": first_shrimp_zero_hour,
        "firstObservedNoDaphniaHour": first_daphnia_zero_hour,
        "firstObservedDryHour": first_dry_hour,
        "samples": samples,
        "wallSeconds": timer.elapsed().as_secs_f64(),
    }))
}

fn pond_intervention() -> Result<Value> {
    let mut world = WorldState::new_pond_with_basins(POND_SEED, 2)?;
    let mut reference = world.clone();
    let basin_id = POND_DEFAULT_BASIN_ID.to_owned();
    world.edit_pond(&PondEdit::AddDetritus { basin_id: basin_id.clone(), mg_c: 2500.0 })?;
    require(world.pond_ledger().ok_or("detritus ledger missing")?.basins[0].matter_input_mg_cnp[0] == 2500.0, "detritus was not booked as boundary input")?;
    world.advance_steps(DAY)?;
    reference.advance_steps(DAY)?;
    let before_drain = world.pond_observation().ok_or("observation before draining missing")?;
    world.edit_pond(&PondEdit::DrainWater { basin_id: basin_id.clone(), liters: before_drain.basins[0].water_liters })?;
    world.validate()?;
    let dry = world.pond_observation().ok_or("dry pond observation missing")?;
    let dry_main = &dry.basins[0];
    require(!dry_main.wet && dry_main.water_liters == 0.0 && dry_main.oxygen_mg_per_liter.is_none()
        && dry_main.shrimp.is_empty() && dry_main.daphnia_count == 0, "drained pool retained aquatic conditions")?;
    world.advance_steps(DAY)?;
    reference.advance_steps(DAY)?;
    world.edit_pond(&PondEdit::AddWater { basin_id, liters: 0.8 })?;
    world.validate()?;
    let refill = world.pond_observation().ok_or("refilled pond observation missing")?;
    let refilled = &refill.basins[0];
    require(refilled.wet && refilled.water_liters == 0.8 && refilled.oxygen_mg_per_liter.is_some_and(|value| value > 0.0)
        && refilled.shrimp.is_empty() && refilled.daphnia_count == 0, "refilling created aquatic life or invalid water")?;
    let ledger = world.pond_ledger().ok_or("refilled pond ledger missing")?;
    require(ledger.basins[0].water_input_liters == 0.8, "refill was not booked as boundary input")?;
    let full = serde_json::to_value(&world)?;
    let control = serde_json::to_value(&reference)?;
    let second_basin = |value: &Value| value.get("pond").and_then(|pond| pond.get("basins")).and_then(|basins| basins.get(1)).cloned();
    let treated_second_basin = second_basin(&full).ok_or("second basin state missing")?;
    let control_second_basin = second_basin(&control).ok_or("control second basin state missing")?;
    require(treated_second_basin == control_second_basin, "independent basin state or RNG changed under other basin edits")?;
    Ok(json!({
        "seed": POND_SEED, "stepCount": world.tick(), "addedDetritusMgC": 2500.0, "waterRefillLiters": 0.8,
        "untouchedBasinExactIncludingRng": true, "treatmentBeforeDraining": before_drain.basins[0],
        "dry": dry_main, "refilled": refilled, "ledger": ledger,
    }))
}

fn mixed_worlds() -> Result<Value> {
    let mut runtime = WorldRuntime::new();
    runtime.create_seasonal_forest("m4-forest", FOREST_SEED)?;
    runtime.create_pond("m4-pond", POND_SEED)?;
    let paused = runtime.set_paused("m4-pond", true)?;
    let before_pond = runtime.pond_ledger("m4-pond")?;
    for _ in 0..8 {
        runtime.advance_ticks("m4-forest", 1200)?;
        runtime.advance_ticks("m4-pond", 1200)?;
    }
    require(runtime.snapshot("m4-pond")? == paused && runtime.pond_ledger("m4-pond")? == before_pond,
        "running forest changed paused pond snapshot or inventory")?;
    let forest = runtime.set_edit_paused("m4-forest", true)?;
    let before_forest = runtime.forest_ledger("m4-forest")?;
    runtime.set_paused("m4-pond", false)?;
    for _ in 0..4 {
        runtime.advance_ticks("m4-pond", 1200)?;
        runtime.advance_ticks("m4-forest", 1200)?;
    }
    require(runtime.snapshot("m4-forest")? == forest && runtime.forest_ledger("m4-forest")? == before_forest,
        "running pond changed edit-paused forest snapshot or inventory")?;
    let pond = runtime.snapshot("m4-pond")?;
    require(forest.simulation_tick == "9600" && pond.simulation_tick == "4800", "mixed world elapsed ticks differ")?;
    Ok(json!({ "pausedSnapshotsAndInventoriesExact": true, "forestTick": forest.simulation_tick, "pondTick": pond.simulation_tick }))
}

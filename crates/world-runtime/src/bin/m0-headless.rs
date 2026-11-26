use serde_json::json;
use world_runtime::{Snapshot, WorldRuntime, STEP_MILLIS};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let no_view = run_fixture(0)?;
    let one_view = run_fixture(1)?;
    let two_views = run_fixture(2)?;
    for view_run in [&one_view, &two_views] {
        assert_eq!(no_view.simulation_tick, view_run.simulation_tick);
        assert_eq!(no_view.daylight_phase, view_run.daylight_phase);
        assert_eq!(no_view.animals, view_run.animals);
        assert_eq!(no_view.moisture, view_run.moisture);
    }

    let result = json!({
        "mode": "M0 illustrative fixture",
        "fixedStepMillis": STEP_MILLIS,
        "snapshotReadsPerTick": [0, 1, 2],
        "sameStateAndTick": true,
        "snapshot": no_view
    });
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

fn run_fixture(snapshot_reads: u8) -> Result<Snapshot, Box<dyn std::error::Error>> {
    let mut runtime = WorldRuntime::new();
    runtime.create_fixture("forest", 42)?;
    for _ in 0..120 {
        runtime.advance_ticks("forest", 1)?;
        for _ in 0..snapshot_reads {
            runtime.snapshot("forest")?;
        }
    }
    Ok(runtime.snapshot("forest")?)
}

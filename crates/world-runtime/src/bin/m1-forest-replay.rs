use serde_json::json;
use world_runtime::WorldRuntime;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut runtime = WorldRuntime::new();
    for id in ["control", "moved"] {
        runtime.create_forest(id, 1206)?;
        runtime.set_daylight_target(id, 0.50)?;
    }
    let before = runtime.set_edit_paused("moved", true)?;
    let changed = runtime.edit_rock("moved", &before.run_generation, &before.edit_revision, 0.74, 0.65)?;
    runtime.set_edit_paused("moved", false)?;
    let control = runtime.advance_ticks("control", 6_000)?;
    let moved = runtime.advance_ticks("moved", 6_000)?;
    println!("{}", serde_json::to_string_pretty(&json!({
        "scenario": "FOREST-ROCK",
        "seed": 1206,
        "elapsedSimulatedSeconds": control.simulated_seconds,
        "beforeRock": before.rock,
        "afterRock": changed.rock,
        "beforeEdit": before.forest,
        "immediateAfterEdit": changed.forest,
        "controlAfterRun": control.forest,
        "movedAfterRun": moved.forest,
        "controlLedger": runtime.forest_ledger("control")?,
        "movedLedger": runtime.forest_ledger("moved")?,
        "scope": "林下功能型实验预设。结果用于模型验证，参数待物种与情景校准。",
    }))?);
    Ok(())
}

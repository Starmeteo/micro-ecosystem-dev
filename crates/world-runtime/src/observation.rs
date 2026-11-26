//! Read-only samples of the authoritative world, retained with its checkpoint.

use eco_core::{STEP_MILLIS, WorldState};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

pub(super) const RECENT_INTERVAL_TICKS: u64 = 10_000 / STEP_MILLIS;
const DAY_INTERVAL_TICKS: u64 = 600_000 / STEP_MILLIS;
const RECENT_SAMPLES: usize = 61;
const DAY_SAMPLES: usize = 145;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationHistory {
    recent: VecDeque<ObservationSample>,
    day: VecDeque<ObservationSample>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ObservationSample {
    simulated_millis: u64,
    reading: Reading,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "theme", content = "metrics", rename_all = "camelCase", deny_unknown_fields)]
enum Reading {
    Forest(ForestReading),
    Pond(Vec<BasinReading>),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ForestReading {
    woodlice: u32,
    soil_water_ml: f64,
    litter_mg_c: f64,
    springtail_mg_c: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BasinReading {
    id: String,
    shrimp: usize,
    daphnia: u32,
    water_liters: f64,
    oxygen_mg_per_liter: Option<f64>,
    detritus_mg_c: f64,
}

impl ObservationHistory {
    pub(super) fn is_empty(&self) -> bool { self.recent.is_empty() && self.day.is_empty() }

    pub(super) fn record(&mut self, state: &WorldState) {
        let simulated_millis = state.simulated_millis();
        if self.recent.back().is_some_and(|point| point.simulated_millis == simulated_millis) { return; }
        let Some(reading) = Reading::from_world(state) else { return; };
        let point = ObservationSample { simulated_millis, reading };
        if self.day.is_empty() || state.tick() % DAY_INTERVAL_TICKS == 0 {
            push_sample(&mut self.day, DAY_SAMPLES, point.clone());
        }
        push_sample(&mut self.recent, RECENT_SAMPLES, point);
    }

    pub(super) fn view(&self, state: &WorldState) -> Self {
        let mut history = self.clone();
        // Loading a legacy checkpoint never writes a sample. Its current state can still
        // be shown as the first observation until the real simulation advances.
        if history.is_empty() { history.record(state); }
        history
    }

    pub(super) fn valid(&self, state: &WorldState) -> bool {
        if self.is_empty() { return true; }
        if self.recent.is_empty() || self.day.is_empty() { return false; }
        let Some(current) = Reading::from_world(state) else { return false; };
        let latest = state.simulated_millis();
        let check = |series: &VecDeque<ObservationSample>, maximum| {
            series.len() <= maximum && series.iter().all(|sample| {
                sample.simulated_millis % STEP_MILLIS == 0 && sample.simulated_millis <= latest
                    && std::mem::discriminant(&sample.reading) == std::mem::discriminant(&current)
                    && sample.reading.valid()
            }) && series.iter().zip(series.iter().skip(1)).all(|(a, b)| a.simulated_millis < b.simulated_millis)
        };
        check(&self.recent, RECENT_SAMPLES) && check(&self.day, DAY_SAMPLES)
    }
}

fn push_sample(series: &mut VecDeque<ObservationSample>, capacity: usize, point: ObservationSample) {
    if series.len() == capacity { series.pop_front(); }
    series.push_back(point);
}

// Use the same readings as the inspectors, rounded once to their useful display precision.
fn measured(value: f64) -> f64 { (value * 1_000.0).round() / 1_000.0 }

fn nonnegative(value: f64) -> bool { value.is_finite() && value >= 0.0 }

impl Reading {
    fn from_world(state: &WorldState) -> Option<Self> {
        if let Some(forest) = state.forest_observation() {
            return Some(Self::Forest(ForestReading {
                woodlice: forest.woodlice_by_side.into_iter().sum(),
                soil_water_ml: measured(forest.water_ml.into_iter().sum()),
                litter_mg_c: measured(forest.litter_mg_c),
                springtail_mg_c: forest.life.map(|life| measured(life.springtail_mg_c.into_iter().sum())),
            }));
        }
        state.pond_observation().map(|pond| Self::Pond(pond.basins.into_iter().map(|basin| BasinReading {
            id: basin.id,
            shrimp: basin.shrimp.len() + basin.static_sections.iter().map(|section| section.shrimp.len()).sum::<usize>(),
            daphnia: basin.daphnia_count,
            water_liters: measured(basin.water_liters),
            oxygen_mg_per_liter: basin.oxygen_mg_per_liter.map(measured),
            detritus_mg_c: measured(basin.detritus_mg_c),
        }).collect()))
    }

    fn valid(&self) -> bool {
        match self {
            Self::Forest(reading) => nonnegative(reading.soil_water_ml) && nonnegative(reading.litter_mg_c)
                && reading.springtail_mg_c.is_none_or(nonnegative),
            Self::Pond(basins) => (1..=4).contains(&basins.len()) && basins.iter().enumerate().all(|(index, basin)| {
                !basin.id.is_empty() && basin.id.len() <= 80 && !basins[..index].iter().any(|previous| previous.id == basin.id)
                    && nonnegative(basin.water_liters) && nonnegative(basin.detritus_mg_c)
                    && basin.oxygen_mg_per_liter.is_none_or(nonnegative)
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PondEdit, RuntimeError, SaveStore, WorldRuntime};

    #[test]
    fn observation_samples_are_deterministic_through_chunking_pause_and_retention() {
        let mut runtime = WorldRuntime::new();
        let forest = runtime.create_forest("forest", 1206).unwrap();
        let control = runtime.create_forest("control", 1206).unwrap();
        let pond = runtime.create_pond("pond", 2729).unwrap();
        let initial = runtime.observation_history("forest", &forest.run_generation).unwrap();
        assert_eq!(initial.recent.len(), 1);
        runtime.advance_ticks("forest", 80).unwrap();
        runtime.advance_ticks("forest", 6_131).unwrap();
        runtime.advance_ticks("control", 6_211).unwrap();
        let history = runtime.observation_history("forest", &forest.run_generation).unwrap();
        assert_eq!(history, runtime.observation_history("control", &control.run_generation).unwrap());
        assert_eq!(history.recent.len(), RECENT_SAMPLES);
        assert_eq!(history.recent.front().unwrap().simulated_millis, 20_000);
        assert_eq!(history.recent.back().unwrap().simulated_millis, 620_000);
        assert_eq!(history.day.back().unwrap().simulated_millis, 600_000);
        assert_eq!(runtime.world("forest").unwrap().saved.state, runtime.world("control").unwrap().saved.state);
        runtime.set_paused("forest", true).unwrap();
        runtime.advance_ticks("forest", 2_000).unwrap();
        assert_eq!(runtime.observation_history("forest", &forest.run_generation).unwrap(), history);
        assert_eq!(runtime.observation_history("pond", &pond.run_generation).unwrap().recent.len(), 1);
        assert!(matches!(runtime.observation_history("forest", &pond.run_generation), Err(RuntimeError::StaleGeneration { .. })));
    }

    #[test]
    fn observation_history_preserves_dry_gaps_and_follows_adopted_checkpoints_only() {
        let path = std::env::temp_dir().join(format!("micro-observation-history-{}", uuid::Uuid::new_v4()));
        let store = SaveStore::new(&path);
        let mut runtime = WorldRuntime::new();
        let pond = runtime.create_pond("pond", 2729).unwrap();
        runtime.save(&store, "pond").unwrap();
        let legacy = runtime.load(&store, "pond").unwrap().snapshot;
        assert!(runtime.world("pond").unwrap().saved.observation_history.is_empty());
        assert_eq!(pond.pond, legacy.pond);
        runtime.advance_ticks("pond", 100).unwrap();
        let paused = runtime.set_edit_paused("pond", true).unwrap();
        let dry = runtime.edit_pond_command("pond", "drain", &paused.run_generation, &paused.edit_revision,
            PondEdit::DrainWater { basin_id: "pond-main".to_owned(), liters: paused.pond.unwrap().basins[0].water_liters }).unwrap();
        let formal = runtime.observation_history("pond", &dry.run_generation).unwrap();
        let discard = runtime.start_trial("pond").unwrap();
        runtime.advance_trial(&discard.world_id, 100).unwrap();
        runtime.discard_trial(&discard.world_id).unwrap();
        assert_eq!(runtime.observation_history("pond", &dry.run_generation).unwrap(), formal);

        let trial = runtime.start_trial("pond").unwrap();
        runtime.advance_trial(&trial.world_id, 100).unwrap();
        let branch = runtime.observation_history(&trial.world_id, &trial.run_generation).unwrap();
        let Reading::Pond(earlier) = &branch.recent[1].reading else { panic!("pond readings") };
        let Reading::Pond(drained) = &branch.recent[2].reading else { panic!("pond readings") };
        assert!(earlier[0].oxygen_mg_per_liter.unwrap() > 0.0);
        assert_eq!(drained[0].oxygen_mg_per_liter, None);
        assert_eq!(drained[0].water_liters, 0.0);
        let adopted = runtime.apply_trial(&store, &trial.world_id).unwrap();
        assert_eq!(runtime.observation_history("pond", &adopted.run_generation).unwrap(), branch);
        let mut reopened = WorldRuntime::new();
        let loaded = reopened.load(&store, "pond").unwrap().snapshot;
        assert_eq!(reopened.observation_history("pond", &loaded.run_generation).unwrap(), branch);
        std::fs::remove_dir_all(path).unwrap();
    }
}

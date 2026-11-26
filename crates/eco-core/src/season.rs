use crate::STEP_MILLIS;
use serde::{Deserialize, Serialize};
use std::f64::consts::TAU;

pub(crate) const TICKS_PER_DAY: u64 = 86_400_000 / STEP_MILLIS;
const DAYS_PER_YEAR: u64 = 365;
const TICKS_PER_YEAR: u64 = TICKS_PER_DAY * DAYS_PER_YEAR;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Season {
    Winter,
    Spring,
    Summer,
    Autumn,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeasonObservation {
    pub year: u64,
    pub day_of_year: u16,
    pub season: Season,
    pub air_temperature_celsius: f64,
    /// Ambient relative humidity as a fraction of saturation.
    pub relative_humidity: f64,
}

/// An experimental temperate climate indexed solely by accumulated simulation ticks.
/// The world starts on the first day of a fixed 365-day year.
pub fn observe(tick: u64) -> SeasonObservation {
    let within_year = tick % TICKS_PER_YEAR;
    let day_of_year = (within_year / TICKS_PER_DAY + 1) as u16;
    let season = match day_of_year {
        60..=151 => Season::Spring,
        152..=243 => Season::Summer,
        244..=334 => Season::Autumn,
        _ => Season::Winter,
    };
    let annual_phase = within_year as f64 / TICKS_PER_YEAR as f64;
    let warmth = ((annual_phase - 0.55) * TAU).cos();
    let humidity = ((annual_phase - 0.31) * TAU).cos();
    SeasonObservation {
        year: tick / TICKS_PER_YEAR + 1,
        day_of_year,
        season,
        air_temperature_celsius: 14.0 + 9.0 * warmth,
        relative_humidity: 0.72 + 0.10 * humidity,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_calendar_switches_at_day_boundaries() {
        for (day, previous, next) in [
            (60, Season::Winter, Season::Spring),
            (152, Season::Spring, Season::Summer),
            (244, Season::Summer, Season::Autumn),
            (335, Season::Autumn, Season::Winter),
        ] {
            let boundary = (day - 1) * TICKS_PER_DAY;
            assert_eq!(observe(boundary - 1).season, previous);
            let current = observe(boundary);
            assert_eq!((current.year, u64::from(current.day_of_year), current.season), (1, day, next));
        }
    }

    #[test]
    fn annual_climate_is_continuous_and_repeatable() {
        let previous = observe(TICKS_PER_YEAR - 1);
        let next = observe(TICKS_PER_YEAR);
        assert_eq!((previous.year, previous.day_of_year, previous.season), (1, 365, Season::Winter));
        assert_eq!((next.year, next.day_of_year, next.season), (2, 1, Season::Winter));
        assert!((previous.air_temperature_celsius - next.air_temperature_celsius).abs() < 1e-5);
        assert!((previous.relative_humidity - next.relative_humidity).abs() < 1e-6);
        assert_eq!(next.air_temperature_celsius, observe(0).air_temperature_celsius);
        assert_eq!(next.relative_humidity, observe(0).relative_humidity);
        let far_future = observe(u64::MAX);
        assert!((1..=365).contains(&far_future.day_of_year));
        assert!((5.0..=23.0).contains(&far_future.air_temperature_celsius));
        assert!((0.62..=0.82).contains(&far_future.relative_humidity));
    }
}

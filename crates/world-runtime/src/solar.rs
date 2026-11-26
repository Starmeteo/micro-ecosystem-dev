//! Location-based daylight from an explicitly supplied UTC timestamp.
//!
//! The solar declination and equation of time follow NOAA GML's Meeus-based
//! calculator. Rise and set use the U.S. Naval Observatory's conventional
//! geometric center altitude at a level horizon, without local outdoor data.
//! <https://gml.noaa.gov/grad/solcalc/calcdetails.html>
//! <https://gml.noaa.gov/grad/solcalc/main.js>
//! <https://aa.usno.navy.mil/faq/RST_defs>

use serde::{Deserialize, Serialize};

pub const SOLAR_MODEL_VERSION: u16 = 1;

const DAY_SECONDS: f64 = 86_400.0;
const SAMPLE_INTERVAL_SECONDS: f64 = 900.0;
const DAILY_INTERVALS: i32 = 96;
const RISE_SET_ALTITUDE_DEGREES: f64 = -0.833_3;
const MIN_UTC_SECONDS: i64 = -5_364_662_400; // 1800-01-01T00:00:00Z
const MAX_UTC_SECONDS: i64 = 4_133_980_800; // 2101-01-01T00:00:00Z (exclusive)

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SolarLocation {
    pub latitude_degrees: f64,
    pub longitude_degrees: f64,
    pub name: String,
}

impl SolarLocation {
    pub fn beijing_sample() -> Self {
        Self {
            latitude_degrees: 39.9042,
            longitude_degrees: 116.4074,
            name: "北京（示例地点）".into(),
        }
    }

    pub fn validate(&self) -> bool {
        !self.name.trim().is_empty()
            && self.name.chars().count() <= 80
            && !self.name.chars().any(char::is_control)
            && (-90.0..=90.0).contains(&self.latitude_degrees)
            && (-180.0..=180.0).contains(&self.longitude_degrees)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SolarDayKind {
    RiseSet,
    ContinuousDay,
    ContinuousNight,
    SingleCrossing,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SolarObservation {
    /// The panorama's circular lighting target: rise .25, noon .50, set .75.
    pub target_phase: f64,
    pub solar_altitude_degrees: f64,
    pub sunrise_utc_seconds: Option<i64>,
    pub sunset_utc_seconds: Option<i64>,
    pub day_kind: SolarDayKind,
}

/// Use the date at the longitude's mean solar time (UTC + four minutes per
/// eastward degree). Event timestamps remain UTC, so an event can display on
/// a different calendar date in Beijing or any other chosen clock zone.
pub fn observe(location: &SolarLocation, utc_seconds: i64) -> Option<SolarObservation> {
    if !location.validate() || !(MIN_UTC_SECONDS..MAX_UTC_SECONDS).contains(&utc_seconds) {
        return None;
    }

    let site = Site::new(location);
    let utc = utc_seconds as f64;
    let apparent = site.apparent_at(utc);
    let mean_local_day = ((utc + site.longitude_seconds) / DAY_SECONDS).floor();
    let day_start = mean_local_day * DAY_SECONDS - site.longitude_seconds;
    let (sunrise, sunset) = site.rise_and_set(day_start);
    let day_kind = match (sunrise, sunset) {
        (Some(_), Some(_)) => SolarDayKind::RiseSet,
        (Some(_), None) | (None, Some(_)) => SolarDayKind::SingleCrossing,
        (None, None) if apparent.horizon_distance() > 0.0 => SolarDayKind::ContinuousDay,
        (None, None) => SolarDayKind::ContinuousNight,
    };

    Some(SolarObservation {
        target_phase: apparent.phase(),
        solar_altitude_degrees: apparent.sin_altitude().clamp(-1.0, 1.0).asin().to_degrees(),
        sunrise_utc_seconds: sunrise,
        sunset_utc_seconds: sunset,
        day_kind,
    })
}

struct Site {
    sin_latitude: f64,
    cos_latitude: f64,
    longitude_seconds: f64,
}

impl Site {
    fn new(location: &SolarLocation) -> Self {
        let (sin_latitude, cos_latitude) = location.latitude_degrees.to_radians().sin_cos();
        Self {
            sin_latitude,
            cos_latitude,
            longitude_seconds: 240.0 * location.longitude_degrees,
        }
    }

    fn apparent_at(&self, utc: f64) -> ApparentSun {
        let orbit = SolarOrbit::at(utc);
        let apparent_seconds = utc.rem_euclid(DAY_SECONDS)
            + self.longitude_seconds
            + orbit.equation_of_time_seconds;
        let hour_angle = std::f64::consts::TAU
            * (apparent_seconds.rem_euclid(DAY_SECONDS) / DAY_SECONDS - 0.5);
        ApparentSun {
            hour_angle,
            mean_altitude_sine: self.sin_latitude * orbit.declination.sin(),
            daily_amplitude: self.cos_latitude * orbit.declination.cos(),
        }
    }

    fn horizon_distance(&self, utc: f64) -> f64 {
        self.apparent_at(utc).horizon_distance()
    }

    fn rise_and_set(&self, start: f64) -> (Option<i64>, Option<i64>) {
        let end = start + DAY_SECONDS;
        // Include neighboring samples to detect a brief high-latitude dip
        // just inside a date boundary. Refine discrete extrema before looking
        // for crossings so a short day/night need not contain a coarse sample.
        let samples: Vec<_> = (-1..=DAILY_INTERVALS + 1)
            .map(|index| {
                let utc = start + f64::from(index) * SAMPLE_INTERVAL_SECONDS;
                (utc, self.horizon_distance(utc))
            })
            .collect();
        let mut points = samples[1..samples.len() - 1].to_vec();
        for triple in samples.windows(3) {
            let (before, a) = triple[0];
            let (_, b) = triple[1];
            let (after, c) = triple[2];
            let maximum = b >= a && b >= c && (b > a || b > c);
            let minimum = b <= a && b <= c && (b < a || b < c);
            if maximum || minimum {
                let extremum = self.refine_extremum(before, after, maximum);
                if (start..end).contains(&extremum) {
                    points.push((extremum, self.horizon_distance(extremum)));
                }
            }
        }
        points.sort_unstable_by(|left, right| left.0.total_cmp(&right.0));

        let mut sunrise = None;
        let mut sunset = None;
        for pair in points.windows(2) {
            let above_before = pair[0].1 > 0.0;
            if above_before == (pair[1].1 > 0.0) {
                continue;
            }
            let utc = self.find_crossing(pair[0].0, pair[1].0, above_before);
            if utc >= end {
                continue;
            }
            let target = if above_before { &mut sunset } else { &mut sunrise };
            target.get_or_insert(utc.round() as i64);
        }
        (sunrise, sunset)
    }

    fn refine_extremum(&self, mut lo: f64, mut hi: f64, maximum: bool) -> f64 {
        for _ in 0..28 {
            let third = (hi - lo) / 3.0;
            let left = lo + third;
            let right = hi - third;
            let increasing = self.horizon_distance(left) < self.horizon_distance(right);
            if increasing == maximum {
                lo = left;
            } else {
                hi = right;
            }
        }
        (lo + hi) / 2.0
    }

    fn find_crossing(&self, mut lo: f64, mut hi: f64, above_before: bool) -> f64 {
        for _ in 0..20 {
            let middle = (lo + hi) / 2.0;
            if (self.horizon_distance(middle) > 0.0) == above_before {
                lo = middle;
            } else {
                hi = middle;
            }
        }
        (lo + hi) / 2.0
    }
}

struct ApparentSun {
    hour_angle: f64,
    mean_altitude_sine: f64,
    daily_amplitude: f64,
}

impl ApparentSun {
    fn sin_altitude(&self) -> f64 {
        self.mean_altitude_sine + self.daily_amplitude * self.hour_angle.cos()
    }

    fn horizon_distance(&self) -> f64 {
        self.sin_altitude() - RISE_SET_ALTITUDE_DEGREES.to_radians().sin()
    }

    fn phase(&self) -> f64 {
        let offset = RISE_SET_ALTITUDE_DEGREES.to_radians().sin() - self.mean_altitude_sine;
        // Compare before dividing. At the geographic poles the daily altitude
        // amplitude approaches zero; the scene uses a stable day/night target.
        if offset <= -self.daily_amplitude {
            return 0.5;
        }
        if offset >= self.daily_amplitude {
            return 0.0;
        }
        let horizon_ratio = (offset / self.daily_amplitude)
            .clamp(-1.0 + f64::EPSILON, 1.0 - f64::EPSILON);
        let horizon_angle = horizon_ratio.acos();
        let angle = self.hour_angle.abs();
        let distance_from_noon = if angle <= horizon_angle {
            0.25 * angle / horizon_angle
        } else {
            0.25 + 0.25 * (angle - horizon_angle) / (std::f64::consts::PI - horizon_angle)
        };
        (0.5 + self.hour_angle.signum() * distance_from_noon).rem_euclid(1.0)
    }
}

struct SolarOrbit {
    declination: f64,
    equation_of_time_seconds: f64,
}

impl SolarOrbit {
    fn at(utc_seconds: f64) -> Self {
        let julian_date = utc_seconds / DAY_SECONDS + 2_440_587.5;
        let t = (julian_date - 2_451_545.0) / 36_525.0;
        let mean_longitude = (280.46646 + t * (36_000.76983 + t * 0.000_303_2))
            .rem_euclid(360.0)
            .to_radians();
        let anomaly = (357.52911 + t * (35_999.05029 - 0.000_153_7 * t)).to_radians();
        let eccentricity = 0.016_708_634 - t * (0.000_042_037 + 0.000_000_126_7 * t);
        let center = (1.914602 - t * (0.004817 + 0.000014 * t)) * anomaly.sin()
            + (0.019993 - 0.000101 * t) * (2.0 * anomaly).sin()
            + 0.000289 * (3.0 * anomaly).sin();
        let omega = (125.04 - 1934.136 * t).to_radians();
        let apparent_longitude = mean_longitude
            + (center - 0.00569 - 0.00478 * omega.sin()).to_radians();
        let obliquity_arcseconds = 21.448 - t * (46.815 + t * (0.00059 - 0.001813 * t));
        let mean_obliquity = 23.0 + (26.0 + obliquity_arcseconds / 60.0) / 60.0;
        let obliquity = (mean_obliquity + 0.00256 * omega.cos()).to_radians();
        let y = (obliquity / 2.0).tan().powi(2);
        let equation = y * (2.0 * mean_longitude).sin()
            - 2.0 * eccentricity * anomaly.sin()
            + 4.0 * eccentricity * y * anomaly.sin() * (2.0 * mean_longitude).cos()
            - 0.5 * y * y * (4.0 * mean_longitude).sin()
            - 1.25 * eccentricity * eccentricity * (2.0 * anomaly).sin();
        Self {
            declination: (obliquity.sin() * apparent_longitude.sin()).asin(),
            equation_of_time_seconds: 240.0 * equation.to_degrees(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn location(latitude_degrees: f64, longitude_degrees: f64) -> SolarLocation {
        SolarLocation { latitude_degrees, longitude_degrees, name: "测试地点".into() }
    }

    fn near_event(actual: Option<i64>, expected: i64) {
        // UTC reference values from NOAA GML's two-pass calculator equations.
        let actual = actual.expect("expected an event");
        assert!(actual.abs_diff(expected) < 120, "UTC event {actual} differs from {expected}");
    }

    #[test]
    fn beijing_horizon_times_and_phase_match_the_utc_reference() {
        let location = SolarLocation::beijing_sample();
        let noon = observe(&location, 1_795_492_800).unwrap(); // 2026-11-24 04:00 UTC
        assert_eq!(noon.day_kind, SolarDayKind::RiseSet);
        near_event(noon.sunrise_utc_seconds, 1_795_475_333); // Nov 23 23:08:53 UTC
        near_event(noon.sunset_utc_seconds, 1_795_510_361); // Nov 24 08:52:41 UTC
        assert!((noon.target_phase - 0.5).abs() < 0.01);
        let rise = observe(&location, noon.sunrise_utc_seconds.unwrap()).unwrap();
        let set = observe(&location, noon.sunset_utc_seconds.unwrap()).unwrap();
        assert!((rise.target_phase - 0.25).abs() < 0.001);
        assert!((set.target_phase - 0.75).abs() < 0.001);
        assert!((rise.solar_altitude_degrees - RISE_SET_ALTITUDE_DEGREES).abs() < 0.01);
    }

    #[test]
    fn western_longitude_sunset_can_be_on_the_next_utc_date() {
        let location = location(37.7749, -122.4194);
        let morning = observe(&location, 1_795_528_800).unwrap(); // Nov 24 14:00 UTC
        near_event(morning.sunrise_utc_seconds, 1_795_532_372);
        near_event(morning.sunset_utc_seconds, 1_795_567_983); // Nov 25 00:53:03 UTC
        assert!((morning.sunrise_utc_seconds.unwrap() / 86_400) < (morning.sunset_utc_seconds.unwrap() / 86_400));
    }

    #[test]
    fn leap_day_and_circular_midnight_stay_continuous() {
        let location = location(0.0, 0.0);
        let leap_day = observe(&location, 1_709_208_000).unwrap();
        near_event(leap_day.sunrise_utc_seconds, 1_709_186_946);
        near_event(leap_day.sunset_utc_seconds, 1_709_230_544);
        let before = observe(&location, 1_795_478_399).unwrap();
        let after = observe(&location, 1_795_478_400).unwrap();
        let delta = (after.target_phase - before.target_phase + 0.5).rem_euclid(1.0) - 0.5;
        assert!(delta.abs() < 0.0001);
        assert_ne!(before.sunrise_utc_seconds, after.sunrise_utc_seconds);
    }

    #[test]
    fn international_date_line_does_not_shift_utc_events_by_a_day() {
        let utc = 1_795_478_400; // 2026-11-24 00:00 UTC
        let east = observe(&location(0.0, 179.9), utc).unwrap();
        let west = observe(&location(0.0, -179.9), utc).unwrap();
        near_event(east.sunrise_utc_seconds, 1_795_455_797); // Nov 23 17:43:17 UTC
        near_event(east.sunset_utc_seconds, 1_795_499_433);
        let events_apart = east.sunrise_utc_seconds.unwrap().abs_diff(west.sunrise_utc_seconds.unwrap());
        assert!(events_apart < 90);
        assert!((east.solar_altitude_degrees - west.solar_altitude_degrees).abs() < 0.3);
        assert!((east.target_phase - west.target_phase).abs() < 0.005);
    }

    #[test]
    fn polar_days_and_nights_have_no_invented_horizon_crossings() {
        for latitude in [69.6492, 90.0] {
            let location = location(latitude, 18.9553);
            let summer = observe(&location, 1_782_043_200).unwrap(); // 2026-06-21 noon UTC
            let winter = observe(&location, 1_797_854_400).unwrap(); // 2026-12-21 noon UTC
            assert_eq!(summer.day_kind, SolarDayKind::ContinuousDay);
            assert_eq!(winter.day_kind, SolarDayKind::ContinuousNight);
            assert!(summer.sunrise_utc_seconds.is_none() && summer.sunset_utc_seconds.is_none());
            assert!(winter.sunrise_utc_seconds.is_none() && winter.sunset_utc_seconds.is_none());
            assert_eq!(summer.target_phase, 0.5);
            assert_eq!(winter.target_phase, 0.0);
        }
    }

    #[test]
    fn rejects_non_finite_coordinates_and_out_of_range_timestamps() {
        assert!(observe(&location(f64::NAN, 0.0), 0).is_none());
        assert!(observe(&location(0.0, 180.01), 0).is_none());
        assert!(observe(&location(0.0, 0.0), i64::MAX).is_none());
    }
}

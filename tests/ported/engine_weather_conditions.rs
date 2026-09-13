/// But it does change once its time is up, or the sky is fixed for the night.
#[test]
fn weather_turns_over_when_its_duration_is_up() {
    let season = nightmare_shift::data::Season::default();
    let front = WeatherCondition {
        weather_type: WeatherType::Rain,
        intensity: WeatherIntensity::Moderate,
        visibility: 60,
        description: "Steady rain".to_string(),
        effects: Vec::new(),
        duration: 10,
        start_time: 0.0,
    };

    // Well past ten minutes. Either the front turns into different weather
    // or it shifts intensity, and either way the clock restarts.
    let past = front.duration as f64 * 60.0 + 1.0;
    let mut rng = macroquad_toolkit::rng::SeededRng::new(0xA11CE);
    let mut turned = 0;
    for run in 0..200 {
        let next = WeatherService::update_weather(&mut rng, &front, &season, past + run as f64);
        if next.intensity != front.intensity || next.weather_type != front.weather_type {
            turned += 1;
        }
        assert!(
            next.start_time >= past,
            "the clock did not restart, so this front will re-roll every frame"
        );
    }
    assert!(
        turned > 0,
        "a front outlived its duration two hundred times and never turned over"
    );
}

/// Weather does not change several times a second.
///
/// `update_weather` is called from the frame loop and rolled a one-in-ten
/// chance of an intensity change on every call -- about six a second at sixty
/// frames. Intensity drives visibility, the weather effects that price a
/// route, passenger spawn weighting and hazard generation, so all of it
/// jittered.
///
/// Deliberately not started from clear weather. `get_random_intensity`
/// returns Light unconditionally for Clear, so a test that begins there can
/// never observe a change and passes whatever the code does -- which is what
/// the first version of this test did.
#[test]
fn weather_holds_still_between_frames() {
    let season = nightmare_shift::data::Season::default();
    let front = WeatherCondition {
        weather_type: WeatherType::Rain,
        intensity: WeatherIntensity::Moderate,
        visibility: 60,
        description: "Steady rain".to_string(),
        effects: Vec::new(),
        duration: 90,
        start_time: 0.0,
    };
    assert_ne!(
        front.weather_type,
        WeatherType::Clear,
        "clear skies cannot change intensity, so this would prove nothing"
    );

    let mut weather = front;
    let mut rng = macroquad_toolkit::rng::SeededRng::new(0xB0B);
    let mut changes = 0;
    for frame in 1..=600 {
        let next =
            WeatherService::update_weather(&mut rng, &weather, &season, frame as f64 * 0.016);
        if next.intensity != weather.intensity {
            changes += 1;
        }
        weather = next;
    }

    assert_eq!(
        changes, 0,
        "the sky changed intensity {changes} times inside ten seconds"
    );
}

use super::*;
use nightmare_shift::data::loader::{load_constants, load_passengers};
use nightmare_shift::engine::{RouteCosts, RouteService, SkillModifiers};
use std::collections::HashMap;

/// Route costs under a given sky, everything else held equal.
///
/// The condition is built here rather than through
/// `create_weather_condition`, which rolls a random intensity — the point
/// is to compare the skies, so the intensity is pinned.
fn costs_under(weather_type: WeatherType) -> RouteCosts {
    let constants = load_constants();
    let intensity = WeatherIntensity::Moderate;
    let weather = WeatherCondition {
        weather_type,
        intensity,
        visibility: WeatherService::calculate_visibility(weather_type, intensity),
        description: String::new(),
        effects: WeatherService::get_weather_effects(weather_type, intensity),
        duration: 60,
        start_time: 0.0,
    };
    let passenger = load_passengers().into_iter().next();
    RouteService::calculate_route_costs(
        RouteType::Normal,
        &constants,
        2,
        Some(&weather),
        None,
        &[],
        &HashMap::new(),
        passenger.as_ref(),
        &SkillModifiers::default(),
    )
}

/// Clear weather must cost nothing extra, or there is no baseline to be
/// worse than.
#[test]
fn clear_weather_is_the_baseline() {
    let clear = costs_under(WeatherType::Clear);
    for worse in [
        WeatherType::Rain,
        WeatherType::Fog,
        WeatherType::Snow,
        WeatherType::Thunderstorm,
    ] {
        let costs = costs_under(worse);
        assert!(
            costs.fuel >= clear.fuel && costs.time >= clear.time,
            "{worse:?} is cheaper than clear: {costs:?} against {clear:?}"
        );
    }
}

/// The weather types must actually differ. Six skies that all cost the
/// same would make the forecast on the status bar decoration, and route
/// risk — which has real consequences — blind to it.
#[test]
fn the_skies_are_not_all_the_same() {
    let profiles: Vec<(WeatherType, RouteCosts)> = [
        WeatherType::Clear,
        WeatherType::Rain,
        WeatherType::Fog,
        WeatherType::Snow,
        WeatherType::Thunderstorm,
        WeatherType::Wind,
    ]
    .into_iter()
    .map(|kind| (kind, costs_under(kind)))
    .collect();

    let distinct: std::collections::HashSet<(u32, u32, u32)> = profiles
        .iter()
        .map(|(_, c)| (c.fuel, c.time, c.risk))
        .collect();

    assert!(
        distinct.len() >= 4,
        "only {} distinct cost profiles across six skies: {:?}",
        distinct.len(),
        profiles
            .iter()
            .map(|(k, c)| (format!("{k:?}"), c.fuel, c.time, c.risk))
            .collect::<Vec<_>>()
    );
}

/// A thunderstorm is the worst sky the game has — it is the only one that
/// draws the supernatural and unsettles the passenger — so it must not
/// price below a shower.
#[test]
fn a_thunderstorm_outweighs_a_shower() {
    let storm = costs_under(WeatherType::Thunderstorm);
    let rain = costs_under(WeatherType::Rain);
    assert!(
        storm.risk >= rain.risk,
        "a thunderstorm is no riskier than rain: {storm:?} against {rain:?}"
    );
}

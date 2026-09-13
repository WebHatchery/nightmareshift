use super::RouteService;
use nightmare_shift::data::loader::{load_constants, load_passengers, GameData};
use nightmare_shift::state::{GameState, PlayerStats, RelationshipLevel};

fn offer() -> (
    GameData,
    GameState,
    PlayerStats,
    nightmare_shift::data::Passenger,
) {
    let data = GameData::load().expect("embedded game data parses");
    let constants = load_constants();
    let state = GameState::new(0.0, &constants.game_constants);
    let passenger = load_passengers().into_iter().next().expect("a roster");
    (data, state, PlayerStats::new(), passenger)
}

/// The offer quotes a range, because the road is not chosen yet and the
/// four of them pay differently. A single number could only ever be right
/// for one road.
#[test]
fn the_four_roads_do_not_all_pay_the_same() {
    let (data, state, stats, passenger) = offer();
    let (low, high) = RouteService::fare_range(&passenger, &state, &data, &stats);
    assert!(low > 0, "the lowest road pays nothing");
    assert!(
        high > low,
        "every road paid {low}, so the spread the offer shows is fictional"
    );
}

/// Standing with a passenger reaches the quote. A Trusted fare pays more
/// than a stranger, and the offer said the same number for both.
#[test]
fn standing_raises_the_quoted_fare() {
    let (data, mut state, stats, passenger) = offer();
    let (_, stranger_high) = RouteService::fare_range(&passenger, &state, &data, &stats);

    let reputation = state.get_passenger_reputation(passenger.id);
    reputation.relationship_level = RelationshipLevel::Trusted;
    let (_, trusted_high) = RouteService::fare_range(&passenger, &state, &data, &stats);

    assert!(
        trusted_high > stranger_high,
        "a trusted fare quoted {trusted_high} against a stranger's {stranger_high}"
    );
}

/// And so do the fare skills, which is half of what the skill tree sells.
#[test]
fn the_fare_skills_raise_the_quote() {
    let (data, state, stats, passenger) = offer();
    let (_, plain) = RouteService::fare_range(&passenger, &state, &data, &stats);

    let mut skilled = PlayerStats::new();
    skilled.unlocked_skills = data
        .skills
        .iter()
        .filter(|skill| skill.effect.target == "fare_multiplier")
        .map(|skill| skill.id.clone())
        .collect();
    assert!(
        !skilled.unlocked_skills.is_empty(),
        "no fare skills authored"
    );
    let (_, boosted) = RouteService::fare_range(&passenger, &state, &data, &skilled);

    assert!(
        boosted > plain,
        "the fare skills quoted {boosted} against {plain}"
    );
}

/// The quote has to contain whatever the payout lands on.
///
/// This is the test that caught the first version of `fare_range`, which
/// called `calculate_fare` once per road. That function rolls a wobble of up
/// to five dollars either way on every call, so four calls were four samples
/// rather than four roads -- and since the range is built during drawing, the
/// number on screen would have changed every frame. It failed with "Scenic
/// pays 34, outside the quoted 5-33".
///
/// Repeated because the payout is random: one draw inside the range proves
/// nothing about the next.
#[test]
fn the_quote_contains_whatever_the_meter_lands_on() {
    use nightmare_shift::data::RouteType;
    use nightmare_shift::engine::GameEngine;

    let (data, state, stats, passenger) = offer();
    let (low, high) = RouteService::fare_range(&passenger, &state, &data, &stats);
    let destination_fare_modifier = data
        .get_location(&passenger.destination)
        .map(|location| location.fare_modifier)
        .unwrap_or(1.0);

    for _ in 0..400 {
        for route in [
            RouteType::Normal,
            RouteType::Shortcut,
            RouteType::Scenic,
            RouteType::Police,
        ] {
            let paid = GameEngine::calculate_fare(
                &mut macroquad_toolkit::rng::SeededRng::new(0xFA5E),
                passenger.fare,
                route,
                &passenger,
                None,
                None,
                &data.constants,
                destination_fare_modifier,
            );
            assert!(
                paid >= low && paid <= high,
                "{route:?} paid {paid}, outside the quoted {low}-{high}"
            );
        }
    }
}

/// And the quote itself does not move between frames, which is what calling
/// the rolling function per road would have caused.
#[test]
fn the_quote_is_the_same_every_time_it_is_asked() {
    let (data, state, stats, passenger) = offer();
    let first = RouteService::fare_range(&passenger, &state, &data, &stats);
    for _ in 0..200 {
        assert_eq!(
            RouteService::fare_range(&passenger, &state, &data, &stats),
            first,
            "the quoted fare changed between frames"
        );
    }
}

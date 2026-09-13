use nightmare_shift::data::loader::load_constants;

/// The quota a night asks for, as `begin_night` computes it.
fn quota_for(night: u32) -> u32 {
    let constants = load_constants();
    let base = constants.game_constants.minimum_earnings;
    let step = constants.game_constants.quota_increase_per_night;
    base + (base as f32 * step * (night - 1) as f32).round() as u32
}

/// A run must ask for more each night, or the campaign is five copies of
/// the same shift.
#[test]
fn the_quota_rises_every_night() {
    let constants = load_constants();
    let nights = constants.game_constants.nights_per_run.max(1);
    let mut previous = 0;
    for night in 1..=nights {
        let quota = quota_for(night);
        assert!(
            quota > previous,
            "night {night} asks {quota}, no more than night {}",
            night - 1
        );
        previous = quota;
    }
}

/// The final night must stay inside what a shift can physically earn.
/// Fuel and the clock do not grow across a run, so a quota that outruns
/// them makes the last night unwinnable rather than hard. The ceiling
/// here is deliberately generous — the point is to catch a step that has
/// been retuned into impossibility, not to pin the current balance.
#[test]
fn the_final_night_stays_within_reach() {
    let constants = load_constants();
    let nights = constants.game_constants.nights_per_run.max(1);
    let final_quota = quota_for(nights);

    // Cheapest leg, two legs a ride, against the clock a shift starts on.
    let game = &constants.game_constants;
    let cheapest_leg = game
        .time_cost_shortcut
        .min(game.time_cost_normal)
        .min(game.time_cost_scenic)
        .min(game.time_cost_police);
    let max_rides = game.initial_time / (cheapest_leg * 2).max(1);

    // The best-paying fare on the roster, times the best route
    // multiplier. `route_fares` are multipliers, not amounts — reading
    // them as money gave a ceiling of seventeen dollars a night.
    let fares = &constants.route_fares;
    let best_multiplier = fares
        .normal
        .max(fares.shortcut)
        .max(fares.scenic)
        .max(fares.police);
    let best_fare = nightmare_shift::data::loader::load_passengers()
        .iter()
        .map(|passenger| passenger.fare)
        .max()
        .unwrap_or(0);
    let ceiling = (max_rides as f32 * best_fare as f32 * best_multiplier).round() as u32;

    assert!(
        final_quota <= ceiling,
        "night {nights} asks {final_quota}, above the {ceiling} a shift could earn \
         at {max_rides} rides of {best_fare} times {best_multiplier}"
    );
}

/// Difficulty must climb across a run and must not exceed the cap the
/// rule generator clamps to, or the last nights are indistinguishable.
#[test]
fn difficulty_climbs_but_stays_within_the_cap() {
    let constants = load_constants();
    let step = constants.game_constants.difficulty_increase_per_night;
    assert!(step > 0, "a run never gets harder");
    assert!(
        step <= constants.scoring.max_difficulty,
        "one night's step {step} exceeds the {} cap",
        constants.scoring.max_difficulty
    );
}

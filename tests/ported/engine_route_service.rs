use nightmare_shift::data::loader::{load_constants, GameData};
use nightmare_shift::data::RouteType;
use nightmare_shift::engine::{RouteService, WeatherService};
use nightmare_shift::state::{GameState, PlayerStats};

/// The soft warning must land far enough ahead of the hard one to be
/// worth acting on, and both must clear the cost of a single route —
/// a warning that arrives with no affordable move left is just an
/// announcement that the shift is over.
#[test]
fn the_shift_end_warning_leaves_room_to_act() {
    let constants = load_constants();
    let timing = &constants.timing;
    assert!(
        timing.shift_end_warning_threshold > timing.critical_time_threshold,
        "warning at {} is not ahead of critical at {}",
        timing.shift_end_warning_threshold,
        timing.critical_time_threshold
    );

    let game = &constants.game_constants;
    let cheapest_route = game
        .time_cost_shortcut
        .min(game.time_cost_normal)
        .min(game.time_cost_scenic)
        .min(game.time_cost_police);
    assert!(
        timing.shift_end_warning_threshold > cheapest_route,
        "the warning at {} arrives with no route affordable (cheapest is {})",
        timing.shift_end_warning_threshold,
        cheapest_route
    );
}

/// Headlights trade a little fuel for a safer nighttime leg, and turning them
/// off must be visible in the same quote the route selector will charge.
#[test]
fn nighttime_headlights_change_the_route_quote() {
    let data = GameData::load().expect("embedded game data should load");
    let mut state = GameState::new(0.0, &data.constants.game_constants);
    state.time_of_day = WeatherService::get_time_of_day(22);
    let stats = PlayerStats::default();

    let lit = RouteService::quote_route(RouteType::Normal, &state, &data, &stats);
    state.headlights_on = false;
    let dark = RouteService::quote_route(RouteType::Normal, &state, &data, &stats);

    assert!(
        dark.fuel < lit.fuel,
        "dark route should avoid headlight load"
    );
    assert!(dark.risk > lit.risk, "dark route should expose more risk");
}

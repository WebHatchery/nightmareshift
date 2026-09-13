use super::*;
use nightmare_shift::data::loader::{load_constants, load_passengers, GameData};

fn driving_state(fuel: f32, time: u32) -> (GameState, GameData, PlayerStats) {
    let data = GameData::load().expect("embedded game data parses");
    let constants = load_constants();
    let mut state = GameState::new(0.0, &constants.game_constants);
    state.current_passenger = load_passengers().into_iter().next();
    state.fuel = fuel;
    state.time_remaining = time;
    (state, data, PlayerStats::new())
}

/// Driving a leg has to actually spend the follow cost of the passenger's
/// own rule.
///
/// Both halves of that were covered — the lookup finds the rule, and the
/// state machine spends what it is handed — and nothing covered the wire
/// between them. Replacing the argument at this call site with `None`, the
/// bug that existed before, left every test passing. So this drives a real
/// leg through `choose_route` twice: once with Mrs. Chen's own rule in
/// force and once with a rule that has nothing to do with her.
///
/// The RNG is pinned to the same seed for both legs. `choose_route` rolls
/// for risk encounters and route pressure, so without that the two legs
/// would differ for reasons that have nothing to do with the rule.
#[test]
fn the_passengers_own_rule_costs_more_than_an_unrelated_one() {
    use nightmare_shift::data::loader::{load_guidelines, load_rules};
    use nightmare_shift::engine::PassengerStateMachine;

    let level_after = |rule_id: u32| {
        let (mut state, data, mut stats) = driving_state(100.0, 480);
        let chen = load_passengers()
            .into_iter()
            .find(|p| p.id == 1)
            .expect("Mrs. Chen");
        state.current_passenger_need_state = PassengerStateMachine::initialize(&chen, 0.0);
        state.current_passenger = Some(chen);
        state.current_guidelines = load_guidelines();
        state.current_rules = load_rules()
            .into_iter()
            .filter(|rule| rule.id == rule_id)
            .collect();
        assert_eq!(state.current_rules.len(), 1, "rule {rule_id} not found");

        macroquad_toolkit::rng::srand(20260730);
        RideService::choose_route(&mut state, &data, &mut stats, RouteType::Normal, 0.0);
        state
            .current_passenger_need_state
            .as_ref()
            .expect("a need state")
            .level
    };

    // Rule 1 "No Eye Contact" is the one guideline 1001 owns, which is
    // where Chen's exception lives. Rule 4 "Windows Sealed" is not hers.
    let own = level_after(1);
    let unrelated = level_after(4);
    assert_ne!(
        own, unrelated,
        "her own rule being in force cost the same as an unrelated one ({own})"
    );
}

#[test]
fn normal_route_relief_is_applied_after_the_passengers_reaction() {
    use nightmare_shift::engine::PassengerStateMachine;

    let (mut relieved, data, _) = driving_state(100.0, 480);
    let passenger = relieved.current_passenger.clone().expect("a passenger");
    relieved.current_passenger_need_state = PassengerStateMachine::initialize(&passenger, 0.0);
    let mut without_relief = relieved.clone();
    let game = &data.constants.game_constants;

    RideService::update_passenger_state(
        &mut relieved,
        RouteType::Normal,
        1.0,
        game.route_preference_stress_scale,
        game.normal_route_need_relief,
    );
    RideService::update_passenger_state(
        &mut without_relief,
        RouteType::Normal,
        1.0,
        game.route_preference_stress_scale,
        0,
    );

    let actual = relieved.current_passenger_need_state.unwrap().level;
    let control = without_relief.current_passenger_need_state.unwrap().level;
    assert_eq!(control - actual, game.normal_route_need_relief);
}

/// A driver with the tank and the clock full is never stranded.
#[test]
fn a_full_tank_is_not_stranded() {
    let (state, data, stats) = driving_state(100.0, 480);
    assert!(!RideService::is_stranded(&state, &data, &stats));
}

/// With too little of either for any of the four routes, the shift has to
/// end — the driving screen offers no other action, so without this the
/// night sits on four disabled buttons and a clock that only moves when a
/// leg is driven.
#[test]
fn no_affordable_route_is_stranded() {
    let (state, data, stats) = driving_state(1.0, 1);
    assert!(
        RideService::is_stranded(&state, &data, &stats),
        "a driver who cannot pay for any route was not counted as stranded"
    );
}

/// Time alone is enough to strand: the cheapest route still costs
/// minutes, so a full tank does not help an empty clock.
#[test]
fn an_empty_clock_strands_a_full_tank() {
    let (state, data, stats) = driving_state(100.0, 1);
    assert!(RideService::is_stranded(&state, &data, &stats));
}

/// With nobody aboard there is no leg to make, so the waiting screen —
/// which can still refuel — must not be cut short.
#[test]
fn an_empty_cab_is_never_stranded() {
    let (mut state, data, stats) = driving_state(1.0, 1);
    state.current_passenger = None;
    assert!(!RideService::is_stranded(&state, &data, &stats));
}

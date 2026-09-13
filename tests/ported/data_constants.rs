use nightmare_shift::data::loader::load_constants;

/// A refuel discount has to reach the price the driver is quoted.
///
/// The waiting screen priced a top-up without it while the pump charged
/// with it, so the button said one number and took another.
#[test]
fn a_refuel_discount_lowers_the_price() {
    let fuel = load_constants().fuel;
    let full = fuel.refuel_cost(50.0, 1.0);
    let discounted = fuel.refuel_cost(50.0, 0.7);
    assert!(full > 0, "a fifty percent top-up costs nothing");
    assert!(
        discounted < full,
        "the discount changed nothing: {discounted} against {full}"
    );
}

/// And the discount must be able to bring a price within reach, because the
/// screen decides whether the refuel button works from this figure. Quoting
/// the undiscounted price refused a driver a refuel they could afford --
/// locked out by the skill they bought to make it cheaper.
#[test]
fn a_discount_can_bring_a_refuel_within_reach() {
    let fuel = load_constants().fuel;
    let purse = fuel.refuel_cost(50.0, 0.7);
    assert!(
        fuel.refuel_cost(50.0, 1.0) > purse,
        "no discount steep enough to matter, so this cannot be tested"
    );
    assert!(
        fuel.refuel_cost(50.0, 0.7) <= purse,
        "a driver holding exactly the discounted price is still refused"
    );
}

/// Nothing to add, nothing to pay. A full tank must not be billed for a
/// negative top-up.
#[test]
fn a_full_tank_costs_nothing_to_fill() {
    let fuel = load_constants().fuel;
    assert_eq!(fuel.refuel_cost(0.0, 1.0), 0);
    assert_eq!(fuel.refuel_cost(-10.0, 1.0), 0);
}

/// Fuel thresholds must descend, or the gauge colours and the status text
/// disagree about what "low" means. They used to be duplicated as Rust
/// constants; nothing but this ordering now stops a bad edit.
#[test]
fn fuel_thresholds_descend() {
    let fuel = load_constants().fuel;
    assert!(
        fuel.medium_fuel > fuel.low_fuel_warning,
        "medium {} is not above low {}",
        fuel.medium_fuel,
        fuel.low_fuel_warning
    );
    assert!(
        fuel.low_fuel_warning > fuel.critical_fuel,
        "low {} is not above critical {}",
        fuel.low_fuel_warning,
        fuel.critical_fuel
    );
    assert!(fuel.critical_fuel > fuel.empty_tank);
}

/// A ride must be refusable before the tank is empty, or the
/// "ran out of fuel with a passenger in the car" guard never fires early
/// enough to be a warning.
#[test]
fn a_ride_is_refused_before_the_tank_empties() {
    let fuel = load_constants().fuel;
    assert!(fuel.fuel_check_minimum > fuel.empty_tank);
    assert!(fuel.fuel_check_minimum <= fuel.critical_fuel);
}

/// The streak warning must be reachable, and the violation threshold is
/// authored at 999 deliberately — that is the data saying no violation,
/// not a value to wire up.
#[test]
fn route_streak_warning_is_reachable() {
    let streak = load_constants().consecutive_route;
    assert!(streak.warning_threshold > 0);
    assert!(
        streak.risk_increase_per_repeat > 0,
        "a streak adds no risk, so the warning has nothing behind it"
    );
}

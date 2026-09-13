use super::*;

fn hazard(effects: HazardEffects) -> EnvironmentalHazard {
    EnvironmentalHazard {
        id: "test".to_string(),
        hazard_type: HazardType::Construction,
        location: "Downtown Bridge".to_string(),
        severity: HazardSeverity::Minor,
        description: "Minor road work on Downtown Bridge".to_string(),
        effects,
        duration: 30,
        start_time: 0.0,
        weather_triggered: false,
    }
}

/// A hazard that closes a route has to say which one. The route cards on
/// the driving screen disable themselves silently otherwise, and by then
/// the night is already underway.
#[test]
fn a_closure_names_the_route_it_closes() {
    let blocked = hazard(HazardEffects {
        route_blocked: Some(vec![RouteType::Shortcut]),
        ..HazardEffects::default()
    });
    let toll = blocked.toll(1.0).expect("a closure costs something");
    assert!(
        toll.contains("Shortcut"),
        "{toll:?} does not name the route"
    );
}

/// Every surcharge the hazard applies to route pricing has to appear, or
/// the briefing understates the night.
#[test]
fn every_surcharge_reaches_the_briefing() {
    let costly = hazard(HazardEffects {
        fuel_increase: Some(4),
        time_delay: Some(7),
        risk_increase: Some(2),
        ..HazardEffects::default()
    });
    let toll = costly.toll(1.0).expect("surcharges cost something");
    for expected in ["4", "7", "2"] {
        assert!(toll.contains(expected), "{toll:?} is missing {expected}");
    }
}

/// A driver with hazard resistance is quoted the softer figure, because
/// that is the one route pricing will charge them.
#[test]
fn hazard_resistance_softens_the_forecast() {
    let costly = hazard(HazardEffects {
        fuel_increase: Some(8),
        time_delay: Some(10),
        risk_increase: Some(2),
        ..HazardEffects::default()
    });

    let unskilled = costly.toll(1.0).expect("a toll");
    let skilled = costly.toll(0.5).expect("a toll");
    assert_ne!(
        unskilled, skilled,
        "hazard resistance changed nothing about the forecast"
    );
    assert!(unskilled.contains("10 min") && skilled.contains("5 min"));
    assert!(unskilled.contains("8 fuel") && skilled.contains("4 fuel"));
}

/// Resistance strong enough to erase a surcharge stops quoting it, rather
/// than printing "+0 min".
#[test]
fn a_surcharge_reduced_to_nothing_is_not_quoted() {
    let slight = hazard(HazardEffects {
        time_delay: Some(1),
        ..HazardEffects::default()
    });
    assert!(slight.toll(1.0).is_some());
    assert!(
        slight.toll(0.1).is_none(),
        "a surcharge rounded away was still quoted"
    );
}

/// A closure is not a surcharge and resistance does not open the road.
#[test]
fn resistance_does_not_reopen_a_closed_route() {
    let blocked = hazard(HazardEffects {
        route_blocked: Some(vec![RouteType::Shortcut]),
        ..HazardEffects::default()
    });
    let toll = blocked.toll(0.1).expect("a closure still costs something");
    assert!(toll.contains("Shortcut"));
}

/// A hazard with nothing behind it says nothing, rather than printing an
/// empty pair of brackets after its description.
#[test]
fn a_toothless_hazard_has_no_toll() {
    assert!(hazard(HazardEffects::default()).toll(1.0).is_none());
    let zeroed = hazard(HazardEffects {
        route_blocked: Some(Vec::new()),
        fuel_increase: Some(0),
        time_delay: Some(0),
        risk_increase: Some(0),
        ..HazardEffects::default()
    });
    assert!(zeroed.toll(1.0).is_none(), "zeroes were reported as costs");
}

/// The display label and the persisted save key are separate tables that
/// happen to agree. This pins that: if a route is ever renamed on screen
/// this fails, which is the moment to decide whether saves migrate.
#[test]
fn the_route_label_still_matches_the_persisted_key() {
    for route in [
        RouteType::Normal,
        RouteType::Shortcut,
        RouteType::Scenic,
        RouteType::Police,
    ] {
        assert_eq!(
            route.label(),
            nightmare_shift::state::PlayerStats::route_key(route),
            "the on-screen name and the save key have diverged"
        );
    }
}

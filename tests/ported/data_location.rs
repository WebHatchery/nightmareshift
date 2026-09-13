use nightmare_shift::data::loader::{load_constants, load_locations, load_passengers};

/// Every location must describe itself. The atmosphere is shown on the
/// ride request next to the risk it explains, so a blank one leaves the
/// player a bare risk number with no reason attached.
#[test]
fn every_location_has_an_atmosphere() {
    for location in load_locations() {
        assert!(
            !location.atmosphere.trim().is_empty(),
            "{} has no atmosphere",
            location.name
        );
    }
}

/// Risk levels must sit inside the range the risk constants describe, or
/// the colouring on the ride request means nothing.
#[test]
fn location_risk_sits_within_the_authored_scale() {
    let risk = load_constants().risk;
    for location in load_locations() {
        assert!(
            location.risk_level >= risk.safe,
            "{} is below the safe floor",
            location.name
        );
        assert!(
            location.risk_level <= risk.max_risk_level + 1,
            "{} at risk {} exceeds the authored scale",
            location.name,
            location.risk_level
        );
    }
}

#[test]
fn locations_author_route_and_spawn_modifiers() {
    for location in load_locations() {
        assert!(location.distance_multiplier > 0.0);
        assert!(location.fuel_multiplier > 0.0);
        assert!(location.spawn_affinity > 0.0);
        assert!(location.destination_risk >= 0.0);
    }
}

/// Every pickup and destination a passenger names must be a real
/// location, or the ride request shows a route to nowhere and the
/// pickup's risk silently defaults.
#[test]
fn every_passenger_route_names_real_locations() {
    let locations = load_locations();
    let known: Vec<&str> = locations.iter().map(|l| l.name.as_str()).collect();
    for passenger in load_passengers() {
        assert!(
            known.contains(&passenger.pickup.as_str()),
            "{} is picked up at unknown location {:?}",
            passenger.name,
            passenger.pickup
        );
        assert!(
            known.contains(&passenger.destination.as_str()),
            "{} is bound for unknown location {:?}",
            passenger.name,
            passenger.destination
        );
    }
}

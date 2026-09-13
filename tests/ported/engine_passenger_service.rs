use super::*;
use nightmare_shift::data::loader::load_passengers;

/// Every id a passenger names as a relation must be a real passenger, or
/// the kin pool silently skips it.
#[test]
fn every_relationship_names_a_real_passenger() {
    let passengers = load_passengers();
    let ids: Vec<u32> = passengers.iter().map(|p| p.id).collect();
    for passenger in &passengers {
        for id in &passenger.relationships {
            assert!(
                ids.contains(id),
                "{} names unknown relation {id}",
                passenger.name
            );
        }
        assert!(
            !passenger.relationships.contains(&passenger.id),
            "{} is related to itself",
            passenger.name
        );
    }
}

/// Carrying someone must make at least one associate available, in either
/// direction, or the mechanic never fires for that passenger.
#[test]
fn kin_is_found_in_both_directions() {
    let passengers = load_passengers();

    // Forward: Jake Morrison (2) names Dr. Hollow (4).
    let kin = PassengerService::kin_of(&[2], &passengers);
    assert!(kin.contains(&4), "forward link not followed: {kin:?}");

    // Backward: Mrs. Chen (1) names nobody, but Old Pete (10) names her.
    let kin = PassengerService::kin_of(&[1], &passengers);
    assert!(kin.contains(&10), "backward link not followed: {kin:?}");
}

/// Someone already carried this shift must never be offered again as kin.
#[test]
fn kin_excludes_passengers_already_carried() {
    let passengers = load_passengers();
    let met = [2, 4];
    for id in PassengerService::kin_of(&met, &passengers) {
        assert!(!met.contains(&id), "kin pool re-offered passenger {id}");
    }
}

/// With nobody carried yet there is no kin pool, so the first fare of a
/// shift is always drawn normally.
#[test]
fn no_kin_before_the_first_fare() {
    assert!(PassengerService::kin_of(&[], &load_passengers()).is_empty());
}

/// The relationship web must be broad enough to matter: most of the
/// roster should be reachable as someone's associate.
#[test]
fn most_of_the_roster_is_reachable_as_kin() {
    let passengers = load_passengers();
    let reachable: usize = passengers
        .iter()
        .filter(|p| !PassengerService::kin_of(&[p.id], &passengers).is_empty())
        .count();
    assert!(
        reachable * 2 > passengers.len(),
        "only {reachable} of {} passengers lead anywhere",
        passengers.len()
    );
}

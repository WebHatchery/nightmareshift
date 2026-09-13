use super::*;
use nightmare_shift::data::loader::load_constants;

/// Standing has to move the numbers the game reads, not just the tallies.
///
/// `relationship_level` is the only field anything consults — the fare
/// multiplier and the route risk modifier branch on it and nothing else.
/// Four call sites raised `positive_choices` and left the level alone, so
/// offering the withered flowers changed nothing until the ride ended.
#[test]
fn a_gift_moves_the_level_the_fare_reads() {
    let constants = load_constants().reputation;
    let mut reputation = PassengerReputation::default();
    assert_eq!(reputation.relationship_level, RelationshipLevel::Neutral);
    let before = reputation.fare_multiplier(&constants);

    reputation.adjust(2, &constants);

    assert_ne!(reputation.relationship_level, RelationshipLevel::Neutral);
    assert!(
        reputation.fare_multiplier(&constants) > before,
        "a gift left the fare where it was"
    );
}

/// A gift is an interaction. Raising only the positive tally made a
/// passenger handed two gifts on their first ride read as 2/1 positive,
/// which is not a ratio.
#[test]
fn a_gift_counts_as_an_interaction() {
    let constants = load_constants().reputation;
    let mut reputation = PassengerReputation::default();
    reputation.adjust(2, &constants);

    assert_eq!(reputation.interactions, 2);
    assert!(
        reputation.positive_choices <= reputation.interactions,
        "{} positive choices out of {} interactions",
        reputation.positive_choices,
        reputation.interactions
    );
}

/// The same path has to work downwards, or a rule's negative consequence
/// costs the player nothing.
#[test]
fn a_slight_moves_it_the_other_way() {
    let constants = load_constants().reputation;
    let mut reputation = PassengerReputation::default();
    reputation.adjust(-2, &constants);

    assert_eq!(reputation.relationship_level, RelationshipLevel::Hostile);
    assert!(
        reputation.risk_modifier(&constants) > 0,
        "a hostile passenger made the roads no more dangerous"
    );
}

/// Nothing at all is still nothing.
#[test]
fn an_empty_adjustment_changes_nothing() {
    let constants = load_constants().reputation;
    let mut reputation = PassengerReputation::default();
    reputation.adjust(0, &constants);

    assert_eq!(reputation.interactions, 0);
    assert_eq!(reputation.relationship_level, RelationshipLevel::Neutral);
}

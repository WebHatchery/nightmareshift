use nightmare_shift::data::loader::load_night_modifiers;
use std::collections::HashSet;

/// Every authored modifier must be well-formed: named, described, its
/// draw chance a probability, its multipliers positive (a zero fare or
/// quota multiplier is a broken economy, not a twist), and its fuel
/// delta survivable against the 100-point tank.
#[test]
fn the_modifier_deck_is_well_formed() {
    let deck = load_night_modifiers();
    assert!(
        (0.0..=1.0).contains(&deck.chance),
        "chance {} is not a probability",
        deck.chance
    );
    assert!(!deck.modifiers.is_empty(), "the deck is empty");

    let mut ids = HashSet::new();
    for modifier in &deck.modifiers {
        assert!(
            ids.insert(modifier.id.clone()),
            "duplicate modifier id {:?}",
            modifier.id
        );
        assert!(!modifier.name.trim().is_empty(), "{}: no name", modifier.id);
        assert!(
            !modifier.description.trim().is_empty(),
            "{}: no description",
            modifier.id
        );
        assert!(
            modifier.fare_mult > 0.0 && modifier.quota_mult > 0.0,
            "{}: a non-positive multiplier",
            modifier.id
        );
        assert!(
            modifier.start_fuel_delta > -90,
            "{}: fuel delta {} leaves no night to drive",
            modifier.id,
            modifier.start_fuel_delta
        );
        assert!(
            modifier.weight > 0,
            "{}: zero weight can never be drawn",
            modifier.id
        );
    }
}

/// A modifier must change something, or it is a name on the briefing
/// and nothing else.
#[test]
fn every_modifier_moves_a_lever() {
    for modifier in load_night_modifiers().modifiers {
        let moves = modifier.fare_mult != 1.0
            || modifier.quota_mult != 1.0
            || modifier.difficulty_bonus > 0
            || modifier.start_fuel_delta != 0
            || modifier.lore_bonus > 0;
        assert!(moves, "{}: authors no effect", modifier.id);
    }
}

/// The weighted draw always lands on an authored entry.
#[test]
fn the_roll_draws_from_the_deck() {
    let deck = load_night_modifiers();
    let mut rng = macroquad_toolkit::rng::SeededRng::new(0x0DDB);
    let mut drawn = 0;
    for _ in 0..200 {
        if let Some(modifier) = deck.roll(&mut rng) {
            drawn += 1;
            assert!(deck.modifiers.iter().any(|m| m.id == modifier.id));
        }
    }
    assert!(
        drawn > 0,
        "200 rolls at chance {} drew nothing",
        deck.chance
    );
}

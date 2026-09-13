use super::*;
use nightmare_shift::data::loader::{load_constants, load_passengers};

fn state_with_passenger() -> GameState {
    let constants = load_constants();
    let mut state = GameState::new(0.0, &constants.game_constants);
    let passenger = load_passengers()
        .into_iter()
        .find(|p| p.state_profile.is_some())
        .expect("a passenger with a profile");
    state.current_passenger_need_state = PassengerNeedState::from_passenger(&passenger, 0.0);
    state.current_passenger = Some(passenger);
    state
}

/// Both kinds of charge count, because the bar answers "is anything
/// protecting me" rather than "which ward do I hold".
#[test]
fn both_kinds_of_ward_count_toward_the_readout() {
    let constants = load_constants();
    let mut state = GameState::new(0.0, &constants.game_constants);
    assert_eq!(state.wards_in_hand(), 0);

    state.rule_immunity_charges = 2;
    assert_eq!(state.wards_in_hand(), 2);

    state.supernatural_protection = 1;
    assert_eq!(
        state.wards_in_hand(),
        3,
        "supernatural protection was left out of the readout"
    );
}

/// A carried ward counts too — `ProtectionService::consume_ward` spends
/// from the item itself, so a driver holding a Blessed Medallion is
/// protected even with both counters at zero, and the readout must say so.
#[test]
fn a_carried_ward_counts_toward_the_readout() {
    let constants = load_constants();
    let catalog = nightmare_shift::data::loader::load_item_catalog();
    let mut state = GameState::new(0.0, &constants.game_constants);
    assert_eq!(state.wards_in_hand(), 0);

    let medallion = catalog.create_item("Blessed Medallion", "Sister Agnes", 0.0);
    let charges = medallion.durability.unwrap_or(1).max(1);
    state.inventory.push(medallion);
    assert_eq!(
        state.wards_in_hand(),
        charges,
        "a carried ward's charges were left out of the readout"
    );

    let absorbed = nightmare_shift::engine::ProtectionService::consume_ward(
        &mut state.inventory,
        nightmare_shift::data::ProtectionType::SupernaturalImmunity,
        None,
    );
    assert!(absorbed.is_some(), "the medallion should absorb");
    assert_eq!(
        state.wards_in_hand(),
        charges - 1,
        "absorbing must spend a visible charge"
    );
}

/// Spending one is visible in the readout, which is the whole point --
/// both charges are decremented silently by the systems that consume them.
#[test]
fn spending_a_ward_lowers_the_readout() {
    let constants = load_constants();
    let mut state = GameState::new(0.0, &constants.game_constants);
    state.rule_immunity_charges = 1;
    state.supernatural_protection = 1;
    let before = state.wards_in_hand();

    state.rule_immunity_charges -= 1;

    assert_eq!(state.wards_in_hand(), before - 1);
}

/// Settling folds the banked trust in and leaves nothing behind, so
/// calling it every frame cannot apply the same escalation twice.
#[test]
fn settling_applies_once_and_drains() {
    let mut state = state_with_passenger();
    let before = state.player_trust;
    state
        .current_passenger_need_state
        .as_mut()
        .expect("a need state")
        .pending_trust = 0.1;

    state.settle_passenger_trust();
    let after = state.player_trust;
    assert!(after > before, "banked trust never reached the driver");

    state.settle_passenger_trust();
    assert_eq!(state.player_trust, after, "the same escalation paid twice");
}

/// Trust stays a probability. `calculate_detection_probability` multiplies
/// by it and compares it against `trustRequired`, so a value outside 0..1
/// would quietly break tell detection rather than fail loudly.
#[test]
fn settling_cannot_push_trust_out_of_range() {
    for (start, pending) in [(0.95_f32, 1.0_f32), (0.05, -1.0)] {
        let mut state = state_with_passenger();
        state.player_trust = start;
        state
            .current_passenger_need_state
            .as_mut()
            .expect("a need state")
            .pending_trust = pending;
        state.settle_passenger_trust();
        assert!(
            (0.0..=1.0).contains(&state.player_trust),
            "trust left the range: {}",
            state.player_trust
        );
    }
}

/// Something has to call it. Banking trust that is never drained is the
/// same bug as never banking it, and every test above passes with the
/// call in `Game::update` deleted.
///
/// This reads the frame loop's source, which is the only way to check a
/// call site that needs a window to run. The cost is real and this
/// project has paid it once: a scanning test broke on code motion when
/// `game.rs` was split, and the fix was repointing the path. If the frame
/// loop moves again, point this at wherever it went.
#[test]
fn the_frame_loop_settles_trust() {
    let frame_loop = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/game.rs"));
    assert!(
        frame_loop.contains("settle_passenger_trust()"),
        "nothing in the frame loop drains pending trust, so escalation \
         banks standing that never reaches the driver"
    );
}

/// With nobody aboard there is nothing to settle.
#[test]
fn settling_with_no_passenger_is_a_no_op() {
    let constants = load_constants();
    let mut state = GameState::new(0.0, &constants.game_constants);
    let before = state.player_trust;
    state.settle_passenger_trust();
    assert_eq!(state.player_trust, before);
}

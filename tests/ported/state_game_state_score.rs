use super::*;
use nightmare_shift::data::loader::load_constants;

fn shift(earnings: u32, rides: u32, time_left: u32, violations: u32) -> GameState {
    let constants = load_constants();
    let mut state = GameState::new(0.0, &constants.game_constants);
    state.earnings = earnings;
    state.rides_completed = rides;
    state.time_remaining = time_left;
    state.rules_violated = violations;
    state
}

/// A held decision keeps whatever is left on its clock.
///
/// The countdown is `30 - (now - start)`, so a pause that does not move
/// the start is not a pause at all — the timer ran behind the menu and
/// forced the choice. Pushing the start by the same amount as the wall
/// clock leaves the remaining time unchanged.
#[test]
fn holding_the_start_time_holds_the_countdown() {
    let remaining = |start: f64, now: f64| (30.0 - (now - start) as f32).max(0.0);

    let start = 100.0;
    let opened_at = 110.0;
    assert_eq!(remaining(start, opened_at), 20.0);

    // Six seconds spent reading the pause menu, the start pushed with it.
    let paused_for = 6.0;
    let held_start = start + paused_for;
    assert_eq!(remaining(held_start, opened_at + paused_for), 20.0);

    // Without the push those six seconds would have come off the clock.
    assert_eq!(remaining(start, opened_at + paused_for), 14.0);
}

/// The run bonus is what distinguishes finishing a campaign from
/// surviving another night, so the outcome screen keys its extra line on
/// this. A night's own banking must not trip it.
#[test]
fn only_a_run_bonus_counts_as_completing_a_run() {
    let nightly = MetaPayout {
        bank: 300,
        lore: 12,
        ..MetaPayout::default()
    };
    assert!(!nightly.completed_a_run());

    let finished = MetaPayout {
        bank: 300,
        lore: 12,
        run_bonus_bank: 1500,
        run_bonus_lore: 15,
    };
    assert!(finished.completed_a_run());
}

/// A fresh shift starts with nothing recorded, so a night cannot inherit
/// the previous one's payout on the screen that reports it.
#[test]
fn a_new_shift_clears_the_recorded_payout() {
    let constants = load_constants();
    let mut state = shift(400, 8, 100, 0);
    state.shift_payout = MetaPayout {
        bank: 200,
        lore: 9,
        run_bonus_bank: 1500,
        run_bonus_lore: 15,
    };
    state.reset_for_new_shift(0.0, &constants.game_constants);
    assert_eq!(state.shift_payout.bank, 0);
    assert_eq!(state.shift_payout.lore, 0);
    assert!(!state.shift_payout.completed_a_run());
}

/// A night that ended before it started must not outscore one that was
/// worked. The time bonus used to be paid regardless, so keeping the
/// whole clock by losing the first fare scored 960 — above real shifts
/// on the leaderboard.
#[test]
fn an_instant_loss_does_not_outscore_a_worked_night() {
    let constants = load_constants();
    let instant_loss = shift(0, 0, constants.game_constants.initial_time, 0);
    let worked = shift(200, 6, 90, 1);
    assert!(
        worked.calculate_score(&constants) > instant_loss.calculate_score(&constants),
        "a worked night scored {} against {} for losing immediately",
        worked.calculate_score(&constants),
        instant_loss.calculate_score(&constants)
    );
}

/// Falling short of the quota forfeits the time bonus entirely, so what
/// is left on the clock cannot carry a failed night.
#[test]
fn time_left_pays_only_when_the_quota_is_met() {
    let constants = load_constants();
    let quota = constants.game_constants.minimum_earnings;

    let short = shift(quota - 1, 3, 200, 0);
    let met = shift(quota, 3, 200, 0);
    let expected_step = constants.scoring.time_bonus_multiplier * 200 + 1;
    assert_eq!(
        met.calculate_score(&constants) - short.calculate_score(&constants),
        expected_step,
        "crossing the quota did not turn the time bonus on"
    );
}

/// Among nights that met the quota, finishing sooner still scores higher
/// — the bonus keeps the meaning it was added for.
#[test]
fn finishing_sooner_still_pays() {
    let constants = load_constants();
    let quota = constants.game_constants.minimum_earnings;
    let brisk = shift(quota + 50, 5, 180, 0);
    let slow = shift(quota + 50, 5, 40, 0);
    assert!(brisk.calculate_score(&constants) > slow.calculate_score(&constants));
}

/// Violations still cost, and cannot push a score below zero.
#[test]
fn violations_cost_without_underflowing() {
    let constants = load_constants();
    let reckless = shift(0, 0, 0, 99);
    assert_eq!(reckless.calculate_score(&constants), 0);
}

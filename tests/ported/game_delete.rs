use super::{DeleteDecision, Game};

/// A first press must never erase. This was a single click from the main
/// menu, taking the bank, every lore fragment, every almanac level and
/// every unlocked skill with it.
#[test]
fn a_first_press_only_arms() {
    assert_eq!(Game::delete_decision(None, 100.0), DeleteDecision::Arm);
}

/// A second press inside the window is the confirmation.
#[test]
fn a_second_press_inside_the_window_erases() {
    let armed_until = 100.0 + Game::DELETE_CONFIRM_WINDOW;
    assert_eq!(
        Game::delete_decision(Some(armed_until), 101.0),
        DeleteDecision::Erase
    );
}

/// Once the window has passed the prompt is stale, and a press starts
/// over rather than destroying a save the player stopped thinking about.
#[test]
fn a_press_after_the_window_arms_again() {
    assert_eq!(
        Game::delete_decision(Some(100.0), 100.0),
        DeleteDecision::Arm
    );
    assert_eq!(
        Game::delete_decision(Some(100.0), 500.0),
        DeleteDecision::Arm
    );
}

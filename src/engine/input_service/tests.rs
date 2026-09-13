use crate::data::loader::load_localization;
use crate::screens::Screen;
use crate::state::{GamePhase, KeyBindings};
use macroquad_toolkit::input::GamepadFrame;

/// Every key a button advertises must be one the input service reads on
/// the screen that shows it.
///
/// This reads the source rather than simulating a keypress, because
/// `is_key_pressed` needs a window. It is still the check that was
/// missing: "Try Again (SPACE)" sat on the game-over screen while Space
/// there emitted `ReturnToMenu`, and "Back to Menu (ESC)" sat beside it
/// while Escape was not read on that screen at all.
#[test]
fn the_outcome_screens_read_the_keys_they_advertise() {
    let source = include_str!("../input_service.rs");
    let arm = source
        .split("Screen::GameOver | Screen::Success =>")
        .nth(1)
        .expect("the outcome screens have an input arm");
    let arm = &arm[..arm.find("Screen::SkillTree").unwrap_or(arm.len())];

    assert!(
        arm.contains("KeyCode::Space"),
        "the outcome screens advertise SPACE and do not read it"
    );
    assert!(
        arm.contains("KeyCode::Escape"),
        "the outcome screens advertise ESC and do not read it"
    );
    assert!(
        arm.contains("TryAgain"),
        "SPACE on the outcome screens does not do what the button says"
    );
    assert!(
        arm.contains("ReturnToMenu"),
        "ESC on the outcome screens does not do what the button says"
    );
}

/// A screen that numbers its options in brackets is promising number
/// keys, and every phase that does so must read them.
///
/// The mid-ride event drew [1], [2], [3] beside its choices and the
/// Interaction phase read no digits at all — the same digits that pick a
/// route one phase earlier. A player taught the binding by the driving
/// screen would press it on the next screen and nothing would happen.
#[test]
fn every_phase_that_numbers_its_options_reads_the_digits() {
    let input = include_str!("../input_service.rs");

    for (phase, screen_source) in [
        (
            "GamePhase::Driving",
            include_str!("../../screens/game_screens/driving.rs"),
        ),
        (
            "GamePhase::Interaction",
            include_str!("../../screens/game_screens/interaction.rs"),
        ),
    ] {
        // The screen advertises digits by numbering its options.
        assert!(
            screen_source.contains(r#"format!("[{}]", i + 1)"#)
                || screen_source.contains(r#""[1]""#)
                || screen_source.contains("key_hint"),
            "{phase}'s screen no longer numbers its options; update this test"
        );

        let arm = input
            .split(&format!("{phase} => {{"))
            .nth(1)
            .unwrap_or_else(|| panic!("{phase} has no input arm"));
        let arm = &arm[..arm.find("GamePhase::").unwrap_or(arm.len())];
        assert!(
            arm.contains("number_keys()"),
            "{phase} numbers its options and reads no digits"
        );
    }
}

/// While a modal overlay is open, the only keys read are the ones the
/// overlay itself advertises: its dismissals, and — for the rules panel,
/// which draws the cab controls as key-labelled buttons — the cab keys.
///
/// Gameplay keys used to pass straight through an open overlay: SPACE
/// with the rules panel up could accept a ride the player was not
/// looking at, and the digits could pick a route behind the pause menu.
#[test]
fn open_overlays_swallow_gameplay_keys() {
    let source = include_str!("../input_service.rs");
    let ride_and_route_keys = [
        "StartGame",
        "AcceptRide",
        "DeclineRide",
        "SelectRoute",
        "SelectEventChoice",
        "UiAction::Continue",
        "FollowGuideline",
        "BreakGuideline",
    ];
    // The pause menu advertises no cab controls, so it swallows those too.
    let pause_forbidden = [
        "StartGame",
        "AcceptRide",
        "DeclineRide",
        "SelectRoute",
        "SelectEventChoice",
        "UiAction::Continue",
        "FollowGuideline",
        "BreakGuideline",
        "capture_cab_controls",
        "PerformRuleAction",
    ];
    for (arm_start, allowed, forbidden) in [
        (
            "Overlay::Pause => {",
            &["TogglePauseMenu"][..],
            &pause_forbidden[..],
        ),
        (
            "Overlay::Panel => {",
            &[
                "ToggleRules",
                "ToggleInventory",
                "TogglePauseMenu",
                "capture_cab_controls",
            ][..],
            &ride_and_route_keys[..],
        ),
    ] {
        let arm = source
            .split(arm_start)
            .nth(1)
            .expect("the overlay arm exists");
        let arm = &arm[..arm.find("Screen::Game").unwrap_or(arm.len())];

        for key in forbidden {
            assert!(
                !arm.contains(key),
                "{arm_start} reads a gameplay key ({key}) through a modal"
            );
        }
        for key in allowed {
            assert!(
                arm.contains(key),
                "{arm_start} no longer reads {key}; the overlay lost an advertised key"
            );
        }
    }
}

/// The guideline decision is the only timed choice, so it must be
/// reachable from the keyboard, and its buttons must say which keys.
#[test]
fn the_timed_decision_has_keys_and_advertises_them() {
    let source = include_str!("../input_service.rs");
    assert!(
        source.contains("GamePhase::GuidelineDecision"),
        "the timed decision has no key input"
    );

    let guidelines = load_localization().ui.game.guidelines;
    assert!(
        guidelines.follow.contains("(F)"),
        "the follow button does not name its key: {:?}",
        guidelines.follow
    );
    assert!(
        guidelines.break_guideline.contains("(B)"),
        "the break button does not name its key: {:?}",
        guidelines.break_guideline
    );
}

#[test]
fn gamepad_confirm_and_cancel_follow_visible_gameplay_actions() {
    let confirm = GamepadFrame {
        connected: true,
        confirm: true,
        ..Default::default()
    };
    let actions = super::InputService::capture_gamepad(
        Screen::Game,
        GamePhase::RideRequest,
        super::Overlay::None,
        confirm,
        false,
    );
    assert_eq!(actions, vec![crate::ui::UiAction::AcceptRide]);

    let cancel = GamepadFrame {
        connected: true,
        cancel: true,
        ..Default::default()
    };
    let actions = super::InputService::capture_gamepad(
        Screen::Game,
        GamePhase::Driving,
        super::Overlay::None,
        cancel,
        false,
    );
    assert_eq!(actions, vec![crate::ui::UiAction::TogglePauseMenu]);
}

#[test]
fn gamepad_directions_choose_numbered_route_options() {
    let frame = GamepadFrame {
        connected: true,
        right: true,
        ..Default::default()
    };
    let actions = super::InputService::capture_gamepad(
        Screen::Game,
        GamePhase::Driving,
        super::Overlay::None,
        frame,
        false,
    );
    assert_eq!(actions, vec![crate::ui::UiAction::SelectRoute(3)]);
}

#[test]
fn custom_binding_labels_are_serializable_and_defaulted() {
    let mut bindings = KeyBindings::default();
    bindings.cycle_accept();
    let encoded = serde_json::to_string(&bindings).expect("bindings serialize");
    let restored: KeyBindings = serde_json::from_str(&encoded).expect("bindings deserialize");
    assert_eq!(restored.accept, "ENTER");
    assert!(restored.conflicts().is_empty());
}

use nightmare_shift::data::loader::{load_constants, load_item_catalog};
use nightmare_shift::engine::ItemService;
use nightmare_shift::game::shift::minutes_on_the_clock;
use nightmare_shift::state::GameState;

/// The precondition the clock arithmetic has to survive: a shift can hold
/// more time than it started with.
#[test]
fn a_time_bonus_can_push_the_clock_past_the_shift_length() {
    let constants = load_constants();
    let catalog = load_item_catalog();
    let initial = constants.game_constants.initial_time;

    let generous = catalog
        .names()
        .into_iter()
        .map(|name| catalog.create_item(&name, "test", 0.0))
        .find(|item| {
            item.can_use
                && item.effects.iter().any(|effect| {
                    matches!(
                        effect.effect_type,
                        nightmare_shift::data::ItemEffectType::TimeBonus
                    )
                })
        })
        .expect("an item that adds time");

    let mut state = GameState::new(0.0, &constants.game_constants);
    state.current_passenger = None;
    assert_eq!(state.time_remaining, initial);
    for effect in &generous.effects {
        ItemService::apply_item_effect(effect, &mut state, &constants.reputation, 0.0);
    }
    assert!(
        state.time_remaining > initial,
        "{} did not push the clock past the shift length, so this test              no longer guards the underflow",
        generous.name
    );

    assert_eq!(
        minutes_on_the_clock(initial, state.time_remaining),
        0,
        "a clock fuller than the shift length must read as no time worked"
    );
}

/// And the ordinary case still measures the night.
#[test]
fn the_clock_measures_what_the_night_has_spent() {
    assert_eq!(minutes_on_the_clock(480, 480), 0);
    assert_eq!(minutes_on_the_clock(480, 300), 180);
    assert_eq!(minutes_on_the_clock(480, 0), 480);
}

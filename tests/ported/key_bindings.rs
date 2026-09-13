use nightmare_shift::state::KeyBindings;

#[test]
fn defaults_are_unique_and_cycles_keep_them_unique() {
    let mut bindings = KeyBindings::default();
    bindings.cycle_accept();
    bindings.cycle_decline();
    bindings.cycle_follow();
    bindings.cycle_break();
    bindings.cycle_pause();
    assert!(bindings.conflicts().is_empty());
}

#[test]
fn imported_conflicts_are_reported() {
    let mut bindings = KeyBindings::default();
    bindings.decline = bindings.accept.clone();
    assert_eq!(bindings.conflicts(), vec!["SPACE: accept and decline"]);
}

/// Every platform-conditional branch must produce something a player can
/// tell apart from the next one.
///
/// The web build has no wall clock, and the leaderboard's date column
/// fell back to the literal "Session" for every entry — ten identical
/// labels on a screen whose only job is distinguishing ten runs. The
/// wasm branch has to derive its label from something that changes.
///
/// This reads the source, so it breaks when the code moves — which it
/// did, when the shift lifecycle came out of `game.rs`. That is the
/// honest cost of the technique and preferable to the alternative, since
/// the branch cannot be evaluated on this target at all.
#[test]
fn the_web_leaderboard_label_varies_between_runs() {
    let source = include_str!("../../game/shift.rs");
    let branch = source
        .split(r#"#[cfg(target_arch = "wasm32")]"#)
        .find(|chunk| chunk.trim_start().starts_with("let date_str"))
        .expect("the wasm leaderboard label still exists");
    let branch = &branch[..branch.find(';').unwrap_or(branch.len())];

    assert!(
        branch.contains("total_shifts_completed"),
        "the web leaderboard label does not vary between runs: {branch:?}"
    );
}

/// Saving must be wired on both targets. The web build routes through the
/// toolkit's slot API and the desktop build through a file; losing either
/// silently drops the meta-progression the whole game accrues.
#[test]
fn both_targets_have_a_save_path() {
    let source = include_str!("../persistence.rs");
    for symbol in [
        "save_json",
        "load_json",
        "save_to_slot",
        "load_from_slot",
        "slot_exists",
        "delete_slot",
    ] {
        assert!(
            source.contains(symbol),
            "persistence no longer references {symbol}"
        );
    }
}

#[test]
fn version_one_save_migrates_without_inventing_a_run() {
    let player_stats = crate::state::PlayerStats::new();
    let value = serde_json::json!({
        "version": 1,
        "player_stats": player_stats,
    });
    let save: super::SaveData = serde_json::from_value(value).expect("legacy save fixture");

    assert_eq!(save.version, 1);
    assert!(save.run.is_none());
    assert!(super::SaveData::VERSION > save.version);
}

#[test]
fn run_checkpoint_round_trip_keeps_rng_and_simulation_time() {
    let mut state = crate::state::GameState::new(12.5, &crate::data::GameConstants::default());
    let _ = state.rng.next_u64();
    state.guideline_time_remaining = 7.0;
    let save = super::SaveData::with_run(crate::state::PlayerStats::new(), &state);
    let encoded = serde_json::to_string(&save).expect("checkpoint serializes");
    let decoded: super::SaveData = serde_json::from_str(&encoded).expect("checkpoint loads");
    let restored = decoded.run.expect("checkpoint has a run").game_state;

    assert_eq!(restored.simulation_time, state.simulation_time);
    assert_eq!(restored.rng.state(), state.rng.state());
    assert_eq!(restored.guideline_time_remaining, 7.0);
}

#[test]
fn newer_save_versions_are_rejected_before_loading() {
    let save = super::SaveData {
        version: super::SaveData::VERSION + 1,
        player_stats: crate::state::PlayerStats::new(),
        run: None,
    };

    assert!(super::Persistence::validate_version(&save).is_err());
}

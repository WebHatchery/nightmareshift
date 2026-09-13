use super::*;
use nightmare_shift::state::GameState;

/// A finished shift with the three numbers the lifetime counters read.
fn shift(earnings: u32, rides: u32, violations: u32) -> GameState {
    let constants = nightmare_shift::data::loader::load_constants();
    let mut state = GameState::new(0.0, &constants.game_constants);
    state.earnings = earnings;
    state.rides_completed = rides;
    state.rules_violated = violations;
    state
}

/// Progress reaching its target has to be the same moment the achievement
/// unlocks.
///
/// This is the drift test. The threshold a card prints and the threshold the
/// condition tests both come from `achievement_targets` precisely so they
/// cannot disagree, and this walks each countable achievement to the line
/// and checks it fires. A card promising "10 / 10 shifts survived" while
/// still reading Locked is the failure being guarded against.
#[test]
fn hitting_the_stated_target_unlocks_the_achievement() {
    use achievement_targets as target;
    let constants = nightmare_shift::data::loader::load_constants();

    let mut stats = PlayerStats::new();
    stats.init_achievements();
    stats.survival_bonuses = target::SHIFTS_SURVIVED;
    assert!(stats
        .check_achievements(None)
        .iter()
        .any(|id| id == "survivor"));

    let mut stats = PlayerStats::new();
    stats.init_achievements();
    stats.unlocked_skills = (0..target::SKILLS_UNLOCKED)
        .map(|n| n.to_string())
        .collect();
    assert!(stats
        .check_achievements(None)
        .iter()
        .any(|id| id == "skill_collector"));

    let mut stats = PlayerStats::new();
    stats.init_achievements();
    for passenger_id in 0..target::PASSENGERS_MASTERED as u32 {
        stats.almanac_progress.insert(
            passenger_id,
            AlmanacEntry {
                passenger_id,
                encountered: true,
                knowledge_level: 3,
                ..AlmanacEntry::default()
            },
        );
    }
    assert_eq!(stats.passengers_mastered(), target::PASSENGERS_MASTERED);
    assert!(stats
        .check_achievements(None)
        .iter()
        .any(|id| id == "almanac_scholar"));

    let mut stats = PlayerStats::new();
    stats.init_achievements();
    let mut shift = GameState::new(0.0, &constants.game_constants);
    shift.earnings = target::SINGLE_SHIFT_EARNINGS;
    assert!(stats
        .check_achievements(Some(FinishedShift::of(&shift, false)))
        .iter()
        .any(|id| id == "big_earner"));
}

/// Every countable achievement reports progress, and the two that are pass
/// or fail on a single night report none rather than a meaningless "0 / 1".
#[test]
fn only_the_countable_achievements_report_progress() {
    let stats = PlayerStats::new();
    for id in [
        "survivor",
        "big_earner",
        "almanac_scholar",
        "skill_collector",
    ] {
        assert!(
            stats.achievement_progress(id).is_some(),
            "{id} counts toward something but reports no progress"
        );
    }
    for id in ["first_shift", "perfect_shift"] {
        assert!(
            stats.achievement_progress(id).is_none(),
            "{id} cannot be part-done and should report nothing"
        );
    }
}

/// Progress never overstates itself. Surviving fifteen shifts still reads
/// ten of ten rather than fifteen of ten.
#[test]
fn progress_does_not_run_past_its_target() {
    use achievement_targets as target;
    let mut stats = PlayerStats::new();
    stats.survival_bonuses = target::SHIFTS_SURVIVED + 5;
    let line = stats.achievement_progress("survivor").expect("a line");
    assert!(
        line.starts_with(&format!(
            "{} / {}",
            target::SHIFTS_SURVIVED,
            target::SHIFTS_SURVIVED
        )),
        "progress overshot its target: {line:?}"
    );
}

/// A check made outside a shift cannot unlock a shift achievement.
///
/// This was three loose parameters, and the two menu callers filled them
/// from a `GameState` whose shift had already ended -- last night's
/// earnings, last night's violations, and a hardcoded `false`. Nothing
/// misfired then, because the shift-end call had already run with the same
/// numbers, but there was no way to say "no shift here" and the next
/// earnings-based achievement would have unlocked from the skill tree.
#[test]
fn a_menu_check_cannot_unlock_a_shift_achievement() {
    let constants = nightmare_shift::data::loader::load_constants();
    let mut finished = GameState::new(0.0, &constants.game_constants);
    finished.earnings = 900;
    finished.rules_violated = 0;

    let mut from_menu = PlayerStats::new();
    from_menu.init_achievements();
    let unlocked = from_menu.check_achievements(None);
    assert!(
        !unlocked.iter().any(|id| id == "big_earner"),
        "the skill tree paid out an achievement for earning money"
    );
    assert!(
        !unlocked.iter().any(|id| id == "perfect_shift"),
        "the skill tree paid out an achievement for a clean shift"
    );

    let mut from_shift = PlayerStats::new();
    from_shift.init_achievements();
    let unlocked = from_shift.check_achievements(Some(FinishedShift::of(&finished, true)));
    assert!(
        unlocked.iter().any(|id| id == "big_earner"),
        "a $900 shift did not earn big_earner"
    );
    assert!(
        unlocked.iter().any(|id| id == "perfect_shift"),
        "a clean surviving shift did not earn perfect_shift"
    );
}

/// Lifetime achievements are askable from anywhere, which is the whole
/// reason the menu calls this at all -- buying a third skill has to pay out
/// without waiting for a shift to end.
#[test]
fn a_menu_check_still_unlocks_lifetime_achievements() {
    let mut stats = PlayerStats::new();
    stats.init_achievements();
    stats.unlocked_skills = vec!["a".into(), "b".into(), "c".into()];
    let unlocked = stats.check_achievements(None);
    assert!(
        unlocked.iter().any(|id| id == "skill_collector"),
        "a third skill did not pay out from the skill tree"
    );
}

/// A revealed story has to leave the almanac able to show it. Marking the
/// backstory alone is not enough — the entry the almanac reads is keyed on
/// the encounter, so a story unlocked for a passenger never met would have
/// nothing to attach to.
#[test]
fn a_revealed_story_leaves_the_almanac_something_to_show() {
    let mut stats = PlayerStats::new();
    assert!(!stats.is_backstory_unlocked(1));
    assert!(stats.is_first_encounter(1));

    stats.reveal_story(1);

    assert!(stats.is_backstory_unlocked(1), "the story stayed hidden");
    assert!(
        !stats.is_first_encounter(1),
        "the story was revealed for a passenger the almanac has no entry for"
    );
}

/// Revealing the same story twice is not an error and does not lose the
/// unlock — a guideline can pay its reward on more than one ride.
#[test]
fn revealing_a_story_twice_keeps_it_revealed() {
    let mut stats = PlayerStats::new();
    stats.reveal_story(7);
    stats.reveal_story(7);
    assert!(stats.is_backstory_unlocked(7));
    assert_eq!(stats.get_encounter_count(7), 2);
}

/// A shift's violations have to reach the lifetime counter. This one
/// counter was left out of `record_shift_completion` while its five
/// siblings were maintained, so it read zero for every save ever written.
#[test]
fn a_shifts_violations_reach_the_lifetime_tally() {
    let mut stats = PlayerStats::new();
    assert_eq!(stats.total_rules_violated, 0);

    stats.record_shift_completion(&shift(120, 3, 2), true, 40);
    assert_eq!(stats.total_rules_violated, 2);

    stats.record_shift_completion(&shift(90, 2, 1), false, 30);
    assert_eq!(
        stats.total_rules_violated, 3,
        "the tally replaced the running total instead of adding to it"
    );
}

/// A clean night must not inflate it, and the siblings must still move —
/// a counter that rises on every shift regardless would look maintained
/// while meaning nothing.
#[test]
fn a_clean_shift_adds_no_violations() {
    let mut stats = PlayerStats::new();
    stats.record_shift_completion(&shift(200, 5, 0), true, 55);

    assert_eq!(stats.total_rules_violated, 0);
    assert_eq!(stats.total_shifts_completed, 1);
    assert_eq!(stats.total_rides_completed, 5);
}

/// The menu line has to have somewhere to put it. `replacen` on a string
/// with too few placeholders silently drops the value, so the count is
/// the only thing standing between a wired stat and an invisible one.
#[test]
fn the_menu_stat_line_has_a_slot_for_every_value() {
    let stats = nightmare_shift::data::loader::load_localization()
        .ui
        .main_menu
        .stats;
    assert_eq!(
        stats.matches("{}").count(),
        4,
        "the menu fills four lifetime counters into {stats:?}"
    );
}

/// Saves written before the `Achievements` registry adoption stored
/// `achievements` as a bare array. Loading one of those saves must not
/// error (which would otherwise wipe the whole `PlayerStats` via the
/// `unwrap_or_else(|_| PlayerStats::new())` fallback in `Game::new`).
#[test]
fn legacy_achievement_array_deserializes() {
    let mut stats = PlayerStats::new();
    stats.init_achievements();
    stats.unlock_achievement("first_shift", "2026-01-01".to_string());
    stats.total_shifts_completed = 3;

    // Reshape the current `{ "achievements": [...] }` registry encoding
    // back into the pre-migration bare-array shape a real legacy save
    // would contain, leaving every other field untouched.
    let mut value = serde_json::to_value(&stats).unwrap();
    let inner = value["achievements"]["achievements"].take();
    value["achievements"] = inner;
    let legacy_json = serde_json::to_string(&value).unwrap();

    let reloaded: PlayerStats = serde_json::from_str(&legacy_json).expect("legacy save loads");
    assert_eq!(reloaded.total_shifts_completed, 3);
    assert!(reloaded.is_achievement_unlocked("first_shift"));
    assert_eq!(reloaded.achievements.len(), stats.achievements.len());
}

/// Current saves serialize `achievements` as the registry's own shape
/// (`{ "achievements": [...] }`); round-tripping must preserve unlocks.
#[test]
fn current_achievement_registry_round_trips() {
    let mut stats = PlayerStats::new();
    stats.init_achievements();
    stats.unlock_achievement("first_shift", "2026-01-01".to_string());

    let json = serde_json::to_string(&stats).unwrap();
    let reloaded: PlayerStats = serde_json::from_str(&json).unwrap();

    assert!(reloaded.is_achievement_unlocked("first_shift"));
    assert_eq!(reloaded.achievements.len(), stats.achievements.len());
}

/// A standing earned in one session must come back in the next — the whole
/// point of moving reputation into the save — and a save from before the
/// field existed must load with no standings rather than fail.
#[test]
fn standings_survive_a_save_round_trip_and_old_saves_still_load() {
    let constants = nightmare_shift::data::loader::load_constants().reputation;
    let mut stats = PlayerStats::new();
    let mut reputation = nightmare_shift::state::PassengerReputation::default();
    reputation.update(true, &constants);
    stats.passenger_reputation.insert(7, reputation);

    let json = serde_json::to_string(&stats).unwrap();
    let reloaded: PlayerStats = serde_json::from_str(&json).unwrap();
    let standing = reloaded
        .passenger_reputation
        .get(&7)
        .expect("the standing survives the round trip");
    assert_eq!(standing.interactions, 1);
    assert_eq!(standing.positive_choices, 1);

    let mut value = serde_json::to_value(&stats).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .remove("passenger_reputation");
    let legacy: PlayerStats = serde_json::from_value(value).expect("old saves load");
    assert!(legacy.passenger_reputation.is_empty());
}

/// A missing `achievements` key (very old saves, from before achievements
/// existed at all) should default to an empty registry rather than
/// failing deserialization.
#[test]
fn missing_achievements_field_defaults_empty() {
    let stats = PlayerStats::new();
    let mut value = serde_json::to_value(&stats).unwrap();
    value.as_object_mut().unwrap().remove("achievements");

    let reloaded: PlayerStats = serde_json::from_value(value).expect("defaults apply");
    assert_eq!(reloaded.achievements.len(), 0);
}

#[test]
fn old_saves_receive_accessible_presentation_defaults() {
    let stats = PlayerStats::new();
    let mut value = serde_json::to_value(&stats).unwrap();
    value.as_object_mut().unwrap().remove("accessibility");

    let reloaded: PlayerStats = serde_json::from_value(value).expect("defaults apply");
    assert_eq!(reloaded.accessibility.text_scale_percent, 100);
    assert!(reloaded.accessibility.captions);
    assert_eq!(reloaded.accessibility.master_volume, 80);
}

#[test]
fn presentation_cycles_stay_inside_supported_values() {
    let mut settings = AccessibilitySettings::default();
    let mut text = Vec::new();
    let mut brightness = Vec::new();
    let mut volume = Vec::new();
    for _ in 0..4 {
        text.push(settings.text_scale_percent);
        brightness.push(settings.brightness_percent);
        volume.push(settings.master_volume);
        settings.cycle_text_scale();
        settings.cycle_brightness();
        settings.cycle_master_volume();
    }
    assert_eq!(text, vec![100, 115, 125, 100]);
    assert_eq!(brightness, vec![100, 115, 90, 100]);
    assert_eq!(volume, vec![80, 100, 0, 50]);
}

#[test]
fn almanac_tiers_remember_whether_play_or_lore_revealed_them() {
    let mut stats = PlayerStats::new();
    assert_eq!(stats.record_ride_survived(7), Some(1));
    let earned = stats.get_almanac_entry(7);
    assert_eq!(earned.earned_knowledge_levels, vec![1]);
    assert!(earned.lore_knowledge_levels.is_empty());

    stats.lore_fragments = 10;
    assert!(stats.upgrade_almanac_knowledge(7, 3));
    let mixed = stats.get_almanac_entry(7);
    assert_eq!(mixed.knowledge_level, 2);
    assert_eq!(mixed.earned_knowledge_levels, vec![1]);
    assert_eq!(mixed.lore_knowledge_levels, vec![2]);
}

#[test]
fn legacy_almanac_entries_default_to_an_unattributed_earlier_record() {
    let legacy = r#"{"passenger_id":4,"encountered":true,"knowledge_level":2}"#;
    let entry: AlmanacEntry = serde_json::from_str(legacy).expect("legacy entry loads");
    assert!(entry.earned_knowledge_levels.is_empty());
    assert!(entry.lore_knowledge_levels.is_empty());
}

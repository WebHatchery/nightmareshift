use super::*;
use nightmare_shift::data::loader::load_passengers;

/// A fixed-seed stream for tests: deterministic, and a fresh one per call
/// so no test's draws depend on another's.
fn test_rng() -> macroquad_toolkit::rng::SeededRng {
    macroquad_toolkit::rng::SeededRng::new(0x7E57)
}

/// A tell raised by escalation can be noticed.
///
/// Every one of them was recorded `player_noticed: false`, unconditionally.
/// The guideline screen labels such a tell "uncertain" and the playtest bot
/// filters it out, so the tells a passenger gives off as they come apart --
/// the ones `tellIntensities` authors per stage -- could never be noticed by
/// anybody, and the detection roll reached only the condition-based tells.
///
/// Rolled many times because it is a probability: one merge proves nothing.
#[test]
fn an_escalation_tell_can_be_noticed() {
    use nightmare_shift::data::loader::load_guidelines;

    let guidelines = load_guidelines();
    let passengers = load_passengers();

    let mut noticed_any = false;
    for passenger in &passengers {
        let Some(mut need) = PassengerStateMachine::initialize(passenger, 0.0) else {
            continue;
        };
        for _ in 0..200 {
            let triggered =
                PassengerStateMachine::apply_stress_delta(&mut need, passenger, 100, 0.0);
            let mut detected = Vec::new();
            PassengerStateMachine::merge_detected_tells(
                &mut test_rng(),
                &mut detected,
                triggered,
                passenger,
                1.0,
                0.0,
                &guidelines,
            );
            if detected.iter().any(|tell| tell.player_noticed) {
                noticed_any = true;
                break;
            }
            need = PassengerStateMachine::initialize(passenger, 0.0).expect("a profile");
        }
        if noticed_any {
            break;
        }
    }
    assert!(
        noticed_any,
        "no escalation tell was ever noticed, at full trust, across the whole roster"
    );
}

/// And a driver nobody trusts notices fewer of them, which is the loop the
/// trust impact exists to feed: standing rises as a passenger frays, and
/// better standing catches more of what they give off.
///
/// Counted across the whole roster and every stage, because a single
/// passenger pushed straight to meltdown crosses only one stage and may have
/// no tell of the matching intensity for it. My first attempt did exactly
/// that and read "caught 0 against 0" -- which was no tells raised, not no
/// tells noticed. The `raised` assertion below is there so the comparison
/// can never be vacuous in that way again.
#[test]
fn trust_changes_how_many_escalation_tells_are_caught() {
    use nightmare_shift::data::loader::load_guidelines;

    let guidelines = load_guidelines();
    let passengers = load_passengers();

    let sweep = |trust: f32| {
        let mut raised = 0usize;
        let mut noticed = 0usize;
        for _ in 0..60 {
            for passenger in &passengers {
                let Some(mut need) = PassengerStateMachine::initialize(passenger, 0.0) else {
                    continue;
                };
                for stage in [NeedStage::Warning, NeedStage::Critical, NeedStage::Meltdown] {
                    let thresholds = need.profile.thresholds.clone();
                    let target = match stage {
                        NeedStage::Warning => thresholds.warning,
                        NeedStage::Critical => thresholds.critical,
                        _ => thresholds.meltdown,
                    };
                    let step = target as i32 - need.level as i32;
                    let triggered =
                        PassengerStateMachine::apply_stress_delta(&mut need, passenger, step, 0.0);
                    let mut detected = Vec::new();
                    PassengerStateMachine::merge_detected_tells(
                        &mut test_rng(),
                        &mut detected,
                        triggered,
                        passenger,
                        trust,
                        0.0,
                        &guidelines,
                    );
                    raised += detected.len();
                    noticed += detected.iter().filter(|tell| tell.player_noticed).count();
                }
            }
        }
        (raised, noticed)
    };

    let (raised_low, noticed_low) = sweep(0.0);
    let (raised_high, noticed_high) = sweep(1.0);

    assert!(
        raised_low > 0 && raised_high > 0,
        "no escalation tells were raised at all, so this compares nothing"
    );
    assert!(
        noticed_high > noticed_low,
        "trust caught {noticed_high} of {raised_high} against {noticed_low} of {raised_low}"
    );
}

/// A passenger past calm always has something to say.
///
/// Their line is the only tell an unstudied driver gets on the guideline
/// screen, which now shows it. If a profile authors nothing for a stage the
/// screen falls silent exactly when the passenger is worst.
#[test]
fn every_escalated_stage_gives_the_passenger_a_line() {
    for passenger in load_passengers() {
        let Some(mut need) = PassengerStateMachine::initialize(&passenger, 0.0) else {
            continue;
        };
        for stage in [NeedStage::Warning, NeedStage::Critical, NeedStage::Meltdown] {
            need.stage = stage;
            assert!(
                PassengerStateMachine::get_dialogue_for_stage(&mut test_rng(), &passenger, &need)
                    .is_some(),
                "{} says nothing at {:?}",
                passenger.name,
                stage
            );
        }
    }
}

/// A calm passenger says nothing extra, which is why no profile authors a
/// `calm` block. The screens fall back to their opening line.
#[test]
fn a_calm_passenger_has_no_escalation_line() {
    for passenger in load_passengers() {
        let Some(mut need) = PassengerStateMachine::initialize(&passenger, 0.0) else {
            continue;
        };
        need.stage = NeedStage::Calm;
        assert!(
            PassengerStateMachine::get_dialogue_for_stage(&mut test_rng(), &passenger, &need)
                .is_none(),
            "{} has an escalation line while still calm",
            passenger.name
        );
    }
}

/// Crossing into a stage banks the trust that stage authors.
#[test]
fn escalating_moves_the_drivers_standing() {
    let chen = load_passengers()
        .into_iter()
        .find(|p| p.id == 1)
        .expect("Mrs. Chen");
    let authored = *chen
        .state_profile
        .as_ref()
        .expect("a profile")
        .trust_impact
        .as_ref()
        .expect("Chen authors a trust impact")
        .get(NeedStage::Warning.key())
        .expect("a warning entry");

    let mut need = PassengerStateMachine::initialize(&chen, 0.0).expect("a profile");
    assert_eq!(need.pending_trust, 0.0);

    let thresholds = need.profile.thresholds.clone();
    let to_warning = thresholds.warning as i32 - need.level as i32;
    PassengerStateMachine::apply_stress_delta(&mut need, &chen, to_warning, 0.0);

    assert_eq!(need.stage, NeedStage::Warning);
    assert_eq!(need.pending_trust, authored);
}

/// A stage pays once. `revealed_stages` already gates the tells to a
/// first crossing and the trust has to be gated with them, or drifting
/// back and forth across a threshold would farm standing.
#[test]
fn a_stage_pays_its_trust_only_once() {
    let chen = load_passengers()
        .into_iter()
        .find(|p| p.id == 1)
        .expect("Mrs. Chen");
    let mut need = PassengerStateMachine::initialize(&chen, 0.0).expect("a profile");
    let thresholds = need.profile.thresholds.clone();

    let to_warning = thresholds.warning as i32 - need.level as i32;
    PassengerStateMachine::apply_stress_delta(&mut need, &chen, to_warning, 0.0);
    let after_first = need.pending_trust;

    // Down below the threshold and back over it.
    PassengerStateMachine::apply_stress_delta(&mut need, &chen, -40, 0.0);
    PassengerStateMachine::apply_stress_delta(&mut need, &chen, 40, 0.0);

    assert_eq!(need.stage, NeedStage::Warning);
    assert_eq!(
        need.pending_trust, after_first,
        "re-crossing warning paid a second time"
    );
}

/// Every stage a profile authors a trust impact for has to name a real
/// stage, or the number is silently never applied.
#[test]
fn authored_trust_impact_stages_are_recognised() {
    let mut checked = 0;
    for passenger in load_passengers() {
        let Some(profile) = &passenger.state_profile else {
            continue;
        };
        let Some(impact) = &profile.trust_impact else {
            continue;
        };
        for key in impact.keys() {
            assert!(
                NeedStage::parse(key).is_some(),
                "{} authors a trust impact for unknown stage {key:?}",
                passenger.name
            );
            checked += 1;
        }
    }
    assert!(checked > 0, "no profile authors a trust impact any more");
}

/// The obey pressure has to reach the level, or wiring a rule's authored
/// follow cost through to here bought nothing.
///
/// Shortcut is excluded deliberately: that branch spends `break_rule`, so
/// a test that drove the passenger down a shortcut would pass whatever the
/// override did.
#[test]
fn a_harsher_rule_wears_the_passenger_down_faster() {
    let passenger = load_passengers()
        .into_iter()
        .find(|p| p.state_profile.is_some())
        .expect("a passenger with a profile");

    let level_after = |pressure: Option<i32>| {
        let mut need = PassengerStateMachine::initialize(&passenger, 0.0).expect("a profile");
        PassengerStateMachine::apply_route_choice(
            &mut need,
            &passenger,
            RouteType::Normal,
            pressure,
            0.0,
            0.0,
        );
        need.level
    };

    let gentle = level_after(Some(3));
    let harsh = level_after(Some(12));
    assert!(
        harsh > gentle,
        "a follow cost of 12 left the passenger no worse off than one of 3 ({harsh} vs {gentle})"
    );
}

/// With no rule of theirs in force the passenger's own flat `obey` still
/// applies, so a shift that happens to draw none of their rules behaves
/// exactly as it did before this was wired.
#[test]
fn no_rule_in_force_falls_back_to_the_passengers_own_obey() {
    let passenger = load_passengers()
        .into_iter()
        .find(|p| p.state_profile.is_some())
        .expect("a passenger with a profile");
    let obey = passenger
        .state_profile
        .as_ref()
        .expect("a profile")
        .need_change
        .obey;

    let level_after = |pressure: Option<i32>| {
        let mut need = PassengerStateMachine::initialize(&passenger, 0.0).expect("a profile");
        PassengerStateMachine::apply_route_choice(
            &mut need,
            &passenger,
            RouteType::Normal,
            pressure,
            0.0,
            0.0,
        );
        need.level
    };

    assert_eq!(level_after(None), level_after(Some(obey)));
}

/// Every stage a profile authors intensities for must parse to at least
/// one real intensity, or the passenger silently falls back to the
/// defaults and the authored tuning does nothing.
#[test]
fn authored_tell_intensities_all_parse() {
    for passenger in load_passengers() {
        let Some(profile) = &passenger.state_profile else {
            continue;
        };
        let Some(map) = &profile.tell_intensities else {
            continue;
        };
        for (stage, names) in map {
            let parsed = names
                .iter()
                .filter(|n| PassengerStateMachine::parse_intensity(n).is_some())
                .count();
            assert_eq!(
                parsed,
                names.len(),
                "{} authors an unrecognised intensity for {stage:?}: {names:?}",
                passenger.name
            );
        }
    }
}

/// Every stage key a profile authors must be one the state machine looks
/// up, otherwise the entry is never read.
#[test]
fn authored_stage_keys_are_recognised() {
    let known = [
        NeedStage::Calm.key(),
        NeedStage::Warning.key(),
        NeedStage::Critical.key(),
        NeedStage::Meltdown.key(),
    ];
    for passenger in load_passengers() {
        let Some(profile) = &passenger.state_profile else {
            continue;
        };
        if let Some(map) = &profile.tell_intensities {
            for stage in map.keys() {
                assert!(
                    known.contains(&stage.as_str()),
                    "{} authors tellIntensities for unknown stage {stage:?}",
                    passenger.name
                );
            }
        }
        if let Some(dialogue) = &profile.dialogue_by_stage {
            for stage in dialogue.keys() {
                assert!(
                    known.contains(&stage.as_str()),
                    "{} authors dialogue for unknown stage {stage:?}",
                    passenger.name
                );
            }
        }
    }
}

/// An authored intensity map must actually change which tells surface,
/// or wiring it bought nothing over the hardcoded defaults.
#[test]
fn authored_intensities_override_the_defaults() {
    let mut map = TellIntensityMap::new();
    map.insert("warning".to_string(), vec!["obvious".to_string()]);
    assert_eq!(
        PassengerStateMachine::stage_intensities(NeedStage::Warning, Some(&map)),
        vec![TellIntensity::Obvious],
    );
    // Unnamed stages keep the default.
    assert_eq!(
        PassengerStateMachine::stage_intensities(NeedStage::Critical, Some(&map)),
        vec![TellIntensity::Obvious],
    );
    assert_eq!(
        PassengerStateMachine::stage_intensities(NeedStage::Calm, Some(&map)),
        vec![TellIntensity::Subtle],
    );
}

/// Every passenger must surface a tell at *both* warning and critical.
/// Authoring an intensity the passenger owns no tell of leaves that stage
/// silent, which is how six of the roster used to reach one of the two
/// stages with nothing to show for it.
#[test]
fn every_passenger_surfaces_a_tell_at_each_stage() {
    for passenger in load_passengers() {
        let Some(profile) = &passenger.state_profile else {
            continue;
        };
        for stage in [NeedStage::Warning, NeedStage::Critical] {
            let surfaces =
                PassengerStateMachine::stage_intensities(stage, profile.tell_intensities.as_ref())
                    .into_iter()
                    .any(|intensity| passenger.tells.iter().any(|t| t.intensity == intensity));
            assert!(surfaces, "{} surfaces no tell at {stage:?}", passenger.name);
        }
    }
}

/// The reveal must escalate: whatever shows at critical must be at least
/// as strong as what showed at warning, or the passenger gets quieter as
/// they get closer to breaking.
#[test]
fn tells_escalate_from_warning_to_critical() {
    for passenger in load_passengers() {
        let Some(profile) = &passenger.state_profile else {
            continue;
        };
        let authored = profile.tell_intensities.as_ref();
        let warning = PassengerStateMachine::stage_intensities(NeedStage::Warning, authored);
        let critical = PassengerStateMachine::stage_intensities(NeedStage::Critical, authored);
        let (Some(faintest_warning), Some(strongest_critical)) =
            (warning.iter().min(), critical.iter().max())
        else {
            continue;
        };
        assert!(
            strongest_critical >= faintest_warning,
            "{} goes quieter at critical ({critical:?}) than at warning ({warning:?})",
            passenger.name
        );
    }
}

/// A verbal tell with a trigger phrase is what the passenger says; a
/// behavioural one is not spoken.
#[test]
fn only_verbal_tells_are_spoken() {
    let behavioural = TriggeredTell {
        tell: PassengerTell {
            tell_type: TellType::Behavioral,
            intensity: TellIntensity::Obvious,
            description: "Fidgets".to_string(),
            trigger_phrase: Some("should not be said".to_string()),
            audio_cue: None,
            animation_cue: None,
            reliability: 1.0,
        },
        exception_id: None,
        related_guideline_id: None,
    };
    assert!(PassengerStateMachine::spoken_tell(&[behavioural]).is_none());

    let verbal = TriggeredTell {
        tell: PassengerTell {
            tell_type: TellType::Verbal,
            intensity: TellIntensity::Moderate,
            description: "Asks about the tide".to_string(),
            trigger_phrase: Some("Tide is wrong".to_string()),
            audio_cue: None,
            animation_cue: None,
            reliability: 1.0,
        },
        exception_id: None,
        related_guideline_id: None,
    };
    let spoken = PassengerStateMachine::spoken_tell(&[verbal]).expect("verbal tell speaks");
    assert!(spoken.contains("Tide is wrong"));
}

#[test]
fn authored_audio_cues_survive_loading_for_caption_and_playback() {
    let passenger_cues = nightmare_shift::data::loader::load_passengers()
        .into_iter()
        .flat_map(|passenger| passenger.tells)
        .filter(|tell| tell.audio_cue.is_some())
        .count();
    let guideline_cues = nightmare_shift::data::loader::load_guidelines()
        .into_iter()
        .flat_map(|guideline| guideline.exceptions)
        .flat_map(|exception| exception.tells)
        .filter(|tell| tell.audio_cue.is_some())
        .count();
    assert!(passenger_cues > 0, "passenger audio cues were discarded");
    assert!(guideline_cues > 0, "guideline audio cues were discarded");
}

use nightmare_shift::data::environment::WeatherCondition;
use nightmare_shift::data::loader::{load_guidelines, load_passengers};
use nightmare_shift::data::ConsequenceType;
use std::collections::HashSet;

/// Reading a passenger correctly pays what the guideline authors.
///
/// This used to be a hardcoded pair built in `calculate_positive_
/// consequences`, so all eighteen guidelines authored `exceptionRewards`
/// -- reputation and a chance at the passenger's story -- and none of it
/// ever paid. An empty list here would silently restore that.
#[test]
fn every_guideline_pays_something_for_a_correct_read() {
    for guideline in load_guidelines() {
        assert!(
            !guideline.exception_rewards.is_empty(),
            "guideline {} pays nothing for reading the passenger right",
            guideline.id
        );
        for reward in &guideline.exception_rewards {
            assert!(
                reward.probability > 0.0,
                "guideline {} authors a reward at zero probability",
                guideline.id
            );
        }
    }
}

/// And at least one of them has to be the story unlock, since that is the
/// only route from reading a passenger well to the almanac knowing more
/// about them.
#[test]
fn a_correct_read_can_reveal_a_passengers_story() {
    let reveals = load_guidelines().iter().any(|guideline| {
        guideline
            .exception_rewards
            .iter()
            .any(|reward| reward.consequence_type == ConsequenceType::StoryUnlock)
    });
    assert!(
        reveals,
        "no guideline can reveal a story, so tell-reading feeds the almanac nothing"
    );
}

/// A passenger's `stateProfile.exceptionId` is what lets satisfying a
/// guideline exception relieve their need. If it names an exception that
/// does not exist, or one that does not target them, the relief path is
/// unreachable and `exceptionRelief` is dead balance.
#[test]
fn every_profile_exception_targets_its_own_passenger() {
    let guidelines = load_guidelines();
    for passenger in load_passengers() {
        let Some(profile) = &passenger.state_profile else {
            continue;
        };
        let Some(exception_id) = &profile.exception_id else {
            continue;
        };
        let matched = guidelines
            .iter()
            .flat_map(|g| g.exceptions.iter())
            .find(|e| &e.id == exception_id)
            .unwrap_or_else(|| {
                panic!(
                    "{} names unknown exception {exception_id:?}",
                    passenger.name
                )
            });
        assert!(
            matched.passenger_ids.contains(&passenger.id)
                || matched.passenger_types.contains(&passenger.supernatural),
            "exception {exception_id:?} does not target {}",
            passenger.name
        );
    }
}

/// Deception must be a real gradient. If every passenger hides the same
/// amount the field may as well not exist, and the almanac's Candour line
/// tells the player nothing worth paying for.
#[test]
fn deception_varies_across_the_roster() {
    let levels: Vec<f32> = load_passengers()
        .iter()
        .map(|p| p.deception_level)
        .collect();
    let lowest = levels.iter().cloned().fold(f32::INFINITY, f32::min);
    let highest = levels.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    assert!(
        highest - lowest > 0.4,
        "deception spans only {lowest}..{highest}"
    );
}

/// Nobody may be authored past total deception or total secrecy, which
/// would make their tells undetectable however well the player plays.
#[test]
fn nobody_is_completely_unreadable() {
    for passenger in load_passengers() {
        assert!(
            (0.0..1.0).contains(&passenger.deception_level),
            "{} has deception {}",
            passenger.name,
            passenger.deception_level
        );
        assert!(
            (0.0..=1.0).contains(&passenger.trust_required),
            "{} requires trust {}",
            passenger.name,
            passenger.trust_required
        );
    }
}

/// The harder a passenger is to read, the more trust they should want
/// first — otherwise the two fields pull against each other and the
/// difficulty they describe is incoherent.
#[test]
fn deception_and_trust_required_agree() {
    let mut passengers = load_passengers();
    passengers.sort_by(|a, b| a.deception_level.total_cmp(&b.deception_level));
    let least = &passengers[0];
    let most = passengers.last().expect("roster is not empty");
    assert!(
        most.trust_required >= least.trust_required,
        "{} hides most but asks less trust than {}",
        most.name,
        least.name
    );
}

/// A conjured tell must actually be false: it may only borrow from a
/// breaking-safer exception that is not live for the passenger it is
/// pinned on, and it must arrive noticed, because unnoticed tells never
/// display. The old implementation cloned a genuine tell — a lie
/// indistinguishable from the truth is not a lie, and the dedupe threw it
/// away regardless.
#[test]
fn a_conjured_tell_points_at_a_dormant_breaking_safer_exception() {
    let guidelines = load_guidelines();
    let weather = WeatherCondition::default();
    // Every exception treated as having won its ride roll, so dormancy here
    // means a failed match or condition and the assertions stay exact.
    let all_live: std::collections::HashSet<String> = guidelines
        .iter()
        .flat_map(|g| g.exceptions.iter())
        .map(|e| e.id.clone())
        .collect();
    let mut conjured = 0;
    let mut conjure_rng = macroquad_toolkit::rng::SeededRng::new(0xBA17);
    for passenger in load_passengers() {
        for _ in 0..10 {
            let Some(tell) = super::GuidelineEngine::conjure_false_tell(
                &mut conjure_rng,
                &passenger,
                &weather,
                &guidelines,
                &all_live,
                0.0,
            ) else {
                continue;
            };
            conjured += 1;
            assert!(tell.player_noticed, "an unnoticed lie never displays");
            let exception_id = tell
                .exception_id
                .as_ref()
                .expect("a false tell names its bait");
            let exception = guidelines
                .iter()
                .flat_map(|g| g.exceptions.iter())
                .find(|e| &e.id == exception_id)
                .expect("the bait exception exists");
            assert!(
                exception.breaking_safer,
                "the bait must tempt a break, not a follow"
            );
            let live = super::GuidelineEngine::passenger_matches_exception(&passenger, exception)
                && super::GuidelineEngine::check_exception_conditions(
                    exception, &weather, &passenger,
                );
            assert!(
                !live,
                "conjured a tell for an exception genuinely live on {}",
                passenger.name
            );
        }
    }
    assert!(conjured > 0, "no passenger ever yields a false tell");
}

/// Every exception must author a probability the ride roll can win. The
/// serde default is 1.0 (always live), so this guards the one authorable
/// mistake left: an explicit 0.0, which would make the exception — and
/// every tell and relief hanging off it — permanently dormant.
#[test]
fn every_exception_authors_a_rollable_probability() {
    for guideline in load_guidelines() {
        for exception in &guideline.exceptions {
            assert!(
                exception.probability > 0.0 && exception.probability <= 1.0,
                "exception {:?} authors probability {} and can never be live",
                exception.id,
                exception.probability
            );
        }
    }
}

/// The false-tell gate opens only for a seasoned, currently-accurate
/// driver. The rookie cases are deterministic; the open-gate case is a
/// coin the test flips enough times to trust.
#[test]
fn false_tells_wait_for_a_seasoned_accurate_driver() {
    use nightmare_shift::state::{GameState, GuidelineAction, GuidelineDecision, PlayerStats};

    let constants = nightmare_shift::data::loader::load_constants();
    let decision = |was_correct: bool| GuidelineDecision {
        guideline_id: 1,
        passenger_id: 1,
        action: GuidelineAction::Follow,
        was_correct,
        tells_present: Vec::new(),
        timestamp: 0.0,
    };

    let mut state = GameState::new(0.0, &constants.game_constants);
    let mut stats = PlayerStats::new();
    let mut gate_rng = macroquad_toolkit::rng::SeededRng::new(0x6A7E);
    stats.total_rides_completed = 100;

    // Two decisions is not a record, however good it looks.
    state.decision_history = vec![decision(true), decision(true)];
    assert!(!super::GuidelineEngine::should_introduce_false_tells(
        &mut gate_rng,
        &state.decision_history,
        &stats
    ));

    // A seasoned driver misreading tonight is not worth deceiving.
    state.decision_history = vec![
        decision(true),
        decision(false),
        decision(false),
        decision(false),
    ];
    assert!(!super::GuidelineEngine::should_introduce_false_tells(
        &mut gate_rng,
        &state.decision_history,
        &stats
    ));

    // A rookie reading perfectly has not earned the lies yet.
    state.decision_history = vec![decision(true), decision(true), decision(true)];
    let rookie = PlayerStats::new();
    assert!(!super::GuidelineEngine::should_introduce_false_tells(
        &mut gate_rng,
        &state.decision_history,
        &rookie
    ));

    // Seasoned and sharp: the gate is a coin toss, so flip until it lands.
    let opened = (0..300).any(|_| {
        super::GuidelineEngine::should_introduce_false_tells(
            &mut gate_rng,
            &state.decision_history,
            &stats,
        )
    });
    assert!(
        opened,
        "the gate never opened for a seasoned, accurate driver"
    );
}

/// Relief is the only downward pressure on a passenger's need, so a
/// profile that authors none can never be settled by reading it right.
#[test]
fn every_profile_authors_relief() {
    for passenger in load_passengers() {
        let Some(profile) = &passenger.state_profile else {
            continue;
        };
        assert!(
            profile.need_change.exception_relief > 0,
            "{} has no exceptionRelief",
            passenger.name
        );
    }
}

/// Reading a passenger correctly must win back more than a single leg of
/// the ride costs, or the relief is cosmetic and the need still ratchets
/// to meltdown no matter how well the player plays.
#[test]
fn relief_outpaces_a_leg_of_need_growth() {
    for passenger in load_passengers() {
        let Some(profile) = &passenger.state_profile else {
            continue;
        };
        let change = &profile.need_change;
        let worst_leg = change.passive + change.obey.max(change.break_rule);
        assert!(
            change.exception_relief > worst_leg,
            "{}: relief {} does not beat one leg's {worst_leg}",
            passenger.name,
            change.exception_relief
        );
    }
}

/// Conversely, an exception that names passenger ids must name real ones.
#[test]
fn every_exception_targets_real_passengers() {
    let ids: HashSet<u32> = load_passengers().iter().map(|p| p.id).collect();
    for guideline in load_guidelines() {
        for exception in &guideline.exceptions {
            for id in &exception.passenger_ids {
                assert!(
                    ids.contains(id),
                    "exception {:?} targets unknown passenger {id}",
                    exception.id
                );
            }
        }
    }
}

/// An exception gated on words nobody says is an exception that never
/// happens.
///
/// `passenger_dialogue` conditions substring-match the value against the
/// passenger's own `dialogue` lines, so the guideline file and the
/// passenger file have to agree about a word -- and they are different
/// files with nothing between them. Twelve of the fourteen agreed. Two did
/// not: Sister Agnes's blessing and Death's offer to trade places were
/// authored in full, each with two tells, a `breakingSafer` flag and a
/// `requiredStage`, and neither passenger had a line containing "bless" or
/// "trade". Both exceptions were unreachable, which also put their rewards,
/// their relief and their tells out of the player's reach.
///
/// Eligibility is checked the same way the engine checks it, so this fails
/// if the word goes missing or if the exception is pointed at a passenger
/// who never says it.
#[test]
fn every_dialogue_exception_has_someone_who_says_the_words() {
    let passengers = load_passengers();
    let mut checked = 0;
    for guideline in load_guidelines() {
        for exception in &guideline.exceptions {
            for condition in &exception.conditions {
                if condition.condition_type != "passenger_dialogue" {
                    continue;
                }
                let word = condition
                    .value
                    .as_str()
                    .expect("a dialogue condition matches a string")
                    .to_lowercase();
                checked += 1;
                let speaker = passengers.iter().find(|passenger| {
                    super::GuidelineEngine::passenger_matches_exception(passenger, exception)
                        && passenger
                            .dialogue
                            .iter()
                            .any(|line| line.to_lowercase().contains(&word))
                });
                assert!(
                    speaker.is_some(),
                    "exception {:?} on guideline {} waits to hear {:?}, and no \
                     passenger it applies to has a line containing it",
                    exception.id,
                    guideline.id,
                    word
                );
            }
        }
    }
    assert!(checked > 0, "no dialogue conditions found to check");
}

/// A passenger's own relief has to be something they can actually earn.
///
/// Sixteen passengers each name an exception in `stateProfile.exceptionId`
/// and author an `exceptionRelief` -- 24 to 35 points off the need that is
/// about to break them -- and reading them right is the only way to spend
/// it. That makes the exception's conditions a promise to that specific
/// passenger, which is a stronger claim than the conditions merely being
/// satisfiable by somebody.
///
/// Jake Morrison could not keep it. He is named on `shortcut_time_critical`
/// by id, points his own state profile at it, and every staged line he has
/// begs for a faster route -- "the thirst is unbearable, take the alleys".
/// The exception asks for stress above zero. `stressLevel` is
/// `serde(default)`, nine of the sixteen omit it, and 0.0 is not a calm
/// passenger but an unauthored one. So the vampire racing dawn could never
/// have his shortcut read as the right call, could never collect the 35, and
/// breaking the rule for him was never the safer play it is written to be.
///
/// This checks both condition kinds, so it also covers the two dialogue
/// exceptions fixed alongside it.
#[test]
fn every_passenger_can_reach_the_relief_they_are_promised() {
    let guidelines = load_guidelines();
    let mut checked = 0;
    for passenger in load_passengers() {
        let Some(profile) = &passenger.state_profile else {
            continue;
        };
        let Some(wanted) = &profile.exception_id else {
            continue;
        };
        let found = guidelines
            .iter()
            .flat_map(|guideline| &guideline.exceptions)
            .find(|exception| &exception.id == wanted);
        let exception = found.unwrap_or_else(|| {
            panic!(
                "{} relies on exception {wanted:?}, which no guideline authors",
                passenger.name
            )
        });
        checked += 1;
        assert!(
            super::GuidelineEngine::passenger_matches_exception(&passenger, exception),
            "{} points at exception {wanted:?} and is not eligible for it",
            passenger.name
        );
        assert!(
            super::GuidelineEngine::check_exception_conditions(
                exception,
                &WeatherCondition::default(),
                &passenger,
            ),
            "{} can never satisfy exception {wanted:?}, so the relief their \
             state profile authors is unreachable",
            passenger.name
        );
    }
    assert!(checked > 0, "no passenger relief to check");
}

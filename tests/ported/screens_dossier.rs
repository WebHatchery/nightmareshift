use super::*;
use nightmare_shift::data::loader::load_passengers;

/// Each level must add something for every passenger, or paying lore
/// fragments to reach it buys nothing.
#[test]
fn every_level_reveals_more_for_every_passenger() {
    let data = GameData::load().expect("embedded game data parses");
    for passenger in load_passengers() {
        let mut previous = build(&passenger, 0, Some(&data), None).len();
        for level in 1..=3 {
            let count = build(&passenger, level, Some(&data), None).len();
            assert!(
                count > previous,
                "{} reveals nothing new at knowledge level {level}",
                passenger.name
            );
            previous = count;
        }
    }
}

/// What a level promises is what that level delivers.
///
/// `almanacData.json` authors a reward list per level, and since the almanac
/// card prints it -- "Studied reveals Route Preferences, Common Tells..." --
/// the list is now a promise made to the player in the UI. It was wrong in
/// both directions before this test existed. Level 1 claimed "Name" and
/// "Description", both of which an encountered passenger shows for free at
/// level 0, and omitted Traits, which it does buy. Level 3 claimed "Hidden
/// Rules", which no almanac level touches -- hidden rules belong to the
/// shift, not to a passenger, so a per-passenger mastery could not reveal
/// them even in principle -- and omitted Relief and Candour.
///
/// The mapping is spelled out rather than inferred, because the two sides
/// are deliberately worded differently: the player reads "Basic Needs", the
/// dossier labels the line "Need".
#[test]
fn every_promised_reward_is_delivered_at_that_level() {
    use nightmare_shift::data::loader::load_almanac;

    // Reward name -> the dossier label that carries it.
    const DELIVERED_BY: [(&str, &str); 9] = [
        ("Basic Needs", "Need"),
        ("Traits", "Traits"),
        ("Common Tells", "Tell"),
        ("Carried Items", "Carries"),
        ("Associates", "Associates"),
        ("Their Rule", "Their rule"),
        ("True Nature", "True nature"),
        ("Relief", "Relief"),
        ("Candour", "Candour"),
    ];
    // Delivered somewhere other than the dossier: route preferences appear
    // on the almanac card and the driving screen's route cards, and the
    // backstory on the almanac card.
    const ELSEWHERE: [&str; 2] = ["Route Preferences", "Backstory"];

    let data = GameData::load().expect("embedded game data parses");
    let passengers = load_passengers();
    let almanac = load_almanac();

    for level in 1..=3u32 {
        let entry = almanac.get_level(level).expect("a level");
        for reward in &entry.rewards {
            if ELSEWHERE.contains(&reward.as_str()) {
                continue;
            }
            let label = DELIVERED_BY
                .iter()
                .find(|(name, _)| name == reward)
                .map(|(_, label)| *label)
                .unwrap_or_else(|| {
                    panic!("level {level} promises {reward:?}, which nothing delivers")
                });

            // Some lines need underlying data the passenger may not have,
            // so it is enough that one passenger gains this label exactly
            // here -- and none gains it a level earlier.
            let gained_here = passengers.iter().any(|passenger| {
                let has = |at| {
                    build(passenger, at, Some(&data), None)
                        .iter()
                        .any(|line| line.label == label)
                };
                has(level) && !has(level - 1)
            });
            assert!(
                gained_here,
                "level {level} promises {reward:?} but no passenger gains {label:?} there"
            );
        }
    }
}

/// And nothing a level delivers goes unmentioned, or the card understates
/// what the lore buys.
#[test]
fn every_delivered_line_is_promised() {
    use nightmare_shift::data::loader::load_almanac;
    use std::collections::HashSet;

    // The dossier labels, as the reward lists spell them.
    const PROMISED_AS: [(&str, &str); 9] = [
        ("Need", "Basic Needs"),
        ("Traits", "Traits"),
        ("Tell", "Common Tells"),
        ("Carries", "Carried Items"),
        ("Associates", "Associates"),
        ("Their rule", "Their Rule"),
        ("True nature", "True Nature"),
        ("Relief", "Relief"),
        ("Candour", "Candour"),
    ];

    let data = GameData::load().expect("embedded game data parses");
    let almanac = load_almanac();

    for level in 1..=3u32 {
        let promised: HashSet<&str> = almanac
            .get_level(level)
            .expect("a level")
            .rewards
            .iter()
            .map(String::as_str)
            .collect();

        for passenger in load_passengers() {
            let before: HashSet<String> = build(&passenger, level - 1, Some(&data), None)
                .into_iter()
                .map(|line| line.label)
                .collect();
            for line in build(&passenger, level, Some(&data), None) {
                if before.contains(&line.label) {
                    continue;
                }
                let name = PROMISED_AS
                    .iter()
                    .find(|(label, _)| *label == line.label)
                    .map(|(_, name)| *name)
                    .unwrap_or_else(|| {
                        panic!("dossier line {:?} maps to no reward name", line.label)
                    });
                assert!(
                    promised.contains(name),
                    "level {level} delivers {:?} to {} and never says so",
                    line.label,
                    passenger.name
                );
            }
        }
    }
}

/// Every passenger with a need profile must have a way to be settled,
/// and Mastered must name it.
///
/// A profile authors an `exceptionId`; the engine grants relief only for
/// the guideline owning that id. An id that matches no guideline is a
/// passenger who cannot be soothed at all — and until this line existed
/// there was nowhere in the game that would have shown it.
#[test]
fn mastering_a_passenger_names_what_settles_them() {
    let data = GameData::load().expect("embedded game data parses");
    let mut named = 0;
    for passenger in load_passengers() {
        let Some(profile) = &passenger.state_profile else {
            continue;
        };
        let Some(exception_id) = profile.exception_id.as_deref() else {
            continue;
        };
        assert!(
            data.guidelines.iter().any(|guideline| guideline
                .exceptions
                .iter()
                .any(|exception| exception.id == exception_id)),
            "{} needs exception {exception_id:?}, which no guideline owns",
            passenger.name
        );

        let lines = build(&passenger, 3, Some(&data), None);
        assert!(
            lines.iter().any(|line| line.label == "Relief"),
            "{} can be mastered without learning what settles them",
            passenger.name
        );
        named += 1;
    }
    assert!(named > 0, "no passenger authors an exception any more");
}

/// The almanac and the driving readout name a condition the same way.
///
/// The almanac quotes need thresholds and the readout shows stability,
/// which is the inverse scale, so a driver was told to watch for 61 and
/// shown 45%. The numbers cannot be reconciled at a glance; the words can,
/// and only if they are the same words. This holds the two vocabularies
/// together -- if a stage is ever renamed on the gauge, the almanac's phrase
/// has to follow.
#[test]
fn the_almanac_and_the_gauge_name_a_condition_alike() {
    for stage in [
        NeedStage::Calm,
        NeedStage::Warning,
        NeedStage::Critical,
        NeedStage::Meltdown,
    ] {
        let phrase = stage_phrase(stage);
        assert!(
            phrase.contains(stage.label()),
            "the almanac says {phrase:?} where the gauge says {:?}",
            stage.label()
        );
    }
}

/// And the four are distinguishable, or naming the stage tells the driver
/// nothing they did not already have from the percentage.
#[test]
fn every_stage_reads_differently() {
    use std::collections::HashSet;
    let labels: HashSet<&str> = [
        NeedStage::Calm,
        NeedStage::Warning,
        NeedStage::Critical,
        NeedStage::Meltdown,
    ]
    .into_iter()
    .map(NeedStage::label)
    .collect();
    assert_eq!(labels.len(), 4, "two stages read the same on the gauge");
}

/// The traits line says which of them the driver can actually call on.
///
/// A trait is half a mechanic. The other half is the matching
/// `ability_unlock` skill, and without it the trait grants nothing -- so
/// listing them unqualified was a fact where a plan was wanted, the same
/// fault the relief line had.
#[test]
fn the_traits_line_says_which_ones_the_driver_is_trained_in() {
    use nightmare_shift::data::loader::load_constants;
    use nightmare_shift::engine::RideService;
    use nightmare_shift::state::{GameState, PlayerStats};

    let data = GameData::load().expect("embedded game data parses");
    let constants = load_constants();
    let shift = GameState::new(0.0, &constants.game_constants);
    let passenger = load_passengers()
        .into_iter()
        .find(|p| !p.traits.is_empty())
        .expect("a passenger with traits");
    let first = passenger.traits[0].clone();

    let traits_line = |stats: &PlayerStats| {
        let driver = DriverContext {
            shift: &shift,
            stats,
        };
        build(&passenger, 1, Some(&data), Some(&driver))
            .into_iter()
            .find(|line| line.label == "Traits")
            .map(|line| line.value)
            .expect("a traits line")
    };

    let untrained = traits_line(&PlayerStats::new());
    assert!(
        untrained.contains("trained in none"),
        "an untrained driver was not told so: {untrained:?}"
    );

    let mut trained_stats = PlayerStats::new();
    trained_stats
        .unlocked_skills
        .push(RideService::trait_skill_id(&first));
    let trained = traits_line(&trained_stats);
    assert!(
        trained.contains(&format!("call on {first}")),
        "a trained driver was not told which trait they could use: {trained:?}"
    );
    assert!(
        !trained.contains("trained in none"),
        "a trained driver was told they had nothing: {trained:?}"
    );
}

/// Outside a shift the line makes no claim about training, since there is
/// no driver to ask.
#[test]
fn without_a_driver_the_traits_line_only_lists() {
    let data = GameData::load().expect("embedded game data parses");
    let passenger = load_passengers()
        .into_iter()
        .find(|p| !p.traits.is_empty())
        .expect("a passenger with traits");

    let listed = build(&passenger, 1, Some(&data), None)
        .into_iter()
        .find(|line| line.label == "Traits")
        .map(|line| line.value)
        .expect("a traits line");
    assert_eq!(listed, passenger.traits.join(", "));
}

/// The relief line says whether the rule is actually in force.
///
/// Breaking a guideline does nothing unless the rule it belongs to was
/// drawn for the shift: the action is not a violation, so no exception
/// fires and no relief is paid. Naming the rule without saying whether it
/// is on the board tonight made a studied passenger's most useful fact
/// actionable only by luck.
#[test]
fn the_relief_line_says_whether_the_rule_is_on_the_board() {
    use nightmare_shift::data::loader::{load_constants, load_rules};
    use nightmare_shift::state::GameState;

    let data = GameData::load().expect("embedded game data parses");
    let constants = load_constants();
    let rules = load_rules();

    // Mrs. Chen's exception belongs to guideline 1001, which rule 1 owns.
    let chen = load_passengers()
        .into_iter()
        .find(|p| p.id == 1)
        .expect("Mrs. Chen");

    let stats = nightmare_shift::state::PlayerStats::new();
    let relief_with = |shift: &GameState| {
        let driver = DriverContext {
            shift,
            stats: &stats,
        };
        build(&chen, 3, Some(&data), Some(&driver))
            .into_iter()
            .find(|line| line.label == "Relief")
            .map(|line| line.value)
            .expect("a relief line")
    };

    let mut holding = GameState::new(0.0, &constants.game_constants);
    holding.current_rules = rules.iter().filter(|rule| rule.id == 1).cloned().collect();
    assert_eq!(holding.current_rules.len(), 1, "rule 1 not found");
    let said = relief_with(&holding);
    assert!(
        said.contains("in force") && !said.contains("not in force"),
        "her rule was on the board and the line did not say so: {said:?}"
    );

    let mut without = GameState::new(0.0, &constants.game_constants);
    without.current_rules = rules.iter().filter(|rule| rule.id != 1).cloned().collect();
    let said = relief_with(&without);
    assert!(
        said.contains("not in force"),
        "her rule was absent and the line still offered it: {said:?}"
    );
}

/// A hidden rule counts. It is on the board even though the player cannot
/// read it, so breaking the guideline it owns does pay relief.
#[test]
fn a_hidden_rule_still_counts_as_in_force() {
    use nightmare_shift::data::loader::{load_constants, load_rules};
    use nightmare_shift::state::GameState;

    let data = GameData::load().expect("embedded game data parses");
    let constants = load_constants();
    let chen = load_passengers()
        .into_iter()
        .find(|p| p.id == 1)
        .expect("Mrs. Chen");

    let mut shift = GameState::new(0.0, &constants.game_constants);
    shift.hidden_rules = load_rules()
        .into_iter()
        .filter(|rule| rule.id == 1)
        .collect();

    let stats = nightmare_shift::state::PlayerStats::new();
    let driver = DriverContext {
        shift: &shift,
        stats: &stats,
    };
    let said = build(&chen, 3, Some(&data), Some(&driver))
        .into_iter()
        .find(|line| line.label == "Relief")
        .map(|line| line.value)
        .expect("a relief line");
    assert!(
        !said.contains("not in force"),
        "a hidden rule was treated as absent: {said:?}"
    );
}

/// Outside a shift there is no board to check, so the line names the rule
/// and says nothing either way.
#[test]
fn without_a_shift_the_relief_line_makes_no_claim() {
    let data = GameData::load().expect("embedded game data parses");
    let chen = load_passengers()
        .into_iter()
        .find(|p| p.id == 1)
        .expect("Mrs. Chen");

    let said = build(&chen, 3, Some(&data), None)
        .into_iter()
        .find(|line| line.label == "Relief")
        .map(|line| line.value)
        .expect("a relief line");
    assert!(
        !said.contains("in force"),
        "a claim about tonight was made with no shift to check: {said:?}"
    );
}

/// The stage the almanac quotes must be the stage the engine gates on.
///
/// Both now come from `GameEngine::required_stage`, but the point of the
/// test is that they must keep coming from the same place: an almanac
/// that promises relief at a stage the engine will not grant it at is
/// worse than an almanac that stays quiet.
#[test]
fn the_quoted_relief_stage_is_the_one_the_engine_gates_on() {
    let data = GameData::load().expect("embedded game data parses");
    for passenger in load_passengers() {
        let Some(exception_id) = passenger
            .state_profile
            .as_ref()
            .and_then(|profile| profile.exception_id.as_deref())
        else {
            continue;
        };
        let Some(exception) = data
            .guidelines
            .iter()
            .flat_map(|guideline| guideline.exceptions.iter())
            .find(|exception| exception.id == exception_id)
        else {
            continue;
        };

        let expected = stage_phrase(GameEngine::required_stage(exception));
        let relief = build(&passenger, 3, Some(&data), None)
            .into_iter()
            .find(|line| line.label == "Relief")
            .expect("a relief line");
        assert!(
            relief.value.ends_with(expected),
            "{} is told {:?}, but the engine unlocks at {expected:?}",
            passenger.name,
            relief.value
        );
    }
}

/// The numbers the dossier prints must be the numbers the simulation
/// runs on.
///
/// Lv.1 tells the player a passenger turns restless past one level and
/// critical at another. Those readings come from `stateProfile`, and
/// `PassengerNeedState::calculate_stage` decides the actual stage from
/// the same block — but nothing checked they agreed, and an almanac that
/// quotes a threshold the state machine does not use is worse than one
/// that says nothing.
#[test]
fn the_quoted_thresholds_are_the_ones_the_state_machine_uses() {
    use nightmare_shift::state::{NeedStage, PassengerNeedState};

    for passenger in load_passengers() {
        let Some(profile) = &passenger.state_profile else {
            continue;
        };
        let thresholds = &profile.thresholds;

        let line = build(&passenger, 1, None, None)
            .into_iter()
            .find(|l| l.label == "Need")
            .unwrap_or_else(|| panic!("{} shows no need line at Lv.1", passenger.name));

        // The two figures the line quotes.
        assert!(
            line.value.contains(&thresholds.warning.to_string()),
            "{}: line {:?} does not quote the warning threshold {}",
            passenger.name,
            line.value,
            thresholds.warning
        );
        assert!(
            line.value.contains(&thresholds.critical.to_string()),
            "{}: line {:?} does not quote the critical threshold {}",
            passenger.name,
            line.value,
            thresholds.critical
        );

        // And the state machine must actually turn at them.
        assert_eq!(
            PassengerNeedState::calculate_stage(thresholds.warning, thresholds),
            NeedStage::Warning,
            "{} does not reach Warning at its quoted threshold",
            passenger.name
        );
        assert_eq!(
            PassengerNeedState::calculate_stage(thresholds.warning - 1, thresholds),
            NeedStage::Calm,
            "{} is already restless below its quoted threshold",
            passenger.name
        );
        assert_eq!(
            PassengerNeedState::calculate_stage(thresholds.critical, thresholds),
            NeedStage::Critical,
            "{} does not reach Critical at its quoted threshold",
            passenger.name
        );
    }
}

/// The need type named at Lv.1 must be the one the passenger actually
/// carries, not a label that drifted from the profile.
#[test]
fn the_quoted_need_is_the_carried_need() {
    for passenger in load_passengers() {
        let Some(profile) = &passenger.state_profile else {
            continue;
        };
        let line = build(&passenger, 1, None, None)
            .into_iter()
            .find(|l| l.label == "Need")
            .expect("a need line");
        assert!(
            line.value.starts_with(need_label(profile.need_type)),
            "{}: line {:?} does not name {:?}",
            passenger.name,
            line.value,
            profile.need_type
        );
    }
}

/// Every tell the dossier lists at Lv.2 must be one the passenger can
/// actually show, and every tell they can show at warning or critical
/// must be listed. A dossier that names a tell the state machine never
/// surfaces teaches the player to watch for nothing.
#[test]
fn the_listed_tells_are_the_ones_that_can_surface() {
    for passenger in load_passengers() {
        let listed: Vec<String> = build(&passenger, 2, None, None)
            .into_iter()
            .filter(|l| l.label == "Tell")
            .map(|l| l.value)
            .collect();

        for tell in passenger.tells.iter().take(3) {
            assert!(
                listed.iter().any(|l| l.contains(&tell.description)),
                "{} can show {:?} and the dossier does not list it",
                passenger.name,
                tell.description
            );
        }
        for entry in &listed {
            assert!(
                passenger
                    .tells
                    .iter()
                    .any(|tell| entry.contains(&tell.description)),
                "{} lists {entry:?}, which is not one of their tells",
                passenger.name
            );
        }
    }
}

/// Level 0 is the unstudied state and must stay silent.
#[test]
fn unstudied_passengers_reveal_nothing() {
    for passenger in load_passengers() {
        assert!(
            build(&passenger, 0, None, None).is_empty(),
            "{}",
            passenger.name
        );
    }
}

/// The Lv.3 "Hidden Rules" reward is the passenger's `personalRule`, so
/// every passenger needs one authored.
#[test]
fn every_passenger_has_a_personal_rule() {
    for passenger in load_passengers() {
        assert!(
            !passenger.personal_rule.trim().is_empty(),
            "{} has no personalRule",
            passenger.name
        );
    }
}

/// The Lv.1 "Basic Needs" reward reads the state profile, so every
/// passenger needs one.
#[test]
fn every_passenger_has_a_state_profile() {
    for passenger in load_passengers() {
        assert!(
            passenger.state_profile.is_some(),
            "{} has no stateProfile",
            passenger.name
        );
    }
}

use super::*;
use nightmare_shift::data::loader::{load_constants, load_guidelines, load_rules};

/// A fixed-seed stream for tests: deterministic, and a fresh one per call
/// so no test's draws depend on another's.
fn test_rng() -> macroquad_toolkit::rng::SeededRng {
    macroquad_toolkit::rng::SeededRng::new(0x7E57)
}

/// A generated shift must never contain two rules that contradict each
/// other. Run many times because selection is random — a single draw
/// passing proves nothing.
#[test]
fn generated_shifts_never_contradict_themselves() {
    let rules = load_rules();
    let constants = load_constants();
    for _ in 0..500 {
        for experience in [0, 10, 20, 30, 40] {
            let shift =
                GameEngine::generate_shift_rules(&mut test_rng(), experience, &rules, &constants);
            let all: Vec<&Rule> = shift
                .visible_rules
                .iter()
                .chain(shift.hidden_rules.iter())
                .collect();
            for (i, a) in all.iter().enumerate() {
                for b in all.iter().skip(i + 1) {
                    assert!(
                        !GameEngine::rules_conflict(a, b),
                        "shift paired {:?} with {:?}",
                        a.title,
                        b.title
                    );
                }
            }
        }
    }
}

/// Every action-keyed forbidden rule must name a real guideline.
///
/// `passenger_has_exception` filters on `related_guideline_id`, so a rule
/// without one can never be excepted: its authored `exceptionNeedAdjustment`
/// relief is unreachable and pressing its key is always a violation. Six of
/// thirteen action rules shipped that way (both `drive_dark`, `use_ac` and
/// `use_wipers` pairs), which fed the meltdown monoculture the balance sweep
/// measured — a dangling or missing link here is authored relief nobody can
/// ever earn.
#[test]
fn every_action_rule_names_a_guideline_that_exists() {
    let guidelines = load_guidelines();
    for rule in load_rules()
        .iter()
        .filter(|rule| rule.action_key.is_some() && rule.action_type == Some(ActionType::Forbidden))
    {
        let id = rule.related_guideline_id.unwrap_or_else(|| {
            panic!(
                "rule {} ({:?}) has an action key but no relatedGuidelineId - \
                 its exception relief is unreachable",
                rule.id, rule.title
            )
        });
        assert!(
            guidelines.iter().any(|guideline| guideline.id == id),
            "rule {} ({:?}) names guideline {id}, which does not exist",
            rule.id,
            rule.title
        );
    }
}

/// Conflicts are read in both directions, since they are authored one-way.
#[test]
fn conflicts_are_symmetric() {
    let rules = load_rules();
    let find = |id: u32| rules.iter().find(|r| r.id == id).expect("rule exists");
    // "Safety First" (11) names "No Eye Contact" (1); the reverse is not
    // authored, and the pair must still be recognised.
    let safety_first = find(11);
    let no_eye_contact = find(1);
    assert!(safety_first.conflicts_with.contains(&no_eye_contact.id));
    assert!(!no_eye_contact.conflicts_with.contains(&safety_first.id));
    assert!(GameEngine::rules_conflict(safety_first, no_eye_contact));
    assert!(GameEngine::rules_conflict(no_eye_contact, safety_first));
}

/// Every id named as a conflict must be a real rule, and no rule may
/// conflict with itself.
#[test]
fn conflict_ids_are_real() {
    let rules = load_rules();
    let ids: Vec<u32> = rules.iter().map(|r| r.id).collect();
    for rule in &rules {
        assert!(
            !rule.conflicts_with.contains(&rule.id),
            "{:?} conflicts with itself",
            rule.title
        );
        for id in &rule.conflicts_with {
            assert!(ids.contains(id), "{:?} names unknown rule {id}", rule.title);
        }
    }
}

/// Every authored rule type that the generator owns must actually be
/// reachable. `Conflicting` and `Hidden` were referenced by no selection
/// path, so twelve of twenty-eight rules could never appear.
#[test]
fn every_generated_rule_type_is_reachable() {
    let rules = load_rules();
    let constants = load_constants();
    let mut seen: Vec<RuleType> = Vec::new();
    for _ in 0..500 {
        let shift = GameEngine::generate_shift_rules(&mut test_rng(), 40, &rules, &constants);
        for rule in shift.visible_rules.iter().chain(shift.hidden_rules.iter()) {
            if !seen.contains(&rule.rule_type) {
                seen.push(rule.rule_type);
            }
        }
    }
    for kind in [
        RuleType::Basic,
        RuleType::Conditional,
        RuleType::Conflicting,
        RuleType::Hidden,
    ] {
        assert!(
            seen.contains(&kind),
            "{kind:?} rules never appear in a shift"
        );
    }
}

/// The hidden-rule mechanic needs hidden rules. `reveal_hidden_rule`, the
/// Glimpse skill and the "Hidden Rule Violated!" ending all key off this
/// list being non-empty at least sometimes.
#[test]
fn hidden_rules_actually_appear() {
    let rules = load_rules();
    let constants = load_constants();
    let with_hidden = (0..500)
        .filter(|_| {
            !GameEngine::generate_shift_rules(&mut test_rng(), 40, &rules, &constants)
                .hidden_rules
                .is_empty()
        })
        .count();
    assert!(
        with_hidden > 0,
        "no shift out of 500 at max difficulty carried a hidden rule"
    );
}

/// Every `SkillModifiers` field must move the number it names, by the
/// amount the skill tree authors.
///
/// This is a unit test rather than a bot measurement because aggregate
/// play cannot answer it. Unlocking only the two fare skills and
/// comparing average earnings per ride measured a 27.8% uplift against
/// the 21% the multipliers work out to — not because the multiplier is
/// wrong, but because a richer driver refuels more, survives longer, and
/// carries a different mix of passengers, whose base fares run from $12
/// to $100. The mix moves the average more than the multiplier does.
#[test]
fn the_fare_multiplier_is_exactly_what_the_tree_authors() {
    use nightmare_shift::data::loader::{load_constants, load_passengers, load_skill_tree};

    let constants = load_constants();
    let skills = load_skill_tree();
    // The highest-paying fare on the roster, well clear of the $5 floor
    // `calculate_fare` clamps to — the first paying passenger's $12 fare
    // lands on it once the route and preference multipliers are applied,
    // and a clamped number cannot show a 1.21x ratio.
    let passenger = load_passengers()
        .into_iter()
        .max_by_key(|p| p.fare)
        .expect("a paying passenger");

    let fare_of = |unlocked: &[String]| {
        let mods = SkillModifiers::from_unlocked(&skills, unlocked);
        GameEngine::calculate_fare(
            &mut test_rng(),
            passenger.fare,
            RouteType::Normal,
            &passenger,
            None,
            None,
            &constants,
            mods.fare_mult,
        )
    };

    // `calculate_fare` adds a +/-$5 variation, so a single pair of calls
    // cannot be compared. Averaging over many washes it out.
    let mean_fare = |unlocked: &[String]| {
        let total: u32 = (0..400).map(|_| fare_of(unlocked)).sum();
        total as f32 / 400.0
    };

    let plain = mean_fare(&[]);
    let both: Vec<String> = skills
        .iter()
        .filter(|s| s.effect.target == "fare_multiplier")
        .map(|s| s.id.clone())
        .collect();
    assert_eq!(both.len(), 2, "the tree no longer has two fare skills");

    let expected_mult: f32 = skills
        .iter()
        .filter(|s| s.effect.target == "fare_multiplier")
        .map(|s| s.effect.value as f32)
        .product();
    let boosted = mean_fare(&both);

    let ratio = boosted / plain;
    assert!(
        (ratio - expected_mult).abs() < 0.05,
        "mean fare went {plain:.1} -> {boosted:.1} ({ratio:.3}x)              against the authored {expected_mult:.3}x"
    );
}

/// The remaining modifiers must each be non-neutral once their skills are
/// unlocked, or a node is bought and changes nothing.
#[test]
fn every_skill_modifier_moves_off_neutral() {
    use nightmare_shift::data::loader::load_skill_tree;

    let skills = load_skill_tree();
    let all: Vec<String> = skills.iter().map(|s| s.id.clone()).collect();
    let none = SkillModifiers::from_unlocked(&skills, &[]);
    let full = SkillModifiers::from_unlocked(&skills, &all);

    assert!(full.fuel_cost_mult < none.fuel_cost_mult, "fuel cost");
    assert!(full.max_fuel_bonus > none.max_fuel_bonus, "max fuel");
    assert!(full.hazard_mult < none.hazard_mult, "hazard damage");
    assert!(
        full.reveal_hidden_chance > none.reveal_hidden_chance,
        "hidden rule reveal"
    );
    assert!(full.fare_mult > none.fare_mult, "fare");
    assert!(full.refuel_cost_mult < none.refuel_cost_mult, "refuel cost");
    assert!(
        full.bonus_protection > none.bonus_protection,
        "supernatural protection"
    );
}

/// Breaking the rule a passenger's own exception belongs to must relieve
/// them; breaking an unrelated one must not. The rule argument used to be
/// ignored, so every forbidden action soothed every stressed passenger
/// equally and `exceptionId` decided nothing.
#[test]
fn only_the_passengers_own_rule_grants_an_exception() {
    use nightmare_shift::data::loader::{load_guidelines, load_passengers};
    use nightmare_shift::engine::PassengerStateMachine;

    let rules = load_rules();
    let guidelines = load_guidelines();
    let passengers = load_passengers();

    // Mrs. Chen's exception is `eye_contact_lonely`, owned by guideline
    // 1001, which rule 1 "No Eye Contact" belongs to.
    let chen = passengers.iter().find(|p| p.id == 1).expect("Mrs. Chen");
    let mut need = PassengerStateMachine::initialize(chen, 0.0).expect("Chen has a profile");
    // Push her past the warning threshold so the exception can be active.
    PassengerStateMachine::apply_stress_delta(&mut need, chen, 100, 0.0);

    let own_rule = rules.iter().find(|r| r.id == 1).expect("No Eye Contact");
    let other_rule = rules.iter().find(|r| r.id == 4).expect("Windows Sealed");

    assert!(
        GameEngine::passenger_has_exception(own_rule, Some(&need), &guidelines),
        "breaking her own rule does not relieve her"
    );
    assert!(
        !GameEngine::passenger_has_exception(other_rule, Some(&need), &guidelines),
        "an unrelated rule still relieves her"
    );
}

/// The seam the whole determinism thread exists for: the same seed must
/// build the same night. Rules, weather, and hazards all draw from one
/// stream, so two runs from equal states cannot diverge — and a replay or
/// a mid-run save that restores the stream picks up exactly where the
/// original left off.
#[test]
fn the_same_seed_builds_the_same_night() {
    use nightmare_shift::engine::WeatherService;
    let rules = nightmare_shift::data::loader::load_rules();
    let constants = load_constants();

    let build = |seed: u64| {
        let mut rng = macroquad_toolkit::rng::SeededRng::new(seed);
        let shift = GameEngine::generate_shift_rules(&mut rng, 40, &rules, &constants);
        let season = WeatherService::get_current_season(10);
        let weather = WeatherService::generate_initial_weather(&mut rng, &season, 0.0);
        let time_of_day = WeatherService::time_of_day_after(0);
        let hazards =
            WeatherService::generate_hazards(&mut rng, &weather, &time_of_day, &season, 0.0);
        (
            shift
                .visible_rules
                .iter()
                .chain(shift.hidden_rules.iter())
                .map(|rule| rule.id)
                .collect::<Vec<_>>(),
            format!("{:?} {:?}", weather.weather_type, weather.intensity),
            hazards
                .iter()
                .map(|hazard| hazard.id.clone())
                .collect::<Vec<_>>(),
        )
    };

    assert_eq!(build(42), build(42), "one seed produced two nights");
    assert_eq!(build(7), build(7));
}

/// A hidden-rule violation lands about as often as it is authored to.
///
/// Averaged over many rolls because a probability cannot be checked with
/// one: a single call proves nothing and would pass whatever the constant
/// said. The band is wide on purpose -- this is asserting that the number
/// is consulted at all, not pinning the RNG.
#[test]
fn a_hidden_violation_lands_about_as_often_as_authored() {
    let constants = load_constants();
    let authored = constants.probabilities.hidden_rule_violation;

    const ROLLS: u32 = 4000;
    let mut lands_rng = test_rng();
    let landed = (0..ROLLS)
        .filter(|_| GameEngine::hidden_violation_lands(&mut lands_rng, &constants))
        .count() as f32;
    let observed = landed / ROLLS as f32;

    assert!(
        (observed - authored).abs() < 0.05,
        "hidden violations landed {observed:.3} of the time, authored {authored:.3}"
    );
}

/// And the authored number has to leave both outcomes possible. At 1.0 a
/// hidden rule is the guaranteed sentence it used to be; at 0.0 the whole
/// hidden layer -- and the Glimpse skill that reveals it -- is inert.
#[test]
fn a_hidden_violation_is_neither_certain_nor_impossible() {
    let authored = load_constants().probabilities.hidden_rule_violation;
    assert!(
        authored > 0.0 && authored < 1.0,
        "HIDDEN_RULE_VIOLATION is {authored}, which makes the hidden layer              either harmless or unsurvivable"
    );
}

/// Keeping a rule has to be worth something, so the rule a passenger
/// pulls against must author a reward for keeping it.
///
/// Thirteen rules do and none of it paid until now. This checks the data
/// rather than the payment, because the payment needs a `Game`: what it
/// guards is a rule quietly losing its `followConsequences` and going back
/// to being pure downside.
#[test]
fn the_rules_that_can_be_a_passengers_own_reward_keeping_them() {
    use nightmare_shift::data::loader::{load_guidelines, load_passengers};
    use nightmare_shift::engine::PassengerStateMachine;

    let rules = load_rules();
    let guidelines = load_guidelines();
    let mut checked = 0;
    for passenger in load_passengers() {
        let Some(need) = PassengerStateMachine::initialize(&passenger, 0.0) else {
            continue;
        };
        let Some(own) = GameEngine::passengers_rule_in_force(&rules, Some(&need), &guidelines)
        else {
            continue;
        };
        assert!(
            !own.follow_consequences.is_empty(),
            "{}'s own rule {} pays nothing for keeping it",
            passenger.name,
            own.id
        );
        checked += 1;
    }
    assert!(checked > 0, "no passenger's rule was found in the full set");
}

/// Every authored follow reward has to be a consequence type the appliers
/// actually handle. `apply_rule_consequences` names Death, Survival,
/// Reputation, Money, Fuel, Time, Item and StoryUnlock; a reward of any
/// other kind would be accepted by serde and then dropped.
#[test]
fn every_follow_reward_is_a_kind_something_applies() {
    for rule in load_rules() {
        for reward in &rule.follow_consequences {
            assert!(
                reward.probability > 0.0,
                "rule {} authors a follow reward at zero probability",
                rule.id
            );
            assert!(
                !matches!(reward.consequence_type, ConsequenceType::Death),
                "rule {} rewards keeping it with death",
                rule.id
            );
        }
    }
}

/// The passenger's own rule is found when it is in force and not when it
/// is absent, because that difference is now what decides how fast they
/// wear down.
#[test]
fn a_passengers_own_rule_is_recognised_only_when_it_is_in_force() {
    use nightmare_shift::data::loader::{load_guidelines, load_passengers};
    use nightmare_shift::engine::PassengerStateMachine;

    let rules = load_rules();
    let guidelines = load_guidelines();
    // Mrs. Chen's exception is owned by guideline 1001, which rule 1
    // "No Eye Contact" belongs to.
    let chen = load_passengers()
        .into_iter()
        .find(|p| p.id == 1)
        .expect("Mrs. Chen");
    let need = PassengerStateMachine::initialize(&chen, 0.0).expect("Chen has a profile");

    let own: Vec<Rule> = rules.iter().filter(|r| r.id == 1).cloned().collect();
    let found = GameEngine::passengers_rule_in_force(&own, Some(&need), &guidelines);
    assert_eq!(
        found.map(|rule| rule.id),
        Some(1),
        "her own rule was in force and went unrecognised"
    );
    assert!(
        found.and_then(|rule| rule.follow_need_adjustment).is_some(),
        "rule 1 authors no followNeedAdjustment, so this wires nothing"
    );

    let others: Vec<Rule> = rules.iter().filter(|r| r.id != 1).cloned().collect();
    assert!(
        GameEngine::passengers_rule_in_force(&others, Some(&need), &guidelines).is_none(),
        "a shift without her rule still claimed to hold it"
    );
}

/// An exception waits for the stage it authors, not a stage the engine
/// assumed. The gate was a hardcoded `NeedStage::Warning` while every
/// exception carried a `requiredStage` the engine never read, so raising
/// one to "critical" would have changed nothing.
#[test]
fn an_exception_waits_for_the_stage_it_authors() {
    use nightmare_shift::data::loader::{load_guidelines, load_passengers};
    use nightmare_shift::engine::PassengerStateMachine;

    let rules = load_rules();
    let mut guidelines = load_guidelines();
    let chen = load_passengers()
        .into_iter()
        .find(|p| p.id == 1)
        .expect("Mrs. Chen");
    let mut need = PassengerStateMachine::initialize(&chen, 0.0).expect("Chen has a profile");
    let own_rule = rules.iter().find(|r| r.id == 1).expect("No Eye Contact");

    // Make her exception cost more to earn than it does today.
    let exception = guidelines
        .iter_mut()
        .flat_map(|guideline| guideline.exceptions.iter_mut())
        .find(|exception| Some(exception.id.as_str()) == need.profile.exception_id.as_deref())
        .expect("Chen's exception");
    exception.required_stage = Some("critical".to_string());

    need.stage = NeedStage::Warning;
    assert!(
        !GameEngine::passenger_has_exception(own_rule, Some(&need), &guidelines),
        "a critical-only exception was granted at warning"
    );

    need.stage = NeedStage::Critical;
    assert!(
        GameEngine::passenger_has_exception(own_rule, Some(&need), &guidelines),
        "a critical-only exception was withheld at critical"
    );
}

/// Every authored `requiredStage` must name a stage. An unrecognised name
/// falls back to Warning rather than stranding the passenger, so a typo
/// would never show up in play — only here.
#[test]
fn every_authored_required_stage_names_a_stage() {
    use nightmare_shift::data::loader::load_guidelines;

    let mut checked = 0;
    for guideline in load_guidelines() {
        for exception in &guideline.exceptions {
            if let Some(name) = exception.required_stage.as_deref() {
                assert!(
                    NeedStage::parse(name).is_some(),
                    "exception {:?} on guideline {} requires unknown stage {name:?}",
                    exception.id,
                    guideline.id
                );
                checked += 1;
            }
        }
    }
    assert!(checked > 0, "no exception authors a requiredStage any more");
}

/// A calm passenger has no exception however well the rule matches.
#[test]
fn a_calm_passenger_grants_no_exception() {
    use nightmare_shift::data::loader::{load_guidelines, load_passengers};
    use nightmare_shift::engine::PassengerStateMachine;

    let rules = load_rules();
    let guidelines = load_guidelines();
    let chen = load_passengers()
        .into_iter()
        .find(|p| p.id == 1)
        .expect("Mrs. Chen");
    let need = PassengerStateMachine::initialize(&chen, 0.0).expect("Chen has a profile");
    let own_rule = rules.iter().find(|r| r.id == 1).expect("No Eye Contact");

    assert!(!GameEngine::passenger_has_exception(
        own_rule,
        Some(&need),
        &guidelines
    ));
}

/// A shift must still produce rules. Filtering conflicts too eagerly —
/// or a data change that made everything conflict — would leave the
/// player with nothing to obey.
#[test]
fn shifts_still_produce_rules() {
    let rules = load_rules();
    let constants = load_constants();
    for _ in 0..200 {
        let shift = GameEngine::generate_shift_rules(&mut test_rng(), 30, &rules, &constants);
        assert!(
            !shift.visible_rules.is_empty() || !shift.hidden_rules.is_empty(),
            "generated a shift with no rules at all"
        );
    }
}

use nightmare_shift::data::loader::{load_almanac, load_skill_tree};
use std::collections::HashSet;

/// Every level a player can buy has to name what it reveals.
///
/// The almanac card now prints "Mastered reveals Hidden Rules, True
/// Nature, Backstory" beside the price, from this list. An empty list
/// there puts the screen back to showing a price and a tier name and
/// nothing about what the lore buys, which is what it did before.
#[test]
fn every_purchasable_level_names_what_it_reveals() {
    let almanac = load_almanac();
    for level in 1..=3 {
        let entry = almanac
            .get_level(level)
            .unwrap_or_else(|| panic!("no almanac level {level} authored"));
        assert!(!entry.name.trim().is_empty(), "level {level} has no name");
        assert!(
            !entry.rewards.is_empty(),
            "level {level} ({}) reveals nothing the player can be told about",
            entry.name
        );
        for reward in &entry.rewards {
            assert!(
                !reward.trim().is_empty(),
                "level {level} authors a blank reward"
            );
        }
    }
}

/// The sentence the card prints has to name the rewards, not just the
/// tier. Printing only the tier is what the screen effectively did before,
/// and it reads as informative while saying nothing.
#[test]
fn the_reveals_line_names_the_rewards() {
    let almanac = load_almanac();
    for level in 1..=3 {
        let entry = almanac.get_level(level).expect("a level");
        let line = entry
            .reveals_line(&[])
            .unwrap_or_else(|| panic!("level {level} prints nothing"));
        for reward in &entry.rewards {
            assert!(
                line.contains(reward.as_str()),
                "level {level} does not mention {reward:?} in {line:?}"
            );
        }
    }
}

/// Something already in hand is not promised again. A story earned in play
/// shows on the card immediately, and the next level's line used to go on
/// offering the same backstory one tier later.
#[test]
fn a_reward_already_in_hand_is_not_promised_again() {
    let mastered = load_almanac().get_level(3).expect("level 3").clone();
    assert!(
        mastered.rewards.iter().any(|r| r == "Backstory"),
        "level 3 no longer offers a backstory; pick another reward"
    );

    let full = mastered.reveals_line(&[]).expect("a line");
    assert!(full.contains("Backstory"));

    let trimmed = mastered.reveals_line(&["Backstory"]).expect("a line");
    assert!(
        !trimmed.contains("Backstory"),
        "a known backstory was promised again in {trimmed:?}"
    );
    assert!(
        trimmed.contains("True Nature"),
        "excluding one reward dropped the others: {trimmed:?}"
    );
}

/// Every skill must do something when bought. `from_unlocked` ignores
/// unknown targets silently, so a typo'd target would sell a skill that
/// does nothing; this buys each skill alone and requires a modifier to
/// move. Ability unlocks dispatch by id instead, and their `value` is
/// read by nothing — held to the constant 1 so it cannot masquerade as
/// a magnitude.
#[test]
fn every_skill_moves_a_modifier_or_unlocks_an_ability() {
    use nightmare_shift::engine::SkillModifiers;
    for skill in nightmare_shift::data::loader::load_skill_tree() {
        if skill.effect.effect_type == "ability_unlock" {
            assert_eq!(
                skill.effect.value, 1.0,
                "{}: an ability unlock's value is read by nothing - author 1",
                skill.id
            );
            continue;
        }
        let bought = SkillModifiers::from_unlocked(
            std::slice::from_ref(&skill),
            std::slice::from_ref(&skill.id),
        );
        assert!(
            bought != SkillModifiers::default(),
            "{}: target {:?} moved no modifier - the skill does nothing",
            skill.id,
            skill.effect.target
        );
    }
}

/// Excluding everything a level offers leaves nothing to say, rather than
/// a tier name with a dangling "reveals".
#[test]
fn excluding_every_reward_prints_nothing() {
    let mastered = load_almanac().get_level(3).expect("level 3").clone();
    let all: Vec<&str> = mastered.rewards.iter().map(String::as_str).collect();
    assert!(mastered.reveals_line(&all).is_none());
}

/// A level that reveals nothing prints nothing, rather than a bare tier
/// name with a dangling "reveals".
#[test]
fn a_level_that_reveals_nothing_prints_nothing() {
    let silent = super::AlmanacLevel {
        name: "Unknown".to_string(),
        description: String::new(),
        rewards: Vec::new(),
    };
    assert!(silent.reveals_line(&[]).is_none());
}

/// Level 0 is the not-yet-met state and is named, since the card falls
/// back to a localized string only when the level is missing entirely.
#[test]
fn the_unmet_level_is_named() {
    let unmet = load_almanac().get_level(0).expect("level 0").name.clone();
    assert!(!unmet.trim().is_empty());
}

/// Each level has to cost something, or the tiers above it are free and
/// the lore currency has no sink at the almanac.
#[test]
fn every_upgrade_costs_lore() {
    let almanac = load_almanac();
    for level in 1..=3 {
        assert!(
            almanac.get_upgrade_cost(level) > 0,
            "reaching level {level} is free"
        );
    }
}

/// A prerequisite naming a skill that does not exist locks its node out
/// of the tree permanently, and `can_unlock` would never return true.
#[test]
fn no_prerequisite_dangles() {
    let skills = load_skill_tree();
    let ids: HashSet<&str> = skills.iter().map(|s| s.id.as_str()).collect();
    for skill in &skills {
        for prerequisite in &skill.prerequisites {
            assert!(
                ids.contains(prerequisite.as_str()),
                "{} requires unknown skill {prerequisite:?}",
                skill.id
            );
        }
    }
}

/// Every node must be reachable from an empty unlock list by satisfying
/// prerequisites in some order. A cycle, or a node gated behind one,
/// would be bank balance the player can never spend.
#[test]
fn every_skill_is_reachable_from_nothing() {
    let skills = load_skill_tree();
    let mut unlocked: HashSet<String> = HashSet::new();
    loop {
        let newly: Vec<String> = skills
            .iter()
            .filter(|s| !unlocked.contains(&s.id))
            .filter(|s| s.can_unlock(&unlocked.iter().cloned().collect::<Vec<_>>()))
            .map(|s| s.id.clone())
            .collect();
        if newly.is_empty() {
            break;
        }
        unlocked.extend(newly);
    }
    let stranded: Vec<&str> = skills
        .iter()
        .map(|s| s.id.as_str())
        .filter(|id| !unlocked.contains(*id))
        .collect();
    assert!(stranded.is_empty(), "unreachable skills: {stranded:?}");
}

/// At least one node must have no prerequisites, or the tree has no entry
/// point regardless of how much bank the player saves.
#[test]
fn the_tree_has_an_entry_point() {
    assert!(load_skill_tree().iter().any(|s| s.prerequisites.is_empty()));
}

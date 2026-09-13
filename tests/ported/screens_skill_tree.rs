use super::*;
use nightmare_shift::data::loader::{load_passengers, load_skill_tree};

/// A fresh driver has studied nobody, so every ability skill reads as
/// zero-of-something and the card warns rather than looking ready.
#[test]
fn a_new_driver_has_no_one_to_use_an_ability_on() {
    let passengers = load_passengers();
    let stats = PlayerStats::default();
    let mut ability_skills = 0;
    for skill in load_skill_tree() {
        let Some((studied, total)) = ability_carriers(&skill, &passengers, &stats) else {
            continue;
        };
        ability_skills += 1;
        assert_eq!(studied, 0, "{} reads as studied on a fresh save", skill.id);
        assert!(total > 0, "{} names a trait no passenger carries", skill.id);
    }
    assert!(ability_skills > 0, "no ability skills to check");
}

/// Studying a carrier moves that skill's count, so the line is live
/// rather than a fixed caption.
#[test]
fn studying_a_carrier_moves_the_count() {
    let passengers = load_passengers();
    let skill = load_skill_tree()
        .into_iter()
        .find(|skill| skill.effect.effect_type == "ability_unlock")
        .expect("an ability skill");
    let carrier = passengers
        .iter()
        .find(|passenger| {
            passenger
                .traits
                .iter()
                .any(|name| RideService::trait_skill_id(name) == skill.effect.target)
        })
        .expect("a carrier");

    let mut stats = PlayerStats::default();
    let before = ability_carriers(&skill, &passengers, &stats)
        .expect("counts")
        .0;
    stats.mark_passenger_encountered(carrier.id);
    stats.lore_fragments += 999;
    stats.upgrade_almanac_knowledge(carrier.id, 0);
    let after = ability_carriers(&skill, &passengers, &stats)
        .expect("counts")
        .0;

    assert_eq!(before + 1, after, "studying a carrier changed nothing");
}

/// Skills that are not ability unlocks have no carrier line at all.
#[test]
fn an_ordinary_skill_has_no_carrier_line() {
    let passengers = load_passengers();
    let stats = PlayerStats::default();
    let skill = load_skill_tree()
        .into_iter()
        .find(|skill| skill.effect.effect_type != "ability_unlock")
        .expect("an ordinary skill");
    assert!(ability_carriers(&skill, &passengers, &stats).is_none());
}

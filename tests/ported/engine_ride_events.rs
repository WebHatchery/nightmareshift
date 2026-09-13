use super::*;
use nightmare_shift::data::loader::{load_constants, load_passengers, load_skill_tree};

/// The ability choice is the one place the almanac and the skill tree pay
/// off together, and the whole link is a string convention: a passenger's
/// `"Night Vision"` trait finds the `night_vision` skill by lowercasing and
/// swapping spaces. Nothing declares that correspondence, so renaming a
/// trait or a skill id on either side deletes the choice in silence -- no
/// compile error, no failing test, just a strongest option that stops being
/// offered. Both directions matter, so both are asserted.
#[test]
fn every_passenger_trait_names_a_skill_that_can_be_bought() {
    let skills = load_skill_tree();
    for passenger in load_passengers() {
        for trait_name in &passenger.traits {
            let skill_id = RideService::trait_skill_id(trait_name);
            assert!(
                skills.iter().any(|skill| skill.id == skill_id),
                "{} has the trait {trait_name:?}, which needs a {skill_id:?} skill \
                 to ever help anyone, and the tree has no such skill",
                passenger.name
            );
        }
    }
}

/// The other direction: an `ability_unlock` skill is bank spent on a trait
/// nobody has unless some passenger carries it.
#[test]
fn every_ability_skill_is_a_trait_some_passenger_has() {
    let passengers = load_passengers();
    for skill in load_skill_tree()
        .iter()
        .filter(|skill| skill.effect.effect_type == "ability_unlock")
    {
        assert!(
            passengers.iter().any(|passenger| {
                passenger
                    .traits
                    .iter()
                    .any(|name| RideService::trait_skill_id(name) == skill.id)
            }),
            "the {} skill costs {} and no passenger has the matching trait, \
             so it can never add an ability choice",
            skill.name,
            skill.cost
        );
    }
}

/// And the wire itself: studying a passenger and owning their ability has
/// to actually put the choice on the screen, and neither half alone may.
#[test]
fn the_ability_choice_needs_the_almanac_and_the_skill_together() {
    let data = GameData::load().expect("embedded game data parses");
    let constants = load_constants();
    let mut state = GameState::new(0.0, &constants.game_constants);

    let passenger = load_passengers()
        .into_iter()
        .find(|p| !p.traits.is_empty())
        .expect("a passenger with a trait");
    let trait_name = passenger.traits[0].clone();
    let skill_id = RideService::trait_skill_id(&trait_name);
    let passenger_id = passenger.id;
    state.current_passenger = Some(passenger);

    let offers_ability = |stats: &PlayerStats| {
        (0..40).any(|_| {
            RideService::generate_mid_ride_event(
                &mut macroquad_toolkit::rng::SeededRng::new(0xE7),
                &state,
                &data,
                stats,
                RouteType::Normal,
            )
            .choices
            .iter()
            .any(|choice| choice.required_trait.as_deref() == Some(trait_name.as_str()))
        })
    };

    let mut neither = PlayerStats::new();
    assert!(
        !offers_ability(&neither),
        "offered with no almanac, no skill"
    );

    let mut skill_only = PlayerStats::new();
    skill_only.unlocked_skills.push(skill_id.clone());
    assert!(
        !offers_ability(&skill_only),
        "the skill alone offered the choice -- studying the passenger is \
         supposed to be the other half of the price"
    );

    neither.almanac_progress.insert(
        passenger_id,
        AlmanacEntry {
            passenger_id,
            encountered: true,
            knowledge_level: 1,
            ..AlmanacEntry::default()
        },
    );
    assert!(
        !offers_ability(&neither),
        "the almanac alone offered the choice"
    );

    let mut both = neither.clone();
    both.unlocked_skills.push(skill_id);
    assert!(
        offers_ability(&both),
        "studied the passenger and bought {trait_name:?}, and the choice \
         never appeared"
    );
}

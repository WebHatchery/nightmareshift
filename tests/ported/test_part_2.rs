use super::*;

#[test]
fn every_ability_skill_has_a_passenger_that_uses_it() {
    let passengers = load_passengers();
    let traits: HashSet<String> = passengers
        .iter()
        .flat_map(|p| p.traits.iter())
        .map(|t| RideService::trait_skill_id(t))
        .collect();
    for skill in load_skill_tree() {
        if skill.effect.effect_type != "ability_unlock" {
            continue;
        }
        assert!(
            traits.contains(&skill.effect.target),
            "no passenger has a trait for ability skill {:?}",
            skill.id
        );
    }
}
#[test]
fn a_gift_is_not_spent_on_an_empty_back_seat() {
    let constants = load_constants();
    let catalog = load_item_catalog();
    let mut checked = 0;

    for name in catalog.names() {
        let item = catalog.create_item(&name, "test", 0.0);
        let needs_company = item
            .effects
            .iter()
            .any(|effect| matches!(effect.effect_type, ItemEffectType::ReputationModifier));
        if !item.can_use || !needs_company {
            continue;
        }
        checked += 1;

        let mut empty_cab = GameState::new(0.0, &constants.game_constants);
        empty_cab.inventory.push(item.clone());
        assert!(
            !ItemService::use_item(&mut empty_cab, 0, &constants.reputation, 0.0),
            "the {name} was used with nobody in the cab"
        );
        assert_eq!(
            empty_cab.inventory.len(),
            1,
            "the {name} was destroyed with nobody to give it to"
        );
        let told = empty_cab
            .current_dialogue
            .as_ref()
            .map(|dialogue| dialogue.text.clone())
            .unwrap_or_default();
        assert!(
            told.contains("back seat"),
            "the {name} refused with {told:?}, which does not say the cab is empty"
        );

        let mut with_fare = GameState::new(0.0, &constants.game_constants);
        with_fare.current_passenger = load_passengers().into_iter().next();
        with_fare.inventory.push(item);
        assert!(
            ItemService::use_item(&mut with_fare, 0, &constants.reputation, 0.0),
            "the {name} did nothing even with a passenger aboard"
        );
    }

    assert!(checked > 0, "no reputation items found to check");
}
#[test]
fn knowing_a_backstory_keeps_its_drop_bonus() {
    let constants = load_constants();
    let passenger = load_passengers().into_iter().next().expect("a passenger");
    let unknown =
        ItemService::calculate_drop_chance(&passenger, RouteType::Normal, false, &constants);
    let known = ItemService::calculate_drop_chance(&passenger, RouteType::Normal, true, &constants);
    assert!(
        known > unknown,
        "known story {known} did not beat {unknown}"
    );
}

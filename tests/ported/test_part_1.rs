use super::*;

#[test]
fn every_passenger_category_names_a_stocked_pool() {
    let pools = load_item_pools();
    for passenger in load_passengers() {
        let category = ItemService::item_category(&passenger);
        let stocked = match category {
            "ghost" => &pools.ghost,
            "vampire" => &pools.vampire,
            "demon" => &pools.demon,
            "occult" => &pools.occult,
            "holy" => &pools.holy,
            "common" => &pools.common,
            other => panic!("{} maps to unknown pool {other:?}", passenger.name),
        };
        assert!(
            !stocked.is_empty(),
            "{} draws from empty pool {category:?}",
            passenger.name
        );
    }
}
#[test]
fn every_pool_is_reachable_from_the_roster() {
    let passengers = load_passengers();
    let reached: HashSet<&str> = passengers.iter().map(ItemService::item_category).collect();
    for pool in ["ghost", "vampire", "demon", "occult", "holy", "common"] {
        assert!(reached.contains(pool), "no passenger draws from {pool:?}");
    }
}
#[test]
fn every_item_can_be_obtained() {
    let catalog = load_item_catalog();
    let pools = load_item_pools();
    let passengers = load_passengers();

    let mut rng = macroquad_toolkit::rng::SeededRng::new(0xC0FFEE);
    let mut reachable: HashSet<String> = HashSet::new();
    for pool in ["ghost", "vampire", "demon", "occult", "holy", "common"] {
        for _ in 0..200 {
            reachable.insert(pools.pick(&mut rng, pool).to_lowercase());
        }
    }
    for passenger in &passengers {
        for name in &passenger.drop_items {
            reachable.insert(name.to_lowercase());
        }
        if let Some(reward) = &passenger.trade_reward {
            reachable.insert(reward.to_lowercase());
        }
    }
    // Rule consequences can also mint items now — an `item` consequence
    // names its reward (rule 2's mixtape, rule 4's talisman).
    for rule in load_rules() {
        for consequence in rule
            .follow_consequences
            .iter()
            .chain(rule.break_consequences.iter())
            .chain(rule.exception_rewards.iter())
        {
            if let Some(name) = &consequence.item {
                reachable.insert(name.to_lowercase());
            }
        }
    }

    let mut names = catalog.names();
    names.sort();
    let orphans: Vec<String> = names
        .into_iter()
        .filter(|name| !reachable.contains(&name.to_lowercase()))
        .collect();
    assert!(
        orphans.is_empty(),
        "nothing in the game can give the player: {orphans:?}"
    );
}
#[test]
fn rule_item_consequences_name_real_items() {
    let catalog = load_item_catalog();
    for rule in load_rules() {
        for consequence in rule
            .follow_consequences
            .iter()
            .chain(rule.break_consequences.iter())
            .chain(rule.exception_rewards.iter())
        {
            if let Some(name) = &consequence.item {
                assert!(
                    catalog.contains(name),
                    "rule {} names an item the catalogue does not define: {name:?}",
                    rule.id
                );
            }
        }
    }
}
#[test]
fn the_crafter_offers_his_work_to_someone_holding_what_he_wants() {
    let constants = load_constants();
    let catalog = load_item_catalog();
    let pools = load_item_pools();
    let collector = load_passengers()
        .into_iter()
        .find(|p| p.trade_reward.is_some())
        .expect("a passenger offering a trade reward");
    let reward = collector.trade_reward.clone().expect("the reward");
    let wanted = collector.wanted_items.first().cloned().expect("a want");

    let offer_with = |inventory: Vec<nightmare_shift::data::InventoryItem>| {
        ItemService::check_trade_offer(
            &mut macroquad_toolkit::rng::SeededRng::new(7),
            &collector,
            &inventory,
            &constants,
            0.0,
            &pools,
            &catalog,
        )
        .map(|offer| offer.offered_item.name)
    };

    let holding = vec![catalog.create_item(&wanted, "test", 0.0)];
    assert_eq!(
        offer_with(holding).as_deref(),
        Some(reward.as_str()),
        "holding what he wants did not get his work offered"
    );

    // Something he has no interest in. Any offer at all here is a roll, so
    // the assertion is only that it is never the reward.
    let unwanted = "Burning Coal";
    assert!(
        !collector.wanted_items.iter().any(|w| w == unwanted),
        "pick an item this passenger does not want"
    );
    for _ in 0..200 {
        let carrying = vec![catalog.create_item(unwanted, "test", 0.0)];
        if let Some(offered) = offer_with(carrying) {
            assert_ne!(
                offered, reward,
                "his work was offered to someone carrying nothing he wants"
            );
        }
    }
}
#[test]
fn every_trade_reward_is_a_real_item() {
    let catalog = load_item_catalog();
    let mut authored = 0;
    for passenger in load_passengers() {
        let Some(reward) = &passenger.trade_reward else {
            continue;
        };
        assert!(
            catalog.contains(reward),
            "{} offers {reward:?}, which is not in the catalogue",
            passenger.name
        );
        assert!(
            !passenger.wanted_items.is_empty(),
            "{} offers a reward for a want they do not have",
            passenger.name
        );
        authored += 1;
    }
    assert!(authored > 0, "no passenger offers a trade reward any more");
}
#[test]
fn a_curse_bites_once_rather_than_every_frame() {
    let constants = load_constants();
    let catalog = load_item_catalog();

    // The Dusty Mirror drains fuel eighty minutes after it is picked up.
    let mut state = GameState::new(0.0, &constants.game_constants);
    state.fuel = 100.0;
    state
        .inventory
        .push(catalog.create_item("Dusty Mirror", "test", 0.0));
    let curse = state.inventory[0]
        .cursed_properties
        .as_ref()
        .expect("the mirror is cursed")
        .clone();
    assert_eq!(
        curse.penalty_type,
        nightmare_shift::data::CursePenalty::FuelDrain
    );

    // Well past its trigger, then several frames of the game running.
    let past_trigger = (curse.triggers_after as f64 + 1.0) * 60.0;
    ItemService::update_items(&mut state, past_trigger);
    let after_one = state.fuel;
    for frame in 1..=10 {
        ItemService::update_items(&mut state, past_trigger + frame as f64 * 0.016);
    }

    assert!(after_one < 100.0, "the curse never bit at all");
    assert_eq!(
        state.fuel, after_one,
        "the curse bit again on later frames: {} against {after_one}",
        state.fuel
    );
}
#[test]
fn age_wears_an_item_down_once_per_step() {
    let constants = load_constants();
    let catalog = load_item_catalog();

    let mut state = GameState::new(0.0, &constants.game_constants);
    state
        .inventory
        .push(catalog.create_item("Rune Stone", "test", 0.0));
    let (start, _) = state.inventory[0]
        .uses_left()
        .expect("the stone counts uses");

    // Twenty-one minutes on: one full ten-minute step past the grace period.
    let aged = 21.0 * 60.0;
    ItemService::update_items(&mut state, aged);
    let after_one = state.inventory[0]
        .uses_left()
        .map(|(left, _)| left)
        .unwrap_or(0);

    for frame in 1..=20 {
        ItemService::update_items(&mut state, aged + frame as f64 * 0.016);
    }
    let after_many = state.inventory[0]
        .uses_left()
        .map(|(left, _)| left)
        .unwrap_or(0);

    assert!(after_one < start, "age did not wear the stone at all");
    assert_eq!(
        after_many, after_one,
        "the stone kept wearing every frame: {after_many} against {after_one}"
    );
}
#[test]
fn wear_accumulates_across_steps() {
    let constants = load_constants();
    let catalog = load_item_catalog();
    let mut state = GameState::new(0.0, &constants.game_constants);
    state
        .inventory
        .push(catalog.create_item("Rune Stone", "test", 0.0));
    let (start, _) = state.inventory[0].uses_left().expect("counted");

    ItemService::update_items(&mut state, 21.0 * 60.0);
    let one_step = state.inventory[0].uses_left().map(|(l, _)| l).unwrap_or(0);
    ItemService::update_items(&mut state, 31.0 * 60.0);
    let two_steps = state.inventory[0].uses_left().map(|(l, _)| l).unwrap_or(0);

    assert!(one_step < start, "the first step took nothing");
    assert!(
        two_steps < one_step,
        "the second step took nothing: {two_steps} against {one_step}"
    );
}
#[test]
fn age_does_not_refund_a_spent_charge() {
    let constants = load_constants();
    let catalog = load_item_catalog();
    let passengers = load_passengers();
    let uncanny = passengers
        .iter()
        .find(|p| p.is_supernatural)
        .expect("a supernatural fare");

    let mut state = GameState::new(0.0, &constants.game_constants);
    state.current_passenger = Some(uncanny.clone());
    state
        .inventory
        .push(catalog.create_item("Prayer Beads", "test", 0.0));
    let (start, _) = state.inventory[0].uses_left().expect("counted");

    assert!(ItemService::use_item(
        &mut state,
        0,
        &constants.reputation,
        0.0
    ));
    let after_use = state.inventory[0].uses_left().map(|(l, _)| l).unwrap_or(0);
    assert_eq!(after_use, start - 1);

    // Now let it age a full step and check the use is still spent.
    ItemService::update_items(&mut state, 21.0 * 60.0);
    let after_age = state.inventory[0].uses_left().map(|(l, _)| l).unwrap_or(0);
    assert!(
        after_age < after_use,
        "age took nothing: {after_age} against {after_use}"
    );
    assert!(
        after_age <= start - 2,
        "age handed the spent charge back: {after_age} of {start}"
    );
}
#[test]
fn every_curse_says_which_item_took_the_toll() {
    let constants = load_constants();
    let catalog = load_item_catalog();
    let mut names: Vec<String> = catalog.names();
    names.sort();

    let mut checked = 0;
    for name in names {
        let item = catalog.create_item(&name, "test", 0.0);
        let Some(curse) = item.cursed_properties.clone() else {
            continue;
        };

        let mut state = GameState::new(0.0, &constants.game_constants);
        state.fuel = 100.0;
        state.inventory.push(item);
        ItemService::update_items(&mut state, (curse.triggers_after as f64 + 1.0) * 60.0);

        let said = state
            .current_dialogue
            .as_ref()
            .map(|dialogue| dialogue.text.clone())
            .unwrap_or_else(|| panic!("{name} took its toll without a word"));
        assert!(
            said.contains(&name),
            "{name:?} bit and the line did not name it: {said:?}"
        );
        checked += 1;
    }
    assert!(checked > 0, "no cursed items in the catalogue");
}
#[test]
fn a_ward_is_spent_over_exactly_the_charges_it_authors() {
    let constants = load_constants();
    let catalog = load_item_catalog();
    let uncanny = load_passengers()
        .into_iter()
        .find(|p| p.is_supernatural)
        .expect("a supernatural fare");

    let medallion = catalog.create_item("Blessed Medallion", "test", 0.0);
    let authored = medallion
        .protective_properties
        .as_ref()
        .and_then(|properties| properties.uses_remaining)
        .expect("the medallion authors ward charges");
    assert!(authored > 1, "pick a ward with more than one charge");

    let mut state = GameState::new(0.0, &constants.game_constants);
    state.current_passenger = Some(uncanny);
    state.inventory.push(medallion);

    for spent in 1..authored {
        assert!(ItemService::use_item(
            &mut state,
            0,
            &constants.reputation,
            0.0
        ));
        assert!(
            !state.inventory.is_empty(),
            "the medallion was gone after {spent} of {authored} charges"
        );
    }

    assert!(ItemService::use_item(
        &mut state,
        0,
        &constants.reputation,
        0.0
    ));
    assert!(
        state.inventory.is_empty(),
        "the medallion outlasted all {authored} of its charges"
    );
}
#[test]
fn no_usable_item_can_be_used_for_ever() {
    let constants = load_constants();
    let catalog = load_item_catalog();
    let uncanny = load_passengers()
        .into_iter()
        .find(|p| p.is_supernatural)
        .expect("a supernatural fare");

    let mut names: Vec<String> = catalog.names();
    names.sort();
    let mut checked = 0;
    for name in names {
        let template = catalog.create_item(&name, "test", 0.0);
        if !template.can_use {
            continue;
        }

        let mut state = GameState::new(0.0, &constants.game_constants);
        state.current_passenger = Some(uncanny.clone());
        state.fuel = 50.0;
        state.inventory.push(template);

        // Its own charge count, plus slack, is more than enough.
        let budget = state.inventory[0].max_durability.unwrap_or(1) + 3;
        for _ in 0..budget {
            if state.inventory.is_empty() {
                break;
            }
            ItemService::use_item(&mut state, 0, &constants.reputation, 0.0);
        }
        assert!(
            state.inventory.is_empty(),
            "{name} outlasted {budget} uses and can be spent for ever"
        );
        checked += 1;
    }
    assert!(checked > 0, "no usable items in the catalogue");
}
#[test]
fn a_counted_item_reports_and_spends_its_uses() {
    let constants = load_constants();
    let catalog = load_item_catalog();
    let passengers = load_passengers();
    let uncanny = passengers
        .iter()
        .find(|p| p.is_supernatural)
        .expect("a supernatural fare");

    // Prayer Beads: four uses, usable, and no condition on the effect.
    let mut state = GameState::new(0.0, &constants.game_constants);
    state.current_passenger = Some(uncanny.clone());
    state
        .inventory
        .push(catalog.create_item("Prayer Beads", "test", 0.0));

    let (before, most) = state.inventory[0]
        .uses_left()
        .expect("the beads count their uses");
    assert_eq!(before, most, "a fresh item did not start full");
    assert!(
        most > 1,
        "pick an item with more than one use to test spending"
    );

    assert!(ItemService::use_item(
        &mut state,
        0,
        &constants.reputation,
        0.0
    ));
    let (after, _) = state.inventory[0]
        .uses_left()
        .expect("the beads are still in hand");
    assert_eq!(after, before - 1, "spending a use did not lower the count");
}
#[test]
fn an_uncounted_item_reports_no_uses() {
    let catalog = load_item_catalog();
    let mut names: Vec<String> = catalog.names();
    names.sort();
    let uncounted = names
        .iter()
        .map(|name| catalog.create_item(name, "test", 0.0))
        .find(|item| item.max_durability.is_none())
        .expect("an item without durability");
    assert!(uncounted.uses_left().is_none());
}
#[test]
fn an_unusable_item_reports_no_uses() {
    let catalog = load_item_catalog();
    let mut names: Vec<String> = catalog.names();
    names.sort();
    let unusable = names
        .iter()
        .map(|name| catalog.create_item(name, "test", 0.0))
        .find(|item| !item.can_use && item.max_durability.is_some())
        .expect("an unusable item that still carries a durability");
    assert!(
        unusable.uses_left().is_none(),
        "{} reported uses it has no way to spend",
        unusable.name
    );
}
#[test]
fn a_conditional_charge_waits_for_its_condition() {
    let constants = load_constants();
    let catalog = load_item_catalog();
    let passengers = load_passengers();
    let mundane = passengers
        .iter()
        .find(|p| !p.is_supernatural)
        .expect("an ordinary fare");
    let uncanny = passengers
        .iter()
        .find(|p| p.is_supernatural)
        .expect("a supernatural fare");

    let use_on = |passenger: &nightmare_shift::data::Passenger| {
        let mut state = GameState::new(0.0, &constants.game_constants);
        state.current_passenger = Some(passenger.clone());
        state
            .inventory
            .push(catalog.create_item("Crystal Pendant", "test", 0.0));
        let used = ItemService::use_item(&mut state, 0, &constants.reputation, 0.0);
        (used, state.rule_immunity_charges, state.inventory.len())
    };

    let (used, charges, kept) = use_on(mundane);
    assert!(!used, "the pendant was spent on an ordinary passenger");
    assert_eq!(charges, 0, "immunity was granted with no condition met");
    assert_eq!(kept, 1, "the item was consumed for nothing");

    let (used, charges, _) = use_on(uncanny);
    assert!(used, "the pendant did nothing for a supernatural passenger");
    assert!(
        charges > 0,
        "no immunity was granted when it should have been"
    );
}
#[test]
fn an_unconditional_item_still_works_on_anyone() {
    let constants = load_constants();
    let catalog = load_item_catalog();
    let mundane = load_passengers()
        .into_iter()
        .find(|p| !p.is_supernatural)
        .expect("an ordinary fare");

    let mut state = GameState::new(0.0, &constants.game_constants);
    state.current_passenger = Some(mundane);
    state.fuel = 10.0;
    state
        .inventory
        .push(catalog.create_item("Burning Coal", "test", 0.0));

    assert!(ItemService::use_item(
        &mut state,
        0,
        &constants.reputation,
        0.0
    ));
    assert!(state.fuel > 10.0, "the fuel bonus did not apply");
}
#[test]
fn every_authored_condition_is_recognised() {
    const KNOWN: [&str; 2] = ["supernatural_encounter", "passenger_present"];
    let catalog = load_item_catalog();
    let mut names: Vec<String> = catalog.names();
    names.sort();
    let mut checked = 0;
    for name in names {
        for effect in &catalog.get(&name).effects {
            if let Some(condition) = effect.condition.as_deref() {
                assert!(
                    KNOWN.contains(&condition),
                    "{name} authors unknown effect condition {condition:?}"
                );
                checked += 1;
            }
        }
    }
    assert!(checked > 0, "no item authors an effect condition any more");
}
#[test]
fn a_reputation_item_lands_on_a_passenger_met_for_the_first_time() {
    let constants = load_constants();
    let catalog = load_item_catalog();
    let passenger = load_passengers().into_iter().next().expect("a roster");

    let mut state = GameState::new(0.0, &constants.game_constants);
    state.current_passenger = Some(passenger.clone());
    assert!(
        state.passenger_reputation.is_empty(),
        "this test is only meaningful before any reputation exists"
    );

    let flowers = catalog.create_item("Withered Flowers", &passenger.name, 0.0);
    let effects: Vec<ItemEffect> = flowers
        .effects
        .iter()
        .filter(|effect| effect.effect_type == ItemEffectType::ReputationModifier)
        .cloned()
        .collect();
    assert!(
        !effects.is_empty(),
        "Withered Flowers no longer carries a reputation effect; pick another item"
    );

    for effect in &effects {
        ItemService::apply_item_effect(effect, &mut state, &constants.reputation, 0.0);
    }

    let reputation = state
        .passenger_reputation
        .get(&passenger.id)
        .expect("offering the flowers recorded nothing");
    assert!(reputation.positive_choices > 0);
    assert_ne!(
        reputation.relationship_level,
        RelationshipLevel::Neutral,
        "standing was tallied but the level the fare reads never moved"
    );
}
#[test]
fn wanted_items_exist_and_are_tradeable() {
    let catalog = load_item_catalog();
    for passenger in load_passengers() {
        for name in &passenger.wanted_items {
            assert!(
                catalog.contains(name),
                "{} wants unknown item {name:?}",
                passenger.name
            );
            assert!(
                catalog.get(name).can_trade,
                "{} wants untradeable item {name:?}",
                passenger.name
            );
        }
    }
}
#[test]
fn some_passenger_wants_to_trade() {
    assert!(load_passengers().iter().any(|p| p.wants_trade));
}
#[test]
fn every_passenger_trait_has_a_skill_that_uses_it() {
    let purchasable: HashSet<String> = load_skill_tree()
        .into_iter()
        .filter(|skill| skill.effect.effect_type == "ability_unlock")
        .map(|skill| skill.effect.target)
        .collect();
    for passenger in load_passengers() {
        for trait_name in &passenger.traits {
            let id = RideService::trait_skill_id(trait_name);
            assert!(
                purchasable.contains(&id),
                "{} carries trait {trait_name:?}, which no skill unlocks",
                passenger.name
            );
        }
    }
}

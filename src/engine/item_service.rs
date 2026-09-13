//! Item service for managing drops, trading, and effects.

use crate::data::*;
use crate::state::*;

/// Item drop result
#[derive(Debug, Clone)]
pub struct ItemDrop {
    pub item: InventoryItem,
}

/// Trade offer from a passenger
#[derive(Debug, Clone)]
pub struct TradeOffer {
    pub passenger_name: String,
    pub offered_item: InventoryItem,
}

/// Item service for managing all item-related logic
pub struct ItemService;

impl ItemService {
    /// Calculate drop chance based on passenger and ride outcome
    pub fn calculate_drop_chance(
        passenger: &Passenger,
        route_type: RouteType,
        backstory_known: bool,
        constants: &ConstantsData,
    ) -> f32 {
        let mut base_chance: f32 = constants.probabilities.item_drop;

        // Rarity modifiers
        base_chance *= match passenger.rarity {
            Rarity::Common => 0.5,
            Rarity::Uncommon => 0.75,
            Rarity::Rare => 1.0,
            Rarity::Legendary => 1.5,
        };

        // Scenic routes increase drop chance
        if route_type == RouteType::Scenic {
            base_chance *= 1.3;
        }

        // Backstory unlock significantly increases drop chance
        if backstory_known {
            base_chance *= 1.5;
        }

        // Supernatural passengers more likely to drop items
        if passenger.is_supernatural {
            base_chance *= 1.2;
        }

        base_chance.min(0.80)
    }

    /// Generate an item drop from a passenger
    pub fn generate_drop(
        rng: &mut macroquad_toolkit::rng::SeededRng,
        passenger: &Passenger,
        route_type: RouteType,
        backstory_known: bool,
        current_time: f64,
        constants: &ConstantsData,
        item_pools: &ItemPools,
        catalog: &ItemCatalog,
    ) -> Option<ItemDrop> {
        let drop_chance =
            Self::calculate_drop_chance(passenger, route_type, backstory_known, constants);

        if rng.next_f32() > drop_chance {
            return None;
        }

        // Determine item based on passenger
        let item =
            Self::select_item_for_passenger(rng, passenger, current_time, item_pools, catalog);

        Some(ItemDrop { item })
    }

    /// Which item pool a passenger's generic drops come from.
    ///
    /// `itemCategory` is authored per passenger; the keyword scan over the
    /// `supernatural` prose is only a fallback for entries that omit it.
    pub fn item_category(passenger: &Passenger) -> &str {
        if let Some(category) = passenger.item_category.as_deref() {
            return category;
        }
        let prose = passenger.supernatural.to_lowercase();
        for (keyword, category) in [
            ("ghost", "ghost"),
            ("specter", "ghost"),
            ("drowned", "ghost"),
            ("vampire", "vampire"),
            ("undead", "vampire"),
            ("demon", "demon"),
            ("reaper", "demon"),
            ("psychic", "occult"),
            ("fortune", "occult"),
            ("nun", "holy"),
            ("priest", "holy"),
        ] {
            if prose.contains(keyword) {
                return category;
            }
        }
        "common"
    }

    /// Select an appropriate item for a passenger to drop
    fn select_item_for_passenger(
        rng: &mut macroquad_toolkit::rng::SeededRng,
        passenger: &Passenger,
        current_time: f64,
        item_pools: &ItemPools,
        catalog: &ItemCatalog,
    ) -> InventoryItem {
        // Check if passenger has specific drop items
        if !passenger.drop_items.is_empty() {
            let idx = rng.below(passenger.drop_items.len());
            let item_name = &passenger.drop_items[idx];
            return catalog.create_item(item_name, &passenger.name, current_time);
        }

        // Otherwise generate based on supernatural type
        let item_name = item_pools.pick(rng, Self::item_category(passenger));
        catalog.create_item(&item_name, &passenger.name, current_time)
    }

    /// Check if a passenger wants to trade
    pub fn check_trade_offer(
        rng: &mut macroquad_toolkit::rng::SeededRng,
        passenger: &Passenger,
        inventory: &[InventoryItem],
        constants: &ConstantsData,
        current_time: f64,
        item_pools: &ItemPools,
        catalog: &ItemCatalog,
    ) -> Option<TradeOffer> {
        // Only some passengers trade
        if !passenger.wants_trade {
            return None;
        }

        // Check if player has something the passenger wants
        let wanted_item = inventory
            .iter()
            .find(|item| passenger.wanted_items.contains(&item.name) && item.can_be_given_away());

        if wanted_item.is_some() || rng.chance(constants.probabilities.trade_offer_chance) {
            // Someone holding what this passenger came for is offered their
            // own work, when they have any to give.
            let reward = wanted_item.and(passenger.trade_reward.as_deref());
            let offered_item = match reward.filter(|name| catalog.contains(name)) {
                Some(name) => catalog.create_item(name, &passenger.name, current_time),
                None => Self::select_item_for_passenger(
                    rng,
                    passenger,
                    current_time,
                    item_pools,
                    catalog,
                ),
            };

            return Some(TradeOffer {
                passenger_name: passenger.name.clone(),
                offered_item,
            });
        }

        None
    }

    /// Apply effects of an item
    /// `current_time` is passed in rather than read from macroquad's clock.
    /// Reaching for the global made the whole effect table impossible to test
    /// outside a window, which is why the reputation effect could sit broken.
    pub fn apply_item_effect(
        effect: &ItemEffect,
        state: &mut GameState,
        reputation_constants: &ReputationConstants,
        current_time: f64,
    ) {
        match effect.effect_type {
            ItemEffectType::FuelBonus => {
                let bonus = effect.value as f32;
                state.fuel = (state.fuel + bonus).min(state.max_fuel);
            }
            ItemEffectType::TimeBonus => {
                state.time_remaining += effect.value as u32;
            }
            ItemEffectType::RuleImmunity => {
                state.rule_immunity_charges += effect.value as u32;
            }
            ItemEffectType::SupernaturalProtection => {
                state.supernatural_protection += effect.value as u32;
            }
            ItemEffectType::FuelDrain => {
                let drain = effect.value as f32;
                state.fuel = (state.fuel - drain).max(0.0);
            }
            ItemEffectType::TimePenalty => {
                let penalty = effect.value as u32;
                state.time_remaining = state.time_remaining.saturating_sub(penalty);
            }
            ItemEffectType::ReputationModifier => {
                // `get_mut` meant this did nothing at all on a first meeting,
                // which is most of them: a reputation entry is not created
                // until a ride with that passenger completes, so the whole
                // point of the withered flowers and the faded photograph was
                // dropped on the floor the first time you offered either.
                if let Some(passenger_id) = state.current_passenger.as_ref().map(|p| p.id) {
                    state
                        .get_passenger_reputation(passenger_id)
                        .adjust(effect.value, reputation_constants);
                }
            }
            ItemEffectType::RuleTrigger => {
                state.curse_danger_bonus += effect.value.max(1) as u32;
                state.current_dialogue = Some(CurrentDialogue {
                    text: "The item hums against tonight's rules. The next route will be riskier."
                        .to_string(),
                    speaker: DialogueSpeaker::Narrator,
                    timestamp: current_time,
                });
            }
        }
    }

    /// Whether an effect's authored `condition` holds right now.
    ///
    /// One item uses this -- the Crystal Pendant's rule immunity is authored
    /// `"supernatural_encounter"` -- and nothing read it, so the pendant
    /// granted a plain unconditional charge and the catalogue described
    /// something the game did not do.
    ///
    /// The reading is an interpretation and worth naming as one. A
    /// supernatural encounter is a momentary roll during a leg, not a state
    /// anything could be asked about, so "am I in one" is unanswerable at the
    /// moment an item is used. What is answerable, and what the pendant is
    /// for, is whether the thing in the cab is supernatural at all.
    ///
    /// An unrecognised condition holds. Failing open means a typo leaves an
    /// item working rather than silently inert, which is the direction this
    /// project has been burned in before.
    fn condition_met(effect: &ItemEffect, state: &GameState) -> bool {
        let Some(condition) = effect.condition.as_deref() else {
            return true;
        };
        match condition {
            "supernatural_encounter" => state
                .current_passenger
                .as_ref()
                .is_some_and(|passenger| passenger.is_supernatural),
            "passenger_present" => state.current_passenger.is_some(),
            _ => true,
        }
    }

    /// Why an item would not answer, in the driver's own terms.
    ///
    /// The one refusal line was written for the Crystal Pendant, which stays
    /// cold because the fare is ordinary. Told to a driver holding flowers at
    /// an empty rank it explains nothing, so each condition says its own piece
    /// and an unrecognised one keeps the original wording.
    fn refusal(condition: &str, item_name: &str) -> String {
        match condition {
            "passenger_present" => {
                format!("The back seat is empty. The {item_name} needs someone to be for.")
            }
            _ => format!("The {item_name} stays cold. Nothing here answers to it."),
        }
    }

    /// Use an item from inventory
    /// Returns true if the item was successfully used
    pub fn use_item(
        state: &mut GameState,
        idx: usize,
        reputation_constants: &ReputationConstants,
        current_time: f64,
    ) -> bool {
        if idx >= state.inventory.len() {
            return false;
        }

        // We need to clone the item to use it while mutating state
        let item = state.inventory[idx].clone();

        if !item.can_use {
            return false;
        }

        // An effect whose condition is not met does not fire, and if that
        // leaves nothing to do the item is not spent at all. Consuming a
        // charge for no result would be a trap, and the player is told why
        // rather than left to wonder.
        let applicable: Vec<&ItemEffect> = item
            .effects
            .iter()
            .filter(|effect| Self::condition_met(effect, state))
            .collect();
        if applicable.is_empty() && !item.effects.is_empty() {
            let reason = item
                .effects
                .iter()
                .find_map(|effect| effect.condition.as_deref())
                .map(|condition| Self::refusal(condition, &item.name))
                .unwrap_or_else(|| Self::refusal("", &item.name));
            state.current_dialogue = Some(CurrentDialogue {
                text: reason,
                speaker: DialogueSpeaker::Narrator,
                timestamp: current_time,
            });
            return false;
        }

        for effect in applicable {
            Self::apply_item_effect(effect, state, reputation_constants, current_time);
        }

        // What using it costs the item.
        //
        // A consumable goes. Anything that counts charges spends one and goes on
        // its last. Anything that counts nothing is spent outright -- three
        // protective items author ward charges but no `maxDurability`, and this
        // branch used to find `None`, do nothing, and hand the item back, so a
        // single Blessed Medallion granted supernatural protection on every
        // click for ever. `ProtectionService::consume_ward` already applies that
        // rule on the passive side; this is the same rule actively.
        if item.item_type == ItemType::Consumable {
            state.inventory.remove(idx);
        } else {
            match state
                .inventory
                .get_mut(idx)
                .and_then(|item| item.durability)
            {
                Some(durability) if durability > 1 => {
                    if let Some(stored_item) = state.inventory.get_mut(idx) {
                        stored_item.durability = Some(durability - 1);
                    }
                }
                _ => {
                    state.inventory.remove(idx);
                }
            }
        }

        true
    }

    /// Updated item states (deterioration, curses)
    pub fn update_items(state: &mut GameState, current_time: f64) {
        // Which curses come due this frame. Marked as they are collected, so
        // each one is charged once however long the item is carried afterwards.
        let mut coming_due: Vec<(String, CursedProperties)> = Vec::new();
        for item in &mut state.inventory {
            if item.curse_fired || !item.is_cursed() || !item.should_trigger_curse(current_time) {
                continue;
            }
            if let Some(curse) = item.cursed_properties.clone() {
                item.curse_fired = true;
                coming_due.push((item.name.clone(), curse));
            }
        }
        for (name, curse) in coming_due {
            Self::apply_curse(&name, &curse, state, current_time);
        }

        // Apply item deterioration
        for item in &mut state.inventory {
            item.apply_deterioration(current_time);
        }

        // Remove broken items
        state.inventory.retain(|item| !item.is_broken());
    }

    /// Charge one curse's toll, and say so.
    ///
    /// Three of the four penalties used to take their toll in silence: fuel and
    /// the clock simply dropped, and the danger bonus climbed, with nothing on
    /// screen connecting any of it to the thing in the driver's pocket. Only
    /// `ForcedChoices` spoke. A curse the player cannot attribute is
    /// indistinguishable from the game cheating, and the inventory already names
    /// which item is cursed and how to be rid of it -- this is the moment that
    /// warning comes true.
    fn apply_curse(
        item_name: &str,
        curse: &CursedProperties,
        state: &mut GameState,
        current_time: f64,
    ) {
        let told = match curse.penalty_type {
            CursePenalty::FuelDrain => {
                state.fuel = (state.fuel - curse.penalty_value as f32).max(0.0);
                format!(
                    "The {} has been drinking. {}% of the tank, gone.",
                    item_name, curse.penalty_value
                )
            }
            CursePenalty::TimeAcceleration => {
                state.time_remaining = state
                    .time_remaining
                    .saturating_sub(curse.penalty_value as u32);
                format!(
                    "The {} runs the clock forward. {} minutes you will not get back.",
                    item_name, curse.penalty_value
                )
            }
            CursePenalty::AttractingDanger => {
                state.curse_danger_bonus += curse.penalty_value as u32;
                format!("The {} starts drawing something toward the cab.", item_name)
            }
            CursePenalty::ForcedChoices => {
                state.curse_danger_bonus += curse.penalty_value.max(1) as u32;
                format!(
                    "The {} narrows your options. The roads ahead feel worse.",
                    item_name
                )
            }
        };

        state.current_dialogue = Some(CurrentDialogue {
            text: told,
            speaker: DialogueSpeaker::Narrator,
            timestamp: current_time,
        });
    }
}

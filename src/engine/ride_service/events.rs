//! The mid-ride event deck: drawing an event, and resolving the player's choice.

use crate::data::event::{EventChoice, EventConsequence, MidRideEvent, RiskTag};
use crate::data::*;
use crate::engine::PassengerStateMachine;
use crate::state::*;

use super::RideService;

impl RideService {
    /// Generate a mid-ride event by drawing from the authored event deck.
    ///
    /// Picks a route-eligible template (weighted), then optionally appends a
    /// passenger-specific "use your ability" choice when the player has both the
    /// almanac knowledge and the matching skill unlocked, and shuffles.
    /// `pub(crate)` so the screenshot harness can seed a real event rather
    /// than hand-building one that could drift from what the game produces.
    pub(crate) fn generate_mid_ride_event(
        rng: &mut macroquad_toolkit::rng::SeededRng,
        state: &GameState,
        data: &GameData,
        stats: &PlayerStats,
        route: RouteType,
    ) -> MidRideEvent {
        let eligible: Vec<&EventTemplate> = data
            .events
            .iter()
            .filter(|e| e.eligible_for(route))
            .collect();

        let (title, description, mut choices) = match Self::pick_weighted_event(rng, &eligible) {
            Some(tpl) => (
                tpl.title.clone(),
                tpl.description.clone(),
                tpl.choices.clone(),
            ),
            None => Self::fallback_event(),
        };

        // Passenger ability choice: strongest option, gated behind almanac + skill.
        if let Some(p) = &state.current_passenger {
            let almanac_unlocked = stats.get_almanac_entry(p.id).knowledge_level >= 1;
            if almanac_unlocked {
                // Any matching trait qualifies, not just the first one listed —
                // a passenger's second and third traits were unreachable before.
                let matched = p
                    .traits
                    .iter()
                    .find(|trait_name| stats.is_skill_unlocked(&Self::trait_skill_id(trait_name)));
                if let Some(trait_name) = matched {
                    choices.push(EventChoice {
                        description: format!("Use your {} to steady the moment", trait_name),
                        risk_type: RiskTag::SpiritualDisturbance,
                        consequence: EventConsequence::Stress(-10),
                        required_trait: Some(trait_name.clone()),
                    });
                }
            }
        }

        rng.shuffle(&mut choices);

        MidRideEvent {
            title,
            description,
            choices,
        }
    }

    /// The skill-tree id that grants a passenger trait's ability choice.
    /// `"Night Vision"` and the `night_vision` skill are the same thing.
    pub(crate) fn trait_skill_id(trait_name: &str) -> String {
        trait_name.to_lowercase().replace(' ', "_")
    }

    /// Pick one event weighted by its `weight` field.
    fn pick_weighted_event<'a>(
        rng: &mut macroquad_toolkit::rng::SeededRng,
        events: &[&'a EventTemplate],
    ) -> Option<&'a EventTemplate> {
        if events.is_empty() {
            return None;
        }
        let total: f32 = events.iter().map(|e| e.weight.max(0.0)).sum();
        if total <= 0.0 {
            return rng.choose(events).copied();
        }
        let mut roll = rng.next_f32() * total;
        for e in events {
            roll -= e.weight.max(0.0);
            if roll <= 0.0 {
                return Some(*e);
            }
        }
        events.last().copied()
    }

    /// Generic event used only if the deck is empty (e.g. data failed to load).
    fn fallback_event() -> (String, String, Vec<EventChoice>) {
        (
            "Unsettling Atmosphere".to_string(),
            "The air grows heavy and silent.".to_string(),
            vec![
                EventChoice {
                    description: "Proceed carefully".to_string(),
                    risk_type: RiskTag::DenseFog,
                    consequence: EventConsequence::Fuel(3.0),
                    required_trait: None,
                },
                EventChoice {
                    description: "Speed through it".to_string(),
                    risk_type: RiskTag::StrangeNoises,
                    consequence: EventConsequence::Risk(2),
                    required_trait: None,
                },
                EventChoice {
                    description: "Take a detour".to_string(),
                    risk_type: RiskTag::RoadConstruction,
                    consequence: EventConsequence::Time(8),
                    required_trait: None,
                },
            ],
        )
    }

    /// Resolve a mid-ride event choice, applying its consequence and returning
    /// to route selection for the second leg of the journey.
    pub fn resolve_event_choice(state: &mut GameState, choice_index: usize) {
        if let Some(event) = &state.current_event {
            if let Some(choice) = event.choices.get(choice_index) {
                match choice.consequence {
                    EventConsequence::Fuel(amount) => {
                        state.fuel = (state.fuel - amount).max(0.0);
                    }
                    EventConsequence::Time(amount) => {
                        state.time_remaining = state.time_remaining.saturating_sub(amount);
                    }
                    EventConsequence::Risk(amount) => {
                        Self::apply_event_stress(state, amount * 8);
                    }
                    EventConsequence::Stress(amount) => {
                        Self::apply_event_stress(state, amount);
                    }
                }
            }
        }

        // On to the final leg: back to Driving to pick a route to the
        // destination, where transition_driving_phase completes the ride.
        // The phase used to stay Pickup here, which made the driving screen
        // label the final leg as a pickup and aim it at the pickup location.
        state.game_phase = GamePhase::Driving;
        state.driving_phase = Some(DrivingPhase::Destination);
    }

    /// Apply a stress delta from an event choice to the current passenger's need
    /// state, merging any tells the change triggers.
    fn apply_event_stress(state: &mut GameState, amount: i32) {
        if let (Some(mut need), Some(passenger)) = (
            state.current_passenger_need_state.clone(),
            state.current_passenger.clone(),
        ) {
            let now = state.simulation_time;
            let triggered =
                PassengerStateMachine::apply_stress_delta(&mut need, &passenger, amount, now);
            state.current_passenger_need_state = Some(need);
            PassengerStateMachine::merge_detected_tells(
                &mut state.rng,
                &mut state.detected_tells,
                triggered,
                &passenger,
                state.player_trust,
                now,
                &state.current_guidelines,
            );
        }
    }
}

#[cfg(test)]
mod tests;

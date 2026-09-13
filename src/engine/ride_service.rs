//! Service for managing the ride lifecycle (spawn, accept, decline, complete).
//!
//! Route selection lives in `route_choice`; the mid-ride event deck lives in
//! `events`. Both extend `RideService` with further `impl` blocks.

mod events;
mod risk;
mod route_choice;

use crate::data::*;
use crate::engine::{
    GameEngine, ItemService, PassengerSelectionContext, PassengerService, PassengerStateMachine,
    RuleModificationService, SkillModifiers,
};
use crate::state::*;

/// Outcome of a route choice
#[derive(Debug, PartialEq, Eq)]
pub enum RouteOutcome {
    Success,
    GameOver(String),
    GuidelineTriggered,
    RideCompleted,
}

pub struct RideService;

impl RideService {
    /// Spawn a new passenger. Returns true if a passenger was found, false otherwise.
    pub fn spawn_passenger(state: &mut GameState, data: &GameData, current_time: f64) -> bool {
        // The Last Fare: the city sends exactly one passenger tonight, and it
        // is Death. No weather weighting, no kin rolls — the night exists to
        // put him in the cab.
        if state.last_fare_night {
            if state.death_delivered {
                return false;
            }
            let Some(death) = data
                .passengers
                .iter()
                .find(|p| p.id == DEATH_PASSENGER_ID)
                .cloned()
            else {
                return false;
            };
            let presented = Self::present_passenger(state, death, current_time);
            if presented {
                // He does not board calm. The ride starts at the warning
                // threshold, so the whole delivery is played against a need
                // that is already moving — the pressure the first
                // measurement of this night showed it lacked.
                if let Some(need) = state.current_passenger_need_state.as_mut() {
                    let restless = need.profile.thresholds.warning.max(need.level);
                    need.level = restless.min(100);
                    need.stage =
                        PassengerNeedState::calculate_stage(need.level, &need.profile.thresholds);
                    need.stability = 1.0 - (need.level as f32 / 100.0);
                }
                state.current_dialogue = Some(CurrentDialogue {
                    text: "The radio is silent. The kerb holds a single figure, \
                           patient as the end of the night."
                        .to_string(),
                    speaker: DialogueSpeaker::Narrator,
                    timestamp: current_time,
                });
            }
            return presented;
        }

        let context = PassengerSelectionContext {
            difficulty_level: state.difficulty_level,
            weather: &state.current_weather,
            time_of_day: &state.time_of_day,
            season: &state.season,
            constants: &data.constants,
        };

        // A night should read as connected rather than a shuffle: having
        // carried someone makes their associates more likely to turn up.
        // `RELATED_PASSENGER_SPAWN` is used as the probability it is authored
        // as — roll it, and on a hit draw only from the kin pool.
        let kin = PassengerService::kin_of(&state.used_passengers, &data.passengers);
        if !kin.is_empty()
            && state
                .rng
                .chance(data.constants.probabilities.related_passenger_spawn)
        {
            let kin_pool: Vec<Passenger> = data
                .passengers
                .iter()
                .filter(|p| kin.contains(&p.id))
                .cloned()
                .collect();
            if let Some(passenger) = PassengerService::select_weather_aware_passenger(
                &mut state.rng,
                &kin_pool,
                &[],
                &context,
            ) {
                // Name the connection, or the mechanic is invisible machinery.
                let link = Self::name_the_link(&passenger, &state.used_passengers, data);
                let presented = Self::present_passenger(state, passenger, current_time);
                if let Some(link) = link {
                    state.current_dialogue = Some(CurrentDialogue {
                        text: link,
                        speaker: DialogueSpeaker::Narrator,
                        timestamp: current_time,
                    });
                }
                return presented;
            }
        }

        match PassengerService::select_weather_aware_passenger(
            &mut state.rng,
            &data.passengers,
            &state.used_passengers,
            &context,
        ) {
            Some(passenger) => Self::present_passenger(state, passenger, current_time),
            None => false,
        }
    }

    /// Say who this fare is connected to, using whichever of the two named
    /// the other — the links are authored one-way in places.
    fn name_the_link(passenger: &Passenger, met: &[u32], data: &GameData) -> Option<String> {
        let other = met.iter().rev().find_map(|id| {
            let met_passenger = data.passengers.iter().find(|p| p.id == *id)?;
            let linked = passenger.relationships.contains(id)
                || met_passenger.relationships.contains(&passenger.id);
            linked.then_some(met_passenger)
        })?;
        Some(format!(
            "Dispatch has another fare. They came up in {}'s night too.",
            other.name
        ))
    }

    /// Put a chosen passenger in front of the player: seed their need state,
    /// pick their opening line, and move to the ride request.
    fn present_passenger(state: &mut GameState, passenger: Passenger, current_time: f64) -> bool {
        state.used_passengers.push(passenger.id);
        state.current_passenger_need_state =
            PassengerStateMachine::initialize(&passenger, current_time);
        // Select dialogue once and store it
        state.current_passenger_dialogue = passenger
            .random_dialogue(&mut state.rng)
            .map(|s| s.to_string());
        // Roll which authored exceptions are in play for this fare, once,
        // here — everything that asks afterwards must get one answer.
        state.live_exceptions = crate::engine::GuidelineEngine::roll_exception_liveness(
            &mut state.rng,
            &passenger,
            &state.current_guidelines,
        );
        state.current_passenger = Some(passenger);
        state.game_phase = GamePhase::RideRequest;
        true
    }

    /// Accept the current ride request.
    /// Returns Ok if successful, Err(reason) if failed (e.g. not enough fuel).
    pub fn accept_ride(state: &mut GameState, constants: &ConstantsData) -> Result<(), String> {
        if state.fuel < constants.fuel.fuel_check_minimum as f32 {
            return Err("You ran out of fuel with a passenger in the car.".to_string());
        }

        let current_time = state.simulation_time;
        if let Some(passenger) = state.current_passenger.clone() {
            state.current_ride = Some(CurrentRide {
                pickup_location: passenger.pickup.clone(),
                destination_location: passenger.destination.clone(),
                passenger,
                route_type: None,
                driving_phase: DrivingPhase::Pickup,
                start_time: current_time,
            });
        }

        state.game_phase = GamePhase::Driving;
        state.driving_phase = Some(DrivingPhase::Pickup);
        // Last ride's swap belongs to last ride; without this its line would
        // still be sitting on the next drop-off summary.
        state.trade_outcome = None;
        state.violations_at_ride_start = state.rules_violated;
        state.ride_baseline = Some(RideBaseline {
            fuel: state.fuel,
            time: state.time_remaining,
            need: state
                .current_passenger_need_state
                .as_ref()
                .map(|need| need.level)
                .unwrap_or(0),
            rules_violated: state.rules_violated,
            comfort_relief: state.telemetry.comfort_relief,
            normal_route_relief: state.telemetry.normal_route_relief,
            ward_interventions: state.telemetry.ward_interventions,
            brink_saves: state.telemetry.brink_saves,
        });

        // Three passengers rewrite the night as they get in.
        let change = state
            .current_passenger
            .clone()
            .and_then(|passenger| RuleModificationService::apply(state, &passenger));

        state.current_dialogue = Some(CurrentDialogue {
            text: match change {
                Some(change) => change.message,
                None => "Ride accepted. Choose a route to the pickup point.".to_string(),
            },
            speaker: DialogueSpeaker::Driver,
            timestamp: current_time,
        });

        Ok(())
    }

    /// Decline the current ride request.
    /// Clears passenger state and returns to Waiting phase (caller should trigger spawn again if needed).
    pub fn decline_ride(state: &mut GameState) {
        state.current_passenger = None;
        state.current_passenger_dialogue = None;
        state.current_passenger_need_state = None;
        state.current_ride = None;
        state.current_event = None;
        state.game_phase = GamePhase::Waiting;
    }

    /// Complete the current ride logic.
    pub fn complete_ride(
        state: &mut GameState,
        data: &GameData,
        stats: &mut PlayerStats,
        route: RouteType,
        current_time: f64,
    ) {
        if let Some(ref passenger) = state.current_passenger.clone() {
            // Calculate fare. The destination's fareModifier and the player's
            // fare-boosting skills (Silver Tongue) both scale the payout.
            // Cloned so the fare call can also borrow the state's rng.
            let reputation = state.passenger_reputation.get(&passenger.id).cloned();
            let skill_mods = SkillModifiers::from_unlocked(&data.skills, &stats.unlocked_skills);
            let night_fare_mult = state
                .night_modifier
                .as_ref()
                .map(|m| m.fare_mult)
                .unwrap_or(1.0);
            let destination_fare_modifier = data
                .get_location(&passenger.destination)
                .map(|l| l.fare_modifier)
                .unwrap_or(1.0)
                * skill_mods.fare_mult
                * night_fare_mult;
            let streak = state.consecutive_route_streak.clone();
            let fare = GameEngine::calculate_fare(
                &mut state.rng,
                passenger.fare,
                route,
                passenger,
                streak.as_ref(),
                reputation.as_ref(),
                &data.constants,
                destination_fare_modifier,
            );

            // Add earnings
            state.earnings += fare;
            state.rides_completed += 1;

            // Delivering Death on The Last Fare is the run's true ending;
            // `continue_from_dropoff` reads this and closes the run.
            if state.last_fare_night && passenger.id == DEATH_PASSENGER_ID {
                state.death_delivered = true;
            }

            // A rule imposed for a few rides runs out.
            for lifted in RuleModificationService::expire_temporary_rules(state) {
                state.current_dialogue = Some(CurrentDialogue {
                    text: format!("\"{}\" is lifted. The night moves on.", lifted),
                    speaker: DialogueSpeaker::Narrator,
                    timestamp: current_time,
                });
            }

            // Check backstory unlock
            let backstory_unlocked = if PassengerService::check_backstory_unlock(
                &mut state.rng,
                passenger.id,
                stats,
                &data.constants,
            ) {
                stats.unlock_backstory(passenger.id);
                Some((passenger.name.clone(), passenger.backstory_details.clone()))
            } else {
                None
            };

            // Record encounter
            stats.record_passenger_encounter(passenger.id);

            // Observational learning. Delivering a passenger is study — the
            // almanac used to be purchase-only, so a run taught the player
            // nothing durable — and any tell the driver actually noticed is
            // inscribed for good.
            if let Some(level) = stats.record_ride_survived(passenger.id) {
                state.consequence_notes.push(format!(
                    "You understand {} better now. (Almanac Lv.{})",
                    passenger.name, level
                ));
            }
            let noticed: Vec<String> = state
                .detected_tells
                .iter()
                .filter(|tell| tell.passenger_id == passenger.id && tell.player_noticed)
                .map(|tell| tell.tell.description.clone())
                .collect();
            for description in noticed {
                stats.record_tell_seen(passenger.id, &description);
            }

            // Update reputation
            let is_positive = passenger
                .get_route_preference(route)
                .map(|p| {
                    matches!(
                        p.preference,
                        PreferenceLevel::Loves | PreferenceLevel::Likes
                    )
                })
                .unwrap_or(false);

            state
                .get_passenger_reputation(passenger.id)
                .update(is_positive, &data.constants.reputation);

            // Generate item drop
            let mut items_received = Vec::new();
            // The item bonus belongs to knowing the passenger's story, not
            // only to the exact ride on which it unlocked. Passing
            // `backstory_unlocked.is_some()` made Mastered passengers lose a
            // benefit that Studied passengers could gain mid-run.
            let backstory_known = stats.is_backstory_unlocked(passenger.id);
            if let Some(drop) = ItemService::generate_drop(
                &mut state.rng,
                passenger,
                route,
                backstory_known,
                current_time,
                &data.constants,
                &data.item_pools,
                &data.items,
            ) {
                items_received.push(drop.item.clone());
                state.inventory.push(drop.item);
            }

            // Check for trade offer
            if let Some(trade) = ItemService::check_trade_offer(
                &mut state.rng,
                passenger,
                &state.inventory,
                &data.constants,
                current_time,
                &data.item_pools,
                &data.items,
            ) {
                state.pending_trade =
                    Some((trade.passenger_name.clone(), trade.offered_item.clone()));
            }

            // Create completion data
            state.fare_contributions.push(FareContribution {
                passenger_id: passenger.id,
                passenger_name: passenger.name.clone(),
                fare,
            });
            state.last_ride_completion = Some(RideCompletion {
                passenger: passenger.clone(),
                fare_earned: fare,
                items_received,
                backstory_unlocked,
                impact: state
                    .ride_baseline
                    .take()
                    .map(|start| {
                        let need_end = state
                            .current_passenger_need_state
                            .as_ref()
                            .map(|need| need.level)
                            .unwrap_or(start.need);
                        RideImpact {
                            fuel_spent: (start.fuel - state.fuel).max(0.0).round() as u32,
                            time_spent: start.time.saturating_sub(state.time_remaining),
                            need_delta: need_end as i32 - start.need as i32,
                            rules_violated: state.rules_violated - start.rules_violated,
                            comfort_relief: state.telemetry.comfort_relief - start.comfort_relief,
                            normal_route_relief: state.telemetry.normal_route_relief
                                - start.normal_route_relief,
                            ward_interventions: state.telemetry.ward_interventions
                                - start.ward_interventions,
                            brink_saves: state.telemetry.brink_saves - start.brink_saves,
                        }
                    })
                    .unwrap_or_default(),
            });

            state.game_phase = GamePhase::DropOff;
        }
    }
}

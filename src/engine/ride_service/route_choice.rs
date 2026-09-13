//! Route selection: cost validation, rule checks, transit effects, and the
//! phase transition that follows a chosen route.

use crate::data::*;
use crate::engine::{
    GameEngine, PassengerStateMachine, ProtectionService, RouteCosts, RouteService,
    RuleEvaluationResult,
};
use crate::state::*;

use super::{RideService, RouteOutcome};

impl RideService {
    /// Choose a route and determine the outcome
    pub fn choose_route(
        state: &mut GameState,
        data: &GameData,
        stats: &mut PlayerStats,
        route: RouteType,
        current_time: f64,
    ) -> RouteOutcome {
        // The same quote the driving screen showed the player.
        let mut costs = RouteService::quote_route(route, state, data, stats);
        Self::apply_curse_route_pressure(state, &mut costs, &data.constants);
        Self::apply_route_streak_pressure(state, &mut costs, route, &data.constants, current_time);

        // 1. Check resources
        if let Some(outcome) = Self::validate_resources(state, &costs) {
            return outcome;
        }

        // 2. Check rule violations
        if let Some(outcome) = Self::check_route_rules(state, route, &data.constants) {
            return outcome;
        }

        // 3. Apply costs and record history
        Self::apply_transit_effects(state, &costs, route, current_time);
        stats.record_route_usage(route);

        // 3a. The night is winding down — say so while it can still be acted on.
        if state.take_shift_end_warning(&data.constants) {
            state.current_dialogue = Some(CurrentDialogue {
                text: format!(
                    "Dispatch: {} minutes left on the shift. Make them count or bring it in.",
                    state.time_remaining
                ),
                speaker: DialogueSpeaker::Narrator,
                timestamp: current_time,
            });
        }

        // 3b. A risky leg can cost more than its stated price.
        let encounter = Self::apply_risk_encounters(state, &costs, &data.constants, current_time);
        if let Some(encounter) = encounter {
            state.current_dialogue = Some(CurrentDialogue {
                text: encounter.message,
                speaker: DialogueSpeaker::Narrator,
                timestamp: current_time,
            });
        }

        // 4. Update passenger state machine
        Self::update_passenger_state(
            state,
            route,
            current_time,
            data.constants.game_constants.route_preference_stress_scale,
            data.constants.game_constants.normal_route_need_relief,
        );
        if Self::is_passenger_meltdown(state)
            && !Self::absorb_meltdown_with_protection(state, current_time)
            && !Self::hold_at_the_brink(state, current_time)
        {
            state.queue_audio(
                "meltdown",
                "[Cabin distortion intensifies into a passenger meltdown]",
                current_time,
            );
            return RouteOutcome::GameOver(
                "The passenger's need became uncontrollable.".to_string(),
            );
        }

        // 5. Check guidelines
        if let Some(outcome) = Self::check_guideline_triggers(state, current_time) {
            return outcome;
        }

        // 6. Progress phase
        Self::transition_driving_phase(state, data, stats, route, current_time)
    }

    /// Whether every route out of here is beyond the fuel or the clock.
    ///
    /// The driving screen refuses a route the player cannot pay for, which
    /// stopped a shift ending on a choice they could not see the price of.
    /// It also created a corner with no way out: with too little left for any
    /// of the four, every button is disabled, the clock only advances when a
    /// leg is driven, and nothing checks for the end of a shift while driving.
    /// The night would sit there with a passenger aboard and no legal move.
    pub fn is_stranded(state: &GameState, data: &GameData, stats: &PlayerStats) -> bool {
        if state.current_passenger.is_none() {
            return false;
        }
        [
            RouteType::Normal,
            RouteType::Shortcut,
            RouteType::Scenic,
            RouteType::Police,
        ]
        .into_iter()
        .filter(|route| {
            !state
                .environmental_hazards
                .iter()
                .any(|hazard| hazard.blocks_route(*route))
        })
        .all(|route| {
            let costs = RouteService::quote_route(route, state, data, stats);
            (state.fuel as u32) < costs.fuel || state.time_remaining < costs.time
        })
    }

    /// Check if player has enough resources for the route
    fn validate_resources(state: &GameState, costs: &RouteCosts) -> Option<RouteOutcome> {
        if (state.fuel as u32) < costs.fuel {
            return Some(RouteOutcome::GameOver(
                "Not enough fuel for this route.".to_string(),
            ));
        }

        if state.time_remaining < costs.time {
            return Some(RouteOutcome::GameOver(
                "Not enough time for this route.".to_string(),
            ));
        }

        None
    }

    /// Check for rule violations specific to the route
    fn check_route_rules(
        state: &mut GameState,
        route: RouteType,
        constants: &ConstantsData,
    ) -> Option<RouteOutcome> {
        if route == RouteType::Shortcut {
            // Check visible rules
            let violation = GameEngine::check_rule_violation(
                &state.current_rules,
                "take_shortcut",
                state.current_passenger_need_state.as_ref(),
                &state.current_guidelines,
            );

            if violation.violation {
                return Self::resolve_rule_violation(state, violation, false);
            }

            // Check hidden rules
            let hidden_violation = GameEngine::check_rule_violation(
                &state.hidden_rules,
                "take_shortcut",
                state.current_passenger_need_state.as_ref(),
                &state.current_guidelines,
            );

            if hidden_violation.violation
                && GameEngine::hidden_violation_lands(&mut state.rng, constants)
            {
                return Self::resolve_rule_violation(state, hidden_violation, true);
            }
        }

        let weather_violation = GameEngine::check_weather_route_violation(
            &state.current_rules,
            route,
            &state.current_weather,
            &state.time_of_day,
        );
        if weather_violation.violation {
            return Self::resolve_rule_violation(state, weather_violation, false);
        }

        let hidden_weather_violation = GameEngine::check_weather_route_violation(
            &state.hidden_rules,
            route,
            &state.current_weather,
            &state.time_of_day,
        );
        if hidden_weather_violation.violation
            && GameEngine::hidden_violation_lands(&mut state.rng, constants)
        {
            return Self::resolve_rule_violation(state, hidden_weather_violation, true);
        }

        None
    }

    fn resolve_rule_violation(
        state: &mut GameState,
        violation: RuleEvaluationResult,
        hidden: bool,
    ) -> Option<RouteOutcome> {
        let rule_title = violation
            .rule
            .as_ref()
            .map(|rule| rule.title.clone())
            .unwrap_or_else(|| "Rule".to_string());

        if hidden {
            if let Some(rule) = &violation.rule {
                state.reveal_hidden_rule(rule.id);
            }
        }

        Self::apply_rule_need_adjustment(state, &violation, state.simulation_time);
        state.rules_violated += 1;
        state.queue_audio(
            "violation",
            format!("[Rule violation: {rule_title}]"),
            state.simulation_time,
        );
        state.adjust_player_trust(-0.08);

        let message = violation
            .message
            .unwrap_or_else(|| "Rule violation".to_string());

        let passenger_id = state.current_passenger.as_ref().map(|p| p.id);
        let forgiven_by = if state.rule_immunity_charges > 0 {
            state.rule_immunity_charges -= 1;
            Some(None)
        } else {
            ProtectionService::consume_ward(
                &mut state.inventory,
                ProtectionType::RuleForgiveness,
                passenger_id,
            )
            .map(|ward| Some(ward.describe()))
        };

        if let Some(ward_name) = forgiven_by {
            state.telemetry.ward_interventions += 1;
            state.queue_audio(
                "ward",
                format!("[Ward absorbs the {rule_title} violation]"),
                state.simulation_time,
            );
            let ward_label = ward_name.unwrap_or_else(|| "ward".to_string());
            state.current_dialogue = Some(CurrentDialogue {
                text: format!(
                    "The {} absorbs the {} violation. {}",
                    ward_label, rule_title, message
                ),
                speaker: DialogueSpeaker::Narrator,
                timestamp: state.simulation_time,
            });
            return None;
        }

        if state.rides_completed == 0 {
            state.current_dialogue = Some(CurrentDialogue {
                text: if hidden {
                    format!("Hidden rule revealed: {}. {}", rule_title, message)
                } else {
                    format!("Rule pressure spikes: {}. {}", rule_title, message)
                },
                speaker: DialogueSpeaker::Narrator,
                timestamp: state.simulation_time,
            });
            return None;
        }

        Some(RouteOutcome::GameOver(if hidden {
            format!("Hidden Rule Violated!\n{}", message)
        } else {
            message
        }))
    }

    fn apply_rule_need_adjustment(
        state: &mut GameState,
        outcome: &RuleEvaluationResult,
        current_time: f64,
    ) {
        if let (Some(mut need_state), Some(passenger)) = (
            state.current_passenger_need_state.clone(),
            state.current_passenger.clone(),
        ) {
            let triggered = PassengerStateMachine::apply_rule_outcome(
                &mut need_state,
                &passenger,
                outcome,
                current_time,
            );
            state.current_passenger_need_state = Some(need_state);
            PassengerStateMachine::merge_detected_tells(
                &mut state.rng,
                &mut state.detected_tells,
                triggered,
                &passenger,
                state.player_trust,
                current_time,
                &state.current_guidelines,
            );
        }
    }

    /// Taking the same road over and over draws attention.
    ///
    /// `CONSECUTIVE_ROUTE.PENALTY_PER_REPEAT` already trimmed the fare, but
    /// `WARNING_THRESHOLD` and `RISK_INCREASE_PER_REPEAT` were unread, so a
    /// streak cost money and nothing else — and the player was never told it
    /// was building. A `VIOLATION_THRESHOLD` was once authored at 999 — the
    /// data's way of saying "no violation" — and has since been deleted
    /// outright rather than kept as a sentinel nothing reads.
    fn apply_route_streak_pressure(
        state: &mut GameState,
        costs: &mut RouteCosts,
        route: RouteType,
        constants: &ConstantsData,
        current_time: f64,
    ) {
        let streak = &constants.consecutive_route;
        let repeats = state
            .consecutive_route_streak
            .as_ref()
            .filter(|s| s.route_type == route)
            .map(|s| s.count)
            .unwrap_or(0);

        if repeats < streak.warning_threshold {
            return;
        }

        let over = repeats + 1 - streak.warning_threshold;
        let added = (over * streak.risk_increase_per_repeat).min(constants.risk.max_risk_level);
        costs.risk = (costs.risk + added).min(constants.risk.max_risk_level);

        state.current_dialogue = Some(CurrentDialogue {
            text: format!(
                "That is {} runs the same way. The road is starting to expect you.",
                repeats + 1
            ),
            speaker: DialogueSpeaker::Narrator,
            timestamp: current_time,
        });
    }

    fn apply_curse_route_pressure(
        state: &mut GameState,
        costs: &mut RouteCosts,
        constants: &ConstantsData,
    ) {
        if state.curse_danger_bonus == 0 {
            return;
        }

        let bonus = state.curse_danger_bonus.min(3);
        costs.risk = (costs.risk + bonus).min(constants.risk.max_risk_level);
        costs.time += bonus * 2;
        state.curse_danger_bonus = state.curse_danger_bonus.saturating_sub(bonus);
    }

    /// Apply costs, tracking, and history
    fn apply_transit_effects(
        state: &mut GameState,
        costs: &RouteCosts,
        route: RouteType,
        current_time: f64,
    ) {
        // Deduct resources
        state.fuel -= costs.fuel as f32;
        state.time_remaining = state.time_remaining.saturating_sub(costs.time);

        // Update route tracking
        state.update_route_streak(route);
        let driving_phase = state.driving_phase.unwrap_or(DrivingPhase::Pickup);

        if let Some(current_ride) = &mut state.current_ride {
            current_ride.route_type = Some(route);
            current_ride.driving_phase = driving_phase;
        }

        // Record history
        let passenger_id = state.current_passenger.as_ref().map(|p| p.id);

        state.route_history.push(RouteHistoryEntry {
            route_type: route,
            driving_phase,
            fuel_cost: costs.fuel,
            time_cost: costs.time,
            risk_level: costs.risk,
            passenger_id,
            timestamp: current_time,
        });
    }

    /// Update passenger stress/state based on route
    pub fn update_passenger_state(
        state: &mut GameState,
        route: RouteType,
        current_time: f64,
        route_preference_stress_scale: f32,
        normal_route_need_relief: u32,
    ) {
        if let (Some(mut need_state), Some(passenger)) = (
            state.current_passenger_need_state.clone(),
            state.current_passenger.clone(),
        ) {
            // What this leg costs the passenger for their own rule staying
            // obeyed, when tonight's rules include it.
            let obey_pressure = GameEngine::passengers_rule_in_force(
                &state.current_rules,
                Some(&need_state),
                &state.current_guidelines,
            )
            .and_then(|rule| rule.follow_need_adjustment);

            let mut triggered_tells = PassengerStateMachine::apply_route_choice(
                &mut need_state,
                &passenger,
                route,
                obey_pressure,
                route_preference_stress_scale,
                current_time,
            );

            // Normal is the steady road: it gives up Shortcut's speed,
            // Scenic's meter, and Police's supernatural safety in exchange
            // for a quieter passenger and the smallest predictable fuel bill.
            // Apply this after preference so a passenger who fears Normal can
            // still dislike it; the route softens the leg rather than erasing
            // authored identity.
            if route == RouteType::Normal && normal_route_need_relief > 0 {
                let before = need_state.level;
                let relief_tells = PassengerStateMachine::apply_stress_delta(
                    &mut need_state,
                    &passenger,
                    -(normal_route_need_relief as i32),
                    current_time,
                );
                state.telemetry.normal_route_relief += before.saturating_sub(need_state.level);
                triggered_tells.extend(relief_tells);
            }

            if let Some(tell) = triggered_tells
                .iter()
                .find(|tell| tell.tell.audio_cue.is_some())
            {
                state.queue_audio(
                    tell.tell.audio_cue.as_deref().unwrap_or("warning"),
                    format!("[Passenger cue: {}]", tell.tell.description),
                    current_time,
                );
            }

            let weather_stress: i32 = state
                .current_weather
                .effects
                .iter()
                .filter(|effect| effect.effect_type == WeatherEffectType::PassengerBehavior)
                .map(|effect| (effect.value.max(0) / 5).max(1))
                .sum();
            if weather_stress > 0 {
                let weather_tells = PassengerStateMachine::apply_stress_delta(
                    &mut need_state,
                    &passenger,
                    weather_stress,
                    current_time,
                );
                triggered_tells.extend(weather_tells);
            }

            // A verbal tell that just fired is the most specific thing the
            // passenger is doing, so it outranks both the route reaction and
            // the generic stage line.
            let spoken_tell = PassengerStateMachine::spoken_tell(&triggered_tells);
            let route_dialogue = Self::route_reaction_dialogue(&mut state.rng, &passenger, route);
            let stage_dialogue = PassengerStateMachine::get_dialogue_for_stage(
                &mut state.rng,
                &passenger,
                &need_state,
            );
            let dialogue = spoken_tell.or(route_dialogue).or(stage_dialogue);

            state.current_passenger_need_state = Some(need_state);

            PassengerStateMachine::merge_detected_tells(
                &mut state.rng,
                &mut state.detected_tells,
                triggered_tells,
                &passenger,
                state.player_trust,
                current_time,
                &state.current_guidelines,
            );

            if let Some(dialogue) = dialogue {
                state.current_dialogue = Some(CurrentDialogue {
                    text: dialogue.clone(),
                    speaker: DialogueSpeaker::Passenger,
                    timestamp: current_time,
                });
                state.current_passenger_dialogue = Some(dialogue);
            }
        }
    }

    fn route_reaction_dialogue(
        rng: &mut macroquad_toolkit::rng::SeededRng,
        passenger: &Passenger,
        route: RouteType,
    ) -> Option<String> {
        let preference = passenger.get_route_preference(route)?;
        if preference.preference == PreferenceLevel::Neutral {
            return None;
        }

        let line = preference.special_dialogue.as_ref()?;
        let trigger_chance = preference.trigger_chance.unwrap_or(1.0).clamp(0.0, 1.0);
        if rng.next_f32() <= trigger_chance {
            Some(line.clone())
        } else {
            None
        }
    }

    fn is_passenger_meltdown(state: &GameState) -> bool {
        state
            .current_passenger_need_state
            .as_ref()
            .map(PassengerStateMachine::is_meltdown)
            .unwrap_or(false)
    }

    /// Try to take a meltdown on the chin using whatever protection is
    /// carried: first the `supernatural_protection` counter granted by item
    /// effects and skills, then an actual warding item from the inventory.
    fn absorb_meltdown_with_protection(state: &mut GameState, current_time: f64) -> bool {
        if !Self::is_passenger_meltdown(state) {
            return false;
        }

        let should_protect = state
            .current_passenger
            .as_ref()
            .map(|passenger| passenger.is_supernatural)
            .unwrap_or(false)
            || state
                .current_weather
                .effects
                .iter()
                .any(|effect| effect.effect_type == WeatherEffectType::SupernaturalAttraction);

        if !should_protect {
            return false;
        }

        // Spend the cheapest thing that will cover it: the counter first, so
        // a carried ward survives while a skill charge is available.
        let passenger_id = state.current_passenger.as_ref().map(|p| p.id);
        // A carried ward pulls the passenger back by its authored strength;
        // a bare counter charge is worth one step of it.
        const RELIEF_PER_STRENGTH: i32 = 18;
        let (absorbed_by, relief) = if state.supernatural_protection > 0 {
            state.supernatural_protection -= 1;
            (None, RELIEF_PER_STRENGTH * 2)
        } else {
            match ProtectionService::consume_ward(
                &mut state.inventory,
                ProtectionType::SupernaturalImmunity,
                passenger_id,
            ) {
                Some(ward) => (
                    Some(ward.describe()),
                    RELIEF_PER_STRENGTH * ward.strength.max(1) as i32,
                ),
                None => return false,
            }
        };
        state.telemetry.ward_interventions += 1;
        state.queue_audio(
            "ward",
            "[A ward flares and forces the supernatural pressure back]",
            current_time,
        );

        if let (Some(mut need_state), Some(passenger)) = (
            state.current_passenger_need_state.clone(),
            state.current_passenger.clone(),
        ) {
            let triggered = PassengerStateMachine::apply_stress_delta(
                &mut need_state,
                &passenger,
                -relief,
                current_time,
            );
            state.current_passenger_need_state = Some(need_state);
            PassengerStateMachine::merge_detected_tells(
                &mut state.rng,
                &mut state.detected_tells,
                triggered,
                &passenger,
                state.player_trust,
                current_time,
                &state.current_guidelines,
            );
            state.current_dialogue = Some(CurrentDialogue {
                text: match absorbed_by {
                    Some(name) => format!(
                        "The {} flares and pulls the passenger back from the edge.",
                        name
                    ),
                    None => "A protective charm flares and pulls the passenger back from the edge."
                        .to_string(),
                },
                speaker: DialogueSpeaker::Narrator,
                timestamp: current_time,
            });
        }

        !Self::is_passenger_meltdown(state)
    }

    /// The first meltdown is a brink, not a death.
    ///
    /// Meltdown was the only game over with neither a probability roll nor a
    /// warning leg — rule breaks roll 0.3–0.7 and guideline misreads roll
    /// their consequence, but the stage flipping killed on the spot. The
    /// first crossing of the *shift* now drags the passenger back under the
    /// threshold, announces itself, and burns the night's one
    /// `brink_spent` charge; the player has a leg to soothe, trade, or
    /// arrive. Any later crossing that night is final. (This started as a
    /// per-ride grace and measured too kind: with one absorb per passenger,
    /// baseline night 1 went 15/15 and the progression tiers stopped
    /// mattering. One per shift keeps the warning without the armor.)
    fn hold_at_the_brink(state: &mut GameState, current_time: f64) -> bool {
        if state.brink_spent {
            return false;
        }
        let Some(mut need_state) = state.current_passenger_need_state.clone() else {
            return false;
        };
        if need_state.brink_reached {
            return false;
        }
        state.brink_spent = true;
        state.telemetry.brink_saves += 1;
        state.queue_audio(
            "brink",
            "[Heartbeat: the passenger reaches the brink]",
            current_time,
        );
        need_state.brink_reached = true;
        need_state.level = need_state.profile.thresholds.meltdown.saturating_sub(1);
        need_state.stage =
            PassengerNeedState::calculate_stage(need_state.level, &need_state.profile.thresholds);
        need_state.stability = 1.0 - (need_state.level as f32 / 100.0);
        need_state.last_updated = current_time;
        state.current_passenger_need_state = Some(need_state);

        state.current_dialogue = Some(CurrentDialogue {
            text: "The passenger is at the brink — one more push and they are lost. \
                   Settle them, or get them there."
                .to_string(),
            speaker: DialogueSpeaker::Narrator,
            timestamp: current_time,
        });
        true
    }

    /// Check if guideline decision should be triggered.
    ///
    /// Fires on the ride's final leg, because resolving the decision completes
    /// the ride (`evaluate_guideline_decision` calls `complete_ride`). This
    /// used to gate on `DrivingPhase::Destination` during a period when the
    /// mid-ride-event flow never set it — so the decision, and with it every
    /// authored guideline exception, was unreachable. The event exit sets the
    /// phase honestly again now, but the leg count stays the gate: it cannot
    /// drift, whatever the phase enum does next.
    fn check_guideline_triggers(state: &mut GameState, current_time: f64) -> Option<RouteOutcome> {
        // `apply_transit_effects` has already recorded this leg, so a count
        // above one means we are on the second (final) leg of the ride.
        let passenger_id = state.current_passenger.as_ref().map(|p| p.id);
        let final_leg = state
            .route_history
            .iter()
            .filter(|r| r.passenger_id == passenger_id)
            .count()
            > 1;

        let should_check =
            !state.current_guidelines.is_empty() && !state.detected_tells.is_empty() && final_leg;

        if should_check {
            // Find a guideline that has detected tells
            if let Some(guideline) = state
                .current_guidelines
                .iter()
                .find(|g| {
                    state.detected_tells.iter().any(|t| {
                        if t.related_guideline != Some(g.id) {
                            return false;
                        }
                        match (&state.current_passenger_need_state, &t.exception_id) {
                            (Some(need_state), Some(exception_id)) => {
                                PassengerStateMachine::is_exception_active(need_state, exception_id)
                            }
                            _ => true,
                        }
                    })
                })
                .cloned()
            {
                // Enter guideline decision phase
                state.active_guideline = Some(guideline);
                state.guideline_decision_start_time = Some(current_time);
                state.guideline_time_remaining = state.guideline_decision_seconds;
                state.game_phase = GamePhase::GuidelineDecision;
                return Some(RouteOutcome::GuidelineTriggered);
            }
        }
        None
    }

    /// Advance game phase
    fn transition_driving_phase(
        state: &mut GameState,
        data: &GameData,
        stats: &mut PlayerStats,
        route: RouteType,
        current_time: f64,
    ) -> RouteOutcome {
        match state.driving_phase {
            Some(DrivingPhase::Pickup) => {
                // Check how many routes we've taken with this passenger
                let pid = state.current_passenger.as_ref().map(|p| p.id);
                let ride_legs = state
                    .route_history
                    .iter()
                    .filter(|r| r.passenger_id == pid)
                    .count();

                if ride_legs <= 1 {
                    // First leg -> Mid-Ride Event. The generator reads the
                    // whole state, so the stream is copied out and written
                    // back — SeededRng is Copy, and the draws must count.
                    let mut rng_for_event = state.rng;
                    state.current_event = Some(Self::generate_mid_ride_event(
                        &mut rng_for_event,
                        state,
                        data,
                        stats,
                        route,
                    ));
                    state.rng = rng_for_event;
                    state.game_phase = GamePhase::Interaction;
                    RouteOutcome::Success
                } else {
                    // Defensive: the final leg normally arrives with the
                    // Destination phase set by the event exit.
                    Self::complete_ride(state, data, stats, route, current_time);
                    RouteOutcome::RideCompleted
                }
            }
            Some(DrivingPhase::Destination) => {
                // Final leg -> Complete Ride (DropOff)
                Self::complete_ride(state, data, stats, route, current_time);
                RouteOutcome::RideCompleted
            }
            None => RouteOutcome::Success,
        }
    }
}

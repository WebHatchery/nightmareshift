use super::Game;
use crate::data::{
    Consequence, ConsequenceType, Passenger, ProtectionType, ReputationConstants, RouteType,
};
use crate::engine::*;
use crate::screens::Screen;
use crate::state::*;

/// What applying a consequence set did, for the caller that owns the kill.
struct AppliedConsequences {
    death_fired: bool,
    /// The fired death's authored description, destined for
    /// `game_over_reason` — a ledger line would never be read.
    death_note: Option<String>,
}

impl Game {
    /// The reputation thresholds, or nothing before the data has loaded.
    fn reputation_constants(&self) -> Option<ReputationConstants> {
        self.game_data
            .as_ref()
            .map(|data| data.constants.reputation.clone())
    }

    pub(super) fn perform_rule_action(&mut self, action_key: String) {
        if !self.can_perform_cab_action(&action_key) {
            self.game_state.current_dialogue = Some(CurrentDialogue {
                text: format!(
                    "{} is not useful right now.",
                    Self::cab_action_label(&action_key)
                ),
                speaker: DialogueSpeaker::Driver,
                timestamp: self.game_state.simulation_time,
            });
            return;
        }

        let current_time = self.game_state.simulation_time;
        let visible = GameEngine::check_rule_violation(
            &self.game_state.current_rules,
            &action_key,
            self.game_state.current_passenger_need_state.as_ref(),
            &self.game_state.current_guidelines,
        );

        if visible.violation || visible.rule.is_some() {
            self.resolve_cab_rule_action(visible, false, &action_key, current_time);
            return;
        }

        let hidden = GameEngine::check_rule_violation(
            &self.game_state.hidden_rules,
            &action_key,
            self.game_state.current_passenger_need_state.as_ref(),
            &self.game_state.current_guidelines,
        );

        if hidden.violation || hidden.rule.is_some() {
            let lands = if let Some(data) = self.game_data.as_ref() {
                GameEngine::hidden_violation_lands(&mut self.game_state.rng, &data.constants)
            } else {
                false
            };
            if !hidden.violation || lands {
                self.resolve_cab_rule_action(hidden, true, &action_key, current_time);
                return;
            }
            // Got away with it, and learned nothing by getting away with it.
            self.game_state.current_dialogue = Some(CurrentDialogue {
                text: "Something shifts in the cab, thinks better of it, and settles.".to_string(),
                speaker: DialogueSpeaker::Narrator,
                timestamp: current_time,
            });
            return;
        }

        // No rule in force forbids the action. This used to be a dead end —
        // eight cab controls whose only unforbidden effect was +0.01 trust —
        // which left the meltdown clock with no counterplay outside the
        // exception system. A comfort-upgraded cab now soothes here: each
        // channel once per ride, so the meter still climbs over a long night.
        if self.try_comfort_soothe(&action_key, current_time) {
            return;
        }

        self.game_state.adjust_player_trust(0.01);
        self.game_state.current_dialogue = Some(CurrentDialogue {
            text: format!(
                "You {}. Nothing answers.",
                Self::cab_action_phrase(&action_key)
            ),
            speaker: DialogueSpeaker::Driver,
            timestamp: current_time,
        });
    }

    /// Spend a comfort upgrade's soothing on the current passenger, if the
    /// action has one, it has not been used this ride, and there is anything
    /// to soothe. Returns whether it did.
    fn try_comfort_soothe(&mut self, action_key: &str, current_time: f64) -> bool {
        let Some(data) = self.game_data.as_ref() else {
            return false;
        };
        let mods = SkillModifiers::from_unlocked(&data.skills, &self.player_stats.unlocked_skills);
        let relief = match action_key {
            "play_music" => mods.soothe_music,
            "use_ac" | "open_window" => mods.soothe_climate,
            "eye_contact" | "stop_vehicle" => mods.soothe_presence,
            _ => 0,
        };
        if relief <= 0
            || self
                .game_state
                .comfort_soothed_actions
                .iter()
                .any(|a| a == action_key)
        {
            return false;
        }
        let (Some(mut need), Some(passenger)) = (
            self.game_state.current_passenger_need_state.clone(),
            self.game_state.current_passenger.clone(),
        ) else {
            return false;
        };
        if need.level == 0 {
            return false;
        }

        self.game_state
            .comfort_soothed_actions
            .push(action_key.to_string());
        let before = need.level;
        let triggered =
            PassengerStateMachine::apply_stress_delta(&mut need, &passenger, -relief, current_time);
        self.game_state.telemetry.comfort_relief += before.saturating_sub(need.level);
        self.game_state.current_passenger_need_state = Some(need);
        PassengerStateMachine::merge_detected_tells(
            &mut self.game_state.rng,
            &mut self.game_state.detected_tells,
            triggered,
            &passenger,
            self.game_state.player_trust,
            current_time,
            &self.game_state.current_guidelines,
        );
        self.game_state.adjust_player_trust(0.02);
        self.game_state.current_dialogue = Some(CurrentDialogue {
            text: format!(
                "You {}. The cab does its quiet work; the passenger eases a little.",
                Self::cab_action_phrase(action_key)
            ),
            speaker: DialogueSpeaker::Narrator,
            timestamp: current_time,
        });
        true
    }

    fn resolve_cab_rule_action(
        &mut self,
        result: RuleEvaluationResult,
        hidden: bool,
        action_key: &str,
        current_time: f64,
    ) {
        if hidden {
            if let Some(rule) = &result.rule {
                self.game_state.reveal_hidden_rule(rule.id);
            }
        }

        self.apply_rule_need_adjustment(&result, current_time);

        let rule_title = result
            .rule
            .as_ref()
            .map(|rule| rule.title.clone())
            .unwrap_or_else(|| "Rule".to_string());

        if result.violation {
            self.game_state.rules_violated += 1;
            self.game_state.queue_audio(
                "violation",
                format!("[Rule violation: {rule_title}]"),
                current_time,
            );
            self.game_state.adjust_player_trust(-0.1);

            let message = result
                .message
                .clone()
                .unwrap_or_else(|| "You violated a rule.".to_string());

            // Immunity charges from item effects and skills first, then an
            // actual `RuleForgiveness` ward carried in the inventory.
            let passenger_id = self.game_state.current_passenger.as_ref().map(|p| p.id);
            let forgiven_by = if self.game_state.rule_immunity_charges > 0 {
                self.game_state.rule_immunity_charges -= 1;
                Some("ward".to_string())
            } else {
                ProtectionService::consume_ward(
                    &mut self.game_state.inventory,
                    ProtectionType::RuleForgiveness,
                    passenger_id,
                )
                .map(|ward| ward.describe())
            };

            if let Some(ward_label) = forgiven_by {
                self.game_state.telemetry.ward_interventions += 1;
                self.game_state.queue_audio(
                    "ward",
                    format!("[{ward_label} absorbs the {rule_title} violation]"),
                    current_time,
                );
                self.game_state.current_dialogue = Some(CurrentDialogue {
                    text: format!(
                        "The {} absorbs the {} violation. {}",
                        ward_label, rule_title, message
                    ),
                    speaker: DialogueSpeaker::Narrator,
                    timestamp: current_time,
                });
                return;
            }

            if self.game_state.rides_completed == 0 {
                self.game_state.current_dialogue = Some(CurrentDialogue {
                    text: if hidden {
                        format!("Hidden rule revealed: {}. {}", rule_title, message)
                    } else {
                        format!("Rule pressure spikes: {}. {}", rule_title, message)
                    },
                    speaker: DialogueSpeaker::Narrator,
                    timestamp: current_time,
                });
                return;
            }

            let break_consequences = result
                .rule
                .as_ref()
                .map(|rule| rule.break_consequences.clone())
                .unwrap_or_default();
            let death_authored = break_consequences
                .iter()
                .any(|consequence| consequence.consequence_type == ConsequenceType::Death);
            let applied = self.apply_rule_consequences(&break_consequences, current_time);

            // Thirteen rules author their own death odds, 0.3 to 0.7, and
            // every one of them used to kill at 1.0 — the roll happened and
            // the shift ended anyway. A rule that authors no death at all
            // keeps the old certainty.
            if death_authored && !applied.death_fired {
                self.game_state.current_dialogue = Some(CurrentDialogue {
                    text: format!(
                        "Rule broken: {}. {} The moment passes; somehow you are still driving.",
                        rule_title, message
                    ),
                    speaker: DialogueSpeaker::Narrator,
                    timestamp: current_time,
                });
                return;
            }

            // The authored death prose rides along with the violation
            // message — it was written for exactly this screen.
            let reason = match applied.death_note {
                Some(note) => format!("{} {}", message, note),
                None => message,
            };
            self.game_state.game_over_reason = Some(reason);
            self.end_shift(false);
        } else {
            self.game_state.adjust_player_trust(0.05);
            if let Some(rule) = &result.rule {
                self.apply_rule_consequences(&rule.exception_rewards, current_time);
            }
            self.game_state.current_dialogue = Some(CurrentDialogue {
                text: format!(
                    "{} was dangerous on paper, but the passenger needed it.",
                    Self::cab_action_label(action_key)
                ),
                speaker: DialogueSpeaker::Narrator,
                timestamp: current_time,
            });
        }
    }

    fn apply_rule_need_adjustment(&mut self, result: &RuleEvaluationResult, current_time: f64) {
        if let (Some(mut need_state), Some(passenger)) = (
            self.game_state.current_passenger_need_state.clone(),
            self.game_state.current_passenger.clone(),
        ) {
            let triggered = PassengerStateMachine::apply_rule_outcome(
                &mut need_state,
                &passenger,
                result,
                current_time,
            );
            self.game_state.current_passenger_need_state = Some(need_state);
            PassengerStateMachine::merge_detected_tells(
                &mut self.game_state.rng,
                &mut self.game_state.detected_tells,
                triggered,
                &passenger,
                self.game_state.player_trust,
                current_time,
                &self.game_state.current_guidelines,
            );
        }
    }

    /// Keep a fired consequence's authored line for the drop-off ledger.
    fn note_consequence(&mut self, description: &str) {
        if !description.is_empty() {
            self.game_state
                .consequence_notes
                .push(description.to_string());
        }
    }

    /// Applies each consequence that wins its probability roll. Non-death
    /// descriptions land in the ride's `consequence_notes` ledger; the death
    /// outcome is returned because the caller owns ending the shift, and its
    /// prose belongs in `game_over_reason` rather than a ledger nobody will
    /// live to read.
    fn apply_rule_consequences(
        &mut self,
        consequences: &[Consequence],
        current_time: f64,
    ) -> AppliedConsequences {
        let mut applied = AppliedConsequences {
            death_fired: false,
            death_note: None,
        };
        for consequence in consequences {
            if self.game_state.rng.next_f32() > consequence.probability.clamp(0.0, 1.0) {
                continue;
            }

            match consequence.consequence_type {
                ConsequenceType::Death => {
                    applied.death_fired = true;
                    if !consequence.description.is_empty() {
                        applied.death_note = Some(consequence.description.clone());
                    }
                }
                ConsequenceType::Survival => {
                    // The same payment the guideline path makes: kept faith
                    // steadies the driver.
                    self.game_state.adjust_player_trust(0.1);
                }
                ConsequenceType::Reputation => {
                    if let Some(passenger_id) = self
                        .game_state
                        .current_passenger
                        .as_ref()
                        .map(|passenger| passenger.id)
                    {
                        if let Some(constants) = self.reputation_constants() {
                            self.game_state
                                .get_passenger_reputation(passenger_id)
                                .adjust(consequence.value, &constants);
                        }
                    }
                }
                ConsequenceType::Money => {
                    if consequence.value >= 0 {
                        self.game_state.earnings += consequence.value as u32;
                    } else {
                        self.game_state.earnings = self
                            .game_state
                            .earnings
                            .saturating_sub(consequence.value.unsigned_abs());
                    }
                }
                ConsequenceType::Fuel => {
                    self.game_state.fuel = (self.game_state.fuel + consequence.value as f32)
                        .clamp(0.0, self.game_state.max_fuel);
                }
                ConsequenceType::Time => {
                    if consequence.value >= 0 {
                        self.game_state.time_remaining += consequence.value as u32;
                    } else {
                        self.game_state.time_remaining = self
                            .game_state
                            .time_remaining
                            .saturating_sub(consequence.value.unsigned_abs());
                    }
                }
                ConsequenceType::Item => {
                    let source = self
                        .game_state
                        .current_passenger
                        .as_ref()
                        .map(|passenger| passenger.name.clone());
                    if let (Some(source), Some(data)) = (source, self.game_data.as_ref()) {
                        // The authored name and count, not a hardcoded
                        // crumpled note: rule 2's mixtape and rule 4's
                        // talisman were both minted as notes.
                        let name = consequence.item.as_deref().unwrap_or("crumpled note");
                        let count = consequence.value.max(1) as usize;
                        let items: Vec<_> = (0..count)
                            .map(|_| data.items.create_item(name, &source, current_time))
                            .collect();
                        self.game_state.inventory.extend(items);
                    }
                }
                ConsequenceType::StoryUnlock => self.unlock_passenger_story(),
            }

            if consequence.consequence_type != ConsequenceType::Death {
                self.note_consequence(&consequence.description);
            }
        }
        applied
    }

    /// Pay a rule's `followConsequences` for having kept it all ride.
    ///
    /// Thirteen of the twenty-eight rules author one — rule 1's is "You
    /// resisted the passenger's pull and finished the ride unharmed" — and
    /// none had ever paid, so a rule was pure downside. It could kill you and
    /// it could be excused, but keeping it was worth nothing.
    ///
    /// Only the passenger's own rule pays, the one their exception belongs to.
    /// That is deliberate and matches how `followNeedAdjustment` is spent: it
    /// is the rule they were pulling against, so it is the one that took
    /// something to keep. Paying every rule in force would hand out three to
    /// five rewards a ride and slam the driver's trust to its ceiling.
    ///
    /// And only if it was actually kept. A violation is usually fatal but not
    /// always — a ward absorbs one and the first ride of a shift is forgiven —
    /// so reaching the drop-off does not prove obedience on its own.
    pub(super) fn pay_for_keeping_the_rule(&mut self, current_time: f64) {
        if self.game_state.rules_violated != self.game_state.violations_at_ride_start {
            return;
        }
        let rewards = GameEngine::passengers_rule_in_force(
            &self.game_state.current_rules,
            self.game_state.current_passenger_need_state.as_ref(),
            &self.game_state.current_guidelines,
        )
        .map(|rule| rule.follow_consequences.clone())
        .unwrap_or_default();

        if !rewards.is_empty() {
            self.apply_rule_consequences(&rewards, current_time);
        }
    }

    /// Reveal a sliver of the current passenger's story.
    ///
    /// `StoryUnlock` only marked the passenger *encountered*, which every ride
    /// does anyway, so the consequence authored as "a sliver of the
    /// passenger's story reveals itself" revealed nothing. Unlocking the
    /// backstory is what that sentence describes, and it pays twice over: the
    /// almanac has something new to show, and `calculate_drop_chance` gives a
    /// passenger whose story is known a half again the chance of leaving an
    /// item behind.
    fn unlock_passenger_story(&mut self) {
        if let Some(passenger_id) = self
            .game_state
            .current_passenger
            .as_ref()
            .map(|passenger| passenger.id)
        {
            self.player_stats.reveal_story(passenger_id);
        }
    }

    fn can_perform_cab_action(&self, action_key: &str) -> bool {
        if self.screen != Screen::Game || self.game_state.current_passenger.is_none() {
            return false;
        }

        match action_key {
            "accept_tip" => self.game_state.game_phase == GamePhase::DropOff,
            "stop_vehicle" => matches!(
                self.game_state.game_phase,
                GamePhase::Driving | GamePhase::Interaction
            ),
            _ => matches!(
                self.game_state.game_phase,
                GamePhase::RideRequest
                    | GamePhase::Driving
                    | GamePhase::Interaction
                    | GamePhase::GuidelineDecision
                    | GamePhase::DropOff
            ),
        }
    }

    fn cab_action_label(action_key: &str) -> &'static str {
        match action_key {
            "eye_contact" => "Make Eye Contact",
            "play_music" => "Play Music",
            "accept_tip" => "Accept Tip",
            "open_window" => "Open Window",
            "use_wipers" => "Use Wipers",
            "drive_dark" => "Kill Headlights",
            "use_ac" => "Use AC",
            "stop_vehicle" => "Stop Cab",
            _ => "Cab Action",
        }
    }

    fn cab_action_phrase(action_key: &str) -> &'static str {
        match action_key {
            "eye_contact" => "meet the passenger's eyes",
            "play_music" => "turn on the radio",
            "accept_tip" => "accept the offered tip",
            "open_window" => "crack the window",
            "use_wipers" => "switch on the wipers",
            "drive_dark" => "kill the headlights",
            "use_ac" => "turn on the AC",
            "stop_vehicle" => "pull the cab over",
            _ => "try the control",
        }
    }

    /// Hand over the item at `item_idx` and take the passenger's offer.
    ///
    /// Giving a passenger something on their `wantedItems` list settles them
    /// and earns standing; giving them any other tradeable object still
    /// completes the swap but is just a swap. Before this, every trade
    /// resolved identically and the authored wants only ever decided whether
    /// an offer appeared at all.
    pub(super) fn complete_trade(&mut self, item_idx: usize) {
        // Bail before consuming the offer: a stale index used to take the
        // pending trade and drop the offered item on the floor.
        if item_idx >= self.game_state.inventory.len() {
            return;
        }
        let Some((_, offered_item)) = self.game_state.pending_trade.take() else {
            return;
        };

        let given = self.game_state.inventory.remove(item_idx);
        self.game_state.inventory.push(offered_item);

        let current_time = self.game_state.simulation_time;
        let Some(passenger) = self.game_state.current_passenger.clone() else {
            return;
        };
        if !passenger.wanted_items.contains(&given.name) {
            // Say so. The swap still happened and the offered item is still
            // the payment, but nothing marked the difference between this and
            // handing over what they asked for, so the wanted list taught the
            // player nothing after the first click.
            self.game_state.trade_outcome = Some(TradeOutcome {
                text: format!(
                    "{} takes the {} and sets it aside. It was not what they came for.",
                    passenger.name, given.name
                ),
                was_wanted: false,
            });
            return;
        }

        self.game_state.trade_outcome = Some(TradeOutcome {
            text: format!(
                "{} turns the {} over in their hands. It was what they came for.",
                passenger.name, given.name
            ),
            was_wanted: true,
        });

        let Some(bonus) = self
            .game_data
            .as_ref()
            .map(|data| data.rewards.wanted_trade)
            .filter(|bonus| bonus.is_active())
        else {
            return;
        };

        if bonus.reputation_bonus > 0 {
            if let Some(constants) = self.reputation_constants() {
                self.game_state
                    .get_passenger_reputation(passenger.id)
                    .adjust(bonus.reputation_bonus as i32, &constants);
            }
        }

        if bonus.need_relief > 0 {
            if let Some(mut need) = self.game_state.current_passenger_need_state.clone() {
                let triggered = PassengerStateMachine::apply_stress_delta(
                    &mut need,
                    &passenger,
                    -bonus.need_relief,
                    current_time,
                );
                self.game_state.current_passenger_need_state = Some(need);
                PassengerStateMachine::merge_detected_tells(
                    &mut self.game_state.rng,
                    &mut self.game_state.detected_tells,
                    triggered,
                    &passenger,
                    self.game_state.player_trust,
                    current_time,
                    &self.game_state.current_guidelines,
                );
            }
        }
    }

    /// Reading a passenger correctly settles them.
    ///
    /// Every `stateProfile` authors an `exceptionRelief` and names the
    /// `exceptionId` that earns it. This is the only downward pressure on a
    /// passenger's need — without it the level only ever rises and the ride
    /// ends in meltdown regardless of how well the player plays. Relief is
    /// paid only for the passenger's own exception; correctly reading some
    /// other guideline is safe, but it is not what this passenger needed.
    fn relieve_need_for_exception(
        &mut self,
        satisfied: Option<&str>,
        dormant: bool,
        passenger: &Passenger,
        current_time: f64,
    ) {
        let Some(satisfied) = satisfied else {
            return;
        };
        let Some(need) = self.game_state.current_passenger_need_state.as_mut() else {
            return;
        };
        if need.profile.exception_id.as_deref() != Some(satisfied) {
            return;
        }

        // A dormant exception pays half: the read was right, but tonight the
        // need had less to give back — the weaker fallback the sweep asked
        // after, instead of the all-or-nothing liveness cliff.
        let full = need.profile.need_change.exception_relief;
        let relief = if dormant { full / 2 } else { full };
        if relief <= 0 {
            return;
        }

        let mut need = need.clone();
        let triggered =
            PassengerStateMachine::apply_stress_delta(&mut need, passenger, -relief, current_time);
        self.game_state.current_passenger_need_state = Some(need);
        PassengerStateMachine::merge_detected_tells(
            &mut self.game_state.rng,
            &mut self.game_state.detected_tells,
            triggered,
            passenger,
            self.game_state.player_trust,
            current_time,
            &self.game_state.current_guidelines,
        );
        self.game_state.current_dialogue = Some(CurrentDialogue {
            text: "They settle back into the seat. You read them right.".to_string(),
            speaker: DialogueSpeaker::Narrator,
            timestamp: current_time,
        });
    }

    pub(super) fn evaluate_guideline_decision(&mut self, action: GuidelineAction) {
        let current_time = self.game_state.simulation_time;

        if let (Some(guideline), Some(passenger)) = (
            self.game_state.active_guideline.clone(),
            self.game_state.current_passenger.clone(),
        ) {
            let result = GuidelineEngine::evaluate_guideline_choice(
                &guideline,
                action,
                &passenger,
                &self.game_state,
            );

            let tells_present: Vec<_> = self
                .game_state
                .detected_tells
                .iter()
                .filter(|tell| tell.related_guideline == Some(guideline.id))
                .map(|tell| tell.tell.clone())
                .collect();

            self.game_state.decision_history.push(GuidelineDecision {
                guideline_id: guideline.id,
                passenger_id: passenger.id,
                action,
                was_correct: result.is_safe,
                tells_present,
                timestamp: current_time,
            });

            if result.is_safe {
                self.game_state.adjust_player_trust(0.08);
            } else {
                self.game_state.adjust_player_trust(-0.12);
            }

            self.relieve_need_for_exception(
                result.satisfied_exception.as_deref(),
                result.dormant,
                &passenger,
                current_time,
            );

            for consequence in &result.consequences {
                match consequence.consequence_type {
                    ConsequenceType::Death => {
                        if self.game_state.rng.next_f32() < consequence.probability.clamp(0.0, 1.0)
                        {
                            self.end_shift(false);
                            // The authored death line joins the verdict —
                            // written for this screen, shown for the first
                            // time. Never echo a description that just
                            // restates the verdict.
                            let reason = if consequence.description.is_empty()
                                || consequence.description == result.message
                            {
                                result.message.clone()
                            } else {
                                format!("{} {}", result.message, consequence.description)
                            };
                            self.game_state.game_over_reason = Some(reason);
                            return;
                        }
                    }
                    ConsequenceType::Survival => {
                        self.game_state.player_trust =
                            (self.game_state.player_trust + 0.1).min(1.0);
                        self.note_consequence(&consequence.description);
                    }
                    ConsequenceType::Reputation => {
                        if let Some(constants) = self.reputation_constants() {
                            self.game_state
                                .get_passenger_reputation(passenger.id)
                                .adjust(consequence.value, &constants);
                        }
                        self.note_consequence(&consequence.description);
                    }
                    ConsequenceType::StoryUnlock => {
                        self.unlock_passenger_story();
                        self.note_consequence(&consequence.description);
                    }
                    ConsequenceType::Item => {}
                    _ => {}
                }
            }

            self.game_state.active_guideline = None;
            self.game_state.guideline_decision_start_time = None;
            self.game_state.detected_tells.clear();
            self.game_state.false_tell_planted = false;

            let completion_route = self
                .game_state
                .current_ride
                .as_ref()
                .and_then(|ride| ride.route_type)
                .or_else(|| {
                    self.game_state
                        .route_history
                        .last()
                        .map(|entry| entry.route_type)
                })
                .unwrap_or(RouteType::Normal);
            self.complete_ride(completion_route);
        }
    }
}

//! Guideline exception detection engine.

use crate::data::*;
use crate::state::*;
use std::collections::HashSet;

/// Result of guideline evaluation
#[derive(Debug, Clone)]
pub struct GuidelineEvaluationResult {
    pub is_safe: bool,
    pub consequences: Vec<Consequence>,
    pub message: String,
    /// The exception the player read correctly, if any. Naming it lets the
    /// caller relieve the passenger's need when it is the one their
    /// `stateProfile.exceptionId` points at.
    pub satisfied_exception: Option<String>,
    /// The satisfied exception was authored but rolled dormant tonight: a
    /// right read of a real tell on a night the need never fully surfaced.
    /// Relief pays at half, and the break is spared the full "dangerous"
    /// death roll — the weaker fallback the balance sweep asked after.
    pub dormant: bool,
}

/// Guideline engine for exception detection and evaluation
pub struct GuidelineEngine;

impl GuidelineEngine {
    /// Roll which of the guidelines' exceptions are live for this passenger.
    ///
    /// Every exception authors a `probability` (0.3–0.6 across the data) and
    /// for the life of the port none was ever rolled — every matching
    /// exception was always in play, so the night was more predictable than
    /// the data asked it to be. Rolled once per presented passenger and kept
    /// on the state, because the tell system, the decision judge, and the
    /// bot must all see the same answer.
    pub fn roll_exception_liveness(
        rng: &mut macroquad_toolkit::rng::SeededRng,
        passenger: &Passenger,
        guidelines: &[Guideline],
    ) -> HashSet<String> {
        let mut live = HashSet::new();
        for guideline in guidelines {
            for exception in &guideline.exceptions {
                if !Self::passenger_matches_exception(passenger, exception) {
                    continue;
                }
                if rng.next_f32() < exception.probability.clamp(0.0, 1.0) {
                    live.insert(exception.id.clone());
                }
            }
        }
        live
    }

    /// Analyze passenger for active tells based on guidelines.
    ///
    /// `already_detected` carries the descriptions the merge would discard
    /// anyway. Skipping them here is not an optimisation: this runs every
    /// frame, each skipped tell is a notice roll not drawn, and a draw per
    /// frame ties the gameplay stream to the frame rate — the exact leak
    /// the seeded-run verification caught. The first roll for a tell was
    /// always the one that counted; now it is the only one made.
    pub fn analyze_passenger(
        rng: &mut macroquad_toolkit::rng::SeededRng,
        passenger: &Passenger,
        weather: &WeatherCondition,
        player_trust: f32,
        guidelines: &[Guideline],
        live_exceptions: &HashSet<String>,
        already_detected: &HashSet<String>,
        current_time: f64,
    ) -> Vec<DetectedTell> {
        let mut detected = Vec::new();

        for guideline in guidelines {
            for exception in &guideline.exceptions {
                // Check if passenger matches exception
                if !Self::passenger_matches_exception(passenger, exception) {
                    continue;
                }

                // The ride's liveness roll gates everything downstream: a
                // rolled-out exception emits no tells and satisfies nothing.
                if !live_exceptions.contains(&exception.id) {
                    continue;
                }

                // Check if conditions are met
                if !Self::check_exception_conditions(exception, weather, passenger) {
                    continue;
                }

                // Add detected tells
                for tell in &exception.tells {
                    if already_detected.contains(&tell.description) {
                        continue;
                    }
                    let player_noticed =
                        Self::calculate_detection_probability(rng, tell, passenger, player_trust);
                    detected.push(DetectedTell {
                        tell: tell.clone(),
                        passenger_id: passenger.id,
                        detection_time: current_time,
                        player_noticed,
                        related_guideline: Some(guideline.id),
                        exception_id: Some(exception.id.clone()),
                    });
                }
            }
        }

        detected
    }

    /// Update detection cycle for game loop
    pub fn update_detection(state: &mut GameState, stats: &PlayerStats, current_time: f64) {
        if matches!(
            state.game_phase,
            GamePhase::Driving | GamePhase::Interaction
        ) {
            let passenger_opt = state.current_passenger.clone();
            if let Some(passenger) = passenger_opt {
                let weather = state.current_weather.clone();
                let player_trust = state.player_trust;
                let guidelines = state.current_guidelines.clone();

                let live_exceptions = state.live_exceptions.clone();
                let decision_history = state.decision_history.clone();
                let already_detected: HashSet<String> = state
                    .detected_tells
                    .iter()
                    .filter(|tell| tell.passenger_id == passenger.id)
                    .map(|tell| tell.tell.description.clone())
                    .collect();
                let mut new_tells = Self::analyze_passenger(
                    &mut state.rng,
                    &passenger,
                    &weather,
                    player_trust,
                    &guidelines,
                    &live_exceptions,
                    &already_detected,
                    current_time,
                );

                // Introduce a false tell for experienced players — at most
                // one per decision window, and the gate rolls exactly once.
                // A failed roll used to retry every frame, which made the
                // authored 30/50% odds effectively 100% and tied the number
                // of stream draws to the frame rate. Eligibility cannot
                // change inside a window (the decision record only moves at
                // ride end), so one roll settles the question.
                if !state.false_tell_planted {
                    if Self::should_introduce_false_tells(&mut state.rng, &decision_history, stats)
                    {
                        if let Some(false_tell) = Self::conjure_false_tell(
                            &mut state.rng,
                            &passenger,
                            &weather,
                            &guidelines,
                            &live_exceptions,
                            current_time,
                        ) {
                            new_tells.push(false_tell);
                        }
                    }
                    state.false_tell_planted = true;
                }

                // Merge
                for tell in new_tells {
                    if !state.detected_tells.iter().any(|t| {
                        t.tell.description == tell.tell.description
                            && t.passenger_id == tell.passenger_id
                    }) {
                        state.detected_tells.push(tell);
                    }
                }
            }
        }
    }

    /// A tell borrowed from an exception that does not apply to this
    /// passenger — bait to break a guideline that should be kept.
    ///
    /// This is the only honest way to lie. The old code cloned a tell the
    /// passenger had genuinely emitted: the merge dedupe discarded it as the
    /// duplicate it was, and had it landed it would only have shown the same
    /// sentence twice. A tell for a dormant breaking-safer exception reads
    /// exactly like the real thing, and acting on it walks into "Breaking X
    /// was dangerous".
    pub fn conjure_false_tell(
        rng: &mut macroquad_toolkit::rng::SeededRng,
        passenger: &Passenger,
        weather: &WeatherCondition,
        guidelines: &[Guideline],
        live_exceptions: &HashSet<String>,
        current_time: f64,
    ) -> Option<DetectedTell> {
        let candidates: Vec<_> = guidelines
            .iter()
            .flat_map(|guideline| {
                guideline
                    .exceptions
                    .iter()
                    .map(move |exception| (guideline, exception))
            })
            .filter(|(_, exception)| {
                // Dormant means any leg of liveness fails — including an
                // exception that matched but lost its ride roll, which makes
                // the most plausible bait of all.
                exception.breaking_safer
                    && !(Self::passenger_matches_exception(passenger, exception)
                        && live_exceptions.contains(&exception.id)
                        && Self::check_exception_conditions(exception, weather, passenger))
            })
            .flat_map(|(guideline, exception)| {
                exception
                    .tells
                    .iter()
                    .map(move |tell| (guideline, exception, tell))
            })
            .collect();

        if candidates.is_empty() {
            return None;
        }
        let pick = rng.below(candidates.len());
        let (guideline, exception, tell) = candidates[pick];
        Some(DetectedTell {
            tell: tell.clone(),
            passenger_id: passenger.id,
            detection_time: current_time,
            // A lie nobody sees is no lie: unnoticed tells never display.
            player_noticed: true,
            related_guideline: Some(guideline.id),
            exception_id: Some(exception.id.clone()),
        })
    }

    /// Check if passenger matches an exception
    pub fn passenger_matches_exception(
        passenger: &Passenger,
        exception: &GuidelineException,
    ) -> bool {
        // Check by ID
        if !exception.passenger_ids.is_empty() && exception.passenger_ids.contains(&passenger.id) {
            return true;
        }

        // Check by type
        if !exception.passenger_types.is_empty()
            && exception.passenger_types.contains(&passenger.supernatural)
        {
            return true;
        }

        // Check by exception ID in passenger data
        passenger.guideline_exceptions.contains(&exception.id)
    }

    /// Check if exception conditions are met
    pub fn check_exception_conditions(
        exception: &GuidelineException,
        weather: &WeatherCondition,
        passenger: &Passenger,
    ) -> bool {
        for condition in &exception.conditions {
            let met = match condition.condition_type.as_str() {
                "passenger_dialogue" => {
                    let value = condition.value.as_str().unwrap_or("");
                    passenger
                        .dialogue
                        .iter()
                        .any(|line| line.to_lowercase().contains(&value.to_lowercase()))
                }
                "passenger_behavior" => {
                    if let Some(value) = condition.value.as_f64() {
                        Self::compare_values(
                            passenger.stress_level as f64,
                            value,
                            condition.operator.as_deref(),
                        )
                    } else {
                        true
                    }
                }
                "environmental" => {
                    let value = condition.value.as_str().unwrap_or("");
                    format!("{:?}", weather.weather_type).to_lowercase() == value.to_lowercase()
                }
                _ => true,
            };

            if !met {
                return false;
            }
        }

        true
    }

    /// Compare values with optional operator
    fn compare_values(actual: f64, expected: f64, operator: Option<&str>) -> bool {
        match operator {
            Some("greater_than") => actual > expected,
            Some("less_than") => actual < expected,
            Some("equals") | None => (actual - expected).abs() < f64::EPSILON,
            _ => true,
        }
    }

    /// Calculate if the player notices a tell.
    ///
    /// Two authored passenger fields feed this and were read by nothing.
    /// `deceptionLevel` is how well a passenger covers what they are, and
    /// scales the tell down; `trustRequired` is how much they need to trust
    /// the driver before they let anything slip at all, and below it their
    /// tells are much harder to catch. Together they mean the Midnight Mayor
    /// and Death's Taxi Driver — 0.6 and 0.7 deception — are genuinely hard
    /// to read, while Tommy Sullivan hides nothing, and that reading anyone
    /// gets easier as the night earns their trust.
    /// Whether the driver catches this tell.
    ///
    /// Public because the state machine needs it too: a tell raised by a
    /// passenger escalating deserves the same roll as one raised by a condition
    /// being met, and for a long time it was exempt and recorded as never
    /// noticed.
    pub fn notices_tell(
        rng: &mut macroquad_toolkit::rng::SeededRng,
        tell: &PassengerTell,
        passenger: &Passenger,
        player_trust: f32,
    ) -> bool {
        Self::calculate_detection_probability(rng, tell, passenger, player_trust)
    }

    fn calculate_detection_probability(
        rng: &mut macroquad_toolkit::rng::SeededRng,
        tell: &PassengerTell,
        passenger: &Passenger,
        player_trust: f32,
    ) -> bool {
        let base_prob = tell.reliability;

        let intensity_mult = match tell.intensity {
            TellIntensity::Subtle => 0.3,
            TellIntensity::Moderate => 0.7,
            TellIntensity::Obvious => 1.0,
        };

        // A passenger can guard their words, manner, and appearance — not
        // the frost on the windows. Environmental tells are the world
        // speaking, so deception and withheld trust cannot mute them; the
        // driver's own attentiveness still applies. Until this branch,
        // `tellType` was authored on every tell and decided nothing here.
        let (candour, guarded) = if tell.tell_type == TellType::Environmental {
            (1.0, 1.0)
        } else {
            (
                (1.0 - passenger.deception_level).clamp(0.0, 1.0),
                if player_trust < passenger.trust_required {
                    0.5
                } else {
                    1.0
                },
            )
        };

        let final_prob =
            base_prob * intensity_mult * candour * guarded * (0.5 + player_trust * 0.5);
        rng.next_f32() < final_prob
    }

    /// Evaluate a guideline choice
    pub fn evaluate_guideline_choice(
        guideline: &Guideline,
        action: GuidelineAction,
        passenger: &Passenger,
        state: &GameState,
    ) -> GuidelineEvaluationResult {
        // Find active exception
        let active_exception = Self::find_active_exception(
            guideline,
            passenger,
            &state.current_weather,
            &state.live_exceptions,
        );

        match (active_exception, action) {
            (Some(exc), GuidelineAction::Break) if exc.breaking_safer => {
                // Breaking was correct
                GuidelineEvaluationResult {
                    is_safe: true,
                    consequences: guideline.exception_rewards.clone(),
                    message: format!(
                        "Breaking \"{}\" was the right choice - {}",
                        guideline.title, exc.description
                    ),
                    satisfied_exception: Some(exc.id.clone()),
                    dormant: false,
                }
            }
            (Some(exc), GuidelineAction::Follow) if !exc.breaking_safer => {
                // Following was correct
                GuidelineEvaluationResult {
                    is_safe: true,
                    consequences: guideline.follow_consequences.clone(),
                    message: format!("Following \"{}\" was the right choice", guideline.title),
                    satisfied_exception: Some(exc.id.clone()),
                    dormant: false,
                }
            }
            (Some(_exc), _) => {
                // Wrong choice
                GuidelineEvaluationResult {
                    is_safe: false,
                    consequences: Self::calculate_negative_consequences(),
                    message: format!(
                        "Wrong choice regarding \"{}\" - misread the passenger",
                        guideline.title
                    ),
                    satisfied_exception: None,
                    dormant: false,
                }
            }
            (None, GuidelineAction::Follow) => {
                // No exception, following is safe
                GuidelineEvaluationResult {
                    is_safe: true,
                    consequences: guideline.follow_consequences.clone(),
                    message: format!("Following \"{}\" was the safe choice", guideline.title),
                    satisfied_exception: None,
                    dormant: false,
                }
            }
            (None, GuidelineAction::Break) => {
                // No *live* exception — but a break made on a real tell is
                // not a blind one. When the passenger's authored
                // breaking-safer exception matched everything except its
                // per-ride liveness roll, the read was right and the night
                // was merely quieter than it looked. Without this, every
                // dormant roll turned a correct read into a 0.7 death — the
                // trap that dominated baseline deaths once meltdowns
                // stopped masking it.
                if let Some(exc) = Self::find_dormant_exception(
                    guideline,
                    passenger,
                    &state.current_weather,
                    &state.live_exceptions,
                )
                .filter(|exc| exc.breaking_safer)
                {
                    return GuidelineEvaluationResult {
                        is_safe: true,
                        consequences: Vec::new(),
                        message: format!(
                            "Breaking \"{}\" answered a need that had already eased. \
                             No harm done tonight.",
                            guideline.title
                        ),
                        satisfied_exception: Some(exc.id.clone()),
                        dormant: true,
                    };
                }
                // No exception at all: breaking is dangerous.
                GuidelineEvaluationResult {
                    is_safe: false,
                    consequences: guideline.break_consequences.clone(),
                    message: format!("Breaking \"{}\" was dangerous", guideline.title),
                    satisfied_exception: None,
                    dormant: false,
                }
            }
        }
    }

    /// The exception on `guideline` that matches `passenger` and its
    /// conditions but lost tonight's liveness roll — authored, applicable,
    /// asleep.
    fn find_dormant_exception(
        guideline: &Guideline,
        passenger: &Passenger,
        weather: &WeatherCondition,
        live_exceptions: &HashSet<String>,
    ) -> Option<GuidelineException> {
        guideline
            .exceptions
            .iter()
            .find(|exception| {
                Self::passenger_matches_exception(passenger, exception)
                    && !live_exceptions.contains(&exception.id)
                    && Self::check_exception_conditions(exception, weather, passenger)
            })
            .cloned()
    }

    /// Find active exception for passenger
    /// The exception on `guideline` that currently applies to `passenger`, if
    /// any — the same check `evaluate_guideline_choice` judges the player's
    /// decision by.
    ///
    /// This was private, so nothing outside could ask the question the game
    /// was about to answer. The playtest bot guessed from whether a detected
    /// tell mentioned a `breakingSafer` exception, ignoring the conditions
    /// that decide whether it is live, and broke guidelines that were not
    /// excepted — eight of twenty shifts with a full skill tree ended on
    /// "Breaking X was dangerous". A decider and a judge disagreeing about
    /// the same question is the same fault as the route quote.
    pub fn find_active_exception(
        guideline: &Guideline,
        passenger: &Passenger,
        weather: &WeatherCondition,
        live_exceptions: &HashSet<String>,
    ) -> Option<GuidelineException> {
        for exception in &guideline.exceptions {
            if Self::passenger_matches_exception(passenger, exception)
                && live_exceptions.contains(&exception.id)
                && Self::check_exception_conditions(exception, weather, passenger)
            {
                return Some(exception.clone());
            }
        }
        None
    }

    /// Calculate negative consequences for wrong choice
    fn calculate_negative_consequences() -> Vec<Consequence> {
        vec![
            Consequence {
                consequence_type: ConsequenceType::Death,
                value: 1,
                // Empty on purpose: the evaluation message already carries
                // this verdict, and the death description is appended to it
                // — a copy here printed the same sentence twice.
                description: String::new(),
                probability: 0.7,
                item: None,
            },
            Consequence {
                consequence_type: ConsequenceType::Reputation,
                value: -20,
                description: "Lost passenger trust through incorrect reading".to_string(),
                probability: 0.9,
                item: None,
            },
        ]
    }

    /// Check if false tells should be introduced.
    ///
    /// Seasoning is a lifetime quantity: the per-shift ride counter this
    /// used to read caps around twelve on a full clock, against a threshold
    /// of twenty, so the gate never opened. Accuracy is this shift's
    /// decision record — the driver worth deceiving is the one currently
    /// reading passengers well, and three decisions is the least a ratio
    /// can be trusted on.
    pub fn should_introduce_false_tells(
        rng: &mut macroquad_toolkit::rng::SeededRng,
        decision_history: &[GuidelineDecision],
        stats: &PlayerStats,
    ) -> bool {
        let decisions = decision_history.len();
        if decisions < 3 {
            return false;
        }
        let correct = decision_history.iter().filter(|d| d.was_correct).count();
        let accuracy = correct as f32 / decisions as f32;

        let seasoned_rides = stats.total_rides_completed;
        if seasoned_rides > 35 && accuracy > 0.6 {
            return rng.next_f32() < 0.5;
        }
        if seasoned_rides > 20 && accuracy > 0.7 {
            return rng.next_f32() < 0.3;
        }

        false
    }
}

//! Passenger state machine for need level progression.

use crate::data::*;
use crate::engine::{GuidelineEngine, RuleEvaluationResult};
use crate::state::*;

/// Triggered tell with context
#[derive(Debug, Clone)]
pub struct TriggeredTell {
    pub tell: PassengerTell,
    pub exception_id: Option<String>,
    pub related_guideline_id: Option<u32>,
}

/// Passenger state machine for tracking need levels
pub struct PassengerStateMachine;

impl PassengerStateMachine {
    /// Initialize need state from passenger profile
    pub fn initialize(passenger: &Passenger, current_time: f64) -> Option<PassengerNeedState> {
        PassengerNeedState::from_passenger(passenger, current_time)
    }

    /// Apply route choice effects to need state.
    ///
    /// `obey_pressure` is how much this leg costs the passenger for the rule
    /// they need broken staying unbroken. When tonight's rules include the one
    /// their exception belongs to, it is that rule's authored
    /// `followNeedAdjustment`; otherwise the passenger's own flat `obey`
    /// stands in. Before this, tonight's rule set had no bearing at all on how
    /// fast a passenger degraded — thirteen rules author a follow cost between
    /// 3 and 12 and none of them was read, so a shift built from gentle rules
    /// and one built from punishing ones wore a passenger down identically.
    ///
    /// The `rule_outcome` parameter this used to take was passed `None` by its
    /// only caller, with a comment saying so. `collect_tells` already falls
    /// back to the profile's own exception in that case, so the argument only
    /// ever selected the fallback.
    pub fn apply_route_choice(
        state: &mut PassengerNeedState,
        passenger: &Passenger,
        route: RouteType,
        obey_pressure: Option<i32>,
        route_preference_stress_scale: f32,
        current_time: f64,
    ) -> Vec<TriggeredTell> {
        let profile = &state.profile;
        let previous_stage = state.stage;

        // Apply passive need change
        let mut level = state.level as i32 + profile.need_change.passive;

        // Route-specific changes
        if route == RouteType::Shortcut {
            level += profile.need_change.break_rule;
        } else {
            level += obey_pressure.unwrap_or(profile.need_change.obey);
        }

        if let Some(preference) = passenger.get_route_preference(route) {
            level += (preference.stress_modifier * route_preference_stress_scale).round() as i32;
        }

        Self::set_level_and_collect_tells(
            state,
            passenger,
            level,
            previous_stage,
            None,
            current_time,
        )
    }

    /// Apply a rule outcome directly to the passenger need state.
    pub fn apply_rule_outcome(
        state: &mut PassengerNeedState,
        passenger: &Passenger,
        rule_outcome: &RuleEvaluationResult,
        current_time: f64,
    ) -> Vec<TriggeredTell> {
        if rule_outcome.need_adjustment == 0 {
            state.last_updated = current_time;
            return Vec::new();
        }

        let previous_stage = state.stage;
        let level = state.level as i32 + rule_outcome.need_adjustment;
        Self::set_level_and_collect_tells(
            state,
            passenger,
            level,
            previous_stage,
            Some(rule_outcome),
            current_time,
        )
    }

    /// Apply a raw stress delta and keep stage/stability in sync.
    pub fn apply_stress_delta(
        state: &mut PassengerNeedState,
        passenger: &Passenger,
        delta: i32,
        current_time: f64,
    ) -> Vec<TriggeredTell> {
        if delta == 0 {
            state.last_updated = current_time;
            return Vec::new();
        }

        let previous_stage = state.stage;
        let level = state.level as i32 + delta;
        Self::set_level_and_collect_tells(
            state,
            passenger,
            level,
            previous_stage,
            None,
            current_time,
        )
    }

    fn set_level_and_collect_tells(
        state: &mut PassengerNeedState,
        passenger: &Passenger,
        level: i32,
        previous_stage: NeedStage,
        rule_outcome: Option<&RuleEvaluationResult>,
        current_time: f64,
    ) -> Vec<TriggeredTell> {
        state.level = level.clamp(0, 100) as u32;
        let new_stage = PassengerNeedState::calculate_stage(state.level, &state.profile.thresholds);

        let triggered_tells =
            if new_stage != previous_stage && !state.revealed_stages.contains_key(&new_stage) {
                state.revealed_stages.insert(new_stage, true);
                // The mask slipping cuts both ways. Every profile authors a
                // `trustImpact` per stage — Mrs. Chen's is +0.05 at warning
                // and +0.1 at critical — and nothing read it, so a passenger
                // visibly coming apart changed nothing about how well the
                // driver could read them. `player_trust` scales tell
                // detection and gates it against `trustRequired`, so this is
                // the loop that makes watching a passenger fray pay off.
                state.pending_trust += state
                    .profile
                    .trust_impact
                    .as_ref()
                    .and_then(|impact| impact.get(new_stage.key()))
                    .copied()
                    .unwrap_or(0.0);
                Self::collect_tells(
                    passenger,
                    new_stage,
                    rule_outcome,
                    state.profile.exception_id.as_deref(),
                    state.profile.tell_intensities.as_ref(),
                )
            } else {
                Vec::new()
            };

        state.stage = new_stage;
        state.stability = 1.0 - (state.level as f32 / 100.0);
        state.last_updated = current_time;

        triggered_tells
    }

    /// Collect tells appropriate for the current stage
    ///
    /// `profile_exception` is the passenger's own `stateProfile.exceptionId`.
    /// A tell raised by the need rising is *about* that exception, so it is
    /// used whenever the rule outcome does not name one; without the fallback
    /// a need-driven tell carries no exception and no guideline, and
    /// `check_guideline_triggers` can never match it.
    fn collect_tells(
        passenger: &Passenger,
        stage: NeedStage,
        rule_outcome: Option<&RuleEvaluationResult>,
        profile_exception: Option<&str>,
        authored: Option<&TellIntensityMap>,
    ) -> Vec<TriggeredTell> {
        let intensities = Self::stage_intensities(stage, authored);

        passenger
            .tells
            .iter()
            .filter(|tell| intensities.contains(&tell.intensity))
            .map(|tell| TriggeredTell {
                tell: tell.clone(),
                exception_id: rule_outcome
                    .and_then(|r| r.triggered_exception.as_ref().map(|e| e.id.clone()))
                    .or_else(|| profile_exception.map(str::to_string)),
                related_guideline_id: rule_outcome
                    .and_then(|r| r.rule.as_ref().and_then(|ru| ru.related_guideline_id)),
            })
            .collect()
    }

    /// Which tell intensities show at a stage.
    ///
    /// A passenger's `stateProfile.tellIntensities` overrides the defaults for
    /// the stages it names, so one passenger can start giving obvious signs at
    /// Warning while another stays subtle right up to Critical. Every profile
    /// authors this map and it was read by nothing, so every passenger on the
    /// roster tipped their hand at exactly the same point.
    pub fn stage_intensities(
        stage: NeedStage,
        authored: Option<&TellIntensityMap>,
    ) -> Vec<TellIntensity> {
        if let Some(map) = authored {
            if let Some(names) = map.get(stage.key()) {
                let parsed: Vec<TellIntensity> = names
                    .iter()
                    .filter_map(|n| Self::parse_intensity(n))
                    .collect();
                if !parsed.is_empty() {
                    return parsed;
                }
            }
        }
        match stage {
            NeedStage::Calm => vec![TellIntensity::Subtle],
            NeedStage::Warning => vec![TellIntensity::Moderate],
            NeedStage::Critical => vec![TellIntensity::Obvious],
            NeedStage::Meltdown => vec![TellIntensity::Obvious],
        }
    }

    /// Parse an authored intensity name, ignoring anything unrecognised so a
    /// typo degrades to the default rather than silencing the passenger.
    pub fn parse_intensity(name: &str) -> Option<TellIntensity> {
        match name.to_lowercase().as_str() {
            "subtle" => Some(TellIntensity::Subtle),
            "moderate" => Some(TellIntensity::Moderate),
            "obvious" => Some(TellIntensity::Obvious),
            _ => None,
        }
    }

    /// The line a passenger actually speaks when a verbal tell fires.
    ///
    /// `PassengerTell` authors a `tellType` and, for verbal ones, a
    /// `triggerPhrase` — the words the player is supposed to catch. Both were
    /// read by nothing, so every tell surfaced only as a description on the
    /// guideline screen and a verbal tell sounded exactly like a behavioural
    /// one. A spoken tell now takes precedence over the generic stage line,
    /// because it is the more specific signal.
    pub fn spoken_tell(triggered: &[TriggeredTell]) -> Option<String> {
        triggered
            .iter()
            .find(|t| t.tell.tell_type == TellType::Verbal && t.tell.trigger_phrase.is_some())
            .and_then(|t| t.tell.trigger_phrase.clone())
            .map(|phrase| format!("\"{}...\"", phrase))
    }

    /// Get stage-specific dialogue
    pub fn get_dialogue_for_stage(
        rng: &mut macroquad_toolkit::rng::SeededRng,
        _passenger: &Passenger,
        state: &PassengerNeedState,
    ) -> Option<String> {
        let stage_key = state.stage.key();

        state
            .profile
            .dialogue_by_stage
            .as_ref()
            .and_then(|map| map.get(stage_key))
            .and_then(|lines| rng.choose(lines).cloned())
    }

    /// Merge triggered tells into detected tells list.
    ///
    /// A tell that names an exception but no guideline is resolved against
    /// `guidelines` to find the one that owns that exception. The guideline
    /// decision only triggers on tells that carry a `related_guideline`, so
    /// this is what connects a rising need to the decision it should provoke.
    /// `player_trust` decides whether the driver actually catches each of these.
    ///
    /// Every tell raised here used to be recorded `player_noticed: false`,
    /// unconditionally. The guideline screen labels such a tell "uncertain" and
    /// the playtest bot filters it out entirely, so the tells a passenger gives
    /// off as they come apart -- the ones `tellIntensities` authors per stage,
    /// and the whole point of watching someone fray -- could never be noticed by
    /// anybody. The detection roll reached only the condition-based tells from
    /// `analyze_passenger`.
    ///
    /// It also left the trust loop unable to pay: trust rises as a passenger
    /// escalates, and better trust improves detection, but the tells escalation
    /// raised were exempt from detection altogether.
    pub fn merge_detected_tells(
        rng: &mut macroquad_toolkit::rng::SeededRng,
        existing: &mut Vec<DetectedTell>,
        triggered: Vec<TriggeredTell>,
        passenger: &Passenger,
        player_trust: f32,
        current_time: f64,
        guidelines: &[Guideline],
    ) {
        let passenger_id = passenger.id;
        for trigger in triggered {
            let related_guideline = trigger.related_guideline_id.or_else(|| {
                let exception_id = trigger.exception_id.as_deref()?;
                guidelines
                    .iter()
                    .find(|g| g.exceptions.iter().any(|e| e.id == exception_id))
                    .map(|g| g.id)
            });
            let noticed =
                GuidelineEngine::notices_tell(rng, &trigger.tell, passenger, player_trust);
            existing.push(DetectedTell {
                tell: trigger.tell,
                passenger_id,
                detection_time: current_time,
                player_noticed: noticed,
                related_guideline,
                exception_id: trigger.exception_id,
            });
        }
    }

    /// Check if exception is active based on need state
    pub fn is_exception_active(state: &PassengerNeedState, exception_id: &str) -> bool {
        if let Some(ref profile_exception) = state.profile.exception_id {
            profile_exception == exception_id && state.stage >= NeedStage::Warning
        } else {
            false
        }
    }

    /// Check if passenger is in meltdown
    pub fn is_meltdown(state: &PassengerNeedState) -> bool {
        state.stage == NeedStage::Meltdown
    }

    /// Check if passenger needs immediate attention
    pub fn is_critical(state: &PassengerNeedState) -> bool {
        state.stage >= NeedStage::Critical
    }

    /// Get stability as percentage (0-100)
    pub fn get_stability_percent(state: &PassengerNeedState) -> u32 {
        (state.stability * 100.0) as u32
    }
}

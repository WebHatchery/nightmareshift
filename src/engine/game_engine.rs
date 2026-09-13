//! Core game engine logic.

use crate::data::*;
use crate::state::*;

/// Result of generating shift rules
#[derive(Debug, Clone)]
pub struct ShiftRules {
    pub visible_rules: Vec<Rule>,
    pub hidden_rules: Vec<Rule>,
    pub difficulty_level: u32,
}

/// Result of rule evaluation
#[derive(Debug, Clone, Default)]
pub struct RuleEvaluationResult {
    pub violation: bool,
    pub rule: Option<Rule>,
    pub message: Option<String>,
    pub need_adjustment: i32,
    pub triggered_exception: Option<GuidelineException>,
}

/// Aggregated gameplay modifiers derived from the player's unlocked skills.
///
/// Computed once from the skill definitions plus the player's unlocked list, so
/// every skill's effect actually reaches the systems it names.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkillModifiers {
    /// Multiplier on route fuel cost (< 1.0 is cheaper).
    pub fuel_cost_mult: f32,
    /// Flat bonus to maximum fuel capacity.
    pub max_fuel_bonus: f32,
    /// Multiplier on fuel/time/risk added by environmental hazards (< 1.0 softer).
    pub hazard_mult: f32,
    /// Per-shift chance to reveal one hidden rule up front.
    pub reveal_hidden_chance: f32,
    /// Multiplier on every fare earned (> 1.0 pays more).
    pub fare_mult: f32,
    /// Multiplier on refuel cost (< 1.0 is a discount).
    pub refuel_cost_mult: f32,
    /// Protective wards granted at the start of each shift.
    pub bonus_protection: u32,
    /// Need soothed when playing music with no rule in force (stereo tier).
    pub soothe_music: i32,
    /// Need soothed by the AC or a cracked window (climate tier).
    pub soothe_climate: i32,
    /// Need soothed by eye contact or pulling over calmly (cabin comfort tier).
    pub soothe_presence: i32,
}

impl Default for SkillModifiers {
    fn default() -> Self {
        Self {
            fuel_cost_mult: 1.0,
            max_fuel_bonus: 0.0,
            hazard_mult: 1.0,
            reveal_hidden_chance: 0.0,
            fare_mult: 1.0,
            refuel_cost_mult: 1.0,
            bonus_protection: 0,
            soothe_music: 0,
            soothe_climate: 0,
            soothe_presence: 0,
        }
    }
}

impl SkillModifiers {
    /// Build the active modifiers from the skill catalog and the player's
    /// unlocked-skill list. Unknown targets are ignored.
    pub fn from_unlocked(skills: &[Skill], unlocked: &[String]) -> Self {
        let mut m = Self::default();
        for skill in skills {
            if !unlocked.contains(&skill.id) {
                continue;
            }
            let v = skill.effect.value as f32;
            match skill.effect.target.as_str() {
                "fuel_consumption" => m.fuel_cost_mult *= v,
                "max_fuel" => m.max_fuel_bonus += v,
                "hazard_damage" => m.hazard_mult *= v,
                "reveal_hidden_chance" => m.reveal_hidden_chance += v,
                "fare_multiplier" => m.fare_mult *= v,
                "refuel_discount" => m.refuel_cost_mult *= (1.0 - v).max(0.0),
                "supernatural_protection" => m.bonus_protection += v as u32,
                "soothe_music" => m.soothe_music += v as i32,
                "soothe_climate" => m.soothe_climate += v as i32,
                "soothe_presence" => m.soothe_presence += v as i32,
                _ => {}
            }
        }
        m
    }
}

/// Core game engine for rule generation and violation checking
pub struct GameEngine;

impl GameEngine {
    /// Generate shift rules based on player experience
    pub fn generate_shift_rules(
        rng: &mut macroquad_toolkit::rng::SeededRng,
        experience: u32,
        all_rules: &[Rule],
        constants: &ConstantsData,
    ) -> ShiftRules {
        let difficulty_level = (experience / constants.scoring.experience_per_level)
            .min(constants.scoring.max_difficulty);

        // Separate rules by type. `Weather` rules are deliberately absent:
        // they are injected by the weather sync in `Game`, not drawn here.
        let pool_of = |kind: RuleType| -> Vec<&Rule> {
            all_rules.iter().filter(|r| r.rule_type == kind).collect()
        };

        let mut selected_rules: Vec<Rule> = Vec::new();

        // Select 2-3 basic rules
        let num_basic = 2 + rng.range_i32(0, 2) as u32;
        Self::draw_compatible_rules(
            rng,
            &pool_of(RuleType::Basic),
            num_basic,
            &mut selected_rules,
        );

        // Add conditional rules based on difficulty
        if difficulty_level >= 1 {
            let num_conditional = 1 + rng.range_i32(0, 2) as u32;
            Self::draw_compatible_rules(
                rng,
                &pool_of(RuleType::Conditional),
                num_conditional,
                &mut selected_rules,
            );
        }

        // `Conflicting` and `Hidden` rules were referenced by no selection
        // path at all, so twelve of the twenty-eight authored rules could
        // never appear — including all three `Hidden` ones, which are the
        // whole point of `reveal_hidden_rule`, the Glimpse skill's
        // `reveal_hidden_chance`, and the "Hidden Rule Violated!" ending.
        // Both are expert-tier content, so they join from difficulty 2.
        if difficulty_level >= 2 {
            Self::draw_compatible_rules(
                rng,
                &pool_of(RuleType::Conflicting),
                1,
                &mut selected_rules,
            );
            Self::draw_compatible_rules(rng, &pool_of(RuleType::Hidden), 1, &mut selected_rules);
        }

        // Separate visible and hidden rules
        let visible_rules: Vec<Rule> = selected_rules
            .iter()
            .filter(|r| r.visible)
            .cloned()
            .collect();
        let hidden_rules: Vec<Rule> = selected_rules
            .iter()
            .filter(|r| !r.visible)
            .cloned()
            .collect();

        ShiftRules {
            visible_rules,
            hidden_rules,
            difficulty_level,
        }
    }

    /// Draw up to `count` rules from `pool` that do not contradict anything
    /// already selected.
    ///
    /// `conflictsWith` is authored on three rules and was read by nothing, so
    /// a shift could hand the player both "Make eye contact with passengers
    /// to ensure they're alert" and "Do not look directly at passengers
    /// tonight" — a night that cannot be driven cleanly no matter what the
    /// player does. Fewer rules is the right failure here: a shift short one
    /// rule is playable, a self-contradictory one is not.
    fn draw_compatible_rules(
        rng: &mut macroquad_toolkit::rng::SeededRng,
        pool: &[&Rule],
        count: u32,
        selected: &mut Vec<Rule>,
    ) {
        let mut shuffled: Vec<&Rule> = pool.to_vec();
        rng.shuffle(&mut shuffled);

        let mut taken = 0;
        for rule in shuffled {
            if taken >= count {
                break;
            }
            if selected
                .iter()
                .any(|chosen| Self::rules_conflict(chosen, rule))
            {
                continue;
            }
            selected.push(rule.clone());
            taken += 1;
        }
    }

    /// Whether two rules contradict each other. Links are authored one-way —
    /// "Safety First" names "No Eye Contact" but not the reverse — so both
    /// directions are checked.
    pub fn rules_conflict(a: &Rule, b: &Rule) -> bool {
        a.id == b.id || a.conflicts_with.contains(&b.id) || b.conflicts_with.contains(&a.id)
    }

    /// Check if an action violates any active rule
    pub fn check_rule_violation(
        rules: &[Rule],
        action: &str,
        need_state: Option<&PassengerNeedState>,
        guidelines: &[Guideline],
    ) -> RuleEvaluationResult {
        for rule in rules {
            if rule.forbids_action(action) {
                // A need state only exists while a passenger is aboard, so it
                // carries the "is anyone here" check the passenger argument
                // used to make.
                if Self::passenger_has_exception(rule, need_state, guidelines) {
                    return RuleEvaluationResult {
                        violation: false,
                        rule: Some(rule.clone()),
                        message: Some("Exception applies - action is safe".to_string()),
                        need_adjustment: rule.exception_need_adjustment.unwrap_or(0),
                        triggered_exception: None,
                    };
                }

                return RuleEvaluationResult {
                    violation: true,
                    rule: Some(rule.clone()),
                    message: Some(rule.get_violation_message().to_string()),
                    need_adjustment: rule.break_need_adjustment.unwrap_or(0),
                    triggered_exception: None,
                };
            }
        }

        RuleEvaluationResult::default()
    }

    /// Check if a route choice violates weather-triggered driving rules.
    pub fn check_weather_route_violation(
        rules: &[Rule],
        route: RouteType,
        weather: &WeatherCondition,
        time_of_day: &TimeOfDay,
    ) -> RuleEvaluationResult {
        for rule in rules.iter().filter(|r| r.rule_type == RuleType::Weather) {
            if Self::weather_route_triggers(rule, route, weather, time_of_day) {
                return RuleEvaluationResult {
                    violation: true,
                    rule: Some(rule.clone()),
                    message: Some(rule.get_violation_message().to_string()),
                    need_adjustment: rule.break_need_adjustment.unwrap_or(10),
                    triggered_exception: None,
                };
            }
        }

        RuleEvaluationResult::default()
    }

    fn weather_route_triggers(
        rule: &Rule,
        route: RouteType,
        weather: &WeatherCondition,
        time_of_day: &TimeOfDay,
    ) -> bool {
        match rule.trigger.as_deref() {
            Some("heavy_fog") => {
                weather.weather_type == WeatherType::Fog
                    && weather.intensity == WeatherIntensity::Heavy
                    && route == RouteType::Shortcut
            }
            Some("snow") => {
                weather.weather_type == WeatherType::Snow && route == RouteType::Shortcut
            }
            Some("latenight_badweather") => {
                time_of_day.phase == TimePhase::Latenight
                    && weather.weather_type != WeatherType::Clear
                    && route == RouteType::Scenic
            }
            Some("low_visibility") => weather.visibility < 30 && route == RouteType::Shortcut,
            _ => false,
        }
    }

    /// Check if passenger has an exception to a rule
    /// Whether breaking `rule` is what the current passenger actually needs.
    ///
    /// The passenger is identified entirely by their `need_state`, whose
    /// profile carries the `exceptionId`; the passenger itself became an
    /// unused parameter once `rule_modification` stopped short-circuiting
    /// here, so it is gone rather than underscored.
    ///
    /// The rule argument used to be `_rule` — ignored entirely — so any
    /// forbidden cab action relieved any stressed passenger by that rule's
    /// `exceptionNeedAdjustment`. Mrs. Chen, who wants to be looked at, was
    /// soothed just as well by opening a window, and the `exceptionId` every
    /// profile authors decided nothing. The rule must now belong to the
    /// guideline that owns the passenger's own exception.
    pub fn passenger_has_exception(
        rule: &Rule,
        need_state: Option<&PassengerNeedState>,
        guidelines: &[Guideline],
    ) -> bool {
        // `rule_modification` used to short-circuit to true here, which
        // excused The Collector, Madame Zelda and the Midnight Mayor from
        // every rule for the whole ride. What they can do is rewrite the
        // night's rules once as they get in — `RuleModificationService` — not
        // ignore the ones that remain.

        let Some(state) = need_state else {
            return false;
        };
        let Some(exception_id) = state.profile.exception_id.as_deref() else {
            return false;
        };

        // The passenger's own exception, on the guideline this rule belongs
        // to, and only once they are visibly far enough gone to justify it.
        guidelines
            .iter()
            .filter(|guideline| rule.related_guideline_id == Some(guideline.id))
            .flat_map(|guideline| guideline.exceptions.iter())
            .filter(|exception| exception.id == exception_id)
            .any(|exception| state.stage >= Self::required_stage(exception))
    }

    /// Whether a hidden-rule violation actually lands.
    ///
    /// `PROBABILITIES.HIDDEN_RULE_VIOLATION` is authored at 0.3 and was read
    /// by nothing, so breaking a rule the player had no way to know about
    /// caught them every single time. Every sibling in that block --
    /// supernatural encounters, high-risk encounters, item drops, kin spawns,
    /// trade offers -- gates whether an event fires, so this one reads the
    /// same way: a hidden rule is a lurking thing that mostly does not notice,
    /// not a guaranteed sentence on a rule nobody could have read.
    ///
    /// A miss reveals nothing. Getting away with it has to teach the player
    /// nothing, or the rule would out itself the first time and the Glimpse
    /// skill -- which buys exactly that knowledge -- would have nothing left
    /// to sell.
    pub fn hidden_violation_lands(
        rng: &mut macroquad_toolkit::rng::SeededRng,
        constants: &ConstantsData,
    ) -> bool {
        rng.chance(constants.probabilities.hidden_rule_violation)
    }

    /// The rule in force tonight that this passenger's exception belongs to.
    ///
    /// This is the rule they need broken. Whether it is among tonight's rules
    /// is the single most useful thing a player can know about a passenger in
    /// advance — it is what the almanac's "Relief" line names — and until now
    /// it changed only whether an exception could be claimed, never how hard
    /// the ride was to hold together.
    pub fn passengers_rule_in_force<'a>(
        rules: &'a [Rule],
        need_state: Option<&PassengerNeedState>,
        guidelines: &[Guideline],
    ) -> Option<&'a Rule> {
        let exception_id = need_state?.profile.exception_id.as_deref()?;
        let owning: Vec<u32> = guidelines
            .iter()
            .filter(|guideline| {
                guideline
                    .exceptions
                    .iter()
                    .any(|exception| exception.id == exception_id)
            })
            .map(|guideline| guideline.id)
            .collect();
        rules.iter().find(|rule| {
            rule.related_guideline_id
                .is_some_and(|id| owning.contains(&id))
        })
    }

    /// The need stage an exception becomes available at.
    ///
    /// Every exception in `guidelineData.json` authors `requiredStage`, all
    /// nineteen of them saying "warning" — and the gate was a hardcoded
    /// `NeedStage::Warning` that never read the field. The two agreed, so
    /// nothing was visibly wrong; authoring "critical" for an exception that
    /// should cost more to earn would simply have been ignored.
    ///
    /// An absent or unrecognised name falls back to Warning, which is what the
    /// code did before and what every exception asks for. A typo therefore
    /// keeps the exception reachable rather than quietly stranding a passenger
    /// with no way to be soothed.
    ///
    /// Public because the almanac dossier quotes it: the stage a passenger's
    /// relief unlocks at is exactly the sort of thing studying them should
    /// buy, and it must be the number the engine gates on, not a second copy
    /// of the same rule.
    pub fn required_stage(exception: &GuidelineException) -> NeedStage {
        exception
            .required_stage
            .as_deref()
            .and_then(NeedStage::parse)
            .unwrap_or(NeedStage::Warning)
    }

    /// Calculate fare with all modifiers
    /// How far either way the meter can land off the settled figure, and the
    /// floor no fare falls below.
    ///
    /// Named because two places need them: the payout rolls inside this band and
    /// the ride offer has to quote a range wide enough to contain wherever it
    /// lands. A quote that excludes the roll is a quote the payout can fall
    /// outside of.
    pub const FARE_VARIATION: f32 = 5.0;
    pub const MINIMUM_FARE: f32 = 5.0;

    /// The fare before the meter's own wobble -- everything that is decided
    /// rather than rolled.
    ///
    /// Split out because `calculate_fare` cannot be asked what a road pays: it
    /// rolls each time it is called, so four calls are four samples rather than
    /// four roads. Calling it per route from a draw function would have made the
    /// number on screen flicker every frame.
    #[allow(clippy::too_many_arguments)]
    pub fn fare_before_variation(
        base_fare: u32,
        route: RouteType,
        passenger: &Passenger,
        consecutive_streak: Option<&RouteStreak>,
        reputation: Option<&PassengerReputation>,
        constants: &ConstantsData,
        destination_fare_modifier: f32,
    ) -> f32 {
        let route_mult = match route {
            RouteType::Shortcut => constants.route_fares.shortcut,
            RouteType::Normal => constants.route_fares.normal,
            RouteType::Scenic => constants.route_fares.scenic,
            RouteType::Police => constants.route_fares.police,
        };

        let pref_mult = passenger
            .get_route_preference(route)
            .map(|p| p.fare_modifier)
            .unwrap_or(1.0);

        let streak_mult = if let Some(streak) = consecutive_streak {
            if streak.route_type == route && streak.count >= 2 {
                1.0 - (streak.count - 1) as f32 * constants.consecutive_route.penalty_per_repeat
            } else {
                1.0
            }
        } else {
            1.0
        };

        let rep_mult = reputation
            .map(|r| r.fare_multiplier(&constants.reputation))
            .unwrap_or(1.0);

        base_fare as f32
            * route_mult
            * pref_mult
            * streak_mult
            * rep_mult
            * destination_fare_modifier
    }

    pub fn calculate_fare(
        rng: &mut macroquad_toolkit::rng::SeededRng,
        base_fare: u32,
        route: RouteType,
        passenger: &Passenger,
        consecutive_streak: Option<&RouteStreak>,
        reputation: Option<&PassengerReputation>,
        constants: &ConstantsData,
        destination_fare_modifier: f32,
    ) -> u32 {
        let fare = Self::fare_before_variation(
            base_fare,
            route,
            passenger,
            consecutive_streak,
            reputation,
            constants,
            destination_fare_modifier,
        );

        let variation = rng.range_f32(-Self::FARE_VARIATION, Self::FARE_VARIATION);

        (fare + variation).max(Self::MINIMUM_FARE) as u32
    }
}

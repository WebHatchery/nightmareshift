//! Game constants loaded from JSON.
//!
//! All game balance values are loaded from assets/constants.json.
//! Do not hardcode values - add them to the JSON file instead.

use serde::{Deserialize, Serialize};

fn default_nights_per_run() -> u32 {
    5
}

fn default_guideline_decision_seconds() -> f32 {
    30.0
}

/// Core game constants
// `Default` exists solely so a `GameState` can be constructed when the
// data failed to load — the loading screen then refuses to advance, so a
// zeroed shift is never actually played.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GameConstants {
    #[serde(rename = "INITIAL_FUEL")]
    pub initial_fuel: u32,
    #[serde(rename = "INITIAL_TIME")]
    pub initial_time: u32,
    /// Duration of the guideline decision window in simulation seconds.
    #[serde(
        rename = "GUIDELINE_DECISION_SECONDS",
        default = "default_guideline_decision_seconds"
    )]
    pub guideline_decision_seconds: f32,
    /// Number of consecutive nights that make up one full run.
    #[serde(rename = "NIGHTS_PER_RUN", default = "default_nights_per_run")]
    pub nights_per_run: u32,
    /// Share of the base quota added per night of a run. The escalation was a
    /// `/ 2` in `begin_night` — the single biggest lever on how a campaign
    /// feels, expressed as a magic number in Rust rather than authored here.
    #[serde(rename = "QUOTA_INCREASE_PER_NIGHT", default = "default_quota_step")]
    pub quota_increase_per_night: f32,
    /// Difficulty levels added per night, on top of lifetime experience.
    #[serde(
        rename = "DIFFICULTY_INCREASE_PER_NIGHT",
        default = "default_difficulty_step"
    )]
    pub difficulty_increase_per_night: u32,
    #[serde(rename = "SURVIVAL_BONUS")]
    pub survival_bonus: u32,
    #[serde(rename = "BACKSTORY_UNLOCK_FIRST")]
    pub backstory_unlock_first: f32,
    #[serde(rename = "BACKSTORY_UNLOCK_REPEAT")]
    pub backstory_unlock_repeat: f32,
    #[serde(rename = "MINIMUM_EARNINGS")]
    pub minimum_earnings: u32,
    #[serde(rename = "FUEL_COST_SHORTCUT")]
    pub fuel_cost_shortcut: u32,
    #[serde(rename = "FUEL_COST_NORMAL")]
    pub fuel_cost_normal: u32,
    #[serde(rename = "FUEL_COST_SCENIC")]
    pub fuel_cost_scenic: u32,
    #[serde(rename = "FUEL_COST_POLICE")]
    pub fuel_cost_police: u32,
    #[serde(rename = "TIME_COST_SHORTCUT")]
    pub time_cost_shortcut: u32,
    #[serde(rename = "TIME_COST_NORMAL")]
    pub time_cost_normal: u32,
    #[serde(rename = "TIME_COST_SCENIC")]
    pub time_cost_scenic: u32,
    #[serde(rename = "TIME_COST_POLICE")]
    pub time_cost_police: u32,
    #[serde(rename = "ROUTE_PREFERENCE_STRESS_SCALE")]
    pub route_preference_stress_scale: f32,
    /// Need relieved on each Normal leg: its identity is the predictable,
    /// steady road rather than speed, premium fare, or supernatural safety.
    #[serde(rename = "NORMAL_ROUTE_NEED_RELIEF", default = "default_normal_relief")]
    pub normal_route_need_relief: u32,
    #[serde(rename = "RISK_NORMAL")]
    pub risk_normal: u32,
    #[serde(rename = "RISK_SHORTCUT")]
    pub risk_shortcut: u32,
    #[serde(rename = "RISK_SCENIC")]
    pub risk_scenic: u32,
    #[serde(rename = "RISK_POLICE")]
    pub risk_police: u32,
}

/// Fuel system thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuelConstants {
    #[serde(rename = "LOW_FUEL_WARNING")]
    pub low_fuel_warning: u32,
    #[serde(rename = "CRITICAL_FUEL")]
    pub critical_fuel: u32,
    #[serde(rename = "MEDIUM_FUEL", default = "default_medium_fuel")]
    pub medium_fuel: u32,
    #[serde(rename = "EMPTY_TANK")]
    pub empty_tank: u32,
    #[serde(rename = "FUEL_CHECK_MINIMUM")]
    pub fuel_check_minimum: u32,
    #[serde(rename = "COST_PER_PERCENT")]
    pub cost_per_percent: f32,
    /// Fuel percentage dispensed by the partial refuel action.
    #[serde(
        rename = "PARTIAL_REFUEL_AMOUNT",
        default = "default_partial_refuel_amount"
    )]
    pub partial_refuel_amount: f32,
}

fn default_partial_refuel_amount() -> f32 {
    25.0
}

impl FuelConstants {
    /// What topping up `amount` percent costs a driver whose refuel discount
    /// is `cost_mult`.
    ///
    /// One formula, because there were two. The waiting screen priced a top-up
    /// without the discount from Negotiator and Haggler while `refuel_full`
    /// charged with it, so the button quoted more than the pump took. Worse,
    /// the screen decided whether the button was even usable from its own
    /// inflated figure -- so a driver holding $40 against a real price of $35
    /// was refused a refuel they could afford, by the very skill they had
    /// bought to make it cheaper.
    pub fn refuel_cost(&self, amount: f32, cost_mult: f32) -> u32 {
        (amount.max(0.0) * self.cost_per_percent * cost_mult) as u32
    }
}

/// Timing constants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingConstants {
    // The five *_DELAY_MS values authored beside these are not read. This
    // game is frame-driven: nothing waits on a millisecond clock, and the
    // pacing they describe belongs to the JavaScript version this was ported
    // from. Serde ignores them.
    #[serde(rename = "SHIFT_END_WARNING_THRESHOLD")]
    pub shift_end_warning_threshold: u32,
    #[serde(rename = "CRITICAL_TIME_THRESHOLD")]
    pub critical_time_threshold: u32,
}

/// Probability constants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbabilityConstants {
    #[serde(rename = "SUPERNATURAL_ENCOUNTER")]
    pub supernatural_encounter: f32,
    #[serde(rename = "HIGH_RISK_ENCOUNTER")]
    pub high_risk_encounter: f32,
    #[serde(rename = "ITEM_DROP")]
    pub item_drop: f32,
    #[serde(rename = "HIDDEN_RULE_VIOLATION")]
    pub hidden_rule_violation: f32,
    #[serde(rename = "RELATED_PASSENGER_SPAWN")]
    pub related_passenger_spawn: f32,
    #[serde(rename = "TRADE_OFFER_CHANCE")]
    pub trade_offer_chance: f32,
}

/// Risk level thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskConstants {
    #[serde(rename = "SAFE")]
    pub safe: u32,
    #[serde(rename = "LOW_RISK")]
    pub low_risk: u32,
    #[serde(rename = "MEDIUM_RISK")]
    pub medium_risk: u32,
    #[serde(rename = "HIGH_RISK")]
    pub high_risk: u32,
    #[serde(rename = "EXTREME_RISK")]
    pub extreme_risk: u32,
    #[serde(rename = "MAX_RISK_LEVEL")]
    pub max_risk_level: u32,
    #[serde(rename = "SUPERNATURAL_THRESHOLD")]
    pub supernatural_threshold: u32,
}

/// Reputation system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationConstants {
    #[serde(rename = "TRUSTED_RATIO")]
    pub trusted_ratio: f32,
    #[serde(rename = "FRIENDLY_RATIO")]
    pub friendly_ratio: f32,
    #[serde(rename = "HOSTILE_RATIO")]
    pub hostile_ratio: f32,
    #[serde(rename = "MINIMUM_INTERACTIONS_FOR_TRUSTED")]
    pub minimum_interactions_for_trusted: u32,
    #[serde(rename = "TRUSTED_FARE_MULT")]
    pub trusted_fare_mult: f32,
    #[serde(rename = "FRIENDLY_FARE_MULT")]
    pub friendly_fare_mult: f32,
    #[serde(rename = "HOSTILE_FARE_MULT")]
    pub hostile_fare_mult: f32,
    #[serde(rename = "DEFAULT_FARE_MULT")]
    pub default_fare_mult: f32,
    #[serde(rename = "TRUSTED_RISK_MOD")]
    pub trusted_risk_mod: i32,
    #[serde(rename = "HOSTILE_RISK_MOD")]
    pub hostile_risk_mod: i32,
}

/// Route fare multipliers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteFareConstants {
    #[serde(rename = "SHORTCUT")]
    pub shortcut: f32,
    #[serde(rename = "NORMAL")]
    pub normal: f32,
    #[serde(rename = "SCENIC")]
    pub scenic: f32,
    #[serde(rename = "POLICE")]
    pub police: f32,
}

/// Consecutive route penalties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsecutiveRouteConstants {
    #[serde(rename = "WARNING_THRESHOLD")]
    pub warning_threshold: u32,
    #[serde(rename = "PENALTY_PER_REPEAT")]
    pub penalty_per_repeat: f32,
    #[serde(rename = "RISK_INCREASE_PER_REPEAT")]
    pub risk_increase_per_repeat: u32,
}

/// Route variation bounds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteVariationConstants {
    #[serde(rename = "MINIMUM_FUEL_COST")]
    pub minimum_fuel_cost: u32,
    #[serde(rename = "MINIMUM_TIME_COST")]
    pub minimum_time_cost: u32,
}

/// Scoring multipliers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringConstants {
    #[serde(rename = "TIME_BONUS_MULTIPLIER")]
    pub time_bonus_multiplier: u32,
    #[serde(rename = "RIDE_BONUS")]
    pub ride_bonus: u32,
    #[serde(rename = "RULE_VIOLATION_PENALTY")]
    pub rule_violation_penalty: u32,
    #[serde(rename = "MAX_DIFFICULTY")]
    pub max_difficulty: u32,
    #[serde(rename = "EXPERIENCE_PER_LEVEL")]
    pub experience_per_level: u32,
    #[serde(rename = "SURVIVAL_BONUS")]
    pub survival_bonus: u32,
}

/// Rarity spawn weights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RarityWeights {
    pub common: u32,
    #[serde(default)]
    pub uncommon: u32,
    pub rare: u32,
    pub legendary: u32,
}

/// Root constants structure matching constants.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstantsData {
    #[serde(rename = "GAME_CONSTANTS")]
    pub game_constants: GameConstants,
    #[serde(rename = "FUEL")]
    pub fuel: FuelConstants,
    #[serde(rename = "TIMING")]
    pub timing: TimingConstants,
    #[serde(rename = "PROBABILITIES")]
    pub probabilities: ProbabilityConstants,
    #[serde(rename = "RISK")]
    pub risk: RiskConstants,
    #[serde(rename = "REPUTATION")]
    pub reputation: ReputationConstants,
    #[serde(rename = "ROUTE_FARES")]
    pub route_fares: RouteFareConstants,
    #[serde(rename = "CONSECUTIVE_ROUTE")]
    pub consecutive_route: ConsecutiveRouteConstants,
    #[serde(rename = "ROUTE_VARIATIONS")]
    pub route_variations: RouteVariationConstants,
    #[serde(rename = "SCORING")]
    pub scoring: ScoringConstants,
    #[serde(rename = "RARITY_WEIGHTS")]
    pub rarity_weights: RarityWeights,
}

// No default() implementation - all values MUST come from constants.json
// If JSON fails to parse, it's a development error that should fail compilation

fn default_medium_fuel() -> u32 {
    40
}

fn default_quota_step() -> f32 {
    0.5
}

fn default_difficulty_step() -> u32 {
    1
}

fn default_normal_relief() -> u32 {
    4
}

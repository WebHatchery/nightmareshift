//! Core game state structure.

mod relationship_level;

use crate::data::*;
use crate::state::{
    AudioEvent, FareContribution, MetaPayout, RideBaseline, RideCompletion, ShiftTelemetry,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// What is happening inside a shift.
///
/// This deliberately does not mirror `Screen`. It used to carry `MainMenu`,
/// `SkillTree`, `Almanac` and `Leaderboard` variants that existed only to
/// shadow the screen the player was on, kept in step by hand in
/// `change_screen` while five other places assigned `screen` directly and
/// could leave the two disagreeing. Nothing ever read them. Navigation is
/// `Screen`'s job; this describes the night.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum GamePhase {
    /// Before a shift has begun — boot, menus, and the meta screens.
    #[default]
    Loading,
    Briefing,
    Waiting,
    RideRequest,
    Driving,
    Interaction,
    GuidelineDecision,
    DropOff,
    GameOver,
    Success,
}

/// Driving sub-phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DrivingPhase {
    Pickup,
    Destination,
}

/// Need stage progression
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub enum NeedStage {
    #[default]
    Calm,
    Warning,
    Critical,
    Meltdown,
}

impl NeedStage {
    /// The JSON key a stage is authored under, in `tellIntensities`,
    /// `dialogueByStage` and an exception's `requiredStage`.
    pub fn key(self) -> &'static str {
        match self {
            Self::Calm => "calm",
            Self::Warning => "warning",
            Self::Critical => "critical",
            Self::Meltdown => "meltdown",
        }
    }

    /// How this stage reads to the driver.
    ///
    /// The one vocabulary for a passenger's condition. The almanac quotes need
    /// thresholds -- "restless past 61, critical at 80" -- while the driving
    /// screen shows stability, which is the inverse scale, so a driver was told
    /// to watch for 61 and shown 45%. Nothing was wrong with either number; they
    /// simply could not be compared. Naming the stage on the readout means the
    /// studied fact and the live gauge use the same words, and the arithmetic
    /// stops mattering.
    pub fn label(self) -> &'static str {
        match self {
            Self::Calm => "settled",
            Self::Warning => "restless",
            Self::Critical => "close to breaking",
            Self::Meltdown => "breaking down",
        }
    }

    /// Parse an authored stage name, or nothing if it names no stage.
    ///
    /// Returning `Option` rather than defaulting keeps the two directions
    /// honest: a caller decides for itself whether an unrecognised name is a
    /// typo to fall back from or a mistake to surface.
    pub fn parse(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "calm" => Some(Self::Calm),
            "warning" => Some(Self::Warning),
            "critical" => Some(Self::Critical),
            "meltdown" => Some(Self::Meltdown),
            _ => None,
        }
    }
}

/// Relationship level with a passenger
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum RelationshipLevel {
    Hostile,
    #[default]
    Neutral,
    Friendly,
    Trusted,
}
/// Current ride information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentRide {
    pub passenger: Passenger,
    pub pickup_location: String,
    pub destination_location: String,
    pub route_type: Option<RouteType>,
    pub driving_phase: DrivingPhase,
    pub start_time: f64,
}

/// Passenger need state tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassengerNeedState {
    pub level: u32,
    pub stage: NeedStage,
    pub stability: f32,
    pub last_updated: f64,
    pub revealed_stages: HashMap<NeedStage, bool>,
    pub profile: PassengerStateProfile,
    /// Trust this passenger's escalation has earned or cost the driver, not
    /// yet folded into `GameState::player_trust`.
    ///
    /// Every profile authors a `trustImpact` per stage and nothing read it.
    /// It is accumulated here rather than applied in place because the state
    /// machine is handed a need state, not the whole game: six different
    /// systems push a passenger's need around and none of them should have to
    /// remember to settle the driver's trust afterwards.
    pub pending_trust: f32,
    /// The passenger has already been held at the brink of meltdown once.
    ///
    /// Meltdown was the one death in the game with no roll and no warning: the
    /// stage flipped and the shift ended on the same leg. The first crossing
    /// now pulls the passenger back to just under the threshold and burns this
    /// flag; a second crossing is the real thing. One leg of grace, once.
    pub brink_reached: bool,
}

impl PassengerNeedState {
    /// Create from passenger's state profile
    pub fn from_passenger(passenger: &Passenger, current_time: f64) -> Option<Self> {
        let profile = passenger.state_profile.clone()?;
        let stage = Self::calculate_stage(profile.initial_level, &profile.thresholds);

        Some(Self {
            level: profile.initial_level.clamp(0, 100),
            stage,
            stability: 1.0 - (profile.initial_level as f32 / 100.0),
            last_updated: current_time,
            revealed_stages: HashMap::new(),
            profile,
            pending_trust: 0.0,
            brink_reached: false,
        })
    }

    /// Take the trust this passenger's escalation has moved, leaving none.
    pub fn take_pending_trust(&mut self) -> f32 {
        std::mem::replace(&mut self.pending_trust, 0.0)
    }

    /// Calculate stage from level and thresholds
    pub fn calculate_stage(level: u32, thresholds: &NeedThresholds) -> NeedStage {
        if level >= thresholds.meltdown {
            NeedStage::Meltdown
        } else if level >= thresholds.critical {
            NeedStage::Critical
        } else if level >= thresholds.warning {
            NeedStage::Warning
        } else {
            NeedStage::Calm
        }
    }
}

/// A detected behavioral tell
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedTell {
    pub tell: PassengerTell,
    pub passenger_id: u32,
    pub detection_time: f64,
    pub player_noticed: bool,
    pub related_guideline: Option<u32>,
    pub exception_id: Option<String>,
}

/// Passenger reputation tracking.
///
/// Serialized into the save via `PlayerStats`, so a standing outlives the
/// run that earned it. Two fields were shed on the way into the save:
/// `negative_choices` was always `interactions - positive_choices` (both
/// update paths bumped them in lockstep), and `last_encounter` was a
/// wall-clock-relative timestamp that would read as nonsense next session.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PassengerReputation {
    pub interactions: u32,
    pub positive_choices: u32,
    #[serde(default)]
    pub relationship_level: RelationshipLevel,
}

impl PassengerReputation {
    /// Update reputation based on choice outcome
    pub fn update(&mut self, positive: bool, constants: &ReputationConstants) {
        self.interactions += 1;
        if positive {
            self.positive_choices += 1;
        }
        self.settle(constants);
    }

    /// Record a deliberate act toward this passenger and settle the level.
    ///
    /// `update` ran once, at ride completion, and was the only path that
    /// recomputed `relationship_level` — which is the only thing anything
    /// reads a reputation for. The fare multiplier and the route risk
    /// modifier both branch on it and nothing else. Four other places raised
    /// the tallies directly: using a reputation item, both of the rule
    /// consequence sites, and the wanted-item trade bonus. All four left the
    /// level where it was, so giving a passenger the withered flowers changed
    /// no number the player could see for the rest of the ride.
    ///
    /// They also raised `positive_choices` without the matching interaction,
    /// which skews the very ratio the level is derived from — a passenger
    /// handed two gifts on their first ride reads as 2/1 positive. Counting
    /// the act keeps the ratio meaning what it says.
    pub fn adjust(&mut self, delta: i32, constants: &ReputationConstants) {
        if delta == 0 {
            return;
        }
        let magnitude = delta.unsigned_abs();
        if delta > 0 {
            self.positive_choices += magnitude;
        }
        self.interactions += magnitude;
        self.settle(constants);
    }

    /// Derive the relationship level from the tallies.
    fn settle(&mut self, constants: &ReputationConstants) {
        let ratio = if self.interactions > 0 {
            self.positive_choices as f32 / self.interactions as f32
        } else {
            0.5
        };

        self.relationship_level = if ratio >= constants.trusted_ratio
            && self.interactions >= constants.minimum_interactions_for_trusted
        {
            RelationshipLevel::Trusted
        } else if ratio >= constants.friendly_ratio {
            RelationshipLevel::Friendly
        } else if ratio <= constants.hostile_ratio {
            RelationshipLevel::Hostile
        } else {
            RelationshipLevel::Neutral
        };
    }

    /// Get fare multiplier based on relationship
    pub fn fare_multiplier(&self, constants: &ReputationConstants) -> f32 {
        match self.relationship_level {
            RelationshipLevel::Trusted => constants.trusted_fare_mult,
            RelationshipLevel::Friendly => constants.friendly_fare_mult,
            RelationshipLevel::Hostile => constants.hostile_fare_mult,
            RelationshipLevel::Neutral => constants.default_fare_mult,
        }
    }

    /// Get risk modifier based on relationship
    pub fn risk_modifier(&self, constants: &ReputationConstants) -> i32 {
        match self.relationship_level {
            RelationshipLevel::Trusted => constants.trusted_risk_mod,
            RelationshipLevel::Hostile => constants.hostile_risk_mod,
            _ => 0,
        }
    }
}

/// Route history entry
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct RouteHistoryEntry {
    pub route_type: RouteType,
    pub driving_phase: DrivingPhase,
    pub fuel_cost: u32,
    pub time_cost: u32,
    pub risk_level: u32,
    pub passenger_id: Option<u32>,
    pub timestamp: f64,
}

/// Consecutive route streak tracking
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RouteStreak {
    pub route_type: RouteType,
    pub count: u32,
}

/// A rule imposed for a limited number of rides.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporaryRuleState {
    pub rule_id: u32,
    pub rides_remaining: u32,
}

/// Guideline decision history
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct GuidelineDecision {
    pub guideline_id: u32,
    pub passenger_id: u32,
    pub action: GuidelineAction,
    pub was_correct: bool,
    pub tells_present: Vec<PassengerTell>,
    pub timestamp: f64,
}

/// Guideline action choice
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuidelineAction {
    Follow,
    Break,
}

/// Dialogue display
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct CurrentDialogue {
    pub text: String,
    pub speaker: DialogueSpeaker,
    pub timestamp: f64,
}

/// The result of handing a passenger something.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeOutcome {
    pub text: String,
    /// Whether the item was on the passenger's `wantedItems` list — the only
    /// case that pays standing and settles their need.
    pub was_wanted: bool,
}

/// Who is speaking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]

pub enum DialogueSpeaker {
    Passenger,
    Driver,
    Narrator,
}

/// Complete game state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    /// Monotonic gameplay time. It advances in fixed simulation ticks while
    /// the shift is active, so waits between rendered frames cannot change
    /// seeded outcomes or item/weather deadlines.
    pub simulation_time: f64,
    // Core resources
    pub fuel: f32,
    /// Maximum fuel capacity for this shift (100 plus any capacity skills).
    pub max_fuel: f32,
    /// Current night within the run (1-based).
    pub night: u32,
    /// True once the final night of a run has been survived.
    pub run_complete: bool,
    /// Whether the one-shot end-of-shift warning has already been given.
    pub shift_end_warning_shown: bool,
    /// Bank and lore this shift paid into the meta-progression, and any
    /// separate bonus for completing the run, so the outcome screen can say
    /// what the night was worth beyond its own fare.
    pub shift_payout: MetaPayout,
    pub earnings: u32,
    pub time_remaining: u32,
    pub rides_completed: u32,
    pub rules_violated: u32,
    pub fare_contributions: Vec<FareContribution>,
    pub telemetry: ShiftTelemetry,
    pub ride_baseline: Option<RideBaseline>,

    // Current shift
    pub current_rules: Vec<Rule>,
    pub hidden_rules: Vec<Rule>,
    pub revealed_hidden_rules: Vec<Rule>,
    /// Rules a passenger imposed for a few rides, and how many are left.
    pub temporary_rules: Vec<TemporaryRuleState>,
    pub current_guidelines: Vec<Guideline>,
    pub inventory: Vec<InventoryItem>,
    pub current_passenger: Option<Passenger>,
    pub current_passenger_dialogue: Option<String>,
    pub current_ride: Option<CurrentRide>,
    pub current_event: Option<MidRideEvent>, // Mid-ride event
    pub game_phase: GamePhase,
    pub driving_phase: Option<DrivingPhase>,
    pub used_passengers: Vec<u32>,
    pub shift_start_time: Option<f64>,
    pub difficulty_level: u32,

    // Passenger state machine
    pub current_passenger_need_state: Option<PassengerNeedState>,
    pub detected_tells: Vec<DetectedTell>,
    pub player_trust: f32,
    pub decision_history: Vec<GuidelineDecision>,

    // Route tracking
    pub route_history: Vec<RouteHistoryEntry>,
    pub consecutive_route_streak: Option<RouteStreak>,

    // Weather system
    pub current_weather: WeatherCondition,
    pub time_of_day: TimeOfDay,
    pub season: Season,
    /// The month this run falls in, rolled once per run by `start_game` so
    /// the seasonal spawn weights and winter conditions rotate into play.
    /// Kept across the run's nights; each night derives `season` from it.
    pub campaign_month: u32,
    /// Whether the false-tell question is settled for the current decision
    /// window — set after the gate's single roll, planted or not. Detection
    /// runs every frame, so without this latch the gate would re-roll each
    /// frame (making its authored odds meaningless) and stack lies until
    /// the panel was all fiction. Cleared wherever `detected_tells` is.
    pub false_tell_planted: bool,
    /// Exceptions that won their authored per-ride probability roll for the
    /// current passenger. Rolled once when the passenger is presented, so
    /// the tell system, the decision judge, and the bot all agree on which
    /// exceptions are in play tonight — rolling at each ask would let them
    /// disagree frame to frame.
    pub live_exceptions: std::collections::HashSet<String>,
    /// The run's gameplay random stream. Every draw that decides anything —
    /// spawns, rolls, hazards, drops — comes from here, so a run is replayed
    /// by its seed and a mid-run save can carry the stream's state. Visual
    /// effects deliberately stay on macroquad's global generator: they draw
    /// every frame, and routing them through this stream would make the
    /// simulation's future depend on how long a screen was looked at.
    pub rng: macroquad_toolkit::rng::SeededRng,
    pub environmental_hazards: Vec<EnvironmentalHazard>,

    // Persistence
    pub passenger_reputation: HashMap<u32, PassengerReputation>,
    pub minimum_earnings: u32,

    // Item effects tracking
    pub rule_immunity_charges: u32,
    pub supernatural_protection: u32,
    pub curse_danger_bonus: u32,
    /// `rules_violated` as it stood when this ride was accepted.
    ///
    /// The shift-long count cannot answer "did the driver keep the rules on
    /// *this* ride", which is what a rule's `followConsequences` is a reward
    /// for. A violation is usually fatal, but not always -- a ward can absorb
    /// one, and the first ride of a shift is forgiven -- so surviving to the
    /// drop-off is not the same as having obeyed.
    pub violations_at_ride_start: u32,
    pub pending_trade: Option<(String, InventoryItem)>, // (passenger_name, offered_item)
    /// What the last swap did, for the drop-off screen to say out loud.
    ///
    /// Completing a trade wrote its result into `current_dialogue`, which only
    /// the driving screen renders — and accepting the next ride overwrites it.
    /// So the reputation and the relief a wanted item earns were paid in
    /// silence, and handing over the wrong thing said nothing at all. This is
    /// read on the screen the trade actually happens on.
    pub trade_outcome: Option<TradeOutcome>,

    // UI state
    pub current_dialogue: Option<CurrentDialogue>,
    /// Authored lines from consequences that fired this ride, shown on the
    /// drop-off screen. ~150 of these are written in the rule and guideline
    /// data and none ever reached a screen before this ledger.
    pub consequence_notes: Vec<String>,
    pub last_ride_completion: Option<RideCompletion>,
    pub game_over_reason: Option<String>,
    pub pending_audio: Option<AudioEvent>,
    pub last_audio_caption: Option<AudioEvent>,

    // Guideline decision state
    pub active_guideline: Option<Guideline>,
    pub guideline_decision_start_time: Option<f64>,
    pub guideline_time_remaining: f32,
    pub guideline_decision_seconds: f32,

    /// Cab actions that have already spent their comfort soothing this ride.
    /// Each comfort channel works once per ride; the list clears at drop-off.
    pub comfort_soothed_actions: Vec<String>,
    /// The night's single brink-of-meltdown grace has been used.
    pub brink_spent: bool,
    /// Tonight's rolled run modifier, if any — quota, difficulty, fare,
    /// fuel and lore hooks all consult it. Never on night 1 or The Last
    /// Fare.
    pub night_modifier: Option<NightModifier>,
    /// This night is The Last Fare: quota is moot and the only passenger the
    /// city sends is Death. Survived, it completes the run.
    pub last_fare_night: bool,
    /// Death's ride was completed this shift — the run's true ending.
    pub death_delivered: bool,
    /// The authored paragraph this ending selected, for the outcome screen.
    /// `None` on interim nights, which end on a button, not a chapter.
    pub epilogue: Option<String>,
}

impl GameState {
    /// Create a new game state with initial values from constants
    pub fn new(current_time: f64, constants: &GameConstants) -> Self {
        Self {
            simulation_time: current_time,
            fuel: constants.initial_fuel as f32,
            max_fuel: 100.0,
            night: 1,
            run_complete: false,
            shift_end_warning_shown: false,
            shift_payout: MetaPayout::default(),
            earnings: 0,
            time_remaining: constants.initial_time,
            rides_completed: 0,
            rules_violated: 0,
            fare_contributions: Vec::new(),
            telemetry: ShiftTelemetry::default(),
            ride_baseline: None,
            current_rules: Vec::new(),
            hidden_rules: Vec::new(),
            revealed_hidden_rules: Vec::new(),
            temporary_rules: Vec::new(),
            current_guidelines: Vec::new(),
            inventory: Vec::new(),
            current_passenger: None,
            current_passenger_dialogue: None,
            current_ride: None,
            current_event: None,
            game_phase: GamePhase::Loading,
            driving_phase: None,
            used_passengers: Vec::new(),
            shift_start_time: None,
            difficulty_level: 0,
            current_passenger_need_state: None,
            detected_tells: Vec::new(),
            player_trust: 0.5,
            decision_history: Vec::new(),
            route_history: Vec::new(),
            consecutive_route_streak: None,
            current_weather: WeatherCondition::default(),
            time_of_day: TimeOfDay::default(),
            season: Season::default(),
            campaign_month: DEFAULT_MONTH,
            false_tell_planted: false,
            live_exceptions: std::collections::HashSet::new(),
            // Seeded from the global generator so ordinary runs stay varied;
            // a seeded/daily run overwrites this with `SeededRng::new(seed)`
            // before play begins. Not reseeded per night: one run, one stream.
            rng: macroquad_toolkit::rng::SeededRng::new(macroquad_toolkit::rng::random_u64()),
            environmental_hazards: Vec::new(),
            passenger_reputation: HashMap::new(),
            minimum_earnings: constants.minimum_earnings,
            rule_immunity_charges: 0,
            supernatural_protection: 0,
            curse_danger_bonus: 0,
            violations_at_ride_start: 0,
            pending_trade: None,
            trade_outcome: None,
            current_dialogue: None,
            consequence_notes: Vec::new(),
            last_ride_completion: None,
            game_over_reason: None,
            pending_audio: None,
            last_audio_caption: None,
            active_guideline: None,
            guideline_decision_start_time: None,
            guideline_time_remaining: constants.guideline_decision_seconds,
            guideline_decision_seconds: constants.guideline_decision_seconds,
            comfort_soothed_actions: Vec::new(),
            brink_spent: false,
            night_modifier: None,
            last_fare_night: false,
            death_delivered: false,
            epilogue: None,
        }
    }

    /// Reset for a new shift using constants
    pub fn reset_for_new_shift(&mut self, current_time: f64, constants: &GameConstants) {
        self.fuel = constants.initial_fuel as f32;
        self.max_fuel = 100.0;
        self.earnings = 0;
        self.time_remaining = constants.initial_time;
        self.minimum_earnings = constants.minimum_earnings;
        self.rides_completed = 0;
        self.rules_violated = 0;
        self.fare_contributions.clear();
        self.telemetry = ShiftTelemetry::default();
        self.ride_baseline = None;
        self.current_rules.clear();
        self.hidden_rules.clear();
        self.revealed_hidden_rules.clear();
        self.temporary_rules.clear();
        self.current_guidelines.clear();
        self.current_passenger = None;
        self.current_passenger_dialogue = None;
        self.current_ride = None;
        self.current_event = None;
        self.game_phase = GamePhase::Waiting;
        self.driving_phase = None;
        self.used_passengers.clear();
        self.shift_start_time = Some(current_time);
        self.shift_end_warning_shown = false;
        self.shift_payout = MetaPayout::default();
        self.current_passenger_need_state = None;
        self.detected_tells.clear();
        self.false_tell_planted = false;
        self.live_exceptions.clear();
        self.route_history.clear();
        self.consecutive_route_streak = None;
        self.environmental_hazards.clear();
        self.player_trust = 0.5;
        self.rule_immunity_charges = 0;
        self.supernatural_protection = 0;
        self.curse_danger_bonus = 0;
        self.pending_trade = None;
        self.trade_outcome = None;
        self.current_dialogue = None;
        self.consequence_notes.clear();
        self.last_ride_completion = None;
        self.game_over_reason = None;
        self.pending_audio = None;
        self.last_audio_caption = None;
        self.active_guideline = None;
        self.guideline_decision_start_time = None;
        self.guideline_time_remaining = self.guideline_decision_seconds;
        self.comfort_soothed_actions.clear();
        self.brink_spent = false;
        self.night_modifier = None;
        self.last_fare_night = false;
        self.death_delivered = false;
        self.epilogue = None;
    }

    /// Check if time is running out
    pub fn is_time_critical(&self, constants: &ConstantsData) -> bool {
        self.time_remaining <= constants.timing.critical_time_threshold
    }

    /// Queue an informational sound and retain its visual equivalent.
    pub fn queue_audio(&mut self, cue: &str, caption: impl Into<String>, timestamp: f64) {
        let event = AudioEvent {
            cue: cue.to_string(),
            caption: caption.into(),
            timestamp,
        };
        self.pending_audio = Some(event.clone());
        self.last_audio_caption = Some(event);
    }

    /// Announce, once per shift, that the night is nearly over.
    ///
    /// `TIMING.SHIFT_END_WARNING_THRESHOLD` is authored at sixty minutes and
    /// was read by nothing; only `CRITICAL_TIME_THRESHOLD` reached the game,
    /// and at fifteen minutes it recolours the clock too late to act on —
    /// every route costs between eighteen and thirty-two. This fires while
    /// there is still a decision to make: one more fare, or bank the night.
    pub fn take_shift_end_warning(&mut self, constants: &ConstantsData) -> bool {
        if self.shift_end_warning_shown
            || self.time_remaining > constants.timing.shift_end_warning_threshold
        {
            return false;
        }
        self.shift_end_warning_shown = true;
        true
    }

    /// Check if shift should end
    pub fn should_end_shift(&self) -> bool {
        self.time_remaining == 0 || self.fuel <= 0.0
    }

    /// Reveal a hidden rule and move it into the active visible rules.
    pub fn reveal_hidden_rule(&mut self, rule_id: u32) -> Option<Rule> {
        let index = self
            .hidden_rules
            .iter()
            .position(|rule| rule.id == rule_id)?;
        let rule = self.hidden_rules.remove(index);
        if !self.revealed_hidden_rules.iter().any(|r| r.id == rule.id) {
            self.revealed_hidden_rules.push(rule.clone());
        }
        if !self.current_rules.iter().any(|r| r.id == rule.id) {
            self.current_rules.push(rule.clone());
        }
        Some(rule)
    }

    /// Clamp player trust after a gameplay outcome.
    pub fn adjust_player_trust(&mut self, delta: f32) {
        self.player_trust = (self.player_trust + delta).clamp(0.0, 1.0);
    }

    /// How many things stand between the driver and the next bad moment.
    ///
    /// Sums the two charges an item can buy — `rule_immunity_charges`, spent
    /// when a rule is broken, and `supernatural_protection`, spent when
    /// something pulls alongside — plus the remaining charges on every
    /// carried item with `protectiveProperties`, which is the pool
    /// `ProtectionService::consume_ward` actually spends from. Counting only
    /// the counters left a driver holding a Blessed Medallion reading "0
    /// wards".
    ///
    /// Summed rather than listed because the question the status bar answers is
    /// whether anything is protecting you, and the dialogue already names which
    /// ward fired when one does.
    pub fn wards_in_hand(&self) -> u32 {
        let carried: u32 = self
            .inventory
            .iter()
            .filter(|item| item.protective_properties.is_some())
            // No authored count still absorbs once before the ward is spent.
            .map(|item| item.durability.unwrap_or(1).max(1))
            .sum();
        self.rule_immunity_charges + self.supernatural_protection + carried
    }

    /// Fold any trust the current passenger's escalation moved into the
    /// driver's standing.
    ///
    /// Called once a frame rather than at each of the six places that push a
    /// passenger's need around, so no system has to remember to do it and none
    /// can do it twice.
    pub fn settle_passenger_trust(&mut self) {
        let delta = self
            .current_passenger_need_state
            .as_mut()
            .map(|need| need.take_pending_trust())
            .unwrap_or(0.0);
        if delta != 0.0 {
            self.adjust_player_trust(delta);
        }
    }

    /// Update consecutive route streak
    pub fn update_route_streak(&mut self, route: RouteType) {
        if let Some(ref mut streak) = self.consecutive_route_streak {
            if streak.route_type == route {
                streak.count += 1;
            } else {
                *streak = RouteStreak {
                    route_type: route,
                    count: 1,
                };
            }
        } else {
            self.consecutive_route_streak = Some(RouteStreak {
                route_type: route,
                count: 1,
            });
        }
    }

    /// Get passenger reputation, creating if needed
    pub fn get_passenger_reputation(&mut self, passenger_id: u32) -> &mut PassengerReputation {
        self.passenger_reputation.entry(passenger_id).or_default()
    }

    /// Calculate current score
    pub fn calculate_score(&self, constants: &ConstantsData) -> u32 {
        let base = self.earnings;
        let ride_bonus = self.rides_completed * constants.scoring.ride_bonus;
        let violation_penalty = self.rules_violated * constants.scoring.rule_violation_penalty;

        // Time left over is only worth something if the night was actually
        // worked. Paid unconditionally it rewards not driving: a shift that
        // ends on the first fare keeps the whole clock, and at two points a
        // minute that is 960 — which outscored real shifts on the
        // leaderboard, where an eight-hour night with nine passengers and no
        // violations sat below two instant losses.
        let time_bonus = if self.earnings >= self.minimum_earnings {
            self.time_remaining * constants.scoring.time_bonus_multiplier
        } else {
            0
        };

        (base + ride_bonus + time_bonus).saturating_sub(violation_penalty)
    }
}

#[cfg(test)]
mod trust_tests;

#[cfg(test)]
mod stage_tests;

#[cfg(test)]
mod reputation_tests;

#[cfg(test)]
mod score_tests;

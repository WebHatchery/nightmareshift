//! Passenger data types matching passengerData.json.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Passenger rarity tier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum Rarity {
    #[default]
    Common,
    Uncommon,
    Rare,
    Legendary,
}

/// Route types for navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum RouteType {
    #[default]
    Normal,
    Shortcut,
    Scenic,
    Police,
}

impl RouteType {
    /// How a route is named to the player.
    ///
    /// Deliberately not shared with `PlayerStats::route_key`, which spells
    /// these the same way but is a persisted save key: changing how a route
    /// reads on screen must not silently rewrite what is on disk.
    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Shortcut => "Shortcut",
            Self::Scenic => "Scenic",
            Self::Police => "Police",
        }
    }
}

/// How a passenger feels about a route
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum PreferenceLevel {
    Loves,
    Likes,
    #[default]
    Neutral,
    Dislikes,
    Fears,
}

impl PreferenceLevel {
    /// How a preference reads on the driving screen.
    ///
    /// Plain ASCII, because the bundled font draws nothing else. These were
    /// authored with emoji and the one caller stripped non-ASCII characters
    /// itself before drawing them — which worked, and made the fix depend on
    /// that caller remembering. The strings are now already sayable.
    pub fn display_text(&self) -> &'static str {
        match self {
            PreferenceLevel::Loves => "LOVES",
            PreferenceLevel::Likes => "Likes",
            PreferenceLevel::Neutral => "",
            PreferenceLevel::Dislikes => "Dislikes",
            PreferenceLevel::Fears => "FEARS",
        }
    }
}

/// Type of behavioral tell
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TellType {
    Verbal,
    Behavioral,
    Visual,
    Environmental,
}

/// Intensity of a tell
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TellIntensity {
    Subtle,
    Moderate,
    Obvious,
}

/// Type of passenger need
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum NeedType {
    Hunger,
    Fear,
    Wrath,
    Decay,
    Loneliness,
    #[default]
    Unknown,
}

/// A behavioral tell that hints at the passenger's state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassengerTell {
    #[serde(rename = "type")]
    pub tell_type: TellType,
    pub intensity: TellIntensity,
    pub description: String,
    #[serde(rename = "triggerPhrase")]
    pub trigger_phrase: Option<String>,
    #[serde(rename = "audioCue", default)]
    pub audio_cue: Option<String>,
    #[serde(rename = "animationCue", default)]
    pub animation_cue: Option<String>,
    #[serde(default)]
    pub reliability: f32,
}

/// Thresholds for need stage transitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeedThresholds {
    pub warning: u32,
    pub critical: u32,
    pub meltdown: u32,
}

/// How need level changes based on actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeedChangeProfile {
    pub passive: i32,
    pub obey: i32,
    #[serde(rename = "break")]
    pub break_rule: i32,
    #[serde(rename = "exceptionRelief")]
    pub exception_relief: i32,
}

/// Stage-specific tell intensities
pub type TellIntensityMap = HashMap<String, Vec<String>>;

/// Stage-specific dialogue lines
pub type DialogueByStage = HashMap<String, Vec<String>>;

/// Stage-specific impact values
pub type StageImpact = HashMap<String, f32>;

/// Complete state profile for a passenger's supernatural nature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassengerStateProfile {
    #[serde(rename = "needType")]
    pub need_type: NeedType,
    #[serde(rename = "initialLevel")]
    pub initial_level: u32,
    pub thresholds: NeedThresholds,
    #[serde(rename = "needChange")]
    pub need_change: NeedChangeProfile,
    #[serde(rename = "exceptionId")]
    pub exception_id: Option<String>,
    #[serde(rename = "tellIntensities")]
    pub tell_intensities: Option<TellIntensityMap>,
    #[serde(rename = "dialogueByStage")]
    pub dialogue_by_stage: Option<DialogueByStage>,
    /// What a passenger's escalation does to the driver's standing, keyed by
    /// stage name. See `NeedStage::key`.
    #[serde(rename = "trustImpact")]
    pub trust_impact: Option<StageImpact>,
    // `confidenceImpact` is authored beside `trustImpact` on thirteen
    // profiles and is deliberately not deserialized: there is no confidence
    // stat anywhere in the game for it to move, and a field parsed into a
    // struct nothing reads is the thing this project keeps finding. Serde
    // ignores it, so the authored numbers survive in the JSON for whoever
    // builds that system.
}

fn one_f32() -> f32 {
    1.0
}

/// Data-driven spawn weighting for a passenger.
///
/// Multipliers are keyed by the lowercase enum name (e.g. `"fog"`, `"latenight"`,
/// `"fall"`); a missing key means a neutral `1.0`. This replaces the per-id match
/// tables that used to live in `PassengerService`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpawnWeighting {
    /// Multiplier by weather type (keys: clear/rain/fog/snow/thunderstorm/wind).
    #[serde(default)]
    pub weather: HashMap<String, f32>,
    /// Multiplier by time phase (keys: dawn/morning/afternoon/dusk/night/latenight).
    #[serde(default)]
    pub time: HashMap<String, f32>,
    /// Multiplier by season (keys: spring/summer/fall/winter).
    #[serde(default)]
    pub season: HashMap<String, f32>,
    /// Extra multiplier applied when weather intensity is Heavy.
    #[serde(rename = "heavyWeatherBoost", default = "one_f32")]
    pub heavy_weather_boost: f32,
    /// If true, the time multiplier is scaled by the hour's supernatural activity.
    #[serde(rename = "supernaturalTimeScaling", default)]
    pub supernatural_time_scaling: bool,
    /// Extra multiplier when it is both a thunderstorm and late night.
    #[serde(rename = "stormLatenightBoost", default = "one_f32")]
    pub storm_latenight_boost: f32,
    /// Extra multiplier when it is both foggy and nighttime.
    #[serde(rename = "fogNightBoost", default = "one_f32")]
    pub fog_night_boost: f32,
}

/// Passenger preference for a specific route type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutePreference {
    pub route: RouteType,
    pub preference: PreferenceLevel,
    pub reason: String,
    #[serde(rename = "fareModifier")]
    pub fare_modifier: f32,
    #[serde(rename = "stressModifier")]
    pub stress_modifier: f32,
    #[serde(rename = "specialDialogue")]
    pub special_dialogue: Option<String>,
    #[serde(rename = "triggerChance")]
    pub trigger_chance: Option<f32>,
}

/// Rule modification capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleModification {
    #[serde(rename = "canModify")]
    pub can_modify: bool,
    #[serde(rename = "type")]
    pub modification_type: String,
    pub description: String,
    /// The rule an `add_temporary` passenger imposes. The Midnight Mayor
    /// authors a full one — the nightmare-difficulty "Mayor's Decree" — and
    /// the field was absent from this struct, so serde dropped it on load.
    #[serde(rename = "newRule", default)]
    pub new_rule: Option<TemporaryRule>,
}

/// A rule a passenger imposes for part of the shift.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporaryRule {
    pub id: u32,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub difficulty: super::rules::Difficulty,
    /// Rides the rule stays in force for.
    #[serde(default = "one_u32")]
    pub duration: u32,
}

fn one_u32() -> u32 {
    1
}

/// The roster id of Death's Taxi Driver, the reaper who collects failed
/// drivers — and, once every soul on the roster is mastered, the run's
/// final fare. The endgame gates on this id, so it is named once here.
pub const DEATH_PASSENGER_ID: u32 = 16;

/// Rule 20, "Let the Cold Be" — the rule whose guideline (1013) owns Death's
/// own exception. The Last Fare forces it into the night's rules so his ride
/// is always played against his rule, with its relief reachable.
pub const DEATHS_RULE_ID: u32 = 20;

/// A supernatural passenger in the game
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Passenger {
    pub id: u32,
    pub name: String,
    // `emoji` is authored on every passenger and read by nothing:
    // the screens draw vector portraits and shapes, not glyphs.
    pub description: String,
    pub pickup: String,
    pub destination: String,
    #[serde(rename = "personalRule")]
    pub personal_rule: String,
    pub supernatural: String,
    pub fare: u32,
    #[serde(default)]
    pub rarity: Rarity,
    #[serde(default)]
    pub items: Vec<String>,
    #[serde(default)]
    pub dialogue: Vec<String>,
    #[serde(default)]
    pub relationships: Vec<u32>,
    #[serde(rename = "backstoryUnlocked", default)]
    pub backstory_unlocked: bool,
    #[serde(rename = "backstoryDetails", default)]
    pub backstory_details: String,
    #[serde(default)]
    pub tells: Vec<PassengerTell>,
    #[serde(rename = "guidelineExceptions", default)]
    pub guideline_exceptions: Vec<String>,
    #[serde(rename = "deceptionLevel", default)]
    pub deception_level: f32,
    #[serde(rename = "stressLevel", default)]
    pub stress_level: f32,
    #[serde(rename = "trustRequired", default)]
    pub trust_required: f32,
    #[serde(rename = "stateProfile")]
    pub state_profile: Option<PassengerStateProfile>,
    #[serde(rename = "routePreferences", default)]
    pub route_preferences: Vec<RoutePreference>,
    #[serde(rename = "ruleModification")]
    pub rule_modification: Option<RuleModification>,

    // Item and trading fields
    #[serde(rename = "dropItems", default)]
    pub drop_items: Vec<String>,
    #[serde(rename = "wantsTrade", default)]
    pub wants_trade: bool,
    #[serde(rename = "wantedItems", default)]
    pub wanted_items: Vec<String>,
    /// What this passenger offers in trade to someone already carrying
    /// something on their `wantedItems` list, instead of a pick from their
    /// drop pool.
    ///
    /// The Collector authors the Soul Protection Ward here. It is the only
    /// thing in the catalogue that stops Death's Taxi Driver, its description
    /// says he crafted it, and before this nothing in the game could give it
    /// to a player at all -- it existed in the item file and in one unit test.
    /// Gating it on holding what he wants means his best work is earned rather
    /// than rolled for.
    #[serde(rename = "tradeReward", default)]
    pub trade_reward: Option<String>,
    #[serde(rename = "isSupernatural", default)]
    pub is_supernatural: bool,
    /// Which `itemPoolData.json` pool this passenger's generic drops come from.
    /// Authored explicitly because `supernatural` is descriptive prose, not a key.
    #[serde(rename = "itemCategory", default)]
    pub item_category: Option<String>,

    // NEW traits field
    #[serde(default)]
    pub traits: Vec<String>,

    /// Data-driven environmental spawn weighting (None = neutral on all factors).
    #[serde(rename = "spawnWeighting", default)]
    pub spawn_weighting: Option<SpawnWeighting>,
}

impl Passenger {
    /// Get random dialogue line, drawn from the shift's stream.
    pub fn random_dialogue(&self, rng: &mut macroquad_toolkit::rng::SeededRng) -> Option<&str> {
        rng.choose(&self.dialogue).map(|s| s.as_str())
    }

    /// Find route preference for a given route type
    pub fn get_route_preference(&self, route: RouteType) -> Option<&RoutePreference> {
        self.route_preferences.iter().find(|p| p.route == route)
    }

    /// Check if passenger fears this route
    pub fn fears_route(&self, route: RouteType) -> bool {
        self.get_route_preference(route)
            .map(|p| p.preference == PreferenceLevel::Fears)
            .unwrap_or(false)
    }
}

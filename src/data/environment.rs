//! Environment data types for weather, time, and hazards.

use super::passenger::RouteType;
use serde::{Deserialize, Serialize};

/// Weather types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum WeatherType {
    #[default]
    Clear,
    Rain,
    Fog,
    Snow,
    Thunderstorm,
    Wind,
}

impl WeatherType {
    /// Get display name for weather type
    pub fn name(&self) -> &'static str {
        match self {
            WeatherType::Clear => "Clear",
            WeatherType::Rain => "Rain",
            WeatherType::Fog => "Fog",
            WeatherType::Snow => "Snow",
            WeatherType::Thunderstorm => "Thunderstorm",
            WeatherType::Wind => "Wind",
        }
    }
}

/// Weather intensity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum WeatherIntensity {
    #[default]
    Light,
    Moderate,
    Heavy,
}

/// Type of weather effect
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeatherEffectType {
    VisibilityReduction,
    FuelConsumption,
    TimeDelay,
    SupernaturalAttraction,
    PassengerBehavior,
    RouteBlockage,
}

/// A weather effect applied during a condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherEffect {
    #[serde(rename = "type")]
    pub effect_type: WeatherEffectType,
    pub value: i32,
    pub description: String,
    #[serde(rename = "appliesTo")]
    pub applies_to: Option<String>,
}

/// Current weather conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherCondition {
    #[serde(rename = "type")]
    pub weather_type: WeatherType,
    pub intensity: WeatherIntensity,
    pub visibility: u32,
    pub description: String,
    // `icon` is an emoji the bundled font cannot draw, and nothing read it.
    #[serde(default)]
    pub effects: Vec<WeatherEffect>,
    pub duration: u32,
    pub start_time: f64,
}

impl Default for WeatherCondition {
    fn default() -> Self {
        Self {
            weather_type: WeatherType::Clear,
            intensity: WeatherIntensity::Light,
            visibility: 100,
            description: "Clear night skies".to_string(),
            effects: Vec::new(),
            duration: 60,
            start_time: 0.0,
        }
    }
}

/// Time of day phases
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum TimePhase {
    Dawn,
    Morning,
    Afternoon,
    Dusk,
    #[default]
    Night,
    Latenight,
}

/// Current time of day
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeOfDay {
    pub phase: TimePhase,
    pub hour: u32,
    pub description: String,
    #[serde(rename = "ambientLight")]
    pub ambient_light: u32,
    #[serde(rename = "supernaturalActivity")]
    pub supernatural_activity: u32,
}

impl Default for TimeOfDay {
    fn default() -> Self {
        Self {
            phase: TimePhase::Night,
            hour: 20,
            description: "Darkness settles over the city".to_string(),
            ambient_light: 15,
            supernatural_activity: 85,
        }
    }
}

impl TimeOfDay {
    /// Check if it's nighttime (night or latenight)
    pub fn is_night(&self) -> bool {
        matches!(self.phase, TimePhase::Night | TimePhase::Latenight)
    }
}

/// Season types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum SeasonType {
    Spring,
    Summer,
    #[default]
    Fall,
    Winter,
}

/// Temperature levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Temperature {
    Cold,
    Cool,
    Mild,
    Warm,
    Hot,
}

/// Current season
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Season {
    #[serde(rename = "type")]
    pub season_type: SeasonType,
    // `month` and `temperature` are authored and read by nothing: the season
    // reaches the game through `season_type` and its description.
    pub description: String,
}

impl Default for Season {
    fn default() -> Self {
        Self {
            season_type: SeasonType::Fall,
            description: "Autumn brings unpredictable conditions".to_string(),
        }
    }
}

/// The month a campaign falls in before a run rolls its own: October,
/// mid-fall. Kept beside [`Season`] because it is season data, not engine
/// tuning.
pub const DEFAULT_MONTH: u32 = 10;

/// Environmental hazard types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HazardType {
    Construction,
    Accident,
    SupernaturalEvent,
    RoadClosure,
    PoliceCheckpoint,
}

/// Hazard severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HazardSeverity {
    Minor,
    Major,
    Extreme,
}

/// Effects of a hazard on routes
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HazardEffects {
    #[serde(rename = "routeBlocked")]
    pub route_blocked: Option<Vec<RouteType>>,
    #[serde(rename = "timeDelay")]
    pub time_delay: Option<u32>,
    #[serde(rename = "fuelIncrease")]
    pub fuel_increase: Option<u32>,
    #[serde(rename = "riskIncrease")]
    pub risk_increase: Option<u32>,
    #[serde(rename = "forcedChoice")]
    pub forced_choice: Option<bool>,
}

/// An environmental hazard affecting routes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentalHazard {
    pub id: String,
    #[serde(rename = "type")]
    pub hazard_type: HazardType,
    pub location: String,
    pub severity: HazardSeverity,
    pub description: String,
    #[serde(default)]
    pub effects: HazardEffects,
    pub duration: u32,
    pub start_time: f64,
    #[serde(rename = "weatherTriggered", default)]
    pub weather_triggered: bool,
}

impl EnvironmentalHazard {
    /// What this hazard does to a route, or `None` if it costs nothing.
    ///
    /// The briefing listed a hazard's location and prose and stopped there, so
    /// the night was planned without knowing that the road work on the
    /// Downtown Bridge closes the Shortcut. The driving screen shows the
    /// blocking hazard on the route card it disables, which is too late to be
    /// a plan. Every number here already reaches route pricing; none of it
    /// reached the player before they chose.
    /// `hazard_mult` is the driver's own hazard resistance, from the skill
    /// tree. Route pricing scales the hazard-inflicted portion of a leg by it,
    /// so a forecast quoting the raw authored numbers overstates what a driver
    /// with Reinforced Chassis will actually pay -- and understates the skill
    /// they bought. The route cards on the driving screen have always shown the
    /// softened figure; this makes the briefing agree with them.
    ///
    /// A forecast rather than a quote: each component is scaled and rounded on
    /// its own, where route pricing sums first and rounds once, so the two can
    /// differ by a unit.
    pub fn toll(&self, hazard_mult: f32) -> Option<String> {
        let scale = |value: u32| (value as f32 * hazard_mult).round() as u32;
        let mut parts: Vec<String> = Vec::new();

        if let Some(blocked) = self
            .effects
            .route_blocked
            .as_ref()
            .filter(|blocked| !blocked.is_empty())
        {
            let names: Vec<&str> = blocked.iter().map(|route| route.label()).collect();
            parts.push(format!("closes {}", names.join(" and ")));
        }
        if let Some(fuel) = self
            .effects
            .fuel_increase
            .map(scale)
            .filter(|value| *value > 0)
        {
            parts.push(format!("+{fuel} fuel"));
        }
        if let Some(time) = self
            .effects
            .time_delay
            .map(scale)
            .filter(|value| *value > 0)
        {
            parts.push(format!("+{time} min"));
        }
        if let Some(risk) = self
            .effects
            .risk_increase
            .map(scale)
            .filter(|value| *value > 0)
        {
            parts.push(format!("+{risk} risk"));
        }

        (!parts.is_empty()).then(|| parts.join(", "))
    }

    /// Check if this hazard blocks a specific route type
    pub fn blocks_route(&self, route: RouteType) -> bool {
        self.effects
            .route_blocked
            .as_ref()
            .map(|blocked| blocked.contains(&route))
            .unwrap_or(false)
    }
}

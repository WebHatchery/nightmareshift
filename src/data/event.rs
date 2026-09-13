use serde::{Deserialize, Serialize};

/// Tags representing potential risks on a route
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RiskTag {
    HighTraffic,
    PolicePatrol,
    SpiritualDisturbance,
    SlipperyRoads,
    RoadConstruction,
    DenseFog,
    GangActivity,
    Potholes,
    FlashFloods,
    StrangeNoises,
}

impl RiskTag {
    /// Get a user-friendly name for the risk
    pub fn name(&self) -> &'static str {
        match self {
            RiskTag::HighTraffic => "High Traffic",
            RiskTag::PolicePatrol => "Police Patrol",
            RiskTag::SpiritualDisturbance => "Spiritual Disturbance",
            RiskTag::SlipperyRoads => "Slippery Roads",
            RiskTag::RoadConstruction => "Road Construction",
            RiskTag::DenseFog => "Dense Fog",
            RiskTag::GangActivity => "Gang Activity",
            RiskTag::Potholes => "Severe Potholes",
            RiskTag::FlashFloods => "Flash Floods",
            RiskTag::StrangeNoises => "Strange Noises",
        }
    }

    /// Get a description for the risk
    pub fn description(&self) -> &'static str {
        match self {
            RiskTag::HighTraffic => "Delays likely.",
            RiskTag::PolicePatrol => "Watch your speed.",
            RiskTag::SpiritualDisturbance => "Entities active.",
            RiskTag::SlipperyRoads => "Hard to control.",
            RiskTag::RoadConstruction => "Detours ahead.",
            RiskTag::DenseFog => "Low visibility.",
            RiskTag::GangActivity => "Avoid stopping.",
            RiskTag::Potholes => "Suspension damage.",
            RiskTag::FlashFloods => "Hydroplane risk.",
            RiskTag::StrangeNoises => "Unsettling sounds.",
        }
    }
}

/// A consequence for an event choice (simplified compared to rules::Consequence)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventConsequence {
    Fuel(f32),
    Time(u32),
    Risk(i32),
    Stress(i32),
}

/// A choice within a mid-ride event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventChoice {
    pub description: String,
    pub risk_type: RiskTag,
    pub consequence: EventConsequence,
    pub required_trait: Option<String>,
}

/// A mid-ride event that occurs during travel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidRideEvent {
    pub title: String,
    pub description: String,
    pub choices: Vec<EventChoice>,
}

fn default_weight() -> f32 {
    1.0
}

/// An authored mid-ride event template loaded from `eventData.json`.
///
/// Templates carry fixed, hand-written choices. At runtime the ride service
/// filters the deck by the current route, picks one weighted by `weight`, and
/// may append a passenger-specific "use your ability" choice before shuffling.
#[derive(Debug, Clone, Serialize, Deserialize)]
// The JSON also authors an `id` label per event; nothing selects or
// deduplicates by it, so it stays an authoring aid and is not deserialized.
pub struct EventTemplate {
    pub id: String,
    pub title: String,
    pub description: String,
    #[serde(default = "default_weight")]
    pub weight: f32,
    /// Routes this event may appear on. Empty means eligible on any route.
    #[serde(default)]
    pub routes: Vec<crate::data::passenger::RouteType>,
    pub choices: Vec<EventChoice>,
}

impl EventTemplate {
    /// Whether this template is eligible for the given route.
    pub fn eligible_for(&self, route: crate::data::passenger::RouteType) -> bool {
        self.routes.is_empty() || self.routes.contains(&route)
    }
}

#[cfg(test)]
mod tests;

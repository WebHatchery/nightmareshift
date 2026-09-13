//! Location data matching locationData.json.

use serde::{Deserialize, Serialize};

fn default_fare_modifier() -> f32 {
    1.0
}

fn default_location_multiplier() -> f32 {
    1.0
}

/// A location in the city where passengers can be picked up or dropped off
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub name: String,
    pub description: String,
    pub atmosphere: String,
    /// Route-risk contribution when this location is a passenger's pickup.
    #[serde(rename = "riskLevel")]
    pub risk_level: u32,
    /// Fare multiplier applied when this location is a passenger's destination.
    /// Remote or dangerous drop-offs pay a premium; safe civic ones pay less.
    #[serde(rename = "fareModifier", default = "default_fare_modifier")]
    pub fare_modifier: f32,
    /// Multiplier for the time and fuel leg to this location.
    #[serde(rename = "distanceMultiplier", default = "default_location_multiplier")]
    pub distance_multiplier: f32,
    #[serde(rename = "fuelMultiplier", default = "default_location_multiplier")]
    pub fuel_multiplier: f32,
    /// How strongly this kerb attracts passengers when they are being dealt.
    #[serde(rename = "spawnAffinity", default = "default_location_multiplier")]
    pub spawn_affinity: f32,
    /// Additional risk on the destination leg, independent of pickup risk.
    #[serde(default)]
    pub destination_risk: f32,
}

#[cfg(test)]
mod tests;

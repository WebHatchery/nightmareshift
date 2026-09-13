//! Balance-measurement records retained by the live simulation.

use crate::data::{InventoryItem, Passenger};
use serde::{Deserialize, Serialize};

/// What a finished shift paid into the meta-progression currencies.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct MetaPayout {
    pub bank: u32,
    pub lore: u32,
    /// Extra bank paid for finishing the whole run, separate from the night.
    pub run_bonus_bank: u32,
    pub run_bonus_lore: u32,
}

impl MetaPayout {
    pub fn completed_a_run(&self) -> bool {
        self.run_bonus_bank > 0 || self.run_bonus_lore > 0
    }
}

/// One fare's contribution to the shift total, retained for balance reports.
///
/// The outcome card only needs the most recent ride, but campaign measurement
/// needs the whole distribution so a single generous passenger cannot hide an
/// otherwise impossible quota.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FareContribution {
    pub passenger_id: u32,
    pub passenger_name: String,
    pub fare: u32,
}

/// Shift-long counters for systems whose value was previously invisible.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShiftTelemetry {
    pub refuel_stops: u32,
    pub refuel_cost_paid: u32,
    pub comfort_relief: u32,
    pub normal_route_relief: u32,
    pub ward_interventions: u32,
    pub brink_saves: u32,
}

/// Snapshot taken when a passenger accepts the ride, used to build a
/// self-contained drop-off receipt without teaching the UI simulation math.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct RideBaseline {
    pub fuel: f32,
    pub time: u32,
    pub need: u32,
    pub rules_violated: u32,
    pub comfort_relief: u32,
    pub normal_route_relief: u32,
    pub ward_interventions: u32,
    pub brink_saves: u32,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct RideImpact {
    pub fuel_spent: u32,
    pub time_spent: u32,
    pub need_delta: i32,
    pub rules_violated: u32,
    pub comfort_relief: u32,
    pub normal_route_relief: u32,
    pub ward_interventions: u32,
    pub brink_saves: u32,
}

/// Ride completion data retained for the drop-off receipt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RideCompletion {
    pub passenger: Passenger,
    pub fare_earned: u32,
    pub items_received: Vec<InventoryItem>,
    pub backstory_unlocked: Option<(String, String)>,
    pub impact: RideImpact,
}

/// One informational sound and its visual equivalent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioEvent {
    pub cue: String,
    pub caption: String,
    pub timestamp: f64,
}

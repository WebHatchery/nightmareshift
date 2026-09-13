//! Per-night run modifiers, matching `nightModifierData.json`.
//!
//! A modifier is a named twist an ordinary mid-run night may carry — "Blood
//! Moon: fares +20%, one rule deeper" — rolled once in `begin_night` on the
//! seeded stream and applied through hooks that already exist: the quota,
//! the difficulty that drives the rule draw, the fare calculation, the
//! starting fuel, and the lore payout. Night 1 never rolls one (the opening
//! stays learnable) and The Last Fare never rolls one (that night is
//! authored whole).

use serde::{Deserialize, Serialize};

fn one_f32() -> f32 {
    1.0
}

fn one_weight() -> u32 {
    1
}

/// One authored night twist. Every numeric field defaults to "no effect",
/// so an entry authors only what it changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NightModifier {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Relative draw weight among the modifiers once a night rolls one.
    #[serde(default = "one_weight")]
    pub weight: u32,
    /// Multiplier on every fare earned this night.
    #[serde(default = "one_f32")]
    pub fare_mult: f32,
    /// Multiplier on the night's earnings quota.
    #[serde(default = "one_f32")]
    pub quota_mult: f32,
    /// Added to the night's effective difficulty before the rule draw
    /// (still capped at `SCORING.MAX_DIFFICULTY`).
    #[serde(default)]
    pub difficulty_bonus: u32,
    /// Applied to the starting fuel, clamped to at least a sliver.
    #[serde(default)]
    pub start_fuel_delta: i32,
    /// Added to the shift's lore payout.
    #[serde(default)]
    pub lore_bonus: u32,
}

/// The modifier deck: one roll decides whether tonight carries one at all,
/// a weighted draw decides which.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NightModifierData {
    /// Chance an eligible night carries any modifier.
    pub chance: f32,
    pub modifiers: Vec<NightModifier>,
}

impl NightModifierData {
    /// Roll tonight's modifier on the seeded stream, or nothing.
    pub fn roll(&self, rng: &mut macroquad_toolkit::rng::SeededRng) -> Option<NightModifier> {
        if self.modifiers.is_empty() || !rng.chance(self.chance.clamp(0.0, 1.0)) {
            return None;
        }
        let total: u32 = self.modifiers.iter().map(|m| m.weight.max(1)).sum();
        let mut pick = rng.range_i32(0, total as i32) as u32;
        for modifier in &self.modifiers {
            let weight = modifier.weight.max(1);
            if pick < weight {
                return Some(modifier.clone());
            }
            pick -= weight;
        }
        self.modifiers.last().cloned()
    }
}

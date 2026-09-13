//! Skill tree and almanac data matching JSON files.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Effect of a skill.
///
/// Dispatch is by `target`: `SkillModifiers::from_unlocked` consumes the
/// seven stat targets, and ability unlocks are consulted by trait id via
/// the unlocked-skill list. `effect_type` routes only one way — the
/// `"ability_unlock"` value marks a skill as a per-passenger ability; the
/// `stat_boost`/`mechanic_unlock`/`passive_bonus` labels are authorial
/// grouping with no behavior, and `value` on an ability unlock is a
/// constant 1. A test in this file holds every effect to one of the two
/// dispatch paths, because `from_unlocked` silently ignores an unknown
/// target — a typo would sell a skill that does nothing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEffect {
    #[serde(rename = "type")]
    pub effect_type: String,
    pub target: String,
    pub value: f64,
}

/// A skill that can be unlocked
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub cost: u32,
    // `icon` is authored on every skill and read by nothing: the skill tree
    // draws a category mark and the name's initials.
    pub category: String,
    #[serde(default)]
    pub prerequisites: Vec<String>,
    pub effect: SkillEffect,
}

impl Skill {
    /// Check if prerequisites are met
    pub fn can_unlock(&self, unlocked_skills: &[String]) -> bool {
        self.prerequisites
            .iter()
            .all(|prereq| unlocked_skills.contains(prereq))
    }
}

/// Almanac level data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlmanacLevel {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub rewards: Vec<String>,
}

impl AlmanacLevel {
    /// How this level reads on the almanac card, or `None` when it reveals
    /// nothing nameable.
    ///
    /// The sentence lives here rather than in the draw call so the wording is
    /// testable without a window -- the same reason `PlayerStats::reveal_story`
    /// holds what a story unlock means.
    /// `already_known` is anything the player has by another route, left out
    /// so a card that has just shown someone's backstory does not go on
    /// promising it one level later. A story can be earned in play instead of
    /// bought with lore.
    pub fn reveals_line(&self, already_known: &[&str]) -> Option<String> {
        let remaining: Vec<&str> = self
            .rewards
            .iter()
            .map(String::as_str)
            .filter(|reward| !already_known.contains(reward))
            .collect();
        if remaining.is_empty() {
            return None;
        }
        Some(format!("{} reveals {}", self.name, remaining.join(", ")))
    }
}

/// Lore costs for unlocking almanac levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoreCosts {
    #[serde(rename = "UNLOCK_LEVEL_1")]
    pub level_1: u32,
    #[serde(rename = "UNLOCK_LEVEL_2")]
    pub level_2: u32,
    #[serde(rename = "UNLOCK_LEVEL_3")]
    pub level_3: u32,
}

/// Almanac data structure matching almanacData.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlmanacData {
    #[serde(rename = "ALMANAC_LEVELS")]
    pub levels: HashMap<String, AlmanacLevel>,
    #[serde(rename = "LORE_COSTS")]
    pub lore_costs: LoreCosts,
}

impl AlmanacData {
    /// Get level by number
    pub fn get_level(&self, level: u32) -> Option<&AlmanacLevel> {
        self.levels.get(&level.to_string())
    }

    /// Get cost to unlock a specific level
    pub fn get_upgrade_cost(&self, target_level: u32) -> u32 {
        match target_level {
            1 => self.lore_costs.level_1,
            2 => self.lore_costs.level_2,
            3 => self.lore_costs.level_3,
            _ => 0,
        }
    }
}

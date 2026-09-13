//! Inventory item types matching the item system.

use super::passenger::Rarity;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Named item-name pools keyed by supernatural category, loaded from
/// `itemPoolData.json`. Used to generate an item drop when a passenger has no
/// explicit `dropItems`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ItemPools {
    #[serde(default)]
    pub ghost: Vec<String>,
    #[serde(default)]
    pub vampire: Vec<String>,
    #[serde(default)]
    pub demon: Vec<String>,
    #[serde(default)]
    pub occult: Vec<String>,
    #[serde(default)]
    pub holy: Vec<String>,
    #[serde(default)]
    pub common: Vec<String>,
}

impl ItemPools {
    /// Pick a random item name from the named category, falling back to the
    /// common pool (and finally a hardcoded name) if a pool is empty. Draws
    /// from the shift's stream so drops replay under a seed.
    pub fn pick(&self, rng: &mut macroquad_toolkit::rng::SeededRng, category: &str) -> String {
        let pool = match category {
            "ghost" => &self.ghost,
            "vampire" => &self.vampire,
            "demon" => &self.demon,
            "occult" => &self.occult,
            "holy" => &self.holy,
            _ => &self.common,
        };
        let pool = if pool.is_empty() { &self.common } else { pool };
        if pool.is_empty() {
            return "Old Key".to_string();
        }
        pool[rng.below(pool.len())].clone()
    }

    /// Every name any pool can produce, in declaration order. Used by the
    /// catalog-coverage test so a name can never be droppable without also
    /// being defined in `itemData.json`.
    pub fn all_names(&self) -> Vec<&str> {
        [
            &self.ghost,
            &self.vampire,
            &self.demon,
            &self.occult,
            &self.holy,
            &self.common,
        ]
        .into_iter()
        .flatten()
        .map(String::as_str)
        .collect()
    }
}

/// Type of inventory item
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ItemType {
    Protective,
    Cursed,
    Consumable,
    Tradeable,
    #[default]
    Story,
}

/// Type of item effect
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemEffectType {
    FuelBonus,
    TimeBonus,
    RuleImmunity,
    SupernaturalProtection,
    FuelDrain,
    TimePenalty,
    RuleTrigger,
    ReputationModifier,
}

/// An effect provided by an item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemEffect {
    #[serde(rename = "type")]
    pub effect_type: ItemEffectType,
    pub value: i32,
    // `duration` is authored on one effect -- the Crystal Pendant's 30 -- and
    // read by nothing. Honouring it means giving every charge its own expiry
    // clock, which is a subsystem no other item in the catalogue would use, so
    // it is not deserialized. The number survives in the JSON for whoever
    // decides that clock is worth building.
    /// An authored precondition for this effect firing. See
    /// `ItemService::condition_met`.
    #[serde(default)]
    pub condition: Option<String>,
}

/// Type of curse penalty
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CursePenalty {
    FuelDrain,
    TimeAcceleration,
    ForcedChoices,
    AttractingDanger,
}

/// Properties of a cursed item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursedProperties {
    #[serde(rename = "penaltyType")]
    pub penalty_type: CursePenalty,
    #[serde(rename = "penaltyValue")]
    pub penalty_value: i32,
    #[serde(rename = "triggersAfter")]
    pub triggers_after: u32,
    #[serde(rename = "canBeRemoved")]
    pub can_be_removed: bool,
    #[serde(rename = "removalCondition")]
    pub removal_condition: Option<String>,
}

/// Type of protection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtectionType {
    SupernaturalImmunity,
    RuleForgiveness,
}

/// Properties of a protective item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectiveProperties {
    #[serde(rename = "protectionType")]
    pub protection_type: ProtectionType,
    #[serde(rename = "protectionStrength")]
    pub protection_strength: u32,
    #[serde(rename = "usesRemaining")]
    pub uses_remaining: Option<u32>,
    #[serde(rename = "protectsAgainst")]
    pub protects_against: Option<Vec<String>>,
}

/// An item in the player's inventory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryItem {
    pub id: String,
    pub name: String,
    pub source: String,
    // `backstoryItem` is on no item in the catalogue and read by nothing,
    // so it deserialized to false sixteen times a night and decided nothing.
    #[serde(rename = "type")]
    pub item_type: ItemType,
    #[serde(default)]
    pub rarity: Rarity,
    pub description: String,
    #[serde(default)]
    pub effects: Vec<ItemEffect>,
    pub durability: Option<u32>,
    #[serde(rename = "maxDurability")]
    pub max_durability: Option<u32>,
    #[serde(rename = "acquiredAt")]
    pub acquired_at: f64,
    #[serde(rename = "canUse")]
    pub can_use: bool,
    #[serde(rename = "canTrade")]
    pub can_trade: bool,
    #[serde(rename = "cursedProperties")]
    pub cursed_properties: Option<CursedProperties>,
    #[serde(rename = "protectiveProperties")]
    pub protective_properties: Option<ProtectiveProperties>,
    /// Whether this item's curse has already taken its toll.
    ///
    /// `should_trigger_curse` is a threshold -- possession time past
    /// `triggersAfter` -- and `update_items` runs every frame, so without this
    /// a held curse charged its penalty sixty times a second once the clock
    /// passed. The Dusty Mirror drains three fuel, which is a full tank inside a
    /// second; the Blood Vial takes thirty minutes off a shift that only has
    /// four hundred and eighty.
    #[serde(default)]
    pub curse_fired: bool,
    /// How much of this item's charge age has already taken.
    ///
    /// `apply_deterioration` works out the total wear an item's age is worth and
    /// so has to know how much of that it has already charged. Without this it
    /// subtracted the running total on every frame the loop ran -- one point per
    /// frame once past twenty minutes -- and wore anything with a charge count to
    /// nothing almost the instant the clock passed.
    #[serde(default)]
    pub aged_by: u32,
}

impl InventoryItem {
    /// How many uses are left, when the item counts them.
    ///
    /// Eleven items author a `maxDurability` and `use_item` spends one each
    /// time, removing the item on its last use. Nothing displayed it, so a
    /// driver holding a three-use ward could not tell a full one from its final
    /// charge, and it vanished from the inventory without warning. An item you
    /// cannot plan around is not much of an item.
    /// Only for an item the driver can actually use. The four cursed items
    /// carry a `maxDurability` in the sixties to the hundreds, which is a decay
    /// clock rather than a charge count -- "100 of 100 uses" on a locket nobody
    /// can use is worse than saying nothing.
    pub fn uses_left(&self) -> Option<(u32, u32)> {
        if !self.can_use {
            return None;
        }
        match (self.durability, self.max_durability) {
            (Some(left), Some(most)) if most > 0 => Some((left, most)),
            _ => None,
        }
    }

    /// Check if this item is cursed
    pub fn is_cursed(&self) -> bool {
        self.item_type == ItemType::Cursed || self.cursed_properties.is_some()
    }

    /// Check if curse should trigger based on time possessed
    pub fn should_trigger_curse(&self, current_time: f64) -> bool {
        if let Some(ref curse) = self.cursed_properties {
            let possession_minutes = (current_time - self.acquired_at) / 60.0;
            possession_minutes >= curse.triggers_after as f64
        } else {
            false
        }
    }

    /// Whether this item can be handed to a passenger.
    ///
    /// `canTrade` says whether it is the sort of thing anyone would take;
    /// a curse's `canBeRemoved` says whether it will let itself be given away
    /// at all. The Contract Fragment has the driver's name on it and refuses.
    /// Both were authored and only the first was read, so a binding contract
    /// could be traded off like a spare coin.
    pub fn can_be_given_away(&self) -> bool {
        if !self.can_trade {
            return false;
        }
        self.cursed_properties
            .as_ref()
            .map(|curse| curse.can_be_removed)
            .unwrap_or(true)
    }

    /// Check if item is broken (durability depleted)
    pub fn is_broken(&self) -> bool {
        self.durability.map(|d| d == 0).unwrap_or(false)
    }

    /// Apply deterioration based on time
    /// Wear this item by however much its age is worth and has not yet taken.
    ///
    /// The formula gives a *total* -- one point per ten minutes past a ten-minute
    /// grace period -- so charging it as a delta is only correct once. It is
    /// charged against `aged_by` rather than recomputed onto `durability`,
    /// because a use spends a charge too and resetting from age alone would hand
    /// those back.
    pub fn apply_deterioration(&mut self, current_time: f64) {
        if self.max_durability.is_none() {
            return;
        }
        let Some(durability) = self.durability.as_mut() else {
            return;
        };

        let age_minutes = (current_time - self.acquired_at) / 60.0;
        if age_minutes <= 10.0 {
            return;
        }

        let owed = ((age_minutes - 10.0) / 10.0) as u32;
        let unpaid = owed.saturating_sub(self.aged_by);
        if unpaid > 0 {
            *durability = durability.saturating_sub(unpaid);
            self.aged_by = owed;
        }
    }
}

/// The item catalog, loaded from `itemData.json`: every droppable item name
/// mapped to the template used to mint an `InventoryItem`.
///
/// Keys are matched case-insensitively (they are stored lowercase), so pool
/// entries and passenger `dropItems` can use display casing.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ItemCatalog {
    templates: HashMap<String, ItemTemplate>,
}

impl ItemCatalog {
    /// Look up a template by item name. Unknown names fall back to an inert
    /// story keepsake rather than failing the drop outright.
    pub fn get(&self, name: &str) -> ItemTemplate {
        self.templates
            .get(&name.to_lowercase())
            .cloned()
            .unwrap_or_else(ItemTemplate::keepsake)
    }

    /// Every defined item name.
    #[cfg(test)]
    pub fn names(&self) -> Vec<String> {
        self.templates.keys().cloned().collect()
    }

    /// True when the catalog defines this name.
    ///
    /// No longer test-only: a passenger's authored `tradeReward` is checked
    /// against the catalogue before being offered, because `create_item` on an
    /// unknown name would hand the player a placeholder rather than fail.
    pub fn contains(&self, name: &str) -> bool {
        self.templates.contains_key(&name.to_lowercase())
    }

    /// Whether the embedded catalog failed to load or contains no entries.
    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }

    /// Create an inventory item from the catalog.
    pub fn create_item(&self, name: &str, source: &str, current_time: f64) -> InventoryItem {
        let template = self.get(name);
        let charges = template.max_durability.or_else(|| {
            template
                .protective_properties
                .as_ref()
                .and_then(|properties| properties.uses_remaining)
        });

        InventoryItem {
            id: format!("{}_{}", name.replace(' ', "_"), current_time as u64),
            name: name.to_string(),
            source: source.to_string(),
            item_type: template.item_type,
            rarity: template.rarity,
            description: template.description,
            effects: template.effects,
            // One pool of charges, whichever counter the data authors it in.
            //
            // A protective item carries two: `maxDurability` pays for using it
            // and `protectiveProperties.usesRemaining` pays for absorbing an
            // encounter. On the seven items that author both they are the same
            // number written twice, and tracked separately they drift -- a ward
            // could absorb three times, still report a full count, and then be
            // used four more, giving eight charges out of an authored four.
            // Three others author only the absorption count, and reading just
            // `maxDurability` left them with no charges at all.
            durability: charges,
            max_durability: charges,
            acquired_at: current_time,
            can_use: template.can_use,
            can_trade: template.can_trade,
            cursed_properties: template.cursed_properties,
            protective_properties: template.protective_properties,
            curse_fired: false,
            aged_by: 0,
        }
    }
}

/// Template for creating inventory items, as authored in `itemData.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemTemplate {
    #[serde(rename = "type", default)]
    pub item_type: ItemType,
    #[serde(default)]
    pub rarity: Rarity,
    pub description: String,
    #[serde(rename = "canUse", default)]
    pub can_use: bool,
    #[serde(rename = "canTrade", default)]
    pub can_trade: bool,
    #[serde(rename = "maxDurability", default)]
    pub max_durability: Option<u32>,
    #[serde(rename = "cursedProperties", default)]
    pub cursed_properties: Option<CursedProperties>,
    #[serde(rename = "protectiveProperties", default)]
    pub protective_properties: Option<ProtectiveProperties>,
    #[serde(default)]
    pub effects: Vec<ItemEffect>,
}

impl ItemTemplate {
    /// The inert fallback used when an item name is not in the catalog.
    fn keepsake() -> Self {
        Self {
            item_type: ItemType::Story,
            rarity: Rarity::Common,
            description: "A mysterious object left behind by a passenger".to_string(),
            can_use: false,
            can_trade: false,
            max_durability: Some(100),
            cursed_properties: None,
            protective_properties: None,
            effects: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests;

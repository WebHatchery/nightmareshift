//! Wards carried in the inventory, and what they will take a hit for.
//!
//! Protection used to be a pair of bare counters on `GameState` —
//! `supernatural_protection` and `rule_immunity_charges` — fed by item
//! *effects* and skills. The `protectiveProperties` block authored on every
//! protective item in `itemData.json` was read by nothing, so a ward's
//! `protectionType` decided nothing, its `usesRemaining` limited nothing, and
//! `protectsAgainst` (the Soul Protection Ward's defence against Death's Taxi
//! Driver specifically) never applied. This spends the wards themselves.

use crate::data::{InventoryItem, ProtectionType};

/// A ward that absorbed something, named so the game can say what saved you.
pub struct AbsorbedBy {
    pub item_name: String,
    /// True when the ward was used up and removed from the inventory.
    pub consumed: bool,
    /// The ward's authored `protectionStrength`, so a stronger charm pulls a
    /// passenger further back from the edge than a scrap of cloak does.
    pub strength: u32,
}

impl AbsorbedBy {
    /// How to name this ward in narration, noting when it was spent for good.
    pub fn describe(&self) -> String {
        if self.consumed {
            format!("{}, spent for good,", self.item_name)
        } else {
            self.item_name.clone()
        }
    }
}

/// Spends inventory wards.
pub struct ProtectionService;

impl ProtectionService {
    /// Whether a ward applies to the passenger in front of you.
    ///
    /// An empty or absent `protectsAgainst` warded against anything; a
    /// populated one only covers the passenger ids it lists.
    fn covers(item: &InventoryItem, passenger_id: Option<u32>) -> bool {
        let Some(properties) = &item.protective_properties else {
            return false;
        };
        match &properties.protects_against {
            None => true,
            Some(ids) if ids.is_empty() => true,
            Some(ids) => passenger_id
                .map(|id| ids.iter().any(|listed| listed == &id.to_string()))
                .unwrap_or(false),
        }
    }

    /// Find the first ward of `kind` that covers `passenger_id`, spend one of
    /// its uses, and drop it from the inventory when it is used up.
    ///
    /// Returns `None` when no ward applies, leaving the inventory untouched so
    /// the caller can fall back to the counters.
    pub fn consume_ward(
        inventory: &mut Vec<InventoryItem>,
        kind: ProtectionType,
        passenger_id: Option<u32>,
    ) -> Option<AbsorbedBy> {
        let index = inventory.iter().position(|item| {
            item.protective_properties
                .as_ref()
                .is_some_and(|properties| properties.protection_type == kind)
                && Self::covers(item, passenger_id)
        })?;

        let item_name = inventory[index].name.clone();
        let strength = inventory[index]
            .protective_properties
            .as_ref()
            .map(|properties| properties.protection_strength)
            .unwrap_or(1);
        // Absorbing spends from the same pool using it does.
        //
        // This used to decrement `protectiveProperties.usesRemaining` while
        // `use_item` decremented `durability`, so the two ran independently: a
        // ward could absorb its way through one count and be used through the
        // other, and the charge readout in the inventory -- which reads
        // `durability` -- went on claiming a full ward after three absorptions.
        // `create_item` now seeds the one pool from whichever counter the item
        // authors.
        let consumed = match inventory[index].durability {
            Some(charges) if charges > 1 => {
                inventory[index].durability = Some(charges - 1);
                false
            }
            // A ward with no authored count at all is spent on first use rather
            // than warding forever.
            _ => true,
        };

        if consumed {
            inventory.remove(index);
        }

        Some(AbsorbedBy {
            item_name,
            consumed,
            strength,
        })
    }
}

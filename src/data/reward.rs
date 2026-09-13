//! Meta-progression payouts, loaded from `rewardData.json`.
//!
//! The bank balance and lore fragments a run yields are what gate the skill
//! tree and the almanac. Per-shift earnings alone pace the twenty-node tree
//! far slower than the almanac, so the two milestone systems the game already
//! tracks — achievements and completing a full multi-night run — pay into
//! them here rather than being scoreboard entries with nothing behind them.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single payout into the meta-progression currencies.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Payout {
    /// Bank balance, spent in the skill tree.
    #[serde(default)]
    pub bank: u32,
    /// Lore fragments, spent in the almanac.
    #[serde(default)]
    pub lore: u32,
}

impl Payout {
    /// True when this payout does nothing.
    pub fn is_empty(&self) -> bool {
        self.bank == 0 && self.lore == 0
    }
}

/// Payout for surviving every night of a run.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct RunCompletionReward {
    #[serde(default)]
    pub bank: u32,
    #[serde(default)]
    pub lore: u32,
    /// Additional bank per night survived, so a longer run pays more.
    #[serde(rename = "bankPerNight", default)]
    pub bank_per_night: u32,
}

impl RunCompletionReward {
    /// The payout for completing a run of `nights` nights.
    pub fn payout(&self, nights: u32) -> Payout {
        Payout {
            bank: self.bank + self.bank_per_night * nights,
            lore: self.lore,
        }
    }
}

/// The rate at which lore fragments are sold back to the depot for bank.
///
/// The two meta-currencies had disjoint sinks: lore bought almanac levels
/// and nothing else, so it went dead once every passenger was mastered,
/// while bank paced the skill tree an order of magnitude slower. Trading one
/// for the other makes the almanac and the skill tree compete for the same
/// resource, so studying the roster deeply and buying broadly into the tree
/// become a choice rather than two unrelated tracks.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct LoreExchange {
    /// Fragments spent per trade.
    #[serde(default)]
    pub lore: u32,
    /// Bank received per trade.
    #[serde(default)]
    pub bank: u32,
}

impl LoreExchange {
    /// True when the exchange is configured to actually trade something.
    pub fn is_available(&self) -> bool {
        self.lore > 0 && self.bank > 0
    }
}

/// What giving a passenger an item they actually asked for is worth.
///
/// `wantedItems` already gates whether a trade is offered, but every trade
/// resolved identically, so handing over the exact thing a passenger wanted
/// paid no more than handing over junk. This is what makes the choice of
/// which item to give matter.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct WantedTradeBonus {
    /// Need level removed — settling a passenger who got what they wanted.
    #[serde(rename = "needRelief", default)]
    pub need_relief: i32,
    /// Positive reputation entries credited with this passenger.
    #[serde(rename = "reputationBonus", default)]
    pub reputation_bonus: u32,
}

impl WantedTradeBonus {
    /// True when the bonus does anything.
    pub fn is_active(&self) -> bool {
        self.need_relief > 0 || self.reputation_bonus > 0
    }
}

/// All meta-progression payouts.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RewardData {
    /// Keyed by achievement id, paid once when the achievement first unlocks.
    #[serde(default)]
    pub achievements: HashMap<String, Payout>,
    #[serde(rename = "runCompletion", default)]
    pub run_completion: RunCompletionReward,
    #[serde(rename = "loreExchange", default)]
    pub lore_exchange: LoreExchange,
    #[serde(rename = "wantedTrade", default)]
    pub wanted_trade: WantedTradeBonus,
}

impl RewardData {
    /// The payout for first unlocking `achievement_id`.
    pub fn for_achievement(&self, achievement_id: &str) -> Payout {
        self.achievements
            .get(achievement_id)
            .copied()
            .unwrap_or_default()
    }
}

impl Payout {
    /// How this payout reads on a card, or `None` when it is worth nothing.
    ///
    /// Achievements fund both halves of the meta-progression -- bank buys
    /// skills, lore buys almanac levels -- and the amounts run from $250 with 3
    /// lore up to $1500 with 8. All of it was authored, paid on unlock, and
    /// shown nowhere, so a player could not tell which goal was worth chasing
    /// first. Big Earner pays the most bank in the game and the card said only
    /// "Locked".
    pub fn describe(&self) -> Option<String> {
        match (self.bank, self.lore) {
            (0, 0) => None,
            (bank, 0) => Some(format!("pays ${bank}")),
            (0, lore) => Some(format!("pays {lore} lore")),
            (bank, lore) => Some(format!("pays ${bank} and {lore} lore")),
        }
    }
}

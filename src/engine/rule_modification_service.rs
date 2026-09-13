//! Passengers who change the rules of the night.
//!
//! Three of the roster author a `ruleModification`: The Collector can remove a
//! rule "in exchange for a terrible price", Madame Zelda reveals a hidden one,
//! and the Midnight Mayor imposes his own Decree. Only `canModify` was ever
//! read, and it was read in `passenger_has_exception`, where returning true
//! meant these three were excused from *every* rule for the whole ride — the
//! opposite of a bargain with a terrible price. The `type` deciding which of
//! the three things happened was ignored, and the Mayor's authored rule was
//! dropped at parse for want of a field to land in.

use crate::data::*;
use crate::state::*;

/// What a passenger's arrival did to tonight's rules, for narration.
pub struct RuleChange {
    pub message: String,
}

pub struct RuleModificationService;

impl RuleModificationService {
    /// Apply a passenger's rule modification, once, as they get in.
    pub fn apply(state: &mut GameState, passenger: &Passenger) -> Option<RuleChange> {
        let modification = passenger.rule_modification.as_ref()?;
        if !modification.can_modify {
            return None;
        }

        match modification.modification_type.as_str() {
            "reveal_hidden" => Self::reveal_hidden(state, passenger),
            "remove_rule" => Self::remove_rule(state, passenger),
            "add_temporary" => Self::add_temporary(state, passenger, modification),
            _ => None,
        }
    }

    /// Madame Zelda sees what tonight is hiding.
    fn reveal_hidden(state: &mut GameState, passenger: &Passenger) -> Option<RuleChange> {
        let rule_id = state.hidden_rules.first().map(|rule| rule.id)?;
        let revealed = state.reveal_hidden_rule(rule_id)?;
        Some(RuleChange {
            message: format!(
                "{} names a rule you were never told: {}.",
                passenger.name, revealed.title
            ),
        })
    }

    /// The Collector takes a rule off the board, and takes something for it.
    ///
    /// The price is danger, not money: the rule is gone but the roads are
    /// worse for the rest of the ride.
    fn remove_rule(state: &mut GameState, passenger: &Passenger) -> Option<RuleChange> {
        if state.current_rules.is_empty() {
            return None;
        }
        let index = state.rng.below(state.current_rules.len());
        let removed = state.current_rules.remove(index);
        state.curse_danger_bonus += 2;
        Some(RuleChange {
            message: format!(
                "{} folds \"{}\" away like a card. The night notices the gap.",
                passenger.name, removed.title
            ),
        })
    }

    /// The Midnight Mayor adds one of his own, for a few rides.
    fn add_temporary(
        state: &mut GameState,
        passenger: &Passenger,
        modification: &RuleModification,
    ) -> Option<RuleChange> {
        let authored = modification.new_rule.as_ref()?;
        if state
            .current_rules
            .iter()
            .any(|rule| rule.id == authored.id)
        {
            return None;
        }

        let mut rule = Rule {
            id: authored.id,
            title: authored.title.clone(),
            description: authored.description.clone(),
            difficulty: authored.difficulty,
            visible: true,
            ..Rule::default()
        };
        rule.rule_type = RuleType::Conditional;
        state.current_rules.push(rule);
        state.temporary_rules.push(TemporaryRuleState {
            rule_id: authored.id,
            rides_remaining: authored.duration.max(1),
        });

        Some(RuleChange {
            message: format!(
                "{} imposes \"{}\" for the next {} rides.",
                passenger.name,
                authored.title,
                authored.duration.max(1)
            ),
        })
    }

    /// Count down any temporary rules and drop the ones that have expired.
    /// Called once per completed ride.
    pub fn expire_temporary_rules(state: &mut GameState) -> Vec<String> {
        let mut lifted = Vec::new();
        for temporary in state.temporary_rules.iter_mut() {
            temporary.rides_remaining = temporary.rides_remaining.saturating_sub(1);
        }
        state.temporary_rules.retain(|temporary| {
            if temporary.rides_remaining > 0 {
                return true;
            }
            if let Some(index) = state
                .current_rules
                .iter()
                .position(|rule| rule.id == temporary.rule_id)
            {
                lifted.push(state.current_rules.remove(index).title);
            }
            false
        });
        lifted
    }
}

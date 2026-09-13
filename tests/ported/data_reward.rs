use nightmare_shift::data::loader::load_rewards;
use nightmare_shift::state::PlayerStats;
use std::collections::HashSet;

/// Every achievement's payout has to be sayable, since the card now prints
/// it. A payout of nothing prints nothing, and that must not happen for a
/// real achievement -- the existing test above covers it being non-zero, and
/// this covers the sentence actually forming.
#[test]
fn every_achievement_payout_reads_as_a_sentence() {
    let rewards = load_rewards();
    for achievement in PlayerStats::achievement_definitions() {
        let payout = rewards.for_achievement(&achievement.id);
        let line = payout
            .describe()
            .unwrap_or_else(|| panic!("{} has nothing to say about what it pays", achievement.id));
        if payout.bank > 0 {
            assert!(
                line.contains(&payout.bank.to_string()),
                "{} pays ${} and says {line:?}",
                achievement.id,
                payout.bank
            );
        }
        if payout.lore > 0 {
            assert!(
                line.contains(&payout.lore.to_string()),
                "{} pays {} lore and says {line:?}",
                achievement.id,
                payout.lore
            );
        }
    }
}

/// Both halves are named when both are paid, and a payout of one currency
/// does not claim the other.
#[test]
fn a_payout_names_only_what_it_pays() {
    use nightmare_shift::data::reward::Payout;

    assert_eq!(Payout { bank: 0, lore: 0 }.describe(), None);

    let bank_only = Payout { bank: 250, lore: 0 }.describe().expect("a line");
    assert!(bank_only.contains("250") && !bank_only.contains("lore"));

    let lore_only = Payout { bank: 0, lore: 4 }.describe().expect("a line");
    assert!(lore_only.contains("lore") && !lore_only.contains('$'));

    let both = Payout { bank: 800, lore: 6 }.describe().expect("a line");
    assert!(both.contains("800") && both.contains('6'));
}

/// Every achievement must be worth something. An achievement with no
/// reward entry is a scoreboard line the meta-progression never sees.
#[test]
fn every_achievement_pays_out() {
    let rewards = load_rewards();
    for achievement in PlayerStats::achievement_definitions() {
        let payout = rewards.for_achievement(&achievement.id);
        assert!(
            !payout.is_empty(),
            "achievement {:?} has no reward",
            achievement.id
        );
    }
}

/// Conversely, a reward keyed to an id no achievement uses is dead data
/// that silently pays nobody — usually a rename or a typo.
#[test]
fn every_reward_names_a_real_achievement() {
    let ids: HashSet<String> = PlayerStats::achievement_definitions()
        .into_iter()
        .map(|a| a.id)
        .collect();
    for id in load_rewards().achievements.keys() {
        assert!(ids.contains(id), "reward {id:?} names no achievement");
    }
}

/// The exchange is what stops lore going dead once every passenger is
/// mastered, so it has to be configured and has to be a real trade.
#[test]
fn lore_exchanges_for_bank() {
    let rate = load_rewards().lore_exchange;
    assert!(rate.is_available(), "lore exchange is not configured");
}

/// A lore fragment must be worth little enough that trading supplements
/// driving rather than replacing it.
///
/// This replaces a weaker check that bounded the exchange against what
/// the almanac *costs* — 144 fragments to master the roster — and passed
/// comfortably while the mechanic printed money. A player does not earn
/// 144 fragments; a sixty-shift session measured 1091, and at the old
/// twenty-four dollars a fragment that surplus was worth $26,184 against
/// a $19,800 tree. The bound has to be against the fragment's own value,
/// because income is what the almanac budget failed to predict.
#[test]
fn a_lore_fragment_cannot_be_worth_much() {
    let rate = load_rewards().lore_exchange;
    let cheapest_skill = nightmare_shift::data::loader::load_skill_tree()
        .iter()
        .map(|skill| skill.cost)
        .min()
        .expect("the tree has nodes");

    let per_fragment = rate.bank as f32 / rate.lore.max(1) as f32;
    let fragments_for_cheapest = cheapest_skill as f32 / per_fragment;
    assert!(
        fragments_for_cheapest >= 50.0,
        "{fragments_for_cheapest:.0} fragments buys the cheapest ${cheapest_skill} skill;              lore is standing in for driving"
    );
}

/// Giving a passenger what they asked for has to be worth more than
/// giving them anything else, or `wantedItems` only ever decides whether
/// an offer appears and never what the player should hand over.
#[test]
fn giving_a_wanted_item_is_rewarded() {
    assert!(
        load_rewards().wanted_trade.is_active(),
        "a wanted trade pays nothing"
    );
}

/// Every passenger must want at least one item that the player can
/// actually part with, otherwise their trade offer can never be
/// satisfied with the thing they asked for.
#[test]
fn every_passengers_wants_are_giveable() {
    let catalog = nightmare_shift::data::loader::load_item_catalog();
    for passenger in nightmare_shift::data::loader::load_passengers() {
        if !passenger.wants_trade {
            continue;
        }
        let giveable = passenger
            .wanted_items
            .iter()
            .filter(|name| catalog.get(name).can_trade)
            .count();
        assert!(
            giveable > 0,
            "{} wants nothing the player can hand over",
            passenger.name
        );
    }
}

/// Surviving a whole run is the game's headline result and must pay.
#[test]
fn completing_a_run_pays_out() {
    let rewards = load_rewards();
    let payout = rewards.run_completion.payout(5);
    assert!(!payout.is_empty(), "run completion pays nothing");
    assert!(
        payout.bank > rewards.run_completion.bank,
        "bankPerNight never applied"
    );
}

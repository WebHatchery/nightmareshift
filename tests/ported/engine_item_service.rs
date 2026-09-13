use super::*;
use nightmare_shift::data::loader::{
    load_constants, load_item_catalog, load_item_pools, load_passengers, load_rules,
    load_skill_tree,
};
use nightmare_shift::engine::RideService;
use std::collections::HashSet;

/// Every authored `itemCategory` must name a real, non-empty pool.
/// A typo here silently demotes a passenger to the common pool.
/// Every pool must be reachable from at least one passenger, otherwise the
/// items authored in it can never drop.
/// Every item in the catalogue has to be obtainable somehow.
///
/// This is the test that found the Soul Protection Ward: legendary, the
/// only thing that turns away Death's Taxi Driver, and reachable from
/// nowhere in the game. It sat in the item file and in one unit test while
/// no pool listed it, no passenger dropped it, and no code created it.
///
/// An unobtainable item is worse than a missing one. It reads as content,
/// it is balanced against, and it is checked for in `ProtectionService`.
/// Every `item` a rule consequence names must exist in the catalogue —
/// `create_item` on an unknown name silently mints a placeholder keepsake
/// instead of failing, so a typo here would read as a bug in the reward.
/// Carrying what the crafter wants gets his work offered; carrying
/// nothing he wants does not.
///
/// This covers the wire rather than the data. The offer is built before the
/// player picks what to hand over, so "already holding something wanted"
/// is the only condition available at that moment -- and it has to be the
/// condition, or the ward is either unreachable again or free.
/// A `tradeReward` naming something the catalogue does not define would
/// hand the player a placeholder, since `create_item` substitutes rather
/// than failing.
/// A curse fires once, not once a frame.
///
/// `update_items` is called from the frame loop and `should_trigger_curse`
/// is a threshold rather than an event -- possession time past
/// `triggersAfter` -- so a held curse applied its penalty on every frame
/// once the clock passed. A one-point fuel drain becomes sixty a second.
/// Age wears an item down by the amount age is worth, not by that amount
/// every frame.
///
/// `apply_deterioration` computes `(age - 10) / 10` -- the total wear since
/// the item was picked up -- and then *subtracts* it, from a function the
/// frame loop calls sixty times a second. At twenty minutes of age that is
/// one point per frame, so anything with a charge count is worn to nothing
/// and swept out of the inventory almost the instant the clock passes.
/// Wear still accumulates across steps. Fixing the per-frame repeat must not
/// have stopped age mattering at all.
/// A charge spent by using the item is not handed back by age.
///
/// This is why the wear is charged against a running total rather than
/// recomputed onto `durability` from age alone: uses and age both spend from
/// the same pool, and recomputing would undo the uses.
/// Every curse names itself when it bites.
///
/// Three of the four penalties took their toll in silence: fuel and the
/// clock simply dropped and the danger bonus climbed, with nothing on screen
/// connecting any of it to the thing in the driver's pocket. A loss the
/// player cannot attribute is indistinguishable from the game cheating.
/// A ward is spent over exactly the charges it authors -- no more, and no
/// fewer.
///
/// The Blessed Medallion authors five absorption charges and no
/// `maxDurability`. Before, `use_item` found no durability, did nothing and
/// handed the item back, so one medallion was unlimited supernatural
/// protection. My first fix for that spent it outright on the first use,
/// which traded unlimited for one and threw away four authored charges. It
/// gets all five now, because both spending paths draw on one pool seeded
/// from whichever counter the item authors.
/// No usable item survives being used enough times.
///
/// Sweeps the whole catalogue rather than the three that were broken, since
/// the fault was a branch that quietly did nothing for a shape of item
/// nobody had thought about.
/// An item that counts its uses reports them, and spending one lowers the
/// count the inventory shows.
///
/// Eleven items author a `maxDurability` and `use_item` spends one per use,
/// removing the item on its last. Nothing displayed it, so a three-use ward
/// looked identical to one on its final charge and then vanished without
/// warning.
/// An item with no durability authored reports none, rather than "0 of 0".
/// And nor does an item the driver cannot use, whatever durability it
/// carries. The cursed items author sixty to a hundred, which is a decay
/// clock rather than a charge count, and reporting it as uses on a locket
/// nobody can use is worse than saying nothing.
/// The Crystal Pendant's charge is conditional, and the condition has to
/// bite. It grants rule immunity authored `"supernatural_encounter"`, so
/// offering it to an ordinary fare should do nothing -- and should not
/// spend the item doing nothing either.
/// An item with no condition on its effects is unaffected by any of this.
/// Every authored condition has to be one `condition_met` recognises.
/// An unknown one holds, so a typo would leave the item working and the
/// authored gate silently absent -- visible only here.
/// A reputation item has to work the first time you offer one.
///
/// The effect used `passenger_reputation.get_mut`, and a reputation entry
/// is not created until a ride with that passenger completes — so on a
/// first meeting, which is most of them, there was no entry and the whole
/// point of the item was dropped on the floor without a word.
/// Trade wants are matched against inventory item names, so every wanted
/// name must exist in the catalog and be tradeable — a want for an
/// untradeable item can never be satisfied.
/// Someone must want to trade, or the whole trade modal is unreachable.
/// And the other direction: every trait a passenger carries must have a
/// skill that buys its use.
///
/// The almanac lists a passenger's traits from Lv.1, so a trait with no
/// skill behind it is advertised to the player and permanently
/// unreachable. The existing test only checked that no skill was wasted;
/// nothing checked that no trait was.
/// Every `ability_unlock` skill must be earnable-into-use: some passenger
/// carries the matching trait, or the skill is bank balance thrown away.
/// A gift with nobody to give it to must not be thrown away.
///
/// The inventory opens on any game screen, including the wait at the rank
/// between fares, and `reputation_modifier` is the one effect that needs
/// somebody in the back seat -- it adjusts the current passenger's opinion
/// of the driver. With the cab empty it found no passenger, adjusted
/// nothing, and the consumable was removed regardless. The withered flowers
/// and the faded photograph are both common, both first-night items, and
/// both could be destroyed for nothing by a curious click.
///
/// `use_item` already refuses to spend an item whose conditions are unmet
/// and says why. These two effects simply never declared a condition, so
/// they now author `passenger_present` and take that existing path.
/// Mastery is durable: a story already known must retain the same drop bonus
/// that the ride which first revealed it received.
mod test_part_1;
mod test_part_2;

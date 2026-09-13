/// A protective item has one pool of charges, not two.
///
/// `durability` pays for using an item and `protectiveProperties.usesRemaining`
/// pays for absorbing an encounter, and on the seven items that author both
/// they are the same number written twice. Tracked separately they drift: a
/// ward can absorb three times and still report a full count, then be used
/// four times, giving eight charges out of an authored four.
#[test]
fn absorbing_and_using_draw_on_the_same_charges() {
    let catalog = load_item_catalog();
    let mut beads = catalog.create_item("Prayer Beads", "Sister Agnes", 0.0);
    let (before, _) = beads.uses_left().expect("the beads count charges");
    assert!(before > 1, "pick an item with more than one charge");

    let mut inventory = vec![beads.clone()];
    ProtectionService::consume_ward(
        &mut inventory,
        nightmare_shift::data::ProtectionType::RuleForgiveness,
        None,
    )
    .expect("the beads ward against rule violations");

    beads = inventory.pop().expect("the beads survived one absorption");
    let (after, _) = beads.uses_left().expect("still counting");
    assert_eq!(
        after,
        before - 1,
        "absorbing spent a different pool from the one the inventory shows"
    );
}

/// And a ward that authors only the absorption count still has that many
/// charges to spend, rather than being thrown away on its first use.
#[test]
fn a_ward_keeps_the_charges_it_authors() {
    let catalog = load_item_catalog();
    let medallion = catalog.create_item("Blessed Medallion", "Sister Agnes", 0.0);
    let authored = medallion
        .protective_properties
        .as_ref()
        .and_then(|properties| properties.uses_remaining)
        .expect("the medallion authors ward charges");
    assert!(authored > 1, "pick a ward with more than one charge");

    let (left, most) = medallion
        .uses_left()
        .expect("a ward with authored charges counts them");
    assert_eq!(
        (left, most),
        (authored, authored),
        "the medallion reports {left} of {most} against {authored} authored"
    );
}

use super::*;
use nightmare_shift::data::loader::load_item_catalog;

fn item(name: &str) -> InventoryItem {
    load_item_catalog().create_item(name, "test", 0.0)
}

/// Every authored ward must be spendable by the passenger it is meant
/// for, or it protects against nothing in practice. Names are sorted
/// because the catalog is a `HashMap` and an unordered sweep made this
/// test pass or fail depending on whether it happened to reach the
/// target-restricted Soul Protection Ward first.
#[test]
fn every_authored_ward_is_consumable_by_its_target() {
    let catalog = load_item_catalog();
    let mut names = catalog.names();
    names.sort();

    let mut checked = 0;
    for name in names {
        let Some(properties) = catalog.get(&name).protective_properties else {
            continue;
        };
        // A ward that names targets is spent by one of them; an
        // untargeted ward is spent by anyone.
        let passenger_id = properties
            .protects_against
            .as_ref()
            .and_then(|ids| ids.first())
            .and_then(|id| id.parse::<u32>().ok())
            .unwrap_or(1);
        let mut inventory = vec![item(&name)];
        assert!(
            ProtectionService::consume_ward(
                &mut inventory,
                properties.protection_type,
                Some(passenger_id),
            )
            .is_some(),
            "{:?} ward {name:?} absorbs nothing for passenger {passenger_id}",
            properties.protection_type
        );
        checked += 1;
    }
    assert!(checked > 0, "no item authors protective properties");
}

/// Only the protection types a system actually spends may be authored.
/// `SafePassage` and `LuckyEncounters` once sat in the schema with no
/// consumer — an item authored with one warded against nothing while
/// looking protective. They are deleted now; this stays as the tripwire
/// for any future variant added without a system that spends it.
#[test]
fn no_item_authors_an_unspent_protection_type() {
    const SPENT: [ProtectionType; 2] = [
        ProtectionType::SupernaturalImmunity,
        ProtectionType::RuleForgiveness,
    ];
    let catalog = load_item_catalog();
    let mut names = catalog.names();
    names.sort();
    for name in names {
        let Some(properties) = catalog.get(&name).protective_properties else {
            continue;
        };
        assert!(
            SPENT.contains(&properties.protection_type),
            "{name:?} authors {:?}, which no system spends",
            properties.protection_type
        );
    }
}

/// A ward listing `protectsAgainst` must not fire for other passengers.
#[test]
fn a_targeted_ward_only_covers_its_target() {
    let mut inventory = vec![item("Soul Protection Ward")];
    assert!(
        ProtectionService::consume_ward(
            &mut inventory,
            ProtectionType::SupernaturalImmunity,
            Some(1),
        )
        .is_none(),
        "the Soul Protection Ward fired for the wrong passenger"
    );
    assert_eq!(inventory.len(), 1, "a ward that did not fire was consumed");

    assert!(
        ProtectionService::consume_ward(
            &mut inventory,
            ProtectionType::SupernaturalImmunity,
            Some(16),
        )
        .is_some(),
        "the Soul Protection Ward did not fire for Death's Taxi Driver"
    );
    assert!(inventory.is_empty(), "a single-use ward was not consumed");
}

/// A multi-use ward survives until its uses run out.
#[test]
fn a_multi_use_ward_is_spent_gradually() {
    let mut inventory = vec![item("Rune Stone")];
    let uses = inventory[0]
        .protective_properties
        .as_ref()
        .and_then(|p| p.uses_remaining)
        .expect("Rune Stone authors usesRemaining");
    for _ in 0..uses - 1 {
        assert!(ProtectionService::consume_ward(
            &mut inventory,
            ProtectionType::SupernaturalImmunity,
            None,
        )
        .is_some());
        assert_eq!(inventory.len(), 1);
    }
    let last =
        ProtectionService::consume_ward(&mut inventory, ProtectionType::SupernaturalImmunity, None)
            .expect("final use");
    assert!(last.consumed);
    assert!(inventory.is_empty());
}

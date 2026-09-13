use nightmare_shift::data::loader::{load_item_catalog, load_item_pools, load_passengers};

/// Every name a pool can produce must exist in the catalog. Without this,
/// adding a name to `itemPoolData.json` silently mints an inert keepsake.
#[test]
fn every_pooled_item_is_in_the_catalog() {
    let pools = load_item_pools();
    let catalog = load_item_catalog();
    let missing: Vec<&str> = pools
        .all_names()
        .into_iter()
        .filter(|name| !catalog.contains(name))
        .collect();
    assert!(
        missing.is_empty(),
        "items missing from catalog: {missing:?}"
    );
}

/// Passenger-specific `dropItems` go through the same catalog.
#[test]
fn every_passenger_drop_item_is_in_the_catalog() {
    let catalog = load_item_catalog();
    let missing: Vec<String> = load_passengers()
        .iter()
        .flat_map(|p| p.drop_items.iter())
        .filter(|name| !catalog.contains(name))
        .cloned()
        .collect();
    assert!(
        missing.is_empty(),
        "items missing from catalog: {missing:?}"
    );
}

/// The point of the catalog: an item a passenger hands you must do
/// something. Every catalog entry either carries effects, wards you, or
/// curses you — a droppable item is never pure inventory clutter.
#[test]
fn every_catalog_item_does_something() {
    let catalog = load_item_catalog();
    let inert: Vec<String> = catalog
        .names()
        .into_iter()
        .filter(|name| {
            let template = catalog.get(name);
            template.effects.is_empty()
                && template.protective_properties.is_none()
                && template.cursed_properties.is_none()
        })
        .collect();
    assert!(inert.is_empty(), "items with no effect at all: {inert:?}");
}

/// Every curse must tell the player how to be rid of it, or the penalty
/// is something that happens to them with no way to act on it.
#[test]
fn every_curse_names_its_way_out() {
    let catalog = load_item_catalog();
    let mut names = catalog.names();
    names.sort();
    for name in names {
        let Some(curse) = catalog.get(&name).cursed_properties else {
            continue;
        };
        if !curse.can_be_removed {
            continue;
        }
        let condition = curse.removal_condition.unwrap_or_default();
        assert!(
            !condition.trim().is_empty(),
            "{name:?} can be removed but does not say how"
        );
    }
}

/// A curse that refuses to be given away must not also be marked
/// tradeable, or the inventory offers a way out that the trade refuses.
#[test]
fn an_unremovable_curse_is_not_offered_for_trade() {
    let catalog = load_item_catalog();
    let mut names = catalog.names();
    names.sort();
    for name in names {
        let template = catalog.get(&name);
        let Some(curse) = &template.cursed_properties else {
            continue;
        };
        if curse.can_be_removed {
            continue;
        }
        assert!(
            !template.can_trade,
            "{name:?} cannot be removed but is marked canTrade"
        );
    }
}

/// A usable item must actually have effects to apply, or "Use" is a no-op
/// button on an inventory row.
#[test]
fn usable_items_have_effects() {
    let catalog = load_item_catalog();
    let empty: Vec<String> = catalog
        .names()
        .into_iter()
        .filter(|name| {
            let template = catalog.get(name);
            template.can_use && template.effects.is_empty()
        })
        .collect();
    assert!(empty.is_empty(), "usable items with no effects: {empty:?}");
}

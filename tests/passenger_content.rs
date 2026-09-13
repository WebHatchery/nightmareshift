use nightmare_shift::data::{load_item_catalog, load_locations, load_passengers, RouteType};
use std::collections::HashSet;
use std::path::Path;

#[test]
fn expanded_roster_has_complete_authored_records_and_portraits() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let passengers = load_passengers();
    let locations = load_locations();
    let catalog = load_item_catalog();
    let passenger_ids: HashSet<u32> = passengers.iter().map(|passenger| passenger.id).collect();
    let location_names: HashSet<&str> = locations
        .iter()
        .map(|location| location.name.as_str())
        .collect();

    assert!(
        passengers.len() > 16,
        "the authored roster should extend past the original 16"
    );
    assert_eq!(
        passenger_ids.len(),
        passengers.len(),
        "passenger IDs must be unique"
    );

    for passenger in &passengers {
        assert!(passenger.id > 0);
        assert!(!passenger.name.trim().is_empty());
        assert!(!passenger.personal_rule.trim().is_empty());
        assert!(
            !passenger.dialogue.is_empty(),
            "{} has no dialogue",
            passenger.name
        );
        assert!(
            !passenger.tells.is_empty(),
            "{} has no authored tells",
            passenger.name
        );
        assert!(
            root.join(format!("assets/passengers/{}.png", passenger.id))
                .is_file(),
            "{} has no matching portrait",
            passenger.name
        );
        assert!(location_names.contains(passenger.pickup.as_str()));
        assert!(location_names.contains(passenger.destination.as_str()));

        let routes: HashSet<RouteType> = passenger
            .route_preferences
            .iter()
            .map(|preference| preference.route)
            .collect();
        assert_eq!(
            routes.len(),
            4,
            "{} must author all route preferences",
            passenger.name
        );
        assert!(routes.contains(&RouteType::Normal));
        assert!(routes.contains(&RouteType::Shortcut));
        assert!(routes.contains(&RouteType::Scenic));
        assert!(routes.contains(&RouteType::Police));

        for relationship in &passenger.relationships {
            assert!(
                passenger_ids.contains(relationship),
                "{} names unknown passenger {relationship}",
                passenger.name
            );
        }
        for item in passenger
            .drop_items
            .iter()
            .chain(passenger.wanted_items.iter())
        {
            assert!(
                catalog.contains(item),
                "{} names unknown item {item}",
                passenger.name
            );
        }
    }
}

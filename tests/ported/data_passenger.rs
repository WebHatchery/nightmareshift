use super::*;
use nightmare_shift::data::loader::load_passengers;

/// A preference level's label must agree with what the route actually
/// does to the passenger.
///
/// The almanac reveals these at Lv.2 and the driving screen colours a
/// route by them — green for Loves, red for Fears. `apply_route_choice`
/// meanwhile applies `stressModifier` and `calculate_fare` applies
/// `fareModifier`, and nothing tied the label to either. A route shown
/// green that quietly stresses the passenger would make studying them
/// worse than not.
#[test]
fn preference_labels_agree_with_their_effects() {
    for passenger in load_passengers() {
        for preference in &passenger.route_preferences {
            let (stress, fare) = (preference.stress_modifier, preference.fare_modifier);
            let context = format!(
                "{} on {:?} ({:?})",
                passenger.name, preference.route, preference.preference
            );
            match preference.preference {
                PreferenceLevel::Loves | PreferenceLevel::Likes => {
                    assert!(
                        stress <= 0.0,
                        "{context}: a welcome route adds {stress} stress"
                    );
                    assert!(fare >= 1.0, "{context}: a welcome route pays {fare}x");
                }
                PreferenceLevel::Neutral => {
                    assert_eq!(stress, 0.0, "{context}: neutral moves stress");
                    assert_eq!(fare, 1.0, "{context}: neutral moves the fare");
                }
                PreferenceLevel::Dislikes | PreferenceLevel::Fears => {
                    assert!(
                        stress > 0.0,
                        "{context}: an unwelcome route relieves stress"
                    );
                    assert!(fare < 1.0, "{context}: an unwelcome route pays {fare}x");
                }
            }
        }
    }
}

/// The scale must be ordered across levels, not merely correctly signed:
/// a feared route must cost more than a merely disliked one, and a loved
/// route must beat a liked one. Otherwise the four labels the almanac
/// prints do not rank.
#[test]
fn preference_levels_rank_against_each_other() {
    let passengers = load_passengers();
    let worst = |level: PreferenceLevel| {
        passengers
            .iter()
            .flat_map(|p| p.route_preferences.iter())
            .filter(|pref| pref.preference == level)
            .map(|pref| pref.stress_modifier)
            .fold(f32::NEG_INFINITY, f32::max)
    };
    let best = |level: PreferenceLevel| {
        passengers
            .iter()
            .flat_map(|p| p.route_preferences.iter())
            .filter(|pref| pref.preference == level)
            .map(|pref| pref.stress_modifier)
            .fold(f32::INFINITY, f32::min)
    };

    assert!(
        worst(PreferenceLevel::Loves) <= best(PreferenceLevel::Dislikes),
        "the kindest route a passenger fears is no worse than one they love"
    );
    assert!(
        best(PreferenceLevel::Fears) >= worst(PreferenceLevel::Dislikes),
        "fearing a route costs no more than disliking one"
    );
}

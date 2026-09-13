use super::*;
use nightmare_shift::data::loader::load_constants;

/// The shift clock must actually reach the dark. It used to run on wall
/// time — one real hour per fiction hour — so Night, Latenight, and
/// everything keyed to them (rule 104, the checkpoint hazard, the night
/// risk bonus, night spawn weights) sat unreachable behind a two-hour
/// play session.
#[test]
fn a_full_shift_reaches_night_and_the_small_hours() {
    assert_eq!(
        WeatherService::time_of_day_after(0).phase,
        TimePhase::Dusk,
        "the shift clocks on at dusk"
    );
    assert_eq!(
        WeatherService::time_of_day_after(120).phase,
        TimePhase::Night,
        "two shift-hours in, it is night"
    );
    assert_eq!(
        WeatherService::time_of_day_after(360).phase,
        TimePhase::Latenight,
        "six shift-hours in, it is the small hours"
    );

    // And the authored shift length spans all of that: the full clock
    // must end in Latenight, or the phases above are theoretical again.
    let initial = load_constants().game_constants.initial_time;
    assert_eq!(
        WeatherService::time_of_day_after(initial).phase,
        TimePhase::Latenight,
        "a spent shift clock ({initial} minutes) should end deep in the night"
    );
}

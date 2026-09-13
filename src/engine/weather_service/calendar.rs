//! Time of day and season: deriving the current phase and season, and the
//! rules the current conditions put in force.

use crate::data::*;

use super::WeatherService;

impl WeatherService {
    /// The hour the night shift clocks on. An eight-hour shift from here
    /// runs 18:00 to 02:00 — dusk, through night, into the small hours.
    pub const SHIFT_START_HOUR: u32 = 18;

    /// Get time of day from hour
    pub fn get_time_of_day(hour: u32) -> TimeOfDay {
        let (phase, description, ambient_light, supernatural_activity) = match hour {
            6..=7 => (
                TimePhase::Dawn,
                "The sky lightens as dawn approaches",
                30,
                70,
            ),
            8..=11 => (
                TimePhase::Morning,
                "Morning light fills the streets",
                85,
                20,
            ),
            12..=16 => (TimePhase::Afternoon, "Bright afternoon sunlight", 100, 10),
            17..=19 => (
                TimePhase::Dusk,
                "The sun sets, casting long shadows",
                40,
                60,
            ),
            20..=23 => (TimePhase::Night, "Darkness settles over the city", 15, 85),
            _ => (
                TimePhase::Latenight,
                "The deepest part of the night",
                5,
                100,
            ),
        };

        TimeOfDay {
            phase,
            hour,
            description: description.to_string(),
            ambient_light,
            supernatural_activity,
        }
    }

    /// Time of day after `minutes_elapsed` on the shift clock.
    ///
    /// The clock runs on shift minutes — the ones routes spend — not
    /// wall-clock seconds. The old version divided real elapsed seconds by
    /// 3600, so a player needed two hours at the keyboard to see 20:00:
    /// Night and Latenight, and everything keyed to them, never happened.
    pub fn time_of_day_after(minutes_elapsed: u32) -> TimeOfDay {
        let hour = (Self::SHIFT_START_HOUR + minutes_elapsed / 60) % 24;
        Self::get_time_of_day(hour)
    }

    /// Get current season from month
    pub fn get_current_season(month: u32) -> Season {
        let (season_type, temperature) = match month {
            3..=5 => (
                SeasonType::Spring,
                if month == 3 {
                    Temperature::Cool
                } else if month == 4 {
                    Temperature::Mild
                } else {
                    Temperature::Warm
                },
            ),
            6..=8 => (
                SeasonType::Summer,
                if month == 6 {
                    Temperature::Warm
                } else {
                    Temperature::Hot
                },
            ),
            9..=11 => (
                SeasonType::Fall,
                if month == 9 {
                    Temperature::Warm
                } else if month == 10 {
                    Temperature::Cool
                } else {
                    Temperature::Cold
                },
            ),
            _ => (SeasonType::Winter, Temperature::Cold),
        };

        let description = match season_type {
            SeasonType::Spring => format!(
                "Spring weather brings {:?} temperatures and frequent changes",
                temperature
            ),
            SeasonType::Summer => format!(
                "Summer heat creates {:?} conditions perfect for night driving",
                temperature
            ),
            SeasonType::Fall => format!(
                "Autumn's {:?} weather brings unpredictable conditions",
                temperature
            ),
            SeasonType::Winter => format!(
                "Winter's {:?} temperatures make every drive challenging",
                temperature
            ),
        };

        Season {
            season_type,
            description,
        }
    }

    /// Get weather-triggered rule IDs
    pub fn get_weather_triggered_rules(
        weather: &WeatherCondition,
        time_of_day: &TimeOfDay,
    ) -> Vec<u32> {
        let mut triggered = Vec::new();

        if weather.weather_type == WeatherType::Thunderstorm {
            triggered.push(101); // Don't use windshield wipers during thunderstorms
        }

        if weather.weather_type == WeatherType::Fog && weather.intensity == WeatherIntensity::Heavy
        {
            triggered.push(102); // Keep headlights on during heavy fog
        }

        if weather.weather_type == WeatherType::Snow {
            triggered.push(103); // Drive under 25 mph in snow
        }

        if time_of_day.phase == TimePhase::Latenight && weather.weather_type != WeatherType::Clear {
            triggered.push(104); // No stops during late night bad weather
        }

        if weather.visibility < 30 {
            triggered.push(105); // Don't use AC during low visibility
        }

        if weather.weather_type == WeatherType::Wind && weather.intensity == WeatherIntensity::Heavy
        {
            triggered.push(106); // Keep windows closed during heavy wind
        }

        triggered
    }
}

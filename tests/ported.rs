//! Integration coverage migrated from the legacy in-source test modules.
//!
//! The production crate exposes only the seams these tests intentionally
//! exercise. Keeping the cases in one integration harness makes the public
//! API boundary obvious while allowing each feature suite to keep its own
//! descriptive test names.

pub use nightmare_shift::audio::*;
pub use nightmare_shift::data::*;
pub use nightmare_shift::engine::*;
pub use nightmare_shift::game::*;
pub use nightmare_shift::screens::game_screens::*;
pub use nightmare_shift::screens::meta_screens::*;
pub use nightmare_shift::screens::*;
pub use nightmare_shift::state::*;
pub use nightmare_shift::ui::*;

#[path = "ported/data_constants.rs"]
mod data_constants;
#[path = "ported/data_constants_campaign.rs"]
mod data_constants_campaign;
#[path = "ported/data_environment.rs"]
mod data_environment;
#[path = "ported/data_epilogue.rs"]
mod data_epilogue;
#[path = "ported/data_event.rs"]
mod data_event;
#[path = "ported/data_inventory.rs"]
mod data_inventory;
#[path = "ported/data_loader.rs"]
mod data_loader;
#[path = "ported/data_location.rs"]
mod data_location;
#[path = "ported/data_night_modifier.rs"]
mod data_night_modifier;
#[path = "ported/data_passenger.rs"]
mod data_passenger;
#[path = "ported/data_reward.rs"]
mod data_reward;
#[path = "ported/data_skill_tree.rs"]
mod data_skill_tree;
#[path = "ported/engine_game_engine.rs"]
mod engine_game_engine;
#[path = "ported/engine_guideline_engine.rs"]
mod engine_guideline_engine;
#[path = "ported/engine_input_service.rs"]
mod engine_input_service;
#[path = "ported/engine_item_service.rs"]
mod engine_item_service;
#[path = "ported/engine_passenger_service.rs"]
mod engine_passenger_service;
#[path = "ported/engine_passenger_state_machine.rs"]
mod engine_passenger_state_machine;
#[path = "ported/engine_protection_service.rs"]
mod engine_protection_service;
#[path = "ported/engine_ride_events.rs"]
mod engine_ride_events;
#[path = "ported/engine_ride_risk.rs"]
mod engine_ride_risk;
#[path = "ported/engine_ride_route_choice.rs"]
mod engine_ride_route_choice;
#[path = "ported/engine_route_service.rs"]
mod engine_route_service;
#[path = "ported/engine_route_service_fare.rs"]
mod engine_route_service_fare;
#[path = "ported/engine_rule_modification.rs"]
mod engine_rule_modification;
#[path = "ported/engine_weather_calendar.rs"]
mod engine_weather_calendar;
#[path = "ported/engine_weather_conditions.rs"]
mod engine_weather_conditions;
#[path = "ported/game_delete.rs"]
mod game_delete;
#[path = "ported/game_shift.rs"]
mod game_shift;
#[path = "ported/screens_dossier.rs"]
mod screens_dossier;
#[path = "ported/screens_skill_tree.rs"]
mod screens_skill_tree;
#[path = "ported/state_game_state_reputation.rs"]
mod state_game_state_reputation;
#[path = "ported/state_game_state_score.rs"]
mod state_game_state_score;
#[path = "ported/state_game_state_stage.rs"]
mod state_game_state_stage;
#[path = "ported/state_game_state_trust.rs"]
mod state_game_state_trust;
#[path = "ported/state_persistence.rs"]
mod state_persistence;
#[path = "ported/state_player_stats.rs"]
mod state_player_stats;

#[path = "ported/audio.rs"]
mod audio_cases;
#[path = "ported/key_bindings.rs"]
mod key_binding_cases;

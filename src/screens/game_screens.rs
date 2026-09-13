//! Game phase screens: Waiting, Driving, Interaction, DropOff, Guidelines.
//!
//! These screens are shown during active gameplay. Each phase lives in its own
//! module; this file only routes the current phase to its renderer.

pub mod dossier;
pub mod driving;
pub mod dropoff;
pub mod guidelines;
pub mod interaction;
pub mod modals;
pub mod ride_request;
mod scene;
pub mod waiting;

pub use dossier::{build, need_label, stage_phrase, DossierLine, DriverContext};
pub use driving::draw_driving;
pub use dropoff::draw_dropoff;
pub use guidelines::draw_guideline_decision;
pub use interaction::draw_interaction;
pub use modals::{draw_inventory_modal, draw_rules_panel};
pub use ride_request::draw_ride_request;
pub use waiting::draw_waiting;

use crate::data::GameData;
use crate::state::{GamePhase, GameState};
use crate::ui::UiAction;

/// Route the active game phase to its screen renderer.
pub fn draw_game(
    game_state: &GameState,
    game_data: Option<&GameData>,
    player_stats: &crate::state::PlayerStats,
) -> UiAction {
    match game_state.game_phase {
        GamePhase::Waiting => draw_waiting(game_state, game_data, player_stats),
        GamePhase::RideRequest => draw_ride_request(game_state, game_data, player_stats),
        GamePhase::Driving => draw_driving(game_state, game_data, player_stats),
        GamePhase::Interaction => draw_interaction(game_state),
        GamePhase::DropOff => draw_dropoff(game_state, game_data),
        GamePhase::GuidelineDecision => {
            draw_guideline_decision(game_state, game_data, player_stats)
        }
        _ => UiAction::None,
    }
}

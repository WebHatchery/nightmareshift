//! Menu screens: Loading, Main Menu, Briefing, Credits, Game Over, Success.
//!
//! Each screen lives in its own module; this file re-exports them so callers
//! keep addressing `screens::menu_screens::draw_*`.

pub mod briefing;
pub mod credits;
pub mod help_options;
pub mod loading;
pub mod main_menu;
pub mod outcome;
mod widgets;

pub use briefing::draw_briefing;
pub use credits::draw_credits;
pub use help_options::draw_help_options;
pub use loading::draw_loading;
pub use main_menu::draw_main_menu;
pub use outcome::{draw_game_over, draw_success};

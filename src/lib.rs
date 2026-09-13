//! Public game modules used by the executable and integration tests.
//!
//! Keeping the game behind a library boundary makes the simulation and its
//! data services testable without requiring the Macroquad entry point.

pub mod audio;
pub mod bot;
pub mod data;
pub mod engine;
pub mod game;
pub mod screens;
pub mod state;
pub mod ui;

pub use game::Game;

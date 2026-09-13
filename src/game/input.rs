//! Input capture and playtest-bot coordination for the active game.

use macroquad::prelude::*;

use super::Game;
use crate::bot::PlaytestDirective;
use crate::engine::{InputService, Overlay};
use crate::screens::Screen;
use crate::ui::UiAction;

impl Game {
    /// Which modal overlay is eating input this frame, if any. The pause
    /// menu wins over the panels because opening it closes them.
    pub(crate) fn active_overlay(&self) -> Overlay {
        if self.screen != Screen::Game {
            Overlay::None
        } else if self.overlays.pause {
            Overlay::Pause
        } else if self.overlays.rules || self.overlays.inventory {
            Overlay::Panel
        } else {
            Overlay::None
        }
    }

    /// Capture keyboard input and dispatch it as intents.
    pub fn handle_input(&mut self) {
        if self.screen == Screen::MainMenu && self.seed_entry.is_some() {
            while let Some(ch) = get_char_pressed() {
                self.handle_ui_action(UiAction::SeedDigit(ch));
            }
            if is_key_pressed(KeyCode::Backspace) {
                self.handle_ui_action(UiAction::EraseSeedDigit);
            }
            if is_key_pressed(KeyCode::Escape) {
                self.handle_ui_action(UiAction::CancelSeedEntry);
            }
            if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
                if let Some(seed) = self
                    .seed_entry
                    .as_deref()
                    .and_then(|value| value.parse::<u64>().ok())
                {
                    self.handle_ui_action(UiAction::StartSeededRun(seed));
                }
            }
            return;
        }
        let mut actions = InputService::capture_input_with_bindings(
            self.screen,
            self.game_state.game_phase,
            self.active_overlay(),
            &self.player_stats.accessibility.key_bindings,
        );
        let gamepad = self.gamepad.capture();
        actions.extend(InputService::capture_gamepad(
            self.screen,
            self.game_state.game_phase,
            self.active_overlay(),
            gamepad,
            self.resume_available,
        ));
        for action in actions {
            self.handle_ui_action(action);
        }
    }

    /// Let the optional playtest bot drive game actions.
    pub fn handle_playtest_bot(&mut self) {
        let directive = if let Some(bot) = self.playtest_bot.as_mut() {
            bot.next_action(
                self.screen,
                &self.game_state,
                &self.player_stats,
                self.game_data.as_ref(),
                get_time(),
            )
        } else {
            PlaytestDirective::None
        };

        match directive {
            PlaytestDirective::None => {}
            PlaytestDirective::Action(action) => self.handle_ui_action(action),
            PlaytestDirective::Stop(code) => {
                #[cfg(not(target_arch = "wasm32"))]
                std::process::exit(code);
                #[cfg(target_arch = "wasm32")]
                let _ = code;
            }
        }
    }
}

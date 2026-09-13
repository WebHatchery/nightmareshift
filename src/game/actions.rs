//! Dispatches `UiAction`s produced by the draw phase into game mutations.

use super::Game;
use crate::data::RouteType;
use crate::engine::*;
use crate::screens::Screen;
use crate::state::*;
use crate::ui::UiAction;

fn cycle_volume(value: u8) -> u8 {
    match value {
        0..=25 => 50,
        26..=50 => 80,
        51..=80 => 100,
        _ => 0,
    }
}

impl Game {
    /// Route a UI action to the focused state owner.
    pub fn handle_ui_action(&mut self, action: UiAction) {
        self.play_action_feedback(&action);
        if self.handle_menu_action(&action) {
            return;
        }
        if self.handle_ride_action(&action) {
            return;
        }
        if self.handle_accessibility_action(&action) {
            return;
        }
        self.handle_meta_action(&action);
    }

    fn play_action_feedback(&mut self, action: &UiAction) {
        let feedback = matches!(
            action,
            UiAction::StartGame
                | UiAction::ResumeRun
                | UiAction::AcceptRide
                | UiAction::SelectRoute(_)
                | UiAction::SelectEventChoice(_)
                | UiAction::Continue
                | UiAction::FollowGuideline
                | UiAction::BreakGuideline
                | UiAction::OpenCredits
        );
        if !matches!(action, UiAction::None) {
            self.audio
                .play_ui_feedback(feedback, &self.player_stats.accessibility);
        }
    }

    fn handle_menu_action(&mut self, action: &UiAction) -> bool {
        match action {
            UiAction::StartGame => self.start_or_brief_game(),
            UiAction::OpenSeedEntry => {
                if self.screen == Screen::MainMenu {
                    self.seed_entry = Some(String::new());
                }
                true
            }
            UiAction::StartDailyRun => {
                if self.screen == Screen::MainMenu {
                    self.menu_seed = Some(Self::daily_seed());
                    self.start_game();
                }
                true
            }
            UiAction::StartSeededRun(seed) => {
                if self.screen == Screen::MainMenu {
                    self.menu_seed = Some(*seed);
                    self.start_game();
                }
                true
            }
            UiAction::SeedDigit(ch) => {
                self.append_seed_digit(*ch);
                true
            }
            UiAction::EraseSeedDigit => {
                self.seed_entry.as_mut().map(String::pop);
                true
            }
            UiAction::CancelSeedEntry => {
                self.seed_entry = None;
                true
            }
            UiAction::ReturnToMenu => {
                self.return_from_menu_action();
                true
            }
            UiAction::TryAgain => {
                self.try_again_from_outcome();
                true
            }
            UiAction::NextNight => {
                if self.screen == Screen::Success && !self.game_state.run_complete {
                    self.advance_night();
                }
                true
            }
            UiAction::ResumeRun => {
                if self.screen == Screen::MainMenu && self.resume_available {
                    self.screen = Screen::Game;
                    self.overlays.close_all();
                }
                true
            }
            UiAction::OpenSkillTree => {
                self.change_screen(Screen::SkillTree);
                true
            }
            UiAction::OpenAlmanac => {
                self.change_screen(Screen::Almanac);
                true
            }
            UiAction::OpenLeaderboard => {
                self.change_screen(Screen::Leaderboard);
                true
            }
            UiAction::OpenHelpOptions => {
                self.help_return_screen = if self.screen == Screen::Game {
                    Screen::Game
                } else {
                    Screen::MainMenu
                };
                self.screen = Screen::HelpOptions;
                true
            }
            UiAction::OpenCredits => {
                if self.screen == Screen::MainMenu {
                    self.change_screen(Screen::Credits);
                }
                true
            }
            UiAction::DeleteSave => {
                self.arm_or_delete_save();
                true
            }
            _ => false,
        }
    }

    fn start_or_brief_game(&mut self) -> bool {
        if self.screen == Screen::MainMenu && self.seed_entry.is_none() {
            self.menu_seed = None;
            self.start_game();
        } else if self.screen == Screen::Briefing {
            self.start_shift();
        }
        true
    }

    fn append_seed_digit(&mut self, ch: char) {
        if self.screen != Screen::MainMenu || !ch.is_ascii_digit() {
            return;
        }
        let Some(digits) = self.seed_entry.as_mut() else {
            return;
        };
        if digits.len() < 19 {
            digits.push(ch);
        }
    }

    fn return_from_menu_action(&mut self) {
        if self.screen == Screen::HelpOptions {
            if self.tutorial_active {
                self.player_stats.tutorial_completed = true;
                self.tutorial_active = false;
                self.save_stats();
            }
            self.screen = self.help_return_screen;
            if self.screen == Screen::MainMenu {
                self.overlays.close_all();
            }
            return;
        }

        let can_leave = matches!(
            self.screen,
            Screen::GameOver
                | Screen::Success
                | Screen::SkillTree
                | Screen::Almanac
                | Screen::Leaderboard
                | Screen::Credits
                | Screen::Briefing
        ) || (self.screen == Screen::Game && self.overlays.pause);
        if can_leave {
            if self.screen == Screen::Game && self.overlays.pause {
                self.resume_available = false;
                self.save_stats();
            }
            self.overlays.close_all();
            self.return_to_menu();
        }
    }

    fn try_again_from_outcome(&mut self) {
        if self.screen == Screen::Success && !self.game_state.run_complete {
            self.advance_night();
        } else if self.screen == Screen::GameOver || self.screen == Screen::Success {
            self.return_to_menu();
            self.start_game();
        }
    }

    fn handle_ride_action(&mut self, action: &UiAction) -> bool {
        match action {
            UiAction::AcceptRide => {
                if self.screen == Screen::Game {
                    self.accept_ride();
                }
            }
            UiAction::DeclineRide => {
                if self.screen == Screen::Game {
                    self.decline_ride();
                }
            }
            UiAction::SelectRoute(index) => self.select_route_action(*index),
            UiAction::SelectEventChoice(index) => {
                if self.screen == Screen::Game {
                    RideService::resolve_event_choice(&mut self.game_state, *index);
                }
            }
            UiAction::Continue => self.continue_action(),
            UiAction::EndShift => self.end_shift_action(),
            UiAction::RefuelFull => self.refuel_action(true),
            UiAction::RefuelPartial => self.refuel_action(false),
            UiAction::ToggleRules => {
                if self.screen == Screen::Game {
                    self.overlays.rules = !self.overlays.rules;
                }
            }
            UiAction::ToggleInventory => {
                if self.screen == Screen::Game {
                    self.overlays.inventory = !self.overlays.inventory;
                }
            }
            UiAction::TogglePauseMenu => self.toggle_pause_action(),
            UiAction::UseItem(index) => {
                if self.screen == Screen::Game && *index < self.game_state.inventory.len() {
                    self.use_item(*index);
                }
            }
            UiAction::PerformRuleAction(action_key) => {
                if self.screen == Screen::Game {
                    self.perform_rule_action(action_key.clone());
                }
            }
            UiAction::AcceptTrade(index) => {
                if self.screen == Screen::Game && self.game_state.game_phase == GamePhase::DropOff {
                    self.complete_trade(*index);
                }
            }
            UiAction::DeclineTrade => {
                if self.screen == Screen::Game && self.game_state.game_phase == GamePhase::DropOff {
                    self.game_state.pending_trade = None;
                }
            }
            UiAction::FollowGuideline => self.decide_guideline(GuidelineAction::Follow),
            UiAction::BreakGuideline => self.decide_guideline(GuidelineAction::Break),
            _ => return false,
        }
        true
    }

    fn select_route_action(&mut self, index: usize) {
        let route = match index {
            0 => RouteType::Normal,
            1 => RouteType::Shortcut,
            2 => RouteType::Scenic,
            3 => RouteType::Police,
            _ => RouteType::Normal,
        };
        if self.screen == Screen::Game && self.game_state.game_phase == GamePhase::Driving {
            self.select_route(route);
        }
    }

    fn continue_action(&mut self) {
        if self.screen != Screen::Game {
            return;
        }
        match self.game_state.game_phase {
            GamePhase::Waiting => self.spawn_passenger(),
            GamePhase::Interaction if self.event_needs_an_answer() => {}
            GamePhase::Interaction => self.continue_past_event(),
            GamePhase::DropOff if self.game_state.pending_trade.is_none() => {
                self.continue_from_dropoff();
            }
            _ => {}
        }
    }

    fn event_needs_an_answer(&self) -> bool {
        self.game_state
            .current_event
            .as_ref()
            .is_some_and(|event| !event.choices.is_empty())
    }

    fn end_shift_action(&mut self) {
        if self.screen == Screen::Game
            && self.game_state.game_phase == GamePhase::Waiting
            && self.game_state.earnings >= self.game_state.minimum_earnings
            && !self.game_state.last_fare_night
        {
            self.end_shift(true);
        }
    }

    fn refuel_action(&mut self, full: bool) {
        if self.screen != Screen::Game
            || self.game_state.game_phase != GamePhase::Waiting
            || self.game_state.last_fare_night
        {
            return;
        }
        if full {
            self.refuel_full();
        } else {
            self.refuel_partial();
        }
    }

    fn toggle_pause_action(&mut self) {
        if self.screen != Screen::Game {
            return;
        }
        self.overlays.pause = !self.overlays.pause;
        if self.overlays.pause {
            self.overlays.rules = false;
            self.overlays.inventory = false;
            self.save_run_checkpoint();
        }
    }

    fn decide_guideline(&mut self, action: GuidelineAction) {
        if self.game_state.game_phase == GamePhase::GuidelineDecision {
            self.evaluate_guideline_decision(action);
        }
    }

    fn handle_accessibility_action(&mut self, action: &UiAction) -> bool {
        let accessibility = &mut self.player_stats.accessibility;
        match action {
            UiAction::CycleAcceptBinding => accessibility.key_bindings.cycle_accept(),
            UiAction::CycleDeclineBinding => accessibility.key_bindings.cycle_decline(),
            UiAction::CycleFollowBinding => accessibility.key_bindings.cycle_follow(),
            UiAction::CycleBreakBinding => accessibility.key_bindings.cycle_break(),
            UiAction::CyclePauseBinding => accessibility.key_bindings.cycle_pause(),
            UiAction::CycleTextScale => accessibility.cycle_text_scale(),
            UiAction::ToggleHighContrast => {
                accessibility.high_contrast = !accessibility.high_contrast
            }
            UiAction::ToggleReducedMotion => {
                accessibility.reduced_motion = !accessibility.reduced_motion
            }
            UiAction::CycleBrightness => accessibility.cycle_brightness(),
            UiAction::ToggleCaptions => accessibility.captions = !accessibility.captions,
            UiAction::ToggleFullscreen => {
                accessibility.fullscreen = !accessibility.fullscreen;
                macroquad::window::set_fullscreen(accessibility.fullscreen);
            }
            UiAction::CycleMasterVolume => accessibility.cycle_master_volume(),
            UiAction::CycleAmbienceVolume => {
                accessibility.ambience_volume = cycle_volume(accessibility.ambience_volume)
            }
            UiAction::CycleMusicVolume => {
                accessibility.music_volume = cycle_volume(accessibility.music_volume)
            }
            UiAction::CycleEffectsVolume => {
                accessibility.effects_volume = cycle_volume(accessibility.effects_volume)
            }
            UiAction::CycleLanguage => {
                accessibility.cycle_language();
                self.reload_localization();
            }
            _ => return false,
        }
        self.save_stats();
        true
    }

    fn reload_localization(&mut self) {
        let code = self.player_stats.accessibility.language.clone();
        if let Some(data) = self.game_data.as_mut() {
            if let Ok(localization) = crate::data::loader::try_load_localization_for(&code) {
                data.localization = localization;
            }
        }
    }

    fn handle_meta_action(&mut self, action: &UiAction) -> bool {
        match action {
            UiAction::PurchaseSkill(skill_id) => self.purchase_skill_action(skill_id),
            UiAction::ExchangeLoreForBank => self.exchange_lore_action(),
            UiAction::UpgradeAlmanacKnowledge(passenger_id) => {
                self.upgrade_almanac_action(*passenger_id)
            }
            UiAction::SelectSkillCategory(category) => {
                self.skill_tree_category = (*category).min(3);
                self.skill_tree_selected = None;
                self.skill_tree_scroll.set_offset(0.0);
                true
            }
            UiAction::SelectSkill(skill_id) => {
                self.skill_tree_selected = Some(skill_id.clone());
                true
            }
            UiAction::SelectAlmanacPassenger(passenger_id) => {
                self.almanac_selected = Some(*passenger_id);
                true
            }
            UiAction::None => true,
            _ => false,
        }
    }

    fn purchase_skill_action(&mut self, skill_id: &str) -> bool {
        let Some(skill) = self
            .game_data
            .as_ref()
            .and_then(|data| data.skills.iter().find(|skill| skill.id == skill_id))
            .cloned()
        else {
            return true;
        };
        if self.player_stats.purchase_skill(&skill.id, skill.cost) {
            let unlocked = self.player_stats.check_achievements(None);
            self.pay_achievement_rewards(&unlocked);
            self.save_stats();
        }
        true
    }

    fn exchange_lore_action(&mut self) -> bool {
        let Some(rate) = self
            .game_data
            .as_ref()
            .map(|data| data.rewards.lore_exchange)
        else {
            return true;
        };
        if rate.is_available()
            && self
                .player_stats
                .exchange_lore_for_bank(rate.lore, rate.bank)
        {
            self.save_stats();
        }
        true
    }

    fn upgrade_almanac_action(&mut self, passenger_id: u32) -> bool {
        let Some(data) = self.game_data.as_ref() else {
            return true;
        };
        let cost = {
            let current = self
                .player_stats
                .get_almanac_entry(passenger_id)
                .knowledge_level;
            data.almanac.get_upgrade_cost(current + 1)
        };
        if self
            .player_stats
            .upgrade_almanac_knowledge(passenger_id, cost)
        {
            let unlocked = self.player_stats.check_achievements(None);
            self.pay_achievement_rewards(&unlocked);
            self.save_stats();
        }
        true
    }
}

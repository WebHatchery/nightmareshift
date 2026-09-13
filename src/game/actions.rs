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
    /// Handle UI actions from draw phase
    pub fn handle_ui_action(&mut self, action: UiAction) {
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
        );
        if !matches!(action, UiAction::None) {
            self.audio
                .play_ui_feedback(feedback, &self.player_stats.accessibility);
        }
        match action {
            UiAction::StartGame => {
                // With the seed modal open, Space is typing, not starting.
                if self.screen == Screen::MainMenu && self.seed_entry.is_none() {
                    // A plain start is a fresh deal: whatever seed the menu
                    // chose for a previous run does not carry over.
                    self.menu_seed = None;
                    self.start_game();
                } else if self.screen == Screen::Briefing {
                    self.start_shift();
                }
            }
            UiAction::OpenSeedEntry => {
                if self.screen == Screen::MainMenu {
                    self.seed_entry = Some(String::new());
                }
            }
            UiAction::StartDailyRun => {
                if self.screen == Screen::MainMenu {
                    self.menu_seed = Some(Self::daily_seed());
                    self.start_game();
                }
            }
            UiAction::StartSeededRun(seed) => {
                if self.screen == Screen::MainMenu {
                    self.menu_seed = Some(seed);
                    self.start_game();
                }
            }
            UiAction::SeedDigit(ch) => {
                if self.screen == Screen::MainMenu
                    && self.seed_entry.is_some()
                    && ch.is_ascii_digit()
                {
                    let digits = self.seed_entry.as_mut().expect("seed entry checked");
                    if digits.len() < 19 {
                        digits.push(ch);
                    }
                }
            }
            UiAction::EraseSeedDigit => {
                if let Some(digits) = self.seed_entry.as_mut() {
                    digits.pop();
                }
            }
            UiAction::CancelSeedEntry => {
                self.seed_entry = None;
            }
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
            UiAction::SelectRoute(idx) => {
                let route_type = match idx {
                    0 => RouteType::Normal,
                    1 => RouteType::Shortcut,
                    2 => RouteType::Scenic,
                    3 => RouteType::Police,
                    _ => RouteType::Normal,
                };
                if self.screen == Screen::Game && self.game_state.game_phase == GamePhase::Driving {
                    self.select_route(route_type);
                }
            }
            UiAction::SelectEventChoice(idx) => {
                if self.screen == Screen::Game {
                    RideService::resolve_event_choice(&mut self.game_state, idx);
                }
            }
            UiAction::Continue => {
                if self.screen == Screen::Game {
                    match self.game_state.game_phase {
                        GamePhase::Waiting => self.spawn_passenger(),
                        GamePhase::Interaction => {
                            // With choices on screen, SPACE is not an answer:
                            // skipping applied no consequence and completed
                            // the ride a leg early.
                            let awaiting_choice = self
                                .game_state
                                .current_event
                                .as_ref()
                                .is_some_and(|event| !event.choices.is_empty());
                            if !awaiting_choice {
                                self.continue_past_event();
                            }
                        }
                        // An open trade offer must be answered, not skipped —
                        // Continue used to decline it silently.
                        GamePhase::DropOff if self.game_state.pending_trade.is_none() => {
                            self.continue_from_dropoff();
                        }
                        _ => {}
                    }
                }
            }
            UiAction::ReturnToMenu => {
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
                } else if self.screen == Screen::GameOver
                    || self.screen == Screen::Success
                    || self.screen == Screen::SkillTree
                    || self.screen == Screen::Almanac
                    || self.screen == Screen::Leaderboard
                    || self.screen == Screen::Briefing
                    || (self.screen == Screen::Game && self.overlays.pause)
                {
                    if self.screen == Screen::Game && self.overlays.pause {
                        // Abandoning from a pause menu deliberately discards
                        // the checkpoint and writes only meta-progression.
                        self.resume_available = false;
                        self.save_stats();
                    }
                    // All of them, not just the pause menu: an open binder
                    // used to trail the player to the menu and reappear
                    // over the next shift.
                    self.overlays.close_all();
                    self.return_to_menu();
                }
            }
            UiAction::TryAgain => {
                // On an interim-night results screen the confirm key presses
                // on into the next night rather than starting the run over.
                if self.screen == Screen::Success && !self.game_state.run_complete {
                    self.advance_night();
                } else if self.screen == Screen::GameOver || self.screen == Screen::Success {
                    self.return_to_menu();
                    self.start_game();
                }
            }
            UiAction::NextNight => {
                if self.screen == Screen::Success && !self.game_state.run_complete {
                    self.advance_night();
                }
            }
            UiAction::EndShift => {
                // No cashing out on The Last Fare — the night ends when Death
                // reaches his door, one way or the other.
                if self.screen == Screen::Game
                    && self.game_state.game_phase == GamePhase::Waiting
                    && self.game_state.earnings >= self.game_state.minimum_earnings
                    && !self.game_state.last_fare_night
                {
                    self.end_shift(true);
                }
            }
            UiAction::RefuelFull => {
                if self.screen == Screen::Game
                    && self.game_state.game_phase == GamePhase::Waiting
                    && !self.game_state.last_fare_night
                {
                    self.refuel_full();
                }
            }
            UiAction::RefuelPartial => {
                if self.screen == Screen::Game
                    && self.game_state.game_phase == GamePhase::Waiting
                    && !self.game_state.last_fare_night
                {
                    self.refuel_partial();
                }
            }
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
            UiAction::TogglePauseMenu => {
                if self.screen == Screen::Game {
                    self.overlays.pause = !self.overlays.pause;
                    // Close other overlays when opening pause menu
                    if self.overlays.pause {
                        self.overlays.rules = false;
                        self.overlays.inventory = false;
                        self.save_run_checkpoint();
                    }
                }
            }
            UiAction::ResumeRun => {
                if self.screen == Screen::MainMenu && self.resume_available {
                    self.screen = Screen::Game;
                    self.overlays.close_all();
                }
            }
            UiAction::ExportSave => {
                let checkpoint = self.resume_available.then_some(&self.game_state);
                match Persistence::export_save(&self.player_stats, checkpoint) {
                    Ok(()) => {
                        self.save_notice = Some("Save backup exported successfully.".to_string())
                    }
                    Err(error) => {
                        self.save_notice = Some(format!("Could not export save: {error}"))
                    }
                }
            }
            UiAction::ImportSave => match Persistence::import_save() {
                Ok(save_data) => {
                    self.player_stats = save_data.player_stats;
                    self.player_stats.init_achievements();
                    self.game_state =
                        save_data.run.map(|run| run.game_state).unwrap_or_else(|| {
                            let constants = self
                                .game_data
                                .as_ref()
                                .map(|data| data.constants.game_constants.clone())
                                .unwrap_or_default();
                            GameState::new(self.game_state.simulation_time, &constants)
                        });
                    self.resume_available = self.game_state.game_phase != GamePhase::Loading;
                    self.save_notice = Some("Save backup imported successfully.".to_string());
                    self.save_stats();
                }
                Err(error) => self.save_notice = Some(format!("Could not import save: {error}")),
            },
            UiAction::CycleAcceptBinding => {
                self.player_stats.accessibility.key_bindings.cycle_accept();
                self.save_stats();
            }
            UiAction::CycleDeclineBinding => {
                self.player_stats.accessibility.key_bindings.cycle_decline();
                self.save_stats();
            }
            UiAction::CycleFollowBinding => {
                self.player_stats.accessibility.key_bindings.cycle_follow();
                self.save_stats();
            }
            UiAction::CycleBreakBinding => {
                self.player_stats.accessibility.key_bindings.cycle_break();
                self.save_stats();
            }
            UiAction::CyclePauseBinding => {
                self.player_stats.accessibility.key_bindings.cycle_pause();
                self.save_stats();
            }
            UiAction::UseItem(idx) => {
                if self.screen == Screen::Game && idx < self.game_state.inventory.len() {
                    self.use_item(idx);
                }
            }
            UiAction::PerformRuleAction(action_key) => {
                if self.screen == Screen::Game {
                    self.perform_rule_action(action_key);
                }
            }
            UiAction::AcceptTrade(item_idx) => {
                if self.screen == Screen::Game && self.game_state.game_phase == GamePhase::DropOff {
                    self.complete_trade(item_idx);
                }
            }
            UiAction::DeclineTrade => {
                if self.screen == Screen::Game && self.game_state.game_phase == GamePhase::DropOff {
                    self.game_state.pending_trade = None;
                }
            }
            UiAction::FollowGuideline => {
                if self.game_state.game_phase == GamePhase::GuidelineDecision {
                    self.evaluate_guideline_decision(GuidelineAction::Follow);
                }
            }
            UiAction::BreakGuideline => {
                if self.game_state.game_phase == GamePhase::GuidelineDecision {
                    self.evaluate_guideline_decision(GuidelineAction::Break);
                }
            }
            UiAction::OpenSkillTree => {
                self.change_screen(Screen::SkillTree);
            }
            UiAction::OpenAlmanac => {
                self.change_screen(Screen::Almanac);
            }
            UiAction::OpenLeaderboard => {
                self.change_screen(Screen::Leaderboard);
            }
            UiAction::OpenHelpOptions => {
                self.help_return_screen = if self.screen == Screen::Game {
                    Screen::Game
                } else {
                    Screen::MainMenu
                };
                self.screen = Screen::HelpOptions;
            }
            UiAction::CycleTextScale => {
                self.player_stats.accessibility.cycle_text_scale();
                self.save_stats();
            }
            UiAction::ToggleHighContrast => {
                self.player_stats.accessibility.high_contrast =
                    !self.player_stats.accessibility.high_contrast;
                self.save_stats();
            }
            UiAction::ToggleReducedMotion => {
                self.player_stats.accessibility.reduced_motion =
                    !self.player_stats.accessibility.reduced_motion;
                self.save_stats();
            }
            UiAction::CycleBrightness => {
                self.player_stats.accessibility.cycle_brightness();
                self.save_stats();
            }
            UiAction::ToggleCaptions => {
                self.player_stats.accessibility.captions =
                    !self.player_stats.accessibility.captions;
                self.save_stats();
            }
            UiAction::ToggleFullscreen => {
                self.player_stats.accessibility.fullscreen =
                    !self.player_stats.accessibility.fullscreen;
                macroquad::window::set_fullscreen(self.player_stats.accessibility.fullscreen);
                self.save_stats();
            }
            UiAction::CycleMasterVolume => {
                self.player_stats.accessibility.cycle_master_volume();
                self.save_stats();
            }
            UiAction::CycleAmbienceVolume => {
                self.player_stats.accessibility.ambience_volume =
                    cycle_volume(self.player_stats.accessibility.ambience_volume);
                self.save_stats();
            }
            UiAction::CycleMusicVolume => {
                self.player_stats.accessibility.music_volume =
                    cycle_volume(self.player_stats.accessibility.music_volume);
                self.save_stats();
            }
            UiAction::CycleEffectsVolume => {
                self.player_stats.accessibility.effects_volume =
                    cycle_volume(self.player_stats.accessibility.effects_volume);
                self.save_stats();
            }
            UiAction::DeleteSave => self.arm_or_delete_save(),
            UiAction::PurchaseSkill(skill_id) => {
                if let Some(ref data) = self.game_data {
                    if let Some(skill) = data.skills.iter().find(|s| s.id == skill_id) {
                        if self.player_stats.purchase_skill(&skill.id, skill.cost) {
                            // No shift here: this is the skill tree.
                            let unlocked = self.player_stats.check_achievements(None);
                            self.pay_achievement_rewards(&unlocked);
                            self.save_stats();
                        }
                    }
                }
            }
            UiAction::ExchangeLoreForBank => {
                if let Some(ref data) = self.game_data {
                    let rate = data.rewards.lore_exchange;
                    if rate.is_available()
                        && self
                            .player_stats
                            .exchange_lore_for_bank(rate.lore, rate.bank)
                    {
                        self.save_stats();
                    }
                }
            }
            UiAction::UpgradeAlmanacKnowledge(passenger_id) => {
                if let Some(ref data) = self.game_data {
                    let current_level = self
                        .player_stats
                        .get_almanac_entry(passenger_id)
                        .knowledge_level;
                    let cost = data.almanac.get_upgrade_cost(current_level + 1);
                    if self
                        .player_stats
                        .upgrade_almanac_knowledge(passenger_id, cost)
                    {
                        // No shift here: this is the almanac.
                        let unlocked = self.player_stats.check_achievements(None);
                        self.pay_achievement_rewards(&unlocked);
                        self.save_stats();
                    }
                }
            }
            UiAction::SelectSkillCategory(category) => {
                self.skill_tree_category = category.min(3);
                self.skill_tree_selected = None;
                self.skill_tree_scroll.set_offset(0.0);
            }
            UiAction::SelectSkill(skill_id) => {
                self.skill_tree_selected = Some(skill_id);
            }
            UiAction::SelectAlmanacPassenger(passenger_id) => {
                self.almanac_selected = Some(passenger_id);
            }
            UiAction::None => {}
        }
    }
}

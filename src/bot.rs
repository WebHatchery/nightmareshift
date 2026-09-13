//! Automated playtest bot for smoke-testing the core gameplay loop.
//!
//! What the bot *is* lives here: its state, the screen-to-action dispatch that
//! drives it, the watchdog that notices when a run has stopped moving, and the
//! logging the progression measurements read. How it is *configured* is in
//! [`launch`], and how it *decides* is in [`tactics`].

use crate::data::GameData;
use crate::screens::Screen;
use crate::state::{GamePhase, GameState, PlayerStats};
use crate::ui::UiAction;

mod launch;
mod tactics;

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaytestStrategy {
    Coverage,
    Conservative,
    Learned,
}

impl PlaytestStrategy {
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    fn parse(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "conservative" | "safe" => Self::Conservative,
            "learned" | "smart" => Self::Learned,
            _ => Self::Coverage,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaytestDirective {
    None,
    Action(UiAction),
    Stop(i32),
}

#[derive(Debug)]
pub struct PlaytestBot {
    strategy: PlaytestStrategy,
    max_shifts: u32,
    max_campaigns: Option<u32>,
    completed_shifts: u32,
    completed_campaigns: u32,
    action_delay: f64,
    last_action_time: f64,
    route_cursor: usize,
    event_cursor: usize,
    guideline_cursor: usize,
    decision_count: u32,
    current_shift_logged: bool,
    last_signature: String,
    stale_since: f64,
    almanac_level: u32,
    unlock_all_skills: bool,
    /// Specific skill ids to unlock, for isolating one effect.
    named_skills: Vec<String>,
    /// Start from empty stats and never write the save. Lifetime shift count
    /// drives `suggested_difficulty`, so a measurement that loads the real
    /// save inherits whatever difficulty the file has accumulated — and then
    /// pollutes it further with every shift it plays. Sweeps set this.
    fresh_stats: bool,
    configured_seed: Option<u64>,
    /// Leg index the bot last spent a soothing cab action on.
    soothed_at_leg: Option<usize>,
    /// Rotates the blind guess at which action settles an unstudied fare.
    soothe_cursor: usize,
    /// Ride count when the last *blind* guess was made. One desperation
    /// gamble per ride: pressing a fresh forbidden control every leg chains
    /// ~0.5 death rolls, which no player does after the first spike.
    guessed_at_ride: Option<u32>,
}

impl PlaytestBot {
    /// Whether this run is isolated from the on-disk save (no load, no write).
    pub fn wants_fresh_stats(&self) -> bool {
        self.fresh_stats
    }

    pub fn next_action(
        &mut self,
        screen: Screen,
        state: &GameState,
        stats: &PlayerStats,
        data: Option<&GameData>,
        now: f64,
    ) -> PlaytestDirective {
        self.update_stale_watchdog(screen, state, now);
        if self.is_stale(now) {
            eprintln!(
                "[BOT] Stopping: no phase/progress change for {:.1}s at {:?}/{:?}",
                now - self.stale_since,
                screen,
                state.game_phase
            );
            self.log_state_summary(state);
            return PlaytestDirective::Stop(2);
        }

        if now - self.last_action_time < self.action_delay {
            return PlaytestDirective::None;
        }

        let action = match screen {
            Screen::Loading => UiAction::None,
            Screen::MainMenu => {
                self.current_shift_logged = false;
                UiAction::StartGame
            }
            Screen::Briefing => {
                self.current_shift_logged = false;
                UiAction::StartGame
            }
            Screen::Game => self.game_action(state, stats, data),
            Screen::GameOver | Screen::Success => {
                if !self.current_shift_logged {
                    self.log_terminal_shift(screen, state);
                    self.current_shift_logged = true;
                    self.completed_shifts += 1;
                    if screen == Screen::GameOver || state.run_complete {
                        self.completed_campaigns += 1;
                    }
                }

                let campaign_limit_reached = self
                    .max_campaigns
                    .is_some_and(|limit| self.completed_campaigns >= limit);
                if campaign_limit_reached || self.completed_shifts >= self.max_shifts {
                    eprintln!("[BOT] Finished {} shift(s).", self.completed_shifts);
                    return PlaytestDirective::Stop(0);
                }

                // Press on through interim nights so the campaign path (nights
                // 2..N) is exercised; otherwise begin a fresh run.
                if screen == Screen::Success && !state.run_complete {
                    UiAction::NextNight
                } else {
                    UiAction::TryAgain
                }
            }
            Screen::SkillTree
            | Screen::Almanac
            | Screen::Leaderboard
            | Screen::HelpOptions
            | Screen::Credits => UiAction::ReturnToMenu,
        };

        if action == UiAction::None {
            PlaytestDirective::None
        } else {
            self.last_action_time = now;
            self.decision_count += 1;
            eprintln!(
                "[BOT] #{:03} {:?}/{:?} -> {:?}",
                self.decision_count, screen, state.game_phase, action
            );
            PlaytestDirective::Action(action)
        }
    }

    fn game_action(
        &mut self,
        state: &GameState,
        stats: &PlayerStats,
        data: Option<&GameData>,
    ) -> UiAction {
        match state.game_phase {
            GamePhase::Waiting => {
                if state.earnings >= state.minimum_earnings && !state.last_fare_night {
                    UiAction::EndShift
                } else if state.fuel < 35.0 && state.earnings >= 15 {
                    UiAction::RefuelPartial
                } else {
                    UiAction::Continue
                }
            }
            GamePhase::RideRequest => UiAction::AcceptRide,
            GamePhase::Driving => {
                // Settle the passenger before driving on. Cab actions are the
                // main counter to a meltdown and the bot never used one, so
                // every meltdown figure it has ever reported was of a bot
                // declining to defend itself.
                // Once per leg only: performing a cab action does not change
                // the phase, so repeating it would spin until the watchdog
                // stopped the run.
                let leg = state.route_history.len();
                match self.soothing_action(state, stats, data) {
                    Some(action_key) if self.soothed_at_leg != Some(leg) => {
                        self.soothed_at_leg = Some(leg);
                        UiAction::PerformRuleAction(action_key)
                    }
                    _ => UiAction::SelectRoute(self.choose_route_index(state, stats, data)),
                }
            }
            GamePhase::Interaction => {
                if let Some(event) = &state.current_event {
                    if event.choices.is_empty() {
                        UiAction::Continue
                    } else {
                        let idx = self.event_cursor % event.choices.len();
                        self.event_cursor += 1;
                        UiAction::SelectEventChoice(idx)
                    }
                } else {
                    UiAction::Continue
                }
            }
            GamePhase::GuidelineDecision => {
                if self.strategy == PlaytestStrategy::Coverage {
                    let action = if self.guideline_cursor.is_multiple_of(2) {
                        UiAction::FollowGuideline
                    } else {
                        UiAction::BreakGuideline
                    };
                    self.guideline_cursor += 1;
                    action
                } else {
                    Self::read_the_passenger(state, stats)
                }
            }
            GamePhase::DropOff => {
                // Continue no longer answers an open trade offer, so the bot
                // declines it explicitly — the same net behaviour it had when
                // Continue declined silently.
                if state.pending_trade.is_some() {
                    UiAction::DeclineTrade
                } else {
                    UiAction::Continue
                }
            }
            GamePhase::GameOver | GamePhase::Success => UiAction::None,
            _ => UiAction::None,
        }
    }

    fn update_stale_watchdog(&mut self, screen: Screen, state: &GameState, now: f64) {
        let signature = format!(
            "{:?}|{:?}|{:?}|{}|{}|{:.0}|{}|{}",
            screen,
            state.game_phase,
            state.driving_phase,
            state.rides_completed,
            state.route_history.len(),
            state.fuel,
            state.earnings,
            state.used_passengers.len()
        );

        if signature != self.last_signature {
            self.last_signature = signature;
            self.stale_since = now;
        } else if self.stale_since == 0.0 {
            self.stale_since = now;
        }
    }

    fn is_stale(&self, now: f64) -> bool {
        self.stale_since > 0.0 && now - self.stale_since > 12.0
    }

    fn log_terminal_shift(&self, screen: Screen, state: &GameState) {
        let route_summary = state
            .route_history
            .iter()
            .map(|entry| format!("{:?}", entry.route_type))
            .collect::<Vec<_>>()
            .join(" -> ");
        // Night and quota are logged because they are what a run escalates:
        // every measurement so far restarted at night one, so nothing showed
        // how far into a campaign a shift got or what it was asked to earn.
        eprintln!(
            "[BOT] Shift {} ended on {:?}: night={}, rides={}, earnings=${}/{}, fuel={:.0}%, time={}, routes=[{}], reason={}",
            self.completed_shifts + 1,
            screen,
            state.night,
            state.rides_completed,
            state.earnings,
            state.minimum_earnings,
            state.fuel,
            state.time_remaining,
            route_summary,
            state
                .game_over_reason
                .as_deref()
                .unwrap_or("completed")
        );
        let modifier = state
            .night_modifier
            .as_ref()
            .map(|modifier| modifier.name.as_str())
            .unwrap_or("None");
        let fares = state
            .fare_contributions
            .iter()
            .map(|fare| {
                format!(
                    "{{\"passenger_id\":{},\"passenger\":{},\"fare\":{}}}",
                    fare.passenger_id,
                    json_string(&fare.passenger_name),
                    fare.fare
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let routes = state
            .route_history
            .iter()
            .map(|entry| json_string(&format!("{:?}", entry.route_type)))
            .collect::<Vec<_>>()
            .join(",");
        let reason = state.game_over_reason.as_deref().unwrap_or("completed");
        let tier = self.progression_tier();
        eprintln!(
            "[BOT_JSON] {{\"run\":{},\"seed\":{},\"night\":{},\"campaign_complete\":{},\"tier\":{},\"modifier\":{},\"rides\":{},\"earnings\":{},\"quota\":{},\"fuel_end\":{:.1},\"time_end\":{},\"wards_end\":{},\"refuel_stops\":{},\"refuel_cost\":{},\"comfort_relief\":{},\"normal_relief\":{},\"ward_interventions\":{},\"brink_saves\":{},\"failure_cause\":{},\"reason\":{},\"fares\":[{}],\"routes\":[{}]}}",
            self.completed_campaigns + 1,
            self.configured_seed.unwrap_or(0),
            state.night,
            state.run_complete,
            json_string(&tier),
            json_string(modifier),
            state.rides_completed,
            state.earnings,
            state.minimum_earnings,
            state.fuel,
            state.time_remaining,
            state.wards_in_hand(),
            state.telemetry.refuel_stops,
            state.telemetry.refuel_cost_paid,
            state.telemetry.comfort_relief,
            state.telemetry.normal_route_relief,
            state.telemetry.ward_interventions,
            state.telemetry.brink_saves,
            json_string(failure_cause(reason, screen, state.rules_violated)),
            json_string(reason),
            fares,
            routes,
        );
    }

    fn progression_tier(&self) -> String {
        if !self.named_skills.is_empty() {
            return "custom-skills".to_string();
        }
        match (self.almanac_level, self.unlock_all_skills) {
            (0, false) => "baseline".to_string(),
            (0, true) => "skills-only".to_string(),
            (level, true) => format!("almanac-{level}-all-skills"),
            (level, false) => format!("almanac-{level}"),
        }
    }

    fn log_state_summary(&self, state: &GameState) {
        eprintln!(
            "[BOT] State: phase={:?}, rides={}, fuel={:.0}%, earnings=${}, time={}, routes={}, passenger={}",
            state.game_phase,
            state.rides_completed,
            state.fuel,
            state.earnings,
            state.time_remaining,
            state.route_history.len(),
            state
                .current_passenger
                .as_ref()
                .map(|passenger| passenger.name.as_str())
                .unwrap_or("none")
        );
    }
}

fn json_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"unavailable\"".to_string())
}

fn failure_cause(reason: &str, screen: Screen, rules_violated: u32) -> &'static str {
    if screen == Screen::Success {
        "success"
    } else {
        let lower = reason.to_ascii_lowercase();
        if lower.contains("time") || lower.contains("next leg") || lower.contains("shift ends") {
            "time"
        } else if lower.contains("quota") || lower.contains("earn") {
            "quota"
        } else if lower.contains("fuel") || lower.contains("tank") {
            "fuel"
        } else if lower.contains("need became uncontrollable") || lower.contains("meltdown") {
            "meltdown"
        } else if lower.contains("hidden rule") {
            "hidden-rule"
        } else if lower.contains("wrong choice") || lower.contains("misread") {
            "misread"
        } else if rules_violated > 0 || lower.contains("rule") || lower.contains("dispatch routes")
        {
            "violation"
        } else {
            "other"
        }
    }
}

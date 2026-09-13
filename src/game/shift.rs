//! The shape of a night and of a run.
//!
//! Starting a run, setting up each night with its escalating quota and
//! difficulty, ending a shift, and paying out what it earned into the bank
//! and the almanac. Kept apart from the frame loop and the ride flow because
//! this is the layer the meta-progression is settled in.

use super::Game;
use crate::data::{EndingFacts, EpilogueCause, EpilogueKind};
use crate::engine::*;
use crate::screens::Screen;
use crate::state::*;
use macroquad::prelude::*;

/// How much of the night has been worked, when the wall clock is unavailable.
///
/// Both halves of this were wrong. The shift length was written here as a bare
/// `480` rather than read from `INITIAL_TIME`, so retuning the night in
/// `constants.json` would have left this one number behind. And the subtraction
/// is on `u32`: the Tarot Card grants a `time_bonus` of 60 and nothing caps the
/// clock at its starting value, so a card played early puts `time_remaining`
/// above the shift length and the old expression underflowed -- a panic in a
/// debug build, and about eight thousand years of logged play time in a release
/// one.
fn minutes_on_the_clock(initial_time: u32, time_remaining: u32) -> u32 {
    initial_time.saturating_sub(time_remaining)
}

impl Game {
    /// Start a new run from night 1.
    pub fn start_game(&mut self) {
        self.resume_available = false;
        self.game_state.night = 1;
        self.game_state.run_complete = false;
        // A fixed seed re-arms at every run start, so retrying a seeded run
        // deals the same campaign draw-for-draw — month, rules, weather,
        // fares, all of it.
        if let Some(seed) = self.run_seed() {
            self.game_state.rng = macroquad_toolkit::rng::SeededRng::new(seed);
        }
        // Deal the saved standings out as this run's working copy.
        self.game_state.passenger_reputation = self.player_stats.passenger_reputation.clone();
        // Each run falls in its own month, so the seasonal spawn weights,
        // the winter hazard bonus, and the winter conditions branch rotate
        // into play across runs instead of being locked to October.
        self.game_state.campaign_month = self.game_state.rng.range_i32(1, 13) as u32;
        self.begin_night();
    }

    /// Advance to the next night of the current run.
    pub(super) fn advance_night(&mut self) {
        self.game_state.night += 1;
        self.begin_night();
    }

    /// Set up and begin the current night (`game_state.night`), scaling
    /// difficulty and the earnings quota with how deep into the run we are.
    pub(super) fn begin_night(&mut self) {
        if let Some(ref data) = self.game_data {
            let current_time = self.game_state.simulation_time;
            self.player_stats.session_start = Some(current_time);

            // Reset per-night resources (fuel, time, earnings) but keep the run's
            // night counter, which is owned by start_game/advance_night.
            self.game_state
                .reset_for_new_shift(current_time, &data.constants.game_constants);
            // A night starts with a clear windshield: no overlay opened over
            // a previous screen may survive into this one.
            self.overlays.close_all();

            // An ordinary mid-run night may carry a modifier — rolled on the
            // seeded stream before anything reads it. Night 1 never does
            // (the opening stays learnable) and The Last Fare never does
            // (that night is authored whole).
            let night = self.game_state.night;
            let nights_per_run = data.constants.game_constants.nights_per_run;
            self.game_state.night_modifier = if night > 1 && night <= nights_per_run {
                data.night_modifiers.roll(&mut self.game_state.rng)
            } else {
                None
            };
            let modifier = self.game_state.night_modifier.clone();

            // Difficulty escalates with the night within the run, layered on top
            // of the player's lifetime experience, and drives the rule count.
            let max_diff = data.constants.scoring.max_difficulty;
            let base_diff = self.player_stats.suggested_difficulty();
            let difficulty_step = data.constants.game_constants.difficulty_increase_per_night;
            let modifier_diff = modifier.as_ref().map(|m| m.difficulty_bonus).unwrap_or(0);
            let effective_diff =
                (base_diff + (night - 1) * difficulty_step + modifier_diff).min(max_diff);
            let synthetic_xp = effective_diff * data.constants.scoring.experience_per_level;
            let shift_rules = GameEngine::generate_shift_rules(
                &mut self.game_state.rng,
                synthetic_xp,
                &data.rules,
                &data.constants,
            );
            self.game_state.current_rules = shift_rules.visible_rules;
            self.game_state.hidden_rules = shift_rules.hidden_rules;
            self.game_state.difficulty_level = shift_rules.difficulty_level;

            // The nightly quota rises by an authored share of the base each
            // night: 150, 225, 300, 375, 450 across a five-night run at the
            // shipped 0.5, against a shift whose fuel and clock do not grow.
            let base_quota = data.constants.game_constants.minimum_earnings;
            let step = data.constants.game_constants.quota_increase_per_night;
            let growth = (base_quota as f32 * step * (night - 1) as f32).round() as u32;
            let quota_mult = modifier.as_ref().map(|m| m.quota_mult).unwrap_or(1.0);
            self.game_state.minimum_earnings =
                ((base_quota + growth) as f32 * quota_mult).round() as u32;

            // Past the last ordinary night lies The Last Fare: quota is moot
            // (Death pays nothing), and the only success is delivering him.
            // Reachable only when night 5 was survived with the whole roster
            // mastered — `end_shift` holds the run open in that case.
            if night > nights_per_run {
                self.game_state.last_fare_night = true;
                self.game_state.minimum_earnings = 0;
            }

            // Apply the player's unlocked-skill effects for this shift.
            let skill_mods =
                SkillModifiers::from_unlocked(&data.skills, &self.player_stats.unlocked_skills);
            self.game_state.max_fuel = 100.0 + skill_mods.max_fuel_bonus;
            self.game_state.supernatural_protection += skill_mods.bonus_protection;
            // A modifier can open the night with a lighter tank; never so
            // light there is no night to drive.
            if let Some(delta) = modifier
                .as_ref()
                .map(|m| m.start_fuel_delta)
                .filter(|delta| *delta != 0)
            {
                self.game_state.fuel =
                    (self.game_state.fuel + delta as f32).clamp(10.0, self.game_state.max_fuel);
            }
            // Glimpse: a chance to reveal one hidden rule up front.
            if skill_mods.reveal_hidden_chance > 0.0
                && self.game_state.rng.chance(skill_mods.reveal_hidden_chance)
            {
                if let Some(rule_id) = self.game_state.hidden_rules.first().map(|r| r.id) {
                    self.game_state.reveal_hidden_rule(rule_id);
                }
            }

            // Load guidelines for tell detection
            self.game_state.current_guidelines = data.guidelines.clone();

            // Initialize weather
            self.game_state.season =
                WeatherService::get_current_season(self.game_state.campaign_month);
            self.game_state.current_weather = WeatherService::generate_initial_weather(
                &mut self.game_state.rng,
                &self.game_state.season,
                current_time,
            );
            self.game_state.time_of_day = WeatherService::time_of_day_after(0);

            // The Last Fare is authored, not rolled: a heavy storm (whose
            // hazards and weather rules follow from it), Death's own rule on
            // the board so his ride is played against it, and a tank at 40
            // with every station closed — the night is one delivery, and the
            // city makes it cost.
            if self.game_state.last_fare_night {
                self.game_state.current_weather =
                    WeatherService::last_fare_storm(&self.game_state.season, current_time);
                if let Some(rule) = data
                    .rules
                    .iter()
                    .find(|rule| rule.id == crate::data::DEATHS_RULE_ID)
                {
                    if !self
                        .game_state
                        .current_rules
                        .iter()
                        .any(|in_force| in_force.id == rule.id)
                    {
                        self.game_state.current_rules.push(rule.clone());
                    }
                }
                self.game_state.fuel = 40.0_f32.min(self.game_state.max_fuel);
            }

            self.game_state.environmental_hazards = WeatherService::generate_hazards(
                &mut self.game_state.rng,
                &self.game_state.current_weather,
                &self.game_state.time_of_day,
                &self.game_state.season,
                current_time,
            );
            self.sync_weather_rules();
            self.last_hazard_update = current_time;

            self.transition.begin_scene();
            self.game_state.game_phase = GamePhase::Briefing;
            self.screen = Screen::Briefing;
            if night == 1
                && self.player_stats.total_shifts_completed == 0
                && !self.player_stats.tutorial_completed
                && self.playtest_bot.is_none()
                && !self.capture_mode
            {
                self.help_return_screen = Screen::Briefing;
                self.tutorial_active = true;
                self.screen = Screen::HelpOptions;
            }
        }
    }

    /// Start the shift after briefing
    pub fn start_shift(&mut self) {
        self.game_state.game_phase = GamePhase::Waiting;
        self.game_state.current_dialogue = Some(CurrentDialogue {
            text: "Dispatch is quiet. Find a passenger when you're ready.".to_string(),
            speaker: DialogueSpeaker::Narrator,
            timestamp: self.game_state.simulation_time,
        });
        self.transition.begin_scene();
        self.screen = Screen::Game;
        // Don't auto-spawn - let player refuel first
    }

    /// Continue after drop off
    pub(super) fn continue_from_dropoff(&mut self) {
        self.game_state.current_passenger = None;
        self.game_state.current_passenger_dialogue = None;
        self.game_state.current_passenger_need_state = None;
        self.game_state.current_ride = None;
        self.game_state.current_event = None;
        self.game_state.last_ride_completion = None;
        self.game_state.driving_phase = None;
        // Leaving the drop-off declines any offer still on the table. The trade
        // modal only draws during DropOff, so a surviving offer would go
        // invisible and then reappear over the next passenger's drop-off,
        // trading an item the previous passenger was holding.
        self.game_state.pending_trade = None;
        // Comfort soothing recharges between fares.
        self.game_state.comfort_soothed_actions.clear();

        // Delivering Death closes the run then and there — nothing about the
        // night after that ride matters.
        if self.game_state.death_delivered {
            self.end_shift(true);
            return;
        }

        // Check end conditions
        if self.game_state.should_end_shift() {
            self.end_shift(self.game_state.earnings >= self.game_state.minimum_earnings);
        } else {
            // Go to Waiting phase to allow refueling
            self.game_state.game_phase = GamePhase::Waiting;
            // Don't spawn passenger immediately - let player refuel first
        }
    }

    /// Bucket a dead run's cause for epilogue selection, from the authored
    /// `game_over_reason` strings this codebase writes — matched here, next
    /// to nothing, because the strings and the buckets live in one repo and
    /// the fallback for a miss is the generic game-over pool.
    fn game_over_cause(&self) -> Option<EpilogueCause> {
        if self.game_state.last_fare_night && !self.game_state.death_delivered {
            return Some(EpilogueCause::LastFareFailed);
        }
        let reason = self.game_state.game_over_reason.as_deref()?;
        if reason.contains("uncontrollable") {
            Some(EpilogueCause::Meltdown)
        } else if reason.contains("Hidden Rule") {
            Some(EpilogueCause::HiddenRule)
        } else if reason.contains("only earned")
            || reason.contains("Not enough")
            || reason.contains("not make the next leg")
            || reason.contains("ran out of fuel")
        {
            Some(EpilogueCause::OutOfNight)
        } else {
            None
        }
    }

    /// Whether the whole roster is mastered — the gate for The Last Fare.
    /// "All knowledge" means every soul in the almanac at Lv.3, the reaper
    /// included, whether studied with lore or learned ride by ride.
    fn death_gate_met(&self) -> bool {
        self.game_data.as_ref().is_some_and(|data| {
            data.passengers.iter().all(|passenger| {
                self.player_stats
                    .get_almanac_entry(passenger.id)
                    .knowledge_level
                    >= 3
            })
        })
    }

    /// End the shift
    pub(super) fn end_shift(&mut self, success: bool) {
        let earned_enough = self.game_state.earnings >= self.game_state.minimum_earnings;
        // On The Last Fare the quota is moot: only delivering Death counts.
        let actually_successful = success
            && earned_enough
            && (!self.game_state.last_fare_night || self.game_state.death_delivered);

        let nights_per_run = self
            .game_data
            .as_ref()
            .map(|d| d.constants.game_constants.nights_per_run)
            .unwrap_or(5)
            .max(1);

        if actually_successful {
            self.game_state.queue_audio(
                "success",
                "[Dispatch chime: quota cleared and the shift is survived]",
                self.game_state.simulation_time,
            );
            // Add survival bonus
            if let Some(ref data) = self.game_data {
                self.game_state.earnings += data.constants.game_constants.survival_bonus;
            }
            // Surviving the final night completes the run — unless the whole
            // roster is mastered, in which case the run holds open for one
            // more night: The Last Fare, which only delivering Death closes.
            self.game_state.run_complete = if self.game_state.last_fare_night {
                self.game_state.death_delivered
            } else {
                self.game_state.night >= nights_per_run && !self.death_gate_met()
            };
            self.transition.begin_scene();
            self.game_state.game_phase = GamePhase::Success;
            self.screen = Screen::Success;
        } else {
            if self
                .game_state
                .game_over_reason
                .as_deref()
                .is_some_and(|reason| reason.contains("uncontrollable"))
            {
                self.game_state.queue_audio(
                    "meltdown",
                    "[Cabin distortion: passenger meltdown]",
                    self.game_state.simulation_time,
                );
            }
            if self.game_state.game_over_reason.is_none() {
                self.game_state.game_over_reason = Some(if self.game_state.last_fare_night {
                    "Dawn broke with the last fare uncollected. The city does not \
                         forgive an unfinished ledger."
                        .to_string()
                } else if !earned_enough {
                    format!(
                        "You only earned ${} but needed ${}.",
                        self.game_state.earnings, self.game_state.minimum_earnings
                    )
                } else {
                    "The night shift has ended...".to_string()
                });
            }
            // Add screen shake for dramatic effect
            self.screen_shake.shake(1.0, 0.5);
            self.transition.begin_scene();
            self.game_state.game_phase = GamePhase::GameOver;
            self.screen = Screen::GameOver;
        }

        // The ending's authored paragraph, chosen on the seeded stream so a
        // seeded run ends on the same words. Interim nights get none — they
        // end on a button, not a chapter. Selected before the run-completion
        // counters move, so "first of its kind" still means first.
        let kind = if actually_successful && self.game_state.run_complete {
            if self.game_state.death_delivered {
                Some(EpilogueKind::DeathDelivered)
            } else {
                Some(EpilogueKind::RunComplete)
            }
        } else if !actually_successful {
            Some(EpilogueKind::GameOver)
        } else {
            None
        };
        self.game_state.epilogue = None;
        if let Some(kind) = kind {
            let facts = EndingFacts {
                kind,
                cause: self.game_over_cause(),
                clean_night: self.game_state.rules_violated == 0,
                first_of_its_kind: match kind {
                    EpilogueKind::DeathDelivered => self.player_stats.death_deliveries == 0,
                    _ => self.player_stats.runs_completed == 0,
                },
            };
            // Cloned out so the deck and the state's rng can be borrowed
            // together; the deck is a few kilobytes once per ending.
            let deck = self.game_data.as_ref().map(|data| data.epilogues.clone());
            if let Some(deck) = deck {
                self.game_state.epilogue =
                    crate::data::select_epilogue(&deck, facts, &mut self.game_state.rng);
            }
        }

        // Record stats
        let play_time = self
            .player_stats
            .session_start
            .map(|start| ((self.game_state.simulation_time - start) / 60.0).max(0.0) as u32)
            .unwrap_or_else(|| {
                let initial = self
                    .game_data
                    .as_ref()
                    .map(|data| data.constants.game_constants.initial_time)
                    .unwrap_or(0);
                minutes_on_the_clock(initial, self.game_state.time_remaining)
            });

        self.player_stats
            .record_shift_completion(&self.game_state, actually_successful, play_time);
        self.player_stats.session_start = None;

        // Generate bank balance from earnings (50% of earnings goes to bank)
        let bank_earnings = self.game_state.earnings / 2;
        self.player_stats.bank_balance += bank_earnings;
        self.game_state.shift_payout.bank = bank_earnings;

        // Generate lore fragments
        // Base: 1 per completed ride
        let mut lore_earned = self.game_state.rides_completed;
        // Bonus: 2 per unlocked backstory this shift
        let backstories_unlocked = self
            .game_state
            .used_passengers
            .iter()
            .filter(|id| self.player_stats.is_backstory_unlocked(**id))
            .count() as u32;
        lore_earned += backstories_unlocked * 2;
        // Difficulty bonus
        lore_earned += self.game_state.difficulty_level;
        // A modifier can thicken the night with story.
        lore_earned += self
            .game_state
            .night_modifier
            .as_ref()
            .map(|m| m.lore_bonus)
            .unwrap_or(0);
        self.player_stats.lore_fragments += lore_earned;
        self.game_state.shift_payout.lore = lore_earned;

        // Mark all encountered passengers in almanac
        for passenger_id in &self.game_state.used_passengers {
            self.player_stats.mark_passenger_encountered(*passenger_id);
        }

        // Fold the night's standings back into the save. The shift plays
        // against the working copy on `game_state`; without this, every
        // reputation earned died with the run.
        self.player_stats.passenger_reputation = self.game_state.passenger_reputation.clone();

        // Add leaderboard entry
        if let Some(ref data) = self.game_data {
            #[cfg(not(target_arch = "wasm32"))]
            let date_str = {
                use chrono::Local;
                Local::now().format("%Y-%m-%d %H:%M").to_string()
            };
            // The web build has no wall clock, and every row reading "Session"
            // told the player nothing — the leaderboard's whole job is
            // telling ten runs apart. The shift counter is already
            // incremented by `record_shift_completion` above and does the
            // same work: it orders the entries and distinguishes them.
            #[cfg(target_arch = "wasm32")]
            let date_str = format!("Shift {}", self.player_stats.total_shifts_completed);

            let score = self.game_state.calculate_score(&data.constants);
            let entry = LeaderboardEntry {
                score,
                date: date_str,
                survived: actually_successful,
                passengers_transported: self.game_state.rides_completed,
                difficulty_level: self.game_state.difficulty_level,
                rules_violated: self.game_state.rules_violated,
            };
            self.player_stats.add_leaderboard_entry(entry);
        }

        // Check and unlock achievements, paying whatever each one is worth.
        let unlocked = self.player_stats.check_achievements(Some(FinishedShift::of(
            &self.game_state,
            actually_successful,
        )));
        self.pay_achievement_rewards(&unlocked);

        // Surviving every night of a run is the game's headline result and
        // used to pay nothing beyond the per-night survival bonus.
        if actually_successful && self.game_state.run_complete {
            self.player_stats.runs_completed += 1;
            if self.game_state.death_delivered {
                self.player_stats.death_deliveries += 1;
            }
            if let Some(data) = &self.game_data {
                let nights = data.constants.game_constants.nights_per_run.max(1);
                let payout = data.rewards.run_completion.payout(nights);
                self.player_stats.bank_balance += payout.bank;
                self.player_stats.lore_fragments += payout.lore;
                self.game_state.shift_payout.run_bonus_bank = payout.bank;
                self.game_state.shift_payout.run_bonus_lore = payout.lore;
            }
        }

        // Auto-save after shift
        self.resume_available = false;
        self.save_stats();
    }

    /// Pay the bank and lore a freshly unlocked achievement is worth.
    ///
    /// The six achievements were pure scoreboard entries; paying them makes
    /// the behaviours they name — surviving, mastering the almanac, buying
    /// into the skill tree — feed the currencies that buy the next upgrade.
    pub(super) fn pay_achievement_rewards(&mut self, unlocked: &[String]) {
        let Some(data) = &self.game_data else {
            return;
        };
        let mut total = crate::data::Payout::default();
        for id in unlocked {
            let payout = data.rewards.for_achievement(id);
            total.bank += payout.bank;
            total.lore += payout.lore;
        }
        if total.is_empty() {
            return;
        }
        self.player_stats.bank_balance += total.bank;
        self.player_stats.lore_fragments += total.lore;
    }
}

#[cfg(test)]
mod tests;

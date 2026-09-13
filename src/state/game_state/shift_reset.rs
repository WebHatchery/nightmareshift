//! Reset transient shift state while preserving the campaign shell.

use super::GameState;
use crate::data::GameConstants;
use crate::state::{GamePhase, MetaPayout, ShiftTelemetry};

impl GameState {
    /// Reset for a new shift using the authored starting resources.
    pub fn reset_for_new_shift(&mut self, current_time: f64, constants: &GameConstants) {
        self.fuel = constants.initial_fuel as f32;
        self.max_fuel = 100.0;
        self.earnings = 0;
        self.time_remaining = constants.initial_time;
        self.headlights_on = true;
        self.minimum_earnings = constants.minimum_earnings;
        self.rides_completed = 0;
        self.rules_violated = 0;
        self.fare_contributions.clear();
        self.telemetry = ShiftTelemetry::default();
        self.ride_baseline = None;
        self.current_rules.clear();
        self.hidden_rules.clear();
        self.revealed_hidden_rules.clear();
        self.temporary_rules.clear();
        self.current_guidelines.clear();
        self.current_passenger = None;
        self.current_passenger_dialogue = None;
        self.current_ride = None;
        self.current_event = None;
        self.game_phase = GamePhase::Waiting;
        self.driving_phase = None;
        self.used_passengers.clear();
        self.shift_start_time = Some(current_time);
        self.shift_end_warning_shown = false;
        self.shift_payout = MetaPayout::default();
        self.current_passenger_need_state = None;
        self.detected_tells.clear();
        self.false_tell_planted = false;
        self.live_exceptions.clear();
        self.route_history.clear();
        self.consecutive_route_streak = None;
        self.environmental_hazards.clear();
        self.player_trust = 0.5;
        self.rule_immunity_charges = 0;
        self.supernatural_protection = 0;
        self.curse_danger_bonus = 0;
        self.pending_trade = None;
        self.trade_outcome = None;
        self.current_dialogue = None;
        self.consequence_notes.clear();
        self.last_ride_completion = None;
        self.game_over_reason = None;
        self.pending_audio = None;
        self.last_audio_caption = None;
        self.active_guideline = None;
        self.guideline_decision_start_time = None;
        self.guideline_time_remaining = self.guideline_decision_seconds;
        self.comfort_soothed_actions.clear();
        self.brink_spent = false;
        self.night_modifier = None;
        self.last_fare_night = false;
        self.death_delivered = false;
        self.epilogue = None;
    }
}

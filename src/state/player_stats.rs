//! Player statistics tracking across sessions.

use super::game_state::PassengerReputation;
use super::key_bindings::KeyBindings;
use crate::data::RouteType;
use macroquad_toolkit::achievements::{Achievement, Achievements};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

/// Leaderboard entry for a completed shift
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub score: u32,
    pub date: String,
    pub survived: bool,
    pub passengers_transported: u32,
    pub difficulty_level: u32,
    pub rules_violated: u32,
}

/// Almanac progress for a passenger
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AlmanacEntry {
    pub passenger_id: u32,
    pub encountered: bool,
    pub knowledge_level: u32,
    /// Rides delivered with this passenger, across every run. Surviving them
    /// is study: crossing `AlmanacEntry::STUDY_THRESHOLDS` raises
    /// `knowledge_level` for free, and lore purchases remain the accelerant.
    #[serde(default)]
    pub rides_survived: u32,
    /// Tell descriptions the driver has personally noticed, kept forever.
    /// The dossier shows these at any knowledge level — what you have seen
    /// with your own eyes is not gated behind lore.
    #[serde(default)]
    pub tells_seen: Vec<String>,
    /// Exact tiers earned by surviving rides, separate from purchased study.
    #[serde(default)]
    pub earned_knowledge_levels: Vec<u32>,
    /// Exact tiers bought with lore, so the dossier can name their source.
    #[serde(default)]
    pub lore_knowledge_levels: Vec<u32>,
}

impl AlmanacEntry {
    /// Rides survived at which knowledge levels 1..=3 are earned by play.
    pub const STUDY_THRESHOLDS: [u32; 3] = [1, 3, 6];
}

/// Accepts saves from before the `Achievements` registry adoption, where
/// `achievements` was a bare `Vec<Achievement>` instead of the registry's
/// `{ achievements: [...] }` shape. Old and new shapes both deserialize
/// cleanly since `Achievement`'s fields are unchanged.
fn deserialize_achievements<'de, D>(deserializer: D) -> Result<Achievements, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Compat {
        Legacy(Vec<Achievement>),
        Current(Achievements),
    }

    Ok(match Compat::deserialize(deserializer)? {
        Compat::Legacy(list) => Achievements::from_definitions(list),
        Compat::Current(achievements) => achievements,
    })
}

/// What each countable achievement asks for.
///
/// Named once because they are needed twice: `check_achievements` tests them,
/// and `achievement_progress` prints how close the driver is. Two copies of a
/// threshold is how a card ends up promising something the condition does not
/// grant, which this project has now been bitten by in two other places.
pub mod achievement_targets {
    pub const SHIFTS_SURVIVED: u32 = 10;
    pub const SINGLE_SHIFT_EARNINGS: u32 = 500;
    pub const PASSENGERS_MASTERED: usize = 5;
    pub const SKILLS_UNLOCKED: usize = 3;
}

/// The three facts about a just-finished shift that achievements ask about.
///
/// Built from the state rather than passed as loose values, for the same reason
/// `record_shift_completion` takes the shift: three scalars in a row is where a
/// call site goes wrong, and `survived` is the only one the state cannot answer
/// for itself.
#[derive(Debug, Clone, Copy)]
pub struct FinishedShift {
    pub earnings: u32,
    pub violations: u32,
    pub survived: bool,
}

/// Player-facing presentation and accessibility preferences.
///
/// These live with the save so native and web builds use the same persistence
/// path as every other durable player choice. Defaults preserve the original
/// presentation while captions remain on: authored sound cues may add
/// atmosphere, but required information is never audio-only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilitySettings {
    #[serde(default = "default_text_scale")]
    pub text_scale_percent: u16,
    #[serde(default)]
    pub high_contrast: bool,
    #[serde(default)]
    pub reduced_motion: bool,
    #[serde(default = "default_brightness")]
    pub brightness_percent: u16,
    #[serde(default = "default_true")]
    pub captions: bool,
    #[serde(default)]
    pub fullscreen: bool,
    #[serde(default = "default_volume")]
    pub master_volume: u8,
    #[serde(default = "default_volume")]
    pub ambience_volume: u8,
    #[serde(default = "default_volume")]
    pub music_volume: u8,
    #[serde(default = "default_volume")]
    pub effects_volume: u8,
    #[serde(default)]
    pub key_bindings: KeyBindings,
}

impl Default for AccessibilitySettings {
    fn default() -> Self {
        Self {
            text_scale_percent: default_text_scale(),
            high_contrast: false,
            reduced_motion: false,
            brightness_percent: default_brightness(),
            captions: true,
            fullscreen: false,
            master_volume: default_volume(),
            ambience_volume: default_volume(),
            music_volume: default_volume(),
            effects_volume: default_volume(),
            key_bindings: KeyBindings::default(),
        }
    }
}

fn default_text_scale() -> u16 {
    100
}

fn default_brightness() -> u16 {
    100
}

fn default_volume() -> u8 {
    80
}

fn default_true() -> bool {
    true
}

impl AccessibilitySettings {
    pub fn text_scale(&self) -> f32 {
        self.text_scale_percent.clamp(100, 125) as f32 / 100.0
    }

    pub fn cycle_text_scale(&mut self) {
        self.text_scale_percent = match self.text_scale_percent {
            0..=100 => 115,
            101..=115 => 125,
            _ => 100,
        };
    }

    pub fn cycle_brightness(&mut self) {
        self.brightness_percent = match self.brightness_percent {
            0..=90 => 100,
            91..=100 => 115,
            _ => 90,
        };
    }

    pub fn cycle_master_volume(&mut self) {
        self.master_volume = match self.master_volume {
            0..=25 => 50,
            26..=50 => 80,
            51..=80 => 100,
            _ => 0,
        };
    }
}

impl FinishedShift {
    pub fn of(shift: &crate::state::GameState, survived: bool) -> Self {
        Self {
            earnings: shift.earnings,
            violations: shift.rules_violated,
            survived,
        }
    }
}

/// Persistent player statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayerStats {
    /// Total shifts completed
    pub total_shifts_completed: u32,
    /// Total rides completed across all shifts
    pub total_rides_completed: u32,
    /// Total earnings across all shifts
    pub total_earnings: u32,
    /// Highest single shift earnings
    pub highest_shift_earnings: u32,
    /// Total play time in minutes
    pub total_play_time: u32,
    /// Number of survival bonuses earned
    pub survival_bonuses: u32,
    /// Total rules violated
    pub total_rules_violated: u32,
    /// Unlocked backstories by passenger ID
    pub unlocked_backstories: HashMap<u32, bool>,
    /// Passenger encounter counts
    pub passenger_encounters: HashMap<u32, u32>,
    /// Unlocked skills by ID
    pub unlocked_skills: Vec<String>,
    /// Bank balance for purchasing skills
    #[serde(default)]
    pub bank_balance: u32,
    /// Almanac progress by passenger ID
    #[serde(default)]
    pub almanac_progress: HashMap<u32, AlmanacEntry>,
    /// Standing with each passenger, carried across runs. The shift plays
    /// against a working copy on `GameState`; `end_shift` folds it back
    /// here and `start_game` deals it out again.
    #[serde(default)]
    pub passenger_reputation: HashMap<u32, PassengerReputation>,
    /// Lore fragments for upgrading almanac knowledge
    #[serde(default)]
    pub lore_fragments: u32,
    /// Leaderboard entries (top 10 scores)
    #[serde(default)]
    pub leaderboard: Vec<LeaderboardEntry>,
    /// Route usage counts
    #[serde(default)]
    pub route_usage: HashMap<String, u32>,
    /// Full five-night runs survived. The save never recorded its headline
    /// result before this.
    #[serde(default)]
    pub runs_completed: u32,
    /// Times Death himself was delivered on The Last Fare — the true ending.
    #[serde(default)]
    pub death_deliveries: u32,
    /// Persistent presentation, input-equivalent, and audio preferences.
    #[serde(default)]
    pub accessibility: AccessibilitySettings,
    /// The first-run handbook has been acknowledged at least once.
    #[serde(default)]
    pub tutorial_completed: bool,
    /// Unlocked achievements
    #[serde(default, deserialize_with = "deserialize_achievements")]
    pub achievements: Achievements,
    /// Current session start time
    #[serde(skip)]
    pub session_start: Option<f64>,
}

impl PlayerStats {
    /// Create new empty player stats
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate player experience level
    pub fn experience_level(&self) -> u32 {
        (self.total_rides_completed + self.total_shifts_completed * 10) / 10
    }

    /// Calculate difficulty based on experience
    pub fn suggested_difficulty(&self) -> u32 {
        (self.total_shifts_completed / 10).min(4)
    }

    /// Fold a finished shift into the lifetime counters.
    ///
    /// Takes the shift rather than its numbers. `total_rules_violated` was
    /// the forgotten sibling here — five counters were maintained in this one
    /// place and it was not among them, so a stat the save has carried since
    /// the port read zero however a night went. Adding a sixth argument would
    /// have left the same hazard in place: with three `u32`s in a row the call
    /// site is where this goes wrong, and passing the state means there is
    /// nothing there to get wrong. `survived` and `play_time` stay parameters
    /// because neither lives on the shift.
    pub fn record_shift_completion(
        &mut self,
        shift: &crate::state::GameState,
        survived: bool,
        play_time: u32,
    ) {
        let earnings = shift.earnings;
        self.total_shifts_completed += 1;
        self.total_rides_completed += shift.rides_completed;
        self.total_earnings += earnings;
        self.total_play_time += play_time;
        self.total_rules_violated += shift.rules_violated;

        if earnings > self.highest_shift_earnings {
            self.highest_shift_earnings = earnings;
        }

        if survived {
            self.survival_bonuses += 1;
        }
    }

    /// Record a passenger encounter
    pub fn record_passenger_encounter(&mut self, passenger_id: u32) {
        *self.passenger_encounters.entry(passenger_id).or_insert(0) += 1;
    }

    /// Everything a revealed story means for a passenger.
    ///
    /// Kept here rather than in the two consequence handlers that call it so
    /// that what a `StoryUnlock` actually does is one decision in one place,
    /// and testable without a window. It marks the encounter as well as the
    /// backstory: an almanac entry has to exist before it can show anything.
    pub fn reveal_story(&mut self, passenger_id: u32) {
        self.record_passenger_encounter(passenger_id);
        self.unlock_backstory(passenger_id);
    }

    /// Unlock a backstory
    pub fn unlock_backstory(&mut self, passenger_id: u32) {
        self.unlocked_backstories.insert(passenger_id, true);
    }

    /// Check if backstory is unlocked
    pub fn is_backstory_unlocked(&self, passenger_id: u32) -> bool {
        self.unlocked_backstories
            .get(&passenger_id)
            .copied()
            .unwrap_or(false)
    }

    /// Get encounter count for a passenger
    pub fn get_encounter_count(&self, passenger_id: u32) -> u32 {
        self.passenger_encounters
            .get(&passenger_id)
            .copied()
            .unwrap_or(0)
    }

    /// Check if this is first encounter with a passenger
    pub fn is_first_encounter(&self, passenger_id: u32) -> bool {
        self.get_encounter_count(passenger_id) == 0
    }

    /// Check if skill is unlocked
    pub fn is_skill_unlocked(&self, skill_id: &str) -> bool {
        self.unlocked_skills.contains(&skill_id.to_string())
    }

    /// Stable key used for persisted route familiarity.
    ///
    /// Spelled the same as `RouteType::label` and deliberately kept its own
    /// table: this string is in every save file, and a change to how a route
    /// reads on screen must not quietly orphan a driver's route mastery.
    pub fn route_key(route: RouteType) -> &'static str {
        match route {
            RouteType::Normal => "Normal",
            RouteType::Shortcut => "Shortcut",
            RouteType::Scenic => "Scenic",
            RouteType::Police => "Police",
        }
    }

    /// Record route usage across runs.
    pub fn record_route_usage(&mut self, route: RouteType) {
        *self
            .route_usage
            .entry(Self::route_key(route).to_string())
            .or_insert(0) += 1;
    }

    /// Get persistent familiarity for a route.
    pub fn get_route_usage(&self, route: RouteType) -> u32 {
        self.route_usage
            .get(Self::route_key(route))
            .copied()
            .unwrap_or(0)
    }

    /// Build a route mastery map for services that still operate on RouteType keys.
    pub fn route_mastery_map(&self) -> HashMap<RouteType, u32> {
        [
            RouteType::Normal,
            RouteType::Shortcut,
            RouteType::Scenic,
            RouteType::Police,
        ]
        .into_iter()
        .filter_map(|route| {
            let usage = self.get_route_usage(route);
            (usage > 0).then_some((route, usage))
        })
        .collect()
    }

    /// Add a leaderboard entry and sort (keep top 10)
    pub fn add_leaderboard_entry(&mut self, entry: LeaderboardEntry) {
        self.leaderboard.push(entry);
        self.leaderboard
            .sort_by_key(|entry| std::cmp::Reverse(entry.score));
        self.leaderboard.truncate(10);
    }

    /// Get almanac entry for a passenger (or default)
    pub fn get_almanac_entry(&self, passenger_id: u32) -> AlmanacEntry {
        self.almanac_progress
            .get(&passenger_id)
            .cloned()
            .unwrap_or(AlmanacEntry {
                passenger_id,
                ..AlmanacEntry::default()
            })
    }

    fn almanac_entry_mut(&mut self, passenger_id: u32) -> &mut AlmanacEntry {
        self.almanac_progress
            .entry(passenger_id)
            .or_insert(AlmanacEntry {
                passenger_id,
                ..AlmanacEntry::default()
            })
    }

    /// Mark passenger as encountered
    pub fn mark_passenger_encountered(&mut self, passenger_id: u32) {
        self.almanac_entry_mut(passenger_id).encountered = true;
    }

    /// Surviving a ride is study. Counts the ride, and when the count crosses
    /// a `STUDY_THRESHOLDS` step the knowledge level rises for free — the
    /// almanac used to be purchase-only, which left nothing about a run that
    /// taught the player anything durable. Returns the new level when one was
    /// just earned this way.
    pub fn record_ride_survived(&mut self, passenger_id: u32) -> Option<u32> {
        let entry = self.almanac_entry_mut(passenger_id);
        entry.encountered = true;
        entry.rides_survived += 1;
        let earned = AlmanacEntry::STUDY_THRESHOLDS
            .iter()
            .filter(|threshold| entry.rides_survived >= **threshold)
            .count() as u32;
        if earned > 0 && !entry.earned_knowledge_levels.contains(&earned) {
            entry.earned_knowledge_levels.push(earned);
        }
        if earned > entry.knowledge_level {
            entry.knowledge_level = earned;
            Some(earned)
        } else {
            None
        }
    }

    /// Inscribe a tell the driver actually noticed. Permanent and free:
    /// what was seen stays seen, whatever the almanac level.
    pub fn record_tell_seen(&mut self, passenger_id: u32, description: &str) {
        let entry = self.almanac_entry_mut(passenger_id);
        if !entry.tells_seen.iter().any(|seen| seen == description) {
            entry.tells_seen.push(description.to_string());
        }
    }

    /// Upgrade almanac knowledge level
    pub fn upgrade_almanac_knowledge(&mut self, passenger_id: u32, cost: u32) -> bool {
        if self.lore_fragments < cost {
            return false;
        }
        let entry = self.almanac_entry_mut(passenger_id);
        entry.encountered = true;
        if entry.knowledge_level >= 3 {
            return false;
        }
        entry.knowledge_level += 1;
        entry.lore_knowledge_levels.push(entry.knowledge_level);
        self.lore_fragments -= cost;
        true
    }

    /// Purchase a skill with bank balance
    pub fn purchase_skill(&mut self, skill_id: &str, cost: u32) -> bool {
        if self.bank_balance >= cost && !self.unlocked_skills.contains(&skill_id.to_string()) {
            self.bank_balance -= cost;
            self.unlocked_skills.push(skill_id.to_string());
            true
        } else {
            false
        }
    }

    /// Sell `lore` fragments back for `bank`. Returns false when the player
    /// cannot cover the trade, leaving both balances untouched.
    pub fn exchange_lore_for_bank(&mut self, lore: u32, bank: u32) -> bool {
        if lore == 0 || self.lore_fragments < lore {
            return false;
        }
        self.lore_fragments -= lore;
        self.bank_balance += bank;
        true
    }

    /// The game's fixed achievement definitions (id, name, description).
    pub(crate) fn achievement_definitions() -> Vec<Achievement> {
        vec![
            Achievement::new("first_shift", "First Night", "Complete your first shift"),
            Achievement::new("survivor", "Survivor", "Survive 10 shifts"),
            Achievement::new(
                "perfect_shift",
                "Perfect Shift",
                "Complete a shift without violating any rules",
            ),
            Achievement::new("big_earner", "Big Earner", "Earn $500 in a single shift"),
            Achievement::new(
                "almanac_scholar",
                "Almanac Scholar",
                "Master knowledge of 5 passengers",
            ),
            Achievement::new("skill_collector", "Skill Collector", "Unlock 3 skills"),
        ]
    }

    /// How many passengers the driver has taken to the top almanac level.
    pub fn passengers_mastered(&self) -> usize {
        self.almanac_progress
            .values()
            .filter(|entry| entry.knowledge_level >= 3)
            .count()
    }

    /// How close the driver is to an achievement, in that achievement's own
    /// terms, or `None` for one that cannot be part-done.
    ///
    /// Four of the six count toward something and the save has been tracking
    /// all four all along -- `survival_bonuses` and `highest_shift_earnings`
    /// were tracked and displayed nowhere at all. So a card said "Survivor --
    /// Survive 10 shifts -- Locked" while the number that measures it sat in
    /// the save unreadable. `first_shift` and `perfect_shift` are pass or fail
    /// on a single night and have nothing to count.
    pub fn achievement_progress(&self, achievement_id: &str) -> Option<String> {
        use achievement_targets as target;
        match achievement_id {
            "survivor" => Some(format!(
                "{} / {} shifts survived",
                self.survival_bonuses.min(target::SHIFTS_SURVIVED),
                target::SHIFTS_SURVIVED
            )),
            "big_earner" => Some(format!(
                "best night ${} of ${}",
                self.highest_shift_earnings,
                target::SINGLE_SHIFT_EARNINGS
            )),
            "almanac_scholar" => Some(format!(
                "{} / {} passengers mastered",
                self.passengers_mastered().min(target::PASSENGERS_MASTERED),
                target::PASSENGERS_MASTERED
            )),
            "skill_collector" => Some(format!(
                "{} / {} skills unlocked",
                self.unlocked_skills.len().min(target::SKILLS_UNLOCKED),
                target::SKILLS_UNLOCKED
            )),
            _ => None,
        }
    }

    /// Reconcile the achievement registry with the current definitions.
    /// Safe to call every load: unlock state and dates are preserved.
    pub fn init_achievements(&mut self) {
        self.achievements
            .sync_definitions(Self::achievement_definitions());
    }

    /// Unlock an achievement
    pub fn unlock_achievement(&mut self, achievement_id: &str, date: String) -> bool {
        self.achievements
            .unlock_with_date(achievement_id, Some(date))
    }

    /// Check if achievement is unlocked
    pub fn is_achievement_unlocked(&self, achievement_id: &str) -> bool {
        self.achievements.is_unlocked(achievement_id)
    }

    /// Check and unlock achievements based on current stats
    /// Evaluate every achievement condition, returning the ids that unlocked
    /// for the first time on this call so the caller can pay their reward.
    /// Already-unlocked achievements are never reported twice.
    pub fn check_achievements(&mut self, shift: Option<FinishedShift>) -> Vec<String> {
        let mut newly_unlocked = Vec::new();
        #[cfg(not(target_arch = "wasm32"))]
        let now = {
            use chrono::Local;
            Local::now().format("%Y-%m-%d").to_string()
        };
        #[cfg(target_arch = "wasm32")]
        let now = "Today".to_string();

        let mastered = self.passengers_mastered();

        // Lifetime conditions: true whenever they become true, so any caller
        // may ask.
        let mut conditions = vec![
            ("first_shift", self.total_shifts_completed >= 1),
            (
                "survivor",
                self.survival_bonuses >= achievement_targets::SHIFTS_SURVIVED,
            ),
            (
                "almanac_scholar",
                mastered >= achievement_targets::PASSENGERS_MASTERED,
            ),
            (
                "skill_collector",
                self.unlocked_skills.len() >= achievement_targets::SKILLS_UNLOCKED,
            ),
        ];

        // Shift conditions only exist when a shift has just ended. They used
        // to be three loose `u32`/`bool` parameters, and the two menu callers
        // -- buying a skill, upgrading the almanac, both of which are here only
        // for `skill_collector` and `almanac_scholar` -- passed
        // `game_state.earnings` and `game_state.rules_violated` from a shift
        // that had already finished, plus a hardcoded `false` for survived.
        // Nothing misfired, because the real shift-end call had already run
        // with the same numbers; but there was no way for a caller to say "no
        // shift here" and the next earnings-based achievement would have
        // unlocked from the skill tree.
        if let Some(shift) = shift {
            conditions.push(("perfect_shift", shift.survived && shift.violations == 0));
            conditions.push((
                "big_earner",
                shift.earnings >= achievement_targets::SINGLE_SHIFT_EARNINGS,
            ));
        }

        for (id, met) in conditions {
            if met && self.unlock_achievement(id, now.clone()) {
                newly_unlocked.push(id.to_string());
            }
        }

        newly_unlocked
    }
}

#[cfg(test)]
mod tests;

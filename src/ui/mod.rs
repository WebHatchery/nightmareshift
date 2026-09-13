//! UI component modules.

pub mod backgrounds;
pub mod components;
pub mod core;
mod gallery;
mod prewarm;
mod primitives;

pub use backgrounds::*;
pub use components::*;
pub use core::*;
pub use gallery::*;
pub use prewarm::*;
pub use primitives::*;

/// Actions triggered by UI interactions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiAction {
    None,
    StartGame,
    ResumeRun,
    ExportSave,
    ImportSave,
    CycleAcceptBinding,
    CycleDeclineBinding,
    CycleFollowBinding,
    CycleBreakBinding,
    CyclePauseBinding,
    AcceptRide,
    DeclineRide,
    SelectRoute(usize),
    SelectEventChoice(usize), // New action for mid-ride events
    Continue,
    ReturnToMenu,
    TryAgain,
    /// Advance from the results screen to the next night of the run.
    NextNight,
    EndShift,
    RefuelFull,
    RefuelPartial,
    ToggleRules,
    ToggleInventory,
    TogglePauseMenu, // ESC pause menu
    UseItem(usize),
    PerformRuleAction(String),
    /// Open the seed-entry modal on the main menu.
    OpenSeedEntry,
    /// Begin a run on today's shared daily seed.
    StartDailyRun,
    /// Open the contributor and asset credits screen.
    OpenCredits,
    /// Begin a run on a player-entered seed.
    StartSeededRun(u64),
    /// Append one digit to the open seed-entry dialog.
    SeedDigit(char),
    /// Remove the last digit from the open seed-entry dialog.
    EraseSeedDigit,
    /// Close the seed-entry dialog without starting a run.
    CancelSeedEntry,
    /// Select a skill-tree category without mutating during drawing.
    SelectSkillCategory(usize),
    /// Select a skill-tree card without mutating during drawing.
    SelectSkill(String),
    /// Select an Almanac passenger without mutating during drawing.
    SelectAlmanacPassenger(u32),
    // Meta-progression screens
    OpenSkillTree,
    OpenAlmanac,
    OpenLeaderboard,
    OpenHelpOptions,
    CycleTextScale,
    ToggleHighContrast,
    ToggleReducedMotion,
    CycleBrightness,
    ToggleCaptions,
    ToggleFullscreen,
    CycleMasterVolume,
    CycleAmbienceVolume,
    CycleMusicVolume,
    CycleEffectsVolume,
    DeleteSave,
    PurchaseSkill(String),
    UpgradeAlmanacKnowledge(u32),
    /// Sell surplus lore fragments back for bank balance.
    ExchangeLoreForBank,
    // Trading
    AcceptTrade(usize),
    DeclineTrade,
    // Guideline decisions
    FollowGuideline,
    BreakGuideline,
}

//! Save/Load persistence system.

use super::{GameState, PlayerStats};
#[cfg(target_arch = "wasm32")]
use macroquad_toolkit::persistence::{delete_slot, load_from_slot, save_to_slot, slot_exists};
#[cfg(not(target_arch = "wasm32"))]
use macroquad_toolkit::persistence::{file_exists, get_app_data_path, load_json, save_json};
#[cfg(not(target_arch = "wasm32"))]
use macroquad_toolkit::persistence::{load_from_slot, save_to_slot};
use serde::{Deserialize, Serialize};

/// Save file name
const GAME_NAME: &str = "nightmare_shift";
const EXPORT_SLOT: &str = "export_backup";
#[cfg(not(target_arch = "wasm32"))]
const SAVE_FILE: &str = "nightmare_shift_save.json";
#[cfg(target_arch = "wasm32")]
const SAVE_SLOT: &str = "autosave";

/// Save data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveData {
    #[serde(default = "legacy_save_version")]
    pub version: u32,
    pub player_stats: PlayerStats,
    /// An optional checkpoint made from the pause menu. Old meta-only saves
    /// deserialize with no run and remain fully supported.
    #[serde(default)]
    pub run: Option<RunSave>,
}

/// A checkpoint of the active shift. `GameState` contains its RNG, current
/// passenger, campaign state, and pause-aware simulation timestamps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSave {
    pub game_state: GameState,
}

fn legacy_save_version() -> u32 {
    1
}

impl SaveData {
    /// Current save format version
    pub const VERSION: u32 = 2;

    /// Create new save data from player stats
    pub fn new(player_stats: PlayerStats) -> Self {
        Self {
            version: Self::VERSION,
            player_stats,
            run: None,
        }
    }

    /// Create a save with a resumable active shift.
    pub fn with_run(player_stats: PlayerStats, game_state: &GameState) -> Self {
        Self {
            version: Self::VERSION,
            player_stats,
            run: Some(RunSave {
                game_state: game_state.clone(),
            }),
        }
    }
}

/// Persistence system for save/load
pub struct Persistence;

impl Persistence {
    #[cfg(not(target_arch = "wasm32"))]
    fn load_data() -> Result<SaveData, String> {
        load_json(Self::get_save_path())
    }

    #[cfg(target_arch = "wasm32")]
    fn load_data() -> Result<SaveData, String> {
        load_from_slot(GAME_NAME, SAVE_SLOT)
    }

    fn validate_version(save_data: &SaveData) -> Result<(), String> {
        if save_data.version > SaveData::VERSION {
            return Err("Save file is from a newer version".to_string());
        }
        Ok(())
    }

    /// Get the save file path
    #[cfg(not(target_arch = "wasm32"))]
    fn get_save_path() -> std::path::PathBuf {
        get_app_data_path(GAME_NAME, SAVE_FILE)
            .unwrap_or_else(|| std::path::PathBuf::from(SAVE_FILE))
    }

    /// Save player stats to file
    pub fn save(player_stats: &PlayerStats) -> Result<(), String> {
        let save_data = SaveData::new(player_stats.clone());
        #[cfg(not(target_arch = "wasm32"))]
        {
            save_json(Self::get_save_path(), &save_data)
        }
        #[cfg(target_arch = "wasm32")]
        {
            save_to_slot(GAME_NAME, SAVE_SLOT, &save_data)
        }
    }

    /// Save player stats and a resumable active shift.
    pub fn save_run(player_stats: &PlayerStats, game_state: &GameState) -> Result<(), String> {
        let save_data = SaveData::with_run(player_stats.clone(), game_state);
        #[cfg(not(target_arch = "wasm32"))]
        {
            save_json(Self::get_save_path(), &save_data)
        }
        #[cfg(target_arch = "wasm32")]
        {
            save_to_slot(GAME_NAME, SAVE_SLOT, &save_data)
        }
    }

    /// Export the current save into a separate cross-platform backup slot.
    /// The browser exposes this as a visible button rather than requiring a
    /// filesystem download, while native builds receive the same recoverable
    /// copy in the toolkit save directory.
    pub fn export_save(
        player_stats: &PlayerStats,
        game_state: Option<&GameState>,
    ) -> Result<(), String> {
        let save_data = game_state
            .map(|state| SaveData::with_run(player_stats.clone(), state))
            .unwrap_or_else(|| SaveData::new(player_stats.clone()));
        save_to_slot(GAME_NAME, EXPORT_SLOT, &save_data)
    }

    /// Read and validate the separate exported save without replacing the
    /// primary save. The caller can show a recoverable error in the menu and
    /// decide when to install the imported record.
    pub fn import_save() -> Result<SaveData, String> {
        let save_data: SaveData = load_from_slot(GAME_NAME, EXPORT_SLOT)?;
        Self::validate_version(&save_data)?;
        Ok(save_data)
    }

    /// Load player stats from file
    pub fn load() -> Result<PlayerStats, String> {
        let save_data = Self::load_data()?;
        Self::validate_version(&save_data)?;
        Ok(save_data.player_stats)
    }

    /// Load the optional active-run checkpoint after applying supported
    /// additive migrations. Version 1 was meta-only, so it naturally returns
    /// `None` here and remains safe to overwrite as version 2.
    pub fn load_run() -> Result<Option<GameState>, String> {
        let save_data = Self::load_data()?;
        Self::validate_version(&save_data)?;
        Ok(save_data.run.map(|run| run.game_state))
    }

    /// Load the save, or set an unreadable one aside and start fresh.
    ///
    /// A load failure used to fall straight through to `PlayerStats::new()`,
    /// and the next auto-save overwrote the file — every meta-progression
    /// wiped by the very mechanism meant to keep it, silently. The bytes now
    /// survive under a quarantine name, and the caller gets a sentence for
    /// the menu. A save that simply does not exist is not a failure and
    /// reports nothing.
    /// The third return value says whether a new save may safely be written.
    /// It is false when quarantine failed, preserving the unreadable original.
    pub fn load_or_quarantine() -> (PlayerStats, Option<String>, bool, Option<GameState>) {
        match Self::load_data().and_then(|save_data| {
            Self::validate_version(&save_data)?;
            Ok(save_data)
        }) {
            Ok(save_data) => (
                save_data.player_stats,
                None,
                true,
                save_data.run.map(|run| run.game_state),
            ),
            Err(_) if !Self::save_exists() => (PlayerStats::new(), None, true, None),
            Err(error) => {
                let notice = Self::quarantine(&error);
                let can_save = !notice.contains("or set aside");
                (PlayerStats::new(), Some(notice), can_save, None)
            }
        }
    }

    /// Move the unreadable save aside and say what happened. Also fires for
    /// a save from a newer build: that file is healthy, but this binary
    /// would overwrite it on the next save, so setting it aside protects it
    /// just the same.
    #[cfg(not(target_arch = "wasm32"))]
    fn quarantine(error: &str) -> String {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_secs())
            .unwrap_or(0);
        let path = Self::get_save_path();
        let quarantined = path.with_file_name(format!("nightmare_shift_save.corrupt-{stamp}.json"));
        match std::fs::rename(&path, &quarantined) {
            Ok(()) => format!(
                "The old save could not be read ({error}). It was set aside as {} and a fresh record begun.",
                quarantined
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("a quarantine file")
            ),
            Err(rename_error) => format!(
                "The old save could not be read ({error}) or set aside ({rename_error}); the next save will overwrite it."
            ),
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn quarantine(error: &str) -> String {
        match macroquad_toolkit::persistence::quarantine_slot(GAME_NAME, SAVE_SLOT) {
            Ok(slot) => format!(
                "The old save could not be read ({error}). It was set aside in slot {slot:?} and a fresh record begun."
            ),
            Err(quarantine_error) => format!(
                "The old save could not be read ({error}) or set aside ({quarantine_error}); the next save will overwrite it."
            ),
        }
    }

    /// Check if a save file exists
    pub fn save_exists() -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            file_exists(Self::get_save_path())
        }
        #[cfg(target_arch = "wasm32")]
        {
            slot_exists(GAME_NAME, SAVE_SLOT)
        }
    }

    /// Delete the save file
    pub fn delete_save() -> Result<(), String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = Self::get_save_path();
            if path.exists() {
                std::fs::remove_file(path).map_err(|e| e.to_string())?;
            }
            Ok(())
        }
        #[cfg(target_arch = "wasm32")]
        {
            delete_slot(GAME_NAME, SAVE_SLOT)
        }
    }
}

#[cfg(test)]
mod tests;

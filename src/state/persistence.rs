//! Save/Load persistence system.

use super::PlayerStats;
#[cfg(target_arch = "wasm32")]
use macroquad_toolkit::persistence::{delete_slot, load_from_slot, save_to_slot, slot_exists};
#[cfg(not(target_arch = "wasm32"))]
use macroquad_toolkit::persistence::{file_exists, get_app_data_path, load_json, save_json};
use serde::{Deserialize, Serialize};

/// Save file name
const GAME_NAME: &str = "nightmare_shift";
#[cfg(not(target_arch = "wasm32"))]
const SAVE_FILE: &str = "nightmare_shift_save.json";
#[cfg(target_arch = "wasm32")]
const SAVE_SLOT: &str = "autosave";

/// Save data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveData {
    pub version: u32,
    pub player_stats: PlayerStats,
}

impl SaveData {
    /// Current save format version
    pub const VERSION: u32 = 1;

    /// Create new save data from player stats
    pub fn new(player_stats: PlayerStats) -> Self {
        Self {
            version: Self::VERSION,
            player_stats,
        }
    }
}

/// Persistence system for save/load
pub struct Persistence;

impl Persistence {
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

    /// Load player stats from file
    pub fn load() -> Result<PlayerStats, String> {
        #[cfg(not(target_arch = "wasm32"))]
        let save_data: SaveData = load_json(Self::get_save_path())?;
        #[cfg(target_arch = "wasm32")]
        let save_data: SaveData = load_from_slot(GAME_NAME, SAVE_SLOT)?;

        // Version check for future migrations
        if save_data.version > SaveData::VERSION {
            return Err("Save file is from a newer version".to_string());
        }

        Ok(save_data.player_stats)
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
    pub fn load_or_quarantine() -> (PlayerStats, Option<String>, bool) {
        match Self::load() {
            Ok(stats) => (stats, None, true),
            Err(_) if !Self::save_exists() => (PlayerStats::new(), None, true),
            Err(error) => {
                let notice = Self::quarantine(&error);
                let can_save = !notice.contains("or set aside");
                (PlayerStats::new(), Some(notice), can_save)
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

//! Persistent keyboard bindings for the decisions that need a shortcut.

use serde::{Deserialize, Serialize};

/// Player-facing shortcuts that can be remapped without changing the visible
/// button path. Values are display labels so they survive native and browser
/// saves without serializing platform-specific key enums.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBindings {
    #[serde(default = "default_accept")]
    pub accept: String,
    #[serde(default = "default_decline")]
    pub decline: String,
    #[serde(default = "default_follow")]
    pub follow: String,
    #[serde(rename = "break", default = "default_break")]
    pub break_guideline: String,
    #[serde(default = "default_pause")]
    pub pause: String,
}

fn default_accept() -> String {
    "SPACE".to_string()
}

fn default_decline() -> String {
    "D".to_string()
}

fn default_follow() -> String {
    "F".to_string()
}

fn default_break() -> String {
    "B".to_string()
}

fn default_pause() -> String {
    "ESC".to_string()
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            accept: default_accept(),
            decline: default_decline(),
            follow: default_follow(),
            break_guideline: default_break(),
            pause: default_pause(),
        }
    }
}

impl KeyBindings {
    fn cycle(value: &mut String, options: &[&str], other: &[&String]) {
        let current = options
            .iter()
            .position(|option| *option == value)
            .unwrap_or(0);
        let next = options
            .iter()
            .cycle()
            .skip(current + 1)
            .find(|option| !other.iter().any(|used| used.as_str() == **option))
            .unwrap_or(&options[current]);
        *value = (*next).to_string();
    }

    pub fn cycle_accept(&mut self) {
        let other = [
            &self.decline,
            &self.follow,
            &self.break_guideline,
            &self.pause,
        ];
        Self::cycle(&mut self.accept, &["SPACE", "ENTER"], &other);
    }

    pub fn cycle_decline(&mut self) {
        let other = [
            &self.accept,
            &self.follow,
            &self.break_guideline,
            &self.pause,
        ];
        Self::cycle(&mut self.decline, &["D", "X"], &other);
    }

    pub fn cycle_follow(&mut self) {
        let other = [
            &self.accept,
            &self.decline,
            &self.break_guideline,
            &self.pause,
        ];
        Self::cycle(&mut self.follow, &["F", "J"], &other);
    }

    pub fn cycle_break(&mut self) {
        let other = [&self.accept, &self.decline, &self.follow, &self.pause];
        Self::cycle(&mut self.break_guideline, &["B", "K"], &other);
    }

    pub fn cycle_pause(&mut self) {
        let other = [
            &self.accept,
            &self.decline,
            &self.follow,
            &self.break_guideline,
        ];
        Self::cycle(&mut self.pause, &["ESC", "P"], &other);
    }

    /// Report collisions in imported or hand-edited settings instead of
    /// silently making one action unreachable.
    pub fn conflicts(&self) -> Vec<String> {
        let entries = [
            ("accept", self.accept.as_str()),
            ("decline", self.decline.as_str()),
            ("follow", self.follow.as_str()),
            ("break", self.break_guideline.as_str()),
            ("pause", self.pause.as_str()),
        ];
        let mut conflicts = Vec::new();
        for (index, (name, key)) in entries.iter().enumerate() {
            for (other, other_key) in entries.iter().skip(index + 1) {
                if key == other_key {
                    conflicts.push(format!("{key}: {name} and {other}"));
                }
            }
        }
        conflicts
    }
}

#[cfg(test)]
mod tests {
    use super::KeyBindings;

    #[test]
    fn defaults_are_unique_and_cycles_keep_them_unique() {
        let mut bindings = KeyBindings::default();
        bindings.cycle_accept();
        bindings.cycle_decline();
        bindings.cycle_follow();
        bindings.cycle_break();
        bindings.cycle_pause();
        assert!(bindings.conflicts().is_empty());
    }

    #[test]
    fn imported_conflicts_are_reported() {
        let mut bindings = KeyBindings::default();
        bindings.decline = bindings.accept.clone();
        assert_eq!(bindings.conflicts(), vec!["SPACE: accept and decline"]);
    }
}

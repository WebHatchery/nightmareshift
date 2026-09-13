//! Informational ambience, tension, and supernatural stingers.
//!
//! Loading failures degrade to captions and silence so headless capture,
//! browsers awaiting a user gesture, and systems without an audio device can
//! still play every decision path.

use crate::screens::Screen;
use crate::state::{AccessibilitySettings, GameState, NeedStage};
use macroquad::audio::{play_sound, set_sound_volume, stop_sound, PlaySoundParams, Sound};
use macroquad_toolkit::assets::AssetPack;
use macroquad_toolkit::audio::load_sound_from_pack_or_file;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cue {
    Engine,
    Rain,
    Tension,
    Warning,
    Violation,
    Ward,
    Brink,
    Meltdown,
    Success,
}

impl Cue {
    const ALL: [Self; 9] = [
        Self::Engine,
        Self::Rain,
        Self::Tension,
        Self::Warning,
        Self::Violation,
        Self::Ward,
        Self::Brink,
        Self::Meltdown,
        Self::Success,
    ];

    fn path(self) -> &'static str {
        match self {
            Self::Engine => "assets/sounds/engine_ambience.wav",
            Self::Rain => "assets/sounds/rain_ambience.wav",
            Self::Tension => "assets/sounds/tension_pulse.wav",
            Self::Warning => "assets/sounds/warning.wav",
            Self::Violation => "assets/sounds/violation.wav",
            Self::Ward => "assets/sounds/ward.wav",
            Self::Brink => "assets/sounds/brink.wav",
            Self::Meltdown => "assets/sounds/meltdown.wav",
            Self::Success => "assets/sounds/success.wav",
        }
    }

    pub fn from_authored(name: &str) -> Self {
        match name {
            "child_distress" => Self::Tension,
            "distressed_breathing" => Self::Tension,
            "haunting_hum" => Self::Rain,
            "hunger_growl" => Self::Brink,
            "labored_breathing" => Self::Tension,
            "panicked_plea" => Self::Brink,
            "voice_escalation" => Self::Brink,
            "violation" => Self::Violation,
            "ward" => Self::Ward,
            "brink" => Self::Brink,
            "meltdown" => Self::Meltdown,
            "success" => Self::Success,
            _ => Self::Warning,
        }
    }
}

pub struct AudioMixer {
    sounds: HashMap<Cue, Sound>,
    loops: HashSet<Cue>,
}

impl AudioMixer {
    pub async fn load() -> Self {
        let pack = match AssetPack::load("assets.zip").await {
            Ok(pack) => Some(pack),
            Err(error) => {
                eprintln!("Asset pack unavailable; using loose audio files: {error}");
                None
            }
        };
        let mut sounds = HashMap::new();
        for cue in Cue::ALL {
            match load_sound_from_pack_or_file(pack.as_ref(), cue.path()).await {
                Ok(sound) => {
                    sounds.insert(cue, sound);
                }
                Err(error) => eprintln!("Audio unavailable for {}: {error}", cue.path()),
            }
        }
        Self {
            sounds,
            loops: HashSet::new(),
        }
    }

    fn volume(settings: &AccessibilitySettings, channel: u8) -> f32 {
        settings.master_volume as f32 / 100.0 * channel as f32 / 100.0
    }

    fn set_loop(&mut self, cue: Cue, wanted: bool, volume: f32) {
        let Some(sound) = self.sounds.get(&cue) else {
            return;
        };
        if wanted {
            if self.loops.insert(cue) {
                play_sound(
                    sound,
                    PlaySoundParams {
                        looped: true,
                        volume,
                    },
                );
            } else {
                set_sound_volume(sound, volume);
            }
        } else if self.loops.remove(&cue) {
            stop_sound(sound);
        }
    }

    fn play(&self, cue: Cue, volume: f32) {
        if volume <= 0.0 {
            return;
        }
        if let Some(sound) = self.sounds.get(&cue) {
            play_sound(
                sound,
                PlaySoundParams {
                    looped: false,
                    volume,
                },
            );
        }
    }

    /// Short UI feedback uses the same effects channel as authored stingers,
    /// and remains optional because captions and button state carry the
    /// meaning even when sound is muted or unavailable.
    pub fn play_ui_feedback(&self, confirm: bool, settings: &AccessibilitySettings) {
        let cue = if confirm { Cue::Success } else { Cue::Warning };
        self.play(cue, Self::volume(settings, settings.effects_volume) * 0.22);
    }

    /// Keep layered ambience aligned with current weather and need, then drain
    /// the one-shot cue queue. Audio conveys state already shown by meters,
    /// reactions and captions; it never owns a rule or required fact alone.
    pub fn sync(
        &mut self,
        screen: Screen,
        state: &mut GameState,
        settings: &AccessibilitySettings,
    ) {
        let in_shift = screen == Screen::Game;
        let ambience = Self::volume(settings, settings.ambience_volume);
        let music = Self::volume(settings, settings.music_volume);
        let effects = Self::volume(settings, settings.effects_volume);
        self.set_loop(Cue::Engine, in_shift, ambience * 0.28);

        let wet = matches!(
            state.current_weather.weather_type,
            crate::data::WeatherType::Rain | crate::data::WeatherType::Thunderstorm
        );
        self.set_loop(Cue::Rain, in_shift && wet, ambience * 0.34);

        let tension = state
            .current_passenger_need_state
            .as_ref()
            .map(|need| match need.stage {
                NeedStage::Calm => 0.0,
                NeedStage::Warning => 0.28,
                NeedStage::Critical => 0.55,
                NeedStage::Meltdown => 0.85,
            })
            .unwrap_or(0.0);
        self.set_loop(Cue::Tension, in_shift && tension > 0.0, music * tension);

        if let Some(event) = state.pending_audio.take() {
            self.play(Cue::from_authored(&event.cue), effects * 0.72);
        }
    }
}

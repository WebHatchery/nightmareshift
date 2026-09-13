use nightmare_shift::audio::Cue;

#[test]
fn every_authored_passenger_cue_uses_a_specific_audio_layer() {
    for cue in [
        "child_distress",
        "distressed_breathing",
        "haunting_hum",
        "hunger_growl",
        "labored_breathing",
        "panicked_plea",
        "voice_escalation",
    ] {
        assert_ne!(Cue::from_authored(cue), Cue::Warning, "{cue} fell through");
    }
}

#[test]
fn unknown_audio_cues_still_degrade_to_the_safe_warning_layer() {
    assert_eq!(Cue::from_authored("future_cue"), Cue::Warning);
}

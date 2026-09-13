use super::*;
use nightmare_shift::data::loader::load_epilogues;

fn rng() -> macroquad_toolkit::rng::SeededRng {
    macroquad_toolkit::rng::SeededRng::new(0xE9)
}

/// Every ending the game can reach must have words waiting: each kind
/// needs an unconditional entry, and each game-over cause resolves.
#[test]
fn every_ending_finds_an_epilogue() {
    let deck = load_epilogues();
    assert!(!deck.is_empty(), "the epilogue deck is empty");
    for kind in [
        EpilogueKind::RunComplete,
        EpilogueKind::DeathDelivered,
        EpilogueKind::GameOver,
    ] {
        for cause in [
            None,
            Some(EpilogueCause::Meltdown),
            Some(EpilogueCause::HiddenRule),
            Some(EpilogueCause::OutOfNight),
            Some(EpilogueCause::LastFareFailed),
        ] {
            for clean in [false, true] {
                for first in [false, true] {
                    let facts = EndingFacts {
                        kind,
                        cause,
                        clean_night: clean,
                        first_of_its_kind: first,
                    };
                    assert!(
                        select_epilogue(&deck, facts, &mut rng()).is_some(),
                        "no epilogue for {kind:?}/{cause:?}/clean={clean}/first={first}"
                    );
                }
            }
        }
    }
}

/// No entry may author an empty paragraph, and every entry needs at
/// least one variant.
#[test]
fn every_entry_has_words() {
    for entry in load_epilogues() {
        assert!(
            !entry.texts.is_empty(),
            "{:?}/{:?} authors no variants",
            entry.kind,
            entry.cause
        );
        for text in &entry.texts {
            assert!(
                text.trim().len() > 40,
                "{:?}/{:?} authors a stub: {text:?}",
                entry.kind,
                entry.cause
            );
        }
    }
}

/// The specific beats the general: a clean run-complete draws from the
/// clean-night entry, not the pooled default.
#[test]
fn the_most_specific_entry_wins() {
    let deck = load_epilogues();
    let clean = EndingFacts {
        kind: EpilogueKind::RunComplete,
        cause: None,
        clean_night: true,
        first_of_its_kind: false,
    };
    let clean_texts: Vec<&String> = deck
        .iter()
        .filter(|entry| entry.kind == EpilogueKind::RunComplete && entry.requires_clean_night)
        .flat_map(|entry| entry.texts.iter())
        .collect();
    assert!(!clean_texts.is_empty(), "no clean-night entry authored");
    for _ in 0..20 {
        let chosen = select_epilogue(&deck, clean, &mut rng()).expect("an epilogue");
        assert!(
            clean_texts.iter().any(|text| **text == chosen),
            "a clean night drew a generic epilogue: {chosen:?}"
        );
    }
}

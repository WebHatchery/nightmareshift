//! Authored ending epilogues, matching `epilogueData.json`.
//!
//! The campaign's endings were mechanically distinct and textually thin: a
//! title, one subtitle line, a button. Each ending now selects a paragraph
//! from an authored deck — filtered by what actually happened (which ending,
//! what killed the run, whether the night was driven clean, whether this is
//! the first of its kind) and drawn on the seeded stream, so a seeded run
//! ends on the same words every time.

use serde::{Deserialize, Serialize};

/// Which ending is being written up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EpilogueKind {
    /// Five ordinary nights survived; the run closed at dawn.
    RunComplete,
    /// The Last Fare delivered — the true ending.
    DeathDelivered,
    /// The run died. `cause` narrows which way.
    GameOver,
}

/// How a dead run died, bucketed for epilogue selection. Derived from the
/// death site's authored reason at `end_shift`, where the strings are made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EpilogueCause {
    /// A passenger's need went past the brink.
    Meltdown,
    /// An unposted rule, found the hard way.
    HiddenRule,
    /// Fuel, clock, or quota — arithmetic, not teeth.
    OutOfNight,
    /// Dawn broke on The Last Fare with Death still waiting.
    LastFareFailed,
}

/// One authored epilogue entry: a kind, optional narrowing conditions, and
/// the variant paragraphs the selector may draw from.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Epilogue {
    pub kind: EpilogueKind,
    /// For `GameOver`: only matches this cause. Absent matches any.
    #[serde(default)]
    pub cause: Option<EpilogueCause>,
    /// Only matches a night driven without a single rule violation.
    #[serde(default)]
    pub requires_clean_night: bool,
    /// Only matches the first run completion / first delivery ever.
    #[serde(default)]
    pub first_of_its_kind: bool,
    pub texts: Vec<String>,
}

/// The facts an ending is judged by.
#[derive(Debug, Clone, Copy)]
pub struct EndingFacts {
    pub kind: EpilogueKind,
    pub cause: Option<EpilogueCause>,
    pub clean_night: bool,
    pub first_of_its_kind: bool,
}

/// Pick the epilogue paragraph for `facts`, most specific entry first.
///
/// Specificity is the count of narrowing conditions an entry sets; among the
/// most specific matches the variants pool together and the seeded stream
/// draws one. Returns `None` only when the deck authors nothing for the
/// kind at all — the tests hold every kind and cause to at least one
/// unconditional entry, so in practice an ending always has words.
pub fn select_epilogue(
    deck: &[Epilogue],
    facts: EndingFacts,
    rng: &mut macroquad_toolkit::rng::SeededRng,
) -> Option<String> {
    let matching: Vec<&Epilogue> = deck
        .iter()
        .filter(|entry| entry.kind == facts.kind)
        .filter(|entry| entry.cause.is_none() || entry.cause == facts.cause)
        .filter(|entry| !entry.requires_clean_night || facts.clean_night)
        .filter(|entry| !entry.first_of_its_kind || facts.first_of_its_kind)
        .collect();

    let specificity = |entry: &Epilogue| {
        entry.cause.is_some() as u32
            + entry.requires_clean_night as u32
            + entry.first_of_its_kind as u32
    };
    let best = matching.iter().map(|entry| specificity(entry)).max()?;
    let pool: Vec<&str> = matching
        .iter()
        .filter(|entry| specificity(entry) == best)
        .flat_map(|entry| entry.texts.iter().map(String::as_str))
        .collect();
    rng.choose(&pool).map(|text| text.to_string())
}

//! What the almanac tells you about the passenger in front of you.
//!
//! `almanacData.json` promises a concrete reward at each knowledge level —
//! Lv.1 "Basic Needs", Lv.2 "Common Tells", Lv.3 "Hidden Rules"/"True Nature".
//! This module turns a passenger plus a knowledge level into those lines, so
//! the ride-request screen and the almanac screen agree on what a level buys
//! and neither can drift from the authored rewards.

use crate::data::{GameData, NeedType, Passenger, TellIntensity};
use crate::engine::{GameEngine, RideService};
use crate::state::NeedStage;

/// One revealed fact about a passenger.
pub struct DossierLine {
    /// Short caption, e.g. `"Need"`.
    pub label: String,
    /// The revealed value.
    pub value: String,
    /// Knowledge level that unlocked this line (1-3), used for colouring.
    pub level: u32,
}

impl DossierLine {
    fn new(label: &str, value: String, level: u32) -> Self {
        Self {
            label: label.to_string(),
            value,
            level,
        }
    }
}

/// Human-readable name for a need type.
pub fn need_label(need: NeedType) -> &'static str {
    match need {
        NeedType::Hunger => "Hunger",
        NeedType::Fear => "Fear",
        NeedType::Wrath => "Wrath",
        NeedType::Decay => "Decay",
        NeedType::Loneliness => "Loneliness",
        NeedType::Unknown => "Unreadable",
    }
}

/// Plain description of how much a passenger covers what they are.
fn candour_label(deception: f32) -> &'static str {
    match deception {
        d if d <= 0.05 => "Hides nothing; their tells read true",
        d if d <= 0.25 => "Mostly straight; the odd sign slips past",
        d if d <= 0.45 => "Guarded; expect to miss things",
        d if d <= 0.65 => "Covers well; absence of a tell proves nothing",
        _ => "Practised liar; trust the almanac over your eyes",
    }
}

/// How a need stage reads as a moment to act on, rather than as a label.
/// Each of these contains `NeedStage::label`, so the almanac and the driving
/// readout name a passenger's condition the same way. A test holds that.
pub fn stage_phrase(stage: NeedStage) -> &'static str {
    match stage {
        NeedStage::Calm => "they are still settled",
        NeedStage::Warning => "they turn restless",
        NeedStage::Critical => "they are close to breaking",
        NeedStage::Meltdown => "they are breaking down",
    }
}

fn intensity_label(intensity: TellIntensity) -> &'static str {
    match intensity {
        TellIntensity::Subtle => "subtle",
        TellIntensity::Moderate => "moderate",
        TellIntensity::Obvious => "obvious",
    }
}

/// Build the facts a player with `knowledge_level` knows about `passenger`.
///
/// Returns an empty list at level 0 — an unstudied passenger reveals nothing
/// beyond what the ride request already shows.
/// What the driver brings to a reading: the shift in progress and what they
/// have unlocked.
///
/// A struct rather than more parameters. Two of these lines were facts that
/// should have been plans -- the relief that needs its rule on tonight's board,
/// and the traits that need the matching skill bought -- and each fix wanted a
/// different piece of context. Growing the argument list a fourth time is how a
/// call site starts passing the wrong thing.
pub struct DriverContext<'a> {
    pub shift: &'a crate::state::GameState,
    pub stats: &'a crate::state::PlayerStats,
}

/// `driver` is the shift in progress and what the driver has unlocked, used to
/// say whether what this passenger needs is actually available. Pass `None`
/// from anywhere outside a shift.
pub fn build(
    passenger: &Passenger,
    knowledge_level: u32,
    data: Option<&GameData>,
    driver: Option<&DriverContext>,
) -> Vec<DossierLine> {
    let mut lines = Vec::new();

    // The driver's own memory needs no almanac. Reputation moves the fare
    // and the route risk, and until this line it moved them from nowhere
    // any screen could see.
    if let Some(driver) = driver {
        if let Some(reputation) = driver.shift.passenger_reputation.get(&passenger.id) {
            if reputation.interactions > 0 {
                lines.push(DossierLine::new(
                    "Standing",
                    format!(
                        "{} - {} kindnesses over {} dealings",
                        reputation.relationship_level.label(),
                        reputation.positive_choices,
                        reputation.interactions
                    ),
                    1,
                ));
            }
        }
    }

    // What the driver has personally witnessed needs no almanac either. A
    // noticed tell is inscribed at ride completion and kept across runs; at
    // Studied and above the catalogued tells below take over, so only the
    // ones the catalogue does not already name are repeated here.
    if let Some(driver) = driver {
        let entry = driver.stats.get_almanac_entry(passenger.id);
        for seen in entry.tells_seen.iter().filter(|seen| {
            knowledge_level < 2
                || !passenger
                    .tells
                    .iter()
                    .take(3)
                    .any(|tell| tell.description == **seen)
        }) {
            lines.push(DossierLine::new("Witnessed", seen.clone(), 1));
        }
    }

    // Lv.1 "Observed" — name, description, and basic needs. The player can
    // already see name and description on the request, so the payoff here is
    // knowing what the passenger needs and where it starts to slip.
    if knowledge_level >= 1 {
        if let Some(profile) = &passenger.state_profile {
            lines.push(DossierLine::new(
                "Need",
                format!(
                    "{} - restless past {}, critical at {}",
                    need_label(profile.need_type),
                    profile.thresholds.warning,
                    profile.thresholds.critical
                ),
                1,
            ));
        }
        if !passenger.traits.is_empty() {
            // Which of these the driver can actually call on.
            //
            // A trait is half a mechanic: the other half is the matching
            // `ability_unlock` skill, and without it the trait grants nothing.
            // Listing them unqualified is the same fault the relief line had --
            // a fact where a plan was wanted. The skill tree already counts
            // this from the other side, telling the player how many carriers of
            // a trait they have studied.
            let usable: Vec<&str> = driver
                .map(|driver| {
                    passenger
                        .traits
                        .iter()
                        .filter(|name| {
                            driver
                                .stats
                                .is_skill_unlocked(&RideService::trait_skill_id(name))
                        })
                        .map(String::as_str)
                        .collect()
                })
                .unwrap_or_default();
            let value = match usable.as_slice() {
                [] if driver.is_some() => format!(
                    "{} - you are trained in none of these",
                    passenger.traits.join(", ")
                ),
                [] => passenger.traits.join(", "),
                trained => format!(
                    "{} - you can call on {}",
                    passenger.traits.join(", "),
                    trained.join(" and ")
                ),
            };
            lines.push(DossierLine::new("Traits", value, 1));
        }
    }

    // Lv.2 "Studied" — the tells to watch for, and what they are known to
    // carry. Catalogued up front rather than only after one fires this ride.
    if knowledge_level >= 2 {
        for tell in passenger.tells.iter().take(3) {
            lines.push(DossierLine::new(
                "Tell",
                format!("{} ({})", tell.description, intensity_label(tell.intensity)),
                2,
            ));
        }
        if !passenger.items.is_empty() {
            lines.push(DossierLine::new("Carries", passenger.items.join(", "), 2));
        }
        // Who they are connected to. The same `relationships` list makes an
        // associate more likely to turn up later in the shift, so knowing it
        // is knowing who the night is about to bring round.
        if let Some(data) = data {
            let names: Vec<&str> = passenger
                .relationships
                .iter()
                .filter_map(|id| data.passengers.iter().find(|p| p.id == *id))
                .map(|p| p.name.as_str())
                .collect();
            if !names.is_empty() {
                lines.push(DossierLine::new("Associates", names.join(", "), 2));
            }
        }
    }

    // Lv.3 "Mastered" — the passenger's own rule, and what they truly are.
    if knowledge_level >= 3 {
        if !passenger.personal_rule.is_empty() {
            lines.push(DossierLine::new(
                "Their rule",
                passenger.personal_rule.clone(),
                3,
            ));
        }
        if !passenger.supernatural.is_empty() {
            lines.push(DossierLine::new(
                "True nature",
                passenger.supernatural.clone(),
                3,
            ));
        }
        // What actually settles them, and when.
        //
        // The engine grants a passenger's exception only for the guideline
        // that owns their `exceptionId`, and only once they have reached the
        // stage that exception authors. Both facts decided whether a ride
        // survived and neither was reachable from anywhere in the game — a
        // player could study a passenger to Mastered and still be guessing
        // which forbidden action would calm them. This is the line the
        // almanac exists to sell.
        if let (Some(data), Some(profile)) = (data, passenger.state_profile.as_ref()) {
            if let Some(exception_id) = profile.exception_id.as_deref() {
                if let Some((guideline, exception)) = data
                    .guidelines
                    .iter()
                    .flat_map(|guideline| {
                        guideline
                            .exceptions
                            .iter()
                            .map(move |exception| (guideline, exception))
                    })
                    .find(|(_, exception)| exception.id == exception_id)
                {
                    // Whether that rule is even in force tonight.
                    //
                    // Breaking a guideline does nothing unless the rule it
                    // belongs to was drawn for this shift: without it the
                    // action is not a violation, so no exception fires and no
                    // relief is paid. The line named the rule and stopped,
                    // which made a studied passenger's most useful fact
                    // actionable only by luck. `passengers_rule_in_force` is
                    // the same lookup the obey pressure and the follow reward
                    // already use.
                    let in_force = driver.map(|driver| {
                        driver
                            .shift
                            .current_rules
                            .iter()
                            .chain(driver.shift.hidden_rules.iter())
                            .any(|rule| rule.related_guideline_id == Some(guideline.id))
                    });
                    let availability = match in_force {
                        Some(true) => " - that rule is in force",
                        Some(false) => " - but that rule is not in force tonight",
                        None => "",
                    };
                    lines.push(DossierLine::new(
                        "Relief",
                        format!(
                            "break \"{}\" once {}{}",
                            guideline.title,
                            stage_phrase(GameEngine::required_stage(exception)),
                            availability
                        ),
                        3,
                    ));
                }
            }
        }
        // How much to trust what you see them do. `deceptionLevel` now scales
        // tell detection, so knowing it is knowing whether an absent tell
        // means calm or means well hidden.
        lines.push(DossierLine::new(
            "Candour",
            candour_label(passenger.deception_level).to_string(),
            3,
        ));
    }

    lines
}

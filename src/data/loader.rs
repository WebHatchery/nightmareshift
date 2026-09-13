//! Data loading from embedded JSON files.

use super::*;
use std::collections::HashSet;

/// All game data loaded from JSON files
pub struct GameData {
    pub passengers: Vec<Passenger>,
    pub rules: Vec<Rule>,
    pub locations: Vec<Location>,
    pub constants: ConstantsData,
    pub skills: Vec<Skill>,
    pub almanac: AlmanacData,
    pub guidelines: Vec<Guideline>,
    pub localization: Localization,
    pub events: Vec<EventTemplate>,
    pub item_pools: ItemPools,
    pub items: ItemCatalog,
    pub rewards: RewardData,
    pub night_modifiers: NightModifierData,
    pub epilogues: Vec<Epilogue>,
    /// Content files that failed to parse and fell back empty. The night
    /// can still run without them — thinner — and the menu says so, since
    /// a stderr line reaches nobody on the web build.
    pub load_errors: Vec<String>,
}

impl GameData {
    /// Load all game data from embedded JSON files.
    ///
    /// Errs only when the game cannot run at all: `constants.json` and the
    /// localization file are structural, and there is nothing sensible to
    /// fall back on. Content files that fail fall back empty and are
    /// reported in `load_errors` instead — a missing roster is a broken
    /// night, not a broken program.
    pub fn load() -> Result<Self, String> {
        let mut data = Self {
            passengers: load_passengers(),
            rules: load_rules(),
            locations: load_locations(),
            constants: try_load_constants()?,
            skills: load_skill_tree(),
            almanac: load_almanac(),
            guidelines: load_guidelines(),
            localization: try_load_localization()?,
            events: load_events(),
            item_pools: load_item_pools(),
            items: load_item_catalog(),
            rewards: load_rewards(),
            night_modifiers: load_night_modifiers(),
            epilogues: load_epilogues(),
            load_errors: Vec::new(),
        };
        for (file, empty) in [
            ("passengerData.json", data.passengers.is_empty()),
            ("shiftRulesData.json", data.rules.is_empty()),
            ("locationData.json", data.locations.is_empty()),
            ("skillTreeData.json", data.skills.is_empty()),
            ("guidelineData.json", data.guidelines.is_empty()),
            ("eventData.json", data.events.is_empty()),
            ("itemPoolData.json", data.item_pools.all_names().is_empty()),
            ("itemData.json", data.items.is_empty()),
            ("rewardData.json", data.rewards.achievements.is_empty()),
            (
                "nightModifierData.json",
                data.night_modifiers.modifiers.is_empty(),
            ),
            ("epilogueData.json", data.epilogues.is_empty()),
            ("almanacData.json", data.almanac.levels.is_empty()),
        ] {
            if empty {
                data.load_errors
                    .push(format!("{file} failed to load; its content is missing"));
            }
        }
        if let Err(error) = data.validate() {
            data.load_errors.push(error);
        }
        Ok(data)
    }

    /// Find a location by name
    pub fn get_location(&self, name: &str) -> Option<&Location> {
        self.locations.iter().find(|l| l.name == name)
    }

    /// Check references and balance invariants after every embedded deck is
    /// parsed. The error names the source file so the loading/menu warning is
    /// actionable on the web build, where stderr is not visible.
    pub fn validate(&self) -> Result<(), String> {
        fn duplicate_u32<I>(source: &str, ids: I) -> Result<(), String>
        where
            I: IntoIterator<Item = u32>,
        {
            let mut seen = HashSet::new();
            for id in ids {
                if !seen.insert(id) {
                    return Err(format!("{source}: duplicate id {id}"));
                }
            }
            Ok(())
        }

        fn duplicate_text<'a, I>(source: &str, ids: I) -> Result<(), String>
        where
            I: IntoIterator<Item = &'a str>,
        {
            let mut seen = HashSet::new();
            for id in ids {
                if !seen.insert(id) {
                    return Err(format!("{source}: duplicate id {id}"));
                }
            }
            Ok(())
        }

        duplicate_u32(
            "passengerData.json",
            self.passengers.iter().map(|passenger| passenger.id),
        )?;
        duplicate_u32("shiftRulesData.json", self.rules.iter().map(|rule| rule.id))?;
        duplicate_u32(
            "guidelineData.json",
            self.guidelines.iter().map(|guideline| guideline.id),
        )?;
        duplicate_text(
            "locationData.json",
            self.locations.iter().map(|location| location.name.as_str()),
        )?;
        duplicate_text(
            "eventData.json",
            self.events.iter().map(|event| event.id.as_str()),
        )?;
        duplicate_text(
            "nightModifierData.json",
            self.night_modifiers
                .modifiers
                .iter()
                .map(|modifier| modifier.id.as_str()),
        )?;

        let passenger_ids: HashSet<u32> = self
            .passengers
            .iter()
            .map(|passenger| passenger.id)
            .collect();
        let location_names: HashSet<&str> = self
            .locations
            .iter()
            .map(|location| location.name.as_str())
            .collect();
        let rule_ids: HashSet<u32> = self.rules.iter().map(|rule| rule.id).collect();
        let guideline_ids: HashSet<u32> = self
            .guidelines
            .iter()
            .map(|guideline| guideline.id)
            .collect();
        let guideline_exception_ids: HashSet<&str> = self
            .guidelines
            .iter()
            .flat_map(|guideline| {
                guideline
                    .exceptions
                    .iter()
                    .map(|exception| exception.id.as_str())
            })
            .collect();
        let traits: HashSet<&str> = self
            .passengers
            .iter()
            .flat_map(|passenger| passenger.traits.iter().map(String::as_str))
            .collect();

        for passenger in &self.passengers {
            for location in [&passenger.pickup, &passenger.destination] {
                if !location_names.contains(location.as_str()) {
                    return Err(format!(
                        "passengerData.json: passenger {} references missing location {location}",
                        passenger.id
                    ));
                }
            }
            if passenger.personal_rule.trim().is_empty() {
                return Err(format!(
                    "passengerData.json: passenger {} has an empty personal rule",
                    passenger.id
                ));
            }
            for relationship in &passenger.relationships {
                if !passenger_ids.contains(relationship) {
                    return Err(format!(
                        "passengerData.json: passenger {} references missing relationship {}",
                        passenger.id, relationship
                    ));
                }
            }
            for exception in &passenger.guideline_exceptions {
                if !guideline_exception_ids.contains(exception.as_str()) {
                    return Err(format!(
                        "passengerData.json: passenger {} references missing guideline exception {exception}",
                        passenger.id
                    ));
                }
            }
            for item in passenger
                .drop_items
                .iter()
                .chain(passenger.wanted_items.iter())
                .chain(passenger.trade_reward.iter())
            {
                if !self.items.contains(item) {
                    return Err(format!(
                        "itemData.json: passenger {} references missing item {item}",
                        passenger.id
                    ));
                }
            }
        }
        for location in &self.locations {
            if location.fare_modifier <= 0.0
                || location.distance_multiplier <= 0.0
                || location.fuel_multiplier <= 0.0
                || location.spawn_affinity <= 0.0
                || location.destination_risk < 0.0
            {
                return Err(format!(
                    "locationData.json: location {} has an invalid route or spawn modifier",
                    location.name
                ));
            }
        }
        for rule in &self.rules {
            if let Some(guideline) = rule.related_guideline_id {
                if !guideline_ids.contains(&guideline) {
                    return Err(format!(
                        "shiftRulesData.json: rule {} references missing guideline {guideline}",
                        rule.id
                    ));
                }
            }
            for conflict in &rule.conflicts_with {
                if !rule_ids.contains(conflict) {
                    return Err(format!(
                        "shiftRulesData.json: rule {} references missing conflict {conflict}",
                        rule.id
                    ));
                }
            }
        }
        for event in &self.events {
            for choice in &event.choices {
                if let Some(trait_name) = &choice.required_trait {
                    if !traits.contains(trait_name.as_str()) {
                        return Err(format!(
                            "eventData.json: event {} references missing trait {trait_name}",
                            event.id
                        ));
                    }
                }
            }
            if event.weight <= 0.0 {
                return Err(format!(
                    "eventData.json: event {} has non-positive weight",
                    event.id
                ));
            }
        }
        for modifier in &self.night_modifiers.modifiers {
            if modifier.weight == 0 || modifier.fare_mult <= 0.0 || modifier.quota_mult <= 0.0 {
                return Err(format!(
                    "nightModifierData.json: modifier {} has an invalid balance value",
                    modifier.id
                ));
            }
        }
        if !(0.0..=1.0).contains(&self.night_modifiers.chance)
            || self.constants.fuel.critical_fuel > self.constants.fuel.low_fuel_warning
            || self.constants.fuel.low_fuel_warning > self.constants.fuel.medium_fuel
            || self.constants.risk.max_risk_level < self.constants.risk.extreme_risk
            || self.constants.game_constants.guideline_decision_seconds <= 0.0
            || self.constants.fuel.partial_refuel_amount <= 0.0
        {
            return Err("constants.json: balance invariants are not valid".to_string());
        }
        for kind in [
            EpilogueKind::RunComplete,
            EpilogueKind::DeathDelivered,
            EpilogueKind::GameOver,
        ] {
            if !self
                .epilogues
                .iter()
                .any(|entry| entry.kind == kind && !entry.texts.is_empty())
            {
                return Err(format!(
                    "epilogueData.json: no non-empty entry for {kind:?}"
                ));
            }
        }
        Ok(())
    }
}

/// Load passengers from embedded JSON
pub fn load_passengers() -> Vec<Passenger> {
    macroquad_toolkit::include_json!("../../assets/passengerData.json").unwrap_or_else(|e| {
        eprintln!("Failed to parse passengers: {}", e);
        Vec::new()
    })
}

/// Load rules from embedded JSON
pub fn load_rules() -> Vec<Rule> {
    macroquad_toolkit::include_json!("../../assets/shiftRulesData.json").unwrap_or_else(|e| {
        eprintln!("Failed to parse rules: {}", e);
        Vec::new()
    })
}

/// Load locations from embedded JSON
pub fn load_locations() -> Vec<Location> {
    macroquad_toolkit::include_json!("../../assets/locationData.json").unwrap_or_else(|e| {
        eprintln!("Failed to parse locations: {}", e);
        Vec::new()
    })
}

/// Load constants, or say exactly which part of the file is wrong.
pub fn try_load_constants() -> Result<ConstantsData, String> {
    macroquad_toolkit::include_json!("../../assets/constants.json")
        .map_err(|e| format!("constants.json: {e}"))
}

/// Load constants from embedded JSON, panicking if the embedded asset is
/// malformed. The production loading path uses [`try_load_constants`] so it
/// can carry the error to the loading screen.
pub fn load_constants() -> ConstantsData {
    try_load_constants().expect("constants.json parses")
}

/// Load skill tree from embedded JSON (JSON is an array directly)
pub fn load_skill_tree() -> Vec<Skill> {
    macroquad_toolkit::include_json!("../../assets/skillTreeData.json").unwrap_or_else(|e| {
        eprintln!("Failed to parse skill tree: {}", e);
        Vec::new()
    })
}

/// Load the ending epilogues from embedded JSON. An unparseable file falls
/// back empty — endings show their title and subtitle, just no paragraph.
pub fn load_epilogues() -> Vec<Epilogue> {
    macroquad_toolkit::include_json!("../../assets/epilogueData.json").unwrap_or_else(|e| {
        eprintln!("Failed to parse epilogues: {}", e);
        Vec::new()
    })
}

/// Load the night-modifier deck from embedded JSON. An unparseable file
/// falls back to an empty deck — no modifier ever rolls, the campaign
/// still runs.
pub fn load_night_modifiers() -> NightModifierData {
    macroquad_toolkit::include_json!("../../assets/nightModifierData.json").unwrap_or_else(|e| {
        eprintln!("Failed to parse night modifiers: {}", e);
        NightModifierData {
            chance: 0.0,
            modifiers: Vec::new(),
        }
    })
}

/// Load almanac data from embedded JSON
pub fn load_almanac() -> AlmanacData {
    macroquad_toolkit::include_json!("../../assets/almanacData.json").unwrap_or_else(|e| {
        eprintln!("Failed to parse almanac: {}", e);
        AlmanacData {
            levels: std::collections::HashMap::new(),
            lore_costs: LoreCosts {
                level_1: 1,
                level_2: 3,
                level_3: 5,
            },
        }
    })
}

/// Load guidelines from embedded JSON
pub fn load_guidelines() -> Vec<Guideline> {
    macroquad_toolkit::include_json!("../../assets/guidelineData.json").unwrap_or_else(|e| {
        eprintln!("Failed to parse guidelines: {}", e);
        Vec::new()
    })
}

/// Load the mid-ride event deck from embedded JSON
pub fn load_events() -> Vec<EventTemplate> {
    macroquad_toolkit::include_json!("../../assets/eventData.json").unwrap_or_else(|e| {
        eprintln!("Failed to parse events: {}", e);
        Vec::new()
    })
}

/// Load item name pools from embedded JSON
pub fn load_item_pools() -> ItemPools {
    macroquad_toolkit::include_json!("../../assets/itemPoolData.json").unwrap_or_else(|e| {
        eprintln!("Failed to parse item pools: {}", e);
        ItemPools::default()
    })
}

/// Load the item catalog from embedded JSON. Every name any pool or passenger
/// can drop is defined here, so a dropped item always carries real effects
/// rather than being an inert keepsake.
pub fn load_item_catalog() -> ItemCatalog {
    macroquad_toolkit::include_json!("../../assets/itemData.json").unwrap_or_else(|e| {
        eprintln!("Failed to parse item catalog: {}", e);
        ItemCatalog::default()
    })
}

/// Load meta-progression payouts from embedded JSON.
pub fn load_rewards() -> RewardData {
    macroquad_toolkit::include_json!("../../assets/rewardData.json").unwrap_or_else(|e| {
        eprintln!("Failed to parse rewards: {}", e);
        RewardData::default()
    })
}

/// Load localization from embedded JSON, with glyphs the bundled font cannot
/// draw removed.
///
/// The UI strings are authored with emoji — a fuel pump on the fuel readout,
/// a shield on the survival skills, a skull on the game-over title. The font
/// has none of them, so they rendered as replacement boxes on the status bar,
/// the waiting screen, the skill tree, the trade offer and the ride summary.
/// One private helper in `ui::components` stripped them for the four status
/// bar fields and nothing else did, so the fix depended on each caller
/// remembering. Cleaning them out of the text once, here, means no caller can
/// forget — and the emoji stay in the JSON for a font that can draw them.
pub fn try_load_localization() -> Result<Localization, String> {
    try_load_localization_for("en")
}

/// Load a supported locale by overlaying its authored strings on the complete
/// English schema. Partial locale files can therefore add translated content
/// without making a missing key crash the loading screen.
pub fn try_load_localization_for(code: &str) -> Result<Localization, String> {
    let mut value: serde_json::Value =
        macroquad_toolkit::include_json!("../../assets/localization/en.json")?;
    if code.eq_ignore_ascii_case("es") {
        let overlay: serde_json::Value =
            macroquad_toolkit::include_json!("../../assets/localization/es.json")?;
        merge_json(&mut value, overlay);
    }
    strip_undrawable_glyphs(&mut value);
    serde_json::from_value(value).map_err(|e| format!("localization/{code} shape: {e}"))
}

/// Load the default English locale, panicking if the embedded schema is
/// malformed. The production loading path uses [`try_load_localization`] so
/// it can carry the error to the loading screen.
pub fn load_localization() -> Localization {
    try_load_localization().expect("localization/en.json parses")
}

/// Remove pictographs the bundled font cannot draw from every string.
///
/// The supported script is ASCII plus Latin accents and Spanish punctuation.
/// The old ASCII-only pass silently mangled translated words such as
/// `Español`; font coverage is prewarmed for this set in `ui::prewarm`.
///
/// Only strings that actually lost a character are trimmed, so the gap a
/// stripped prefix leaves does not show as a stray indent while deliberate
/// spacing — the leaderboard detail line is indented on purpose — survives.
pub fn strip_undrawable_glyphs(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(text) => {
            let cleaned: String = text
                .chars()
                .filter(|ch| {
                    ch.is_ascii()
                        || ('\u{00A1}'..='\u{00BF}').contains(ch)
                        || ('\u{00C0}'..='\u{024F}').contains(ch)
                })
                .collect();
            if cleaned.len() != text.len() {
                *text = cleaned.trim().to_string();
            }
        }
        serde_json::Value::Array(items) => items.iter_mut().for_each(strip_undrawable_glyphs),
        serde_json::Value::Object(map) => {
            map.values_mut().for_each(strip_undrawable_glyphs);
        }
        _ => {}
    }
}

fn merge_json(base: &mut serde_json::Value, overlay: serde_json::Value) {
    match base {
        serde_json::Value::Object(base_map) => {
            if let serde_json::Value::Object(overlay_map) = overlay {
                for (key, value) in overlay_map {
                    if let Some(existing) = base_map.get_mut(&key) {
                        merge_json(existing, value);
                    } else {
                        base_map.insert(key, value);
                    }
                }
            } else {
                *base = overlay;
            }
        }
        _ => *base = overlay,
    }
}

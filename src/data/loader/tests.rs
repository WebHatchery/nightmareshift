use super::*;

/// No localized string may reach the UI carrying a glyph the font cannot
/// draw. Each one renders as a replacement box, which is what the fuel
/// readout, the skill tree categories and the trade title were showing.
#[test]
fn localization_carries_no_undrawable_glyphs() {
    let json = macroquad_toolkit::include_json_str!("../../../assets/localization/en.json");
    let mut value: serde_json::Value = serde_json::from_str(json).expect("valid json");
    strip_undrawable_glyphs(&mut value);

    fn walk(value: &serde_json::Value, path: &str) {
        match value {
            serde_json::Value::String(text) => {
                let bad: Vec<char> = text.chars().filter(|ch| !ch.is_ascii()).collect();
                assert!(bad.is_empty(), "{path} still carries {bad:?}");
            }
            serde_json::Value::Array(items) => {
                for (i, item) in items.iter().enumerate() {
                    walk(item, &format!("{path}[{i}]"));
                }
            }
            serde_json::Value::Object(map) => {
                for (key, item) in map {
                    walk(item, &format!("{path}.{key}"));
                }
            }
            _ => {}
        }
    }
    walk(&value, "");
}

/// Text written in Rust gets no stripping pass either.
///
/// The two tests either side of this one walk JSON, which is where the
/// glyph problem was found and fixed. It was still living in the source:
/// an em dash in the almanac's need line and in both of the guideline
/// screen's almanac hints, a bullet in its decision log, an arrow between
/// a passenger's pickup and destination, and four emoji on the route
/// preference labels. The last of those never reached the screen only
/// because its single caller filtered non-ASCII characters itself before
/// drawing — which is the arrangement the loader's stripping pass exists
/// to replace, since it makes correctness depend on each caller
/// remembering.
///
/// Only string literals are checked. Comments are prose for people and
/// several here use em dashes deliberately.
#[test]
fn displayed_source_text_carries_no_undrawable_glyphs() {
    // Files whose literals reach a font, plus the data types that build
    // strings for them. Listed rather than globbed because `include_str!`
    // needs a literal path; a new screen should be added here.
    const SOURCES: [(&str, &str); 8] = [
        ("ui/components.rs", include_str!("../../ui/components.rs")),
        ("data/passenger.rs", include_str!("../passenger.rs")),
        ("data/event.rs", include_str!("../event.rs")),
        (
            "screens/game_screens/dossier.rs",
            include_str!("../../screens/game_screens/dossier.rs"),
        ),
        (
            "screens/game_screens/guidelines.rs",
            include_str!("../../screens/game_screens/guidelines.rs"),
        ),
        (
            "screens/game_screens/dropoff.rs",
            include_str!("../../screens/game_screens/dropoff.rs"),
        ),
        (
            "screens/game_screens/interaction.rs",
            include_str!("../../screens/game_screens/interaction.rs"),
        ),
        (
            "screens/game_screens/modals.rs",
            include_str!("../../screens/game_screens/modals.rs"),
        ),
    ];

    for (name, source) in SOURCES {
        for (number, line) in source.lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            let mut in_string = false;
            let mut escaped = false;
            for ch in line.chars() {
                if escaped {
                    escaped = false;
                    continue;
                }
                match ch {
                    '\\' if in_string => escaped = true,
                    '"' => in_string = !in_string,
                    _ if in_string && !ch.is_ascii() => panic!(
                        "{name}:{} draws {ch:?}, which the bundled font renders as a box: {}",
                        number + 1,
                        code.trim()
                    ),
                    _ => {}
                }
            }
        }
    }
}

/// The content files get no stripping pass, so what is authored there is
/// what is drawn.
///
/// `en.json` is cleaned at load, which made it easy to assume the problem
/// was solved everywhere. It was not: three of the vampire's and the
/// hunted man's escalation lines carried an em dash, so the sentence that
/// tells the player their passenger is about to lose control broke into a
/// replacement box at exactly the dramatic beat. The sulfur crystal's
/// description carried another, which nothing revealed until item
/// descriptions were displayed at all.
///
/// Stripping these the way localization is stripped would be worse than
/// fixing them: an em dash joins two clauses, and deleting it runs the
/// words together. Plain ASCII is authored here on purpose.
#[test]
fn displayed_content_carries_no_undrawable_glyphs() {
    // `emoji` on a passenger and `icon` on a skill are parsed into their
    // structs and read by nothing — the menus draw vector shapes keyed by
    // name instead. They are exempt because they never reach a font, not
    // because they are safe to display.
    const UNREAD_KEYS: [&str; 2] = ["emoji", "icon"];

    fn walk(value: &serde_json::Value, path: &str, file: &str) {
        match value {
            serde_json::Value::String(text) => {
                let bad: Vec<char> = text.chars().filter(|ch| !ch.is_ascii()).collect();
                assert!(
                    bad.is_empty(),
                    "{file}{path} carries {bad:?}, which the bundled font draws as boxes"
                );
            }
            serde_json::Value::Array(items) => {
                for (i, item) in items.iter().enumerate() {
                    walk(item, &format!("{path}[{i}]"), file);
                }
            }
            serde_json::Value::Object(map) => {
                for (key, item) in map {
                    if UNREAD_KEYS.contains(&key.as_str()) {
                        continue;
                    }
                    walk(item, &format!("{path}.{key}"), file);
                }
            }
            _ => {}
        }
    }

    for (file, json) in [
        (
            "passengerData.json",
            macroquad_toolkit::include_json_str!("../../../assets/passengerData.json"),
        ),
        (
            "itemData.json",
            macroquad_toolkit::include_json_str!("../../../assets/itemData.json"),
        ),
        (
            "locationData.json",
            macroquad_toolkit::include_json_str!("../../../assets/locationData.json"),
        ),
        (
            "skillTreeData.json",
            macroquad_toolkit::include_json_str!("../../../assets/skillTreeData.json"),
        ),
        (
            "almanacData.json",
            macroquad_toolkit::include_json_str!("../../../assets/almanacData.json"),
        ),
        (
            "guidelineData.json",
            macroquad_toolkit::include_json_str!("../../../assets/guidelineData.json"),
        ),
        (
            "eventData.json",
            macroquad_toolkit::include_json_str!("../../../assets/eventData.json"),
        ),
        (
            "shiftRulesData.json",
            macroquad_toolkit::include_json_str!("../../../assets/shiftRulesData.json"),
        ),
    ] {
        let value: serde_json::Value =
            serde_json::from_str(json).unwrap_or_else(|e| panic!("{file} is not valid: {e}"));
        walk(&value, "", file);
    }
}

/// Every non-ASCII character authored in `en.json` must be a pictograph
/// the font cannot draw. If prose ever arrives with an accent or a curly
/// quote in it, stripping would quietly mangle the word, so that should
/// be a decision rather than a silent loss.
#[test]
fn every_non_ascii_character_is_a_pictograph() {
    // Alarm clock, stopwatch, warning sign, fuel pump, check mark, and the
    // variation selector that follows several of them.
    const KNOWN: [u32; 6] = [0x23F0, 0x23F1, 0x26A0, 0x26FD, 0x2713, 0xFE0F];
    let json = macroquad_toolkit::include_json_str!("../../../assets/localization/en.json");
    for ch in json.chars().filter(|ch| !ch.is_ascii()) {
        let code = ch as u32;
        let pictographic = KNOWN.contains(&code) || (0x1F000..=0x1FAFF).contains(&code);
        assert!(
            pictographic,
            "U+{code:04X} is not a known pictograph; stripping it would mangle text"
        );
    }
}

/// Stripping must not eat the text around the glyph, and must leave
/// strings that never had one exactly as authored.
#[test]
fn stripping_preserves_the_words_and_deliberate_spacing() {
    let mut stripped = serde_json::json!("\u{26FD} Fuel: {}% - {}");
    strip_undrawable_glyphs(&mut stripped);
    assert_eq!(stripped, serde_json::json!("Fuel: {}% - {}"));

    let mut indented = serde_json::json!("  {} passengers | Difficulty {}");
    strip_undrawable_glyphs(&mut indented);
    assert_eq!(
        indented,
        serde_json::json!("  {} passengers | Difficulty {}"),
        "deliberate indentation was reformatted"
    );
}

#[test]
fn spanish_locale_keeps_accented_font_coverage() {
    let localization = try_load_localization_for("es").expect("Spanish locale parses");
    assert_eq!(localization.meta.code, "es");
    assert_eq!(localization.meta.language, "Español");
    assert!(localization.ui.main_menu.subtitle.contains("Confia"));
}

#[test]
fn embedded_game_data_passes_semantic_validation() {
    let data = GameData::load().expect("structural game data should load");
    assert!(
        data.load_errors.is_empty(),
        "unexpected data errors: {:?}",
        data.load_errors
    );
    data.validate()
        .expect("embedded decks should satisfy their references and balance");
}

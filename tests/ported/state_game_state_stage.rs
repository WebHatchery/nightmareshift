use super::*;

/// The two directions have to agree. They were separate mappings — the
/// state machine spelled stages one way for `tellIntensities` and
/// `dialogueByStage`, and an exception's `requiredStage` was read by
/// nothing at all — so nothing held them together.
#[test]
fn stage_names_round_trip() {
    for stage in [
        NeedStage::Calm,
        NeedStage::Warning,
        NeedStage::Critical,
        NeedStage::Meltdown,
    ] {
        assert_eq!(NeedStage::parse(stage.key()), Some(stage));
    }
}

/// Authoring is case-insensitive, and a name that is not a stage is not
/// quietly treated as one.
#[test]
fn parsing_is_forgiving_about_case_and_nothing_else() {
    assert_eq!(NeedStage::parse("CRITICAL"), Some(NeedStage::Critical));
    assert_eq!(NeedStage::parse("Warning"), Some(NeedStage::Warning));
    assert_eq!(NeedStage::parse("panicking"), None);
    assert_eq!(NeedStage::parse(""), None);
}

/// The order the exception gate compares on. Reading `requiredStage`
/// means nothing if a later stage does not count as having reached an
/// earlier one.
#[test]
fn later_stages_include_earlier_ones() {
    assert!(NeedStage::Meltdown > NeedStage::Critical);
    assert!(NeedStage::Critical > NeedStage::Warning);
    assert!(NeedStage::Warning > NeedStage::Calm);
}

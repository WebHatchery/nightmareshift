use nightmare_shift::data::loader::load_constants;

/// The thresholds must be ordered and reachable, or one of the two
/// encounters can never fire: a `HIGH_RISK` above `MAX_RISK_LEVEL` would
/// be unreachable, and one below `SUPERNATURAL_THRESHOLD` would mean the
/// milder encounter is never even considered.
#[test]
fn risk_thresholds_are_ordered_and_reachable() {
    let risk = load_constants().risk;
    assert!(
        risk.supernatural_threshold < risk.high_risk,
        "supernatural threshold {} is not below high risk {}",
        risk.supernatural_threshold,
        risk.high_risk
    );
    assert!(
        risk.high_risk <= risk.max_risk_level,
        "high risk {} is above the {} ceiling and can never be reached",
        risk.high_risk,
        risk.max_risk_level
    );
    assert!(risk.extreme_risk <= risk.max_risk_level);
}

/// Both encounter probabilities must be live. A zero would leave the
/// threshold authored but the encounter silent.
#[test]
fn both_encounter_probabilities_can_fire() {
    let probabilities = load_constants().probabilities;
    assert!(probabilities.supernatural_encounter > 0.0);
    assert!(probabilities.high_risk_encounter > 0.0);
    assert!(probabilities.supernatural_encounter <= 1.0);
    assert!(probabilities.high_risk_encounter <= 1.0);
}

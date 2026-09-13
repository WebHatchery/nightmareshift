use super::*;

/// Both risk strings are drawn under an event choice now, so both have to
/// be sayable by the bundled font. This project has shipped replacement
/// boxes in displayed text more than once, and these two live in Rust
/// rather than in the JSON the content glyph test walks.
#[test]
fn every_risk_tag_reads_as_plain_text() {
    for tag in [
        RiskTag::HighTraffic,
        RiskTag::PolicePatrol,
        RiskTag::SpiritualDisturbance,
        RiskTag::SlipperyRoads,
        RiskTag::RoadConstruction,
        RiskTag::DenseFog,
        RiskTag::GangActivity,
        RiskTag::Potholes,
        RiskTag::FlashFloods,
        RiskTag::StrangeNoises,
    ] {
        for text in [tag.name(), tag.description()] {
            assert!(!text.trim().is_empty(), "{tag:?} has an empty label");
            assert!(
                text.is_ascii(),
                "{tag:?} carries a glyph the font cannot draw: {text:?}"
            );
        }
    }
}

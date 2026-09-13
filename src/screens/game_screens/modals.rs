//! Overlay panels reachable during a shift: the inventory and the rules panel.

use macroquad::prelude::*;

use crate::data::{self, GameData, Rarity};
use crate::state::GameState;
use crate::ui::draw_ui_text;
use crate::ui::{
    colors, draw_glass_button, draw_glass_panel, draw_item_art, draw_modal_scrim, draw_small_caps,
    draw_wrapped_text, fonts, spacing, UiAction, UiRect,
};

/// Draw the inventory modal
pub fn draw_inventory_modal(game_state: &GameState, game_data: Option<&GameData>) -> UiAction {
    if let Some(data) = game_data {
        draw_modal_scrim();

        // Panel
        let panel_w = (screen_width() - 180.0).min(920.0);
        let panel_h = (screen_height() - 180.0).min(640.0);
        let panel_x = (screen_width() - panel_w) / 2.0;
        let panel_y = (screen_height() - panel_h) / 2.0;
        let panel_rect = UiRect::new(panel_x, panel_y, panel_w, panel_h);

        draw_glass_panel(panel_rect, colors::ACCENT_SKY);

        let inner = panel_rect.inset(spacing::PADDING_LG);
        let mut y = inner.y;

        // Title
        draw_ui_text(
            &data.localization.ui.game.inventory.title,
            inner.x,
            y,
            fonts::SIZE_XL,
            colors::ACCENT_SKY,
        );
        y += 40.0;

        // Help text
        draw_ui_text(
            &data.localization.ui.game.inventory.hint,
            inner.x,
            y,
            fonts::SIZE_SM,
            colors::TEXT_MUTED,
        );
        y += 30.0;

        // Item count
        // "Items: {}"
        let count_text = data
            .localization
            .ui
            .game
            .inventory
            .count
            .replace("{}", &game_state.inventory.len().to_string());
        draw_ui_text(
            &count_text,
            inner.x,
            y,
            fonts::SIZE_MD,
            colors::TEXT_PRIMARY,
        );
        y += 35.0;

        // Draw items
        if game_state.inventory.is_empty() {
            draw_ui_text(
                &data.localization.ui.game.inventory.empty,
                inner.x,
                y,
                fonts::SIZE_MD,
                colors::TEXT_MUTED,
            );
        } else {
            let row_h = 92.0;
            for (i, item) in game_state.inventory.iter().enumerate() {
                // Item background
                let item_bg = if i % 2 == 0 {
                    Color::new(0.08, 0.10, 0.10, 0.76)
                } else {
                    Color::new(0.05, 0.065, 0.065, 0.76)
                };
                draw_rectangle(inner.x, y - 5.0, inner.w, row_h, item_bg);
                draw_rectangle_lines(inner.x, y - 5.0, inner.w, row_h, 1.0, colors::BORDER_DIM);
                draw_item_art(UiRect::new(inner.x + 8.0, y + 8.0, 54.0, 54.0), &item.name);

                // Rarity color
                let rarity_color = match item.rarity {
                    Rarity::Common => colors::TEXT_SECONDARY,
                    Rarity::Uncommon => colors::ACCENT_PRIMARY,
                    Rarity::Rare => colors::ACCENT_SKY,
                    Rarity::Legendary => colors::ACCENT_GOLD,
                };

                // Item name
                draw_ui_text(
                    &item.name,
                    inner.x + 74.0,
                    y + 15.0,
                    fonts::SIZE_MD,
                    rarity_color,
                );

                // Rarity badge
                let rarity_text = format!("{:?}", item.rarity);
                draw_ui_text(
                    &rarity_text,
                    inner.x + 74.0,
                    y + 35.0,
                    fonts::SIZE_XS,
                    colors::TEXT_MUTED,
                );

                // Charges left, for the items that count them.
                if let Some((left, most)) = item.uses_left() {
                    let colour = if left <= 1 {
                        colors::ACCENT_WARNING
                    } else {
                        colors::TEXT_MUTED
                    };
                    draw_ui_text(
                        &format!("{left} of {most} uses"),
                        inner.x + inner.w * 0.62,
                        y + 35.0,
                        fonts::SIZE_XS,
                        colour,
                    );
                }

                // Source
                // "from {}"
                let source_text = data
                    .localization
                    .ui
                    .game
                    .inventory
                    .source
                    .replace("{}", &item.source);
                draw_ui_text(
                    &source_text,
                    inner.x + inner.w * 0.34,
                    y + 35.0,
                    fonts::SIZE_XS,
                    colors::TEXT_MUTED,
                );

                // What a curse is doing to you, and the way out of it.
                // The inventory named the item and who left it and nothing
                // else, so a driver carrying the Old Locket had no way to
                // learn it was drawing danger, let alone that Sister Agnes
                // would take it off their hands.
                if let Some(curse) = &item.cursed_properties {
                    let penalty = match curse.penalty_type {
                        data::CursePenalty::FuelDrain => "burns fuel",
                        data::CursePenalty::TimeAcceleration => "eats the clock",
                        data::CursePenalty::AttractingDanger => "draws danger",
                        data::CursePenalty::ForcedChoices => "narrows the road",
                    };
                    let way_out = curse
                        .removal_condition
                        .as_deref()
                        .filter(|_| curse.can_be_removed)
                        .unwrap_or("Cannot be given away");
                    draw_ui_text(
                        &format!("Cursed - {} | {}", penalty, way_out),
                        inner.x + 74.0,
                        y + 55.0,
                        fonts::SIZE_XS,
                        if curse.can_be_removed {
                            colors::ACCENT_WARNING
                        } else {
                            colors::FUEL_CRITICAL
                        },
                    );
                }

                // What the thing actually is. Every item in the catalogue is
                // authored with a description and until now nothing displayed
                // one, so twenty-five written lines sat in the data doing
                // nothing. It is also the only warning a cursed gift gets
                // before it is accepted: the Old Locket "whispers forgotten
                // names" long before the inventory admits it draws danger.
                if !item.description.is_empty() {
                    draw_wrapped_text(
                        &item.description,
                        inner.x + 74.0,
                        y + 75.0,
                        inner.w - 84.0,
                        fonts::SIZE_XS,
                        14.0,
                        colors::TEXT_SECONDARY,
                        1,
                    );
                }

                // Can use indicator
                if item.can_use {
                    let use_text = &data.localization.ui.game.inventory.click_to_use;
                    if draw_glass_button(
                        UiRect::new(inner.x + inner.w - 156.0, y + 9.0, 136.0, 40.0),
                        use_text,
                        colors::ACCENT_SKY,
                        true,
                    ) {
                        return UiAction::UseItem(i);
                    }
                }

                y += row_h + 10.0;

                // Check if we're running out of space
                if y > inner.y + panel_h - 80.0 {
                    draw_ui_text(
                        &data.localization.ui.game.inventory.more,
                        inner.x,
                        y,
                        fonts::SIZE_SM,
                        colors::TEXT_MUTED,
                    );
                    break;
                }
            }
        }
    }

    UiAction::None
}

/// Draw the rules panel
pub fn draw_rules_panel(game_state: &GameState, game_data: Option<&GameData>) -> UiAction {
    // Semi-transparent overlay
    draw_modal_scrim();

    // Panel
    let panel_w = (screen_width() - 180.0).min(900.0);
    let panel_h = (screen_height() - 180.0).min(620.0);
    let panel_x = (screen_width() - panel_w) / 2.0;
    let panel_y = (screen_height() - panel_h) / 2.0;
    let panel_rect = UiRect::new(panel_x, panel_y, panel_w, panel_h);

    draw_glass_panel(panel_rect, colors::ACCENT_PRIMARY);

    let inner = panel_rect.inset(spacing::PADDING_LG);
    let mut y = inner.y + 24.0;

    if let Some(data) = game_data {
        // Title
        draw_ui_text(
            &data.localization.ui.game.rules.title,
            inner.x,
            y,
            fonts::SIZE_XL,
            colors::ACCENT_PRIMARY,
        );
        y += 42.0;

        // Help text
        draw_ui_text(
            &data.localization.ui.game.rules.hint,
            inner.x,
            y,
            fonts::SIZE_SM,
            colors::TEXT_MUTED,
        );
        y += 42.0;
    } else {
        // Fallback
        // Title
        draw_ui_text(
            "CURRENT RULES",
            inner.x,
            y,
            fonts::SIZE_XL,
            colors::ACCENT_PRIMARY,
        );
        y += 42.0;

        // Help text
        draw_ui_text(
            "Press R to close",
            inner.x,
            y,
            fonts::SIZE_SM,
            colors::TEXT_MUTED,
        );
        y += 42.0;
    }

    if game_state.current_passenger.is_some() {
        draw_small_caps(
            "CAB CONTROLS",
            inner.x,
            y,
            fonts::SIZE_SM,
            colors::TEXT_MUTED,
        );
        y += 24.0;

        let controls = cab_controls_for_rules_panel(game_state.game_phase);
        let cols = 4;
        let gap = 10.0;
        let btn_h = 38.0;
        let btn_w = (inner.w - gap * (cols as f32 - 1.0)) / cols as f32;
        for (idx, (key, label, action_key, enabled)) in controls.iter().enumerate() {
            let col = idx % cols;
            let row = idx / cols;
            let rect = UiRect::new(
                inner.x + col as f32 * (btn_w + gap),
                y + row as f32 * (btn_h + gap),
                btn_w,
                btn_h,
            );
            let label = if *action_key == "drive_dark" && !game_state.headlights_on {
                "Headlights On"
            } else {
                label
            };
            let text = format!("[{}] {}", key, label);
            if draw_glass_button(rect, &text, colors::BORDER, *enabled) {
                return UiAction::PerformRuleAction((*action_key).to_string());
            }
        }
        y += 2.0 * (btn_h + gap) + 20.0;
    }

    let rule_card_h = 100.0;
    // Draw rules
    for (rule_idx, rule) in game_state.current_rules.iter().enumerate() {
        let card = UiRect::new(inner.x, y, inner.w, rule_card_h);
        draw_glass_panel(card, colors::BORDER_DIM);

        // Rule title with difficulty color
        let difficulty_color = match rule.difficulty {
            data::Difficulty::Easy => colors::FUEL_GOOD,
            data::Difficulty::Medium => colors::ACCENT_WARNING,
            data::Difficulty::Hard => colors::FUEL_LOW,
            data::Difficulty::Expert => colors::FUEL_CRITICAL,
            data::Difficulty::Nightmare => colors::ACCENT_DANGER,
        };

        // Plain numbering: rules are a list, not a set of options, and
        // bracketed digits read as key bindings everywhere else on screen.
        let key = format!("{}.", rule_idx + 1);
        draw_small_caps(
            &key,
            card.x + 14.0,
            card.y + 38.0,
            fonts::SIZE_SM,
            colors::TEXT_MUTED,
        );
        draw_ui_text(
            &rule.title,
            card.x + 58.0,
            card.y + 34.0,
            fonts::SIZE_MD,
            difficulty_color,
        );

        // Rule description (wrapped if too long)
        let desc = if rule.description.len() > 70 {
            format!("{}...", &rule.description[..70])
        } else {
            rule.description.clone()
        };
        draw_ui_text(
            &desc,
            card.x + 58.0,
            card.y + 60.0,
            fonts::SIZE_SM,
            colors::TEXT_SECONDARY,
        );

        // Why obeying keeps you alive. Thirteen rules author a
        // `defaultOutcome` explaining themselves and it was shown nowhere, so
        // the rules read as arbitrary instructions rather than as things the
        // night has reasons for.
        if let Some(outcome) = &rule.default_outcome {
            let reason = if outcome.len() > 82 {
                format!("{}...", &outcome[..82])
            } else {
                outcome.clone()
            };
            draw_small_caps(
                &reason,
                card.x + 58.0,
                card.y + 80.0,
                fonts::SIZE_XS,
                colors::ACCENT_SKY,
            );
        }
        y = card.bottom() + 14.0;

        // Check if we're running out of space
        if y > inner.y + panel_h - 80.0 {
            draw_ui_text("...", inner.x, y, fonts::SIZE_SM, colors::TEXT_MUTED);
            break;
        }
    }

    UiAction::None
}

fn cab_controls_for_rules_panel(
    game_phase: crate::state::GamePhase,
) -> [(&'static str, &'static str, &'static str, bool); 8] {
    let ride_active = matches!(
        game_phase,
        crate::state::GamePhase::RideRequest
            | crate::state::GamePhase::Driving
            | crate::state::GamePhase::Interaction
            | crate::state::GamePhase::GuidelineDecision
            | crate::state::GamePhase::DropOff
    );
    [
        ("E", "Eye Contact", "eye_contact", ride_active),
        ("M", "Music", "play_music", ride_active),
        (
            "T",
            "Accept Tip",
            "accept_tip",
            game_phase == crate::state::GamePhase::DropOff,
        ),
        ("W", "Open Window", "open_window", ride_active),
        ("Y", "Wipers", "use_wipers", ride_active),
        ("H", "Headlights Off", "drive_dark", ride_active),
        ("A", "AC", "use_ac", ride_active),
        (
            "S",
            "Stop Cab",
            "stop_vehicle",
            matches!(
                game_phase,
                crate::state::GamePhase::Driving | crate::state::GamePhase::Interaction
            ),
        ),
    ]
}

//! The guideline decision screen: timer, detected tells, and follow/break.

use macroquad::prelude::*;

use crate::data::{self, GameData};
use crate::state::GameState;
use crate::ui::draw_ui_text;
use crate::ui::{
    colors, draw_cockpit_background, draw_glass_button, draw_glass_panel, draw_small_caps,
    draw_wrapped_text, fonts, layout, spacing, UiAction, UiRect,
};

use super::scene::draw_bottom_taxi_scene;

fn label_with_binding(label: &str, binding: &str) -> String {
    let base = label
        .strip_suffix(')')
        .and_then(|without_close| without_close.rsplit_once(" ("))
        .map_or(label, |(base, _)| base);
    format!("{base} ({binding})")
}

/// Draw the guideline decision screen
pub fn draw_guideline_decision(
    game_state: &GameState,
    game_data: Option<&GameData>,
    player_stats: &crate::state::PlayerStats,
) -> UiAction {
    draw_cockpit_background();

    if screen_width() < 980.0 || screen_height() < 560.0 {
        return draw_narrow_guideline_decision(game_state, game_data, player_stats);
    }

    let scene_h = (screen_height() * 0.25).clamp(200.0, 280.0);
    let scene_rect = UiRect::new(
        70.0,
        screen_height() - scene_h - 34.0,
        screen_width() - 140.0,
        scene_h,
    );
    draw_bottom_taxi_scene(scene_rect);

    if let Some(ref guideline) = game_state.active_guideline {
        if let Some(data) = game_data {
            let center_x = screen_width() / 2.0;
            let rect_w = (screen_width() - 140.0).min(860.0);
            let rect_h = (scene_rect.y - layout::STATUS_BAR_HEIGHT - 76.0).clamp(440.0, 520.0);
            let rect = UiRect::centered_x(
                screen_width(),
                layout::STATUS_BAR_HEIGHT + 34.0,
                rect_w,
                rect_h,
            );
            draw_glass_panel(rect, colors::ACCENT_WARNING);

            let inner = rect.inset(spacing::PADDING_LG);
            let mut y = inner.y;

            // Title
            draw_ui_text(
                &data.localization.ui.game.guidelines.title,
                inner.x,
                y + 28.0,
                fonts::SIZE_XL,
                colors::ACCENT_WARNING,
            );
            y += 50.0;

            // Timer with color coding
            let time_left = game_state.guideline_time_remaining;
            let timer_color = if time_left <= 10.0 {
                colors::FUEL_CRITICAL
            } else if time_left <= 20.0 {
                colors::ACCENT_WARNING
            } else {
                colors::FUEL_GOOD
            };

            // "⏱️ Time: {:.1}s"
            let timer_text = data
                .localization
                .ui
                .game
                .guidelines
                .timer
                .replace("{:.1}", &format!("{:.1}", time_left));

            draw_ui_text(&timer_text, inner.x, y + 20.0, fonts::SIZE_LG, timer_color);
            y += 50.0;

            // Guideline info
            draw_ui_text(
                &data.localization.ui.game.guidelines.label,
                inner.x,
                y + 18.0,
                fonts::SIZE_MD,
                colors::TEXT_MUTED,
            );
            y += 25.0;
            draw_ui_text(
                &guideline.title,
                inner.x,
                y + 18.0,
                fonts::SIZE_LG,
                colors::ACCENT_SKY,
            );
            y += 35.0;

            // How hard this read is. Five difficulty tiers are authored
            // across the guidelines and decided nothing until shown here.
            let (difficulty_label, difficulty_color) = match guideline.difficulty {
                data::Difficulty::Easy => ("An easy read", colors::FUEL_GOOD),
                data::Difficulty::Medium => ("A fair read", colors::TEXT_MUTED),
                data::Difficulty::Hard => ("A hard read", colors::ACCENT_WARNING),
                data::Difficulty::Expert => ("An expert read", colors::ACCENT_WARNING),
                data::Difficulty::Nightmare => ("A nightmare read", colors::FUEL_CRITICAL),
            };
            draw_small_caps(
                difficulty_label,
                inner.x,
                y + 12.0,
                fonts::SIZE_XS,
                difficulty_color,
            );
            y += 22.0;

            // Description (truncated)
            let desc_preview = if guideline.description.len() > 60 {
                format!("{}...", &guideline.description[..60])
            } else {
                guideline.description.clone()
            };
            draw_ui_text(
                &desc_preview,
                inner.x,
                y + 16.0,
                fonts::SIZE_SM,
                colors::TEXT_PRIMARY,
            );
            y += 50.0;

            // The last thing the passenger said.
            //
            // Sixteen profiles author eighty escalation lines across warning,
            // critical and meltdown, and they are written during route
            // resolution -- which is also where a guideline decision gets
            // triggered. The line lands in state and this screen appears over
            // the top of it, so the most immediate tell there is went unread
            // on the one screen that asks the player to read the passenger.
            // It costs no almanac level, unlike the verdict below it.
            if let Some(spoken) = game_state.current_passenger_dialogue.as_ref() {
                let speaker = game_state
                    .current_passenger
                    .as_ref()
                    .map(|passenger| passenger.name.as_str())
                    .unwrap_or("The passenger");
                y = draw_wrapped_text(
                    &format!("{speaker}: \"{spoken}\""),
                    inner.x,
                    y + 16.0,
                    inner.w,
                    fonts::SIZE_SM,
                    17.0,
                    colors::CAB_YELLOW,
                    2,
                ) + 12.0;
            }

            // What a studied passenger's file says about this guideline.
            // The decision is judged on whether an exception is live, and
            // until now nothing told the player that — tells hint at it, but
            // the conditions deciding it were invisible. Reading the same
            // check the engine judges by is what almanac Lv.2 buys here.
            if let Some(passenger) = game_state.current_passenger.as_ref() {
                let studied = player_stats.get_almanac_entry(passenger.id).knowledge_level >= 2;
                if studied {
                    let active = crate::engine::GuidelineEngine::find_active_exception(
                        guideline,
                        passenger,
                        &game_state.current_weather,
                        &game_state.live_exceptions,
                    );
                    let (verdict, colour) = match &active {
                        Some(exception) if exception.breaking_safer => (
                            format!("Almanac: an exception applies - {}", exception.description),
                            colors::FUEL_GOOD,
                        ),
                        Some(exception) => (
                            format!("Almanac: {} - the rule still holds", exception.description),
                            colors::ACCENT_WARNING,
                        ),
                        None => (
                            "Almanac: nothing excuses breaking this tonight".to_string(),
                            colors::ACCENT_WARNING,
                        ),
                    };
                    draw_wrapped_text(
                        &verdict,
                        inner.x,
                        y + 16.0,
                        inner.w,
                        fonts::SIZE_SM,
                        17.0,
                        colour,
                        2,
                    );
                    y += 40.0;
                }
            }

            // Detected tells
            draw_ui_text(
                &data.localization.ui.game.guidelines.tells_label,
                inner.x,
                y + 18.0,
                fonts::SIZE_MD,
                colors::TEXT_MUTED,
            );
            y += 30.0;

            let relevant_tells: Vec<_> = game_state
                .detected_tells
                .iter()
                .filter(|t| t.related_guideline == Some(guideline.id))
                .collect();

            if relevant_tells.is_empty() {
                draw_ui_text(
                    &data.localization.ui.game.guidelines.no_tells,
                    inner.x + 20.0,
                    y + 16.0,
                    fonts::SIZE_SM,
                    colors::TEXT_MUTED,
                );
                y += 25.0;
            } else {
                for tell in relevant_tells.iter().take(3) {
                    let (intensity_text, intensity_color) = match tell.tell.intensity {
                        data::TellIntensity::Subtle => (
                            &data.localization.ui.game.guidelines.intensity.subtle,
                            colors::TEXT_MUTED,
                        ),
                        data::TellIntensity::Moderate => (
                            &data.localization.ui.game.guidelines.intensity.moderate,
                            colors::ACCENT_WARNING,
                        ),
                        data::TellIntensity::Obvious => (
                            &data.localization.ui.game.guidelines.intensity.obvious,
                            colors::FUEL_CRITICAL,
                        ),
                    };

                    let noticed_text = if tell.player_noticed {
                        "noticed"
                    } else {
                        "uncertain"
                    };
                    // Which sense caught it. The four authored tell types
                    // read identically on this panel until labelled, and the
                    // channel matters: an environmental tell is the world
                    // speaking, not the passenger performing.
                    let channel = match tell.tell.tell_type {
                        data::TellType::Verbal => "heard",
                        data::TellType::Behavioral => "manner",
                        data::TellType::Visual => "seen",
                        data::TellType::Environmental => "the cab",
                    };
                    let age = (get_time() - tell.detection_time).max(0.0);
                    let tell_text = format!(
                        "- [{} / {}] {} ({}, {:.0}s)",
                        intensity_text, channel, tell.tell.description, noticed_text, age
                    );
                    draw_ui_text(
                        &tell_text,
                        inner.x + 20.0,
                        y + 16.0,
                        fonts::SIZE_SM,
                        intensity_color,
                    );
                    y += 25.0;
                }
            }

            y += 30.0;

            // Decision buttons
            let btn_w = 200.0;
            let btn_h = 50.0;
            let btn_spacing = 20.0;

            if let Some(last_decision) = game_state.decision_history.last() {
                let last_action = match last_decision.action {
                    crate::state::GuidelineAction::Follow => "followed",
                    crate::state::GuidelineAction::Break => "broke",
                };
                let history_text = format!(
                    "Last decision: #{} passenger {} {} with {} tells ({:.0}s ago)",
                    last_decision.guideline_id,
                    last_decision.passenger_id,
                    last_action,
                    last_decision.tells_present.len(),
                    (get_time() - last_decision.timestamp).max(0.0)
                );
                draw_ui_text(
                    &history_text,
                    inner.x,
                    y - 8.0,
                    fonts::SIZE_XS,
                    colors::TEXT_MUTED,
                );
            }

            // Follow guideline button (left)
            let follow_text = label_with_binding(
                &data.localization.ui.game.guidelines.follow,
                &player_stats.accessibility.key_bindings.follow,
            );
            let break_text = label_with_binding(
                &data.localization.ui.game.guidelines.break_guideline,
                &player_stats.accessibility.key_bindings.break_guideline,
            );
            if draw_glass_button(
                UiRect::new(center_x - btn_w - btn_spacing / 2.0, y, btn_w, btn_h),
                &follow_text,
                colors::FUEL_GOOD,
                true,
            ) {
                return UiAction::FollowGuideline;
            }

            if draw_glass_button(
                UiRect::new(center_x + btn_spacing / 2.0, y, btn_w, btn_h),
                &break_text,
                colors::ACCENT_DANGER,
                true,
            ) {
                return UiAction::BreakGuideline;
            }

            // Auto-decide if time runs out
            if time_left <= 0.0 {
                return UiAction::FollowGuideline;
            }
        }
    }

    UiAction::None
}

fn draw_narrow_guideline_decision(
    game_state: &GameState,
    game_data: Option<&GameData>,
    player_stats: &crate::state::PlayerStats,
) -> UiAction {
    let (Some(guideline), Some(data)) = (game_state.active_guideline.as_ref(), game_data) else {
        return UiAction::None;
    };
    let panel = UiRect::new(
        12.0,
        layout::STATUS_BAR_HEIGHT + 8.0,
        screen_width() - 24.0,
        110.0,
    );
    draw_glass_panel(panel, colors::ACCENT_WARNING);
    draw_small_caps(
        &data.localization.ui.game.guidelines.title,
        panel.x + 12.0,
        panel.y + 18.0,
        fonts::SIZE_XS,
        colors::ACCENT_WARNING,
    );
    let timer_color = if game_state.guideline_time_remaining <= 10.0 {
        colors::FUEL_CRITICAL
    } else {
        colors::FUEL_GOOD
    };
    draw_small_caps(
        &format!("TIME {:.1}s", game_state.guideline_time_remaining.max(0.0)),
        panel.x + panel.w - 82.0,
        panel.y + 18.0,
        fonts::SIZE_XS,
        timer_color,
    );
    draw_ui_text(
        &guideline.title,
        panel.x + 12.0,
        panel.y + 42.0,
        fonts::SIZE_LG,
        colors::ACCENT_SKY,
    );
    let description = macroquad_toolkit::ui::truncate_text_to_width(
        &guideline.description,
        panel.w - 24.0,
        fonts::SIZE_XS,
    );
    draw_small_caps(
        &description,
        panel.x + 12.0,
        panel.y + 61.0,
        fonts::SIZE_XS,
        colors::TEXT_SECONDARY,
    );
    if let Some(tell) = game_state
        .detected_tells
        .iter()
        .find(|tell| tell.related_guideline == Some(guideline.id))
    {
        draw_small_caps(
            &format!("TELL: {}", tell.tell.description),
            panel.x + 12.0,
            panel.y + 78.0,
            fonts::SIZE_XS,
            colors::ACCENT_GOLD,
        );
    }

    let gap = 8.0;
    let button_w = (screen_width() - 24.0 - gap) / 2.0;
    let y = screen_height() - 34.0;
    let follow = label_with_binding(
        &data.localization.ui.game.guidelines.follow,
        &player_stats.accessibility.key_bindings.follow,
    );
    let break_rule = label_with_binding(
        &data.localization.ui.game.guidelines.break_guideline,
        &player_stats.accessibility.key_bindings.break_guideline,
    );
    if draw_glass_button(
        UiRect::new(12.0, y, button_w, 26.0),
        &follow,
        colors::FUEL_GOOD,
        true,
    ) {
        return UiAction::FollowGuideline;
    }
    if draw_glass_button(
        UiRect::new(12.0 + button_w + gap, y, button_w, 26.0),
        &break_rule,
        colors::ACCENT_DANGER,
        true,
    ) {
        return UiAction::BreakGuideline;
    }
    UiAction::None
}

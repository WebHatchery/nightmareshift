//! The pre-shift briefing: rules, night conditions, and shift targets.

use macroquad::prelude::*;

use crate::data::GameData;
use crate::state::GameState;
use crate::ui::draw_ui_text;
use crate::ui::{
    colors, draw_glass_button, draw_glass_panel, draw_noir_city_background, draw_small_caps,
    draw_wrapped_text, fonts, UiAction, UiRect,
};
use macroquad_toolkit::ui::format_clock;

fn draw_briefing_taxi_scene(rect: UiRect) {
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        Color::new(0.010, 0.012, 0.012, 0.92),
    );
    draw_rectangle(rect.x, rect.y, rect.w, 1.0, Color::new(1.0, 1.0, 1.0, 0.10));
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, colors::BORDER_DIM);

    let road_y = rect.y + rect.h * 0.68;
    draw_rectangle(
        rect.x,
        road_y,
        rect.w,
        rect.h * 0.32,
        Color::new(0.020, 0.024, 0.022, 1.0),
    );
    for i in 0..8 {
        let x = rect.x + rect.w * (i as f32 / 8.0);
        draw_rectangle(
            x + rect.w * 0.03,
            road_y + rect.h * 0.14,
            rect.w * 0.05,
            3.0,
            Color::new(0.95, 0.58, 0.08, 0.32),
        );
    }

    for i in 0..6 {
        let x = rect.x + rect.w * (0.12 + i as f32 * 0.15);
        let lamp_top = rect.y + rect.h * (0.16 + (i % 2) as f32 * 0.05);
        draw_line(
            x,
            lamp_top + 26.0,
            x,
            road_y + 26.0,
            2.0,
            Color::new(0.28, 0.24, 0.19, 0.70),
        );
        draw_circle(x, lamp_top, 5.0, colors::CAB_YELLOW);
        draw_circle(x, lamp_top, 24.0, Color::new(0.95, 0.58, 0.08, 0.08));
    }

    let tx = rect.x + rect.w * 0.50;
    let ty = rect.y + rect.h * 0.47;
    let scale = (rect.w / 820.0).clamp(0.72, 1.18);
    let body_w = 330.0 * scale;
    let body_h = 72.0 * scale;
    let body_x = tx - body_w / 2.0;
    let body_y = ty;
    draw_rectangle(
        body_x,
        body_y,
        body_w,
        body_h,
        Color::new(0.74, 0.45, 0.06, 0.95),
    );
    draw_rectangle(
        body_x + body_w * 0.14,
        body_y - body_h * 0.44,
        body_w * 0.46,
        body_h * 0.44,
        Color::new(0.55, 0.34, 0.06, 0.92),
    );
    draw_rectangle(
        body_x + body_w * 0.20,
        body_y - body_h * 0.31,
        body_w * 0.15,
        body_h * 0.25,
        Color::new(0.06, 0.09, 0.10, 0.86),
    );
    draw_rectangle(
        body_x + body_w * 0.39,
        body_y - body_h * 0.31,
        body_w * 0.17,
        body_h * 0.25,
        Color::new(0.06, 0.09, 0.10, 0.86),
    );
    draw_rectangle(
        body_x + body_w * 0.26,
        body_y - body_h * 0.66,
        body_w * 0.22,
        body_h * 0.18,
        colors::CAB_YELLOW,
    );
    draw_ui_text(
        "TAXI",
        body_x + body_w * 0.31,
        body_y - body_h * 0.51,
        fonts::SIZE_SM * scale,
        colors::BLACK,
    );
    draw_rectangle(
        body_x + body_w * 0.03,
        body_y + body_h * 0.46,
        body_w * 0.14,
        body_h * 0.14,
        Color::new(0.95, 0.10, 0.04, 0.58),
    );
    draw_rectangle(
        body_x + body_w * 0.84,
        body_y + body_h * 0.45,
        body_w * 0.11,
        body_h * 0.12,
        Color::new(1.0, 0.86, 0.44, 0.68),
    );
    draw_circle(
        body_x + body_w * 0.17,
        body_y + body_h * 0.78,
        body_h * 0.28,
        colors::BLACK,
    );
    draw_circle(
        body_x + body_w * 0.82,
        body_y + body_h * 0.78,
        body_h * 0.28,
        colors::BLACK,
    );
    draw_circle(
        body_x + body_w * 0.17,
        body_y + body_h * 0.78,
        body_h * 0.13,
        Color::new(0.12, 0.13, 0.12, 1.0),
    );
    draw_circle(
        body_x + body_w * 0.82,
        body_y + body_h * 0.78,
        body_h * 0.13,
        Color::new(0.12, 0.13, 0.12, 1.0),
    );

    draw_rectangle(
        body_x - body_w * 0.10,
        body_y + body_h * 0.88,
        body_w * 1.20,
        6.0,
        Color::new(0.95, 0.58, 0.08, 0.18),
    );
}

/// Draw the briefing screen
pub fn draw_briefing(
    game_state: &GameState,
    game_data: Option<&GameData>,
    player_stats: &crate::state::PlayerStats,
    run_seed: Option<u64>,
) -> UiAction {
    draw_noir_city_background();

    if let Some(data) = game_data {
        if screen_width() < 980.0 || screen_height() < 560.0 {
            return draw_narrow_briefing(game_state, data, run_seed);
        }
        let screen_w = screen_width();
        let screen_h = screen_height();
        let margin = (screen_w * 0.045).clamp(30.0, 70.0);
        let header_h = 100.0;
        let footer_h = 76.0;
        let content_top = header_h + 30.0;
        let content_bottom = screen_h - footer_h;
        let gap = 18.0;

        let header = UiRect::new(margin, 28.0, screen_w - margin * 2.0, header_h);
        draw_glass_panel(header, colors::BORDER_DIM);
        let header_inner = header.inset(18.0);

        draw_ui_text(
            &data.localization.ui.briefing.title,
            header_inner.x,
            header_inner.y + 40.0,
            44.0,
            colors::CAB_YELLOW,
        );
        // Campaign progress: which night of the run, plus a framing line that
        // advances the driver's week-long story.
        let night_label = data
            .localization
            .ui
            .briefing
            .night_label
            .replacen("{}", &game_state.night.to_string(), 1)
            .replacen(
                "{}",
                &data.constants.game_constants.nights_per_run.to_string(),
                1,
            );
        draw_small_caps(
            &night_label,
            header_inner.x,
            header_inner.y + 70.0,
            fonts::SIZE_MD,
            colors::TEXT_MUTED,
        );
        if let Some(premise) = data
            .localization
            .ui
            .briefing
            .premise
            .get((game_state.night as usize).saturating_sub(1))
            .or_else(|| data.localization.ui.briefing.premise.last())
        {
            let premise_width = header_inner.w * 0.56;
            let premise = macroquad_toolkit::ui::truncate_text_to_width(
                premise,
                premise_width,
                fonts::SIZE_SM,
            );
            draw_small_caps(
                &premise,
                header_inner.x + header_inner.w * 0.42,
                header_inner.y + 58.0,
                fonts::SIZE_SM,
                colors::TEXT_SECONDARY,
            );
        }

        let available_h = content_bottom - content_top;
        let top_h = if screen_w < 1100.0 {
            (available_h - gap - 82.0).clamp(390.0, 520.0)
        } else {
            (available_h * 0.58).clamp(360.0, 520.0)
        };
        let scene_h = content_bottom - content_top - top_h - gap;
        let left_w = ((screen_w - margin * 2.0 - gap) * 0.62).clamp(560.0, 1050.0);
        let right_w = screen_w - margin * 2.0 - gap - left_w;
        let rules_panel = UiRect::new(margin, content_top, left_w, top_h);
        let conditions_panel = UiRect::new(margin + left_w + gap, content_top, right_w, top_h);
        let scene_panel = UiRect::new(
            margin,
            content_top + top_h + gap,
            screen_w - margin * 2.0,
            scene_h,
        );

        draw_glass_panel(rules_panel, colors::BORDER_DIM);
        draw_glass_panel(conditions_panel, colors::BORDER_DIM);

        let rules_inner = rules_panel.inset(18.0);
        draw_small_caps(
            &data.localization.ui.briefing.rules_title,
            rules_inner.x,
            rules_inner.y + 16.0,
            fonts::SIZE_LG,
            colors::CAB_YELLOW,
        );

        let mut y = rules_inner.y + 54.0;
        for (i, rule) in game_state.current_rules.iter().enumerate() {
            let card_h = 76.0;
            let card = UiRect::new(rules_inner.x, y, rules_inner.w, card_h);
            draw_rectangle(
                card.x,
                card.y,
                card.w,
                card.h,
                Color::new(0.025, 0.030, 0.032, 0.92),
            );
            draw_rectangle(card.x, card.y, 4.0, card.h, colors::CAB_YELLOW);
            draw_rectangle_lines(card.x, card.y, card.w, card.h, 1.0, colors::BORDER_DIM);
            draw_small_caps(
                &format!("Rule {}", i + 1),
                card.x + 18.0,
                card.y + 24.0,
                fonts::SIZE_SM,
                colors::CAB_YELLOW,
            );
            draw_ui_text(
                &rule.title,
                card.x + 96.0,
                card.y + 27.0,
                fonts::SIZE_LG,
                colors::TEXT_PRIMARY,
            );
            draw_wrapped_text(
                &rule.description,
                card.x + 96.0,
                card.y + 50.0,
                card.w - 116.0,
                fonts::SIZE_XS,
                15.0,
                colors::TEXT_SECONDARY,
                2,
            );
            y += card_h + 12.0;
            if y > rules_panel.bottom() - 22.0 {
                break;
            }
        }
        let target_y = (rules_panel.bottom() - 112.0).max(y + 10.0);
        if target_y + 84.0 <= rules_panel.bottom() - 16.0 {
            draw_small_caps(
                "Shift Targets",
                rules_inner.x,
                target_y - 14.0,
                fonts::SIZE_SM,
                colors::TEXT_MUTED,
            );
            let stat_gap = 12.0;
            let stat_w = (rules_inner.w - stat_gap * 2.0) / 3.0;
            let stats = [
                (
                    "Fuel",
                    format!("{:.0}%", game_state.fuel),
                    colors::FUEL_GOOD,
                ),
                (
                    "Minimum",
                    format!("${}", game_state.minimum_earnings),
                    colors::ACCENT_GOLD,
                ),
                (
                    "Difficulty",
                    format!("{}", game_state.difficulty_level + 1),
                    colors::ACCENT_SKY,
                ),
            ];
            for (idx, (label, value, color)) in stats.iter().enumerate() {
                let rect = UiRect::new(
                    rules_inner.x + idx as f32 * (stat_w + stat_gap),
                    target_y,
                    stat_w,
                    74.0,
                );
                draw_rectangle(
                    rect.x,
                    rect.y,
                    rect.w,
                    rect.h,
                    Color::new(0.025, 0.030, 0.032, 0.92),
                );
                draw_rectangle(rect.x, rect.y, 4.0, rect.h, *color);
                draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, colors::BORDER_DIM);
                draw_ui_text(value, rect.x + 18.0, rect.y + 31.0, fonts::SIZE_XL, *color);
                draw_small_caps(
                    label,
                    rect.x + 18.0,
                    rect.y + 55.0,
                    fonts::SIZE_XS,
                    colors::TEXT_MUTED,
                );
            }
        }

        let conditions_inner = conditions_panel.inset(18.0);
        draw_small_caps(
            "Night Conditions",
            conditions_inner.x,
            conditions_inner.y + 16.0,
            fonts::SIZE_LG,
            colors::CAB_YELLOW,
        );

        let mut side_y = conditions_inner.y + 58.0;
        let weather_label = &data.localization.ui.briefing.weather_title;
        let weather_text = format!(
            "{} {} - {}",
            weather_label,
            game_state.current_weather.weather_type.name(),
            game_state.current_weather.description
        );
        // The weather box is a fixed height while the description inside it
        // runs one to three lines, so `side_y` has to clear the box rather
        // than the text: advancing by the text height alone drew the weather
        // advisory and the hazard list inside the box on any clear night.
        const WEATHER_PANEL_H: f32 = 108.0;
        let weather_panel_top = side_y;
        draw_rectangle(
            conditions_inner.x,
            side_y,
            conditions_inner.w,
            WEATHER_PANEL_H,
            Color::new(0.020, 0.040, 0.050, 0.92),
        );
        draw_rectangle(
            conditions_inner.x,
            side_y,
            4.0,
            WEATHER_PANEL_H,
            colors::ACCENT_SKY,
        );
        draw_rectangle_lines(
            conditions_inner.x,
            side_y,
            conditions_inner.w,
            WEATHER_PANEL_H,
            1.0,
            colors::BORDER_DIM,
        );
        draw_ui_text(
            game_state.current_weather.weather_type.name(),
            conditions_inner.x + 18.0,
            side_y + 34.0,
            fonts::SIZE_XL,
            colors::ACCENT_SKY,
        );
        let weather_text_bottom = draw_wrapped_text(
            &weather_text,
            conditions_inner.x + 18.0,
            side_y + 62.0,
            conditions_inner.w - 36.0,
            fonts::SIZE_SM,
            18.0,
            colors::TEXT_SECONDARY,
            3,
        );
        side_y = weather_text_bottom.max(weather_panel_top + WEATHER_PANEL_H) + 18.0;

        // The season line: computed every run and previously shown nowhere,
        // though it steers spawn weights and hazard chances.
        side_y = draw_wrapped_text(
            &game_state.season.description,
            conditions_inner.x,
            side_y,
            conditions_inner.w,
            fonts::SIZE_SM,
            18.0,
            colors::TEXT_MUTED,
            2,
        ) + 14.0;

        // Tonight's rolled modifier, named in the forecast — it has already
        // moved the quota and the fares the driver is about to be offered,
        // so a night that plays different must say so up front.
        if let Some(modifier) = &game_state.night_modifier {
            side_y = draw_wrapped_text(
                &format!(
                    "{} — {}",
                    modifier.name.to_uppercase(),
                    modifier.description
                ),
                conditions_inner.x,
                side_y,
                conditions_inner.w,
                fonts::SIZE_SM,
                18.0,
                colors::ACCENT_DANGER,
                3,
            ) + 14.0;
        }

        let weather_rule_ids = crate::engine::WeatherService::get_weather_triggered_rules(
            &game_state.current_weather,
            &game_state.time_of_day,
        );
        if !weather_rule_ids.is_empty() {
            let advisory = format!("Weather advisories active: {}", weather_rule_ids.len());
            side_y = draw_wrapped_text(
                &advisory,
                conditions_inner.x,
                side_y,
                conditions_inner.w,
                fonts::SIZE_SM,
                18.0,
                colors::ACCENT_WARNING,
                2,
            ) + 14.0;
        }

        // What this driver's own kit does to a hazard, so the forecast is
        // theirs rather than the hazard's.
        let hazard_mult = crate::engine::SkillModifiers::from_unlocked(
            &data.skills,
            &player_stats.unlocked_skills,
        )
        .hazard_mult;

        if !game_state.environmental_hazards.is_empty() {
            draw_small_caps(
                "Active Hazards",
                conditions_inner.x,
                side_y,
                fonts::SIZE_LG,
                colors::ACCENT_WARNING,
            );
            side_y += 30.0;
            for hazard in game_state.environmental_hazards.iter().take(3) {
                // `hazard.description` already names the location -- "Minor
                // road work on Downtown Bridge" -- so prefixing it printed
                // the place twice. What it never said was what the hazard
                // costs, which is the only reason to read this list.
                let text = match hazard.toll(hazard_mult) {
                    Some(toll) => format!("{} ({})", hazard.description, toll),
                    None => hazard.description.clone(),
                };
                side_y = draw_wrapped_text(
                    &text,
                    conditions_inner.x,
                    side_y,
                    conditions_inner.w,
                    fonts::SIZE_SM,
                    18.0,
                    colors::TEXT_SECONDARY,
                    2,
                ) + 10.0;
            }
        } else {
            draw_small_caps(
                "No active environmental hazards reported.",
                conditions_inner.x,
                side_y,
                fonts::SIZE_SM,
                colors::TEXT_MUTED,
            );
        }
        let condition_stats_y = (conditions_panel.bottom() - 112.0).max(side_y + 12.0);
        if condition_stats_y + 82.0 <= conditions_panel.bottom() - 16.0 {
            draw_small_caps(
                "Readings",
                conditions_inner.x,
                condition_stats_y - 14.0,
                fonts::SIZE_SM,
                colors::TEXT_MUTED,
            );
            let stat_gap = 10.0;
            let stat_w = (conditions_inner.w - stat_gap * 2.0) / 3.0;
            let readings = [
                (
                    "Visibility",
                    format!("{}%", game_state.current_weather.visibility),
                    colors::ACCENT_SKY,
                ),
                (
                    "Activity",
                    format!("{}%", game_state.time_of_day.supernatural_activity),
                    colors::ACCENT_WARNING,
                ),
                (
                    "Hour",
                    format_clock(game_state.time_of_day.hour, 0),
                    colors::TEXT_SECONDARY,
                ),
            ];
            for (idx, (label, value, color)) in readings.iter().enumerate() {
                let rect = UiRect::new(
                    conditions_inner.x + idx as f32 * (stat_w + stat_gap),
                    condition_stats_y,
                    stat_w,
                    74.0,
                );
                draw_rectangle(
                    rect.x,
                    rect.y,
                    rect.w,
                    rect.h,
                    Color::new(0.025, 0.030, 0.032, 0.92),
                );
                draw_rectangle(rect.x, rect.y, 4.0, rect.h, *color);
                draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, colors::BORDER_DIM);
                draw_ui_text(value, rect.x + 14.0, rect.y + 31.0, fonts::SIZE_LG, *color);
                draw_small_caps(
                    label,
                    rect.x + 14.0,
                    rect.y + 55.0,
                    fonts::SIZE_XS,
                    colors::TEXT_MUTED,
                );
            }
        }

        if scene_panel.h >= 120.0 {
            draw_briefing_taxi_scene(scene_panel);
        } else {
            draw_glass_panel(scene_panel, colors::BORDER_DIM);
        }

        let btn_rect = UiRect::new(screen_w / 2.0 - 170.0, screen_h - 62.0, 340.0, 48.0);
        if draw_glass_button(
            btn_rect,
            &data.localization.ui.briefing.begin_space,
            colors::CAB_YELLOW,
            true,
        ) {
            return UiAction::StartGame;
        }
        // A seeded run says so, or a replayed night would look like an
        // impossible coincidence.
        if let Some(seed) = run_seed {
            draw_small_caps(
                &format!("Seeded run: {seed}"),
                screen_w - margin - 220.0,
                screen_h - 40.0,
                fonts::SIZE_XS,
                colors::ACCENT_SKY,
            );
        }

        // The night can still be walked away from: nothing is committed
        // until the shift starts.
        let back_rect = UiRect::new(margin, screen_h - 62.0, 220.0, 48.0);
        if draw_glass_button(
            back_rect,
            &data.localization.ui.common.back_button,
            colors::TEXT_MUTED,
            true,
        ) {
            return UiAction::ReturnToMenu;
        }
    }

    UiAction::None
}

fn draw_narrow_briefing(
    game_state: &GameState,
    data: &GameData,
    run_seed: Option<u64>,
) -> UiAction {
    let margin = 10.0;
    let width = screen_width() - margin * 2.0;
    let panel = UiRect::new(margin, 10.0, width, screen_height() - 20.0);
    draw_glass_panel(panel, colors::BORDER_DIM);
    let inner = panel.inset(10.0);
    draw_ui_text(
        &data.localization.ui.briefing.title,
        inner.x,
        inner.y + 20.0,
        20.0,
        colors::CAB_YELLOW,
    );
    let night_label = data
        .localization
        .ui
        .briefing
        .night_label
        .replacen("{}", &game_state.night.to_string(), 1)
        .replacen(
            "{}",
            &data.constants.game_constants.nights_per_run.to_string(),
            1,
        );
    draw_small_caps(
        &night_label,
        inner.x,
        inner.y + 40.0,
        fonts::SIZE_XS,
        colors::TEXT_MUTED,
    );
    let rule_text = game_state
        .current_rules
        .first()
        .map(|rule| format!("RULE: {} — {}", rule.title, rule.description))
        .unwrap_or_else(|| "RULE: Watch every passenger before you act.".to_string());
    draw_wrapped_text(
        &rule_text,
        inner.x,
        inner.y + 58.0,
        inner.w,
        fonts::SIZE_XS,
        11.0,
        colors::TEXT_PRIMARY,
        3,
    );
    draw_small_caps(
        &format!(
            "WEATHER: {} | FUEL: {:.0}% | TARGET: ${}",
            game_state.current_weather.weather_type.name(),
            game_state.fuel,
            game_state.minimum_earnings
        ),
        inner.x,
        inner.y + 101.0,
        fonts::SIZE_XS,
        colors::ACCENT_SKY,
    );
    if let Some(seed) = run_seed {
        draw_small_caps(
            &format!("SEEDED RUN: {seed}"),
            inner.x,
            inner.y + 118.0,
            fonts::SIZE_XS,
            colors::ACCENT_SKY,
        );
    }

    let button_gap = 8.0;
    let button_w = (inner.w - button_gap) / 2.0;
    let button_y = panel.bottom() - 38.0;
    if draw_glass_button(
        UiRect::new(inner.x, button_y, button_w, 28.0),
        &data.localization.ui.briefing.begin_space,
        colors::CAB_YELLOW,
        true,
    ) {
        return UiAction::StartGame;
    }
    if draw_glass_button(
        UiRect::new(inner.x + button_w + button_gap, button_y, button_w, 28.0),
        &data.localization.ui.common.back_button,
        colors::TEXT_MUTED,
        true,
    ) {
        return UiAction::ReturnToMenu;
    }
    UiAction::None
}

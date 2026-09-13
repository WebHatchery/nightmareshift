//! The between-rides screen: fuel, refuelling, shift metrics, and dispatch.

use macroquad::prelude::*;

use crate::data::GameData;
use crate::state::GameState;
use crate::ui::draw_ui_text;
use crate::ui::{
    colors, draw_cockpit_background, draw_glass_button, draw_glass_panel, draw_small_caps,
    draw_ui_icon, draw_wrapped_text, fonts, get_fuel_color, layout, spacing, UiAction, UiIcon,
    UiRect,
};
use macroquad_toolkit::ui::format_clock;

use super::scene::{draw_bottom_taxi_scene, draw_metric_tile};

/// Draw the waiting for passenger screen
pub fn draw_waiting(
    game_state: &GameState,
    game_data: Option<&GameData>,
    player_stats: &crate::state::PlayerStats,
) -> UiAction {
    draw_cockpit_background();

    if screen_width() < 980.0 || screen_height() < 560.0 {
        return draw_narrow_waiting(game_state, game_data);
    }

    let scene_h = (screen_height() * 0.28).clamp(220.0, 310.0);
    let scene_rect = UiRect::new(
        70.0,
        screen_height() - scene_h - 34.0,
        screen_width() - 140.0,
        scene_h,
    );
    draw_bottom_taxi_scene(scene_rect);

    if let Some(data) = game_data {
        let panel_w = (screen_width() - 140.0).min(1060.0);
        let panel_h = (scene_rect.y - layout::STATUS_BAR_HEIGHT - 78.0).clamp(390.0, 470.0);
        let panel = UiRect::centered_x(
            screen_width(),
            layout::STATUS_BAR_HEIGHT + 36.0,
            panel_w,
            panel_h,
        );
        draw_glass_panel(panel, colors::BORDER);
        let inner = panel.inset(spacing::PADDING_MD);
        let left_w = inner.w * 0.56;
        let right_x = inner.x + left_w + 28.0;
        let right_w = inner.w - left_w - 28.0;
        let mut y = inner.y + 16.0;

        draw_ui_icon(
            UiIcon::Cab,
            inner.x + 10.0,
            y - 4.0,
            20.0,
            colors::CAB_YELLOW,
        );
        draw_small_caps(
            "Dispatch",
            inner.x + 28.0,
            y,
            fonts::SIZE_SM,
            colors::CAB_YELLOW,
        );
        y += 36.0;
        draw_ui_text(
            &data.localization.ui.game.waiting.looking,
            inner.x,
            y,
            fonts::SIZE_XXL,
            colors::TEXT_PRIMARY,
        );
        y += 52.0;

        let fuel_pct = game_state.fuel;
        let fuel_color = get_fuel_color(fuel_pct, &data.constants.fuel);
        let fuel_status = if fuel_pct <= data.constants.fuel.critical_fuel as f32 {
            &data.localization.ui.game.waiting.fuel_status.critical
        } else if fuel_pct <= data.constants.fuel.low_fuel_warning as f32 {
            &data.localization.ui.game.waiting.fuel_status.low
        } else if fuel_pct <= 40.0 {
            &data.localization.ui.game.waiting.fuel_status.medium
        } else {
            &data.localization.ui.game.waiting.fuel_status.good
        };

        // "⛽ Fuel: {:.0}% - {}"
        let fuel_text = data
            .localization
            .ui
            .game
            .waiting
            .fuel_label
            .replace("{:.0}", &format!("{:.0}", fuel_pct))
            .replace("{}", fuel_status)
            .replace("%", &data.localization.ui.common.percent);
        draw_wrapped_text(
            &fuel_text,
            inner.x,
            y,
            left_w,
            fonts::SIZE_LG,
            26.0,
            fuel_color,
            2,
        );
        y += 46.0;

        let inv_hint = data
            .localization
            .ui
            .game
            .waiting
            .inventory_hint
            .replace("{}", &game_state.inventory.len().to_string());
        draw_wrapped_text(
            &inv_hint,
            inner.x,
            y,
            left_w,
            fonts::SIZE_SM,
            19.0,
            colors::TEXT_MUTED,
            2,
        );

        if game_state.last_fare_night {
            // Every station is closed to this fare; what is in the tank is
            // the night's whole budget.
            let refuel_y = inner.y + inner.h - 112.0;
            draw_small_caps(
                "No station serves this fare.",
                inner.x,
                refuel_y,
                fonts::SIZE_SM,
                colors::ACCENT_DANGER,
            );
        } else if fuel_pct < game_state.max_fuel {
            let refuel_y = inner.y + inner.h - 112.0;
            draw_small_caps(
                &data.localization.ui.game.waiting.refuel_title,
                inner.x,
                refuel_y,
                fonts::SIZE_SM,
                colors::TEXT_SECONDARY,
            );

            let btn_w = (left_w - 16.0) / 2.0;
            let btn_h = 44.0;

            // The driver's own discount, so the price on the button is the
            // price at the pump.
            let refuel_mult = crate::engine::SkillModifiers::from_unlocked(
                &data.skills,
                &player_stats.unlocked_skills,
            )
            .refuel_cost_mult;

            let fuel_needed = game_state.max_fuel - fuel_pct;
            let full_cost = data.constants.fuel.refuel_cost(fuel_needed, refuel_mult);
            let full_label = data
                .localization
                .ui
                .game
                .waiting
                .full_tank
                .replace("{}", &full_cost.to_string());
            let can_afford_full = game_state.earnings >= full_cost;

            if draw_glass_button(
                UiRect::new(inner.x, refuel_y + 26.0, btn_w, btn_h),
                &full_label,
                colors::ACCENT_SKY,
                can_afford_full,
            ) {
                return UiAction::RefuelFull;
            }

            let partial_amount = data.constants.fuel.partial_refuel_amount.min(fuel_needed);
            let partial_cost = data.constants.fuel.refuel_cost(partial_amount, refuel_mult);
            let partial_label = data
                .localization
                .ui
                .game
                .waiting
                .partial
                .replace("{}", &partial_cost.to_string());
            let can_afford_partial = game_state.earnings >= partial_cost;

            if draw_glass_button(
                UiRect::new(inner.x + btn_w + 16.0, refuel_y + 26.0, btn_w, btn_h),
                &partial_label,
                colors::ACCENT_SKY,
                can_afford_partial,
            ) {
                return UiAction::RefuelPartial;
            }
        }

        let hours = game_state.time_remaining / 60;
        let mins = game_state.time_remaining % 60;
        let tile_gap = 12.0;
        let tile_w = (right_w - tile_gap) / 2.0;
        let tile_h = 86.0;
        draw_metric_tile(
            UiRect::new(right_x, inner.y + 18.0, tile_w, tile_h),
            "Fuel",
            &format!("{:.0}%", fuel_pct),
            fuel_color,
        );
        draw_metric_tile(
            UiRect::new(right_x + tile_w + tile_gap, inner.y + 18.0, tile_w, tile_h),
            "Earnings",
            &format!("${}", game_state.earnings),
            colors::ACCENT_GOLD,
        );
        draw_metric_tile(
            UiRect::new(right_x, inner.y + 18.0 + tile_h + tile_gap, tile_w, tile_h),
            "Time",
            &format_clock(hours, mins),
            colors::TEXT_PRIMARY,
        );
        draw_metric_tile(
            UiRect::new(
                right_x + tile_w + tile_gap,
                inner.y + 18.0 + tile_h + tile_gap,
                tile_w,
                tile_h,
            ),
            "Rides",
            &game_state.rides_completed.to_string(),
            colors::TEXT_PRIMARY,
        );

        let weather_rect = UiRect::new(
            right_x,
            inner.y + 18.0 + (tile_h + tile_gap) * 2.0,
            right_w,
            76.0,
        );
        draw_metric_tile(
            weather_rect,
            "Weather",
            game_state.current_weather.weather_type.name(),
            colors::ACCENT_SKY,
        );

        let earned_enough =
            game_state.earnings >= game_state.minimum_earnings && !game_state.last_fare_night;
        let find_y = panel.bottom() - 72.0;
        if earned_enough {
            let action_gap = 14.0;
            let action_w = (right_w - action_gap) / 2.0;
            let end_rect = UiRect::new(right_x, find_y, action_w, 50.0);
            if draw_glass_button(end_rect, "Cash Out Shift", colors::FUEL_GOOD, true) {
                return UiAction::EndShift;
            }
            let find_rect = UiRect::new(right_x + action_w + action_gap, find_y, action_w, 50.0);
            if draw_glass_button(
                find_rect,
                &data.localization.ui.game.waiting.find_passenger,
                colors::CAB_YELLOW,
                true,
            ) {
                return UiAction::Continue;
            }
            return UiAction::None;
        }

        let find_rect = UiRect::new(right_x, find_y, right_w, 50.0);
        if draw_glass_button(
            find_rect,
            &data.localization.ui.game.waiting.find_passenger,
            colors::CAB_YELLOW,
            true,
        ) {
            return UiAction::Continue;
        }

        UiAction::None
    } else {
        UiAction::None
    }
}

fn draw_narrow_waiting(game_state: &GameState, game_data: Option<&GameData>) -> UiAction {
    let Some(data) = game_data else {
        return UiAction::None;
    };
    let panel = UiRect::new(
        12.0,
        layout::STATUS_BAR_HEIGHT + 8.0,
        screen_width() - 24.0,
        110.0,
    );
    draw_glass_panel(panel, colors::BORDER);
    draw_small_caps(
        "DISPATCH",
        panel.x + 12.0,
        panel.y + 20.0,
        fonts::SIZE_XS,
        colors::CAB_YELLOW,
    );
    draw_ui_text(
        &data.localization.ui.game.waiting.looking,
        panel.x + 12.0,
        panel.y + 48.0,
        fonts::SIZE_LG,
        colors::TEXT_PRIMARY,
    );
    draw_small_caps(
        &format!(
            "FUEL {:.0}%  |  EARNINGS ${}  |  RIDES {}",
            game_state.fuel, game_state.earnings, game_state.rides_completed
        ),
        panel.x + 12.0,
        panel.y + 72.0,
        fonts::SIZE_XS,
        colors::TEXT_MUTED,
    );

    let button_y = screen_height() - 34.0;
    let button_w = screen_width() - 24.0;
    let earned_enough =
        game_state.earnings >= game_state.minimum_earnings && !game_state.last_fare_night;
    if earned_enough {
        let gap = 6.0;
        let half = (button_w - gap) / 2.0;
        if draw_glass_button(
            UiRect::new(12.0, button_y, half, 26.0),
            "CASH OUT",
            colors::FUEL_GOOD,
            true,
        ) {
            return UiAction::EndShift;
        }
        if draw_glass_button(
            UiRect::new(12.0 + half + gap, button_y, half, 26.0),
            &data.localization.ui.game.waiting.find_passenger,
            colors::CAB_YELLOW,
            true,
        ) {
            return UiAction::Continue;
        }
    } else if draw_glass_button(
        UiRect::new(12.0, button_y, button_w, 26.0),
        &data.localization.ui.game.waiting.find_passenger,
        colors::CAB_YELLOW,
        true,
    ) {
        return UiAction::Continue;
    }
    UiAction::None
}

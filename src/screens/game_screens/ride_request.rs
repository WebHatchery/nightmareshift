//! The ride offer screen: passenger portrait, fare, and accept/decline.

use macroquad::prelude::*;

use crate::data::{GameData, Rarity};
use crate::state::{GameState, PlayerStats};
use crate::ui::draw_ui_text;
use crate::ui::{
    colors, draw_cockpit_background, draw_glass_button, draw_glass_panel, draw_passenger_portrait,
    draw_small_caps, draw_wrapped_text, fonts, layout, spacing, UiAction, UiRect,
};

use super::dossier;
use super::scene::draw_bottom_taxi_scene;

/// Draw the ride request screen
pub fn draw_ride_request(
    game_state: &GameState,
    game_data: Option<&GameData>,
    player_stats: &PlayerStats,
) -> UiAction {
    draw_cockpit_background();

    let scene_h = (screen_height() * 0.30).clamp(220.0, 320.0);
    let scene_rect = UiRect::new(
        70.0,
        screen_height() - scene_h - 34.0,
        screen_width() - 140.0,
        scene_h,
    );
    draw_bottom_taxi_scene(scene_rect);

    if let Some(ref passenger) = game_state.current_passenger {
        let panel_w = screen_width().min(1040.0);
        let panel_h = (screen_height() * 0.50).clamp(440.0, 540.0);
        let panel = UiRect::centered_x(
            screen_width(),
            layout::STATUS_BAR_HEIGHT + 36.0,
            panel_w,
            panel_h,
        );
        draw_glass_panel(panel, colors::BORDER);
        let inner = panel.inset(spacing::PADDING_MD);

        let portrait_size = (inner.h - 12.0).min(inner.w * 0.47).clamp(360.0, 500.0);
        let portrait_rect = UiRect::new(inner.x, inner.y, portrait_size, portrait_size);
        draw_passenger_portrait(portrait_rect, passenger.id);

        let info_x = portrait_rect.x + portrait_rect.w + 28.0;
        let info_w = inner.x + inner.w - info_x;
        let mut y = inner.y + 12.0;

        draw_small_caps(
            "Ride Request",
            info_x,
            y,
            fonts::SIZE_SM,
            colors::TEXT_MUTED,
        );
        y += 38.0;
        draw_ui_text(
            &passenger.name,
            info_x,
            y,
            fonts::SIZE_XXL,
            colors::CAB_YELLOW,
        );
        y += 30.0;

        let rarity_text = format!("{:?}", passenger.rarity);
        let rarity_color = match passenger.rarity {
            Rarity::Common => colors::TEXT_MUTED,
            Rarity::Uncommon => colors::ACCENT_PRIMARY,
            Rarity::Rare => colors::ACCENT_SKY,
            Rarity::Legendary => colors::ACCENT_GOLD,
        };
        draw_small_caps(&rarity_text, info_x, y, fonts::SIZE_XS, rarity_color);
        y += 34.0;

        y = draw_wrapped_text(
            &passenger.description,
            info_x,
            y,
            info_w,
            fonts::SIZE_MD,
            21.0,
            colors::TEXT_SECONDARY,
            2,
        ) + 14.0;

        let route = format!("{} -> {}", passenger.pickup, passenger.destination);
        y = draw_wrapped_text(
            &route,
            info_x,
            y,
            info_w,
            fonts::SIZE_MD,
            21.0,
            colors::TEXT_PRIMARY,
            2,
        ) + 4.0;

        // What kind of places these are. Every location authors an
        // `atmosphere` and a `riskLevel`; the risk has been feeding route
        // costs all along through the pickup, while the atmosphere naming the
        // reason for it was read by nothing. A driver deciding whether to take
        // a fare should know they are being called out to a cemetery.
        if let Some(data) = game_data {
            let describe = |name: &String| {
                data.get_location(name)
                    .map(|location| (location.atmosphere.as_str(), location.risk_level))
            };
            if let (Some((from, from_risk)), Some((to, to_risk))) = (
                describe(&passenger.pickup),
                describe(&passenger.destination),
            ) {
                let worst = from_risk.max(to_risk);
                let colour = if worst >= data.constants.risk.high_risk {
                    colors::FUEL_CRITICAL
                } else if worst > data.constants.risk.low_risk {
                    colors::ACCENT_WARNING
                } else {
                    colors::TEXT_MUTED
                };
                y = draw_wrapped_text(
                    &format!("{} to {}", from, to),
                    info_x,
                    y,
                    info_w,
                    fonts::SIZE_XS,
                    15.0,
                    colour,
                    2,
                );
            }
        }
        y += 14.0;

        // What this fare is worth, across the four roads. A single number here
        // was the authored base, before standing, destination and fare skills,
        // and the driver has to decide whether to take the ride on it.
        let fare_text = match game_data {
            Some(data) => {
                let (low, high) = crate::engine::RouteService::fare_range(
                    passenger,
                    game_state,
                    data,
                    player_stats,
                );
                if low == high {
                    format!("${low}")
                } else {
                    format!("${low} - ${high}")
                }
            }
            None => format!("${}", passenger.fare),
        };

        draw_ui_text(&fare_text, info_x, y, fonts::SIZE_XL, colors::ACCENT_GOLD);
        y += 42.0;

        // What the almanac has bought you about this fare. Studying a
        // passenger pays off here, before the accept/decline decision.
        let btn_h = 48.0;
        let dossier_limit = panel.bottom() - spacing::PADDING_MD - btn_h - 16.0;
        let knowledge = player_stats.get_almanac_entry(passenger.id).knowledge_level;
        let driver = dossier::DriverContext {
            shift: game_state,
            stats: player_stats,
        };
        let lines = dossier::build(passenger, knowledge, game_data, Some(&driver));
        if !lines.is_empty() {
            draw_small_caps(
                &format!("Almanac Lv.{}", knowledge),
                info_x,
                y,
                fonts::SIZE_XS,
                colors::ACCENT_GOLD,
            );
            y += 18.0;
        }
        for line in &lines {
            if y + 16.0 > dossier_limit {
                break;
            }
            let color = match line.level {
                1 => colors::ACCENT_SKY,
                2 => colors::ACCENT_GOLD,
                _ => colors::FUEL_GOOD,
            };
            draw_small_caps(&line.label, info_x, y, fonts::SIZE_XS, color);
            y = draw_wrapped_text(
                &line.value,
                info_x + 82.0,
                y,
                info_w - 82.0,
                fonts::SIZE_XS,
                15.0,
                colors::TEXT_SECONDARY,
                2,
            ) + 4.0;
        }

        if let Some(dialogue_text) = game_state.current_passenger_dialogue.as_ref() {
            if y + 18.0 <= dossier_limit {
                let preview = if dialogue_text.len() > 100 {
                    format!("\"{}...\"", &dialogue_text[..100])
                } else {
                    format!("\"{}\"", dialogue_text)
                };
                draw_wrapped_text(
                    &preview,
                    info_x,
                    y + 8.0,
                    info_w,
                    fonts::SIZE_SM,
                    18.0,
                    colors::TEXT_MUTED,
                    2,
                );
            }
        }

        let accept_text = format!(
            "Accept ({})",
            player_stats.accessibility.key_bindings.accept
        );
        let decline_text = format!(
            "Decline ({})",
            player_stats.accessibility.key_bindings.decline
        );

        let gap = 14.0;
        let btn_w = (info_w - gap) / 2.0;
        let btn_y = panel.bottom() - spacing::PADDING_MD - btn_h;
        if draw_glass_button(
            UiRect::new(info_x, btn_y, btn_w, btn_h),
            &accept_text,
            colors::ACCENT_PRIMARY,
            true,
        ) {
            return UiAction::AcceptRide;
        }
        if draw_glass_button(
            UiRect::new(info_x + btn_w + gap, btn_y, btn_w, btn_h),
            &decline_text,
            colors::ACCENT_DANGER,
            true,
        ) {
            return UiAction::DeclineRide;
        }
    }
    UiAction::None
}

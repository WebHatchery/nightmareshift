//! The route selection screen: route cards, risk tags, and passenger reactions.

use macroquad::prelude::*;

use crate::data::{self, GameData, PreferenceLevel, RouteType, TimePhase};
use crate::state::{DrivingPhase, GameState};
use crate::ui::draw_ui_text;
use crate::ui::{
    colors, draw_cockpit_background, draw_glass_button, draw_glass_panel, draw_small_caps,
    draw_ui_icon, draw_wrapped_text, fonts, layout, UiAction, UiIcon, UiRect,
};

use super::scene::{draw_bottom_taxi_scene, pulsing_warning_color};

/// Draw the driving/route selection screen
pub fn draw_driving(
    game_state: &GameState,
    game_data: Option<&GameData>,
    player_stats: &crate::state::PlayerStats,
) -> UiAction {
    draw_cockpit_background();

    if screen_width() < 980.0 || screen_height() < 560.0 {
        return draw_narrow_driving(game_state, game_data);
    }

    let scene_h = (screen_height() * 0.27).clamp(210.0, 300.0);
    let scene_rect = UiRect::new(
        70.0,
        screen_height() - scene_h - 34.0,
        screen_width() - 140.0,
        scene_h,
    );
    draw_bottom_taxi_scene(scene_rect);

    if let Some(data) = game_data {
        let margin = 26.0;
        let top = layout::STATUS_BAR_HEIGHT + 22.0;
        let gap = 18.0;
        let passenger_w = (screen_width() * 0.17).clamp(260.0, 330.0);
        let left_w = screen_width() - margin * 2.0 - gap - passenger_w;
        let narrow_cards = left_w < 760.0;
        let cost_column_w = if narrow_cards { 150.0 } else { 154.0 };
        let tag_column_w = if narrow_cards { 325.0 } else { 430.0 };
        let left_x = margin;
        let right_x = left_x + left_w + gap;
        let content_bottom = scene_rect.y - 22.0;

        let header = UiRect::new(left_x, top, left_w, 88.0);
        draw_glass_panel(header, colors::BORDER_DIM);
        let header_inner = header.inset(14.0);

        let phase_text = match game_state.driving_phase {
            Some(DrivingPhase::Pickup) => &data.localization.ui.game.driving.pickup,
            Some(DrivingPhase::Destination) => &data.localization.ui.game.driving.destination,
            None => "Driving...",
        };

        draw_ui_text(
            phase_text,
            header_inner.x,
            header_inner.y + 26.0,
            fonts::SIZE_XL,
            colors::TEXT_PRIMARY,
        );

        if let Some(need_state) = &game_state.current_passenger_need_state {
            let stability = crate::engine::PassengerStateMachine::get_stability_percent(need_state);
            let critical = crate::engine::PassengerStateMachine::is_critical(need_state);
            // Named as well as measured. The almanac sells the thresholds in
            // need levels and this gauge reads the inverse, so without the word
            // the two could not be compared.
            let state_text = format!(
                "Passenger stability: {}% - {}",
                stability,
                need_state.stage.label()
            );
            let state_color = if critical {
                colors::FUEL_CRITICAL
            } else {
                colors::FUEL_GOOD
            };
            draw_small_caps(
                &state_text,
                header_inner.x,
                header_inner.y + 52.0,
                fonts::SIZE_XS,
                state_color,
            );
        }

        if let Some(dialogue) = &game_state.current_dialogue {
            let speaker = match dialogue.speaker {
                crate::state::DialogueSpeaker::Passenger => "Passenger",
                crate::state::DialogueSpeaker::Driver => "Driver",
                crate::state::DialogueSpeaker::Narrator => "Dispatch",
            };
            let elapsed = (get_time() - dialogue.timestamp).max(0.0);
            let line = format!("{} ({:.0}s ago): {}", speaker, elapsed, dialogue.text);
            let preview = if line.len() > 86 {
                format!("{}...", &line[..86])
            } else {
                line
            };
            draw_ui_text(
                &preview,
                header_inner.x,
                header_inner.y + 68.0,
                fonts::SIZE_XS,
                colors::TEXT_SECONDARY,
            );
        }

        if let Some(ride) = &game_state.current_ride {
            let target = match game_state.driving_phase {
                Some(DrivingPhase::Pickup) => &ride.pickup_location,
                Some(DrivingPhase::Destination) => &ride.destination_location,
                None => &ride.destination_location,
            };
            let elapsed = (get_time() - ride.start_time).max(0.0);
            let ride_text = format!("{} -> {} | {:.0}s", ride.passenger.name, target, elapsed);
            draw_small_caps(
                &ride_text,
                header_inner.x + left_w * 0.54,
                header_inner.y + 52.0,
                fonts::SIZE_XS,
                colors::TEXT_MUTED,
            );

            // Where that road ends, as the location file paints it. All 24
            // locations author this line; it was loaded and never drawn.
            if let Some(location) = data.get_location(target) {
                draw_ui_text(
                    &location.description,
                    header_inner.x + left_w * 0.54,
                    header_inner.y + 68.0,
                    fonts::SIZE_XS,
                    colors::TEXT_MUTED,
                );
            }
        }

        let r = &data.localization.ui.game.driving.routes;
        let routes = [
            (RouteType::Normal, &r.normal, &r.normal_desc, "[1]"),
            (RouteType::Shortcut, &r.shortcut, &r.shortcut_desc, "[2]"),
            (RouteType::Scenic, &r.scenic, &r.scenic_desc, "[3]"),
            (RouteType::Police, &r.police, &r.police_desc, "[4]"),
        ];

        let passenger_opt = game_state.current_passenger.as_ref();
        let passenger_knowledge = passenger_opt
            .map(|passenger| player_stats.get_almanac_entry(passenger.id).knowledge_level)
            .unwrap_or(0);
        let reveal_route_preferences = passenger_knowledge >= 2;
        let reveal_preference_reasons = passenger_knowledge >= 3;
        let mut route_y = header.bottom() + 10.0;
        let route_h = ((content_bottom - route_y - 24.0) / 4.0 - 8.0).clamp(66.0, 82.0);
        for (i, (route_type, name, desc, key)) in routes.iter().enumerate() {
            let is_blocked = game_state
                .environmental_hazards
                .iter()
                .any(|h| h.blocks_route(*route_type));

            let mut weather_warning = String::new();
            if matches!(
                game_state.current_weather.intensity,
                data::WeatherIntensity::Heavy
            ) && *route_type == RouteType::Shortcut
            {
                let w_type = game_state.current_weather.weather_type.name();
                weather_warning = data
                    .localization
                    .ui
                    .game
                    .driving
                    .weather_warning
                    .replace("{}", w_type);
            }
            if matches!(
                game_state.time_of_day.phase,
                TimePhase::Night | TimePhase::Latenight
            ) && *route_type == RouteType::Shortcut
            {
                if !weather_warning.is_empty() {
                    weather_warning.push_str(" + Night");
                } else {
                    weather_warning = data.localization.ui.game.driving.night_warning.clone();
                }
            }

            let route_usage = player_stats.get_route_usage(*route_type);
            let route_risk_known = route_usage > 0;
            let preference = passenger_opt.and_then(|p| p.get_route_preference(*route_type));
            let border = if is_blocked {
                colors::ACCENT_DANGER
            } else if reveal_route_preferences {
                if let Some(pref) = preference {
                    match pref.preference {
                        PreferenceLevel::Loves | PreferenceLevel::Likes => colors::FUEL_GOOD,
                        PreferenceLevel::Dislikes => colors::ACCENT_WARNING,
                        PreferenceLevel::Fears => colors::FUEL_CRITICAL,
                        PreferenceLevel::Neutral => colors::BORDER,
                    }
                } else {
                    colors::BORDER
                }
            } else {
                colors::BORDER
            };

            let route_title_color = if reveal_route_preferences {
                if let Some(pref) = preference {
                    match pref.preference {
                        PreferenceLevel::Loves | PreferenceLevel::Likes => colors::FUEL_GOOD,
                        PreferenceLevel::Dislikes => colors::ACCENT_WARNING,
                        PreferenceLevel::Fears => colors::FUEL_CRITICAL,
                        PreferenceLevel::Neutral => colors::TEXT_PRIMARY,
                    }
                } else if i == 0 {
                    colors::CAB_YELLOW
                } else {
                    colors::TEXT_PRIMARY
                }
            } else if i == 0 {
                colors::CAB_YELLOW
            } else {
                colors::TEXT_PRIMARY
            };

            let preference_label = if reveal_route_preferences {
                preference.and_then(|pref| {
                    let pref_text = pref.preference.display_text();
                    let pref_label = pref_text
                        .chars()
                        .filter(|ch| ch.is_ascii())
                        .collect::<String>();
                    let pref_label = pref_label.trim();
                    if pref_label.is_empty() {
                        None
                    } else {
                        let pref_color = match pref.preference {
                            PreferenceLevel::Loves | PreferenceLevel::Likes => colors::FUEL_GOOD,
                            PreferenceLevel::Neutral => colors::TEXT_MUTED,
                            PreferenceLevel::Dislikes => colors::ACCENT_WARNING,
                            PreferenceLevel::Fears => colors::FUEL_CRITICAL,
                        };
                        Some((
                            if narrow_cards {
                                format!("Known: {}", pref_label)
                            } else {
                                format!("Known: client {}", pref_label)
                            },
                            pref_color,
                        ))
                    }
                })
            } else {
                Some(("Preference unknown".to_string(), colors::TEXT_MUTED))
            };

            let route_detail = if reveal_preference_reasons {
                preference.map(|pref| pref.reason.as_str()).unwrap_or(*desc)
            } else {
                *desc
            };

            let tint = if is_blocked {
                Color::new(0.30, 0.04, 0.03, 0.28)
            } else if reveal_route_preferences {
                if let Some(pref) = preference {
                    match pref.preference {
                        PreferenceLevel::Loves => Color::new(0.06, 0.28, 0.08, 0.18),
                        PreferenceLevel::Likes => Color::new(0.06, 0.22, 0.10, 0.16),
                        PreferenceLevel::Neutral => Color::new(0.0, 0.0, 0.0, 0.0),
                        PreferenceLevel::Dislikes => Color::new(0.32, 0.18, 0.04, 0.14),
                        PreferenceLevel::Fears => Color::new(0.32, 0.04, 0.04, 0.18),
                    }
                } else {
                    Color::new(0.0, 0.0, 0.0, 0.0)
                }
            } else {
                Color::new(0.0, 0.0, 0.0, 0.0)
            };

            let visible_count = if route_usage >= 5 {
                3
            } else if route_usage >= 3 {
                2
            } else if route_usage >= 1 {
                1
            } else {
                0
            };

            // Quote what the engine will actually charge, not the base
            // constants. The two used to differ by everything weather,
            // hazards, mastery and skills contribute.
            let quote = crate::engine::RouteService::quote_route(
                *route_type,
                game_state,
                data,
                player_stats,
            );
            let (time_cost, fuel_cost, risk) = (quote.time, quote.fuel, quote.risk);
            // A route you cannot pay for is a lost shift the moment you pick
            // it, so it is refused here rather than in `validate_resources`.
            // Why a route cannot be taken, not merely that it cannot.
            //
            // An unaffordable card was drawn exactly like an open one -- name,
            // costs, risk tags -- and simply did not respond to a click. A
            // blocked route at least said BLOCKED. So the driver clicked a road
            // that looked available, nothing happened, and nothing on screen
            // explained it. This says which of the two ran out.
            let short_of_fuel = (game_state.fuel as u32) < quote.fuel;
            let short_of_time = game_state.time_remaining < quote.time;
            let unaffordable = short_of_fuel || short_of_time;
            let shortfall = if short_of_fuel {
                Some(data.localization.ui.game.driving.no_fuel.as_str())
            } else if short_of_time {
                Some(data.localization.ui.game.driving.no_time.as_str())
            } else {
                None
            };
            let selectable = !is_blocked && !unaffordable;

            let card = UiRect::new(left_x, route_y, left_w, route_h);
            if selectable && draw_glass_button(card, "", border, true) {
                return UiAction::SelectRoute(i);
            }
            if !selectable {
                draw_glass_button(card, "", colors::ACCENT_DANGER, false);
            }
            draw_rectangle(card.x, card.y, card.w, card.h, tint);
            draw_rectangle_lines(card.x, card.y, card.w, card.h, 1.0, border);

            // All five authored tiers. This topped out at "High" above
            // medium, so `HIGH_RISK` and `EXTREME_RISK` were constants the
            // player could feel but never read.
            let (risk_label, risk_color) = if !route_risk_known {
                ("Unknown", colors::TEXT_MUTED)
            } else if risk <= data.constants.risk.safe {
                ("Safe", colors::FUEL_GOOD)
            } else if risk <= data.constants.risk.low_risk {
                ("Low", colors::FUEL_GOOD)
            } else if risk <= data.constants.risk.medium_risk {
                ("Medium", colors::ACCENT_WARNING)
            } else if risk >= data.constants.risk.extreme_risk {
                ("Extreme", colors::FUEL_CRITICAL)
            } else {
                ("High", colors::FUEL_CRITICAL)
            };

            if is_blocked {
                // Say which route is shut.
                //
                // This branch drew the word BLOCKED and the hazard's reason and
                // nothing else -- no number, no route name -- so a night with two
                // roads closed showed two identical cards reading "BLOCKED" and
                // left the driver to work out which two. The key and the name
                // stay put, in the same places the open cards use, and BLOCKED
                // moves to where an open card carries its client reaction.
                draw_ui_text(
                    key,
                    card.x + 16.0,
                    card.y + 25.0,
                    fonts::SIZE_MD,
                    colors::TEXT_MUTED,
                );
                draw_small_caps(
                    name,
                    card.x + 54.0,
                    card.y + 25.0,
                    fonts::SIZE_MD,
                    colors::TEXT_MUTED,
                );
                draw_small_caps(
                    &data.localization.ui.game.driving.blocked,
                    card.x + 176.0,
                    card.y + 25.0,
                    fonts::SIZE_XS,
                    colors::FUEL_CRITICAL,
                );
                if let Some(hazard) = game_state
                    .environmental_hazards
                    .iter()
                    .find(|h| h.blocks_route(*route_type))
                {
                    draw_wrapped_text(
                        &hazard.description,
                        card.x + 54.0,
                        card.y + 45.0,
                        card.w - 90.0,
                        fonts::SIZE_XS,
                        16.0,
                        colors::ACCENT_WARNING,
                        2,
                    );
                }
            } else {
                draw_ui_text(
                    key,
                    card.x + 16.0,
                    card.y + 25.0,
                    fonts::SIZE_MD,
                    colors::TEXT_MUTED,
                );
                draw_small_caps(
                    name,
                    card.x + 54.0,
                    card.y + 25.0,
                    fonts::SIZE_MD,
                    route_title_color,
                );
                // The shortfall takes the slot the client reaction uses, since
                // a road the cab cannot reach matters more than how the fare
                // feels about it.
                match (&shortfall, &preference_label) {
                    (Some(reason), _) => draw_small_caps(
                        reason,
                        card.x + 176.0,
                        card.y + 25.0,
                        fonts::SIZE_XS,
                        colors::FUEL_CRITICAL,
                    ),
                    (None, Some((label, color))) => draw_small_caps(
                        label,
                        card.x + 176.0,
                        card.y + 25.0,
                        fonts::SIZE_XS,
                        *color,
                    ),
                    (None, None) => {}
                }
                draw_ui_text(
                    route_detail,
                    card.x + 54.0,
                    card.y + 45.0,
                    fonts::SIZE_XS,
                    colors::TEXT_SECONDARY,
                );
                draw_small_caps(
                    &format!("Known floor  {} min", time_cost),
                    card.x + card.w - cost_column_w,
                    card.y + 25.0,
                    fonts::SIZE_XS,
                    colors::TEXT_MUTED,
                );
                draw_small_caps(
                    &format!("Known floor  -{}%", fuel_cost),
                    card.x + card.w - cost_column_w,
                    card.y + 41.0,
                    fonts::SIZE_XS,
                    colors::TEXT_MUTED,
                );
                draw_small_caps(
                    &format!(
                        "{} hazards  {}",
                        if route_risk_known { "Known" } else { "Unknown" },
                        risk_label
                    ),
                    card.x + card.w - cost_column_w,
                    card.y + 57.0,
                    fonts::SIZE_XS,
                    risk_color,
                );
                draw_ui_icon(
                    UiIcon::Risk,
                    card.x + card.w - cost_column_w - 10.0,
                    card.y + 54.0,
                    12.0,
                    risk_color,
                );

                if !weather_warning.is_empty() {
                    draw_ui_text(
                        &weather_warning,
                        card.x + 54.0,
                        card.y + 58.0,
                        fonts::SIZE_XS,
                        pulsing_warning_color(),
                    );
                }
            }

            // No tags on a shut road. They are advice about a leg the driver
            // might choose, and three of them beside a closure is noise that
            // also has to be kept clear of the hazard's reason.
            if is_blocked {
                route_y += route_h + 8.0;
                continue;
            }

            use crate::engine::RouteService;
            let seed = game_state.rides_completed as u64
                + game_state
                    .current_passenger
                    .as_ref()
                    .map(|p| p.id as u64)
                    .unwrap_or(0)
                + i as u64;
            let risk_tags = RouteService::generate_risk_tags(
                *route_type,
                Some(&game_state.current_weather),
                Some(&game_state.time_of_day),
                game_state.current_passenger.as_ref(), // Pass passenger for context
                seed,
            );

            // The risk tags and the cost column used to start 244 and 154 from
            // the card's right edge, and a tag like "Road Construction: Detours
            // ahead." runs about 190 pixels -- so the two ran straight through
            // each other, and the most-used screen in the game read "Road
            // ConstructionDETOURS-A17 MIN". Truncating by character count could
            // not have saved it either: 28 characters is a different width for
            // every tag.
            //
            // The tags now have their own band ending short of the costs, and
            // are cut to that width rather than to a character count.
            let cost_x = card.x + card.w - cost_column_w;
            let tag_min_x = if narrow_cards { 285.0 } else { 250.0 };
            let tag_x = (card.x + card.w - tag_column_w).max(card.x + tag_min_x);
            let tag_w = (cost_x - tag_x - 20.0).max(60.0);
            let tag_y = card.y + 25.0;
            for (tag_idx, tag) in risk_tags.iter().enumerate() {
                let is_visible = tag_idx < visible_count;
                let (text, color) = if is_visible {
                    (
                        format!("{}: {}", tag.name(), tag.description()),
                        colors::ACCENT_WARNING,
                    )
                } else {
                    ("???".to_string(), colors::TEXT_MUTED)
                };
                let label =
                    macroquad_toolkit::ui::truncate_text_to_width(&text, tag_w, fonts::SIZE_XS);
                draw_ui_text(
                    &label,
                    tag_x,
                    tag_y + (tag_idx as f32 * 15.0),
                    fonts::SIZE_XS,
                    color,
                );
            }

            route_y += route_h + 8.0;
        }

        if let Some(passenger) = &game_state.current_passenger {
            let passenger_rect = UiRect::new(right_x, top, passenger_w, content_bottom - top);
            crate::ui::PassengerCard::draw(
                passenger,
                passenger_rect,
                game_state.current_passenger_dialogue.as_ref(),
            );
        }
    }

    UiAction::None
}

fn draw_narrow_driving(game_state: &GameState, game_data: Option<&GameData>) -> UiAction {
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
    let phase = match game_state.driving_phase {
        Some(DrivingPhase::Pickup) => &data.localization.ui.game.driving.pickup,
        Some(DrivingPhase::Destination) => &data.localization.ui.game.driving.destination,
        None => "DRIVING",
    };
    draw_small_caps(
        phase,
        panel.x + 12.0,
        panel.y + 18.0,
        fonts::SIZE_XS,
        colors::CAB_YELLOW,
    );
    let destination = game_state
        .current_ride
        .as_ref()
        .map(|ride| match game_state.driving_phase {
            Some(DrivingPhase::Pickup) => ride.pickup_location.as_str(),
            _ => ride.destination_location.as_str(),
        })
        .unwrap_or("the next road");
    draw_ui_text(
        &format!("TO {destination}"),
        panel.x + 12.0,
        panel.y + 42.0,
        fonts::SIZE_LG,
        colors::TEXT_PRIMARY,
    );
    draw_small_caps(
        &format!("FUEL {:.0}%  |  CHOOSE A ROUTE", game_state.fuel),
        panel.x + 12.0,
        panel.y + 60.0,
        fonts::SIZE_XS,
        colors::TEXT_MUTED,
    );

    let routes = [
        (RouteType::Normal, "NORMAL", colors::CAB_YELLOW),
        (RouteType::Shortcut, "SHORTCUT", colors::ACCENT_WARNING),
        (RouteType::Scenic, "SCENIC", colors::ACCENT_SKY),
        (RouteType::Police, "POLICE", colors::FUEL_GOOD),
    ];
    let gap = 5.0;
    let button_w = (screen_width() - 24.0 - gap) / 2.0;
    let button_h = 25.0;
    let top = screen_height() - button_h * 2.0 - gap - 10.0;
    for (index, (route, label, color)) in routes.into_iter().enumerate() {
        let blocked = game_state
            .environmental_hazards
            .iter()
            .any(|hazard| hazard.blocks_route(route));
        let rect = UiRect::new(
            12.0 + (index % 2) as f32 * (button_w + gap),
            top + (index / 2) as f32 * (button_h + gap),
            button_w,
            button_h,
        );
        if draw_glass_button(rect, label, color, !blocked) {
            return UiAction::SelectRoute(index);
        }
    }
    UiAction::None
}

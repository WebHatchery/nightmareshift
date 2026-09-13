//! The mid-ride event screen: event prompt and choice buttons.

use macroquad::prelude::*;

use crate::state::GameState;
use crate::ui::draw_ui_text;
use crate::ui::{
    colors, draw_cockpit_background, draw_glass_button, draw_glass_panel, draw_passenger_portrait,
    draw_small_caps, draw_wrapped_text, fonts, layout, spacing, UiAction, UiRect,
};

use super::scene::draw_bottom_taxi_scene;

/// Draw the interaction screen (Mid-Ride Event)
/// Takes no player stats and no game data: everything this screen needs to
/// know about what the player has unlocked was already decided by
/// `generate_mid_ride_event`, and asking again is how the two came to
/// disagree.
pub fn draw_interaction(game_state: &GameState) -> UiAction {
    draw_cockpit_background();

    if screen_width() < 980.0 || screen_height() < 560.0 {
        return draw_narrow_interaction(game_state);
    }

    let scene_h = (screen_height() * 0.27).clamp(210.0, 300.0);
    let scene_rect = UiRect::new(
        70.0,
        screen_height() - scene_h - 34.0,
        screen_width() - 140.0,
        scene_h,
    );
    draw_bottom_taxi_scene(scene_rect);

    if let Some(event) = &game_state.current_event {
        let rect_w = (screen_width() - 140.0).min(980.0);
        let rect_h = (scene_rect.y - layout::STATUS_BAR_HEIGHT - 76.0).clamp(430.0, 520.0);
        let rect = UiRect::centered_x(
            screen_width(),
            layout::STATUS_BAR_HEIGHT + 34.0,
            rect_w,
            rect_h,
        );
        draw_glass_panel(rect, colors::BORDER);

        let inner = rect.inset(spacing::PADDING_MD);
        let left_w = inner.w * 0.46;
        let right_x = inner.x + left_w + 28.0;
        let right_w = inner.w - left_w - 28.0;
        let mut y = inner.y + 16.0;

        draw_small_caps(
            "Mid-Ride Event",
            inner.x,
            y,
            fonts::SIZE_SM,
            colors::CAB_YELLOW,
        );
        y += 34.0;
        draw_ui_text(
            &event.title,
            inner.x,
            y,
            fonts::SIZE_XXL,
            colors::TEXT_PRIMARY,
        );
        y += 44.0;

        draw_wrapped_text(
            &event.description,
            inner.x,
            y,
            left_w,
            fonts::SIZE_MD,
            23.0,
            colors::TEXT_SECONDARY,
            8,
        );

        if let Some(passenger) = &game_state.current_passenger {
            let portrait_size = left_w.min(inner.h - 210.0).clamp(180.0, 260.0);
            let portrait_rect = UiRect::new(
                inner.x,
                inner.y + inner.h - portrait_size - 14.0,
                portrait_size,
                portrait_size,
            );
            draw_passenger_portrait(portrait_rect, passenger.id);
            draw_ui_text(
                &passenger.name,
                portrait_rect.x + portrait_rect.w + 18.0,
                portrait_rect.y + 42.0,
                fonts::SIZE_LG,
                colors::CAB_YELLOW,
            );
            let route_bottom = draw_wrapped_text(
                &format!("{} -> {}", passenger.pickup, passenger.destination),
                portrait_rect.x + portrait_rect.w + 18.0,
                portrait_rect.y + 72.0,
                left_w - portrait_size - 18.0,
                fonts::SIZE_SM,
                18.0,
                colors::TEXT_MUTED,
                3,
            );

            // And what they last said. The escalation line is written during
            // route resolution, which is the same step that can send the
            // player here, so this screen was covering it up.
            if let Some(spoken) = game_state.current_passenger_dialogue.as_ref() {
                draw_wrapped_text(
                    &format!("\"{spoken}\""),
                    portrait_rect.x + portrait_rect.w + 18.0,
                    route_bottom + 12.0,
                    left_w - portrait_size - 18.0,
                    fonts::SIZE_SM,
                    17.0,
                    colors::CAB_YELLOW,
                    3,
                );
            }
        }

        let mut choice_y = inner.y + 28.0;
        for (i, choice) in event.choices.iter().enumerate() {
            let btn_h = 78.0;
            let btn_rect = UiRect::new(right_x, choice_y, right_w, btn_h);
            // The ability choice only exists because the generator already
            // checked the passenger has this trait, the player has studied
            // them, and the matching skill is bought. Re-deriving that here
            // got it wrong: the screen asked for an unlocked *backstory*
            // while the generator asks for almanac knowledge, so a choice
            // could be offered with its explanation withheld.
            let hint_text = choice.required_trait.as_ref().and_then(|req_trait| {
                game_state
                    .current_passenger
                    .as_ref()
                    .filter(|passenger| passenger.traits.contains(req_trait))
                    .map(|passenger| format!("{}'s {} helps!", passenger.name, req_trait))
            });

            if draw_glass_button(btn_rect, "", colors::ACCENT_WARNING, true) {
                return UiAction::SelectEventChoice(i);
            }
            draw_small_caps(
                &format!("[{}]", i + 1),
                right_x + 16.0,
                choice_y + 28.0,
                fonts::SIZE_SM,
                colors::CAB_YELLOW,
            );
            draw_wrapped_text(
                &choice.description,
                right_x + 58.0,
                choice_y + 23.0,
                right_w - 78.0,
                fonts::SIZE_SM,
                18.0,
                colors::TEXT_PRIMARY,
                2,
            );
            // What kind of trouble this option courts.
            //
            // Every authored choice carries a `risk_type` — forty-eight of
            // them across sixteen events — and `RiskTag::name` and `description` were written
            // to say what they mean. Nothing ever called either, so a player picked
            // between three lines of prose with no idea which was the
            // spiritual gamble and which was merely a detour through
            // roadworks. The ability choice is exempt: it is not a gamble,
            // it is knowledge paying off, and it says so on this same row.
            let footnote = match &hint_text {
                Some(hint) => Some((hint.clone(), colors::FUEL_GOOD)),
                None if choice.required_trait.is_none() => Some((
                    format!(
                        "{} - {}",
                        choice.risk_type.name(),
                        choice.risk_type.description()
                    ),
                    colors::TEXT_MUTED,
                )),
                None => None,
            };
            if let Some((text, color)) = footnote {
                draw_ui_text(
                    &text,
                    right_x + 58.0,
                    choice_y + btn_h - 12.0,
                    fonts::SIZE_XS,
                    color,
                );
            }

            choice_y += btn_h + 14.0;
            if choice_y > rect.bottom() - 70.0 {
                break;
            }
        }
    } else if let Some(ref passenger) = game_state.current_passenger {
        // Fallback or legacy interaction (START of ride dialogue if any?)
        // Currently `Interaction` phase is reused for start pickup too?
        // No, Pickup phase transitions to Interaction.
        // My implementation in RideService sets current_event when transitioning to Interaction.
        // So `current_event` SHOULD be present.
        // But if I want to support legacy behavior just in case:
        let rect = UiRect::centered_x(screen_width(), 150.0, 500.0, 150.0);
        draw_glass_panel(rect, colors::BORDER);

        let inner = rect.inset(spacing::PADDING_MD);

        // Name
        draw_ui_text(
            &passenger.name,
            inner.x,
            inner.y + 24.0,
            fonts::SIZE_LG,
            colors::ACCENT_WARNING,
        );

        // Dialogue
        if let Some(ref dialogue) = game_state.current_passenger_dialogue {
            let preview = if dialogue.len() > 70 {
                format!("\"{}...\"", &dialogue[..70])
            } else {
                format!("\"{}\"", dialogue)
            };
            draw_ui_text(
                &preview,
                inner.x,
                inner.y + 60.0,
                fonts::SIZE_MD,
                colors::TEXT_PRIMARY,
            );
        }

        // Continue Button
        if draw_glass_button(
            UiRect::new(
                screen_width() / 2.0 - 100.0,
                rect.bottom() + 40.0,
                200.0,
                50.0,
            ),
            "Continue",
            colors::CAB_YELLOW,
            true,
        ) {
            return UiAction::Continue;
        }
    }
    UiAction::None
}

fn draw_narrow_interaction(game_state: &GameState) -> UiAction {
    let Some(event) = game_state.current_event.as_ref() else {
        return UiAction::Continue;
    };
    let panel = UiRect::new(
        12.0,
        layout::STATUS_BAR_HEIGHT + 8.0,
        screen_width() - 24.0,
        110.0,
    );
    draw_glass_panel(panel, colors::BORDER);
    draw_small_caps(
        "MID-RIDE EVENT",
        panel.x + 12.0,
        panel.y + 18.0,
        fonts::SIZE_XS,
        colors::CAB_YELLOW,
    );
    draw_ui_text(
        &event.title,
        panel.x + 12.0,
        panel.y + 40.0,
        fonts::SIZE_LG,
        colors::TEXT_PRIMARY,
    );
    draw_wrapped_text(
        &event.description,
        panel.x + 12.0,
        panel.y + 58.0,
        panel.w - 24.0,
        fonts::SIZE_XS,
        11.0,
        colors::TEXT_SECONDARY,
        2,
    );
    if event.choices.is_empty() {
        if draw_glass_button(
            UiRect::new(12.0, screen_height() - 34.0, screen_width() - 24.0, 26.0),
            "CONTINUE",
            colors::CAB_YELLOW,
            true,
        ) {
            return UiAction::Continue;
        }
        return UiAction::None;
    }

    let gap = 5.0;
    let button_w = (screen_width() - 24.0 - gap * (event.choices.len().min(3) - 1) as f32)
        / event.choices.len().min(3) as f32;
    for (index, choice) in event.choices.iter().take(3).enumerate() {
        let rect = UiRect::new(
            12.0 + index as f32 * (button_w + gap),
            screen_height() - 34.0,
            button_w,
            26.0,
        );
        if draw_glass_button(rect, "", colors::ACCENT_WARNING, true) {
            return UiAction::SelectEventChoice(index);
        }
        let label = macroquad_toolkit::ui::truncate_text_to_width(
            &choice.description,
            button_w - 10.0,
            fonts::SIZE_XS,
        );
        draw_small_caps(
            &format!("{} {}", index + 1, label),
            rect.x + 5.0,
            rect.y + 17.0,
            fonts::SIZE_XS,
            colors::TEXT_PRIMARY,
        );
    }
    UiAction::None
}

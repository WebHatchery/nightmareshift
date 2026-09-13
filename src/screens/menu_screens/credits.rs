//! Contributor and asset attribution shown from the main menu.

use macroquad::prelude::*;

use crate::ui::{
    colors, draw_glass_button, draw_glass_panel, draw_noir_city_background, draw_small_caps,
    draw_ui_text, draw_wrapped_text, fonts, UiAction, UiRect,
};

/// Draw the credits sequence and return to the menu when its visible button is pressed.
pub fn draw_credits() -> UiAction {
    draw_noir_city_background();

    let panel = UiRect::centered_x(
        screen_width(),
        screen_height() * 0.08,
        (screen_width() - 44.0).min(760.0),
        (screen_height() - 120.0).max(480.0),
    );
    draw_glass_panel(panel, colors::ACCENT_SKY);
    let inner = panel.inset(28.0);

    draw_ui_text(
        "CREDITS",
        inner.x,
        inner.y + 34.0,
        fonts::SIZE_XXL,
        colors::ACCENT_SKY,
    );
    draw_small_caps(
        "A night ride through a city that should be asleep",
        inner.x,
        inner.y + 62.0,
        fonts::SIZE_SM,
        colors::CAB_YELLOW,
    );

    let entries = [
        (
            "A WebHatchery original",
            "Design, programming, writing, and production",
        ),
        (
            "Built with Macroquad",
            "Rust game runtime and cross-platform rendering",
        ),
        (
            "Powered by macroquad-toolkit",
            "Shared input, UI, audio, persistence, and publishing support",
        ),
        (
            "Voices of the back seat",
            "Original passenger rules, tells, dialogue, and outcome text",
        ),
        (
            "The midnight city",
            "Original locations, portraits, driving scenes, interface art, and item illustrations",
        ),
        (
            "Sound in the cab",
            "Authored ambience and feedback cues bundled with the game",
        ),
    ];
    let mut y = inner.y + 104.0;
    for (title, body) in entries {
        draw_small_caps(title, inner.x, y, fonts::SIZE_SM, colors::ACCENT_GOLD);
        y = draw_wrapped_text(
            body,
            inner.x,
            y + 22.0,
            inner.w,
            fonts::SIZE_SM,
            19.0,
            colors::TEXT_SECONDARY,
            3,
        ) + 20.0;
    }

    let button = UiRect::new(inner.x, panel.bottom() - 66.0, inner.w.min(240.0), 44.0);
    if draw_glass_button(button, "Back to Menu (ESC)", colors::ACCENT_SKY, true) {
        UiAction::ReturnToMenu
    } else {
        UiAction::None
    }
}

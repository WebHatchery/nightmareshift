//! Always-available controls, tutorial reference, and accessibility options.

use crate::state::PlayerStats;
use crate::ui::{
    colors, draw_glass_button, draw_glass_panel, draw_noir_city_background, draw_small_caps,
    draw_ui_text, draw_wrapped_text, fonts, UiAction, UiRect,
};
use macroquad::prelude::*;

fn option_button(rect: UiRect, key: &str, label: &str, value: &str) -> bool {
    let text = format!("[{key}] {label}: {value}");
    draw_glass_button(rect, &text, colors::ACCENT_SKY, true)
}

/// Draw the shared help and settings surface. It is a real screen rather than
/// a game-only modal so the same controls are available before a run and from
/// the pause menu without maintaining two copies.
pub fn draw_help_options(stats: &PlayerStats, tutorial_active: bool) -> UiAction {
    draw_noir_city_background();
    let margin = (screen_width() * 0.04).clamp(22.0, 64.0);
    let top = 34.0;
    let bottom = 68.0;
    let gap = 18.0;
    let content_h = screen_height() - top - bottom;
    // Two full-height panels remain more legible than stacking at desktop
    // widths: the handbook's five sections cannot fit in half a 720px-tall
    // viewport once 125% text is enabled. True mobile is not a supported
    // target; 900px is the narrow browser tier validated by captures.
    let wide = screen_width() >= 800.0;
    let help_w = if wide {
        (screen_width() - margin * 2.0 - gap) * 0.47
    } else {
        screen_width() - margin * 2.0
    };
    let option_x = if wide { margin + help_w + gap } else { margin };
    let option_y = if wide {
        top
    } else {
        top + content_h * 0.49 + gap
    };
    let option_h = if wide {
        content_h
    } else {
        content_h * 0.51 - gap
    };

    let help = UiRect::new(
        margin,
        top,
        help_w,
        if wide { content_h } else { content_h * 0.49 },
    );
    draw_glass_panel(help, colors::CAB_YELLOW);
    let inner = help.inset(20.0);
    let heading_size = if help.w < 500.0 {
        28.0
    } else {
        fonts::SIZE_XXL
    };
    draw_ui_text(
        if tutorial_active {
            "FIRST SHIFT TUTORIAL"
        } else {
            "DRIVER'S HANDBOOK"
        },
        inner.x,
        inner.y + 32.0,
        heading_size,
        colors::CAB_YELLOW,
    );
    draw_small_caps(
        "Everything required to finish a first fare",
        inner.x,
        inner.y + 58.0,
        fonts::SIZE_SM,
        colors::TEXT_SECONDARY,
    );

    let sections = [
        ("1  Read the request", "Pickup, destination and the fare range are known. Passenger facts are marked Known, Inferred or Unknown."),
        ("2  Compare the roads", "Normal steadies need and saves fuel. Shortcut saves time and fuel at high risk. Scenic pays more but runs long. Police suppresses supernatural risk for a larger fuel bill."),
        ("3  Watch the passenger", "Observed files reveal need thresholds. Tells point toward exceptions; Rules explains the standing instructions. A justified break is not the same as a violation."),
        ("4  Use the cab", "E eye contact  M music  W window  Y wipers  H lights off  A climate  S stop. Open Rules (R) for touch/mouse buttons."),
        ("5  Finish the night", "Drop-off itemizes the fare and consequences. Refuelling spends earnings, so bank the quota before dawn when one more fare is unsafe."),
    ];
    let dense_help = stats.accessibility.text_scale_percent > 100 && help.w < 500.0;
    let section_gap = if dense_help { 5.0 } else { 14.0 };
    let body_line_height = if dense_help { 15.0 } else { 16.0 };
    let mut y = inner.y + 92.0;
    for (title, body) in sections {
        draw_small_caps(title, inner.x, y, fonts::SIZE_SM, colors::ACCENT_GOLD);
        y = draw_wrapped_text(
            body,
            inner.x,
            y + if dense_help { 19.0 } else { 22.0 },
            inner.w,
            fonts::SIZE_XS,
            body_line_height,
            colors::TEXT_SECONDARY,
            if help.w < 500.0 { 4 } else { 3 },
        ) + section_gap;
    }

    let options = UiRect::new(
        option_x,
        option_y,
        screen_width() - option_x - margin,
        option_h,
    );
    draw_glass_panel(options, colors::ACCENT_SKY);
    let oi = options.inset(20.0);
    draw_ui_text(
        "OPTIONS",
        oi.x,
        oi.y + 32.0,
        heading_size,
        colors::ACCENT_SKY,
    );
    draw_small_caps(
        "All settings save immediately",
        oi.x,
        oi.y + 58.0,
        fonts::SIZE_SM,
        colors::TEXT_SECONDARY,
    );

    let settings = &stats.accessibility;
    let rows = [
        (
            "T",
            "Text scale",
            format!("{}%", settings.text_scale_percent),
            UiAction::CycleTextScale,
        ),
        (
            "H",
            "High contrast",
            if settings.high_contrast { "On" } else { "Off" }.to_string(),
            UiAction::ToggleHighContrast,
        ),
        (
            "R",
            "Reduced motion",
            if settings.reduced_motion { "On" } else { "Off" }.to_string(),
            UiAction::ToggleReducedMotion,
        ),
        (
            "B",
            "Brightness",
            format!("{}%", settings.brightness_percent),
            UiAction::CycleBrightness,
        ),
        (
            "C",
            "Captions",
            if settings.captions { "On" } else { "Off" }.to_string(),
            UiAction::ToggleCaptions,
        ),
        (
            "F",
            "Fullscreen",
            if settings.fullscreen { "On" } else { "Off" }.to_string(),
            UiAction::ToggleFullscreen,
        ),
        (
            "1",
            "Master",
            format!("{}%", settings.master_volume),
            UiAction::CycleMasterVolume,
        ),
        (
            "2",
            "Ambience",
            format!("{}%", settings.ambience_volume),
            UiAction::CycleAmbienceVolume,
        ),
        (
            "3",
            "Music",
            format!("{}%", settings.music_volume),
            UiAction::CycleMusicVolume,
        ),
        (
            "4",
            "Effects",
            format!("{}%", settings.effects_volume),
            UiAction::CycleEffectsVolume,
        ),
        (
            "A",
            "Accept shortcut",
            settings.key_bindings.accept.clone(),
            UiAction::CycleAcceptBinding,
        ),
        (
            "D",
            "Decline shortcut",
            settings.key_bindings.decline.clone(),
            UiAction::CycleDeclineBinding,
        ),
        (
            "F",
            "Follow shortcut",
            settings.key_bindings.follow.clone(),
            UiAction::CycleFollowBinding,
        ),
        (
            "B",
            "Break shortcut",
            settings.key_bindings.break_guideline.clone(),
            UiAction::CycleBreakBinding,
        ),
        (
            "P",
            "Pause shortcut",
            settings.key_bindings.pause.clone(),
            UiAction::CyclePauseBinding,
        ),
        (
            "L",
            "Language",
            if settings.language.eq_ignore_ascii_case("es") {
                "ES".to_string()
            } else {
                "EN".to_string()
            },
            UiAction::CycleLanguage,
        ),
        (
            "X",
            "Export backup",
            "Keep a restore copy".to_string(),
            UiAction::ExportSave,
        ),
        (
            "M",
            "Import backup",
            "Restore exported copy".to_string(),
            UiAction::ImportSave,
        ),
    ];
    let columns = if options.w >= 620.0 { 2 } else { 1 };
    let row_gap = 10.0;
    let col_gap = 12.0;
    let button_w = (oi.w - col_gap * (columns - 1) as f32) / columns as f32;
    let button_h =
        ((oi.h - 118.0) / rows.len().div_ceil(columns) as f32 - row_gap).clamp(34.0, 46.0);
    for (idx, (key, label, value, action)) in rows.iter().enumerate() {
        let col = idx % columns;
        let row = idx / columns;
        let rect = UiRect::new(
            oi.x + col as f32 * (button_w + col_gap),
            oi.y + 82.0 + row as f32 * (button_h + row_gap),
            button_w,
            button_h,
        );
        if option_button(rect, key, label, value) {
            return action.clone();
        }
    }

    let conflicts = settings.key_bindings.conflicts();
    if !conflicts.is_empty() {
        draw_small_caps(
            &format!("Binding conflict: {}", conflicts.join(", ")),
            oi.x,
            options.bottom() - 18.0,
            fonts::SIZE_XS,
            colors::FUEL_CRITICAL,
        );
    }

    if draw_glass_button(
        UiRect::new(
            screen_width() / 2.0 - 130.0,
            screen_height() - 54.0,
            260.0,
            40.0,
        ),
        if tutorial_active {
            "Begin Briefing (ESC)"
        } else {
            "Back (ESC)"
        },
        colors::CAB_YELLOW,
        true,
    ) {
        return UiAction::ReturnToMenu;
    }
    UiAction::None
}

//! The main menu: driver record panel and the primary command list.

use macroquad::prelude::*;

use crate::data::GameData;
use crate::state::{Persistence, PlayerStats};
use crate::ui::draw_ui_text;
use crate::ui::{
    colors, draw_glass_panel, draw_modal_scrim, draw_small_caps, draw_title_background,
    draw_ui_logo, draw_wrapped_text, fonts, UiAction, UiRect,
};

use super::widgets::draw_menu_command;

/// Draw the main menu.
///
/// `seed_entry` is the seed modal's in-progress digits, owned by `Game` so
/// the text survives across frames; `Some` means the modal is open and this
/// screen is consuming the keyboard. Keyboard input is dispatched by `Game`
/// before drawing, leaving this renderer read-only.
pub fn draw_main_menu(
    player_stats: &PlayerStats,
    game_data: Option<&GameData>,
    delete_armed: bool,
    save_notice: Option<&str>,
    resume_available: bool,
    daily_seed: u64,
    seed_entry: Option<&str>,
) -> UiAction {
    draw_title_background();

    if screen_width() < 980.0 || screen_height() < 560.0 {
        return draw_narrow_main_menu(delete_armed, save_notice, resume_available, seed_entry);
    }

    // Default strings if data missing (shouldn't happen)
    let title_text = if let Some(d) = game_data {
        &d.localization.ui.main_menu.title
    } else {
        "NIGHTMARE SHIFT"
    };
    let subtitle_text = if let Some(d) = game_data {
        &d.localization.ui.main_menu.subtitle
    } else {
        "Survive the night."
    };

    let menu_scale = (screen_width() / 1920.0)
        .min(screen_height() / 1080.0)
        .clamp(0.45, 1.0);
    let title_x = (70.0 * menu_scale).clamp(30.0, 70.0);
    let title_size = (72.0 * menu_scale).clamp(32.0, 72.0);
    let title_gap = title_size * 0.92;
    let mut title_y = (112.0 * menu_scale).clamp(50.0, 122.0);
    if screen_width() >= 700.0 {
        let logo_w = (screen_width() * 0.30).clamp(260.0, 480.0);
        draw_ui_logo(Rect::new(title_x, title_y - 48.0, logo_w, logo_w * 0.43));
        title_y += logo_w * 0.43;
    } else {
        for line in title_text.split_whitespace() {
            draw_ui_text(line, title_x, title_y, title_size, colors::TEXT_PRIMARY);
            title_y += title_gap;
        }
    }
    draw_small_caps(
        subtitle_text,
        title_x,
        title_y + 12.0,
        fonts::SIZE_MD,
        colors::CAB_YELLOW,
    );

    // The seed-entry modal owns the frame while it is open: a dimmed
    // backdrop replaces the command list (so nothing is clicked through
    // it) and the keyboard types the seed — digits build it, Enter deals
    // that night, Escape walks away. The Space shortcut to Start is
    // suppressed in the dispatcher while this is `Some`.
    if let Some(digits) = seed_entry {
        draw_modal_scrim();
        let panel = UiRect::centered_x(screen_width(), screen_height() * 0.32, 460.0_f32, 190.0);
        draw_glass_panel(panel, colors::ACCENT_SKY);
        let inner = panel.inset(20.0);
        draw_small_caps(
            "Seeded Run",
            inner.x,
            inner.y + 14.0,
            fonts::SIZE_SM,
            colors::ACCENT_SKY,
        );
        draw_ui_text(
            "Type a night number. The same number deals the same night.",
            inner.x,
            inner.y + 44.0,
            fonts::SIZE_XS,
            colors::TEXT_SECONDARY,
        );

        let shown = if digits.is_empty() {
            "_".to_string()
        } else {
            digits.to_string()
        };
        draw_ui_text(
            &shown,
            inner.x,
            inner.y + 84.0,
            fonts::SIZE_XL,
            colors::TEXT_PRIMARY,
        );
        draw_small_caps(
            "Type digits or tap the controls below",
            inner.x,
            inner.y + 112.0,
            fonts::SIZE_XS,
            colors::TEXT_MUTED,
        );
        let button_y = inner.y + 132.0;
        let button_gap = 10.0;
        let button_w = (inner.w - button_gap * 2.0) / 3.0;
        let buttons = [
            ("ERASE", UiAction::EraseSeedDigit, colors::ACCENT_WARNING),
            (
                "START",
                digits
                    .parse::<u64>()
                    .ok()
                    .map_or(UiAction::None, UiAction::StartSeededRun),
                colors::FUEL_GOOD,
            ),
            ("CANCEL", UiAction::CancelSeedEntry, colors::ACCENT_DANGER),
        ];
        for (idx, (label, action, color)) in buttons.into_iter().enumerate() {
            let rect = UiRect::new(
                inner.x + idx as f32 * (button_w + button_gap),
                button_y,
                button_w,
                32.0,
            );
            if crate::ui::draw_glass_button(rect, label, color, action != UiAction::None) {
                return action;
            }
        }
        return UiAction::None;
    }

    if let Some(data) = game_data {
        let stats = data
            .localization
            .ui
            .main_menu
            .stats
            .replacen("{}", &player_stats.total_shifts_completed.to_string(), 1)
            .replacen("{}", &player_stats.total_earnings.to_string(), 1)
            .replacen("{}", &player_stats.total_rides_completed.to_string(), 1)
            .replacen("{}", &player_stats.total_rules_violated.to_string(), 1);

        // Play time was accumulated and saved from the first release and
        // never shown anywhere until this line.
        let progression = format!(
            "Experience Lv. {} | Suggested Difficulty {} | {}h {:02}m behind the wheel",
            player_stats.experience_level(),
            player_stats.suggested_difficulty() + 1,
            player_stats.total_play_time / 60,
            player_stats.total_play_time % 60
        );
        let unlocked_count = player_stats
            .achievements
            .iter()
            .filter(|a| a.unlocked)
            .count();
        let total_count = player_stats.achievements.len();
        let achievements_text = data
            .localization
            .ui
            .main_menu
            .achievements
            .replacen("{}", &unlocked_count.to_string(), 1)
            .replacen("{}", &total_count.to_string(), 1);

        if screen_width() >= 980.0 && screen_height() >= 560.0 {
            let stats_rect = UiRect::new((screen_width() - 306.0).max(470.0), 88.0, 260.0, 132.0);
            draw_glass_panel(stats_rect, colors::BORDER_DIM);
            let stats_inner = stats_rect.inset(14.0);
            draw_small_caps(
                "Driver Record",
                stats_inner.x,
                stats_inner.y + 12.0,
                fonts::SIZE_SM,
                colors::CAB_YELLOW,
            );
            draw_wrapped_text(
                &stats,
                stats_inner.x,
                stats_inner.y + 38.0,
                stats_inner.w,
                fonts::SIZE_XS,
                16.0,
                colors::TEXT_SECONDARY,
                2,
            );
            draw_wrapped_text(
                &progression,
                stats_inner.x,
                stats_inner.y + 74.0,
                stats_inner.w,
                fonts::SIZE_XS,
                16.0,
                colors::TEXT_MUTED,
                2,
            );
            draw_ui_text(
                &achievements_text,
                stats_inner.x,
                stats_inner.y + 112.0,
                fonts::SIZE_XS,
                colors::ACCENT_GOLD,
            );
        }

        let menu_w = (screen_width() * 0.36).clamp(230.0, 380.0);
        let menu_h = (62.0 * menu_scale).clamp(38.0, 62.0);
        let gap = (12.0 * menu_scale).clamp(6.0, 12.0);
        let menu_items = 8.0
            + if Persistence::save_exists() { 1.0 } else { 0.0 }
            + if resume_available { 1.0 } else { 0.0 };
        let total_menu_h = menu_h * menu_items + gap * (menu_items - 1.0);
        let right_margin = 46.0 * menu_scale;
        let taxi_right = screen_width() * 0.36;
        let taxi_clear_x = taxi_right + (28.0 * menu_scale).clamp(14.0, 28.0);
        let max_menu_x = screen_width() - menu_w - right_margin;
        let base_menu_x = (screen_width() * 0.16)
            .max(title_x + 185.0 * menu_scale)
            .min(max_menu_x);
        let menu_x = if taxi_clear_x <= max_menu_x {
            base_menu_x.max(taxi_clear_x)
        } else {
            base_menu_x
        };
        let min_menu_y = title_y + 86.0 * menu_scale;
        let max_menu_y = (screen_height() - total_menu_h - 54.0 * menu_scale).max(min_menu_y);
        let menu_y = (screen_height() * 0.38).max(min_menu_y).min(max_menu_y);

        let start_label = data
            .localization
            .ui
            .main_menu
            .start_space
            .replace("(SPACE)", "");
        if draw_menu_command(
            UiRect::new(menu_x, menu_y, menu_w, menu_h),
            "wheel",
            start_label.trim(),
            "SPACE",
            colors::CAB_YELLOW,
            menu_scale,
        ) {
            return UiAction::StartGame;
        }

        // The determinism seam, opened to the player: one shared night a
        // day, or any night by number. Both re-arm the run stream the same
        // way `--seed` does, and the briefing badge names the seed.
        if draw_menu_command(
            UiRect::new(
                menu_x,
                menu_y + (menu_h + gap) * if resume_available { 2.0 } else { 1.0 },
                menu_w,
                menu_h,
            ),
            "wheel",
            "Daily Shift",
            &format!("Night #{daily_seed} - dealt to everyone"),
            colors::ACCENT_SKY,
            menu_scale,
        ) {
            return UiAction::StartDailyRun;
        }
        if draw_menu_command(
            UiRect::new(
                menu_x,
                menu_y + (menu_h + gap) * if resume_available { 3.0 } else { 2.0 },
                menu_w,
                menu_h,
            ),
            "wheel",
            "Seeded Run",
            "Replay a night by number",
            colors::TEXT_SECONDARY,
            menu_scale,
        ) {
            return UiAction::OpenSeedEntry;
        }

        let menu_offset = if resume_available { 1.0 } else { 0.0 };
        if resume_available
            && draw_menu_command(
                UiRect::new(menu_x, menu_y + menu_h + gap, menu_w, menu_h),
                "wheel",
                "Resume Run",
                "Continue the saved night",
                colors::FUEL_GOOD,
                menu_scale,
            )
        {
            return UiAction::ResumeRun;
        }

        let skill_btn_text = data
            .localization
            .ui
            .meta
            .skill_tree
            .button
            .replace("{}", &player_stats.bank_balance.to_string());
        let skill_detail = skill_btn_text
            .split_once('(')
            .map(|(_, detail)| detail.trim_end_matches(')').to_string())
            .unwrap_or_else(|| format!("${} Available", player_stats.bank_balance));
        if draw_menu_command(
            UiRect::new(
                menu_x,
                menu_y + (menu_h + gap) * (3.0 + menu_offset),
                menu_w,
                menu_h,
            ),
            "tree",
            "Skill Tree",
            &skill_detail,
            colors::TEXT_SECONDARY,
            menu_scale,
        ) {
            return UiAction::OpenSkillTree;
        }

        let almanac_btn_text = data
            .localization
            .ui
            .meta
            .almanac
            .button
            .replace("{}", &player_stats.lore_fragments.to_string());
        let almanac_detail = almanac_btn_text
            .split_once('(')
            .map(|(_, detail)| detail.trim_end_matches(')').to_string())
            .unwrap_or_else(|| format!("{} Lore Fragments", player_stats.lore_fragments));
        if draw_menu_command(
            UiRect::new(
                menu_x,
                menu_y + (menu_h + gap) * (4.0 + menu_offset),
                menu_w,
                menu_h,
            ),
            "book",
            "Almanac",
            &almanac_detail,
            colors::TEXT_SECONDARY,
            menu_scale,
        ) {
            return UiAction::OpenAlmanac;
        }

        let leaderboard_btn_text = data
            .localization
            .ui
            .meta
            .leaderboard
            .button
            .chars()
            .filter(|ch| ch.is_ascii())
            .collect::<String>()
            .trim()
            .to_string();
        if draw_menu_command(
            UiRect::new(
                menu_x,
                menu_y + (menu_h + gap) * (5.0 + menu_offset),
                menu_w,
                menu_h,
            ),
            "trophy",
            &leaderboard_btn_text,
            "Best Runs",
            colors::TEXT_SECONDARY,
            menu_scale,
        ) {
            return UiAction::OpenLeaderboard;
        }

        if draw_menu_command(
            UiRect::new(
                menu_x,
                menu_y + (menu_h + gap) * (6.0 + menu_offset),
                menu_w,
                menu_h,
            ),
            "book",
            "Help & Options",
            "Controls, tutorial, accessibility",
            colors::ACCENT_SKY,
            menu_scale,
        ) {
            return UiAction::OpenHelpOptions;
        }

        if draw_menu_command(
            UiRect::new(
                menu_x,
                menu_y + (menu_h + gap) * (7.0 + menu_offset),
                menu_w,
                menu_h,
            ),
            "book",
            "Credits",
            "People, tools, and the midnight city",
            colors::TEXT_SECONDARY,
            menu_scale,
        ) {
            return UiAction::OpenCredits;
        }

        // Deleting a save was one click from the menu, and it takes the bank
        // balance, every lore fragment, every almanac level, every unlocked
        // skill, the leaderboard and the achievements with it. The first
        // click now only arms the button; `Game` holds the arming and expires
        // it, so a mis-click resolves itself by being left alone.
        if Persistence::save_exists() {
            let (label, detail, colour) = if delete_armed {
                (
                    "Confirm Delete",
                    "Erases skills, almanac and bank",
                    colors::FUEL_CRITICAL,
                )
            } else {
                ("Delete Save", "Reset Progress", colors::ACCENT_DANGER)
            };
            if draw_menu_command(
                UiRect::new(
                    menu_x,
                    menu_y + (menu_h + gap) * (8.0 + menu_offset),
                    menu_w,
                    menu_h,
                ),
                "delete",
                label,
                detail,
                colour,
                menu_scale,
            ) {
                return UiAction::DeleteSave;
            }
        }

        // Degraded-content warnings and the save-quarantine notice: a
        // content file that failed to parse fell back empty, and an
        // unreadable save was set aside — until these lines the only
        // witness was stderr, which the web build has no way to show.
        let warnings = save_notice
            .into_iter()
            .chain(data.load_errors.iter().map(String::as_str));
        for (idx, warning) in warnings.enumerate() {
            draw_ui_text(
                warning,
                title_x,
                screen_height() - 38.0 - idx as f32 * 18.0,
                fonts::SIZE_XS,
                colors::FUEL_CRITICAL,
            );
        }

        // Locale and data version, somewhere a player can actually read
        // them — the loading screen shows for two frames.
        let meta_text = format!(
            "{} {} v{}",
            data.localization.meta.language,
            data.localization.meta.code,
            data.localization.meta.version
        );
        draw_ui_text(
            &meta_text,
            title_x,
            screen_height() - 18.0,
            fonts::SIZE_XS,
            colors::TEXT_MUTED,
        );
    }

    UiAction::None
}

fn draw_narrow_main_menu(
    delete_armed: bool,
    save_notice: Option<&str>,
    resume_available: bool,
    seed_entry: Option<&str>,
) -> UiAction {
    let title_color = colors::TEXT_PRIMARY;
    draw_ui_text("NIGHTMARE", 16.0, 28.0, 22.0, title_color);
    draw_ui_text("SHIFT", 16.0, 50.0, 22.0, title_color);
    draw_small_caps(
        "SURVIVE THE NIGHT",
        16.0,
        66.0,
        fonts::SIZE_XS,
        colors::CAB_YELLOW,
    );

    if let Some(digits) = seed_entry {
        return draw_narrow_seed_entry(digits);
    }

    let mut commands = vec![
        ("START", UiAction::StartGame, colors::CAB_YELLOW),
        ("DAILY", UiAction::StartDailyRun, colors::ACCENT_SKY),
        ("SEEDED", UiAction::OpenSeedEntry, colors::TEXT_SECONDARY),
    ];
    if resume_available {
        commands.insert(1, ("RESUME", UiAction::ResumeRun, colors::FUEL_GOOD));
    }
    commands.extend([
        ("SKILLS", UiAction::OpenSkillTree, colors::TEXT_SECONDARY),
        ("ALMANAC", UiAction::OpenAlmanac, colors::TEXT_SECONDARY),
        ("SCORES", UiAction::OpenLeaderboard, colors::TEXT_SECONDARY),
        ("HELP", UiAction::OpenHelpOptions, colors::ACCENT_SKY),
        ("CREDITS", UiAction::OpenCredits, colors::TEXT_SECONDARY),
    ]);
    if Persistence::save_exists() {
        let label = if delete_armed { "CONFIRM" } else { "DELETE" };
        commands.push((label, UiAction::DeleteSave, colors::ACCENT_DANGER));
    }

    let columns = 3;
    let gap = 4.0;
    let top = 72.0;
    let button_w =
        ((screen_width() - 32.0 - gap * (columns - 1) as f32) / columns as f32).max(70.0);
    let rows = commands.len().div_ceil(columns);
    let button_h =
        ((screen_height() - top - 12.0 - gap * (rows - 1) as f32) / rows as f32).clamp(22.0, 30.0);
    let total_w = button_w * columns as f32 + gap * (columns - 1) as f32;
    let start_x = (screen_width() - total_w) / 2.0;
    for (index, (label, action, accent)) in commands.into_iter().enumerate() {
        let column = index % columns;
        let row = index / columns;
        let rect = UiRect::new(
            start_x + column as f32 * (button_w + gap),
            top + row as f32 * (button_h + gap),
            button_w,
            button_h,
        );
        if draw_compact_menu_command(rect, label, accent) {
            return action;
        }
    }

    if let Some(notice) = save_notice {
        draw_small_caps(
            notice,
            16.0,
            screen_height() - 4.0,
            fonts::SIZE_XS,
            colors::FUEL_CRITICAL,
        );
    }
    UiAction::None
}

fn draw_compact_menu_command(rect: UiRect, label: &str, accent: Color) -> bool {
    let clicked = crate::ui::draw_glass_button(rect, "", accent, true);
    let dims = crate::ui::measure_ui_text(label, None, fonts::SIZE_XS as u16, 1.0);
    draw_small_caps(
        label,
        rect.x + (rect.w - dims.width) / 2.0,
        rect.y + rect.h * 0.64,
        fonts::SIZE_XS,
        colors::TEXT_PRIMARY,
    );
    clicked
}

fn draw_narrow_seed_entry(digits: &str) -> UiAction {
    draw_modal_scrim();
    let panel = UiRect::new(
        12.0,
        12.0,
        (screen_width() - 24.0).max(260.0),
        (screen_height() - 24.0).max(140.0),
    );
    draw_glass_panel(panel, colors::ACCENT_SKY);
    let inner = panel.inset(14.0);
    draw_small_caps(
        "SEEDED RUN",
        inner.x,
        inner.y + 12.0,
        fonts::SIZE_SM,
        colors::ACCENT_SKY,
    );
    draw_wrapped_text(
        "Type a night number. The same number deals the same night.",
        inner.x,
        inner.y + 34.0,
        inner.w,
        fonts::SIZE_XS,
        12.0,
        colors::TEXT_SECONDARY,
        2,
    );
    draw_ui_text(
        if digits.is_empty() { "_" } else { digits },
        inner.x,
        inner.y + 62.0,
        fonts::SIZE_XL,
        colors::TEXT_PRIMARY,
    );

    let button_gap = 6.0;
    let button_w = (inner.w - button_gap * 2.0) / 3.0;
    let digit_gap = 3.0;
    let digit_w = (inner.w - digit_gap * 4.0) / 5.0;
    for (index, digit) in "1234567890".chars().enumerate() {
        let rect = UiRect::new(
            inner.x + (index % 5) as f32 * (digit_w + digit_gap),
            inner.y + 72.0 + (index / 5) as f32 * 23.0,
            digit_w,
            20.0,
        );
        if crate::ui::draw_glass_button(
            rect,
            &digit.to_string(),
            colors::ACCENT_SKY,
            digits.len() < 19,
        ) {
            return UiAction::SeedDigit(digit);
        }
    }

    let button_y = panel.bottom() - 40.0;
    let buttons = [
        ("ERASE", UiAction::EraseSeedDigit, colors::ACCENT_WARNING),
        (
            "START",
            digits
                .parse::<u64>()
                .ok()
                .map_or(UiAction::None, UiAction::StartSeededRun),
            colors::FUEL_GOOD,
        ),
        ("CANCEL", UiAction::CancelSeedEntry, colors::ACCENT_DANGER),
    ];
    for (index, (label, action, color)) in buttons.into_iter().enumerate() {
        let rect = UiRect::new(
            inner.x + index as f32 * (button_w + button_gap),
            button_y,
            button_w,
            28.0,
        );
        if crate::ui::draw_glass_button(rect, label, color, action != UiAction::None) {
            return action;
        }
    }
    UiAction::None
}

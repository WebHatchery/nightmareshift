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

struct DesktopMenuLayout {
    scale: f32,
    menu_x: f32,
    menu_y: f32,
    menu_w: f32,
    menu_h: f32,
    gap: f32,
    resume_offset: f32,
}

impl DesktopMenuLayout {
    fn new(scale: f32, title_x: f32, title_y: f32, resume: bool, has_save: bool) -> Self {
        let menu_w = (screen_width() * 0.36).clamp(230.0, 380.0);
        let menu_h = (62.0 * scale).clamp(38.0, 62.0);
        let gap = (12.0 * scale).clamp(6.0, 12.0);
        let menu_items = 8.0 + f32::from(has_save) + f32::from(resume);
        let total_menu_h = menu_h * menu_items + gap * (menu_items - 1.0);
        let right_margin = 46.0 * scale;
        let taxi_clear_x = screen_width() * 0.36 + (28.0 * scale).clamp(14.0, 28.0);
        let max_menu_x = screen_width() - menu_w - right_margin;
        let base_menu_x = (screen_width() * 0.16)
            .max(title_x + 185.0 * scale)
            .min(max_menu_x);
        let menu_x = if taxi_clear_x <= max_menu_x {
            base_menu_x.max(taxi_clear_x)
        } else {
            base_menu_x
        };
        let min_menu_y = title_y + 86.0 * scale;
        let max_menu_y = (screen_height() - total_menu_h - 54.0 * scale).max(min_menu_y);
        let menu_y = (screen_height() * 0.38).max(min_menu_y).min(max_menu_y);

        Self {
            scale,
            menu_x,
            menu_y,
            menu_w,
            menu_h,
            gap,
            resume_offset: f32::from(resume),
        }
    }

    fn rect(&self, index: f32) -> UiRect {
        UiRect::new(
            self.menu_x,
            self.menu_y + (self.menu_h + self.gap) * index,
            self.menu_w,
            self.menu_h,
        )
    }
}

/// Draw the main menu.
///
/// `seed_entry` is the seed modal's in-progress digits, owned by `Game` so
/// the text survives across frames. Keyboard input is dispatched by `Game`
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
        return draw_narrow_main_menu(
            game_data,
            delete_armed,
            save_notice,
            resume_available,
            seed_entry,
        );
    }

    let (scale, title_x, title_y) = draw_desktop_branding(game_data);
    if let Some(digits) = seed_entry {
        return draw_desktop_seed_entry(digits);
    }
    let Some(data) = game_data else {
        return UiAction::None;
    };
    let has_save = Persistence::save_exists();
    let layout = DesktopMenuLayout::new(scale, title_x, title_y, resume_available, has_save);
    draw_driver_record(data, player_stats);
    if let Some(action) = draw_primary_commands(&layout, data, resume_available, daily_seed) {
        return action;
    }
    if let Some(action) = draw_progression_commands(&layout, data, player_stats) {
        return action;
    }
    if let Some(action) = draw_info_commands(&layout, data) {
        return action;
    }
    if let Some(action) = draw_delete_command(&layout, delete_armed, has_save) {
        return action;
    }
    draw_desktop_footer(data, title_x, save_notice);
    UiAction::None
}

fn draw_desktop_branding(game_data: Option<&GameData>) -> (f32, f32, f32) {
    let title = game_data
        .map(|data| data.localization.ui.main_menu.title.as_str())
        .unwrap_or("NIGHTMARE SHIFT");
    let subtitle = game_data
        .map(|data| data.localization.ui.main_menu.subtitle.as_str())
        .unwrap_or("Survive the night.");
    let scale = (screen_width() / 1920.0)
        .min(screen_height() / 1080.0)
        .clamp(0.45, 1.0);
    let title_x = (70.0 * scale).clamp(30.0, 70.0);
    let title_size = (72.0 * scale).clamp(32.0, 72.0);
    let title_gap = title_size * 0.92;
    let mut title_y = (112.0 * scale).clamp(50.0, 122.0);
    if screen_width() >= 700.0 {
        let logo_w = (screen_width() * 0.30).clamp(260.0, 480.0);
        draw_ui_logo(Rect::new(title_x, title_y - 48.0, logo_w, logo_w * 0.43));
        title_y += logo_w * 0.43;
    } else {
        for line in title.split_whitespace() {
            draw_ui_text(line, title_x, title_y, title_size, colors::TEXT_PRIMARY);
            title_y += title_gap;
        }
    }
    draw_small_caps(
        subtitle,
        title_x,
        title_y + 12.0,
        fonts::SIZE_MD,
        colors::CAB_YELLOW,
    );
    (scale, title_x, title_y)
}

fn draw_desktop_seed_entry(digits: &str) -> UiAction {
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
    draw_ui_text(
        if digits.is_empty() { "_" } else { digits },
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
    for (index, (label, action, color)) in buttons.into_iter().enumerate() {
        let rect = UiRect::new(
            inner.x + index as f32 * (button_w + button_gap),
            inner.y + 132.0,
            button_w,
            32.0,
        );
        if crate::ui::draw_glass_button(rect, label, color, action != UiAction::None) {
            return action;
        }
    }
    UiAction::None
}

fn draw_driver_record(data: &GameData, player_stats: &PlayerStats) {
    let stats = data
        .localization
        .ui
        .main_menu
        .stats
        .replacen("{}", &player_stats.total_shifts_completed.to_string(), 1)
        .replacen("{}", &player_stats.total_earnings.to_string(), 1)
        .replacen("{}", &player_stats.total_rides_completed.to_string(), 1)
        .replacen("{}", &player_stats.total_rules_violated.to_string(), 1);
    let progression = format!(
        "Experience Lv. {} | Suggested Difficulty {} | {}h {:02}m behind the wheel",
        player_stats.experience_level(),
        player_stats.suggested_difficulty() + 1,
        player_stats.total_play_time / 60,
        player_stats.total_play_time % 60
    );
    let unlocked = player_stats
        .achievements
        .iter()
        .filter(|a| a.unlocked)
        .count();
    let achievements = data
        .localization
        .ui
        .main_menu
        .achievements
        .replacen("{}", &unlocked.to_string(), 1)
        .replacen("{}", &player_stats.achievements.len().to_string(), 1);
    let rect = UiRect::new((screen_width() - 306.0).max(470.0), 88.0, 260.0, 132.0);
    draw_glass_panel(rect, colors::BORDER_DIM);
    let inner = rect.inset(14.0);
    draw_small_caps(
        "Driver Record",
        inner.x,
        inner.y + 12.0,
        fonts::SIZE_SM,
        colors::CAB_YELLOW,
    );
    draw_wrapped_text(
        &stats,
        inner.x,
        inner.y + 38.0,
        inner.w,
        fonts::SIZE_XS,
        16.0,
        colors::TEXT_SECONDARY,
        2,
    );
    draw_wrapped_text(
        &progression,
        inner.x,
        inner.y + 74.0,
        inner.w,
        fonts::SIZE_XS,
        16.0,
        colors::TEXT_MUTED,
        2,
    );
    draw_ui_text(
        &achievements,
        inner.x,
        inner.y + 112.0,
        fonts::SIZE_XS,
        colors::ACCENT_GOLD,
    );
}

fn draw_primary_commands(
    layout: &DesktopMenuLayout,
    data: &GameData,
    resume_available: bool,
    daily_seed: u64,
) -> Option<UiAction> {
    let start_label = data
        .localization
        .ui
        .main_menu
        .start_space
        .replace("(SPACE)", "");
    if draw_menu_command(
        layout.rect(0.0),
        "wheel",
        start_label.trim(),
        "SPACE",
        colors::CAB_YELLOW,
        layout.scale,
    ) {
        return Some(UiAction::StartGame);
    }
    if resume_available
        && draw_menu_command(
            layout.rect(1.0),
            "wheel",
            "Resume Run",
            "Continue the saved night",
            colors::FUEL_GOOD,
            layout.scale,
        )
    {
        return Some(UiAction::ResumeRun);
    }
    if draw_menu_command(
        layout.rect(1.0 + layout.resume_offset),
        "wheel",
        "Daily Shift",
        &format!("Night #{daily_seed} - dealt to everyone"),
        colors::ACCENT_SKY,
        layout.scale,
    ) {
        return Some(UiAction::StartDailyRun);
    }
    if draw_menu_command(
        layout.rect(2.0 + layout.resume_offset),
        "wheel",
        "Seeded Run",
        "Replay a night by number",
        colors::TEXT_SECONDARY,
        layout.scale,
    ) {
        return Some(UiAction::OpenSeedEntry);
    }
    None
}

fn draw_progression_commands(
    layout: &DesktopMenuLayout,
    data: &GameData,
    player_stats: &PlayerStats,
) -> Option<UiAction> {
    let skill_button = data
        .localization
        .ui
        .meta
        .skill_tree
        .button
        .replace("{}", &player_stats.bank_balance.to_string());
    let skill_detail = skill_button
        .split_once('(')
        .map(|(_, detail)| detail.trim_end_matches(')').to_string())
        .unwrap_or_else(|| format!("${} Available", player_stats.bank_balance));
    if draw_menu_command(
        layout.rect(3.0 + layout.resume_offset),
        "tree",
        "Skill Tree",
        &format!("${skill_detail} Available"),
        colors::TEXT_SECONDARY,
        layout.scale,
    ) {
        return Some(UiAction::OpenSkillTree);
    }
    let almanac_button = data
        .localization
        .ui
        .meta
        .almanac
        .button
        .replace("{}", &player_stats.lore_fragments.to_string());
    let almanac_detail = almanac_button
        .split_once('(')
        .map(|(_, detail)| detail.trim_end_matches(')').to_string())
        .unwrap_or_else(|| format!("{} Lore Fragments", player_stats.lore_fragments));
    if draw_menu_command(
        layout.rect(4.0 + layout.resume_offset),
        "book",
        "Almanac",
        &almanac_detail,
        colors::TEXT_SECONDARY,
        layout.scale,
    ) {
        return Some(UiAction::OpenAlmanac);
    }
    None
}

fn draw_info_commands(layout: &DesktopMenuLayout, data: &GameData) -> Option<UiAction> {
    let leaderboard_label = data
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
        layout.rect(5.0 + layout.resume_offset),
        "trophy",
        &leaderboard_label,
        "Best Runs",
        colors::TEXT_SECONDARY,
        layout.scale,
    ) {
        return Some(UiAction::OpenLeaderboard);
    }
    if draw_menu_command(
        layout.rect(6.0 + layout.resume_offset),
        "book",
        "Help & Options",
        "Controls, tutorial, accessibility",
        colors::ACCENT_SKY,
        layout.scale,
    ) {
        return Some(UiAction::OpenHelpOptions);
    }
    if draw_menu_command(
        layout.rect(7.0 + layout.resume_offset),
        "book",
        "Credits",
        "People, tools, and the midnight city",
        colors::TEXT_SECONDARY,
        layout.scale,
    ) {
        return Some(UiAction::OpenCredits);
    }
    None
}

fn draw_delete_command(
    layout: &DesktopMenuLayout,
    delete_armed: bool,
    has_save: bool,
) -> Option<UiAction> {
    if !has_save {
        return None;
    }
    let (label, detail, colour) = if delete_armed {
        (
            "Confirm Delete",
            "Erases skills, almanac and bank",
            colors::FUEL_CRITICAL,
        )
    } else {
        ("Delete Save", "Reset Progress", colors::ACCENT_DANGER)
    };
    draw_menu_command(
        layout.rect(8.0 + layout.resume_offset),
        "delete",
        label,
        detail,
        colour,
        layout.scale,
    )
    .then_some(UiAction::DeleteSave)
}

fn draw_desktop_footer(data: &GameData, title_x: f32, save_notice: Option<&str>) {
    let warnings = save_notice
        .into_iter()
        .chain(data.load_errors.iter().map(String::as_str));
    for (index, warning) in warnings.enumerate() {
        draw_ui_text(
            warning,
            title_x,
            screen_height() - 38.0 - index as f32 * 18.0,
            fonts::SIZE_XS,
            colors::FUEL_CRITICAL,
        );
    }
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

fn draw_narrow_main_menu(
    game_data: Option<&GameData>,
    delete_armed: bool,
    save_notice: Option<&str>,
    resume_available: bool,
    seed_entry: Option<&str>,
) -> UiAction {
    let (title, subtitle) = game_data
        .map(|data| {
            (
                data.localization.ui.main_menu.title.as_str(),
                data.localization.ui.main_menu.subtitle.as_str(),
            )
        })
        .unwrap_or(("NIGHTMARE SHIFT", "SURVIVE THE NIGHT"));
    draw_wrapped_text(
        title,
        16.0,
        28.0,
        screen_width() - 32.0,
        22.0,
        22.0,
        colors::TEXT_PRIMARY,
        2,
    );
    draw_small_caps(
        &macroquad_toolkit::ui::truncate_text_to_width(
            subtitle,
            screen_width() - 32.0,
            fonts::SIZE_XS,
        ),
        16.0,
        66.0,
        fonts::SIZE_XS,
        colors::CAB_YELLOW,
    );

    if let Some(digits) = seed_entry {
        return draw_narrow_seed_entry(digits);
    }

    let commands = narrow_commands(delete_armed, resume_available);
    let (button_w, button_h, start_x) = narrow_button_layout(commands.len());
    let gap = 4.0;
    for (index, (label, action, accent)) in commands.into_iter().enumerate() {
        let column = index % 3;
        let row = index / 3;
        let rect = UiRect::new(
            start_x + column as f32 * (button_w + gap),
            72.0 + row as f32 * (button_h + gap),
            button_w,
            button_h,
        );
        if draw_compact_menu_command(rect, label, accent) {
            return action;
        }
    }
    draw_narrow_notice(save_notice);
    UiAction::None
}

fn narrow_commands(
    delete_armed: bool,
    resume_available: bool,
) -> Vec<(&'static str, UiAction, Color)> {
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
    commands
}

fn narrow_button_layout(count: usize) -> (f32, f32, f32) {
    let columns = 3.0;
    let gap = 4.0;
    let top = 72.0;
    let button_w = ((screen_width() - 32.0 - gap * (columns - 1.0)) / columns).max(70.0);
    let rows = count.div_ceil(columns as usize);
    let button_h =
        ((screen_height() - top - 12.0 - gap * (rows - 1) as f32) / rows as f32).clamp(22.0, 30.0);
    let total_w = button_w * columns + gap * (columns - 1.0);
    let start_x = (screen_width() - total_w) / 2.0;
    (button_w, button_h, start_x)
}

fn draw_narrow_notice(save_notice: Option<&str>) {
    if let Some(notice) = save_notice {
        draw_small_caps(
            notice,
            16.0,
            screen_height() - 4.0,
            fonts::SIZE_XS,
            colors::FUEL_CRITICAL,
        );
    }
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

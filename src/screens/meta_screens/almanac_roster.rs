//! Compact passenger roster and one persistent knowledge dossier.

use crate::data::{GameData, PreferenceLevel};
use crate::state::PlayerStats;
use crate::ui::{
    colors, draw_glass_button, draw_glass_panel, draw_noir_city_background,
    draw_passenger_portrait, draw_small_caps, draw_ui_icon, draw_ui_text, draw_wrapped_text, fonts,
    UiAction, UiIcon, UiRect,
};
use macroquad::prelude::*;
use macroquad_toolkit::ui::ScrollArea;

fn level_color(level: u32) -> Color {
    match level {
        0 => colors::TEXT_MUTED,
        1 => colors::ACCENT_SKY,
        2 => colors::ACCENT_GOLD,
        _ => colors::FUEL_GOOD,
    }
}

fn draw_unknown_portrait(rect: UiRect) {
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        Color::new(0.02, 0.025, 0.025, 0.96),
    );
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, colors::BORDER_DIM);
    draw_ui_text(
        "?",
        rect.center().x - 8.0,
        rect.center().y + 10.0,
        fonts::SIZE_XXL,
        colors::TEXT_MUTED,
    );
}

fn knowledge_source_line(entry: &crate::state::AlmanacEntry) -> String {
    if entry.knowledge_level == 0 {
        return format!(
            "Observed in play: {} rides survived | {} tells witnessed",
            entry.rides_survived,
            entry.tells_seen.len()
        );
    }
    let tiers = (1..=entry.knowledge_level)
        .map(|level| {
            let earned = entry.earned_knowledge_levels.contains(&level);
            let bought = entry.lore_knowledge_levels.contains(&level);
            let source = match (earned, bought) {
                (true, true) => "earned + lore",
                (true, false) => "earned by rides",
                (false, true) => "bought with lore",
                (false, false) => "earlier record",
            };
            format!("Lv.{level} {source}")
        })
        .collect::<Vec<_>>()
        .join(" | ");
    format!(
        "{tiers}  •  {} rides / {} tells discovered",
        entry.rides_survived,
        entry.tells_seen.len()
    )
}

fn draw_dossier(
    rect: UiRect,
    passenger: &crate::data::Passenger,
    player_stats: &PlayerStats,
    data: &GameData,
) -> UiAction {
    draw_glass_panel(rect, colors::BORDER_DIM);
    let entry = player_stats.get_almanac_entry(passenger.id);
    let inner = rect.inset(18.0);
    draw_small_caps(
        "Selected passenger dossier",
        inner.x,
        inner.y + 14.0,
        fonts::SIZE_SM,
        colors::ACCENT_SKY,
    );

    if !entry.encountered {
        let portrait = UiRect::new(inner.x, inner.y + 34.0, 118.0, 118.0);
        draw_unknown_portrait(portrait);
        draw_ui_text(
            "Unknown Passenger",
            portrait.x + portrait.w + 18.0,
            portrait.y + 32.0,
            fonts::SIZE_XL,
            colors::TEXT_MUTED,
        );
        draw_wrapped_text(
            "Complete a ride with this passenger to add confirmed details to the almanac.",
            portrait.x + portrait.w + 18.0,
            portrait.y + 62.0,
            inner.w - portrait.w - 18.0,
            fonts::SIZE_SM,
            19.0,
            colors::TEXT_SECONDARY,
            3,
        );
        return UiAction::None;
    }

    let portrait_size = inner.w.min(720.0).mul_add(0.16, 42.0).clamp(108.0, 152.0);
    let portrait = UiRect::new(inner.x, inner.y + 32.0, portrait_size, portrait_size);
    draw_passenger_portrait(portrait, passenger.id);
    let info_x = portrait.x + portrait.w + 18.0;
    draw_ui_text(
        &passenger.name,
        info_x,
        portrait.y + 30.0,
        fonts::SIZE_XL,
        colors::TEXT_PRIMARY,
    );
    let level_name = data
        .almanac
        .get_level(entry.knowledge_level)
        .map(|level| level.name.as_str())
        .unwrap_or(&data.localization.ui.meta.almanac.unknown_level);
    draw_small_caps(
        &format!("Lv.{}  {}", entry.knowledge_level, level_name),
        info_x,
        portrait.y + 56.0,
        fonts::SIZE_SM,
        level_color(entry.knowledge_level),
    );
    draw_wrapped_text(
        &knowledge_source_line(&entry),
        info_x,
        portrait.y + 84.0,
        inner.w - portrait.w - 18.0,
        fonts::SIZE_XS,
        16.0,
        colors::ACCENT_GOLD,
        3,
    );

    let mut y = portrait.y + portrait.h + 22.0;
    y = draw_wrapped_text(
        &passenger.description,
        inner.x,
        y,
        inner.w,
        fonts::SIZE_SM,
        19.0,
        colors::TEXT_SECONDARY,
        2,
    ) + 10.0;

    if entry.knowledge_level >= 1 {
        y = draw_wrapped_text(
            &format!("Known traits: {}", passenger.traits.join(", ")),
            inner.x,
            y,
            inner.w,
            fonts::SIZE_XS,
            16.0,
            colors::ACCENT_SKY,
            2,
        ) + 8.0;
    }
    if !entry.tells_seen.is_empty() {
        y = draw_wrapped_text(
            &format!("Discovered tells: {}", entry.tells_seen.join("; ")),
            inner.x,
            y,
            inner.w,
            fonts::SIZE_XS,
            16.0,
            colors::FUEL_GOOD,
            2,
        ) + 8.0;
    }
    if entry.knowledge_level >= 2 {
        let preferences = passenger
            .route_preferences
            .iter()
            .filter(|pref| pref.preference != PreferenceLevel::Neutral)
            .map(|pref| format!("{:?} {:?}", pref.preference, pref.route))
            .collect::<Vec<_>>()
            .join(" | ");
        y = draw_wrapped_text(
            &format!("Known route preferences: {preferences}"),
            inner.x,
            y,
            inner.w,
            fonts::SIZE_XS,
            16.0,
            colors::CAB_YELLOW,
            2,
        ) + 8.0;
    }
    if entry.knowledge_level >= 3 || player_stats.is_backstory_unlocked(passenger.id) {
        draw_wrapped_text(
            &format!("Story discovered in play: {}", passenger.backstory_details),
            inner.x,
            y,
            inner.w,
            fonts::SIZE_XS,
            16.0,
            colors::ACCENT_GOLD,
            2,
        );
    }

    if entry.knowledge_level < 3 {
        let next_level = entry.knowledge_level + 1;
        let cost = data.almanac.get_upgrade_cost(next_level);
        let already: &[&str] = if player_stats.is_backstory_unlocked(passenger.id) {
            &["Backstory"]
        } else {
            &[]
        };
        let revelation = data
            .almanac
            .get_level(next_level)
            .and_then(|level| level.reveals_line(already))
            .unwrap_or_else(|| "No unrevealed dossier fields remain".to_string());
        let button = UiRect::new(inner.x, rect.y + rect.h - 52.0, 210.0, 34.0);
        draw_wrapped_text(
            &format!("Next: {revelation}"),
            button.x + button.w + 16.0,
            button.y + 13.0,
            inner.w - button.w - 16.0,
            fonts::SIZE_XS,
            15.0,
            colors::TEXT_MUTED,
            2,
        );
        let affordable = player_stats.lore_fragments >= cost;
        let cost_label = data
            .localization
            .ui
            .meta
            .almanac
            .upgrade_cost
            .replace("{}", &cost.to_string());
        if draw_glass_button(
            button,
            &format!("Study Lv.{next_level} • {cost_label}"),
            colors::ACCENT_GOLD,
            affordable,
        ) && affordable
        {
            return UiAction::UpgradeAlmanacKnowledge(passenger.id);
        }
    }
    UiAction::None
}

pub fn draw_almanac(
    player_stats: &PlayerStats,
    game_data: Option<&GameData>,
    scroll: &mut ScrollArea,
    selected: Option<u32>,
) -> UiAction {
    draw_noir_city_background();
    let Some(data) = game_data else {
        return UiAction::None;
    };
    let w = screen_width();
    let h = screen_height();
    let margin = (w * 0.045).clamp(30.0, 70.0);
    let header = UiRect::new(margin, 28.0, w - margin * 2.0, 82.0);
    draw_glass_panel(header, colors::BORDER_DIM);
    draw_ui_text(
        &data.localization.ui.meta.almanac.title,
        header.x + 18.0,
        header.y + 34.0,
        fonts::SIZE_XXL,
        colors::CAB_YELLOW,
    );
    draw_ui_icon(
        UiIcon::Lore,
        header.x + header.w - 34.0,
        header.y + 40.0,
        28.0,
        colors::ACCENT_GOLD,
    );
    let fragments = data
        .localization
        .ui
        .meta
        .almanac
        .fragments
        .replace("{}", &player_stats.lore_fragments.to_string());
    draw_small_caps(
        &fragments,
        header.x + 18.0,
        header.y + 62.0,
        fonts::SIZE_SM,
        colors::ACCENT_GOLD,
    );
    draw_small_caps(
        "Earned: rides and tells  •  Bought: lore study",
        header.x + header.w * 0.48,
        header.y + 48.0,
        fonts::SIZE_XS,
        colors::TEXT_MUTED,
    );

    let effective_selected = selected.or_else(|| {
        data.passengers
            .iter()
            .find(|passenger| player_stats.get_almanac_entry(passenger.id).encountered)
            .or_else(|| data.passengers.first())
            .map(|passenger| passenger.id)
    });
    let body_top = 124.0;
    let body_bottom = h - 66.0;
    let body_h = body_bottom - body_top;
    let gap = 18.0;
    let roster_w = (w * 0.36).clamp(300.0, 620.0);
    let roster = UiRect::new(margin, body_top, roster_w, body_h);
    let dossier = UiRect::new(
        roster.x + roster.w + gap,
        body_top,
        w - margin - (roster.x + roster.w + gap),
        body_h,
    );
    draw_glass_panel(roster, colors::BORDER_DIM);
    draw_small_caps(
        "Passenger roster",
        roster.x + 14.0,
        roster.y + 26.0,
        fonts::SIZE_SM,
        colors::CAB_YELLOW,
    );

    let columns = if roster.w >= 430.0 { 2 } else { 1 };
    let card_gap = 8.0;
    let card_w = (roster.w - 28.0 - card_gap * (columns - 1) as f32) / columns as f32;
    let card_h = 82.0;
    let rows = data.passengers.len().div_ceil(columns);
    let view = Rect::new(
        roster.x + 10.0,
        roster.y + 38.0,
        roster.w - 20.0,
        roster.h - 48.0,
    );
    let content_h = rows as f32 * (card_h + card_gap);
    scroll.update(view, content_h);
    for (index, passenger) in data.passengers.iter().enumerate() {
        let col = index % columns;
        let row = index / columns;
        let card = UiRect::new(
            view.x + col as f32 * (card_w + card_gap),
            view.y + row as f32 * (card_h + card_gap) - scroll.offset(),
            card_w,
            card_h,
        );
        if card.y + card.h < view.y || card.y > view.y + view.h {
            continue;
        }
        let entry = player_stats.get_almanac_entry(passenger.id);
        let is_selected = effective_selected == Some(passenger.id);
        let accent = if is_selected {
            colors::ACCENT_GOLD
        } else if entry.encountered {
            colors::ACCENT_SKY
        } else {
            colors::BORDER_DIM
        };
        if draw_glass_button(card, "", accent, true) && !scroll.absorbs_press() {
            return UiAction::SelectAlmanacPassenger(passenger.id);
        }
        let portrait = UiRect::new(card.x + 8.0, card.y + 8.0, 54.0, 66.0);
        if entry.encountered {
            draw_passenger_portrait(portrait, passenger.id);
        } else {
            draw_unknown_portrait(portrait);
        }
        let text_x = portrait.x + portrait.w + 8.0;
        draw_wrapped_text(
            if entry.encountered {
                &passenger.name
            } else {
                "Unknown"
            },
            text_x,
            card.y + 26.0,
            card.w - portrait.w - 24.0,
            fonts::SIZE_SM,
            16.0,
            if entry.encountered {
                colors::TEXT_PRIMARY
            } else {
                colors::TEXT_MUTED
            },
            2,
        );
        draw_small_caps(
            &format!(
                "Lv.{} • {} rides",
                entry.knowledge_level, entry.rides_survived
            ),
            text_x,
            card.y + 66.0,
            fonts::SIZE_XS,
            level_color(entry.knowledge_level),
        );
    }
    scroll.draw_scrollbar(view, content_h);

    if let Some(passenger) = data
        .passengers
        .iter()
        .find(|passenger| Some(passenger.id) == effective_selected)
    {
        let action = draw_dossier(dossier, passenger, player_stats, data);
        if action != UiAction::None {
            return action;
        }
    }
    let back = UiRect::new(w / 2.0 - 108.0, h - 56.0, 216.0, 40.0);
    if draw_glass_button(
        back,
        &data.localization.ui.common.back_button,
        colors::ACCENT_SKY,
        true,
    ) {
        return UiAction::ReturnToMenu;
    }
    UiAction::None
}

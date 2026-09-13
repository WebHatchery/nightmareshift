//! Skill tree screen: spend bank balance to unlock permanent skills.

use crate::data::GameData;
use crate::engine::RideService;
use crate::state::PlayerStats;
use crate::ui::{
    colors, draw_glass_button, draw_glass_panel, draw_noir_city_background, draw_small_caps,
    draw_wrapped_text, fonts, UiAction, UiRect,
};
use crate::ui::{draw_ui_text, measure_ui_text};
use macroquad::prelude::*;
use macroquad_toolkit::ui::ScrollArea;

fn skill_category_label(category: &str, data: &GameData) -> String {
    let loc_cats = &data.localization.ui.meta.skill_tree.categories;
    match category {
        "survival" => loc_cats.survival.clone(),
        "occult" => loc_cats.occult.clone(),
        "efficiency" => loc_cats.efficiency.clone(),
        // Hardcoded like the other ~145 strings pending localization (TODO.md).
        "comfort" => "Comfort".to_string(),
        _ => category.to_string(),
    }
}

fn draw_skill_category_mark(category: &str, x: f32, y: f32, color: Color) {
    match category {
        "survival" => {
            draw_rectangle_lines(x - 10.0, y - 13.0, 20.0, 25.0, 2.0, color);
            draw_line(x, y - 18.0, x, y + 18.0, 2.0, color);
            draw_line(x - 14.0, y - 4.0, x + 14.0, y - 4.0, 2.0, color);
        }
        "occult" => {
            draw_circle_lines(x, y, 16.0, 2.0, color);
            draw_circle_lines(x, y, 6.0, 1.5, color);
            draw_line(x - 20.0, y, x + 20.0, y, 1.5, color);
        }
        "efficiency" => {
            draw_circle_lines(x, y, 15.0, 2.0, color);
            draw_line(x - 9.0, y + 10.0, x + 11.0, y - 10.0, 2.0, color);
            draw_line(x - 3.0, y + 10.0, x + 13.0, y + 10.0, 2.0, color);
        }
        "comfort" => {
            // A bench seat: back rest and cushion.
            draw_line(x - 14.0, y - 14.0, x - 14.0, y + 8.0, 2.0, color);
            draw_rectangle_lines(x - 14.0, y + 2.0, 28.0, 10.0, 2.0, color);
            draw_line(x + 14.0, y + 12.0, x + 14.0, y + 16.0, 2.0, color);
            draw_line(x - 14.0, y + 12.0, x - 14.0, y + 16.0, 2.0, color);
        }
        _ => {
            draw_circle_lines(x, y, 14.0, 2.0, color);
        }
    }
}

/// For an `ability_unlock` skill, how many passengers carry its trait and how
/// many of those the player has studied. `None` for every other skill.
///
/// The choice an ability skill buys only appears for a passenger who carries
/// the matching trait *and* whom the player has studied to Almanac Lv.1. That
/// second requirement is nowhere in `prerequisites`, so the skill tree used to
/// offer Night Vision for $750 with no hint that an unstudied roster makes it
/// inert — which is most of why a skills-only run measures barely better than
/// no progression at all. The studied count is the number of times the
/// purchase can ever fire.
pub fn ability_carriers(
    skill: &crate::data::Skill,
    passengers: &[crate::data::Passenger],
    player_stats: &PlayerStats,
) -> Option<(usize, usize)> {
    if skill.effect.effect_type != "ability_unlock" {
        return None;
    }
    let carried = passengers.iter().filter(|passenger| {
        passenger
            .traits
            .iter()
            .any(|name| RideService::trait_skill_id(name) == skill.effect.target)
    });
    Some(carried.fold((0, 0), |(studied, total), passenger| {
        let known = player_stats.get_almanac_entry(passenger.id).knowledge_level >= 1;
        (studied + usize::from(known), total + 1)
    }))
}

fn draw_skill_card(
    rect: UiRect,
    skill: &crate::data::Skill,
    is_unlocked: bool,
    can_unlock: bool,
    player_stats: &PlayerStats,
    data: &GameData,
) -> UiAction {
    let (border, status, status_color) = if is_unlocked {
        (
            colors::FUEL_GOOD,
            data.localization.ui.meta.skill_tree.unlocked.as_str(),
            colors::FUEL_GOOD,
        )
    } else if can_unlock {
        (colors::CAB_YELLOW, "Available", colors::CAB_YELLOW)
    } else {
        (colors::BORDER_DIM, "Locked", colors::TEXT_MUTED)
    };

    let bg = if is_unlocked {
        Color::new(0.035, 0.090, 0.045, 0.82)
    } else if can_unlock {
        Color::new(0.100, 0.075, 0.025, 0.82)
    } else {
        colors::GLASS
    };
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, bg);
    draw_rectangle(rect.x, rect.y, 4.0, rect.h, border);
    draw_rectangle(rect.x, rect.y, rect.w, 1.0, Color::new(1.0, 1.0, 1.0, 0.10));
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, border);

    let icon_x = rect.x + 24.0;
    let icon_y = rect.y + 30.0;
    draw_circle_lines(icon_x, icon_y, 15.0, 1.5, colors::TEXT_MUTED);
    let initials = skill
        .name
        .split_whitespace()
        .filter_map(|part| part.chars().next())
        .take(2)
        .collect::<String>();
    let initials_width = measure_ui_text(&initials, None, fonts::SIZE_XS as u16, 1.0).width;
    draw_ui_text(
        &initials,
        icon_x - initials_width / 2.0,
        icon_y + 4.0,
        fonts::SIZE_XS,
        colors::TEXT_SECONDARY,
    );

    let text_x = rect.x + 54.0;
    draw_ui_text(
        &skill.name,
        text_x,
        rect.y + 27.0,
        fonts::SIZE_MD,
        colors::TEXT_PRIMARY,
    );
    draw_small_caps(
        &format!("${}  {}", skill.cost, status),
        text_x,
        rect.y + 47.0,
        fonts::SIZE_XS,
        status_color,
    );

    let desc_bottom = draw_wrapped_text(
        &skill.description,
        text_x,
        rect.y + 66.0,
        rect.w - 72.0,
        fonts::SIZE_XS,
        15.0,
        colors::TEXT_SECONDARY,
        2,
    );

    let mut prereq_text = if skill.prerequisites.is_empty() {
        "No prerequisite".to_string()
    } else {
        let met = skill
            .prerequisites
            .iter()
            .filter(|id| player_stats.unlocked_skills.contains(id))
            .count();
        format!("Prerequisites {}/{}", met, skill.prerequisites.len())
    };

    // An ability skill has a second requirement that is not in
    // `prerequisites` at all: the choice it buys only appears for a
    // passenger who carries the matching trait *and* whom the player has
    // studied to Almanac Lv.1. Buying one against an unstudied roster buys
    // nothing, and this screen said nothing about it — which is why a
    // skills-only run measures as barely better than no progression.
    //
    // Naming the count rather than the rule makes it actionable: five
    // passengers carry Night Vision, and the number of them you have
    // studied is the number of times this purchase can ever fire.
    let mut unmet_ability = false;
    if let Some((studied, total)) = ability_carriers(skill, &data.passengers, player_stats) {
        prereq_text = format!("{prereq_text}  |  {studied}/{total} carriers studied");
        unmet_ability = studied == 0;
    }

    draw_small_caps(
        &prereq_text,
        text_x,
        (desc_bottom + 12.0).min(rect.y + rect.h - 18.0),
        fonts::SIZE_XS,
        if unmet_ability {
            colors::ACCENT_WARNING
        } else {
            colors::TEXT_MUTED
        },
    );
    let effect_text = if skill.effect.effect_type == "ability_unlock" {
        format!("Gameplay effect: unlocks {} choices", skill.effect.target)
    } else if matches!(
        skill.effect.target.as_str(),
        "fuel_consumption" | "hazard_damage" | "fare_multiplier"
    ) {
        format!(
            "Mechanical magnitude: {} x{:.2}",
            skill.effect.target.replace('_', " "),
            skill.effect.value
        )
    } else {
        format!(
            "Mechanical magnitude: {} +{}",
            skill.effect.target.replace('_', " "),
            skill.effect.value
        )
    };
    draw_small_caps(
        &effect_text,
        text_x,
        (desc_bottom + 32.0).min(rect.y + rect.h - 52.0),
        fonts::SIZE_XS,
        colors::ACCENT_SKY,
    );

    if can_unlock {
        let buy_rect = UiRect::new(rect.x + rect.w - 116.0, rect.y + rect.h - 36.0, 96.0, 26.0);
        if draw_glass_button(
            buy_rect,
            &data.localization.ui.meta.skill_tree.purchase,
            colors::CAB_YELLOW,
            true,
        ) {
            return UiAction::PurchaseSkill(skill.id.clone());
        }
    }

    UiAction::None
}

/// Draw the skill tree screen
pub fn draw_skill_tree(
    player_stats: &PlayerStats,
    game_data: Option<&GameData>,
    scroll: &mut ScrollArea,
    selected_category: usize,
    selected_skill: Option<&str>,
) -> UiAction {
    draw_noir_city_background();

    if let Some(data) = game_data {
        let screen_w = screen_width();
        let screen_h = screen_height();
        let center_x = screen_w / 2.0;
        let margin = (screen_w * 0.045).clamp(30.0, 70.0);
        let header_h = 92.0;
        let footer_h = 66.0;
        let content_top = header_h + 26.0;
        let content_bottom = screen_h - footer_h;

        let header_rect = UiRect::new(margin, 28.0, screen_w - margin * 2.0, header_h);
        draw_glass_panel(header_rect, colors::BORDER_DIM);
        let header_inner = header_rect.inset(18.0);

        let title = &data.localization.ui.meta.skill_tree.title;
        draw_ui_text(
            title,
            header_inner.x,
            header_inner.y + 34.0,
            fonts::SIZE_XXL,
            colors::FUEL_GOOD,
        );

        let balance = data
            .localization
            .ui
            .meta
            .skill_tree
            .bank_balance
            .replace("{}", &player_stats.bank_balance.to_string());
        draw_small_caps(
            &balance,
            header_inner.x,
            header_inner.y + 62.0,
            fonts::SIZE_MD,
            colors::CAB_YELLOW,
        );
        draw_small_caps(
            "Choose a category, then inspect the selected upgrade.",
            header_inner.x + header_inner.w * 0.52,
            header_inner.y + 22.0,
            fonts::SIZE_XS,
            colors::TEXT_MUTED,
        );

        // Sell surplus lore back for bank. Without this the two currencies
        // never meet: lore goes dead once the almanac is mastered while the
        // skill tree stays starved.
        let rate = data.rewards.lore_exchange;
        if rate.is_available() {
            let exchange_rect = UiRect::new(
                header_inner.x + header_inner.w * 0.52,
                header_inner.y + 34.0,
                236.0,
                30.0,
            );
            let label = format!("Trade {} lore -> ${}", rate.lore, rate.bank);
            let affordable = player_stats.lore_fragments >= rate.lore;
            if draw_glass_button(exchange_rect, &label, colors::ACCENT_GOLD, affordable)
                && affordable
            {
                return UiAction::ExchangeLoreForBank;
            }
            draw_small_caps(
                &format!("{} lore fragments held", player_stats.lore_fragments),
                exchange_rect.x + exchange_rect.w + 14.0,
                exchange_rect.y + 20.0,
                fonts::SIZE_XS,
                colors::TEXT_MUTED,
            );
        }

        let categories = ["survival", "occult", "efficiency", "comfort"];
        let category_index = selected_category.min(categories.len() - 1);
        let tab_gap = 10.0;
        let tab_w = ((screen_w - margin * 2.0 - tab_gap * 3.0) / 4.0).max(120.0);
        for (idx, category) in categories.iter().enumerate() {
            let tab = UiRect::new(
                margin + idx as f32 * (tab_w + tab_gap),
                content_top,
                tab_w,
                42.0,
            );
            let selected = idx == category_index;
            if draw_glass_button(
                tab,
                &skill_category_label(category, data),
                if selected {
                    colors::CAB_YELLOW
                } else {
                    colors::BORDER_DIM
                },
                true,
            ) && !selected
            {
                return UiAction::SelectSkillCategory(idx);
            }
        }

        let category = categories[category_index];
        let skills = data
            .skills
            .iter()
            .filter(|skill| skill.category == category)
            .collect::<Vec<_>>();
        let effective_skill =
            if selected_skill.is_none_or(|id| !skills.iter().any(|skill| skill.id == id)) {
                skills.first().map(|skill| skill.id.as_str())
            } else {
                selected_skill
            };

        let body_top = content_top + 56.0;
        let body_h = content_bottom - body_top;
        let gap = 18.0;
        let list_w = if screen_w >= 1100.0 {
            (screen_w * 0.42).clamp(390.0, 620.0)
        } else {
            (screen_w - margin * 2.0) * 0.46
        };
        let detail_w = screen_w - margin * 2.0 - list_w - gap;
        let list_panel = UiRect::new(margin, body_top, list_w, body_h);
        let detail_panel = UiRect::new(margin + list_w + gap, body_top, detail_w, body_h);
        draw_glass_panel(list_panel, colors::BORDER_DIM);
        draw_glass_panel(detail_panel, colors::BORDER_DIM);

        let unlocked_count = skills
            .iter()
            .filter(|skill| player_stats.is_skill_unlocked(&skill.id))
            .count();
        draw_skill_category_mark(
            category,
            list_panel.x + 28.0,
            list_panel.y + 28.0,
            colors::CAB_YELLOW,
        );
        draw_small_caps(
            &format!(
                "{}  {}/{} unlocked",
                skill_category_label(category, data),
                unlocked_count,
                skills.len()
            ),
            list_panel.x + 56.0,
            list_panel.y + 32.0,
            fonts::SIZE_SM,
            colors::CAB_YELLOW,
        );

        let card_h = 88.0;
        let card_gap = 10.0;
        let list_view = Rect::new(
            list_panel.x + 10.0,
            list_panel.y + 52.0,
            list_panel.w - 20.0,
            list_panel.h - 62.0,
        );
        let list_height = skills.len() as f32 * (card_h + card_gap);
        scroll.update(list_view, list_height);
        let mut y = list_view.y - scroll.offset();
        for skill in &skills {
            let is_selected = effective_skill == Some(skill.id.as_str());
            let is_unlocked = player_stats.is_skill_unlocked(&skill.id);
            let available = skill.can_unlock(&player_stats.unlocked_skills)
                && player_stats.bank_balance >= skill.cost
                && !is_unlocked;
            let accent = if is_selected {
                colors::CAB_YELLOW
            } else if is_unlocked {
                colors::FUEL_GOOD
            } else {
                colors::BORDER_DIM
            };
            let card = UiRect::new(list_view.x, y, list_view.w - 8.0, card_h);
            if y + card_h > list_view.y && y < list_view.y + list_view.h {
                if draw_glass_button(card, "", accent, true) && !scroll.absorbs_press() {
                    return UiAction::SelectSkill(skill.id.clone());
                }
                draw_ui_text(
                    &skill.name,
                    card.x + 16.0,
                    card.y + 28.0,
                    fonts::SIZE_MD,
                    colors::TEXT_PRIMARY,
                );
                draw_small_caps(
                    &format!(
                        "${}  {}",
                        skill.cost,
                        if is_unlocked {
                            "Unlocked"
                        } else if available {
                            "Available"
                        } else {
                            "Locked"
                        }
                    ),
                    card.x + 16.0,
                    card.y + 48.0,
                    fonts::SIZE_XS,
                    if available {
                        colors::CAB_YELLOW
                    } else {
                        colors::TEXT_MUTED
                    },
                );
                draw_wrapped_text(
                    &skill.description,
                    card.x + 16.0,
                    card.y + 68.0,
                    card.w - 32.0,
                    fonts::SIZE_XS,
                    14.0,
                    colors::TEXT_SECONDARY,
                    1,
                );
            }
            y += card_h + card_gap;
        }
        scroll.draw_scrollbar(list_view, list_height);

        if let Some(skill) = skills
            .iter()
            .find(|skill| effective_skill == Some(skill.id.as_str()))
        {
            draw_small_caps(
                "Selected upgrade",
                detail_panel.x + 18.0,
                detail_panel.y + 28.0,
                fonts::SIZE_SM,
                colors::ACCENT_SKY,
            );
            let is_unlocked = player_stats.is_skill_unlocked(&skill.id);
            let can_unlock = skill.can_unlock(&player_stats.unlocked_skills)
                && !is_unlocked
                && player_stats.bank_balance >= skill.cost;
            let action = draw_skill_card(
                UiRect::new(
                    detail_panel.x + 14.0,
                    detail_panel.y + 44.0,
                    detail_panel.w - 28.0,
                    detail_panel.h - 58.0,
                ),
                skill,
                is_unlocked,
                can_unlock,
                player_stats,
                data,
            );
            if action != UiAction::None {
                return action;
            }
        }

        draw_rectangle(
            0.0,
            0.0,
            screen_w,
            content_top - 8.0,
            Color::new(0.0, 0.0, 0.0, 0.10),
        );
        draw_rectangle(
            0.0,
            content_bottom,
            screen_w,
            screen_h - content_bottom,
            Color::new(0.0, 0.0, 0.0, 0.35),
        );
        let back_rect = UiRect::new(center_x - 108.0, screen_h - 56.0, 216.0, 40.0);
        if draw_glass_button(
            back_rect,
            &data.localization.ui.common.back_button,
            colors::ACCENT_SKY,
            true,
        ) {
            return UiAction::ReturnToMenu;
        }
    }

    UiAction::None
}

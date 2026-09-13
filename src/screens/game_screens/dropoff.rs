//! The drop-off screen: ride completion summary and the trade offer modal.

use macroquad::prelude::*;

use crate::data::{GameData, Rarity};
use crate::state::GameState;
use crate::ui::draw_ui_text;
use crate::ui::{
    colors, draw_cockpit_background, draw_glass_button, draw_glass_panel, draw_small_caps,
    draw_wrapped_text, fonts, layout, spacing, CompletionSummary, UiAction, UiRect,
};

use super::scene::draw_bottom_taxi_scene;

/// Vertical layout of the trade panel, measured from the panel's inner top.
/// The item buttons start below two wrapped lines of "they are looking for"
/// and the "select an item" label that sits 20px above them.
const BUTTONS_TOP: f32 = 190.0;
const ITEM_ROW_H: f32 = 45.0;
const DECLINE_GAP: f32 = 22.0;
const BUTTON_H: f32 = 40.0;

/// Draw the dropoff screen
pub fn draw_dropoff(game_state: &GameState, game_data: Option<&GameData>) -> UiAction {
    draw_cockpit_background();

    if screen_width() < 980.0 || screen_height() < 560.0 {
        return draw_narrow_dropoff(game_state, game_data);
    }

    let scene_h = (screen_height() * 0.27).clamp(210.0, 300.0);
    let scene_rect = UiRect::new(
        70.0,
        screen_height() - scene_h - 34.0,
        screen_width() - 140.0,
        scene_h,
    );
    draw_bottom_taxi_scene(scene_rect);

    if let Some(ref completion) = game_state.last_ride_completion {
        let rect_w = (screen_width() - 140.0).min(880.0);
        let rect_h = (scene_rect.y - layout::STATUS_BAR_HEIGHT - 76.0).clamp(440.0, 520.0);
        let rect = UiRect::centered_x(
            screen_width(),
            layout::STATUS_BAR_HEIGHT + 34.0,
            rect_w,
            rect_h,
        );
        // CompletionSummary now needs game_data
        let completion_action = CompletionSummary::draw(completion, rect, game_data);

        if let Some(route) = game_state.route_history.last() {
            let route_text = format!(
                "Last leg: {:?} {:?} | fuel -{} | time -{} | {:.0}s ago",
                route.driving_phase,
                route.route_type,
                route.fuel_cost,
                route.time_cost,
                (get_time() - route.timestamp).max(0.0)
            );
            draw_ui_text(
                &route_text,
                rect.x + 16.0,
                rect.bottom() - 96.0,
                12.0,
                colors::TEXT_MUTED,
            );
        }

        // The ride's consequence ledger: the authored lines for whatever
        // fired this ride, stacked upward so the newest sits closest to the
        // summary rows. Three at most; the ledger is flavor, not a log.
        for (idx, note) in game_state
            .consequence_notes
            .iter()
            .rev()
            .take(3)
            .enumerate()
        {
            draw_ui_text(
                note,
                rect.x + 16.0,
                rect.bottom() - 116.0 - idx as f32 * 18.0,
                fonts::SIZE_XS,
                colors::ACCENT_SKY,
            );
        }

        // What the last swap did, on the screen the swap happened on. This
        // used to be written into `current_dialogue`, which only the driving
        // screen renders and which accepting the next ride overwrites — so
        // the standing and the relief a wanted item earns were paid without
        // the player ever being told, and handing over the wrong thing said
        // nothing at all.
        if let Some(ref outcome) = game_state.trade_outcome {
            draw_wrapped_text(
                &outcome.text,
                rect.x + 16.0,
                rect.bottom() - 76.0,
                rect.w - 32.0,
                fonts::SIZE_SM,
                16.0,
                if outcome.was_wanted {
                    colors::ACCENT_GOLD
                } else {
                    colors::TEXT_MUTED
                },
                1,
            );
        }

        // Show trade offer if available
        if let Some((ref passenger_name, ref offered_item)) = game_state.pending_trade {
            if let Some(data) = game_data {
                draw_rectangle(
                    0.0,
                    0.0,
                    screen_width(),
                    screen_height(),
                    Color::new(0.0, 0.0, 0.0, 0.50),
                );
                // Tradeable items, carrying their real inventory index. The
                // filter must come before the cap: taking the first three
                // items and then skipping untradeable ones hid every
                // tradeable item past index two, and left the player staring
                // at "select an item" with no buttons when the first three
                // happened to be bound or cursed.
                let tradeable: Vec<(usize, &crate::data::InventoryItem)> = game_state
                    .inventory
                    .iter()
                    .enumerate()
                    .filter(|(_, item)| item.can_be_given_away())
                    .take(3)
                    .collect();

                // Size the panel to what is in it. It was a fixed height with
                // the decline button pinned at a fixed offset, so a player
                // carrying one tradeable item got a button, then a hand's
                // width of nothing, then Decline.
                let rows = tradeable.len().max(1) as f32;
                let panel_h = BUTTONS_TOP + rows * ITEM_ROW_H + DECLINE_GAP + BUTTON_H + 32.0;
                let trade_rect =
                    UiRect::centered_x(screen_width(), 146.0, screen_width().min(460.0), panel_h);
                draw_glass_panel(trade_rect, colors::ACCENT_SKY);

                let inner = trade_rect.inset(spacing::PADDING_MD);

                draw_ui_text(
                    &data.localization.ui.game.trade.title,
                    inner.x,
                    inner.y + 24.0,
                    fonts::SIZE_LG,
                    colors::ACCENT_SKY,
                );

                // Message
                let msg = data
                    .localization
                    .ui
                    .game
                    .trade
                    .wants_to_trade
                    .replace("{}", passenger_name);
                draw_ui_text(
                    &msg,
                    inner.x,
                    inner.y + 50.0,
                    fonts::SIZE_MD,
                    colors::TEXT_PRIMARY,
                );

                // Offered item
                let rarity_color = match offered_item.rarity {
                    Rarity::Common => colors::TEXT_MUTED,
                    Rarity::Uncommon => colors::ACCENT_SKY,
                    Rarity::Rare => Color::from_hex(0x87CEEB),
                    Rarity::Legendary => colors::ACCENT_WARNING,
                };

                let offer_text = data
                    .localization
                    .ui
                    .game
                    .trade
                    .offering
                    .replace("{}", &offered_item.name);
                draw_ui_text(
                    &offer_text,
                    inner.x,
                    inner.y + 76.0,
                    fonts::SIZE_MD,
                    rarity_color,
                );

                // What is being handed over. The offer named the item and
                // coloured it by rarity, which says how scarce it is and
                // nothing about what it does — so a cursed gift and a
                // protective one looked alike. The authored description is
                // the diegetic warning: it hints rather than labels.
                if !offered_item.description.is_empty() {
                    draw_wrapped_text(
                        &offered_item.description,
                        inner.x,
                        inner.y + 94.0,
                        inner.w,
                        fonts::SIZE_XS,
                        15.0,
                        colors::TEXT_SECONDARY,
                        2,
                    );
                }

                // Name what this passenger actually wants. `wantedItems` is
                // authored per passenger and already decides whether a trade is
                // offered at all; without showing it the player is picking
                // blind and the data only ever works behind their back.
                let wanted: &[String] = game_state
                    .current_passenger
                    .as_ref()
                    .map(|p| p.wanted_items.as_slice())
                    .unwrap_or(&[]);
                let wants_text = if wanted.is_empty() {
                    data.localization.ui.game.trade.any_item.clone()
                } else {
                    format!("They are looking for: {}", wanted.join(", "))
                };
                draw_wrapped_text(
                    &wants_text,
                    inner.x,
                    inner.y + 132.0,
                    inner.w,
                    fonts::SIZE_SM,
                    16.0,
                    if wanted.is_empty() {
                        colors::TEXT_MUTED
                    } else {
                        colors::ACCENT_GOLD
                    },
                    2,
                );

                // Buttons
                let btn_y = inner.y + BUTTONS_TOP;
                let btn_w = 200.0;
                let center_x = screen_width() / 2.0;

                if !tradeable.is_empty() {
                    draw_ui_text(
                        &data.localization.ui.game.trade.select,
                        inner.x,
                        btn_y - 20.0,
                        fonts::SIZE_SM,
                        colors::TEXT_PRIMARY,
                    );

                    for (slot, (index, item)) in tradeable.iter().enumerate() {
                        let is_wanted = wanted.contains(&item.name);
                        let label = if is_wanted {
                            format!("{} (wanted)", item.name)
                        } else {
                            item.name.clone()
                        };
                        let item_btn_y = btn_y + (slot as f32 * ITEM_ROW_H);
                        if draw_glass_button(
                            UiRect::new(center_x - btn_w / 2.0, item_btn_y, btn_w, 35.0),
                            &label,
                            if is_wanted {
                                colors::ACCENT_GOLD
                            } else {
                                colors::ACCENT_SKY
                            },
                            true,
                        ) {
                            return UiAction::AcceptTrade(*index);
                        }
                    }

                    let decline_y = btn_y + tradeable.len() as f32 * ITEM_ROW_H + DECLINE_GAP;
                    if draw_glass_button(
                        UiRect::new(center_x - btn_w / 2.0, decline_y, btn_w, BUTTON_H),
                        &data.localization.ui.game.trade.decline,
                        colors::ACCENT_DANGER,
                        true,
                    ) {
                        return UiAction::DeclineTrade;
                    }
                } else {
                    // Nothing tradeable to offer — inventory may still hold
                    // bound items like the Blessed Medallion.
                    draw_ui_text(
                        &data.localization.ui.game.trade.empty,
                        inner.x,
                        btn_y,
                        fonts::SIZE_SM,
                        colors::ACCENT_WARNING,
                    );
                    if draw_glass_button(
                        UiRect::new(center_x - btn_w / 2.0, btn_y + 30.0, btn_w, BUTTON_H),
                        &data.localization.ui.common.continue_text,
                        colors::ACCENT_SKY,
                        true,
                    ) {
                        return UiAction::DeclineTrade;
                    }
                }
            }
        } else {
            return completion_action;
        }
    }
    UiAction::None
}

fn draw_narrow_dropoff(game_state: &GameState, game_data: Option<&GameData>) -> UiAction {
    let Some(completion) = game_state.last_ride_completion.as_ref() else {
        return UiAction::None;
    };
    let panel = UiRect::new(
        12.0,
        layout::STATUS_BAR_HEIGHT + 8.0,
        screen_width() - 24.0,
        110.0,
    );
    draw_glass_panel(panel, colors::FUEL_GOOD);
    draw_small_caps(
        "RIDE COMPLETE",
        panel.x + 12.0,
        panel.y + 18.0,
        fonts::SIZE_XS,
        colors::FUEL_GOOD,
    );
    draw_ui_text(
        &completion.passenger.name,
        panel.x + 12.0,
        panel.y + 43.0,
        fonts::SIZE_LG,
        colors::CAB_YELLOW,
    );
    draw_small_caps(
        &format!(
            "FARE +${}  |  FUEL -{}  |  TIME -{}m  |  RULES +{}",
            completion.fare_earned,
            completion.impact.fuel_spent,
            completion.impact.time_spent,
            completion.impact.rules_violated
        ),
        panel.x + 12.0,
        panel.y + 66.0,
        fonts::SIZE_XS,
        if completion.impact.rules_violated > 0 {
            colors::ACCENT_DANGER
        } else {
            colors::TEXT_SECONDARY
        },
    );
    if let Some((passenger_name, offered_item)) = game_state.pending_trade.as_ref() {
        draw_small_caps(
            &format!("{passenger_name} offers {}", offered_item.name),
            panel.x + 12.0,
            panel.y + 84.0,
            fonts::SIZE_XS,
            colors::ACCENT_GOLD,
        );
    }
    let label = game_data
        .map(|data| data.localization.ui.common.continue_text.as_str())
        .unwrap_or("CONTINUE");
    if draw_glass_button(
        UiRect::new(12.0, screen_height() - 34.0, screen_width() - 24.0, 26.0),
        label,
        colors::FUEL_GOOD,
        true,
    ) {
        return if game_state.pending_trade.is_some() {
            UiAction::DeclineTrade
        } else {
            UiAction::Continue
        };
    }
    UiAction::None
}

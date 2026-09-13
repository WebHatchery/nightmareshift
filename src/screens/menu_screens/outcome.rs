//! End-of-night screens: the game over report and the success payout.

use macroquad::prelude::*;

use crate::data::GameData;
use crate::state::GameState;
use crate::ui::{
    colors, draw_glass_button, draw_glass_panel, draw_noir_city_background, draw_small_caps,
    draw_wrapped_text, fonts, spacing, UiAction, UiRect,
};
use crate::ui::{draw_ui_text, measure_ui_text};

/// Draw the game over screen
pub fn draw_game_over(game_state: &GameState, game_data: Option<&GameData>) -> UiAction {
    draw_noir_city_background();
    if screen_width() < 980.0 || screen_height() < 560.0 {
        return draw_narrow_game_over(game_state, game_data);
    }
    let center_x = screen_width() / 2.0;

    if let Some(data) = game_data {
        // Taller when an epilogue paragraph rides along.
        let panel_h = if game_state.epilogue.is_some() {
            470.0
        } else {
            364.0
        };
        let panel = UiRect::centered_x(screen_width(), 96.0, screen_width().min(560.0), panel_h);
        draw_glass_panel(panel, colors::ACCENT_DANGER);
        let inner = panel.inset(spacing::PADDING_LG);

        let title = &data.localization.ui.game_over.title;
        let title_size = 46.0;
        let title_width = measure_ui_text(title, None, title_size as u16, 1.0).width;
        draw_ui_text(
            title,
            center_x - title_width / 2.0,
            inner.y + 50.0,
            title_size,
            colors::FUEL_CRITICAL,
        );

        if let Some(ref reason) = game_state.game_over_reason {
            draw_wrapped_text(
                reason,
                inner.x,
                inner.y + 92.0,
                inner.w,
                fonts::SIZE_MD,
                22.0,
                colors::TEXT_SECONDARY,
                3,
            );
        }

        let score = game_state.calculate_score(&data.constants);
        let stats = data
            .localization
            .ui
            .game_over
            .stats
            .replacen("{}", &game_state.earnings.to_string(), 1)
            .replacen("{}", &game_state.rides_completed.to_string(), 1)
            .replacen("{}", &score.to_string(), 1);

        draw_ui_text(
            &stats,
            inner.x,
            inner.y + 186.0,
            fonts::SIZE_MD,
            colors::TEXT_PRIMARY,
        );

        // The meta payout runs on death exactly as it does on success — half
        // the fares bank, the lore keeps — but this screen never said so, and
        // an early death looked like it bought nothing.
        let payout = &game_state.shift_payout;
        let banked = format!("Banked: ${} | Lore: {}", payout.bank, payout.lore);
        draw_ui_text(
            &banked,
            inner.x,
            inner.y + 220.0,
            fonts::SIZE_MD,
            colors::ACCENT_SKY,
        );

        // The ending's authored paragraph — what this death was, in the
        // city's own voice.
        if let Some(ref epilogue) = game_state.epilogue {
            draw_wrapped_text(
                epilogue,
                inner.x,
                inner.y + 256.0,
                inner.w,
                fonts::SIZE_SM,
                19.0,
                colors::TEXT_SECONDARY,
                6,
            );
        }

        if draw_glass_button(
            UiRect::new(center_x - 150.0, panel.bottom() - 70.0, 300.0, 46.0),
            &data.localization.ui.common.try_again,
            colors::ACCENT_DANGER,
            true,
        ) {
            return UiAction::TryAgain;
        }
    }

    UiAction::None
}

/// Draw the success screen
pub fn draw_success(game_state: &GameState, game_data: Option<&GameData>) -> UiAction {
    draw_noir_city_background();
    if screen_width() < 980.0 || screen_height() < 560.0 {
        return draw_narrow_success(game_state, game_data);
    }
    let center_x = screen_width() / 2.0;

    if let Some(data) = game_data {
        // Tall enough for the meta-payout lines beneath the score; the run
        // summary carries one more than a night's does, and a completed run
        // carries its epilogue paragraph besides.
        let panel_h = if game_state.epilogue.is_some() {
            540.0
        } else if game_state.run_complete {
            430.0
        } else {
            400.0
        };
        let panel = UiRect::centered_x(screen_width(), 88.0, screen_width().min(560.0), panel_h);
        draw_glass_panel(panel, colors::ACCENT_GOLD);
        let inner = panel.inset(spacing::PADDING_LG);

        // Interim nights and the final run-victory show different framing.
        let interim = !game_state.run_complete;
        let nights_left = data
            .constants
            .game_constants
            .nights_per_run
            .saturating_sub(game_state.night);
        // Held open past the final ordinary night, the run has one fare left.
        let last_fare_awaits =
            interim && game_state.night >= data.constants.game_constants.nights_per_run;

        let title = if game_state.death_delivered {
            "THE LAST FARE".to_string()
        } else if interim {
            data.localization
                .ui
                .success
                .night_title
                .replace("{}", &game_state.night.to_string())
        } else {
            data.localization.ui.success.run_title.clone()
        };
        let title_size = 48.0;
        let title_width = measure_ui_text(&title, None, title_size as u16, 1.0).width;
        draw_ui_text(
            &title,
            center_x - title_width / 2.0,
            inner.y + 50.0,
            title_size,
            colors::ACCENT_GOLD,
        );

        let subtitle = if game_state.death_delivered {
            "You drove Death to his own door, and he tipped his hat. \
             Dawn belongs to you."
                .to_string()
        } else if last_fare_awaits {
            "Every soul in the almanac is known. The city has one more fare for you.".to_string()
        } else if interim {
            data.localization
                .ui
                .success
                .night_subtitle
                .replace("{}", &nights_left.to_string())
        } else {
            data.localization.ui.success.run_subtitle.clone()
        };
        let sub_width = measure_ui_text(&subtitle, None, 24, 1.0).width;
        draw_ui_text(
            &subtitle,
            center_x - sub_width / 2.0,
            inner.y + 92.0,
            24.0,
            colors::ACCENT_PRIMARY,
        );

        let y = inner.y + 142.0;

        let earnings_text = data
            .localization
            .ui
            .success
            .total_earnings
            .replace("{}", &game_state.earnings.to_string());
        draw_ui_text(&earnings_text, inner.x, y, 20.0, colors::ACCENT_GOLD);

        let rides_text = data
            .localization
            .ui
            .success
            .rides_completed
            .replace("{}", &game_state.rides_completed.to_string());
        draw_ui_text(&rides_text, inner.x, y + 30.0, 20.0, WHITE);

        let bonus_text = data.localization.ui.success.survival_bonus.replace(
            "{}",
            &data.constants.game_constants.survival_bonus.to_string(),
        );
        draw_ui_text(&bonus_text, inner.x, y + 60.0, 20.0, colors::FUEL_GOOD);

        let score_text = data.localization.ui.success.final_score.replace(
            "{}",
            &game_state.calculate_score(&data.constants).to_string(),
        );
        draw_ui_text(&score_text, inner.x, y + 100.0, 24.0, colors::ACCENT_DANGER);

        // What the night bought towards the next one. The bank and lore a
        // shift pays are what the skill tree and the almanac are bought with,
        // and the screen that ends the night never mentioned them — nor the
        // separate bonus a completed run pays, which has been credited
        // silently since it was wired.
        let payout = &game_state.shift_payout;
        let banked = format!(
            "Banked: ${} | Lore: {}",
            payout.bank + payout.run_bonus_bank,
            payout.lore + payout.run_bonus_lore
        );
        draw_ui_text(&banked, inner.x, y + 134.0, 20.0, colors::ACCENT_SKY);

        if payout.completed_a_run() {
            let bonus = format!(
                "  including ${} and {} lore for seeing the run out",
                payout.run_bonus_bank, payout.run_bonus_lore
            );
            draw_ui_text(&bonus, inner.x, y + 160.0, 16.0, colors::FUEL_GOOD);
        }

        // The run's authored epilogue — dawn in the city's own voice.
        if let Some(ref epilogue) = game_state.epilogue {
            draw_wrapped_text(
                epilogue,
                inner.x,
                y + 192.0,
                inner.w,
                fonts::SIZE_SM,
                19.0,
                colors::TEXT_SECONDARY,
                6,
            );
        }

        if interim {
            // One button: press on into the next night.
            if draw_glass_button(
                UiRect::new(center_x - 110.0, panel.bottom() - 62.0, 220.0, 42.0),
                &data.localization.ui.success.next_night,
                colors::ACCENT_GOLD,
                true,
            ) {
                return UiAction::NextNight;
            }
        } else {
            // Run complete: start a fresh run or bank progress at the menu.
            if draw_glass_button(
                UiRect::new(center_x - 210.0, panel.bottom() - 62.0, 200.0, 42.0),
                &data.localization.ui.common.try_again,
                colors::ACCENT_GOLD,
                true,
            ) {
                return UiAction::TryAgain;
            }
            if draw_glass_button(
                UiRect::new(center_x + 10.0, panel.bottom() - 62.0, 200.0, 42.0),
                &data.localization.ui.common.back_button,
                colors::ACCENT_PRIMARY,
                true,
            ) {
                return UiAction::ReturnToMenu;
            }
        }
    }

    UiAction::None
}

fn draw_narrow_game_over(game_state: &GameState, game_data: Option<&GameData>) -> UiAction {
    let Some(data) = game_data else {
        return UiAction::None;
    };
    let panel = UiRect::new(12.0, 10.0, screen_width() - 24.0, screen_height() - 20.0);
    draw_glass_panel(panel, colors::ACCENT_DANGER);
    draw_ui_text(
        &data.localization.ui.game_over.title,
        panel.x + 12.0,
        panel.y + 28.0,
        22.0,
        colors::FUEL_CRITICAL,
    );
    if let Some(reason) = game_state.game_over_reason.as_ref() {
        draw_wrapped_text(
            reason,
            panel.x + 12.0,
            panel.y + 50.0,
            panel.w - 24.0,
            fonts::SIZE_XS,
            11.0,
            colors::TEXT_SECONDARY,
            2,
        );
    }
    draw_small_caps(
        &format!(
            "EARNINGS ${}  |  RIDES {}  |  BANKED ${}",
            game_state.earnings, game_state.rides_completed, game_state.shift_payout.bank
        ),
        panel.x + 12.0,
        panel.y + 84.0,
        fonts::SIZE_XS,
        colors::ACCENT_SKY,
    );
    if draw_glass_button(
        UiRect::new(12.0, screen_height() - 34.0, screen_width() - 24.0, 26.0),
        &data.localization.ui.common.try_again,
        colors::ACCENT_DANGER,
        true,
    ) {
        return UiAction::TryAgain;
    }
    UiAction::None
}

fn draw_narrow_success(game_state: &GameState, game_data: Option<&GameData>) -> UiAction {
    let Some(data) = game_data else {
        return UiAction::None;
    };
    let panel = UiRect::new(12.0, 10.0, screen_width() - 24.0, screen_height() - 20.0);
    draw_glass_panel(panel, colors::ACCENT_GOLD);
    let title = if game_state.death_delivered {
        "THE LAST FARE"
    } else if game_state.run_complete {
        "SHIFT COMPLETE"
    } else {
        "NIGHT COMPLETE"
    };
    draw_ui_text(
        title,
        panel.x + 12.0,
        panel.y + 28.0,
        22.0,
        colors::ACCENT_GOLD,
    );
    draw_small_caps(
        &format!(
            "EARNINGS ${}  |  RIDES {}  |  BANKED ${}  |  LORE {}",
            game_state.earnings,
            game_state.rides_completed,
            game_state.shift_payout.bank,
            game_state.shift_payout.lore
        ),
        panel.x + 12.0,
        panel.y + 58.0,
        fonts::SIZE_XS,
        colors::TEXT_PRIMARY,
    );
    let (label, action) = if game_state.run_complete {
        (
            &data.localization.ui.common.back_button,
            UiAction::ReturnToMenu,
        )
    } else {
        (
            &data.localization.ui.success.next_night,
            UiAction::NextNight,
        )
    };
    if draw_glass_button(
        UiRect::new(12.0, screen_height() - 34.0, screen_width() - 24.0, 26.0),
        label,
        colors::ACCENT_GOLD,
        true,
    ) {
        return action;
    }
    UiAction::None
}

//! Nightmare Shift - A horror-themed taxi driving survival game.
//!
//! Drive supernatural passengers through the night, follow mysterious rules,
//! and try to survive until dawn.

use macroquad::prelude::*;
use macroquad_toolkit::capture;
use nightmare_shift::Game;

const DEFAULT_WINDOW_WIDTH: i32 = 1920;
const DEFAULT_WINDOW_HEIGHT: i32 = 1080;

fn window_conf() -> Conf {
    // Hide the window for capture runs, and for bot runs launched with
    // NIGHTMARE_SHIFT_HEADLESS=1. Armed here (not in main) so the window never
    // flashes onto the desktop while the game loads.
    capture::headless::arm("NIGHTMARE_SHIFT");

    // Built by hand (not capture::capture_window_conf) to keep sample_count: 0
    // and the always-off high_dpi that this game already relied on.
    Conf {
        window_title: "Nightmare Shift".to_string(),
        window_width: capture::env_i32("NIGHTMARE_SHIFT_WINDOW_WIDTH", DEFAULT_WINDOW_WIDTH),
        window_height: capture::env_i32("NIGHTMARE_SHIFT_WINDOW_HEIGHT", DEFAULT_WINDOW_HEIGHT),
        window_resizable: true,
        sample_count: 0,
        high_dpi: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    // First thing, before any data loads: a native panic leaves
    // crash_log.txt beside the save file instead of only a vanished window.
    macroquad_toolkit::crash::install_crash_log("nightmare_shift");

    let mut game = Game::new().await;

    // Screenshot harness: when NIGHTMARE_SHIFT_CAPTURE_PATH is set, seed a
    // scene, simulate deterministic frames, write a PNG, and exit.
    if let Some(configs) = capture::CaptureConfig::all_from_env("NIGHTMARE_SHIFT") {
        for config in configs {
            game.begin_capture_scene(&config.scene);
            capture::run_capture_once(&config, |_dt| {
                game.update();
                game.handle_input();
                let action = game.draw();
                game.handle_ui_action(action);
                game.handle_playtest_bot();
            })
            .await;
        }
        return;
    }

    loop {
        game.update();
        game.handle_input();
        let action = game.draw();
        game.handle_ui_action(action);
        game.handle_playtest_bot();
        next_frame().await;
    }
}

//! UI core types and utilities.

use macroquad::prelude::*;
use macroquad_toolkit::ui::button;
use std::{cell::Cell, cell::RefCell, collections::HashMap};

use super::icons::{draw_ui_icon, UiIcon};

thread_local! {
    static PASSENGER_PORTRAITS: RefCell<HashMap<u32, Texture2D>> = RefCell::new(HashMap::new());
    static COCKPIT_BACKGROUND: RefCell<Option<Texture2D>> = const { RefCell::new(None) };
    static PRESENTATION: Cell<Presentation> = const { Cell::new(Presentation::DEFAULT) };
    static BUTTON_CURSOR: Cell<usize> = const { Cell::new(0) };
    static BUTTON_COUNT: Cell<usize> = const { Cell::new(1) };
    static FOCUS_INDEX: Cell<usize> = const { Cell::new(0) };
    static FOCUS_VISIBLE: Cell<bool> = const { Cell::new(false) };
}

#[derive(Debug, Clone, Copy)]
struct Presentation {
    text_scale: f32,
    high_contrast: bool,
    reduced_motion: bool,
    brightness: f32,
}

impl Presentation {
    const DEFAULT: Self = Self {
        text_scale: 1.0,
        high_contrast: false,
        reduced_motion: false,
        brightness: 1.0,
    };
}

/// Apply the persisted presentation settings before drawing a frame.
pub fn set_presentation(settings: &crate::state::AccessibilitySettings) {
    PRESENTATION.with(|value| {
        value.set(Presentation {
            text_scale: settings.text_scale(),
            high_contrast: settings.high_contrast,
            reduced_motion: settings.reduced_motion,
            brightness: settings.brightness_percent.clamp(80, 120) as f32 / 100.0,
        });
    });
}

pub fn reduced_motion() -> bool {
    PRESENTATION.with(|value| value.get().reduced_motion)
}

pub fn brightness() -> f32 {
    PRESENTATION.with(|value| value.get().brightness)
}

/// Reset sequential keyboard focus before a frame's components are drawn.
/// Every shared button registers itself in draw order, which makes Tab and
/// arrow navigation available on all screens without parallel per-screen
/// focus tables that can drift from the visible layout.
pub fn begin_ui_frame() {
    let last_count = BUTTON_CURSOR.with(|cursor| {
        let count = cursor.get();
        cursor.set(0);
        count
    });
    if last_count > 0 {
        BUTTON_COUNT.with(|count| count.set(last_count));
        FOCUS_INDEX.with(|focus| focus.set(focus.get().min(last_count - 1)));
    }
    let forward = is_key_pressed(KeyCode::Tab)
        && !is_key_down(KeyCode::LeftShift)
        && !is_key_down(KeyCode::RightShift)
        || is_key_pressed(KeyCode::Down)
        || is_key_pressed(KeyCode::Right);
    let backward = (is_key_pressed(KeyCode::Tab)
        && (is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift)))
        || is_key_pressed(KeyCode::Up)
        || is_key_pressed(KeyCode::Left);
    if forward || backward {
        FOCUS_VISIBLE.with(|visible| visible.set(true));
        let count = BUTTON_COUNT.with(|value| value.get().max(1));
        FOCUS_INDEX.with(|focus| {
            focus.set(if backward {
                (focus.get() + count - 1) % count
            } else {
                (focus.get() + 1) % count
            });
        });
    }
}

fn register_button() -> (usize, bool) {
    let index = BUTTON_CURSOR.with(|cursor| {
        let index = cursor.get();
        cursor.set(index + 1);
        index
    });
    BUTTON_COUNT.with(|count| count.set(count.get().max(index + 1)));
    let focused = FOCUS_VISIBLE.with(|visible| visible.get())
        && FOCUS_INDEX.with(|focus| focus.get() == index);
    (index, focused)
}

fn accessible_color(color: Color) -> Color {
    PRESENTATION.with(|value| {
        if !value.get().high_contrast || color.a < 0.5 {
            return color;
        }
        let luminance = color.r * 0.2126 + color.g * 0.7152 + color.b * 0.0722;
        if luminance >= 0.58 {
            color
        } else {
            let lift = 0.72 / luminance.max(0.12);
            Color::new(
                (color.r * lift).min(1.0),
                (color.g * lift).min(1.0),
                (color.b * lift).min(1.0),
                color.a,
            )
        }
    })
}

pub fn draw_ui_text(text: &str, x: f32, y: f32, size: f32, color: Color) {
    let scale = PRESENTATION.with(|value| value.get().text_scale);
    macroquad_toolkit::ui::draw_ui_text(text, x, y, size * scale, accessible_color(color));
}

pub fn measure_ui_text(
    text: &str,
    font: Option<&Font>,
    font_size: u16,
    font_scale: f32,
) -> TextDimensions {
    let scale = PRESENTATION.with(|value| value.get().text_scale);
    macroquad_toolkit::ui::measure_ui_text(
        text,
        font,
        (font_size as f32 * scale).round() as u16,
        font_scale,
    )
}

/// Theme colors for the game.
pub mod colors {
    use macroquad::prelude::Color;

    pub const BLACK: Color = Color::new(0.015, 0.017, 0.016, 1.0);
    pub const INK: Color = Color::new(0.035, 0.045, 0.043, 1.0);
    pub const GLASS: Color = Color::new(0.030, 0.040, 0.040, 0.82);
    pub const GLASS_LIGHT: Color = Color::new(0.100, 0.115, 0.110, 0.76);
    pub const BORDER: Color = Color::new(0.310, 0.330, 0.310, 0.70);
    pub const BORDER_DIM: Color = Color::new(0.185, 0.200, 0.190, 0.65);

    pub const TEXT_PRIMARY: Color = Color::new(0.880, 0.850, 0.760, 1.0);
    pub const TEXT_SECONDARY: Color = Color::new(0.650, 0.640, 0.590, 1.0);
    pub const TEXT_MUTED: Color = Color::new(0.440, 0.450, 0.420, 1.0);

    pub const ACCENT_PRIMARY: Color = Color::new(0.220, 0.700, 0.330, 1.0);
    pub const ACCENT_DANGER: Color = Color::new(0.860, 0.200, 0.160, 1.0);
    pub const ACCENT_WARNING: Color = Color::new(0.930, 0.610, 0.080, 1.0);
    pub const ACCENT_GOLD: Color = Color::new(1.0, 0.670, 0.080, 1.0);
    pub const ACCENT_SKY: Color = Color::new(0.320, 0.620, 0.880, 1.0);
    pub const CAB_YELLOW: Color = Color::new(0.950, 0.620, 0.080, 1.0);
    pub const ROAD_REFLECT: Color = Color::new(0.740, 0.430, 0.080, 0.35);

    pub const FUEL_GOOD: Color = Color::new(0.330, 0.780, 0.240, 1.0);
    pub const FUEL_LOW: Color = Color::new(0.960, 0.640, 0.080, 1.0);
    pub const FUEL_CRITICAL: Color = Color::new(0.930, 0.180, 0.120, 1.0);
}

/// Standard spacing values
pub mod spacing {
    pub const PADDING_MD: f32 = 16.0;
    pub const PADDING_LG: f32 = 24.0;
}

/// Layout constants for consistent UI positioning
pub mod layout {
    // Status bar
    pub const STATUS_BAR_HEIGHT: f32 = 72.0;
}

/// Font sizes
pub mod fonts {
    pub const SIZE_XS: f32 = 12.0;
    pub const SIZE_SM: f32 = 14.0;
    pub const SIZE_MD: f32 = 16.0;
    pub const SIZE_LG: f32 = 20.0;
    pub const SIZE_XL: f32 = 24.0;
    pub const SIZE_XXL: f32 = 36.0;
}

/// A positioned rectangle for UI layouts (shared toolkit type).
pub use macroquad_toolkit::ui::UiRect;

pub fn draw_glass_panel(rect: UiRect, border: Color) {
    let surface = macroquad_toolkit::ui::SurfaceStyle::new(colors::GLASS)
        .with_top_highlight(1.0, Color::new(1.0, 1.0, 1.0, 0.12))
        .with_border(1.0, border);
    macroquad_toolkit::ui::draw_surface(rect.rect(), &surface);
}

pub fn draw_divider(x: f32, y: f32, h: f32) {
    draw_line(x, y, x, y + h, 1.0, colors::BORDER);
}

pub fn draw_small_caps(text: &str, x: f32, y: f32, size: f32, color: Color) {
    draw_ui_text(&text.to_uppercase(), x, y, size, color);
}

pub fn draw_glass_button(rect: UiRect, label: &str, accent: Color, enabled: bool) -> bool {
    let (_, focused) = register_button();
    let keyboard_pressed =
        focused && (is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter));
    let clicked = enabled && (button(rect.x, rect.y, rect.w, rect.h, "") || keyboard_pressed);
    let hovered = enabled && rect.contains_mouse();
    let bg = if hovered {
        colors::GLASS_LIGHT
    } else {
        colors::GLASS
    };
    let border = if enabled { accent } else { colors::BORDER_DIM };

    let surface = macroquad_toolkit::ui::SurfaceStyle::new(bg)
        .with_left_accent(4.0, border)
        .with_top_highlight(1.0, Color::new(1.0, 1.0, 1.0, 0.10))
        .with_border(1.0, border);
    macroquad_toolkit::ui::draw_surface(rect.rect(), &surface);
    if focused {
        draw_rectangle_lines(
            rect.x - 2.0,
            rect.y - 2.0,
            rect.w + 4.0,
            rect.h + 4.0,
            3.0,
            colors::TEXT_PRIMARY,
        );
    }

    let text_color = if enabled {
        colors::TEXT_PRIMARY
    } else {
        colors::TEXT_MUTED
    };
    let dims = measure_ui_text(label, None, fonts::SIZE_MD as u16, 1.0);
    draw_ui_text(
        label,
        rect.x + (rect.w - dims.width) / 2.0,
        rect.y + rect.h / 2.0 + dims.height / 2.0 - 2.0,
        fonts::SIZE_MD,
        text_color,
    );
    clicked
}

pub fn draw_wrapped_text(
    text: &str,
    x: f32,
    mut y: f32,
    max_width: f32,
    size: f32,
    line_height: f32,
    color: Color,
    max_lines: usize,
) -> f32 {
    let scale = PRESENTATION.with(|value| value.get().text_scale);
    // The shared wrapper measures/draws scaled glyphs, so wrapping must use
    // the corresponding unscaled width. Otherwise 125% text chooses the same
    // line breaks as 100% and spills across the next panel.
    let logical_width = max_width / scale.max(1.0);
    let mut lines = macroquad_toolkit::ui::wrap_text(text, logical_width, size);
    if max_lines > 0 && lines.len() > max_lines {
        lines.truncate(max_lines);
        if let Some(last) = lines.last_mut() {
            *last = macroquad_toolkit::ui::truncate_text_to_width(
                &format!("{last}..."),
                logical_width,
                size,
            );
        }
    }

    for line in lines {
        draw_ui_text(&line, x, y, size, color);
        y += line_height * scale;
    }

    y
}

pub fn draw_stat_block(icon: UiIcon, value: &str, label: &str, x: f32, y: f32, color: Color) {
    draw_ui_icon(icon, x + 10.0, y + 14.0, 20.0, color);
    draw_ui_text(value, x + 26.0, y + 16.0, fonts::SIZE_LG, color);
    draw_small_caps(
        label,
        x + 26.0,
        y + 34.0,
        fonts::SIZE_XS,
        colors::TEXT_MUTED,
    );
}

pub fn draw_noir_city_background() {
    let w = screen_width();
    let h = screen_height();
    clear_background(colors::BLACK);
    draw_rectangle(0.0, 0.0, w, h, colors::INK);

    // Wet street and distant buildings.
    draw_rectangle(
        0.0,
        h * 0.54,
        w,
        h * 0.46,
        Color::new(0.025, 0.030, 0.028, 1.0),
    );
    for i in 0..8 {
        let x = i as f32 * w / 8.0;
        let bh = 145.0 + ((i * 31) % 90) as f32;
        draw_rectangle(
            x,
            h * 0.54 - bh,
            w / 9.0,
            bh,
            Color::new(0.020, 0.026, 0.026, 1.0),
        );
        if i % 2 == 0 {
            draw_rectangle(
                x + 18.0,
                h * 0.54 - 62.0,
                12.0,
                18.0,
                Color::new(0.800, 0.480, 0.120, 0.20),
            );
        }
    }

    // Street lamps and reflections.
    for i in 0..4 {
        let x = w * (0.42 + i as f32 * 0.14);
        let y = h * (0.18 + i as f32 * 0.08);
        draw_line(
            x,
            y,
            x - 12.0,
            h * 0.58,
            1.0,
            Color::new(0.28, 0.28, 0.24, 0.45),
        );
        draw_circle(x, y, 7.0, Color::new(1.0, 0.72, 0.30, 0.65));
        draw_circle(x, y, 26.0, Color::new(0.95, 0.55, 0.12, 0.08));
        draw_rectangle(x - 6.0, h * 0.58, 12.0, h * 0.33, colors::ROAD_REFLECT);
    }

    // Taxi silhouette.
    let tx = w * 0.07;
    let ty = h * 0.66;
    draw_rectangle(
        tx,
        ty,
        w * 0.29,
        h * 0.13,
        Color::new(0.38, 0.24, 0.04, 0.92),
    );
    draw_rectangle(
        tx + 38.0,
        ty - 42.0,
        w * 0.17,
        48.0,
        Color::new(0.26, 0.17, 0.04, 0.92),
    );
    draw_rectangle(tx + 44.0, ty - 60.0, 80.0, 20.0, colors::CAB_YELLOW);
    draw_ui_text("TAXI", tx + 55.0, ty - 45.0, fonts::SIZE_LG, colors::BLACK);
    draw_circle(tx + 52.0, ty + 76.0, 30.0, colors::BLACK);
    draw_circle(tx + w * 0.25, ty + 76.0, 30.0, colors::BLACK);
    draw_rectangle(
        tx + 14.0,
        ty + 34.0,
        76.0,
        22.0,
        Color::new(0.95, 0.10, 0.04, 0.55),
    );

    // Rain streaks.
    for i in 0..80 {
        let x = ((i * 73) % 800) as f32 / 800.0 * w;
        let y = ((i * 47) % 600) as f32 / 600.0 * h;
        draw_line(
            x,
            y,
            x - 5.0,
            y + 28.0,
            1.0,
            Color::new(0.55, 0.62, 0.66, 0.15),
        );
    }

    draw_rectangle(0.0, 0.0, w, h, Color::new(0.0, 0.0, 0.0, 0.25));
}

pub fn draw_cockpit_background() {
    let w = screen_width();
    let h = screen_height();
    if crate::ui::backgrounds::draw_active_driving_background() {
        draw_rectangle(0.0, 0.0, w, h, Color::new(0.0, 0.0, 0.0, 0.30));
        return;
    }
    COCKPIT_BACKGROUND.with(|cached| {
        let mut cached = cached.borrow_mut();
        if cached.is_none() {
            let texture = Texture2D::from_file_with_format(
                include_bytes!("../../assets/ui/cockpit_background.png"),
                Some(ImageFormat::Png),
            );
            texture.set_filter(FilterMode::Linear);
            *cached = Some(texture);
        }
        let texture = cached.as_ref().expect("cockpit texture cached");
        let tex_w = texture.width();
        let tex_h = texture.height();
        let dest_aspect = w / h.max(1.0);
        let tex_aspect = tex_w / tex_h.max(1.0);
        let source = if dest_aspect > tex_aspect {
            let src_h = tex_w / dest_aspect;
            Rect::new(0.0, (tex_h - src_h) / 2.0, tex_w, src_h)
        } else {
            let src_w = tex_h * dest_aspect;
            Rect::new((tex_w - src_w) / 2.0, 0.0, src_w, tex_h)
        };
        draw_texture_ex(
            texture,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(w, h)),
                source: Some(source),
                ..Default::default()
            },
        );
    });
    draw_rectangle(0.0, 0.0, w, h, Color::new(0.0, 0.0, 0.0, 0.30));
}

fn passenger_portrait_bytes(id: u32) -> Option<&'static [u8]> {
    match id {
        1 => Some(include_bytes!("../../assets/passengers/1.png")),
        2 => Some(include_bytes!("../../assets/passengers/2.png")),
        3 => Some(include_bytes!("../../assets/passengers/3.png")),
        4 => Some(include_bytes!("../../assets/passengers/4.png")),
        5 => Some(include_bytes!("../../assets/passengers/5.png")),
        6 => Some(include_bytes!("../../assets/passengers/6.png")),
        7 => Some(include_bytes!("../../assets/passengers/7.png")),
        8 => Some(include_bytes!("../../assets/passengers/8.png")),
        9 => Some(include_bytes!("../../assets/passengers/9.png")),
        10 => Some(include_bytes!("../../assets/passengers/10.png")),
        11 => Some(include_bytes!("../../assets/passengers/11.png")),
        12 => Some(include_bytes!("../../assets/passengers/12.png")),
        13 => Some(include_bytes!("../../assets/passengers/13.png")),
        14 => Some(include_bytes!("../../assets/passengers/14.png")),
        15 => Some(include_bytes!("../../assets/passengers/15.png")),
        16 => Some(include_bytes!("../../assets/passengers/16.png")),
        17 => Some(include_bytes!("../../assets/passengers/17.png")),
        _ => None,
    }
}

fn passenger_portrait_texture(id: u32) -> Option<Texture2D> {
    PASSENGER_PORTRAITS.with(|portraits| {
        let mut portraits = portraits.borrow_mut();
        if let Some(texture) = portraits.get(&id) {
            return Some(texture.clone());
        }

        let bytes = passenger_portrait_bytes(id)?;
        let texture = Texture2D::from_file_with_format(bytes, Some(ImageFormat::Png));
        texture.set_filter(FilterMode::Linear);
        portraits.insert(id, texture.clone());
        Some(texture)
    })
}

pub fn draw_passenger_portrait(rect: UiRect, seed: u32) {
    if let Some(texture) = passenger_portrait_texture(seed) {
        let tex_w = texture.width();
        let tex_h = texture.height();
        let dest_aspect = rect.w / rect.h.max(1.0);
        let tex_aspect = tex_w / tex_h.max(1.0);
        let source = if dest_aspect > tex_aspect {
            let src_h = tex_w / dest_aspect;
            Rect::new(0.0, (tex_h - src_h) / 2.0, tex_w, src_h)
        } else {
            let src_w = tex_h * dest_aspect;
            Rect::new((tex_w - src_w) / 2.0, 0.0, src_w, tex_h)
        };

        draw_texture_ex(
            &texture,
            rect.x,
            rect.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(rect.w, rect.h)),
                source: Some(source),
                ..Default::default()
            },
        );
        draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            Color::new(0.0, 0.0, 0.0, 0.18),
        );
        draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, colors::BORDER);
        return;
    }

    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        Color::new(0.030, 0.045, 0.048, 1.0),
    );
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        Color::new(0.0, 0.0, 0.0, 0.20),
    );

    for i in 0..6 {
        let bx = rect.x + 14.0 + i as f32 * (rect.w - 28.0) / 6.0;
        let bh = 30.0 + ((seed + i * 17) % 65) as f32;
        draw_rectangle(
            bx,
            rect.y + rect.h - bh,
            14.0,
            bh,
            Color::new(0.070, 0.085, 0.085, 1.0),
        );
        draw_rectangle(
            bx + 3.0,
            rect.y + rect.h - bh + 8.0,
            4.0,
            8.0,
            Color::new(0.90, 0.56, 0.18, 0.20),
        );
    }

    let cx = rect.center().x;
    let cy = rect.y + rect.h * 0.48;
    draw_circle(cx, cy - 25.0, 28.0, Color::new(0.420, 0.360, 0.300, 1.0));
    draw_circle(cx, cy - 34.0, 31.0, Color::new(0.050, 0.045, 0.043, 1.0));
    draw_rectangle(
        cx - 41.0,
        cy + 4.0,
        82.0,
        74.0,
        Color::new(0.045, 0.056, 0.062, 1.0),
    );
    draw_triangle(
        Vec2::new(cx - 16.0, cy + 1.0),
        Vec2::new(cx + 16.0, cy + 1.0),
        Vec2::new(cx, cy + 34.0),
        Color::new(0.650, 0.620, 0.560, 0.40),
    );
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        Color::new(0.0, 0.0, 0.0, 0.28),
    );
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, colors::BORDER);
}

/// Get color for fuel level.
///
/// The thresholds live in `constants.json` under `FUEL`. They used to be
/// duplicated as `layout::FUEL_*_THRESHOLD` constants that happened to match,
/// so editing the authored values — the file the data-driven rule tells you
/// to edit — changed the numbers the game reasoned with but not the ones it
/// coloured the gauge by.
pub fn get_fuel_color(fuel: f32, fuel_constants: &crate::data::FuelConstants) -> Color {
    if fuel <= fuel_constants.critical_fuel as f32 {
        colors::FUEL_CRITICAL // Red - Critical
    } else if fuel <= fuel_constants.low_fuel_warning as f32 {
        colors::FUEL_LOW // Red/Orange - Low
    } else if fuel <= fuel_constants.medium_fuel as f32 {
        colors::ACCENT_WARNING // Yellow - Medium
    } else {
        colors::FUEL_GOOD // Green - Good
    }
}

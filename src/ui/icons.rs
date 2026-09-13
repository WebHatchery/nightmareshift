//! Code-native status icons used by the responsive UI.

use macroquad::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiIcon {
    Fuel,
    Time,
    Fare,
    Risk,
    Weather,
    Rules,
    Inventory,
    Wards,
    Lore,
    Cab,
    Rides,
}

/// Small code-native icon atlas; it avoids platform-dependent emoji glyphs
/// while keeping the same symbols available to every responsive layout.
pub fn draw_ui_icon(icon: UiIcon, cx: f32, cy: f32, size: f32, color: Color) {
    let scale = size / 24.0;
    let stroke = (1.8 * scale).max(1.0);
    match icon {
        UiIcon::Fuel => draw_fuel(cx, cy, scale, stroke, color),
        UiIcon::Time => draw_time(cx, cy, scale, stroke, color),
        UiIcon::Fare => draw_fare(cx, cy, scale, stroke, color),
        UiIcon::Risk => draw_risk(cx, cy, scale, stroke, color),
        UiIcon::Weather => draw_weather(cx, cy, scale, stroke, color),
        UiIcon::Rules => draw_rules(cx, cy, scale, stroke, color),
        UiIcon::Inventory => draw_inventory(cx, cy, scale, stroke, color),
        UiIcon::Wards => draw_wards(cx, cy, scale, stroke, color),
        UiIcon::Lore => draw_lore(cx, cy, scale, stroke, color),
        UiIcon::Cab | UiIcon::Rides => draw_cab(cx, cy, scale, stroke, color),
    }
}

fn draw_fuel(cx: f32, cy: f32, s: f32, stroke: f32, color: Color) {
    draw_rectangle_lines(
        cx - 8.0 * s,
        cy - 10.0 * s,
        12.0 * s,
        20.0 * s,
        stroke,
        color,
    );
    draw_line(
        cx - 6.0 * s,
        cy - 5.0 * s,
        cx + 2.0 * s,
        cy - 5.0 * s,
        stroke,
        color,
    );
    draw_line(
        cx + 4.0 * s,
        cy - 7.0 * s,
        cx + 9.0 * s,
        cy - 2.0 * s,
        stroke,
        color,
    );
    draw_line(
        cx + 9.0 * s,
        cy - 2.0 * s,
        cx + 9.0 * s,
        cy + 8.0 * s,
        stroke,
        color,
    );
}

fn draw_time(cx: f32, cy: f32, s: f32, stroke: f32, color: Color) {
    draw_circle_lines(cx, cy, 10.0 * s, stroke, color);
    draw_line(cx, cy, cx, cy - 6.0 * s, stroke, color);
    draw_line(cx, cy, cx + 5.0 * s, cy + 3.0 * s, stroke, color);
}

fn draw_fare(cx: f32, cy: f32, s: f32, stroke: f32, color: Color) {
    draw_circle_lines(cx, cy, 10.0 * s, stroke, color);
    draw_line(cx, cy - 7.0 * s, cx, cy + 7.0 * s, stroke, color);
    draw_line(
        cx - 4.0 * s,
        cy - 4.0 * s,
        cx + 4.0 * s,
        cy - 4.0 * s,
        stroke,
        color,
    );
    draw_line(
        cx - 4.0 * s,
        cy + 4.0 * s,
        cx + 4.0 * s,
        cy + 4.0 * s,
        stroke,
        color,
    );
}

fn draw_risk(cx: f32, cy: f32, s: f32, stroke: f32, color: Color) {
    draw_triangle_lines(
        Vec2::new(cx, cy - 11.0 * s),
        Vec2::new(cx - 11.0 * s, cy + 9.0 * s),
        Vec2::new(cx + 11.0 * s, cy + 9.0 * s),
        stroke,
        color,
    );
    draw_line(cx, cy - 4.0 * s, cx, cy + 3.0 * s, stroke, color);
    draw_circle(cx, cy + 6.0 * s, 1.4 * s, color);
}

fn draw_weather(cx: f32, cy: f32, s: f32, stroke: f32, color: Color) {
    draw_circle_lines(cx - 4.0 * s, cy, 6.0 * s, stroke, color);
    draw_circle_lines(cx + 3.0 * s, cy - 3.0 * s, 7.0 * s, stroke, color);
    draw_line(
        cx - 9.0 * s,
        cy + 5.0 * s,
        cx + 10.0 * s,
        cy + 5.0 * s,
        stroke,
        color,
    );
    draw_line(
        cx - 4.0 * s,
        cy + 8.0 * s,
        cx - 6.0 * s,
        cy + 12.0 * s,
        stroke,
        color,
    );
    draw_line(
        cx + 4.0 * s,
        cy + 8.0 * s,
        cx + 2.0 * s,
        cy + 12.0 * s,
        stroke,
        color,
    );
}

fn draw_rules(cx: f32, cy: f32, s: f32, stroke: f32, color: Color) {
    draw_rectangle_lines(
        cx - 8.0 * s,
        cy - 10.0 * s,
        16.0 * s,
        20.0 * s,
        stroke,
        color,
    );
    for row in [-5.0, 0.0, 5.0] {
        draw_line(
            cx - 4.0 * s,
            cy + row * s,
            cx + 5.0 * s,
            cy + row * s,
            stroke,
            color,
        );
    }
}

fn draw_inventory(cx: f32, cy: f32, s: f32, stroke: f32, color: Color) {
    draw_rectangle_lines(
        cx - 9.0 * s,
        cy - 5.0 * s,
        18.0 * s,
        14.0 * s,
        stroke,
        color,
    );
    draw_arc(cx, cy - 5.0 * s, 8, 180.0, 180.0, 5.0 * s, stroke, color);
}

fn draw_wards(cx: f32, cy: f32, s: f32, stroke: f32, color: Color) {
    draw_triangle_lines(
        Vec2::new(cx, cy + 11.0 * s),
        Vec2::new(cx - 9.0 * s, cy - 8.0 * s),
        Vec2::new(cx + 9.0 * s, cy - 8.0 * s),
        stroke,
        color,
    );
    draw_line(cx, cy - 6.0 * s, cx, cy + 6.0 * s, stroke, color);
}

fn draw_lore(cx: f32, cy: f32, s: f32, stroke: f32, color: Color) {
    draw_rectangle_lines(
        cx - 10.0 * s,
        cy - 8.0 * s,
        9.0 * s,
        17.0 * s,
        stroke,
        color,
    );
    draw_rectangle_lines(cx + 1.0 * s, cy - 8.0 * s, 9.0 * s, 17.0 * s, stroke, color);
    draw_line(cx, cy - 8.0 * s, cx, cy + 9.0 * s, stroke, color);
}

fn draw_cab(cx: f32, cy: f32, s: f32, stroke: f32, color: Color) {
    draw_rectangle_lines(
        cx - 10.0 * s,
        cy - 3.0 * s,
        20.0 * s,
        9.0 * s,
        stroke,
        color,
    );
    draw_line(
        cx - 6.0 * s,
        cy - 3.0 * s,
        cx - 2.0 * s,
        cy - 8.0 * s,
        stroke,
        color,
    );
    draw_line(
        cx - 2.0 * s,
        cy - 8.0 * s,
        cx + 6.0 * s,
        cy - 3.0 * s,
        stroke,
        color,
    );
    draw_circle(cx - 6.0 * s, cy + 7.0 * s, 2.5 * s, color);
    draw_circle(cx + 6.0 * s, cy + 7.0 * s, 2.5 * s, color);
}

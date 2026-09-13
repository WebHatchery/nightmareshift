//! Small authored item illustrations used wherever inventory objects are shown.

use macroquad::prelude::*;
use std::{cell::RefCell, collections::HashMap};

use super::{colors, UiRect};

thread_local! {
    static ITEM_TEXTURES: RefCell<HashMap<bool, Texture2D>> = RefCell::new(HashMap::new());
}

/// Draw the best available illustration for an item. The generic asset is a
/// deliberate fallback because most of the catalogue is text-authored.
pub fn draw_item_art(rect: UiRect, item_name: &str) {
    let is_locket = item_name.eq_ignore_ascii_case("old locket");
    ITEM_TEXTURES.with(|cached| {
        let mut cached = cached.borrow_mut();
        let texture = cached.entry(is_locket).or_insert_with(|| {
            let bytes: &[u8] = if is_locket {
                include_bytes!("../../assets/items/item_locket.png")
            } else {
                include_bytes!("../../assets/items/item_generic.png")
            };
            let texture = Texture2D::from_file_with_format(bytes, Some(ImageFormat::Png));
            texture.set_filter(FilterMode::Linear);
            texture
        });
        draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            Color::new(0.02, 0.025, 0.025, 0.92),
        );
        draw_texture_ex(
            texture,
            rect.x,
            rect.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(rect.w, rect.h)),
                ..Default::default()
            },
        );
        draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, colors::BORDER_DIM);
    });
}

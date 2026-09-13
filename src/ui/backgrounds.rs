//! Painted menu backgrounds that are independent of gameplay state.

use macroquad::prelude::*;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
};

thread_local! {
    static TITLE_BACKGROUND: RefCell<Option<Texture2D>> = const { RefCell::new(None) };
    static LOGO: RefCell<Option<Texture2D>> = const { RefCell::new(None) };
    static DRIVING_BACKGROUNDS: RefCell<HashMap<DrivingBackdrop, Texture2D>> =
        RefCell::new(HashMap::new());
    static ACTIVE_DRIVING_BACKDROP: Cell<DrivingBackdrop> =
        const { Cell::new(DrivingBackdrop::City) };
}

/// Authored scene family used by the current pickup or destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DrivingBackdrop {
    City,
    Forest,
    Industrial,
}

impl DrivingBackdrop {
    /// Select a scene without making the renderer depend on location IDs.
    pub fn for_location(location: Option<&str>) -> Self {
        let location = location.unwrap_or_default().to_ascii_lowercase();
        if location.contains("forest")
            || location.contains("cemetery")
            || location.contains("cathedral")
            || location.contains("seminary")
        {
            Self::Forest
        } else if location.contains("industrial")
            || location.contains("warehouse")
            || location.contains("harbor")
            || location.contains("crime")
        {
            Self::Industrial
        } else {
            Self::City
        }
    }
}

/// Set the driving scene before a game screen draws its background.
pub fn set_active_driving_backdrop(backdrop: DrivingBackdrop) {
    ACTIVE_DRIVING_BACKDROP.with(|active| active.set(backdrop));
}

/// Draw the selected authored driving scene.
pub fn draw_active_driving_background() -> bool {
    let backdrop = ACTIVE_DRIVING_BACKDROP.with(Cell::get);
    let texture = DRIVING_BACKGROUNDS.with(|cached| {
        let mut cached = cached.borrow_mut();
        let texture = cached.entry(backdrop).or_insert_with(|| {
            let bytes: &[u8] = match backdrop {
                DrivingBackdrop::City => include_bytes!("../../assets/ui/bg_driving_city.png"),
                DrivingBackdrop::Forest => include_bytes!("../../assets/ui/bg_driving_forest.png"),
                DrivingBackdrop::Industrial => {
                    include_bytes!("../../assets/ui/bg_driving_industrial.png")
                }
            };
            let texture = Texture2D::from_file_with_format(bytes, Some(ImageFormat::Png));
            texture.set_filter(FilterMode::Linear);
            texture
        });
        texture.clone()
    });
    draw_texture_cover(&texture, screen_width(), screen_height());
    true
}

/// Draw the title logo without asking a platform font to reproduce its lettering.
pub fn draw_ui_logo(rect: Rect) {
    LOGO.with(|cached| {
        let mut cached = cached.borrow_mut();
        if cached.is_none() {
            let texture = Texture2D::from_file_with_format(
                include_bytes!("../../assets/ui/ui_logo.png"),
                Some(ImageFormat::Png),
            );
            texture.set_filter(FilterMode::Linear);
            *cached = Some(texture);
        }
        draw_texture_ex(
            cached.as_ref().expect("logo texture cached"),
            rect.x,
            rect.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(rect.w, rect.h)),
                ..Default::default()
            },
        );
    });
}

fn draw_texture_cover(texture: &Texture2D, width: f32, height: f32) {
    let tex_w = texture.width();
    let tex_h = texture.height();
    let dest_aspect = width / height.max(1.0);
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
            dest_size: Some(vec2(width, height)),
            source: Some(source),
            ..Default::default()
        },
    );
}

/// Draw the original title key art as a full-bleed, aspect-cropped backdrop.
/// The texture is embedded so it is available before the asset archive loads.
pub fn draw_title_background() {
    let w = screen_width();
    let h = screen_height();
    TITLE_BACKGROUND.with(|cached| {
        let mut cached = cached.borrow_mut();
        if cached.is_none() {
            let texture = Texture2D::from_file_with_format(
                include_bytes!("../../assets/ui/title_background.png"),
                Some(ImageFormat::Png),
            );
            texture.set_filter(FilterMode::Linear);
            *cached = Some(texture);
        }
        let texture = cached.as_ref().expect("title texture cached");
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
    draw_rectangle(0.0, 0.0, w, h, Color::new(0.0, 0.0, 0.0, 0.24));
    draw_rectangle(0.0, 0.0, w * 0.58, h, Color::new(0.0, 0.0, 0.0, 0.20));
}

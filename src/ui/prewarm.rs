//! Prepare the default-font atlas before the first queued draw on WebGL.

/// Rasterize the UI character set at every authored type size before drawing.
/// Macroquad grows its font atlas by replacing a texture; doing that midway
/// through the first rendered frame can leave earlier text commands pointing
/// at the retired WebGL texture until the batch flushes.
pub fn prewarm_ui_glyphs() {
    const GLYPHS: &str =
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyzÁÉÍÓÚÜÑáéíóúüñ¿¡0123456789 $%+-:;,.!?/()[]#'→•×";
    for size in [12_u16, 14, 16, 20, 24, 28, 36, 44, 72] {
        let _ = macroquad_toolkit::ui::measure_ui_text(GLYPHS, None, size, 1.0);
    }
}

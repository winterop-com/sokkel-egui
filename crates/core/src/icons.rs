//! Lucide, the one icon set, as sokkel-ui uses it through lucide-react: drawn from Lucide's icon
//! font in a family of its own.

use egui::{Align2, Color32, Context, FontFamily, FontId, Painter, Pos2, RichText};
pub use lucide_icons::Icon;

/// The font family Lucide is installed under.
pub const FAMILY: &str = "lucide";

/// Lucide at `size` points.
pub fn font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(FAMILY.into()))
}

/// An icon as text, for a label or a button's text. Before the icon font is ready (the first frame
/// after `install`) check [`ready`], or paint with [`paint`], which waits by itself.
pub fn rich(icon: Icon, size: f32) -> RichText {
    RichText::new(char::from(icon).to_string()).font(font(size))
}

/// An icon by its Lucide name in kebab case, such as `circle-check`.
pub fn by_name(name: &str) -> Option<Icon> {
    Icon::try_from(name).ok()
}

/// Whether the context's fonts hold Lucide yet: fonts set by `install` reach the painter on the
/// frame after, so an icon in the first frame is skipped rather than drawn from a missing font.
pub fn ready(ctx: &Context) -> bool {
    ctx.fonts(|fonts| {
        fonts
            .definitions()
            .families
            .contains_key(&FontFamily::Name(FAMILY.into()))
    })
}

/// Paint an icon centered on `center`; nothing on a frame before the icon font is ready.
pub fn paint(painter: &Painter, center: Pos2, icon: Icon, size: f32, color: Color32) {
    if !ready(painter.ctx()) {
        painter.ctx().request_repaint();
        return;
    }
    painter.text(
        center,
        Align2::CENTER_CENTER,
        char::from(icon).to_string(),
        font(size),
        color,
    );
}

use std::collections::BTreeMap;

use eframe::{
    CreationContext,
    egui::{FontFamily, FontId, ImageSource, TextStyle, Ui, Vec2, include_image, vec2},
};
use egui::{Color32, Stroke, widget_style::SeparatorStyle};

#[macro_export]
macro_rules! set_font {
    ($cc:expr, $path:expr) => {
        use std::ffi::OsStr;
        let name = Path::new($path)
            .file_name()
            .unwrap_or(&OsStr::new($path))
            .to_string_lossy()
            .into_owned();
        let mut fonts = FontDefinitions::default();
        fonts.font_data.insert(
            name.clone(),
            Arc::new(FontData::from_static(include_bytes!($path))),
        );
        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .expect("No Proportional fonts found.")
            .insert(0, name.clone());
        fonts
            .families
            .get_mut(&FontFamily::Monospace)
            .expect("No Proportional fonts found.")
            .insert(0, name.clone());

        $cc.egui_ctx.set_fonts(fonts);
    };
}

pub const DEFAULT_IMAGE: ImageSource<'static> = include_image!("../assets/heart_256.ico");
pub const TILE_SIZE: Vec2 = vec2(128.0, 128.0);
pub const BUTTON_BORDER_WIDTH: f32 = 4.;
pub const BUTTON_PADDING: Vec2 = Vec2::splat(BUTTON_BORDER_WIDTH * 4.);
pub const MOD_TILE_IMAGE_PADDING: f32 = 16.0;
pub const SEPARATOR_STYLE: SeparatorStyle = SeparatorStyle {
    spacing: 8.0,
    stroke: Stroke {
        width: 8.0,
        color: Color32::RED,
    },
};

pub const INFO_PANEL_BOTTOM_HEIGHT: f32 = 32.;
pub const INFO_PANEL_WIDTH: f32 = 192.0;

const FONT_SIZE_DIFFERENCE: f32 = 0.70;

#[inline]
pub fn popup_size(ui: &Ui) -> Vec2 {
    ui.content_rect().size() * 0.75
}

pub fn set_font_settings(cc: &CreationContext, size: f32, style: FontFamily) {
    let text_styles: BTreeMap<TextStyle, FontId> = [
        (
            TextStyle::Small,
            FontId::new(size * FONT_SIZE_DIFFERENCE, style.clone()),
        ),
        (TextStyle::Body, FontId::new(size, style.clone())),
        (TextStyle::Button, FontId::new(size, style.clone())),
        (
            TextStyle::Heading,
            FontId::new(size * (1.0 + FONT_SIZE_DIFFERENCE), style),
        ),
        (
            TextStyle::Monospace,
            FontId::new(size, FontFamily::Monospace),
        ),
    ]
    .into();
    cc.egui_ctx
        .all_styles_mut(|style| style.text_styles = text_styles.clone());
}

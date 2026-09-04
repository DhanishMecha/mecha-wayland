pub mod default_atlas {
    include!(concat!(env!("OUT_DIR"), "/widget_fonts_gen.rs"));
}

use assets::BakedFont;
use theme::TextVariant;

pub fn default_font_for_role(variant: TextVariant) -> &'static BakedFont {
    match variant {
        TextVariant::DisplayLarge => &default_atlas::WIDGET_FONTS_FONT_SPACE_GROTESK_57,
        TextVariant::DisplayMedium => &default_atlas::WIDGET_FONTS_FONT_SPACE_GROTESK_45,
        TextVariant::DisplaySmall => &default_atlas::WIDGET_FONTS_FONT_SPACE_GROTESK_36,
        TextVariant::HeadlineLarge => &default_atlas::WIDGET_FONTS_FONT_SPACE_GROTESK_32,
        TextVariant::HeadlineMedium => &default_atlas::WIDGET_FONTS_FONT_SPACE_GROTESK_28,
        TextVariant::HeadlineSmall => &default_atlas::WIDGET_FONTS_FONT_SPACE_GROTESK_24,
        TextVariant::TitleLarge => &default_atlas::WIDGET_FONTS_FONT_SPACE_GROTESK_22,
        TextVariant::TitleMedium => &default_atlas::WIDGET_FONTS_FONT_SPACE_GROTESK_16,
        TextVariant::TitleSmall => &default_atlas::WIDGET_FONTS_FONT_SPACE_GROTESK_14,
        TextVariant::BodyLarge => &default_atlas::WIDGET_FONTS_FONT_SPACE_GROTESK_16,
        TextVariant::BodyMedium => &default_atlas::WIDGET_FONTS_FONT_SPACE_GROTESK_14,
        TextVariant::BodySmall => &default_atlas::WIDGET_FONTS_FONT_SPACE_GROTESK_12,
        TextVariant::LabelLarge => &default_atlas::WIDGET_FONTS_FONT_SPACE_GROTESK_14,
        TextVariant::LabelMedium => &default_atlas::WIDGET_FONTS_FONT_SPACE_GROTESK_12,
        TextVariant::LabelSmall => &default_atlas::WIDGET_FONTS_FONT_SPACE_GROTESK_11,
    }
}

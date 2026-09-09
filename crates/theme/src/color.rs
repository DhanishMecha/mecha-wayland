use utils::Color;

//  ColorScheme
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorScheme {
    //  Primary
    pub primary: Color,
    pub on_primary: Color,
    pub primary_container: Color,
    pub on_primary_container: Color,

    //  Secondary
    pub secondary: Color,
    pub on_secondary: Color,
    pub secondary_container: Color,
    pub on_secondary_container: Color,
    pub secondary_fixed_dim: Color,

    //  Tertiary
    pub tertiary: Color,
    pub on_tertiary: Color,
    pub tertiary_container: Color,
    pub on_tertiary_container: Color,

    //  Error
    pub error: Color,
    pub on_error: Color,
    pub error_container: Color,
    pub on_error_container: Color,

    //  Background
    pub background: Color,
    pub on_background: Color,

    //  Surface
    pub surface: Color,
    pub on_surface: Color,
    pub surface_variant: Color,
    pub on_surface_variant: Color,

    //  Surface containers (M3 extended variant)
    pub surface_dim: Color,
    pub surface_bright: Color,
    pub surface_container_lowest: Color,
    pub surface_container_low: Color,
    pub surface_container: Color,
    pub surface_container_high: Color,
    pub surface_container_highest: Color,

    //  Outline
    pub outline: Color,
    pub outline_variant: Color,

    //  Inverse
    pub inverse_surface: Color,
    pub inverse_on_surface: Color,
    pub inverse_primary: Color,

    //  Utility
    pub scrim: Color,
    pub shadow: Color,
}

impl ColorScheme {
    /// Comet  baseline **light** color scheme.
    pub fn baseline_light() -> Self {
        Self {
            // Primary — Comet Orange
            primary: Color::from_rgb8(255, 114, 48),
            on_primary: Color::from_rgb8(5, 5, 5),
            primary_container: Color::from_rgb8(225, 91, 14),
            on_primary_container: Color::from_rgb8(244, 244, 245),

            // Secondary
            secondary: Color::from_rgb8(244, 244, 245),
            on_secondary: Color::from_rgb8(255, 114, 48),
            secondary_container: Color::from_rgb8(219, 219, 220),
            on_secondary_container: Color::from_rgb8(117, 117, 122),
            secondary_fixed_dim: Color::from_rgb8(200, 200, 203),

            // Tertiary
            tertiary: Color::from_rgb8(125, 82, 96),
            on_tertiary: Color::from_rgb8(255, 255, 255),
            tertiary_container: Color::from_rgb8(255, 216, 228),
            on_tertiary_container: Color::from_rgb8(55, 11, 30),

            // Error
            error: Color::from_rgb8(255, 66, 66),
            on_error: Color::from_rgb8(244, 244, 245),
            error_container: Color::from_rgb8(184, 0, 0),
            on_error_container: Color::from_rgb8(255, 235, 235),

            // Background
            background: Color::from_rgb8(244, 244, 245),
            on_background: Color::from_rgb8(36, 36, 36),

            // Surface
            surface: Color::from_rgb8(219, 219, 220),
            on_surface: Color::from_rgb8(65, 65, 68),
            surface_variant: Color::from_rgb8(244, 244, 245),
            on_surface_variant: Color::from_rgb8(165, 165, 167),

            // Surface containers
            surface_dim: Color::from_rgb8(255, 255, 255),
            surface_bright: Color::from_rgb8(255, 255, 255),
            surface_container_lowest: Color::from_rgb8(244, 244, 245),
            surface_container_low: Color::from_rgb8(237, 237, 238),
            surface_container: Color::from_rgb8(237, 237, 238),
            surface_container_high: Color::from_rgb8(244, 244, 245),
            surface_container_highest: Color::from_rgb8(255, 255, 255),

            // Outline
            outline: Color::from_rgb8(200, 200, 203),
            outline_variant: Color::from_rgb8(244, 244, 245),

            // Inverse
            inverse_surface: Color::from_rgb8(5, 5, 5),
            inverse_on_surface: Color::from_rgb8(255, 255, 255),
            inverse_primary: Color::from_rgb8(36, 36, 36),

            // Utility
            scrim: Color::from_rgb8(165, 165, 167),
            shadow: Color::from_rgb8(165, 165, 167),
        }
    }

    /// Comet baseline **dark** color scheme.
    pub fn baseline_dark() -> Self {
        Self {
            // Primary — Comet Orange
            primary: Color::from_rgb8(249, 100, 13),
            on_primary: Color::from_rgb8(255, 255, 255),
            primary_container: Color::from_rgb8(252, 166, 123),
            on_primary_container: Color::from_rgb8(20, 20, 21),

            // Secondary
            secondary: Color::from_rgb8(20, 20, 21),
            on_secondary: Color::from_rgb8(255, 114, 48),
            secondary_container: Color::from_rgb8(35, 35, 37),
            on_secondary_container: Color::from_rgb8(161, 161, 165),
            secondary_fixed_dim: Color::from_rgb8(55, 55, 57),

            // Tertiary
            tertiary: Color::from_rgb8(239, 184, 200),
            on_tertiary: Color::from_rgb8(74, 37, 50),
            tertiary_container: Color::from_rgb8(99, 59, 72),
            on_tertiary_container: Color::from_rgb8(255, 216, 228),

            // Error
            error: Color::from_rgb8(255, 66, 66),
            on_error: Color::from_rgb8(245, 245, 245),
            error_container: Color::from_rgb8(168, 4, 4),
            on_error_container: Color::from_rgb8(254, 233, 233),

            // Background
            background: Color::from_rgb8(20, 20, 21),
            on_background: Color::from_rgb8(245, 245, 245),

            // Surface
            surface: Color::from_rgb8(35, 35, 37),
            on_surface: Color::from_rgb8(245, 245, 245),
            surface_variant: Color::from_rgb8(20, 20, 21),
            on_surface_variant: Color::from_rgb8(100, 100, 104),

            // Surface containers
            surface_dim: Color::from_rgb8(5, 5, 5),
            surface_bright: Color::from_rgb8(35, 35, 37),
            surface_container_lowest: Color::from_rgb8(5, 5, 5),
            surface_container_low: Color::from_rgb8(20, 20, 21),
            surface_container: Color::from_rgb8(25, 25, 26),
            surface_container_high: Color::from_rgb8(35, 35, 37),
            surface_container_highest: Color::from_rgb8(55, 55, 57),

            // Outline
            outline: Color::from_rgb8(55, 55, 57),
            outline_variant: Color::from_rgb8(20, 20, 21),

            // Inverse
            inverse_surface: Color::from_rgb8(255, 255, 255),
            inverse_on_surface: Color::from_rgb8(25, 25, 26),
            inverse_primary: Color::from_rgb8(245, 245, 245),

            // Utility
            scrim: Color::from_rgb8(5, 5, 5),
            shadow: Color::from_rgb8(5, 5, 5),
        }
    }
}

//  ColorVariant

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorVariant {
    Primary,
    OnPrimary,
    PrimaryContainer,
    OnPrimaryContainer,

    Secondary,
    OnSecondary,
    SecondaryContainer,
    OnSecondaryContainer,
    SecondaryFixedDim,

    Tertiary,
    OnTertiary,
    TertiaryContainer,
    OnTertiaryContainer,

    Error,
    OnError,
    ErrorContainer,
    OnErrorContainer,

    Background,
    OnBackground,

    Surface,
    OnSurface,
    SurfaceVariant,
    OnSurfaceVariant,

    SurfaceDim,
    SurfaceBright,
    SurfaceContainerLowest,
    SurfaceContainerLow,
    SurfaceContainer,
    SurfaceContainerHigh,
    SurfaceContainerHighest,

    Outline,
    OutlineVariant,

    InverseSurface,
    InverseOnSurface,
    InversePrimary,

    Scrim,
    Shadow,
}

impl ColorVariant {
    /// Resolve this role to a concrete [`utils::Color`] from the given scheme.
    #[inline]
    pub fn resolve(self, scheme: &ColorScheme) -> Color {
        match self {
            ColorVariant::Primary => scheme.primary,
            ColorVariant::OnPrimary => scheme.on_primary,
            ColorVariant::PrimaryContainer => scheme.primary_container,
            ColorVariant::OnPrimaryContainer => scheme.on_primary_container,

            ColorVariant::Secondary => scheme.secondary,
            ColorVariant::OnSecondary => scheme.on_secondary,
            ColorVariant::SecondaryContainer => scheme.secondary_container,
            ColorVariant::OnSecondaryContainer => scheme.on_secondary_container,
            ColorVariant::SecondaryFixedDim => scheme.secondary_fixed_dim,

            ColorVariant::Tertiary => scheme.tertiary,
            ColorVariant::OnTertiary => scheme.on_tertiary,
            ColorVariant::TertiaryContainer => scheme.tertiary_container,
            ColorVariant::OnTertiaryContainer => scheme.on_tertiary_container,

            ColorVariant::Error => scheme.error,
            ColorVariant::OnError => scheme.on_error,
            ColorVariant::ErrorContainer => scheme.error_container,
            ColorVariant::OnErrorContainer => scheme.on_error_container,

            ColorVariant::Background => scheme.background,
            ColorVariant::OnBackground => scheme.on_background,

            ColorVariant::Surface => scheme.surface,
            ColorVariant::OnSurface => scheme.on_surface,
            ColorVariant::SurfaceVariant => scheme.surface_variant,
            ColorVariant::OnSurfaceVariant => scheme.on_surface_variant,

            ColorVariant::SurfaceDim => scheme.surface_dim,
            ColorVariant::SurfaceBright => scheme.surface_bright,
            ColorVariant::SurfaceContainerLowest => scheme.surface_container_lowest,
            ColorVariant::SurfaceContainerLow => scheme.surface_container_low,
            ColorVariant::SurfaceContainer => scheme.surface_container,
            ColorVariant::SurfaceContainerHigh => scheme.surface_container_high,
            ColorVariant::SurfaceContainerHighest => scheme.surface_container_highest,

            ColorVariant::Outline => scheme.outline,
            ColorVariant::OutlineVariant => scheme.outline_variant,

            ColorVariant::InverseSurface => scheme.inverse_surface,
            ColorVariant::InverseOnSurface => scheme.inverse_on_surface,
            ColorVariant::InversePrimary => scheme.inverse_primary,

            ColorVariant::Scrim => scheme.scrim,
            ColorVariant::Shadow => scheme.shadow,
        }
    }
}

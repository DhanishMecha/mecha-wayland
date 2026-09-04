//! Material 3 dynamic color palette generation.
//!
//! Generates a full [`super::ColorScheme`] from any seed [`utils::Color`] using
//! the L*C*h* color space — a perceptually uniform cylindrical model that closely
//! approximates the HCT space used by Material You.
//!
//! # Pipeline
//! ```text
//! seed color
//!   → sRGB → Linear → XYZ → L*a*b* → L*C*h*
//!   → extract hue, derive palette chromas
//!   → for each tone: L*C*h* → L*a*b* → XYZ → Linear → sRGB (gamut-mapped)
//! ```
//!
//! # Usage
//! ```rust,ignore
//! use utils::Color;
//! use theme::ColorScheme;
//!
//! let seed = Color::from_rgb8(103, 80, 164);
//! let light = ColorScheme::from_seed(seed);
//! let dark  = ColorScheme::dark_from_seed(seed);
//! ```

use utils::Color;

// ── Constants ─────────────────────────────────────────────────────────────────

const WP_X: f64 = 0.95047;
const WP_Y: f64 = 1.00000;
const WP_Z: f64 = 1.08883;

const LAB_E: f64 = 0.008856;
const LAB_K: f64 = 7.787;

// ── sRGB <-> Linear ───────────────────────────────────────────────────────────

fn linearize(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn delinearize(c: f64) -> f64 {
    if c <= 0.0031308 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

// ── Linear RGB <-> XYZ (D65) ──────────────────────────────────────────────────

fn linear_to_xyz(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    (
        0.4124564 * r + 0.3575761 * g + 0.1804375 * b,
        0.2126729 * r + 0.7151522 * g + 0.0721750 * b,
        0.0193339 * r + 0.1191920 * g + 0.9503041 * b,
    )
}

fn xyz_to_linear(x: f64, y: f64, z: f64) -> (f64, f64, f64) {
    (
        3.2404542 * x - 1.5371385 * y - 0.4985314 * z,
        -0.9692660 * x + 1.8760108 * y + 0.0415560 * z,
        0.0556434 * x - 0.2040259 * y + 1.0572252 * z,
    )
}

// ── XYZ <-> L*a*b* ────────────────────────────────────────────────────────────

fn lab_f(t: f64) -> f64 {
    if t > LAB_E {
        t.cbrt()
    } else {
        LAB_K * t + 16.0 / 116.0
    }
}

fn lab_f_inv(t: f64) -> f64 {
    let t3 = t * t * t;
    if t3 > LAB_E {
        t3
    } else {
        (t - 16.0 / 116.0) / LAB_K
    }
}

fn xyz_to_lab(x: f64, y: f64, z: f64) -> (f64, f64, f64) {
    let fx = lab_f(x / WP_X);
    let fy = lab_f(y / WP_Y);
    let fz = lab_f(z / WP_Z);
    (116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz))
}

fn lab_to_xyz(l: f64, a: f64, b: f64) -> (f64, f64, f64) {
    let fy = (l + 16.0) / 116.0;
    let fx = a / 500.0 + fy;
    let fz = fy - b / 200.0;
    (
        WP_X * lab_f_inv(fx),
        WP_Y * lab_f_inv(fy),
        WP_Z * lab_f_inv(fz),
    )
}

// ── L*a*b* <-> L*C*h* ─────────────────────────────────────────────────────────

fn lab_to_lch(l: f64, a: f64, b: f64) -> (f64, f64, f64) {
    let c = (a * a + b * b).sqrt();
    let h = b.atan2(a).to_degrees().rem_euclid(360.0);
    (l, c, h)
}

fn lch_to_lab(l: f64, c: f64, h: f64) -> (f64, f64, f64) {
    let h_rad = h.to_radians();
    (l, c * h_rad.cos(), c * h_rad.sin())
}

// ── Gamut mapping ─────────────────────────────────────────────────────────────

fn lch_to_srgb_raw(l: f64, c: f64, h: f64) -> (f64, f64, f64) {
    let (_, a, b) = lch_to_lab(l, c, h);
    let (x, y, z) = lab_to_xyz(l, a, b);
    let (lr, lg, lb) = xyz_to_linear(x, y, z);
    (delinearize(lr), delinearize(lg), delinearize(lb))
}

fn in_gamut(l: f64, c: f64, h: f64) -> bool {
    const TOL: f64 = 0.0001;
    let (r, g, b) = lch_to_srgb_raw(l, c, h);
    r >= -TOL && r <= 1.0 + TOL && g >= -TOL && g <= 1.0 + TOL && b >= -TOL && b <= 1.0 + TOL
}

/// Convert L*C*h* to a gamut-mapped sRGB Color using binary-search chroma reduction.
fn lch_to_color(l: f64, c: f64, h: f64) -> Color {
    let chroma = c.max(0.0);
    let (r, g, b) = if chroma == 0.0 || in_gamut(l, chroma, h) {
        let (r, g, b) = lch_to_srgb_raw(l, chroma, h);
        (r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0))
    } else {
        let mut lo = 0.0_f64;
        let mut hi = chroma;
        for _ in 0..24 {
            let mid = (lo + hi) / 2.0;
            if in_gamut(l, mid, h) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let (r, g, b) = lch_to_srgb_raw(l, lo, h);
        (r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0))
    };
    Color::from_rgb8(
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
    )
}

fn color_to_lch(color: Color) -> (f64, f64, f64) {
    let r = linearize(color.r as f64);
    let g = linearize(color.g as f64);
    let b = linearize(color.b as f64);
    let (x, y, z) = linear_to_xyz(r, g, b);
    let (l, a, lab_b) = xyz_to_lab(x, y, z);
    lab_to_lch(l, a, lab_b)
}

// ── TonalPalette ──────────────────────────────────────────────────────────────

/// A Material 3 tonal palette: fixed hue and chroma, varying tone (0–100).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TonalPalette {
    pub hue: f64,
    pub chroma: f64,
}

impl TonalPalette {
    pub fn of(hue: f64, chroma: f64) -> Self {
        Self {
            hue: hue.rem_euclid(360.0),
            chroma: chroma.max(0.0),
        }
    }

    /// Get the color at the given tone (0 = black, 100 = white).
    pub fn tone(&self, tone: f64) -> Color {
        lch_to_color(tone.clamp(0.0, 100.0), self.chroma, self.hue)
    }
}

// ── CorePalettes ──────────────────────────────────────────────────────────────

/// The six core M3 tonal palettes derived from a seed color.
#[derive(Debug, Clone, PartialEq)]
pub struct CorePalettes {
    pub primary: TonalPalette,
    pub secondary: TonalPalette,
    pub tertiary: TonalPalette,
    pub neutral: TonalPalette,
    pub neutral_variant: TonalPalette,
    pub error: TonalPalette,
}

impl CorePalettes {
    /// Derive M3 core palettes from a seed color.
    pub fn from_seed(seed: Color) -> Self {
        let (_, chroma, hue) = color_to_lch(seed);
        Self {
            primary: TonalPalette::of(hue, chroma.max(48.0)),
            secondary: TonalPalette::of(hue, 16.0),
            tertiary: TonalPalette::of(hue + 60.0, 24.0),
            neutral: TonalPalette::of(hue, 4.0),
            neutral_variant: TonalPalette::of(hue, 8.0),
            error: TonalPalette::of(25.0, 84.0),
        }
    }
}

// ── ColorScheme generation ────────────────────────────────────────────────────

use super::ColorScheme;

impl ColorScheme {
    /// Generate a Material 3 **light** color scheme from a seed color.
    pub fn from_seed(seed: Color) -> Self {
        let p = CorePalettes::from_seed(seed);
        let (pr, s, t, n, nv, e) = (
            &p.primary,
            &p.secondary,
            &p.tertiary,
            &p.neutral,
            &p.neutral_variant,
            &p.error,
        );
        Self {
            primary: pr.tone(40.0),
            on_primary: pr.tone(100.0),
            primary_container: pr.tone(90.0),
            on_primary_container: pr.tone(10.0),
            secondary: s.tone(40.0),
            on_secondary: s.tone(100.0),
            secondary_container: s.tone(90.0),
            on_secondary_container: s.tone(10.0),
            tertiary: t.tone(40.0),
            on_tertiary: t.tone(100.0),
            tertiary_container: t.tone(90.0),
            on_tertiary_container: t.tone(10.0),
            error: e.tone(40.0),
            on_error: e.tone(100.0),
            error_container: e.tone(90.0),
            on_error_container: e.tone(10.0),
            background: n.tone(98.0),
            on_background: n.tone(10.0),
            surface: n.tone(98.0),
            on_surface: n.tone(10.0),
            surface_variant: nv.tone(90.0),
            on_surface_variant: nv.tone(30.0),
            surface_dim: n.tone(87.0),
            surface_bright: n.tone(98.0),
            surface_container_lowest: n.tone(100.0),
            surface_container_low: n.tone(96.0),
            surface_container: n.tone(94.0),
            surface_container_high: n.tone(92.0),
            surface_container_highest: n.tone(90.0),
            outline: nv.tone(50.0),
            outline_variant: nv.tone(80.0),
            inverse_surface: n.tone(20.0),
            inverse_on_surface: n.tone(95.0),
            inverse_primary: pr.tone(80.0),
            scrim: n.tone(0.0),
            shadow: n.tone(0.0),
        }
    }

    /// Generate a Material 3 **dark** color scheme from a seed color.
    pub fn dark_from_seed(seed: Color) -> Self {
        let p = CorePalettes::from_seed(seed);
        let (pr, s, t, n, nv, e) = (
            &p.primary,
            &p.secondary,
            &p.tertiary,
            &p.neutral,
            &p.neutral_variant,
            &p.error,
        );
        Self {
            primary: pr.tone(80.0),
            on_primary: pr.tone(20.0),
            primary_container: pr.tone(30.0),
            on_primary_container: pr.tone(90.0),
            secondary: s.tone(80.0),
            on_secondary: s.tone(20.0),
            secondary_container: s.tone(30.0),
            on_secondary_container: s.tone(90.0),
            tertiary: t.tone(80.0),
            on_tertiary: t.tone(20.0),
            tertiary_container: t.tone(30.0),
            on_tertiary_container: t.tone(90.0),
            error: e.tone(80.0),
            on_error: e.tone(20.0),
            error_container: e.tone(30.0),
            on_error_container: e.tone(90.0),
            background: n.tone(6.0),
            on_background: n.tone(90.0),
            surface: n.tone(6.0),
            on_surface: n.tone(90.0),
            surface_variant: nv.tone(30.0),
            on_surface_variant: nv.tone(80.0),
            surface_dim: n.tone(6.0),
            surface_bright: n.tone(24.0),
            surface_container_lowest: n.tone(4.0),
            surface_container_low: n.tone(10.0),
            surface_container: n.tone(12.0),
            surface_container_high: n.tone(17.0),
            surface_container_highest: n.tone(22.0),
            outline: nv.tone(60.0),
            outline_variant: nv.tone(30.0),
            inverse_surface: n.tone(90.0),
            inverse_on_surface: n.tone(20.0),
            inverse_primary: pr.tone(40.0),
            scrim: n.tone(0.0),
            shadow: n.tone(0.0),
        }
    }
}

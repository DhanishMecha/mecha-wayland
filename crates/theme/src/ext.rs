use app::{App, Context, Signal, Spawner, Widget};
use utils::Color;

use crate::color::ThemeColor;
use crate::typography::{TextVariant, TypographyStyle};
use crate::{MechanixTheme, ThemeMode};

pub const MISSING_THEME_ERR: &str = "MechanixTheme resource not found. Ensure MechanixTheme was added as a module or resource to the App.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ThemeChanged;
impl Signal for ThemeChanged {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ApplyTheme;
impl app::Event for ApplyTheme {}

pub fn on_theme_changed(app: &mut App, _: &ThemeChanged) {
    app.emit_all(ApplyTheme);
}

pub trait ContextThemeExt {
    fn theme(&self) -> MechanixTheme;
    fn with_theme<R>(&self, f: impl FnOnce(&MechanixTheme) -> R) -> R;

    #[inline]
    fn theme_mode(&self) -> ThemeMode {
        self.with_theme(|t| t.mode)
    }

    #[inline]
    fn color(&self, role: ThemeColor) -> Color {
        self.with_theme(|t| t.color(role))
    }

    #[inline]
    fn typography(&self, variant: TextVariant) -> TypographyStyle {
        self.with_theme(|t| t.typography(variant))
    }

    fn set_theme(&mut self, theme: MechanixTheme);
    fn update_theme<F: FnOnce(&mut MechanixTheme)>(&mut self, f: F);
}

impl<W: Widget> ContextThemeExt for Context<'_, W> {
    fn theme(&self) -> MechanixTheme {
        self.with_theme(|t| t.clone())
    }

    fn with_theme<R>(&self, f: impl FnOnce(&MechanixTheme) -> R) -> R {
        let theme = self.resource::<MechanixTheme>().expect(MISSING_THEME_ERR);
        f(&theme)
    }

    fn set_theme(&mut self, theme: MechanixTheme) {
        let mut current = self
            .resource_mut::<MechanixTheme>()
            .expect(MISSING_THEME_ERR);
        *current = theme;
        self.signal(ThemeChanged);
    }

    fn update_theme<F: FnOnce(&mut MechanixTheme)>(&mut self, f: F) {
        let mut current = self
            .resource_mut::<MechanixTheme>()
            .expect(MISSING_THEME_ERR);
        f(&mut current);
        self.signal(ThemeChanged);
    }
}

pub trait AppThemeExt {
    fn theme(&self) -> MechanixTheme;
    fn with_theme<R>(&self, f: impl FnOnce(&MechanixTheme) -> R) -> R;

    #[inline]
    fn theme_mode(&self) -> ThemeMode {
        self.with_theme(|t| t.mode)
    }

    #[inline]
    fn color(&self, role: ThemeColor) -> Color {
        self.with_theme(|t| t.color(role))
    }

    #[inline]
    fn typography(&self, variant: TextVariant) -> TypographyStyle {
        self.with_theme(|t| t.typography(variant))
    }

    fn set_theme(&mut self, theme: MechanixTheme);
    fn update_theme<F: FnOnce(&mut MechanixTheme)>(&mut self, f: F);
}

impl AppThemeExt for App {
    fn theme(&self) -> MechanixTheme {
        self.with_theme(|t| t.clone())
    }

    fn with_theme<R>(&self, f: impl FnOnce(&MechanixTheme) -> R) -> R {
        let theme = self.resource::<MechanixTheme>().expect(MISSING_THEME_ERR);
        f(&theme)
    }

    fn set_theme(&mut self, theme: MechanixTheme) {
        let mut current = self
            .resource_mut::<MechanixTheme>()
            .expect(MISSING_THEME_ERR);
        *current = theme;
        self.signal(ThemeChanged);
    }

    fn update_theme<F: FnOnce(&mut MechanixTheme)>(&mut self, f: F) {
        let mut current = self
            .resource_mut::<MechanixTheme>()
            .expect(MISSING_THEME_ERR);
        f(&mut current);
        self.signal(ThemeChanged);
    }
}

pub trait SpawnerThemeExt {
    fn theme(&self) -> MechanixTheme;
    fn with_theme<R>(&self, f: impl FnOnce(&MechanixTheme) -> R) -> R;

    #[inline]
    fn theme_mode(&self) -> ThemeMode {
        self.with_theme(|t| t.mode)
    }

    #[inline]
    fn color(&self, role: ThemeColor) -> Color {
        self.with_theme(|t| t.color(role))
    }

    #[inline]
    fn typography(&self, variant: TextVariant) -> TypographyStyle {
        self.with_theme(|t| t.typography(variant))
    }
}

impl<W: Widget> SpawnerThemeExt for Spawner<'_, W> {
    fn theme(&self) -> MechanixTheme {
        self.with_theme(|t| t.clone())
    }

    fn with_theme<R>(&self, f: impl FnOnce(&MechanixTheme) -> R) -> R {
        let theme = self.resource::<MechanixTheme>().expect(MISSING_THEME_ERR);
        f(&theme)
    }
}

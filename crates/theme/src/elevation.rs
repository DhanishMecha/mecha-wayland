#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Elevation(pub f32);

impl Elevation {
    pub const LEVEL0: Self = Self(0.0);
    pub const LEVEL1: Self = Self(1.0);
    pub const LEVEL2: Self = Self(3.0);
    pub const LEVEL3: Self = Self(6.0);
    pub const LEVEL4: Self = Self(8.0);
    pub const LEVEL5: Self = Self(12.0);

    #[inline]
    pub fn dp(self) -> f32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ElevationScale {
    pub level0: Elevation,
    pub level1: Elevation,
    pub level2: Elevation,
    pub level3: Elevation,
    pub level4: Elevation,
    pub level5: Elevation,
}

impl Default for ElevationScale {
    fn default() -> Self {
        Self {
            level0: Elevation::LEVEL0,
            level1: Elevation::LEVEL1,
            level2: Elevation::LEVEL2,
            level3: Elevation::LEVEL3,
            level4: Elevation::LEVEL4,
            level5: Elevation::LEVEL5,
        }
    }
}

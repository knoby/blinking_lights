use core::ops::{Add, Mul, Sub};

/// Struct to store the data for an LED
#[derive(Copy, Clone, Debug, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    /// Create a new Struct from RGB Value
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Create new Red Color
    pub fn red() -> Self {
        Self { r: 255, g: 0, b: 0 }
    }

    /// Create new Red Color
    pub fn blue() -> Self {
        Self { r: 0, g: 255, b: 0 }
    }

    /// Create new Red Color
    pub fn green() -> Self {
        Self { r: 0, g: 0, b: 255 }
    }

    /// Create new Red Color
    pub fn white() -> Self {
        Self {
            r: 255,
            g: 255,
            b: 255,
        }
    }

    /// Create new Red Color
    pub fn led_off() -> Self {
        Self { r: 0, g: 0, b: 0 }
    }

    /// Invert the current color
    pub fn invert(&mut self) {
        self.r = 255 - self.r;
        self.g = 255 - self.g;
        self.b = 255 - self.b;
    }
}

impl From<[u8; 3]> for Color {
    fn from(array: [u8; 3]) -> Self {
        Self {
            r: array[0],
            g: array[1],
            b: array[2],
        }
    }
}

impl Into<[u8; 3]> for Color {
    fn into(self) -> [u8; 3] {
        [self.r, self.g, self.b]
    }
}

impl Add for Color {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self {
            r: self.r.saturating_add(other.r),
            g: self.g.saturating_add(other.g),
            b: self.b.saturating_add(other.b),
        }
    }
}

impl Sub for Color {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self {
            r: self.r.saturating_sub(other.r),
            g: self.g.saturating_sub(other.g),
            b: self.b.saturating_sub(other.b),
        }
    }
}

impl Mul for Color {
    type Output = Self;

    fn mul(self, other: Self) -> Self::Output {
        Self {
            r: self.r.saturating_mul(other.r),
            g: self.g.saturating_mul(other.g),
            b: self.b.saturating_mul(other.b),
        }
    }
}

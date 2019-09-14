use crate::color;

/// Holds the data for the Matrix
#[derive(Debug, Default)]
pub struct LedMatrix8x8 {
    pub data: [[color::Color; 8]; 8],
}

impl LedMatrix8x8 {
    /// Init from color
    pub fn new(color: color::Color) -> Self {
        Self {
            data: [[color; 8]; 8],
        }
    }

    /// Invert all colors
    pub fn invert(&mut self) {
        for led in self.data.iter_mut().flatten() {
            led.invert();
        }
    }

    /// Shift Led Colors positiv
    pub fn shift_pos(&mut self) {
        // save last element
        let mut saved_element = self.data[7][7];
        for led in self.data.iter_mut().flatten() {
            core::mem::swap(&mut (*led), &mut saved_element);
        }
    }

    /// Shift Led Colors negative
    pub fn shift_neg(&mut self) {
        // save last element
        let mut saved_element = self.data[0][0];
        for led in self.data.iter_mut().flatten().rev() {
            core::mem::swap(&mut (*led), &mut saved_element);
        }
    }
}

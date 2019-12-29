use crate::color;
use cast::{u16, u32};
use stm32f1xx_hal::prelude::*;
//use stm32f1xx_hal::timer::PclkSrc;

// Trait for PWM Output of a pattern with double buffer support
pub struct PwmPatternDB {
    tim: stm32f1xx_hal::device::TIM2,
    dma_ch: stm32f1xx_hal::dma::dma1::C2,
    double_buffer: [u16; 48], // the double buffer --> 3*8 byte for the bits of the color
    lower_buffer_active: bool, // DMA Transfer is active for the lower part of the buffer
    duty_zero: u16,
    duty_one: u16,
}

impl PwmPatternDB {
    pub fn new(
        tim: stm32f1xx_hal::timer::Timer<stm32f1xx_hal::device::TIM2>,
        mut dma_ch: stm32f1xx_hal::dma::dma1::C2,
        clocks: stm32f1xx_hal::rcc::Clocks,
    ) -> Self {
        // Get the internal registers of the timer
        let tim = tim.release();

        // Calculate the timer prescaler
        //let frequency = 800.khz(); // T = 1.25u --> Length for WS2812
        let frequency = 2.hz(); // Testing Frequency to visulise LED

        let timer_clock = clocks.pclk2();

        let ticks = timer_clock.0 / frequency.0;
        let psc = u16((ticks - 1) / (1 << 16)).unwrap();
        tim.psc.write(|w| unsafe { w.psc().bits(psc) });

        let arr = u16(ticks / u32(psc + 1)).unwrap();

        tim.arr.write(|w| unsafe { w.bits(u32(arr)) });

        tim.ccr1.write(|w| w.ccr().bits(arr / 2));

        // Trigger an update event to load the prescaler value to the clock
        tim.egr.write(|w| w.ug().set_bit());

        // The above line raises an update event which will indicate
        // that the timer is already finished. Since this is not the case,
        // it should be cleared
        tim.sr.modify(|_, w| w.uif().clear_bit());

        // Enable the Interrupt on Compare and Update and DMA on Update
        tim.dier
            .write(|w| w.uie().set_bit().cc1ie().set_bit().ude().set_bit());

        // Configuration of the dma

        // Compare Address Base + Offset Length 16 bit
        let arr_address = 0x4000_0000 + 0x34;
        dma_ch.set_peripheral_address(arr_address, false);

        // Set Number of transfers
        unsafe {
            dma_ch.ch().ndtr.write(|w| w.bits(48));
        }

        // Configuration of DMA
        unsafe {
            dma_ch.ch().cr.write(|w| {
                w.msize()
                    .bits(0x01) // Write 16 bit
                    .psize()
                    .bits(0x01) // Write 16 bit
                    .minc()
                    .set_bit() // Inc Memocry Add
                    .circ()
                    .set_bit() // Enable Circular Mode
                    .dir()
                    .set_bit() // Memory -> Per
                    .htie()
                    .set_bit() // Enable Half Transfer Interrupt
                    .tcie()
                    .set_bit() // Enable Transfer Comple Interrupt
            });
        }

        PwmPatternDB {
            tim,
            dma_ch,
            double_buffer: [0; 48],
            lower_buffer_active: false,
            duty_zero: (u32::from(arr) * 28_u32 / 100_u32) as u16,
            duty_one: (u32::from(arr) * 56_u32 / 100_u32) as u16,
        }
    }

    fn set_next_buffer(&mut self, color: color::Color) {
        if self.lower_buffer_active {
            self.set_color_pattern(color, true);
        } else {
            self.set_color_pattern(color, false);
        }
    }

    /// Set the Memory Address an start timer and dma
    pub fn start(&mut self, first_color: color::Color) {
        // Set Memory Add
        self.dma_ch
            .set_memory_address(&self.double_buffer as *const _ as u32, true);
        // mark lower as active
        self.lower_buffer_active = true;
        // Set the first color
        self.set_color_pattern(first_color, self.lower_buffer_active);
        // Prepare reset pattern
        set_reset_pattern(&mut self.double_buffer[24..48]);
        // Enable DMA
        self.dma_ch.start();
        // Start the timer
        self.tim.cr1.modify(|_, w| w.cen().set_bit());
    }

    /// Stop the DMA transfer
    pub fn stop(&mut self) {
        self.dma_ch.stop();
    }

    /// Reset timer interrupt flag
    pub fn reset_isr_tim(&mut self) {
        self.tim
            .sr
            .modify(|_, w| w.uif().clear_bit().cc1if().clear_bit());
    }

    /// Reset dma interrupt flag
    pub fn reset_isr_dma(&mut self) {
        self.dma_ch
            .ifcr()
            .write(|w| w.ctcif2().clear().chtif2().clear());
    }

    /// Check if the timer interrupt is a compare interrupt
    pub fn is_cmp_irq(&self) -> bool {
        !self.tim.sr.read().uif().is_update_pending()
    }

    // Generates the timing data for the Color
    fn set_color_pattern(&mut self, color: color::Color, use_upper_bytes: bool) {
        let mut bit_mask = 0b1000_0000;
        let color_byte: [u8; 3] = color.into();
        let offset = if use_upper_bytes { 0 } else { 24 };
        for bit_num in 0..8 {
            self.double_buffer[bit_num + offset] = if (color_byte[0] & bit_mask) > 0 {
                self.duty_one
            } else {
                self.duty_zero
            };

            bit_mask = bit_mask >> 1;
        }
        for bit_num in 8..16 {
            self.double_buffer[bit_num + offset] = if (color_byte[1] & bit_mask) > 0 {
                self.duty_one
            } else {
                self.duty_zero
            };

            bit_mask = bit_mask >> 1;
        }
        for bit_num in 16..24 {
            self.double_buffer[bit_num + offset] = if (color_byte[2] & bit_mask) > 0 {
                self.duty_one
            } else {
                self.duty_zero
            };

            bit_mask = bit_mask >> 1;
        }
    }
}

// Generates the timing data for a reset
fn set_reset_pattern(slice: &mut [u16]) {
    for byte in slice {
        *byte = 0;
    }
}

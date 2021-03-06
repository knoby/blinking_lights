use crate::color;
use cast::{u16, u32};
use stm32f1xx_hal::gpio::{Alternate, PushPull};
use stm32f1xx_hal::prelude::*;

// Buffer to hold the data for the dma
// Adjust NUM_LEDS to the desired number
const NUM_LEDS: usize = 64;
const NUM_RESET: usize = 50;
// Size is calculated as following:
// Each color has 8 bits --> 8 bytes to decode the timings for the timer
// one LED has 3 colors --> 24 bytes
// after putting the colors to the led we need a reset pulse. this is coded as 50 bytes with zero length
pub type DisplayBuffer = [u16; 24 * NUM_LEDS + NUM_RESET];

impl InitBuffer for DisplayBuffer {
    fn new() -> Self {
        [0; 24 * NUM_LEDS + NUM_RESET]
    }
}

pub trait InitBuffer {
    fn new() -> DisplayBuffer;
}

// Trait for PWM Output of a pattern with double buffer support
pub struct PwmPatternDB {
    tim: stm32f1xx_hal::device::TIM2,
    dma_ch: stm32f1xx_hal::dma::dma1::C2,
    duty_zero: u16,
    duty_one: u16,
    _out_pin: stm32f1xx_hal::gpio::gpioa::PA0<Alternate<PushPull>>, // Only saved but is controled by the timer
}

impl PwmPatternDB {
    pub fn new(
        tim: stm32f1xx_hal::timer::Timer<stm32f1xx_hal::device::TIM2>,
        mut dma_ch: stm32f1xx_hal::dma::dma1::C2,
        clocks: stm32f1xx_hal::rcc::Clocks,
        out_pin: stm32f1xx_hal::gpio::gpioa::PA0<Alternate<PushPull>>,
    ) -> Self {
        // Get the internal registers of the timer
        let tim = tim.release();

        // Calculate the timer prescaler
        let frequency = 800_000.hz(); // T = 1.25u --> Length for WS2812

        let timer_clock = clocks.pclk2();

        let ticks = timer_clock.0 / frequency.0;
        let psc = u16((ticks - 1) / (1 << 16)).unwrap();
        tim.psc.write(|w| w.psc().bits(psc));

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

        // Enable PWM Output for Channel 1
        unsafe {
            tim.ccmr1_output().write(|w| {
                w.cc1s()
                    .bits(0b00) // Set Output Mode
                    .oc1fe()
                    .clear_bit() // Disable fast compare
                    .oc1pe()
                    .set_bit() // Disable Preload Register
                    .oc1m()
                    .bits(0b110) // Enable PWM Mode 1 --> Output active aslong as ctn < cmp
            });
        }

        tim.ccer.write(|w| w.cc1e().set_bit()); // Enable Output Circuit

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
                    .clear_bit() // Disable Circular Mode
                    .dir()
                    .set_bit() // Memory -> Per
                    .htie()
                    .clear_bit() // Disable Half Transfer Interrupt
                    .tcie()
                    .set_bit() // Enable Transfer Comple Interrupt
            });
        }

        PwmPatternDB {
            tim,
            dma_ch,
            duty_zero: (u32::from(arr) * 35_u32 / 125_u32) as u16,
            duty_one: (u32::from(arr) * 60_u32 / 125_u32) as u16,
            _out_pin: out_pin,
        }
    }

    /// Set the Memory Address an start timer and dma
    pub fn start(&mut self, buffer: &mut DisplayBuffer) {
        // Set Memory Add
        self.dma_ch
            .set_memory_address(buffer as *const _ as u32, true);
        // Set Number of transfers
        unsafe {
            self.dma_ch.ch().ndtr.write(|w| w.bits(buffer.len() as u32));
        }
        // Load value zero to the ccr
        unsafe {
            self.tim.ccr1.write(|w| w.bits(0x00));
        }

        self.tim.egr.write(|w| w.ug().set_bit());
        // Enable DMA
        self.dma_ch.start();
        // Start the timer
        self.tim.cr1.modify(|_, w| w.cen().set_bit());
    }

    /// Stop the Output
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

    /// Check if transfer is active
    pub fn is_active(&self) -> bool {
        self.dma_ch.in_progress()
    }

    // Generates the timing data for the Color
    pub fn set_color_pattern(
        &self,
        color: color::Color,
        led_num: usize,
        buffer: &mut DisplayBuffer,
    ) {
        let mut bit_mask = 0b1000_0000;
        for bit_num in 0..8 {
            buffer[bit_num + (24 * led_num)] = if (color.g & bit_mask) > 0 {
                self.duty_one
            } else {
                self.duty_zero
            };

            bit_mask >>= 1;
        }
        let mut bit_mask = 0b1000_0000;
        for bit_num in 8..16 {
            buffer[bit_num + (24 * led_num)] = if (color.r & bit_mask) > 0 {
                self.duty_one
            } else {
                self.duty_zero
            };

            bit_mask >>= 1;
        }
        let mut bit_mask = 0b1000_0000;
        for bit_num in 16..24 {
            buffer[bit_num + (24 * led_num)] = if (color.b & bit_mask) > 0 {
                self.duty_one
            } else {
                self.duty_zero
            };

            bit_mask >>= 1;
        }
    }
}

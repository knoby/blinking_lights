use cast::{u16, u32};
use stm32f1xx_hal::prelude::*;
use stm32f1xx_hal::timer::PclkSrc;

// Trait for PWM Output of a pattern with double buffer support
pub struct PwmPatternDB {
    tim: stm32f1xx_hal::device::TIM2,
    dma_ch: stm32f1xx_hal::dma::dma1::C2,
    clocks: stm32f1xx_hal::rcc::Clocks,
}

impl PwmPatternDB {
    pub fn new(
        tim: stm32f1xx_hal::timer::Timer<stm32f1xx_hal::device::TIM2>,
        dma_ch: stm32f1xx_hal::dma::dma1::C2,
        clocks: stm32f1xx_hal::rcc::Clocks,
    ) -> Self {
        // Get the internal registers of the timer
        let tim = tim.release();

        // Calculate the timer prescaler
        let frequency = 8.hz();
        let timer_clock = stm32f1xx_hal::device::TIM2::get_clk(&clocks);

        let ticks = timer_clock.0 / frequency.0;
        let psc = u16((ticks - 1) / (1 << 16)).unwrap();

        tim.psc.write(|w| unsafe { w.psc().bits(psc) });

        let arr = u16(ticks / u32(psc + 1)).unwrap();

        tim.arr.write(|w| unsafe { w.bits(u32(arr)) });

        // Trigger an update event to load the prescaler value to the clock
        tim.egr.write(|w| w.ug().set_bit());
        // The above line raises an update event which will indicate
        // that the timer is already finished. Since this is not the case,
        // it should be cleared
        tim.sr.modify(|_, w| w.uif().clear_bit());

        // Enable the Interrupt
        tim.dier.write(|w| w.uie().set_bit());

        // Start the timer
        tim.cr1.modify(|_, w| w.cen().set_bit());

        PwmPatternDB {
            tim,
            dma_ch,
            clocks,
        }
    }

    pub fn reset_isr(&mut self) {
        self.tim.sr.modify(|_, w| w.uif().clear_bit());
    }
}

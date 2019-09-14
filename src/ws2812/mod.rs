use cast::{u16, u32};
use stm32f1xx_hal::prelude::*;
use stm32f1xx_hal::timer::PclkSrc;

// Trait for PWM Output of a pattern with double buffer support
pub struct PwmPatternDB {
    tim: stm32f1xx_hal::device::TIM2,
    dma_ch: stm32f1xx_hal::dma::dma1::C2,
    //clocks: stm32f1xx_hal::rcc::Clocks,
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

        let timer_clock = stm32f1xx_hal::device::TIM2::get_clk(&clocks);

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
            //clocks,
        }
    }

    /// Set the Memory Address an start timer and dma
    pub fn start(&mut self, buffer: &mut [u16; 48]) {
        for item in buffer.iter_mut().step_by(2) {
            *item = 40_000;
        }

        for item in buffer.iter_mut().skip(1).step_by(2) {
            *item = 5000;
        }
        // Set Memory Add
        self.dma_ch
            .set_memory_address(buffer as *const _ as u32, true);
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
}

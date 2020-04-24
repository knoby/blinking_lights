//#![deny(unsafe_code)]
//#![deny(warnings)]
//#![allow(deprecated)]
#![no_main]
#![no_std]

// panic-handler crate
//extern crate panic_semihosting;
extern crate panic_halt;

use cortex_m;
use embedded_hal::digital::v2::OutputPin;
use rtfm::app;
use rtfm::cyccnt::{Duration, Instant, U32Ext};
use stm32f1xx_hal::gpio::{Output, PushPull};
use stm32f1xx_hal::prelude::*;

mod color;
mod led_matrix_8x8;
mod ws2812;

#[app(device = stm32f1xx_hal::stm32, peripherals=true, monotonic=rtfm::cyccnt::CYCCNT)]
const APP: () = {
    struct Resources {
        ON_BOARD_LED: stm32f1xx_hal::gpio::gpioa::PA5<Output<PushPull>>,
        WS2812: ws2812::PwmPatternDB,
        DISPLAY_BUFFER: ws2812::DisplayBuffer,
    }

    #[init(schedule=[], resources=[])]
    fn init(mut c: init::Context) -> init::LateResources {
        // Initialize (enable) the monotonic timer (CYCCNT)
        c.core.DCB.enable_trace();
        // required on devices that software lock the DWT (e.g. STM32F7)
        unsafe { c.core.DWT.lar.write(0xC5ACCE55) }
        c.core.DWT.enable_cycle_counter();

        // Freeze clock frequencies
        let mut flash = c.device.FLASH.constrain();
        let mut rcc = c.device.RCC.constrain();
        let clocks = rcc
            .cfgr
            .sysclk(64.mhz())
            .pclk1(32.mhz())
            .pclk2(64.mhz())
            .freeze(&mut flash.acr);

        // Setup On Board LED
        let mut gpioa = c.device.GPIOA.split(&mut rcc.apb2);
        let led = gpioa.pa5.into_push_pull_output(&mut gpioa.crl);

        // Setup the PWM Output
        let tim2 = stm32f1xx_hal::timer::Timer::tim2(c.device.TIM2, &clocks, &mut rcc.apb1);
        let dma1_c2 = c.device.DMA1.split(&mut rcc.ahb).2;
        let ws2812 = ws2812::PwmPatternDB::new(
            tim2,
            dma1_c2,
            clocks,
            gpioa.pa0.into_alternate_push_pull(&mut gpioa.crl),
        );

        init::LateResources {
            ON_BOARD_LED: led,
            WS2812: ws2812,
            DISPLAY_BUFFER: [0; 24 * 64 + 50],
        }
    }

    #[idle(schedule = [update_display])]
    fn idle(c: idle::Context) -> ! {
        c.schedule.update_display(Instant::now()).unwrap();
        loop {
            cortex_m::asm::nop();
        }
    }

    #[task(resources = [WS2812, DISPLAY_BUFFER, ON_BOARD_LED], schedule=[update_display])]
    fn update_display(mut c: update_display::Context) {
        static mut led_num: usize = 0;

        for num in 0..64 {
            c.resources.WS2812.set_color_pattern(
                color::Color::led_off(),
                num,
                c.resources.DISPLAY_BUFFER,
            );
        }
        for num in 0..64 {
            c.resources.WS2812.set_color_pattern(
                color::Color::new(1 * *led_num as u8, 0, 1 * (64 - *led_num as u8)),
                num,
                c.resources.DISPLAY_BUFFER,
            );
        }

        c.resources.WS2812.start(c.resources.DISPLAY_BUFFER);
        while c.resources.WS2812.is_active() {
            cortex_m::asm::nop();
        }
        c.resources.WS2812.reset_isr_dma();
        c.resources.WS2812.stop();
        if *led_num < 64 {
            *led_num += 1;
        } else {
            *led_num = 0;
        }
        c.schedule
            .update_display(Instant::now() + 64_00_000.cycles())
            .unwrap();
    }

    extern "C" {
        fn EXTI0();
    }
};

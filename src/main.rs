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
    fn init(c: init::Context) -> init::LateResources {
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

    #[idle(resources = [WS2812, DISPLAY_BUFFER])]
    fn idle(mut c: idle::Context) -> ! {
        // Set the first three Colors
        c.resources.WS2812.set_color_pattern(
            color::Color::red(),
            0,
            &mut c.resources.DISPLAY_BUFFER,
        );
        c.resources.WS2812.set_color_pattern(
            color::Color::green(),
            1,
            &mut c.resources.DISPLAY_BUFFER,
        );
        c.resources.WS2812.set_color_pattern(
            color::Color::blue(),
            2,
            &mut c.resources.DISPLAY_BUFFER,
        );
        // Start the LEDs
        c.resources.WS2812.start(c.resources.DISPLAY_BUFFER);
        loop {
            cortex_m::asm::nop();
        }
    }

    extern "C" {
        fn EXTI0();
    }
};

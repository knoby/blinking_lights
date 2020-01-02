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
        DISPLAY_BUFFER: led_matrix_8x8::LedMatrix8x8,
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
            DISPLAY_BUFFER: led_matrix_8x8::LedMatrix8x8::new(color::Color::white()),
        }
    }

    #[idle(resources = [WS2812, ON_BOARD_LED])]
    fn idle(mut c: idle::Context) -> ! {
        // Set the LED off
        c.resources.ON_BOARD_LED.lock(|LED| LED.set_low().unwrap());
        // Start the LEDs
        c.resources.WS2812.lock(|WS2812| {
            WS2812.start(color::Color::white());
            WS2812.set_next_buffer(color::Color::white());
        });
        loop {
            cortex_m::asm::nop();
        }
    }

    #[task(binds=DMA1_CHANNEL2, resources = [WS2812, ON_BOARD_LED])]
    fn dma1_channel2(c: dma1_channel2::Context) {
        static mut color_count: u8 = 0;

        *color_count += 1;
        if *color_count == 3 {
            c.resources.WS2812.stop();
        }
        if *color_count == 2 {
            c.resources.WS2812.set_reset_pattern();
        }

        c.resources.WS2812.reset_isr_dma();
    }

    extern "C" {
        fn EXTI0();
    }
};

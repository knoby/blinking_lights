//#![deny(unsafe_code)]
//#![deny(warnings)]
//#![allow(deprecated)]
#![no_main]
#![no_std]

// panic-handler crate
extern crate panic_semihosting;

use cortex_m;
use rtfm::app;
use stm32f1xx_hal::gpio::{Output, PushPull};
use stm32f1xx_hal::prelude::*;

mod color;
mod led_matrix_8x8;
mod ws2812;

#[app(device = stm32f1xx_hal::stm32, peripherals=true)]
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
        let ws2812 = ws2812::PwmPatternDB::new(tim2, dma1_c2, clocks);

        init::LateResources {
            ON_BOARD_LED: led,
            WS2812: ws2812,
            DISPLAY_BUFFER: led_matrix_8x8::LedMatrix8x8::new(color::Color::white()),
        }
    }

    #[idle(resources = [WS2812])]
    fn idle(mut c: idle::Context) -> ! {
        // Start the LEDs
        c.resources
            .WS2812
            .lock(|WS2812| WS2812.start(color::Color::red()));
        loop {
            cortex_m::asm::nop();
        }
    }

    #[task(binds=TIM2,resources = [ON_BOARD_LED, WS2812])]
    fn tim2(c: tim2::Context) {
        c.resources.ON_BOARD_LED.toggle().unwrap();
        c.resources.WS2812.reset_isr_tim();
    }

    #[task(binds=DMA1_CHANNEL2,resources = [WS2812])]
    fn dma1_channel2(c: dma1_channel2::Context) {
        c.resources.WS2812.reset_isr_dma();
    }

    extern "C" {
        fn EXTI0();
    }
};

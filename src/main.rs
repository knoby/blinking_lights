#![deny(unsafe_code)]
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

#[app(device = stm32f1xx_hal::stm32)]
const APP: () = {
    static mut ON_BOARD_LED: stm32f1xx_hal::gpio::gpioa::PA5<Output<PushPull>> = ();

    #[init(schedule=[cyclic])]
    fn init(c: init::Context) -> init::LateResources {
        // Freeze clock frequencies
        let mut flash = c.device.FLASH.constrain();
        let mut rcc = c.device.RCC.constrain();
        let _clocks = rcc
            .cfgr
            .sysclk(64.mhz())
            .pclk1(32.mhz())
            .pclk2(64.mhz())
            .freeze(&mut flash.acr);

        // Setup On Board LED
        let mut gpioa = c.device.GPIOA.split(&mut rcc.apb2);
        let led = gpioa.pa5.into_push_pull_output(&mut gpioa.crl);

        c.schedule
            .cyclic(rtfm::Instant::now() + 32_000_000.cycles())
            .unwrap();

        init::LateResources { ON_BOARD_LED: led }
    }

    #[idle()]
    fn idle(_c: idle::Context) -> ! {
        loop {
            cortex_m::asm::nop();
        }
    }

    #[task(schedule=[cyclic], resources=[ON_BOARD_LED])]
    fn cyclic(c: cyclic::Context) {
        #[allow(deprecated)]
        c.resources.ON_BOARD_LED.toggle();
        c.schedule
            .cyclic(rtfm::Instant::now() + 32_000_000.cycles())
            .unwrap();
    }

    extern "C" {
        fn EXTI0();
    }
};

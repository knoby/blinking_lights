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

mod ws2812;

#[app(device = stm32f1xx_hal::stm32)]
const APP: () = {
    static mut ON_BOARD_LED: stm32f1xx_hal::gpio::gpioa::PA5<Output<PushPull>> = ();
    static mut TIMER: ws2812::PwmPatternDB = ();
    static mut DMA_BUFFER: [u16; 48] = [0; 48];

    #[init(schedule=[cyclic], resources=[DMA_BUFFER])]
    fn init(mut c: init::Context) -> init::LateResources {
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
        let mut led = gpioa.pa5.into_push_pull_output(&mut gpioa.crl);

        // Setup the PWM Output
        let tim2 = stm32f1xx_hal::timer::Timer::tim2(c.device.TIM2, 1.hz(), clocks, &mut rcc.apb1);
        let dma1_c2 = c.device.DMA1.split(&mut rcc.ahb).2;
        let mut ws2812 = ws2812::PwmPatternDB::new(tim2, dma1_c2, clocks);

        c.schedule
            .cyclic(rtfm::Instant::now() + 32_000_000.cycles())
            .unwrap();

        ws2812.start(&mut c.resources.DMA_BUFFER);

        init::LateResources {
            ON_BOARD_LED: led,
            TIMER: ws2812,
        }
    }

    #[idle()]
    fn idle(_c: idle::Context) -> ! {
        loop {
            cortex_m::asm::nop();
        }
    }

    #[task(schedule=[cyclic], resources=[])]
    fn cyclic(c: cyclic::Context) {
        c.schedule
            .cyclic(rtfm::Instant::now() + 32_000_000.cycles())
            .unwrap();
    }

    #[interrupt( resources = [ON_BOARD_LED, TIMER])]
    fn TIM2(mut c: TIM2::Context) {
        if c.resources.TIMER.is_cmp_irq() {
            #[allow(deprecated)]
            c.resources.ON_BOARD_LED.set_low();
        } else {
            #[allow(deprecated)]
            c.resources.ON_BOARD_LED.set_high();
        }

        c.resources.TIMER.reset_isr_tim();
    }

    #[interrupt( resources = [TIMER])]
    fn DMA1_CHANNEL2(mut c: DMA1_CHANNEL2::Context) {
        c.resources.TIMER.reset_isr_dma();
    }

    extern "C" {
        fn EXTI0();
    }
};

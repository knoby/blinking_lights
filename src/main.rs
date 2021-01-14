#![no_main]
#![no_std]

// panic-handler crate
//extern crate panic_semihosting;
extern crate panic_halt;

use rtic::app;
use rtic::cyccnt::{Instant, U32Ext};
use stm32f1xx_hal::gpio::{Output, PushPull};
use stm32f1xx_hal::prelude::*;

mod color;
mod led_matrix_8x8;
mod ws2812;

use ws2812::InitBuffer;

#[app(device = stm32f1xx_hal::device, peripherals=true, monotonic=rtic::cyccnt::CYCCNT)]
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
            DISPLAY_BUFFER: ws2812::DisplayBuffer::new(),
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
    fn update_display(cx: update_display::Context) {
        static mut TOP_LED: usize = 0;

        for num in 0..64 {
            cx.resources.WS2812.set_color_pattern(
                color::Color::led_off(),
                num,
                cx.resources.DISPLAY_BUFFER,
            );
        }
        for num in 0..64 {
            cx.resources.WS2812.set_color_pattern(
                color::Color::new(*TOP_LED as u8, 0, 64 - *TOP_LED as u8),
                num,
                cx.resources.DISPLAY_BUFFER,
            );
        }

        cx.resources.WS2812.start(cx.resources.DISPLAY_BUFFER);
        while cx.resources.WS2812.is_active() {
            cortex_m::asm::nop();
        }
        cx.resources.WS2812.reset_isr_dma();
        cx.resources.WS2812.stop();
        if *TOP_LED < 64 {
            *TOP_LED += 1;
        } else {
            *TOP_LED = 0;
        }
        cx.schedule
            .update_display(Instant::now() + 6_400_000.cycles())
            .unwrap();
    }

    extern "C" {
        fn EXTI0();
    }
};

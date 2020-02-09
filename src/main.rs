#![no_std]
#![no_main]

extern crate panic_halt;

use cortex_m::asm::delay;
use stm32l0xx_hal::gpio::{gpiob::PB11, Output, PushPull};
use stm32l0xx_hal::{prelude::*, rcc};

#[rtfm::app(device=stm32l0::stm32l0x3, peripherals=true)]
const APP: () = {
    struct Resources {
        status_led: PB11<Output<PushPull>>,
    }

    #[init]
    fn init(cx: init::Context) -> init::LateResources {
        let peripherals = cx.device;

        let clock_config = rcc::Config::pll(
            rcc::PLLSource::HSE(12.mhz()),
            rcc::PLLMul::Mul8,
            rcc::PLLDiv::Div4,
        );

        let mut rcc = peripherals.RCC.freeze(clock_config);
        let gpiob = peripherals.GPIOB.split(&mut rcc);
        let status_led = gpiob.pb11.into_push_pull_output();

        init::LateResources { status_led }
    }

    #[idle(resources=[status_led])]
    fn idle(cx: idle::Context) -> ! {
        loop {
            cx.resources.status_led.set_high().unwrap();
            delay(12000000);
            cx.resources.status_led.set_low().unwrap();
            delay(12000000);
        }
    }
};

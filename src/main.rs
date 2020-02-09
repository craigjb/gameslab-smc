#![no_std]
#![no_main]

extern crate panic_halt;

use stm32l0xx_hal::gpio::{gpiob::PB11, Output, PushPull};
use stm32l0xx_hal::{prelude::*, rcc, timer::Timer};

#[rtfm::app(device=stm32l0::stm32l0x3, peripherals=true)]
const APP: () = {
    struct Resources {
        status_led: PB11<Output<PushPull>>,
        led_timer: Timer<stm32l0::stm32l0x3::TIM2>,
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

        let mut led_timer = peripherals.TIM2.timer(2.hz(), &mut rcc);
        led_timer.listen();

        init::LateResources {
            status_led,
            led_timer,
        }
    }

    #[task(binds=TIM2, resources=[led_timer, status_led])]
    fn tim2_interrupt(cx: tim2_interrupt::Context) {
        cx.resources.led_timer.clear_irq();
        if cx.resources.status_led.is_set_high().unwrap() {
            cx.resources.status_led.set_low().unwrap();
        } else {
            cx.resources.status_led.set_high().unwrap();
        }
    }
};

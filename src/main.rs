#![no_std]
#![no_main]

extern crate panic_halt;

mod leds;
mod switch;
mod zynq;

use stm32l0::stm32l0x3 as pac;
use stm32l0xx_hal::{prelude::*, rcc, syscfg::SYSCFG};

#[rtfm::app(device=stm32l0::stm32l0x3, peripherals=true)]
const APP: () = {
    struct Resources {
        #[init(0)]
        tick: u32,
        leds: leds::LedsState,
        switch: switch::SwitchState,
        zynq: zynq::ZynqState,
    }

    #[init]
    fn init(cx: init::Context) -> init::LateResources {
        let core = cx.core;
        let mut peripherals = cx.device;

        let clock_config = rcc::Config::pll(
            rcc::PLLSource::HSE(12.mhz()),
            rcc::PLLMul::Mul8,
            rcc::PLLDiv::Div4,
        );

        // flash needs a wait state for this speed
        peripherals.FLASH.acr.modify(|_, w| w.latency().set_bit());

        let mut rcc = peripherals.RCC.freeze(clock_config);
        let mut syscfg = SYSCFG::new(peripherals.SYSCFG, &mut rcc);
        let gpiob = peripherals.GPIOB.split(&mut rcc);
        let gpioc = peripherals.GPIOC.split(&mut rcc);

        let mut tick_timer = core.SYST.timer(10.hz(), &mut rcc);
        tick_timer.listen();

        let mut leds = leds::LedsState::new(gpiob.pb10, gpiob.pb11, peripherals.TIM2, &mut rcc);
        leds.charge_blink();

        let pb0 = gpiob.pb0.into_floating_input();
        let switch = switch::SwitchState::new(pb0, &mut peripherals.EXTI, &mut syscfg);
        let zynq = zynq::ZynqState::new(
            gpioc.pc0, gpioc.pc1, gpioc.pc2, gpioc.pc3, gpioc.pc4, gpioc.pc5, gpioc.pc6, gpioc.pc7,
            gpioc.pc8,
        );

        init::LateResources { leds, switch, zynq }
    }

    #[task(binds=SysTick, resources=[tick, leds, zynq])]
    fn tick_100ms(cx: tick_100ms::Context) {
        *cx.resources.tick += 1;
        cx.resources.leds.tick(*cx.resources.tick);
        cx.resources.zynq.tick(*cx.resources.tick);
    }

    #[task(binds = EXTI0_1, resources=[switch, tick, leds, zynq])]
    fn interrupt_exti0_1(cx: interrupt_exti0_1::Context) {
        if cx.resources.switch.was_toggled(*cx.resources.tick) {
            cx.resources.zynq.power_toggle();
            if cx.resources.zynq.is_power_on() {
                cx.resources.leds.status_on();
            } else {
                cx.resources.leds.status_off();
            }
        }
    }
};

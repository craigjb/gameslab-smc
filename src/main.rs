#![no_std]
#![no_main]

extern crate panic_halt;

mod battery;
mod leds;
mod switch;
mod uart;
mod usb;
mod zynq;

use hal::{exti::Exti, prelude::*, rcc, syscfg::SYSCFG};
use stm32l0::stm32l0x3 as pac;
use stm32l0xx_hal as hal;

#[rtfm::app(device=stm32l0::stm32l0x3, peripherals=true)]
const APP: () = {
    struct Resources {
        #[init(0)]
        tick: u32,
        status_led: leds::StatusLed,
        switch: switch::SwitchState,
        zynq: zynq::ZynqState,
        usb: usb::UsbState,
        uart: uart::UartState,
        battery: battery::BatteryState,
    }

    #[init]
    fn init(cx: init::Context) -> init::LateResources {
        let core = cx.core;
        let peripherals = cx.device;

        let clock_config = rcc::Config::pll(
            rcc::PLLSource::HSE(12.mhz()),
            rcc::PLLMul::Mul8,
            rcc::PLLDiv::Div4,
        );

        // flash needs a wait state for >= 16 MHz sysclock
        // HAL doesn't support this part yet in the flash module
        peripherals.FLASH.acr.modify(|_, w| w.latency().set_bit());

        let mut rcc = peripherals.RCC.freeze(clock_config);
        let mut syscfg = SYSCFG::new(peripherals.SYSCFG, &mut rcc);
        let gpioa = peripherals.GPIOA.split(&mut rcc);
        let gpiob = peripherals.GPIOB.split(&mut rcc);
        let gpioc = peripherals.GPIOC.split(&mut rcc);

        let mut tick_timer = core.SYST.timer(10.hz(), &mut rcc);
        tick_timer.listen();

        let (status_led, charge_led) =
            leds::create_leds(gpiob.pb10, gpiob.pb11, peripherals.TIM2, &mut rcc);
        let mut exti = Exti::new(peripherals.EXTI);
        let switch =
            switch::SwitchState::new(gpiob.pb0.into_floating_input(), &mut exti, &mut syscfg);
        let zynq = zynq::ZynqState::new(
            gpioc.pc0.into_push_pull_output(),
            gpioc.pc1.into_push_pull_output(),
            gpioc.pc2.into_push_pull_output(),
            gpioc.pc3.into_push_pull_output(),
            gpioc.pc4.into_floating_input(),
            gpioc.pc5.into_floating_input(),
            gpioc.pc6.into_floating_input(),
            gpioc.pc7.into_floating_input(),
            gpioc.pc8.into_push_pull_output(),
        );
        let battery = battery::BatteryState::new(
            peripherals.I2C1,
            gpiob.pb8,
            gpiob.pb9,
            charge_led,
            &mut rcc,
        );
        let usb = usb::UsbState::new(peripherals.USB, gpioa.pa11, gpioa.pa12, &rcc);
        let uart = uart::UartState::new(
            peripherals.LPUART1,
            gpioc.pc10,
            gpioc.pc11,
            peripherals.DMA1,
            &mut rcc,
        );

        init::LateResources {
            status_led,
            switch,
            zynq,
            usb,
            uart,
            battery,
        }
    }

    #[idle(resources=[uart, battery])]
    fn idle(mut cx: idle::Context) -> ! {
        loop {
            cx.resources
                .battery
                .lock(|battery| battery.update_if_needed());
        }
    }

    #[task(binds=USB, priority=3, resources=[usb, uart])]
    fn interrupt_usb(cx: interrupt_usb::Context) {
        cx.resources.usb.poll();
        cx.resources.uart.interrupt_usb(cx.resources.usb);
    }

    #[task(binds=DMA1_CHANNEL2_3, priority=3, resources=[uart, usb])]
    fn interrupt_dma(mut cx: interrupt_dma::Context) {
        cx.resources.uart.interrupt_dma(&mut cx.resources.usb);
    }

    #[task(binds=AES_RNG_LPUART1, priority=3, resources=[uart, usb])]
    fn interrupt_lpuart(mut cx: interrupt_lpuart::Context) {
        cx.resources.uart.interrupt_lpuart(&mut cx.resources.usb);
    }

    #[task(binds=SysTick, priority=2, resources=[tick, zynq, battery])]
    fn tick_100ms(cx: tick_100ms::Context) {
        *cx.resources.tick += 1;
        cx.resources.battery.tick(*cx.resources.tick);
        cx.resources.zynq.tick(*cx.resources.tick);
    }

    #[task(binds = EXTI0_1, priority=2, resources=[switch, tick, status_led, zynq])]
    fn interrupt_exti0_1(cx: interrupt_exti0_1::Context) {
        if cx.resources.switch.was_toggled(*cx.resources.tick) {
            cx.resources.zynq.power_toggle();
            if cx.resources.zynq.is_power_on() {
                cx.resources.status_led.on();
            } else {
                cx.resources.status_led.off();
            }
        }
    }
};

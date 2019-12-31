#![no_std]
#![no_main]

extern crate panic_halt;

use cortex_m::asm::delay;
use cortex_m_rt::entry;
use stm32l0::stm32l0x3;
use stm32l0xx_hal::{prelude::*, rcc};

#[entry]
fn main() -> ! {
    let peripherals = stm32l0x3::Peripherals::take().unwrap();

    let clock_config = rcc::Config::pll(
        rcc::PLLSource::HSE(12.mhz()),
        rcc::PLLMul::Mul8,
        rcc::PLLDiv::Div4,
    );
    let mut rcc = peripherals.RCC.freeze(clock_config);
    let gpiob = peripherals.GPIOB.split(&mut rcc);
    let mut status_led = gpiob.pb11.into_push_pull_output();

    loop {
        status_led.set_high().unwrap();
        delay(12000000);
        status_led.set_low().unwrap();
        delay(12000000);
    }
}

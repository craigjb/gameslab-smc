use crate::pac::{CorePeripherals, PWR, RCC};
use crate::pac::{GPIOA, GPIOB, GPIOC};
use cortex_m::asm::{dsb, wfi};

pub struct PowerState {
    power_state: bool,
    usb_connected: bool,
    hseon: bool,
    pllon: bool,
    sw_bits: u8,
    gpioa_mode: Option<u32>,
    gpiob_mode: Option<u32>,
    gpioc_mode: Option<u32>,
}

static mut POWER_STATE: PowerState = PowerState {
    power_state: false,
    usb_connected: false,
    sw_bits: 0,
    hseon: false,
    pllon: false,
    gpioa_mode: None,
    gpiob_mode: None,
    gpioc_mode: None,
};

pub fn set_sleep_power_state(state: bool) {
    unsafe {
        POWER_STATE.power_state = state;
    }
}

pub fn set_usb_connected(state: bool) {
    unsafe {
        POWER_STATE.usb_connected = state;
    }
}

pub fn init() {
    let rcc = unsafe { &*RCC::ptr() };
    rcc.apb1enr.modify(|_, w| w.pwren().set_bit());
}

pub fn sleep_if_needed() -> bool {
    if unsafe { POWER_STATE.power_state || POWER_STATE.usb_connected } {
        return false;
    }

    let rcc = unsafe { &*RCC::ptr() };

    unsafe {
        let core = &mut CorePeripherals::steal();
        core.SYST.disable_counter();
        core.SYST.disable_interrupt();
        core.SCB.set_sleepdeep();
    }

    // Save current clock states
    unsafe {
        POWER_STATE.sw_bits = rcc.cfgr.read().sw().bits();
        POWER_STATE.hseon = rcc.cr.read().hseon().bit_is_set();
        POWER_STATE.pllon = rcc.cr.read().pllon().bit_is_set();
    }

    prepare_gpio_for_sleep();

    // Switch internal OSC to HSI
    rcc.cfgr.modify(|_, w| w.sw().bits(0b01));
    while rcc.cfgr.read().sw().bits() != 0b01 {}

    // Set to wake-up using HSI
    rcc.cfgr.modify(|_, w| w.stopwuck().set_bit());

    // Configure Stop mode
    let pwr = unsafe { &*PWR::ptr() };
    pwr.cr.modify(|_, w| {
        w.ulp()
            .set_bit()
            .cwuf()
            .set_bit()
            .pdds()
            .stop_mode()
            .lpsdsr()
            .low_power_mode()
    });

    // Wait for WUF to be cleared
    while pwr.csr.read().wuf().bit_is_set() {}

    // Enter Stop mode
    dsb();
    wfi();

    wake_gpio_from_sleep();
    handle_wakeup();
    return true;
}

fn prepare_gpio_for_sleep() {
    unsafe {
        let gpioa = &*GPIOA::ptr();
        let gpioa_mode = gpioa.moder.read().bits();
        POWER_STATE.gpioa_mode = Some(gpioa_mode);
        let gpiob = &*GPIOB::ptr();
        let gpiob_mode = gpiob.moder.read().bits();
        POWER_STATE.gpiob_mode = Some(gpiob_mode);
        let gpioc = &*GPIOC::ptr();
        POWER_STATE.gpioc_mode = Some(gpioc.moder.read().bits());

        gpioa.moder.write(|w| w.bits(0xFFCFFFFF | gpioa_mode));
        gpiob.moder.write(|w| w.bits(0xFFFFFFFC | gpiob_mode));
        gpioc.moder.write(|w| w.bits(0xFFFFFFFF));
    }
}

fn wake_gpio_from_sleep() {
    unsafe {
        if let Some(mode) = POWER_STATE.gpioa_mode {
            let gpioa = &*GPIOA::ptr();
            gpioa.moder.write(|w| w.bits(mode));
        }
        if let Some(mode) = POWER_STATE.gpiob_mode {
            let gpiob = &*GPIOB::ptr();
            gpiob.moder.write(|w| w.bits(mode));
        }
        if let Some(mode) = POWER_STATE.gpioc_mode {
            let gpioc = &*GPIOC::ptr();
            gpioc.moder.write(|w| w.bits(mode));
        }
    }
}

fn handle_wakeup() {
    let rcc = unsafe { &*RCC::ptr() };

    if unsafe { POWER_STATE.hseon } {
        // Enable HSE
        rcc.cr.modify(|_, w| w.hseon().set_bit());
        while rcc.cr.read().hserdy().bit_is_clear() {}
    }

    if unsafe { POWER_STATE.pllon } {
        rcc.cr.modify(|_, w| w.pllon().set_bit());
        // Wait for PLL if enabled
        while rcc.cr.read().pllrdy().bit_is_clear() {}
    }

    // Switch back to original clock source
    rcc.cfgr
        .modify(|_, w| w.sw().bits(unsafe { POWER_STATE.sw_bits }));
    while rcc.cfgr.read().sw().bits() != unsafe { POWER_STATE.sw_bits } {}

    unsafe {
        let core = &mut CorePeripherals::steal();
        core.SYST.enable_counter();
        core.SYST.enable_interrupt();
    }
}

use crate::hal::{
    gpio::{
        gpiob::{PB8, PB9},
        Analog, OpenDrain, Output,
    },
    i2c::I2c,
    prelude::*,
    rcc::Rcc,
};
use crate::leds::ChargeLed;
use crate::pac::I2C1;

pub struct BatteryState {
    i2c: I2c<I2C1, PB9<Output<OpenDrain>>, PB8<Output<OpenDrain>>>,
    charge_led: ChargeLed,
    buffer: [u8; 2],
    should_update: bool,
    last_update: u32,
}

const BQ24250_ADDR: u8 = 0x6A;
const STC3115_ADDR: u8 = 0x70;
const UPDATE_INTERVAL: u32 = 5;

impl BatteryState {
    pub fn new(
        i2c1: I2C1,
        scl: PB8<Analog>,
        sda: PB9<Analog>,
        charge_led: ChargeLed,
        rcc: &mut Rcc,
    ) -> Self {
        let i2c = i2c1.i2c(
            sda.into_open_drain_output(),
            scl.into_open_drain_output(),
            400.khz(),
            rcc,
        );
        Self {
            i2c,
            charge_led,
            buffer: [0; 2],
            last_update: 0,
            should_update: true,
        }
    }

    pub fn update_if_needed(&mut self) {
        if !self.should_update {
            return;
        }
        self.should_update = false;
        self.i2c
            .write_read(BQ24250_ADDR, &[0x0], &mut self.buffer[0..1])
            .unwrap();
        match (self.buffer[0] & 0x30) >> 4 {
            0 => self.charge_led.off(),
            1 => self.charge_led.blink(),
            2 => self.charge_led.on(),
            _ => self.charge_led.off(),
        }
    }

    pub fn tick(&mut self, tick: u32) {
        self.charge_led.tick(tick);
        if tick >= self.last_update + UPDATE_INTERVAL {
            self.should_update = true;
            self.last_update = tick;
        }
    }
}

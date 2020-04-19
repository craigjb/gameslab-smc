use super::power;
use stm32l0xx_hal::{
    gpio::{
        gpioc::{PC0, PC1, PC2, PC3, PC4, PC5, PC6, PC7, PC8},
        Floating, Input, Output, PushPull,
    },
    prelude::*,
};

pub struct ZynqState {
    power_supplies: PowerSupplies,
    power_state: PowerState,
}

struct PowerSupplies {
    en_1v0: PC0<Output<PushPull>>,
    pg_1v0: PC4<Input<Floating>>,
    en_1v5: PC1<Output<PushPull>>,
    pg_1v5: PC5<Input<Floating>>,
    en_1v8: PC2<Output<PushPull>>,
    pg_1v8: PC6<Input<Floating>>,
    en_3v3: PC3<Output<PushPull>>,
    pg_3v3: PC7<Input<Floating>>,
    zynq_por: PC8<Output<PushPull>>,
}

#[derive(Clone)]
enum PowerState {
    Stage0Up,
    Stage1Up,
    Stage2Up,
    Stage3Up,
    On,
    Off,
    Stage3Down,
    Stage2Down,
    Stage1Down,
    Stage0Down,
}

impl ZynqState {
    pub fn new(
        pc0: PC0<Output<PushPull>>,
        pc1: PC1<Output<PushPull>>,
        pc2: PC2<Output<PushPull>>,
        pc3: PC3<Output<PushPull>>,
        pc4: PC4<Input<Floating>>,
        pc5: PC5<Input<Floating>>,
        pc6: PC6<Input<Floating>>,
        pc7: PC7<Input<Floating>>,
        pc8: PC8<Output<PushPull>>,
    ) -> Self {
        Self {
            power_supplies: PowerSupplies {
                en_1v0: pc0,
                pg_1v0: pc4,
                en_1v5: pc1,
                pg_1v5: pc5,
                en_1v8: pc2,
                pg_1v8: pc6,
                en_3v3: pc3,
                pg_3v3: pc7,
                zynq_por: pc8,
            },
            power_state: PowerState::Off,
        }
    }

    pub fn power_up(&mut self) {
        power::set_sleep_power_state(true);
        self.power_state = match self.power_state {
            // if we're already powering on, don't do anything
            PowerState::On
            | PowerState::Stage0Up
            | PowerState::Stage1Up
            | PowerState::Stage2Up
            | PowerState::Stage3Up => self.power_state.clone(),
            PowerState::Off
            | PowerState::Stage3Down
            | PowerState::Stage2Down
            | PowerState::Stage1Down
            | PowerState::Stage0Down => {
                // if we're off, start the sequence
                self.power_supplies.zynq_por.set_low().unwrap();
                PowerState::Stage0Up
            }
        };
    }

    pub fn power_down(&mut self) {
        power::set_sleep_power_state(false);
        self.power_state = match self.power_state {
            PowerState::Off
            | PowerState::Stage3Down
            | PowerState::Stage2Down
            | PowerState::Stage1Down
            | PowerState::Stage0Down => self.power_state.clone(),
            PowerState::On
            | PowerState::Stage0Up
            | PowerState::Stage1Up
            | PowerState::Stage2Up
            | PowerState::Stage3Up => {
                // if we're on, start the sequence
                self.power_supplies.zynq_por.set_low().unwrap();
                PowerState::Stage3Down
            }
        }
    }

    pub fn power_toggle(&mut self) {
        match self.power_state {
            PowerState::Stage3Down
            | PowerState::Stage2Down
            | PowerState::Stage1Down
            | PowerState::Stage0Down
            | PowerState::Off => self.power_up(),
            PowerState::On
            | PowerState::Stage0Up
            | PowerState::Stage1Up
            | PowerState::Stage2Up
            | PowerState::Stage3Up => self.power_down(),
        }
    }

    pub fn is_power_on(&self) -> bool {
        match self.power_state {
            PowerState::Stage3Down
            | PowerState::Stage2Down
            | PowerState::Stage1Down
            | PowerState::Stage0Down
            | PowerState::Off => false,
            PowerState::On
            | PowerState::Stage0Up
            | PowerState::Stage1Up
            | PowerState::Stage2Up
            | PowerState::Stage3Up => true,
        }
    }

    pub fn tick(&mut self, _: u32) {
        self.power_state = match self.power_state {
            PowerState::On | PowerState::Off => self.power_state.clone(),
            PowerState::Stage0Up => {
                self.power_supplies.en_1v0.set_high().unwrap();
                PowerState::Stage1Up
            }
            PowerState::Stage1Up => {
                if self.power_supplies.pg_1v0.is_high().unwrap() {
                    self.power_supplies.en_1v8.set_high().unwrap();
                    PowerState::Stage2Up
                } else {
                    self.power_state.clone()
                }
            }
            PowerState::Stage2Up => {
                if self.power_supplies.pg_1v8.is_high().unwrap() {
                    self.power_supplies.en_1v5.set_high().unwrap();
                    self.power_supplies.en_3v3.set_high().unwrap();
                    PowerState::Stage3Up
                } else {
                    self.power_state.clone()
                }
            }
            PowerState::Stage3Up => {
                if self.power_supplies.pg_1v5.is_high().unwrap()
                    && self.power_supplies.pg_3v3.is_high().unwrap()
                {
                    self.power_supplies.zynq_por.set_high().unwrap();
                    PowerState::On
                } else {
                    self.power_state.clone()
                }
            }
            PowerState::Stage3Down => {
                self.power_supplies.en_1v5.set_low().unwrap();
                self.power_supplies.en_3v3.set_low().unwrap();
                PowerState::Stage2Down
            }
            PowerState::Stage2Down => {
                if self.power_supplies.pg_1v5.is_low().unwrap()
                    && self.power_supplies.pg_3v3.is_low().unwrap()
                {
                    self.power_supplies.en_1v8.set_low().unwrap();
                    PowerState::Stage1Down
                } else {
                    self.power_state.clone()
                }
            }
            PowerState::Stage1Down => {
                if self.power_supplies.pg_1v8.is_low().unwrap() {
                    self.power_supplies.en_1v0.set_low().unwrap();
                    PowerState::Stage0Down
                } else {
                    self.power_state.clone()
                }
            }
            PowerState::Stage0Down => {
                if self.power_supplies.pg_1v0.is_low().unwrap() {
                    PowerState::Off
                } else {
                    self.power_state.clone()
                }
            }
        }
    }
}

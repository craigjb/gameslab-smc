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
}

impl ZynqState {
    pub fn new(
        pc0: PC0<Input<Floating>>,
        pc1: PC1<Input<Floating>>,
        pc2: PC2<Input<Floating>>,
        pc3: PC3<Input<Floating>>,
        pc4: PC4<Input<Floating>>,
        pc5: PC5<Input<Floating>>,
        pc6: PC6<Input<Floating>>,
        pc7: PC7<Input<Floating>>,
        pc8: PC8<Input<Floating>>,
    ) -> Self {
        Self {
            power_supplies: PowerSupplies {
                en_1v0: pc0.into_push_pull_output(),
                pg_1v0: pc4.into_floating_input(),
                en_1v5: pc1.into_push_pull_output(),
                pg_1v5: pc5.into_floating_input(),
                en_1v8: pc2.into_push_pull_output(),
                pg_1v8: pc6.into_floating_input(),
                en_3v3: pc3.into_push_pull_output(),
                pg_3v3: pc7.into_floating_input(),
                zynq_por: pc8.into_push_pull_output(),
            },
            power_state: PowerState::Off,
        }
    }

    pub fn power_up(&mut self) {
        self.power_state = match self.power_state {
            // if we're already powering on, don't do anything
            PowerState::On
            | PowerState::Stage0Up
            | PowerState::Stage1Up
            | PowerState::Stage2Up
            | PowerState::Stage3Up => self.power_state.clone(),
            PowerState::Off => {
                // if we're off, start the sequence
                self.power_supplies.zynq_por.set_low().unwrap();
                PowerState::Stage0Up
            }
        };
    }

    pub fn power_toggle(&mut self) {
        match self.power_state {
            PowerState::Off => self.power_up(),
            PowerState::On
            | PowerState::Stage0Up
            | PowerState::Stage1Up
            | PowerState::Stage2Up
            | PowerState::Stage3Up => {}
        }
    }

    pub fn is_power_on(&self) -> bool {
        match self.power_state {
            PowerState::Off => false,
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
        }
    }
}

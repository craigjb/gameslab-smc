use crate::pac::EXTI;
use stm32l0xx_hal::{
    exti::{
        line::{ExtiLine, GpioLine},
        TriggerEdge,
    },
    gpio::{gpiob::PB0, Floating, Input, Port},
    prelude::*,
    syscfg::SYSCFG,
};

pub struct SwitchState {
    last_event: u32,
}

const DEBOUNCE_TIME: u32 = 3;

impl SwitchState {
    pub fn new(_pb0: PB0<Input<Floating>>, exti: &mut EXTI, syscfg: &mut SYSCFG) -> Self {
        exti.listen_gpio(
            syscfg,
            Port::PB,
            GpioLine::from_raw_line(0).unwrap(),
            TriggerEdge::Falling,
        );
        Self { last_event: 0 }
    }

    pub fn was_toggled(&mut self, tick: u32) -> bool {
        if EXTI::is_pending(GpioLine::from_raw_line(0).unwrap()) {
            EXTI::unpend(GpioLine::from_raw_line(0).unwrap());
            if tick > self.last_event + DEBOUNCE_TIME {
                self.last_event = tick;
                true
            } else {
                false
            }
        } else {
            false
        }
    }
}

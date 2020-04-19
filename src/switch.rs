use stm32l0xx_hal::{
    exti::{Exti, ExtiLine, GpioLine, TriggerEdge},
    gpio::{gpiob::PB0, Floating, Input, Port},
    syscfg::SYSCFG,
};

pub struct SwitchState {}

impl SwitchState {
    pub fn new(_pb0: PB0<Input<Floating>>, exti: &mut Exti, syscfg: &mut SYSCFG) -> Self {
        exti.listen_gpio(
            syscfg,
            Port::PB,
            GpioLine::from_raw_line(0).unwrap(),
            TriggerEdge::Falling,
        );
        Self {}
    }

    pub fn was_toggled(&mut self) -> bool {
        if Exti::is_pending(GpioLine::from_raw_line(0).unwrap()) {
            Exti::unpend(GpioLine::from_raw_line(0).unwrap());
            true
        } else {
            false
        }
    }
}

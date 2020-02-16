use crate::pac::TIM2;
use stm32l0xx_hal::{
    gpio::{
        gpiob::{PB10, PB11},
        Analog,
    },
    prelude::*,
    pwm,
    rcc::Rcc,
};

pub struct LedsState {
    status: pwm::Pwm<TIM2, pwm::C4, pwm::Assigned<PB11<Analog>>>,
    charge: pwm::Pwm<TIM2, pwm::C3, pwm::Assigned<PB10<Analog>>>,
    charge_blinking: bool,
    charge_blinking_up: bool,
    charge_blink_index: usize,
}

const BLINK_MAX_DUTY: u16 = 600;
const BLINK_MIN_DUTY: u16 = 20;
const BLINK_DUTY_TABLE: [u16; 29] = [
    0, 1, 1, 1, 2, 3, 4, 5, 6, 8, 10, 13, 16, 21, 26, 32, 41, 51, 64, 80, 99, 124, 154, 193, 240,
    299, 373, 465, 579,
];
const STATUS_MAX_DUTY: u16 = 400;

impl LedsState {
    pub fn new(pb10: PB10<Analog>, pb11: PB11<Analog>, tim2: TIM2, rcc: &mut Rcc) -> Self {
        let timer2 = pwm::Timer::new(tim2, 10.khz(), rcc);
        let mut status = timer2.channel4.assign(pb11);
        let mut charge = timer2.channel3.assign(pb10);

        status.enable();
        charge.enable();

        Self {
            status,
            charge,
            charge_blinking: false,
            charge_blinking_up: true,
            charge_blink_index: 0,
        }
    }

    pub fn status_on(&mut self) {
        self.status.set_duty(STATUS_MAX_DUTY);
    }

    pub fn status_off(&mut self) {
        self.status.set_duty(0);
    }

    pub fn charge_on(&mut self) {
        self.charge.set_duty(BLINK_MAX_DUTY);
        self.charge_blinking = false;
    }

    pub fn charge_off(&mut self) {
        self.charge.set_duty(0);
        self.charge_blinking = false;
    }

    pub fn charge_blink(&mut self) {
        self.charge_blinking = true;
        self.charge_blinking_up = true;
        self.charge_blink_index = 0;
        self.charge.set_duty(BLINK_MIN_DUTY);
    }

    pub fn tick(&mut self, _: u32) {
        if !self.charge_blinking {
            return;
        }
        if self.charge_blinking_up {
            self.charge_blink_index += 1;
            if self.charge_blink_index >= BLINK_DUTY_TABLE.len() - 1 {
                self.charge_blinking_up = false;
            }
        } else {
            self.charge_blink_index -= 1;
            if self.charge_blink_index == 0 {
                self.charge_blinking_up = true;
            }
        }
        let duty = BLINK_DUTY_TABLE[self.charge_blink_index];
        self.charge.set_duty(BLINK_MIN_DUTY + duty);
    }
}

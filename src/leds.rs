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

pub struct StatusLed {
    pwm: pwm::Pwm<TIM2, pwm::C4, pwm::Assigned<PB11<Analog>>>,
}

pub struct ChargeLed {
    pwm: pwm::Pwm<TIM2, pwm::C3, pwm::Assigned<PB10<Analog>>>,
    blinking: bool,
    blinking_up: bool,
    blink_index: usize,
}

pub fn create_leds(
    pb10: PB10<Analog>,
    pb11: PB11<Analog>,
    tim2: TIM2,
    rcc: &mut Rcc,
) -> (StatusLed, ChargeLed) {
    let timer2 = pwm::Timer::new(tim2, 10.khz(), rcc);
    let mut status = timer2.channel4.assign(pb11);
    let mut charge = timer2.channel3.assign(pb10);

    status.enable();
    charge.enable();

    (
        StatusLed { pwm: status },
        ChargeLed {
            pwm: charge,
            blinking: false,
            blinking_up: true,
            blink_index: 0,
        },
    )
}

const BLINK_MAX_DUTY: u16 = 600;
const BLINK_MIN_DUTY: u16 = 20;
const BLINK_DUTY_TABLE: [u16; 29] = [
    0, 1, 1, 1, 2, 3, 4, 5, 6, 8, 10, 13, 16, 21, 26, 32, 41, 51, 64, 80, 99, 124, 154, 193, 240,
    299, 373, 465, 579,
];
const STATUS_MAX_DUTY: u16 = 400;

impl StatusLed {
    pub fn on(&mut self) {
        self.pwm.set_duty(STATUS_MAX_DUTY);
    }

    pub fn off(&mut self) {
        self.pwm.set_duty(0);
    }
}

impl ChargeLed {
    pub fn on(&mut self) {
        self.pwm.set_duty(BLINK_MAX_DUTY);
        self.blinking = false;
    }

    pub fn off(&mut self) {
        self.pwm.set_duty(0);
        self.blinking = false;
    }

    pub fn blink(&mut self) {
        if !self.blinking {
            self.blinking = true;
            self.blinking_up = true;
            self.blink_index = 0;
            self.pwm.set_duty(BLINK_MIN_DUTY);
        }
    }

    pub fn tick(&mut self, _: u32) {
        if !self.blinking {
            return;
        }
        if self.blinking_up {
            self.blink_index += 1;
            if self.blink_index >= BLINK_DUTY_TABLE.len() - 1 {
                self.blinking_up = false;
            }
        } else {
            self.blink_index -= 1;
            if self.blink_index == 0 {
                self.blinking_up = true;
            }
        }
        let duty = BLINK_DUTY_TABLE[self.blink_index];
        self.pwm.set_duty(BLINK_MIN_DUTY + duty);
    }
}

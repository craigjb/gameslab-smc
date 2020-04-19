use super::power;
use crate::hal::{
    exti::{Exti, ExtiLine, GpioLine, TriggerEdge},
    gpio::{
        gpioa::{PA10, PA11, PA12},
        Analog, Floating, Input, Port,
    },
    rcc::Rcc,
    syscfg::SYSCFG,
    usb::{UsbBus, USB},
};
use crate::pac;
use embedded_hal::digital::v2::InputPin;
use usb_device::{bus::UsbBusAllocator, prelude::*};
use usbd_serial::{SerialPort, USB_CLASS_CDC};

static mut USB_BUS: Option<UsbBusAllocator<UsbBus<USB>>> = None;

pub struct UsbState {
    device: UsbDevice<'static, UsbBus<USB>>,
    serial: SerialPort<'static, UsbBus<USB>>,
    usb_detect: PA10<Input<Floating>>,
}

impl UsbState {
    pub fn new(
        usb: pac::USB,
        pa11: PA11<Analog>,
        pa12: PA12<Analog>,
        rcc: &Rcc,
        pa10: PA10<Input<Floating>>,
        exti: &mut Exti,
        syscfg: &mut SYSCFG,
    ) -> Self {
        exti.listen_gpio(
            syscfg,
            Port::PA,
            GpioLine::from_raw_line(10).unwrap(),
            TriggerEdge::Both,
        );

        let usb = USB::new_with_pll(usb, pa11, pa12, rcc);
        unsafe { USB_BUS = Some(UsbBus::new(usb)) };

        let serial = SerialPort::new(unsafe { USB_BUS.as_ref().unwrap() });

        let device = UsbDeviceBuilder::new(
            unsafe { USB_BUS.as_ref().unwrap() },
            UsbVidPid(0x5824, 0x27dd),
        )
        .manufacturer("craigjb.com")
        .product("Gameslab")
        .serial_number("0.1.1")
        .device_class(USB_CLASS_CDC)
        .max_power(500)
        .build();

        power::set_usb_connected(pa10.is_high().unwrap());
        device.bus().force_reenumeration(|| {});
        UsbState {
            device,
            serial,
            usb_detect: pa10,
        }
    }

    pub fn reset(&mut self) {
        self.device.bus().force_reenumeration(|| {});
    }

    pub fn poll(&mut self) {
        self.device.poll(&mut [&mut self.serial]);
    }

    pub fn write_uart_data(&mut self, data: &[u8]) {
        match self.serial.write(data) {
            _ => {}
        };
    }

    pub fn read_usb_data(&mut self, data: &mut [u8]) -> usize {
        match self.serial.read(data) {
            Ok(c) => c,
            Err(UsbError::WouldBlock) => 0,
            Err(_) => panic!(),
        }
    }

    pub fn handle_detect_interrupt(&mut self) {
        if Exti::is_pending(GpioLine::from_raw_line(10).unwrap()) {
            Exti::unpend(GpioLine::from_raw_line(10).unwrap());
            power::set_usb_connected(self.usb_detect.is_high().unwrap());
        }
    }
}

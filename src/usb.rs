use crate::hal::{
    gpio::{
        gpioa::{PA11, PA12},
        Analog,
    },
    rcc::Rcc,
    usb::{UsbBus, USB},
};
use crate::pac;
use usb_device::{bus::UsbBusAllocator, prelude::*};
use usbd_serial::{SerialPort, USB_CLASS_CDC};

static mut USB_BUS: Option<UsbBusAllocator<UsbBus<USB>>> = None;

pub struct UsbState {
    device: UsbDevice<'static, UsbBus<USB>>,
    serial: SerialPort<'static, UsbBus<USB>>,
}

impl UsbState {
    pub fn new(usb: pac::USB, pa11: PA11<Analog>, pa12: PA12<Analog>, rcc: &Rcc) -> Self {
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

        device.bus().force_reenumeration(|| {});
        UsbState { device, serial }
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
}

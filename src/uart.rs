use crate::hal::{
    dma::{Channel, Interrupts, DMA},
    gpio::{
        gpioc::{PC10, PC11},
        Analog,
    },
    prelude::*,
    rcc::Rcc,
    serial,
    serial::{Event, Serial1LpExt},
};
use crate::pac::{
    dma1::ch::cr::{DIR_A, PL_A},
    DMA1, LPUART1,
};
use crate::usb::UsbState;
use nb::block;

pub struct UartState {
    dma: DMA,
    rx: serial::Rx<LPUART1>,
    tx: serial::Tx<LPUART1>,
    last_flush: usize,
    out_buffer_read_idx: usize,
    out_buffer_write_idx: usize,
}

const UART_BAUD: u32 = 115200;

const IN_BUFFER_SIZE: usize = 128;
const HALF_IN_BUFFER_SIZE: usize = IN_BUFFER_SIZE / 2;
static mut IN_BUFFER: [u8; IN_BUFFER_SIZE] = [0; IN_BUFFER_SIZE];

const OUT_BUFFER_SIZE: usize = 32;
static mut OUT_BUFFER: [u8; OUT_BUFFER_SIZE] = [0; OUT_BUFFER_SIZE];

impl UartState {
    pub fn new(
        lpuart1: LPUART1,
        pc10: PC10<Analog>,
        pc11: PC11<Analog>,
        dma1: DMA1,
        rcc: &mut Rcc,
    ) -> Self {
        let mut dma = DMA::new(dma1, rcc);
        let mut serial = lpuart1
            .usart(
                (pc10, pc11),
                serial::Config::default().baudrate(UART_BAUD.bps()),
                rcc,
            )
            .unwrap();
        serial.listen(Event::Idle);
        let (tx, rx) = serial.split();

        let rx_channel = &dma.channels.channel3;
        rx_channel.select_target(&mut dma.handle, &rx);
        let peripheral_address = &unsafe { &*LPUART1::ptr() }.rdr as *const _ as u32;
        rx_channel.set_peripheral_address(&mut dma.handle, peripheral_address);
        let mem_address = unsafe { &IN_BUFFER[0] as *const _ as u32 };
        rx_channel.set_memory_address(&mut dma.handle, mem_address);
        rx_channel.set_transfer_len(&mut dma.handle, IN_BUFFER_SIZE as u16);
        rx_channel.configure::<u8>(&mut dma.handle, PL_A::HIGH, DIR_A::FROMPERIPHERAL, true);
        rx_channel.enable_interrupts(Interrupts {
            transfer_error: false,
            half_transfer: true,
            transfer_complete: true,
        });
        rx_channel.clear_complete_flag();
        rx_channel.start();

        Self {
            dma,
            rx,
            tx,
            last_flush: 0,
            out_buffer_read_idx: 0,
            out_buffer_write_idx: 0,
        }
    }

    pub fn interrupt_dma(&mut self, usb: &mut UsbState) {
        let rx_channel = &mut self.dma.channels.channel3;
        if rx_channel.is_complete() {
            rx_channel.clear_complete_flag();
            usb.write_uart_data(unsafe { &IN_BUFFER[self.last_flush..IN_BUFFER_SIZE] });
            self.last_flush = 0;
        } else if rx_channel.is_half_complete() {
            rx_channel.clear_half_complete_flag();
            usb.write_uart_data(unsafe { &IN_BUFFER[self.last_flush..HALF_IN_BUFFER_SIZE] });
            self.last_flush = HALF_IN_BUFFER_SIZE;
        }
    }

    pub fn interrupt_lpuart(&mut self, usb: &mut UsbState) {
        self.rx.clear_idle();
        let rx_channel = &mut self.dma.channels.channel3;
        let transfers_left = rx_channel.get_transfers_left(&mut self.dma.handle);
        let pos = IN_BUFFER_SIZE - transfers_left as usize;
        usb.write_uart_data(unsafe { &IN_BUFFER[self.last_flush..pos] });
        self.last_flush = pos;
    }

    pub fn interrupt_usb(&mut self, usb: &mut UsbState) {
        if self.out_buffer_read_idx <= self.out_buffer_write_idx {
            let num_read =
                usb.read_usb_data(unsafe { &mut OUT_BUFFER[self.out_buffer_write_idx..] });
            self.out_buffer_write_idx = self.out_buffer_write_idx + num_read;
            if self.out_buffer_write_idx >= OUT_BUFFER_SIZE {
                self.out_buffer_write_idx = 0;
            }
        }
        if self.out_buffer_read_idx > self.out_buffer_write_idx {
            let num_read = usb.read_usb_data(unsafe {
                &mut OUT_BUFFER[self.out_buffer_write_idx..self.out_buffer_read_idx]
            });
            self.out_buffer_write_idx = self.out_buffer_write_idx + num_read;
        }
        self.process(false);
    }

    pub fn process(&mut self, block: bool) {
        if self.out_buffer_read_idx != self.out_buffer_write_idx {
            if block {
                block!(self
                    .tx
                    .write(unsafe { OUT_BUFFER[self.out_buffer_read_idx] }))
                .ok();
            } else {
                match self
                    .tx
                    .write(unsafe { OUT_BUFFER[self.out_buffer_read_idx] })
                {
                    Err(nb::Error::WouldBlock) => return,
                    _ => {}
                };
            }
            let mut new_idx = self.out_buffer_read_idx + 1;
            if new_idx >= OUT_BUFFER_SIZE {
                new_idx = 0;
            }
            self.out_buffer_read_idx = new_idx;
        }
    }
}

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
use bbqueue::{consts::U256, BBBuffer, ConstBBBuffer};
use cortex_m::asm;

pub struct UartState {
    dma: DMA,
    rx: serial::Rx<LPUART1>,
    tx: serial::Tx<LPUART1>,
    last_flush: usize,
    tx_producer: bbqueue::Producer<'static, U256>,
    tx_consumer: bbqueue::Consumer<'static, U256>,
    tx_cur_read_len: usize,
}

const UART_BAUD: u32 = 115200;

const IN_BUFFER_SIZE: usize = 128;
const HALF_IN_BUFFER_SIZE: usize = IN_BUFFER_SIZE / 2;
static mut IN_BUFFER: [u8; IN_BUFFER_SIZE] = [0; IN_BUFFER_SIZE];

static TX_BUFFER: BBBuffer<U256> = BBBuffer(ConstBBBuffer::new());

impl UartState {
    pub fn new(
        lpuart1: LPUART1,
        pc10: PC10<Analog>,
        pc11: PC11<Analog>,
        dma1: DMA1,
        rcc: &mut Rcc,
    ) -> Self {
        let (tx_producer, tx_consumer) = TX_BUFFER.try_split().unwrap();

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

        let tx_channel = &dma.channels.channel2;
        tx_channel.select_target(&mut dma.handle, &tx);
        let peripheral_address = &unsafe { &*LPUART1::ptr() }.tdr as *const _ as u32;
        tx_channel.set_peripheral_address(&mut dma.handle, peripheral_address);
        tx_channel.configure::<u8>(&mut dma.handle, PL_A::HIGH, DIR_A::FROMMEMORY, true);
        tx_channel.clear_complete_flag();
        tx_channel.enable_interrupts(Interrupts {
            transfer_error: false,
            half_transfer: false,
            transfer_complete: true,
        });

        let rx_channel = &dma.channels.channel3;
        rx_channel.select_target(&mut dma.handle, &rx);
        let peripheral_address = &unsafe { &*LPUART1::ptr() }.rdr as *const _ as u32;
        rx_channel.set_peripheral_address(&mut dma.handle, peripheral_address);
        let mem_address = unsafe { &IN_BUFFER[0] as *const _ as u32 };
        rx_channel.set_memory_address(&mut dma.handle, mem_address);
        rx_channel.set_transfer_len(&mut dma.handle, IN_BUFFER_SIZE as u16);
        rx_channel.configure::<u8>(&mut dma.handle, PL_A::HIGH, DIR_A::FROMPERIPHERAL, true);
        rx_channel.clear_complete_flag();
        rx_channel.enable_interrupts(Interrupts {
            transfer_error: false,
            half_transfer: true,
            transfer_complete: true,
        });
        rx_channel.start();

        Self {
            dma,
            rx,
            tx,
            last_flush: 0,
            tx_producer,
            tx_consumer,
            tx_cur_read_len: 0,
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
        let tx_channel = &mut self.dma.channels.channel2;
        if tx_channel.is_enabled() && tx_channel.is_complete() {
            tx_channel.stop();
            while tx_channel.is_enabled() {
                asm::nop();
            }
            tx_channel.clear_complete_flag();
            let grant = self.tx_consumer.read().unwrap();
            grant.release(self.tx_cur_read_len);
            self.start_tx();
        }
    }

    pub fn interrupt_lpuart(&mut self, usb: &mut UsbState) {
        if self.rx.is_idle() {
            self.rx.clear_idle();
            let rx_channel = &mut self.dma.channels.channel3;

            let transfers_left = rx_channel.get_transfers_left(&mut self.dma.handle);
            let pos = IN_BUFFER_SIZE - transfers_left as usize;
            usb.write_uart_data(unsafe { &IN_BUFFER[self.last_flush..pos] });
            self.last_flush = pos;
        }
        if self.tx.is_transmission_complete() {
            self.tx.disable_tc_interrupt();
            self.tx.clear_transmission_complete();
            self.start_tx();
        }
    }

    pub fn interrupt_usb(&mut self, usb: &mut UsbState) {
        let mut grant = self.tx_producer.grant_max_remaining(128).unwrap();
        let num_read = usb.read_usb_data(grant.buf());
        grant.commit(num_read);
        if num_read > 0 {
            self.start_tx();
        }
    }

    pub fn start_tx(&mut self) {
        let tx_channel = &mut self.dma.channels.channel2;
        if tx_channel.is_enabled() {
            return;
        }

        let grant = match self.tx_consumer.read() {
            Ok(g) => g,
            Err(bbqueue::Error::InsufficientSize) => return,
            Err(_) => panic!(),
        };
        let mem_len = grant.buf().len();
        if mem_len > 1 {
            let mem_address = &grant.buf()[0] as *const _ as u32;
            self.tx_cur_read_len = mem_len;
            tx_channel.set_memory_address(&mut self.dma.handle, mem_address);
            tx_channel.set_transfer_len(&mut self.dma.handle, mem_len as u16);
            tx_channel.start();
        } else if mem_len == 1 {
            // single-byte DMA transfers don't seem to work, they duplicate the byte
            // so use a single byte write to the uart
            self.tx.enable_tc_interrupt();
            match self.tx.write(grant.buf()[0]) {
                Ok(_) => {
                    // if we can write, not busy, mark as written
                    grant.release(1);
                }
                // if we can't write, no big deal, the uart is busy with something and
                // the interrupt will trigger this again
                _ => {}
            }
        }
    }
}

use crate::utils::{
    Exception,
    channel::{Receiver, Sender, channel},
};

use super::{Device, Irq, Size};

// NS16550A

pub(crate) const UART_START: u32 = 0x10000000;
pub(crate) const UART_END: u32 = UART_START + 8 - 1;
pub(crate) const INTERRUPT_ID: u32 = 1;

const UART_RBR_DLL: u32 = UART_START;
const UART_THR: u32 = UART_START;

const UART_IER_ILM: u32 = UART_START + 1;
const UART_IER_RDI: u8 = 0b00000001;
const UART_IER_THRI: u8 = 0b00000010;

const UART_IIR: u32 = UART_START + 2;
const UART_FCR: u32 = UART_START + 2;
const UART_IIR_NO_INT: u8 = 0b00000001;
const UART_IIR_THRI: u8 = 0b00000010;
const UART_IIR_RDI: u8 = 0b00000100;
const UART_FCR_ENABLE_FIFO: u8 = 0b00000001;
const UART_FCR_CLEAR_RCVR: u8 = 0b00000010;
const UART_FCR_CLEAR_XMIT: u8 = 0b00000100;

const UART_LCR: u32 = UART_START + 3;
const UART_LCR_DLAB: u8 = 0b10000000;

const UART_MCR: u32 = UART_START + 4;
const UART_MCR_LOOP: u8 = 0b00010000;

const UART_LSR: u32 = UART_START + 5;
const UART_LSR_DR: u8 = 0b00000001;
const UART_LSR_OE: u8 = 0b00000010;
const UART_LSR_BI: u8 = 0b00010000;
const UART_LSR_THRE: u8 = 0b00100000;
const UART_LSR_TEMT: u8 = 0b01000000;

const UART_MSR: u32 = UART_START + 6;
const UART_MCR_OUT2: u8 = 0b00001000;

const UART_SCR: u32 = UART_START + 7;

#[derive(Debug)]
pub(crate) struct Uart {
    receiver: Receiver<u8>,
    loop_sender: Sender<u8>,
    sender: Sender<u8>,
    lcr: u8,
    dll: u8,
    dlm: u8,
    ier: u8,
    iir: u8,
    mcr: u8,
    lsr: u8,
    scr: u8,
    fcr: u8,
}

impl Uart {
    pub(crate) fn new() -> (Uart, Sender<u8>, Receiver<u8>) {
        let (recv_send, recv) = channel();
        let (send, send_recv) = channel();
        (
            Uart {
                receiver: recv,
                loop_sender: recv_send.clone(),
                sender: send,
                lcr: 0,
                dll: 0x0c,
                dlm: 0,
                ier: 0,
                iir: UART_IIR_NO_INT,
                mcr: UART_MCR_OUT2,
                #[allow(clippy::eq_op)]
                lsr: UART_LSR_TEMT | UART_LSR_TEMT,
                scr: 0,
                fcr: 0,
            },
            recv_send,
            send_recv,
        )
    }
}

impl Device for Uart {
    fn clk(&mut self, irq: &mut Irq) {
        // TODO: backoff counter?
        if self.receiver.avaliable() {
            self.lsr |= UART_LSR_DR;
        }

        if self.lcr & UART_FCR_CLEAR_RCVR != 0 {
            self.receiver.clear();
            self.lsr &= !UART_LSR_DR & !UART_FCR_CLEAR_RCVR;
        }

        if self.lcr & UART_FCR_CLEAR_XMIT != 0 {
            self.lcr &= !UART_FCR_CLEAR_XMIT;
            self.lsr |= UART_LSR_TEMT | UART_LSR_THRE;
        }

        let mut interrupts: u8 = 0;
        if (self.ier & UART_IER_RDI) != 0 && (self.lsr & UART_LSR_DR) != 0 {
            interrupts |= UART_IIR_RDI;
        }

        if (self.ier & UART_IER_THRI) != 0 && (self.lsr & UART_LSR_TEMT) != 0 {
            interrupts |= UART_IIR_THRI;
        }

        if interrupts != 0 {
            self.iir = UART_IIR_NO_INT;
            irq.irq(INTERRUPT_ID, false);
        } else {
            self.iir = interrupts;
            irq.irq(INTERRUPT_ID, true);
        }

        if self.ier & UART_IER_THRI == 0 {
            self.lsr |= UART_LSR_TEMT | UART_LSR_THRE;
        }
    }

    fn read(&mut self, address: u32, size: Size) -> Result<u64, Exception> {
        if size != Size::_1 {
            return Err(Exception::BusException);
        }
        match address {
            UART_RBR_DLL => {
                let res = if self.lcr & UART_LCR_DLAB != 0 {
                    self.dll
                } else if self.lsr & UART_LSR_BI != 0 {
                    0
                } else if self.receiver.avaliable() {
                    self.lsr &= !UART_LSR_OE;
                    self.receiver.recv()
                } else {
                    0
                };
                Ok(res as u64)
            }
            UART_IER_ILM => Ok(if self.lcr & UART_LCR_DLAB != 0 {
                self.dlm as u64
            } else {
                self.ier as u64
            }),
            UART_IIR => Ok(self.iir as u64),
            UART_LCR => Ok(self.lcr as u64),
            UART_MCR => Ok(self.mcr as u64),
            UART_LSR => Ok(self.lsr as u64),
            UART_SCR => Ok(self.scr as u64),
            UART_MSR => Ok(0),
            _ => Err(Exception::BusException),
        }
    }

    fn write(&mut self, address: u32, size: Size, data: u64) -> Result<(), Exception> {
        if size != Size::_1 {
            return Err(Exception::BusException);
        }
        match address {
            UART_THR => {
                if self.lcr & UART_LCR_DLAB != 0 {
                    self.dll = data as u8;
                } else if self.fcr & UART_FCR_ENABLE_FIFO == 0 && self.receiver.avaliable() {
                    self.lsr |= UART_LSR_OE;
                } else {
                    self.lsr |= UART_LSR_TEMT | UART_LSR_THRE;
                    if self.mcr & UART_MCR_LOOP != 0 {
                        self.loop_sender.send(data as u8);
                    } else {
                        self.sender.send(data as u8);
                    }
                }
                Ok(())
            }
            UART_IER_ILM => {
                if self.lcr & UART_LCR_DLAB != 0 {
                    self.dlm = data as u8;
                } else {
                    self.ier = data as u8 & 0b1111;
                }
                Ok(())
            }
            UART_FCR => {
                self.fcr = data as u8;
                Ok(())
            }
            UART_LCR => {
                self.lcr = data as u8;
                Ok(())
            }
            UART_MCR => {
                self.mcr = data as u8 & 0b11111;
                Ok(())
            }
            UART_SCR => {
                self.scr = data as u8;
                Ok(())
            }
            _ => Err(Exception::BusException),
        }
    }
}

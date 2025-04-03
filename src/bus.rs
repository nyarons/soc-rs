use crate::{
    devices::{
        Device, Irq,
        memory::{MEMORY_END, MEMORY_START, Memory},
        plic::{PLIC_END, PLIC_START, Plic},
        uart::{UART_END, UART_START, Uart},
        ysyx::{YSYX_END, YSYX_START, Ysyx, YsyxCommand},
    },
    utils::{
        Exception, Size,
        channel::{Receiver, Sender},
    },
};

#[derive(Debug)]
pub struct Bus {
    memory: Memory,
    plic: Plic,
    uart: Uart,
    ysyx: Ysyx,

    count: u64,
}

#[derive(Debug)]
pub struct DeviceController {
    pub uart_sender: Sender<u8>,
    pub uart_receiver: Receiver<u8>,
    pub ysyx_receiver: Receiver<YsyxCommand>,
}

impl Bus {
    pub fn new() -> (Bus, DeviceController) {
        let (uart, uart_sender, uart_receiver) = Uart::new();
        let (ysyx, ysyx_receiver) = Ysyx::new();
        (
            Bus {
                memory: Memory::new(),
                plic: Plic::new(),
                uart,
                ysyx,
                count: 0,
            },
            DeviceController {
                uart_sender,
                uart_receiver,
                ysyx_receiver,
            },
        )
    }

    pub fn clk(&mut self) {
        if self.count > 1000 {
            self.count = 0;
            let mut irq = Irq::new();
            self.memory.clk(&mut irq);
            self.plic.clk(&mut irq);
            self.uart.clk(&mut irq);
            self.ysyx.clk(&mut irq);
            for (irq, enable) in irq {
                self.plic.irq(irq, enable);
            }
        } else {
            self.count += 1;
        }
    }

    pub fn read(&mut self, address: u32, size: Size) -> Result<u64, Exception> {
        match address {
            MEMORY_START..=MEMORY_END => self.memory.read(address, size),
            PLIC_START..=PLIC_END => self.plic.read(address, size),
            UART_START..=UART_END => self.uart.read(address, size),
            YSYX_START..=YSYX_END => self.ysyx.read(address, size),
            _ => Err(Exception::BusException),
        }
    }

    pub fn write(&mut self, address: u32, size: Size, data: u64) -> Result<(), Exception> {
        match address {
            MEMORY_START..=MEMORY_END => self.memory.write(address, size, data),
            PLIC_START..=PLIC_END => self.plic.write(address, size, data),
            UART_START..=UART_END => self.uart.write(address, size, data),
            YSYX_START..=YSYX_END => self.ysyx.write(address, size, data),
            _ => Err(Exception::BusException),
        }
    }

    pub fn interrupt(&mut self) -> Option<bool> {
        self.plic.check_interrupt()
    }
}

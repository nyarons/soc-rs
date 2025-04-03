use crate::utils::Exception;

use super::{Device, Irq, Size};

pub(crate) const PLIC_START: u32 = 0x0C000000;
pub(crate) const PLIC_END: u32 = PLIC_START + 0x3FFFFFF;

pub(crate) const PLIC_SOURCE_PRIORITY_START: u32 = PLIC_START + 0x000004;
pub(crate) const PLIC_SOURCE_PRIORITY_END: u32 = PLIC_START + 0x000FFF;

pub(crate) const PLIC_PENDING_START: u32 = PLIC_START + 0x001000;
pub(crate) const PLIC_PENDING_END: u32 = PLIC_START + 0x00107F;

pub(crate) const PLIC_SOURCE_ENABLE_START: u32 = PLIC_START + 0x002000;
pub(crate) const PLIC_SOURCE_ENABLE_END: u32 = PLIC_START + 0x1F1FFF;

pub(crate) const PLIC_THRESHOLD_CLIAM_COMPLETE_START: u32 = PLIC_START + 0x200000;
pub(crate) const PLIC_THRESHOLD_CLIAM_COMPLETE_END: u32 = PLIC_START + 0x3FFFFFF;

// TODO: hart count
const HART_COUNT: usize = 1;
const INTERRUPT_COUNT: usize = 64;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Pair<T: Clone + Copy> {
    machine: T,
    supervisor: T,
}

impl<T: Clone + Copy> Pair<T> {
    pub(crate) fn at(&self, index: usize) -> &T {
        match index {
            0 => &self.machine,
            1 => &self.supervisor,
            _ => unreachable!(),
        }
    }

    pub(crate) fn at_mut(&mut self, index: usize) -> &mut T {
        match index {
            0 => &mut self.machine,
            1 => &mut self.supervisor,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct Plic {
    priorities: [u32; 1023],
    pending: [u32; 32],
    enable: [Pair<[u32; 32]>; HART_COUNT * 2],
    threshold: [Pair<u32>; HART_COUNT * 2],
    claimed: [Pair<[bool; 1024]>; HART_COUNT * 2],
    update: bool,
}

impl Plic {
    pub(crate) fn new() -> Plic {
        Plic {
            priorities: [0; 1023],
            pending: [0; 32],
            enable: [Pair {
                machine: [0; 32],
                supervisor: [0; 32],
            }; HART_COUNT * 2],
            threshold: [Pair {
                machine: 0,
                supervisor: 0,
            }; HART_COUNT * 2],
            claimed: [Pair {
                machine: [false; 1024],
                supervisor: [false; 1024],
            }; HART_COUNT * 2],
            update: false,
        }
    }

    pub(crate) fn irq(&mut self, irq: u32, enable: bool) {
        let index = (irq / 32) as usize;
        let offset = irq % 32;
        let pending = self.pending[index];
        if enable {
            self.pending[index] |= 1 << offset;
        } else {
            self.pending[index] &= !(1 << offset);
        }
        if pending != self.pending[index] {
            self.update = true;
        }
    }

    pub(crate) fn check_interrupt(&mut self) -> Option<bool> {
        if self.update {
            self.update = false;
            // HART_COUNT = 1
            // for context in 0..HART_COUNT {}
            return Some(self.highest_irq(0) != 0);
        }
        None
    }

    fn complete(&mut self, context: usize, irq: u32) {
        self.claimed[context / 2].at_mut(context % 2)[irq as usize] = false;
    }

    fn claim(&mut self, context: usize) -> u32 {
        let irq = self.highest_irq(context);
        let index = (irq / 8) as usize;
        let offset = irq % 8;
        self.pending[index] &= !(1 << offset);
        self.claimed[context / 2].at_mut(context % 2)[irq as usize] = true;
        irq
    }

    fn highest_irq(&mut self, context: usize) -> u32 {
        let mut irq: u32 = 0;
        let mut priority = 0;
        for i in 1..INTERRUPT_COUNT {
            let index = i / 32;
            let offset = i % 32;
            let hart = context / 2;
            let mode = context % 2;
            if self.enable[hart].at(mode)[index] & (1 << offset) != 0
                && self.pending[index] & (1 << offset) != 0
                && !self.claimed[hart].at(mode)[index]
                && self.priorities[i] > *self.threshold[hart].at(mode)
                && self.priorities[i] > priority
            {
                irq = i as u32;
                priority = self.priorities[i];
            }
        }
        irq
    }
}

impl Device for Plic {
    fn clk(&mut self, _irq: &mut Irq) {}

    fn read(&mut self, address: u32, size: Size) -> Result<u64, Exception> {
        if size != Size::_4 {
            return Err(Exception::BusException);
        }
        match address {
            PLIC_SOURCE_PRIORITY_START..=PLIC_SOURCE_PRIORITY_END => {
                Ok(self.priorities[(address - PLIC_SOURCE_PRIORITY_START) as usize] as u64)
            }
            PLIC_PENDING_START..=PLIC_PENDING_END => {
                Ok(self.pending[(address - PLIC_PENDING_START) as usize] as u64)
            }
            PLIC_SOURCE_ENABLE_START..=PLIC_SOURCE_ENABLE_END => {
                let offset = (address - PLIC_SOURCE_ENABLE_START) as usize;
                let context = offset / 0x80;
                let item = offset % 0x80;
                Ok(self.enable[context / 2].at(context % 2)[item] as u64)
            }
            PLIC_THRESHOLD_CLIAM_COMPLETE_START..=PLIC_THRESHOLD_CLIAM_COMPLETE_END => {
                let offset = (address - PLIC_THRESHOLD_CLIAM_COMPLETE_START) as usize;
                let context = offset / 0x1000;
                let item = offset % 0x1000;
                match item {
                    // threshold
                    0 => Ok(*self.threshold[context / 2].at(context % 2) as u64),
                    // claim
                    1 => {
                        let irq = self.claim(context);
                        self.update = true;
                        Ok(irq as u64)
                    }
                    _ => Err(Exception::BusException),
                }
            }
            _ => Err(Exception::BusException),
        }
    }

    fn write(&mut self, address: u32, size: Size, data: u64) -> Result<(), Exception> {
        if size != Size::_4 {
            return Err(Exception::BusException);
        }
        match address {
            PLIC_SOURCE_PRIORITY_START..=PLIC_SOURCE_PRIORITY_END => {
                self.priorities[(address - PLIC_SOURCE_PRIORITY_START) as usize] = data as u32
            }
            PLIC_SOURCE_ENABLE_START..=PLIC_SOURCE_ENABLE_END => {
                let offset = (address - PLIC_SOURCE_ENABLE_START) as usize;
                let context = offset / 0x80;
                let item = offset % 0x80;
                self.enable[context / 2].at_mut(context % 2)[item] = data as u32;
            }
            PLIC_THRESHOLD_CLIAM_COMPLETE_START..=PLIC_THRESHOLD_CLIAM_COMPLETE_END => {
                let offset = (address - PLIC_THRESHOLD_CLIAM_COMPLETE_START) as usize;
                let context = offset / 0x1000;
                let item = offset % 0x1000;
                match item {
                    // threshold
                    0 => *self.threshold[context / 2].at_mut(context % 2) = data as u32,
                    // complete
                    1 => self.complete(context, data as u32),
                    _ => return Err(Exception::BusException),
                };
            }
            _ => return Err(Exception::BusException),
        };
        Ok(())
    }
}

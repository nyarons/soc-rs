use crate::utils::Size;

pub(crate) mod memory;
pub(crate) mod plic;
pub(crate) mod uart;
pub(crate) mod ysyx;

pub(crate) struct Irq {
    irqs: Vec<(u32, bool)>,
}

impl Irq {
    pub(crate) fn new() -> Irq {
        Irq { irqs: Vec::new() }
    }

    pub(crate) fn irq(&mut self, irq: u32, enable: bool) {
        self.irqs.push((irq, enable));
    }
}

impl Iterator for Irq {
    type Item = (u32, bool);

    fn next(&mut self) -> Option<Self::Item> {
        self.irqs.pop()
    }
}

pub(crate) trait Device {
    fn clk(&mut self, irq: &mut Irq);
    fn read(&mut self, address: u32, size: Size) -> Result<u64, crate::utils::Exception>;
    fn write(&mut self, address: u32, size: Size, data: u64)
    -> Result<(), crate::utils::Exception>;
}

use std::sync::{Arc, Mutex};

use crate::utils::Exception;

use super::{Device, Irq, Size};

pub(crate) const MEMORY_SIZE: usize = 1024 * 1024 * 1024;
pub(crate) const MEMORY_START: u32 = 0x80000000;
pub(crate) const MEMORY_END: u32 = MEMORY_START + MEMORY_SIZE as u32 - 1;

#[derive(Debug)]
pub(crate) struct Memory {
    mem: *mut u8,
    _boxed: Arc<Mutex<Box<[u8]>>>,
}

impl Memory {
    pub(crate) fn new() -> Memory {
        let mut mem: Box<[u8]> = vec![0; MEMORY_SIZE].into_boxed_slice();
        Memory {
            mem: &mut mem[0],
            _boxed: Arc::new(Mutex::new(mem)),
        }
    }
}

impl Device for Memory {
    fn clk(&mut self, _irq: &mut Irq) {}

    fn read(&mut self, address: u32, size: Size) -> Result<u64, Exception> {
        let address = address - MEMORY_START;
        match size {
            Size::_1 => Ok((unsafe { *(self.mem.wrapping_add(address as usize)) }) as u64),
            Size::_2 => {
                Ok(
                    u16::from_le(unsafe { *(self.mem.wrapping_add(address as usize) as *const _) })
                        as u64,
                )
            }
            Size::_4 => {
                Ok(
                    u32::from_le(unsafe { *(self.mem.wrapping_add(address as usize) as *const _) })
                        as u64,
                )
            }
            Size::_8 => Ok(u64::from_le(unsafe {
                *(self.mem.wrapping_add(address as usize) as *const _)
            })),
        }
    }

    fn write(&mut self, address: u32, size: Size, data: u64) -> Result<(), Exception> {
        let address = address - MEMORY_START;
        match size {
            Size::_1 => unsafe {
                *(self.mem.wrapping_add(address as usize)) = data as u8;
                Ok(())
            },
            Size::_2 => unsafe {
                *(self.mem.wrapping_add(address as usize) as *mut _) = (data as u16).to_le();
                Ok(())
            },
            Size::_4 => unsafe {
                *(self.mem.wrapping_add(address as usize) as *mut _) = (data as u32).to_le();
                Ok(())
            },
            Size::_8 => unsafe {
                *(self.mem.wrapping_add(address as usize) as *mut _) = data.to_le();
                Ok(())
            },
        }
    }
}

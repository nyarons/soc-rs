pub(crate) mod channel;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Size {
    // byte
    _1,
    // half word
    _2,
    // word
    _4,
    // double word
    _8,
}

pub(crate) fn u32_to_u8(arr: &mut [u32]) -> &mut [u8] {
    let len = 4 * arr.len();
    let ptr = arr.as_ptr() as *mut u8;
    unsafe { std::slice::from_raw_parts_mut(ptr, len) }
}

#[derive(Debug)]
pub enum Exception {
    BusException,
}

use crate::cpu::Exception;
use core::ops::Range;

macro_rules! mmap {
    ($db: tt $ds: tt $dr: tt $base: expr, $size: expr) => {
        pub const $db: u64 = $base;
        pub const $ds: u64 = $size;
        pub const $dr: Range<u64> = $base..$base + $size;
    };
}

mmap!(RAM_BASE      RAM_SIZE      RAM_RANGE      0x8000_0000, 32 * 1024 * 1024);
mmap!(PLIC_BASE     PLIC_SIZE     PLIC_RANGE     0x0c00_0000, 0x0400_0000);
mmap!(CLINT_BASE    CLINT_SIZE    CLINT_RANGE    0x0200_0000, 0x0001_0000);

pub struct Bus<'a> {
    ram: crate::ram::Ram<'a>,
    plic: crate::plic::Plic,
    clint: crate::clint::Clint,
}

impl<'a> Bus<'a> {
    pub fn new(ram: crate::ram::Ram<'a>) -> Self {
        Self {
            ram,
            plic: crate::plic::Plic::new(),
            clint: crate::clint::Clint::new(),
        }
    }
}

macro_rules! gen {
    ($l: tt $s: tt $t: tt $sz: tt) => {
        impl Bus<'_> {
            pub(crate) fn $l(&mut self, addr: u64) -> Result<$t, Exception> {
                if RAM_RANGE.contains(&addr) { return self.ram.$l(addr); }
                if PLIC_RANGE.contains(&addr) { return self.plic.$l(addr); }
                if CLINT_RANGE.contains(&addr) { return self.clint.$l(addr); }
                Err(Exception::LoadAccessFault)
            }

            pub(crate) fn $s(&mut self, addr: u64, val: $t) -> Result<(), Exception> {
                if RAM_RANGE.contains(&addr) { return self.ram.$s(addr, val); }
                if PLIC_RANGE.contains(&addr) { return self.plic.$s(addr, val); }
                if CLINT_RANGE.contains(&addr) { return self.clint.$s(addr, val); }
                Err(Exception::StoreAccessFault)
            }
        }
    };
}

gen!(load_u8 store_u8 u8 1);
gen!(load_u16 store_u16 u16 2);
gen!(load_u32 store_u32 u32 4);
gen!(load_u64 store_u64 u64 8);

pub(crate) trait Device {
    fn load_u8(&mut self, _addr: u64) -> Result<u8, Exception> { Err(Exception::LoadAccessFault) }
    fn load_u16(&mut self, _addr: u64) -> Result<u16, Exception> { Err(Exception::LoadAccessFault) }
    fn load_u32(&mut self, _addr: u64) -> Result<u32, Exception> { Err(Exception::LoadAccessFault) }
    fn load_u64(&mut self, _addr: u64) -> Result<u64, Exception> { Err(Exception::LoadAccessFault) }

    fn store_u8(&mut self, _addr: u64, _val: u8) -> Result<(), Exception> { Err(Exception::StoreAccessFault) }
    fn store_u16(&mut self, _addr: u64, _val: u16) -> Result<(), Exception> { Err(Exception::StoreAccessFault) }
    fn store_u32(&mut self, _addr: u64, _val: u32) -> Result<(), Exception> { Err(Exception::StoreAccessFault) }
    fn store_u64(&mut self, _addr: u64, _val: u64) -> Result<(), Exception> { Err(Exception::StoreAccessFault) }
}

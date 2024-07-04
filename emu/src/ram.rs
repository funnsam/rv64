use crate::bus::*;
use crate::cpu::Exception;

pub struct Ram<'a> {
    ram: &'a mut [u8],
}

impl<'a> Ram<'a> {
    pub fn new(ram: &'a mut [u8]) -> Self {
        Self { ram }
    }
}

macro_rules! gen {
    ($l: tt $s: tt $t: tt $sz: tt) => {
        fn $l(&mut self, addr: u64) -> Result<$t, Exception> {
            let start = addr - RAM_BASE;
            if start + $sz <= self.ram.len() as u64 {
                Ok($t::from_le_bytes(self.ram[start as usize..start as usize + $sz].try_into().unwrap()))
            } else {
                Err(Exception::LoadAccessFault)
            }
        }

        fn $s(&mut self, addr: u64, val: $t) -> Result<(), Exception> {
            let start = addr - RAM_BASE;
            if start + $sz <= self.ram.len() as u64 {
                self.ram[start as usize..start as usize + $sz].copy_from_slice(&val.to_le_bytes());
                Ok(())
            } else {
                Err(Exception::StoreAccessFault)
            }
        }
    };
}

impl Device for Ram<'_> {
    gen!(load_u8 store_u8 u8 1);
    gen!(load_u16 store_u16 u16 2);
    gen!(load_u32 store_u32 u32 4);
    gen!(load_u64 store_u64 u64 8);
}

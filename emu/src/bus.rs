pub struct Bus<'a> {
    ram: &'a mut [u8],
}

impl<'a> Bus<'a> {
    pub fn new(ram: &'a mut [u8]) -> Self {
        Self { ram }
    }
}

macro_rules! gen {
    ($l: tt $s: tt $t: tt $sz: tt) => {
        impl Bus<'_> {
            pub fn $l(&self, addr: u64) -> Result<$t, ()> {
                let start = addr - 0x80000000;
                if start + $sz <= self.ram.len() as u64 {
                    Ok($t::from_le_bytes(self.ram[start as usize..start as usize + $sz].try_into().unwrap()))
                } else {
                    Err(())
                }
            }

            pub fn $s(&mut self, addr: u64, val: $t) -> Result<(), ()> {
                let start = addr - 0x80000000;
                if start + $sz <= self.ram.len() as u64 {
                    self.ram[start as usize..start as usize + $sz].copy_from_slice(&val.to_le_bytes());
                    Ok(())
                } else {
                    Err(())
                }
            }
        }
    };
}

gen!(load_u8 store_u8 u8 1);
gen!(load_u16 store_u16 u16 2);
gen!(load_u32 store_u32 u32 4);
gen!(load_u64 store_u64 u64 8);

use super::*;

macro_rules! gen {
    ($t: tt $l: tt $s: tt $amo: tt $nl: tt $ns: tt $sz: tt) => {
        impl<'a> Cpu<'a> {
            pub(crate) fn $l(&mut self, a: u64, _aqrl: AqRlMode) -> Result<$t, Exception> {
                self.amo_rs.aquire(a, $sz);
                self.$nl(a)
            }

            pub(crate) fn $s(&mut self, a: u64, d: $t, _aqrl: AqRlMode) -> Result<(), ()> {
                self.amo_rs.check_ownership(a, $sz)?;
                self.amo_rs.length = 0;
                self.$ns(a, d).map_err(|_| ())
            }

            pub(crate) fn $amo<T: Fn($t) -> $t>(&mut self, a: u64, _aqrl: AqRlMode, f: T) -> Result<$t, Exception> {
                let b = self.$nl(a)?;
                let v = f(b);
                self.$ns(a, v)?;
                Ok(b)
            }
        }
    };
}

gen!(u32 atomic_load_u32 atomic_store_u32 atomic_mo_u32 mmu_load_u32 mmu_store_u32 1);
gen!(u64 atomic_load_u64 atomic_store_u64 atomic_mo_u64 mmu_load_u64 mmu_store_u64 2);

pub(crate) struct AqRlMode {
    pub aq: bool,
    pub rl: bool,
}

impl AqRlMode {
    pub(crate) fn from_bits(b: u32) -> Self {
        Self {
            aq: b & 2 != 0,
            rl: b & 1 != 0,
        }
    }
}

const SET_SIZE: usize = 16;

pub(crate) struct ReservationSet {
    set: [u64; SET_SIZE],
    length: usize,
}

impl ReservationSet {
    pub(crate) fn new() -> Self {
        Self {
            set: [0; 16],
            length: 0,
        }
    }

    fn aquire_4(&mut self, addr: u64) {
        self.set[self.length] = addr;
        self.length = (self.length + 1) % SET_SIZE;
    }

    fn aquire(&mut self, addr: u64, sz: usize) {
        for i in 0..sz {
            self.aquire_4(addr + i as u64 * 4)
        }
    }

    fn check_ownership_4(&mut self, addr: u64) -> Result<(), ()> {
        self.set[..self.length].contains(&addr).then_some(()).ok_or(())
    }

    fn check_ownership(&mut self, addr: u64, sz: usize) -> Result<(), ()> {
        for i in 0..sz {
            self.check_ownership_4(addr + i as u64 * 4)?;
        }

        Ok(())
    }
}

use crate::bus::*;
use crate::cpu::Exception;

pub(crate) struct Clint {
    mtime: u64,
    mtimecmp: u64,
}

const CLINT_MTIMECMP: u64 = CLINT_BASE + 0x4000;
const CLINT_MTIME: u64 = CLINT_BASE + 0xbff8;

impl Clint {
    pub fn new() -> Self {
        Self {
            mtime: 0,
            mtimecmp: 0,
        }
    }
}

impl Device for Clint {
    fn load_u64(&mut self, addr: u64) -> Result<u64, Exception> {
        Ok(match addr {
            CLINT_MTIMECMP => self.mtimecmp,
            CLINT_MTIME => self.mtime,
            _ => 0,
        })
    }

    fn store_u64(&mut self, addr: u64, val: u64) -> Result<(), Exception> {
        Ok(match addr {
            CLINT_MTIMECMP => self.mtimecmp = val,
            CLINT_MTIME => self.mtime = val,
            _ => {},
        })
    }
}

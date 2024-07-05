// basically https://github.com/d0iasm/rvemu-for-book/blob/main/step10/src/plic.rs but with my
// bus interface

use crate::bus::*;
use crate::cpu::Exception;

pub(crate) struct Plic {
    pending: u32,
    senable: u32,
    spriority: u32,
    sclaim: u32,
}

const PLIC_PENDING: u64 = PLIC_BASE + 0x1000;
const PLIC_SENABLE: u64 = PLIC_BASE + 0x2080;
const PLIC_SPRIORITY: u64 = PLIC_BASE + 0x201000;
const PLIC_SCLAIM: u64 = PLIC_BASE + 0x201004;

impl Plic {
    pub(crate) fn new() -> Self {
        Self {
            pending: 0,
            senable: 0,
            spriority: 0,
            sclaim: 0,
        }
    }
}

impl Device for Plic {
    fn load_u32(&mut self, addr: u64) -> Result<u32, Exception> {
        Ok(match addr {
            PLIC_PENDING => self.pending,
            PLIC_SENABLE => self.senable,
            PLIC_SPRIORITY => self.spriority,
            PLIC_SCLAIM => self.sclaim,
            _ => 0,
        })
    }

    fn store_u32(&mut self, addr: u64, val: u32) -> Result<(), Exception> {
        Ok(match addr {
            PLIC_PENDING => self.pending = val,
            PLIC_SENABLE => self.senable = val,
            PLIC_SPRIORITY => self.spriority = val,
            PLIC_SCLAIM => self.sclaim = val,
            _ => {}
        })
    }
}

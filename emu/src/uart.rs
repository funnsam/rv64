//! 16550(A) UART chip emulation
// TODO:
// - input
// - output to anything not just stdout
// - interrupts

use crate::bus::*;
use crate::cpu::Exception;

const RHR: u64 = UART_BASE + 0;
const THR: u64 = UART_BASE + 0;
const IER: u64 = UART_BASE + 1;
const FCR: u64 = UART_BASE + 2;
const ISR: u64 = UART_BASE + 2;

pub struct Uart {
}

impl Uart {
    pub fn new() -> Self {
        Self {
        }
    }
}

impl Device for Uart {
    fn load_u8(&mut self, addr: u64) -> Result<u8, Exception> {
        Ok(match addr {
            _ => 0,
        })
    }

    fn store_u8(&mut self, addr: u64, val: u8) -> Result<(), Exception> {
        Ok(match addr {
            THR => print!("{}", val as char),
            _ => {},
        })
    }
}

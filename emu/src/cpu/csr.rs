use super::Exception;

// machine info
pub const CSR_MVENDORID: u64 = 0xf11;
pub const CSR_MARCHID: u64 = 0xf12;
pub const CSR_MIMPID: u64 = 0xf13;
pub const CSR_MHARTID: u64 = 0xf14;
pub const CSR_MCONFIGPTR: u64 = 0xf15;
// machine trap setup
pub const CSR_MSTATUS: u64 = 0x300;
pub const CSR_MISA: u64 = 0x301;
pub const CSR_MEDELEG: u64 = 0x302;
pub const CSR_MIDELEG: u64 = 0x303;
pub const CSR_MTVEC: u64 = 0x305;
pub const CSR_MCOUNTEREN: u64 = 0x306;
// machine trap handling
pub const CSR_MSCRATCH: u64 = 0x340;
pub const CSR_MEPC: u64 = 0x341;
pub const CSR_MCAUSE: u64 = 0x342;
pub const CSR_MTVAL: u64 = 0x343;
pub const CSR_MIP: u64 = 0x344;
pub const CSR_MTINST: u64 = 0x345;
pub const CSR_MTVAL2: u64 = 0x346;

impl<'a> super::Cpu<'a> {
    pub(super) fn csr_read_cpu(&self, a: u64) -> u64 {
        unsafe { self._csr_read(a, false).unwrap_unchecked() }
    }

    pub(super) fn csr_read(&self, a: u64) -> Result<u64, Exception> {
        self._csr_read(a, true)
    }

    fn _csr_read(&self, a: u64, err: bool) -> Result<u64, Exception> {
        let a = a & 4095;
        println!("csrr {a:03x}");
        self.check_csr_perm(a, err)?;

        Ok(match a {
            CSR_MISA => (2 << 62) | (1 << 8) | (1 << 12) | (1 << 18) /* | 1 << 5 | 1 << 3 */,
            CSR_MHARTID => 0,
            _ => self.csrs[a as usize],
        })
    }

    pub(super) fn csr_write_cpu(&mut self, a: u64, d: u64) {
        unsafe { self._csr_write(a, d, false).unwrap_unchecked() }
    }

    pub(super) fn csr_write(&mut self, a: u64, d: u64) -> Result<(), Exception> {
        self._csr_write(a, d, true)
    }

    fn _csr_write(&mut self, a: u64, d: u64, err: bool) -> Result<(), Exception> {
        let a = a & 4095;
        println!("csrw {a:03x} {d:016x}");
        self.check_csr_perm(a, err)?;

        if a >> 10 == 3 {
            panic!("csr wr wo {a:03x} {d:016x}");
        }

        match a {
            CSR_MISA => {},
            _ => self.csrs[a as usize] = d,
        }

        Ok(())
    }

    fn check_csr_perm(&self, a: u64, err: bool) -> Result<(), Exception> {
        if err && (a >> 10) & 3 > self.mode as _ {
            return Err(Exception::IllegalInst);
        }

        Ok(())
    }
}

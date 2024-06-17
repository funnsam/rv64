use super::*;

// supervisor trap setup
pub const CSR_SSTATUS: u64 = 0x100;
pub const CSR_SIE: u64 = 0x104;
pub const CSR_STVEC: u64 = 0x105;
pub const CSR_SCOUNTEREN: u64 = 0x106;
// supervisor configuration
pub const CSR_SENVCFG: u64 = 0x10a;
// supervisor counter setup
pub const CSR_SCOUNTERINHIBIT: u64 = 0x120;
// supervisor trap handling
pub const CSR_SSCRATCH: u64 = 0x140;
pub const CSR_SEPC: u64 = 0x141;
pub const CSR_SCAUSE: u64 = 0x142;
pub const CSR_STVAL: u64 = 0x143;
pub const CSR_SIP: u64 = 0x144;
pub const CSR_SCOUNTOVF: u64 = 0xda0;
// supervisor protection & translation
pub const CSR_SATP: u64 = 0x180;
// debug/trace regiser
pub const CSR_SCONTEXT: u64 = 0x5a8;
// supervisor state enable regisers
pub const CSR_SSTATEEN0: u64 = 0x10c;
pub const CSR_SSTATEEN1: u64 = 0x10d;
pub const CSR_SSTATEEN2: u64 = 0x10e;
pub const CSR_SSTATEEN3: u64 = 0x10f;

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
pub const CSR_MIE: u64 = 0x304;
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

const MSTAT_S_MASK: u64 = 0x8000_0003_000f_e7e2;
const MSTAT_W_MASK: u64 = 0x7fff_ffc0_fff2_19bf;

impl<'a> Cpu<'a> {
    pub fn csr_init(&mut self) {
        self.csrs[CSR_MSTATUS as usize] = 0x0000_000a_0000_0000;
    }

    pub fn csr_read_cpu(&self, a: u64) -> u64 {
        unsafe { self._csr_read(a, false).unwrap_unchecked() }
    }

    pub fn csr_read(&self, a: u64) -> Result<u64, Exception> {
        println!("csrr {a:03x}");
        self._csr_read(a, true)
    }

    fn _csr_read(&self, a: u64, err: bool) -> Result<u64, Exception> {
        let a = a & 4095;
        self.check_csr_perm(a, err)?;

        Ok(match a {
            CSR_MISA => (2 << 62) | (1 << 8) | (1 << 12) | (1 << 18) /* | 1 << 5 | 1 << 3 */,
            CSR_MHARTID => 0,
            CSR_SSTATUS => self.csr_read_cpu(CSR_MSTATUS) & MSTAT_S_MASK,
            CSR_SATP => {
                // TODO: enable this code after paging
                // if err && (self.csr_read_cpu(CSR_MSTATUS) >> 20) & 1 == 1 && self.mode == Mode::Supervisor {
                //     return Err(Exception::IllegalInst);
                // }

                self.csrs[a as usize]
            }
            0x7a0 | 0x7a5 => 1,
            _ => self.csrs[a as usize],
        })
    }

    pub fn csr_write_cpu(&mut self, a: u64, d: u64) {
        unsafe { self._csr_write(a, d, false).unwrap_unchecked() }
    }

    pub fn csr_write(&mut self, a: u64, d: u64) -> Result<(), Exception> {
        println!("csrw {a:03x} {d:016x}");
        self._csr_write(a, d, true)
    }

    fn _csr_write(&mut self, a: u64, d: u64, err: bool) -> Result<(), Exception> {
        let a = a & 4095;
        self.check_csr_perm(a, err)?;

        if a >> 10 == 3 {
            return Err(Exception::IllegalInst);
        }

        match a {
            CSR_MISA => {},
            CSR_MSTATUS => {
                self.csrs[a as usize] &= !MSTAT_W_MASK;
                self.csrs[a as usize] |= d & MSTAT_W_MASK;
            },
            CSR_SSTATUS => {
                const MASK: u64 = MSTAT_S_MASK & MSTAT_W_MASK;

                let mut mstat = self.csr_read_cpu(CSR_MSTATUS);
                println!("{mstat:016x}");
                mstat &= !MASK;
                mstat |= d & MASK;
                self.csr_write_cpu(CSR_MSTATUS, mstat);
                println!("{mstat:016x}");
            },
            CSR_SATP => {
                // TODO: enable this code after paging
                // if err && (self.csr_read_cpu(CSR_MSTATUS) >> 20) & 1 == 1 && self.mode == Mode::Supervisor {
                //     return Err(Exception::IllegalInst);
                // }

                self.csrs[a as usize] = d;
            }
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

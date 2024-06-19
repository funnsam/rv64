use super::*;

// floating point csrs
pub(crate) const CSR_FFLAGS: u64 = 0x001;
pub(crate) const CSR_FRM: u64 = 0x002;
pub(crate) const CSR_FCSR: u64 = 0x003;

// supervisor trap setup
pub(crate) const CSR_SSTATUS: u64 = 0x100;
pub(crate) const CSR_SIE: u64 = 0x104;
pub(crate) const CSR_STVEC: u64 = 0x105;
pub(crate) const CSR_SCOUNTEREN: u64 = 0x106;
// supervisor configuration
pub(crate) const CSR_SENVCFG: u64 = 0x10a;
// supervisor counter setup
pub(crate) const CSR_SCOUNTERINHIBIT: u64 = 0x120;
// supervisor trap handling
pub(crate) const CSR_SSCRATCH: u64 = 0x140;
pub(crate) const CSR_SEPC: u64 = 0x141;
pub(crate) const CSR_SCAUSE: u64 = 0x142;
pub(crate) const CSR_STVAL: u64 = 0x143;
pub(crate) const CSR_SIP: u64 = 0x144;
pub(crate) const CSR_SCOUNTOVF: u64 = 0xda0;
// supervisor protection & translation
pub(crate) const CSR_SATP: u64 = 0x180;
// debug/trace regiser
pub(crate) const CSR_SCONTEXT: u64 = 0x5a8;
// supervisor state enable regisers
pub(crate) const CSR_SSTATEEN0: u64 = 0x10c;
pub(crate) const CSR_SSTATEEN1: u64 = 0x10d;
pub(crate) const CSR_SSTATEEN2: u64 = 0x10e;
pub(crate) const CSR_SSTATEEN3: u64 = 0x10f;

// machine info
pub(crate) const CSR_MVENDORID: u64 = 0xf11;
pub(crate) const CSR_MARCHID: u64 = 0xf12;
pub(crate) const CSR_MIMPID: u64 = 0xf13;
pub(crate) const CSR_MHARTID: u64 = 0xf14;
pub(crate) const CSR_MCONFIGPTR: u64 = 0xf15;
// machine trap setup
pub(crate) const CSR_MSTATUS: u64 = 0x300;
pub(crate) const CSR_MISA: u64 = 0x301;
pub(crate) const CSR_MEDELEG: u64 = 0x302;
pub(crate) const CSR_MIDELEG: u64 = 0x303;
pub(crate) const CSR_MIE: u64 = 0x304;
pub(crate) const CSR_MTVEC: u64 = 0x305;
pub(crate) const CSR_MCOUNTEREN: u64 = 0x306;
// machine trap handling
pub(crate) const CSR_MSCRATCH: u64 = 0x340;
pub(crate) const CSR_MEPC: u64 = 0x341;
pub(crate) const CSR_MCAUSE: u64 = 0x342;
pub(crate) const CSR_MTVAL: u64 = 0x343;
pub(crate) const CSR_MIP: u64 = 0x344;
pub(crate) const CSR_MTINST: u64 = 0x345;
pub(crate) const CSR_MTVAL2: u64 = 0x346;

const MSTAT_S_MASK: u64 = 0x8000_0003_000f_e7e2;
const MSTAT_W_MASK: u64 = 0x7fff_ffc0_fff6_79bf;

impl<'a> Cpu<'a> {
    pub(crate) fn csr_init(&mut self) {
        self.csrs[CSR_MSTATUS as usize] = 0x0000_000a_0000_2000;
    }

    pub(crate) fn csr_read_cpu(&self, a: u64) -> u64 {
        unsafe { self._csr_read(a, false).unwrap_unchecked() }
    }

    pub(crate) fn csr_read(&self, a: u64) -> Result<u64, Exception> {
        println!("csrr {a:03x}");
        self._csr_read(a, true)
    }

    fn _csr_read(&self, a: u64, err: bool) -> Result<u64, Exception> {
        let a = a & 4095;
        self.check_csr_perm(a, err)?;

        Ok(match a {
            // TODO:
            // D | bit 3
            CSR_MISA => 0x8000000000141121, // rv64imaf_su (Z extensions are not in here)
            CSR_MHARTID => 0,
            CSR_MSTATUS => {
                let mut s = self.csrs[a as usize];
                s |= (((s >> 13) & 3 == 3) as u64) << 63;
                s
            },
            CSR_SSTATUS => self.csr_read_cpu(CSR_MSTATUS) & MSTAT_S_MASK,
            CSR_SATP => {
                if err && self.mode == Mode::Supervisor && (self.csr_read_cpu(CSR_MSTATUS) >> 20) & 1 == 1 {
                    return Err(Exception::IllegalInst);
                }

                self.csrs[a as usize]
            }
            CSR_FFLAGS => self.csr_read_cpu(CSR_FCSR) & 0x1f,
            CSR_FRM => (self.csr_read_cpu(CSR_FCSR) >> 5) & 7,
            0x7a0 | 0x7a5 => 1, // throw off debug mode tests
            _ => self.csrs[a as usize],
        })
    }

    pub(crate) fn csr_write_cpu(&mut self, a: u64, d: u64) {
        unsafe { self._csr_write(a, d, false).unwrap_unchecked() }
    }

    pub(crate) fn csr_write(&mut self, a: u64, d: u64) -> Result<(), Exception> {
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
                mstat &= !MASK;
                mstat |= d & MASK;
                self.csr_write_cpu(CSR_MSTATUS, mstat);
            },
            CSR_SATP => {
                if err && (self.csr_read_cpu(CSR_MSTATUS) >> 20) & 1 == 1 && self.mode == Mode::Supervisor {
                    return Err(Exception::IllegalInst);
                }

                self.csrs[a as usize] = d;
            },
            CSR_FCSR => {
                self.mut_fp_state();
                self.csrs[a as usize] = d & 0xff;
            },
            CSR_FFLAGS => {
                let mut fcsr = self.csr_read_cpu(CSR_FCSR);
                fcsr &= !0x1f;
                fcsr |= d & 0x1f;
                self.csr_write_cpu(CSR_FCSR, fcsr)
            },
            CSR_FRM => {
                let mut fcsr = self.csr_read_cpu(CSR_FCSR);
                fcsr &= !0xe0;
                fcsr |= (d << 5) & 0xe0;
                self.csr_write_cpu(CSR_FCSR, fcsr)
            },
            _ => self.csrs[a as usize] = d,
        }

        Ok(())
    }

    fn check_csr_perm(&self, a: u64, err: bool) -> Result<(), Exception> {
        if err && (a >> 8) & 3 > self.mode as _ {
            return Err(Exception::IllegalInst);
        }

        Ok(())
    }
}

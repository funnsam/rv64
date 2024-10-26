use super::*;

const PERM_R: u64 = 0x02;
const PERM_W: u64 = 0x04;
const PERM_X: u64 = 0x08;
const PERM_U: u64 = 0x10;

impl<'a> Cpu<'a> {
    pub(crate) fn flush_mapping(&mut self) -> Result<(), Exception> {
        let satp = self.csr_read(csr::CSR_SATP)?;

        self.pages = match satp >> 60 {
            0 => Paging::Bare,
            8 => Paging::Sv39 { address: (satp & 0xfffffffffff) << 12 },
            _ => return Ok(()),
        };

        Ok(())
    }

    pub(crate) fn resolve_paging(&mut self, a: u64, perm_mask: (u64, u64, bool)) -> Result<u64, ()> {
        if perm_mask.2 || (self.mode == Mode::Machine && (perm_mask.0 & PERM_X) != 0) {
            return Ok(a);
        }

        // println!("page");

        match self.pages {
            Paging::Bare => Ok(a),
            Paging::Sv39 { address } => {
                self.resolve_sv39(a, perm_mask, address)
            },
        }
    }

    fn resolve_sv39(&mut self, a: u64, perm_mask: (u64, u64, bool), mut address: u64) -> Result<u64, ()> {
        let offset = a & 0xfff;
        let vpn0 = (a >> 12) & 0x1ff;
        let vpn1 = (a >> 21) & 0x1ff;
        let vpn2 = (a >> 30) & 0x1ff;
        let vpn = [vpn0, vpn1, vpn2];
        let store = (perm_mask.0 & PERM_W) != 0;

        for i in (0..=2).rev() {
            let pte_addr = address + (vpn[i] << 3);
            let pte = self.bus.load_u64(pte_addr).map_err(|_| ())?;

            // rv64 priv: If pte.v=0, or if pte.r=0 and pte.w=1
            if (pte & 1 == 0) || (pte & 2 == 0 && pte & 4 == 4) {
                // println!("a");
                return Err(());
            }

            // rv64 priv: If pte.r=1 or pte.x=1, go to step 5
            if pte & (PERM_R | PERM_X) == 0 {
                address = pte >> 10 << 12;
                continue;
            }

            if pte & perm_mask.0 != perm_mask.0 || pte & perm_mask.1 != 0 {
                return Err(());
            }

            let pte_ppn0 = (pte >> 10) & 0x1ff;
            let pte_ppn1 = (pte >> 19) & 0x1ff;
            let pte_ppn2 = (pte >> 28) & 0x3ffffff;
            let pte_ppn = [pte_ppn0, pte_ppn1, pte_ppn2];

            // misaligned
            if i > 0 && pte_ppn[..i as usize].iter().any(|v| *v != 0) {
                return Err(());
            }

            if pte & 0x20 == 0 || (store && pte & 0x40 == 0) {
                self.bus.store_u64(pte_addr, pte | 0x40 | ((store as u64) << 7)).map_err(|_| ())?;
            }

            let mut ppn = [0; 3];
            let (l, r) = ppn.split_at_mut(i);
            if i > 0 {
                l.copy_from_slice(&vpn[..i]);
            }

            r.copy_from_slice(&pte_ppn[i..]);

            return Ok((ppn[2] << 30) | (ppn[1] << 21) | (ppn[0] << 12) | offset);
        }

        Err(())
    }

    pub(crate) fn mmu_load_xu32(&mut self, a: u64) -> Result<u32, Exception> {
        self.resolve_paging(a, self.get_perm(PERM_X))
            .map_err(|_| Exception::InstPageFault)
            .and_then(|a| self.bus.load_u32(a).map_err(|_| Exception::InstAccessFault))
    }

    fn get_perm(&self, p: u64) -> (u64, u64, bool) {
        self.get_perm_mode(p, self.mode)
    }

    fn get_perm_mode(&self, p: u64, mode: Mode) -> (u64, u64, bool) {
        match mode {
            Mode::User => (p | PERM_U, 0, false),
            Mode::Supervisor => {
                (p, (!(self.csr_read_cpu(csr::CSR_MSTATUS) >> 18) & 1) * PERM_U, false)
            },
            Mode::Hypervisor => (p, 0, false),
            Mode::Machine => {
                let mstat = self.csr_read_cpu(csr::CSR_MSTATUS);
                let mpp = Mode::from_code((mstat >> 11) & 3);
                if (mstat >> 17) & 1 != 0 && mpp != Mode::Machine {
                    self.get_perm_mode(p, mpp)
                } else {
                    (p, 0, true)
                }
            },
        }
    }
}

#[derive(Debug)]
pub(crate) enum Paging {
    Bare,
    Sv39 { address: u64 },
}

macro_rules! gen {
    ($t: tt $l: tt $s: tt $r: tt $w: tt) => {
        impl Cpu<'_> {
            pub(crate) fn $r(&mut self, a: u64) -> Result<$t, Exception> {
                // println!("{} {a:016x}", stringify!($t));
                self.resolve_paging(a, self.get_perm(PERM_R))
                    .map_err(|_| Exception::LoadPageFault)
                    .and_then(|a| self.bus.$l(a))
            }

            pub(crate) fn $w(&mut self, a: u64, d: $t) -> Result<(), Exception> {
                // println!("{} {a:016x}", stringify!($t));
                self.resolve_paging(a, self.get_perm(PERM_W))
                    .map_err(|_| Exception::StorePageFault)
                    .and_then(|a| self.bus.$s(a, d))
            }
        }
    };
}

gen!(u8 load_u8 store_u8 mmu_load_u8 mmu_store_u8);
gen!(u16 load_u16 store_u16 mmu_load_u16 mmu_store_u16);
gen!(u32 load_u32 store_u32 mmu_load_u32 mmu_store_u32);
gen!(u64 load_u64 store_u64 mmu_load_u64 mmu_store_u64);

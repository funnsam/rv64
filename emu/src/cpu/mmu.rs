use super::*;

impl<'a> Cpu<'a> {
    pub fn flush_mapping(&mut self) -> Result<(), Exception> {
        let satp = self.csr_read(csr::CSR_SATP)?;
        Ok(())
    }

    pub fn resolve_sv39(&self, a: u64) -> Result<u64, ()> {
        match self.pages {
            Paging::Bare => Ok(a),
            Paging::Sv39 { mut address } => {
                let offset = a & 0xfff;
                let vpn0 = (a >> 12) & 0x1ff;
                let vpn1 = (a >> 21) & 0x1ff;
                let vpn2 = (a >> 30) & 0x1ff;
                let vpn = [vpn0, vpn1, vpn2];

                for i in (0..=2).rev() {
                    let pte = self.bus.load_u64(address + vpn[i] << 3)?;

                    // rv64 priv: If pte.v=0, or if pte.r=0 and pte.w=1
                    if (pte & 1 == 0) || (pte & 2 == 0 && pte & 4 == 4) {
                        return Err(());
                    }

                    // rv64 priv: If pte.r=1 or pte.x=1, go to step 5
                    if pte & 10 != 0 {
                        // TODO: check priv

                        // misaligned
                        if i > 0 && pte >> 10 & ((1 << (i - 1)) - 1) != 0 {
                            return Err(());
                        }

                        if pte & 0x40 == 0 {
                            // TODO: step 7
                            todo!();
                        }

                        // TODO: step 8 ppn
                        return if i > 0 {
                            // superpage
                            Ok(offset)
                        } else {
                            Ok(offset)
                        };
                    }

                    address = pte >> 10 << 12;
                }
                Err(())
            }
        }
    }
}

pub enum Paging {
    Bare,
    Sv39 { address: u64 },
}

// struct Sv39Entry {
//     ppn2: u32,
//     ppn1: u16,
//     ppn0: u16,
//     dirty: bool,
//     access: bool,
//     global: bool,
//     user: bool,
//     exec: bool,
//     read: bool,
//     write: bool,
// }

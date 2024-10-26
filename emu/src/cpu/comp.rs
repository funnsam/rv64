use super::*;

impl Cpu<'_> {
    pub(super) fn comp_expand(&self, inst: u16) -> Result<u32, Exception> {
        macro_rules! decode {
            (ci $($weird: tt $rds1: tt $exec: expr),* $(,)?) => {{
                let rds1 = (inst >> 7) & 31;
                match rds1 as u32 {
                    $($rds1 => {
                        let imm = if $weird {
                            ((((inst & 0x40) >> 6)
                                | ((inst & 0x04) >> 1)
                                | ((inst & 0x20) >> 3)
                                | (inst & 0x18)) as i16
                            | ((inst as i16 & 0x1000) << 3 >> 10)) as u32
                        } else {
                            (((inst & 0x7c) >> 2) as i16 | ((inst as i16 & 0x1000) << 3 >> 10)) as u32
                        };
                        $exec(imm)
                    },)*
                }
            }};
            (ciw $exec: expr) => {{
                let rd = ((inst & 0x1c) >> 2) + 8;
                let imm = ((inst & 0x40) >> 4)
                    | ((inst & 0x20) >> 2)
                    | ((inst & 0x1800) >> 7)
                    | ((inst & 0x780) >> 1);
                $exec(rd as u32, imm as u32)
            }};
            (cls $exec: expr) => {{
                let rds2 = ((inst & 0x1c) >> 2) + 8;
                let r1 = ((inst & 0x380) >> 7) + 8;
                let imm = ((inst & 0x40) >> 4) // [2]
                    | ((inst & 0x1c00) >> 7) // [5:3]
                    | ((inst & 0x20) << 1); // [6]
                $exec(rds2 as u32, r1 as u32, imm as u32)
            }};
            (clsd $exec: expr) => {{
                let rds2 = ((inst & 0x1c) >> 2) + 8;
                let r1 = ((inst & 0x380) >> 7) + 8;
                let imm = ((inst & 0x1c00) >> 7) // [5:3]
                    | ((inst & 0x60) << 1); // [7:6]
                $exec(rds2 as u32, r1 as u32, imm as u32)
            }};
            (cbf2 $($f2: tt $itop: tt $exec: expr),* $(,)?) => {{
                let rds1 = ((inst & 0x380) >> 7) + 8;
                let r2 = ((inst & 0x1c) >> 2) + 8;
                let imm = ((inst & 0x7c) >> 2) as i16
                    | ((inst as i16 & 0x1000) << 3 >> 10);
                match ((inst & 0xc00) >> 10, (imm >> 3) & 7) {
                    $(($f2, $itop) => $exec(rds1 as u32, r2 as u32, imm as u32),)*
                }
            }};
            (cb $exec: expr) => {{
                let r1 = ((inst & 0x380) >> 7) + 8;
                let imm = ((inst & 0x18) >> 2) as i16 // [2:1]
                    | ((inst & 0xc00) >> 7) as i16 // [4:3]
                    | ((inst & 0x4) << 3) as i16 // [5]
                    | ((inst & 0x60) << 1) as i16 // [7:6]
                    | ((inst as i16 & 0x1000) << 3 >> 10); // [8]
                $exec(r1 as u32, imm as u32)
            }};
            (cr $($f1: tt $rds1: tt $r2: tt $exec: expr),* $(,)?) => {{
                let rds1 = (inst & 0xf80) >> 7;
                let r2 = (inst & 0x7c) >> 2;
                match ((inst & 0x1000) >> 12, rds1 as u32, r2 as u32) {
                    $(($f1, $rds1, $r2) => $exec(),)*
                }
            }};
            (cisw $exec: expr) => {{
                let rd = (inst & 0xf80) >> 7;
                let imm = ((inst & 0x70) >> 2) // [4:2]
                    | ((inst & 0x1000) >> 7) // [5]
                    | ((inst & 0xc) << 4); // [7:6]
                $exec(rd as u32, imm as u32)
            }};
            (cisd $exec: expr) => {{
                let rd = (inst & 0xf80) >> 7;
                let imm = ((inst & 0x60) >> 2) // [4:3]
                    | ((inst & 0x1000) >> 7) // [5]
                    | ((inst & 0x1c) << 4); // [8:6]
                $exec(rd as u32, imm as u32)
            }};
            (cssw $exec: expr) => {{
                let r2 = (inst & 0x7c) >> 2;
                let imm = ((inst & 0x1e00) >> 7) // [5:2]
                    | ((inst & 0x180) >> 1); // [7:6]
                $exec(r2 as u32, imm as u32)
            }};
            (cssd $exec: expr) => {{
                let r2 = (inst & 0x7c) >> 2;
                let imm = ((inst & 0x1c00) >> 7) // [5:3]
                    | ((inst & 0x380) >> 1); // [8:6]
                $exec(r2 as u32, imm as u32)
            }};
        }

        if inst == 0 { return Err(Exception::IllegalInst); }

        match (inst & 3, inst >> 13) {
            (0, 0) => decode!(ciw |rd, imm| Ok(0x00010013_u32 | (imm << 20) | (rd << 7))),
            (0, 1) => decode!(clsd |rd, r1, imm| Ok(0x00003007_u32 | (rd << 7) | (r1 << 15) | (imm << 20))),
            (0, 2) => decode!(cls |rd, r1, imm| Ok(0x00002003_u32 | (rd << 7) | (r1 << 15) | (imm << 20))),
            (0, 3) => decode!(clsd |rd, r1, imm| Ok(0x00003003_u32 | (rd << 7) | (r1 << 15) | (imm << 20))),
            (0, 5) => decode!(clsd |r2, r1, imm| Ok(0x00003027_u32 | (r1 << 15) | (r2 << 20) | ((imm & 0x1f) << 7) | ((imm & !0x1f) << 20))),
            (0, 6) => decode!(cls |r2, r1, imm| Ok(0x00002023_u32 | (r1 << 15) | (r2 << 20) | ((imm & 0x1f) << 7) | ((imm & !0x1f) << 20))),
            (0, 7) => decode!(clsd |r2, r1, imm| Ok(0x00003023_u32 | (r1 << 15) | (r2 << 20) | ((imm & 0x1f) << 7) | ((imm & !0x1f) << 20))),
            (1, 0) => decode!(ci
                false rds1 |imm| Ok(0x00000013_u32 | (rds1 << 7) | (rds1 << 15) | (imm << 20)),
            ),
            (1, 1) => decode!(ci
                false rds1 |imm| Ok(0x0000001b_u32 | (rds1 << 7) | (rds1 << 15) | (imm << 20)),
            ),
            (1, 2) => decode!(ci
                false rd |imm| Ok(0x00000013_u32 | (rd << 7) | (imm << 20)),
            ),
            (1, 3) => decode!(ci
                true 2 |imm| Ok(0x00010113_u32 | (imm << 24)),
                false rd |imm| Ok(0x00000037_u32 | (rd << 7) | (imm << 12)),
            ),
            (1, 4) => decode!(cbf2
                0 _ |rds1, _, imm| Ok(0x00005013_u32 | (rds1 << 7) | (rds1 << 15) | ((imm & 0x3f) << 20)),
                1 _ |rds1, _, imm| Ok(0x40005013_u32 | (rds1 << 7) | (rds1 << 15) | ((imm & 0x3f) << 20)),
                2 _ |rds1, _, imm| Ok(0x00007013_u32 | (rds1 << 7) | (rds1 << 15) | (imm << 20)),
                3 0 |rds1, r2, _| Ok(0x40000033_u32 | (rds1 << 7) | (rds1 << 15) | (r2 << 20)),
                3 1 |rds1, r2, _| Ok(0x00004033_u32 | (rds1 << 7) | (rds1 << 15) | (r2 << 20)),
                3 2 |rds1, r2, _| Ok(0x00006033_u32 | (rds1 << 7) | (rds1 << 15) | (r2 << 20)),
                3 3 |rds1, r2, _| Ok(0x00007033_u32 | (rds1 << 7) | (rds1 << 15) | (r2 << 20)),
                3 4 |rds1, r2, _| Ok(0x4000003b_u32 | (rds1 << 7) | (rds1 << 15) | (r2 << 20)),
                3 5 |rds1, r2, _| Ok(0x0000003b_u32 | (rds1 << 7) | (rds1 << 15) | (r2 << 20)),
                _ _ |_, _, _| Err(Exception::IllegalInst),
            ),
            (1, 5) => {
                let imm = ((inst & 0x38) >> 2) as i16 // [3:1]
                    | ((inst & 0x800) >> 7) as i16 // [4]
                    | ((inst & 0x4) << 3) as i16 // [5]
                    | ((inst & 0x80) >> 1) as i16 // [6]
                    | ((inst & 0x40) << 1) as i16 // [7]
                    | ((inst & 0x600) >> 1) as i16 // [9:8]
                    | ((inst & 0x100) << 2) as i16 // [10]
                    | ((inst as i16 & 0x1000) << 3 >> 4); // [11]
                let imm = imm as u32;
                Ok(0x0000006f_u32 | ((imm & 0x7fe) << 20) | ((imm & 0x800) << 9) | (imm & 0xff000) | ((imm & 0x10000) << 11))
            },
            (1, 6) => decode!(cb |r1, imm| Ok(b(0x00000063_u32, r1, imm))),
            (1, 7) => decode!(cb |r1, imm| Ok(b(0x00001063_u32, r1, imm))),
            (2, 0) => decode!(ci
                false rds1 |imm| Ok(0x00001013_u32 | (rds1 << 7) | (rds1 << 15) | ((imm & 0x3f) << 20)),
            ),
            (2, 1) => decode!(cisd |rd, imm| Ok(0x00003007_u32 | (rd << 7) | (imm << 20))),
            (2, 2) => decode!(cisw |rd, imm| Ok(0x00012003_u32 | (rd << 7) | (imm << 20))),
            (2, 3) => decode!(cisd |rd, imm| Ok(0x00013003_u32 | (rd << 7) | (imm << 20))),
            (2, 4) => decode!(cr
                0 r1 0 || Ok(0x00000067_u32 | (r1 << 15)),
                0 rd r2 || Ok(0x00000033_u32 | (rd << 7) | (r2 << 20)),
                1 0 0 || Ok(0x00100073),
                1 r1 0 || Ok(0x000000e7_u32 | (r1 << 15)),
                1 rds1 r2 || Ok(0x00000033_u32 | (rds1 << 7) | (rds1 << 15) | (r2 << 20)),
                _ _ _ || unreachable!(),
            ),
            (2, 5) => decode!(cssd |r2, imm| Ok(0x00003027_u32 | (r2 << 20) | ((imm & 0x1f) << 7) | ((imm & !0x1f) << 20))),
            (2, 6) => decode!(cssw |r2, imm| Ok(0x00012023_u32 | (r2 << 20) | ((imm & 0x1f) << 7) | ((imm & !0x1f) << 20))),
            (2, 7) => decode!(cssd |r2, imm| Ok(0x00013023_u32 | (r2 << 20) | ((imm & 0x1f) << 7) | ((imm & !0x1f) << 20))),
            _ => Err(Exception::IllegalInst),
        }
    }
}

fn b(base: u32, r1: u32, imm: u32) -> u32 {
    base | (r1 << 15) | ((imm & 0x1e) << 7) | ((imm & 0x7e0) << 20) | ((imm & 0x800) >> 4) | ((imm & 0x1000) << 19)
}

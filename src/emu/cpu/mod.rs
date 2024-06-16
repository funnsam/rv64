use super::*;
mod csr;

pub struct Cpu<'a> {
    bus: &'a mut bus::Bus<'a>,
    io: &'a mut io::Io,

    regs: [u64; 31],
    pc: u64,

    csrs: Box<[u64; 4096]>,
    mode: Mode,
}

impl<'a> Cpu<'a> {
    pub fn new(bus: &'a mut bus::Bus<'a>, io: &'a mut io::Io) -> Self {
        Self {
            bus,
            io,

            regs: [0; 31],
            pc: 0x80000000,

            csrs: Box::new([0; 4096]),
            mode: Mode::Machine,
        }
    }

    pub fn step(&mut self) {
        let inst = self.fetch();
        if let Err(ex) = self.execute(inst) {
            self.exception(ex);
        }
    }

    fn fetch(&mut self) -> u32 {
        let i = self.bus.load_u32(self.pc);
        self.pc += 4;
        i
    }

    fn execute(&mut self, inst: u32) -> Result<(), Exception> {
        let opc = inst & 0x7f;

        macro_rules! exec {
            (r [$($f3: tt $f7: tt $exec: expr),* $(,)?]) => {{
                let r1 = (inst >> 15) & 0x1f;
                let r1 = self.read_reg(r1 as _);
                let r2 = (inst >> 20) & 0x1f;
                let r2 = self.read_reg(r2 as _);
                let rd = (inst >> 7) & 0x1f;

                let v = match ((inst >> 12) & 7, inst >> 25) {
                    $(
                        ($f3, $f7) => $exec(r1, r2),
                    )*
                    (f3, f7) => todo!("r {inst:08x} {opc:02x} {f3:01x} {f7:02x}"),
                };

                self.write_reg(rd as _, v);
            }};
            (i [$($f3: tt $exec: expr),* $(,)?]) => {{
                exec!(ii [$($f3 false |a, b, _ds, _as| $exec(a, b)),*]);
            }};
            (ii [$($f3: tt $im: tt $exec: expr),* $(,)?]) => {{
                let rs = (inst >> 15) & 0x1f;
                let im = (inst as i32 >> 20) as u64;
                let rd = (inst >> 7) & 0x1f;

                let v = match (inst >> 12) & 7 {
                    $(
                        $f3 => {
                            let rs_v = if $im { rs as _ } else { self.read_reg(rs as _) };
                            $exec(rs_v, im, rd, rs)
                        },
                    )*
                    f3 => todo!("{inst:08x} i {opc:02x} {f3:01x}"),
                };

                self.write_reg(rd as _, v);
            }};
            (s [$($f3: tt $exec: expr),* $(,)?]) => {{
                let r1 = (inst >> 15) & 0x1f;
                let r1 = self.read_reg(r1 as _);
                let r2 = (inst >> 20) & 0x1f;
                let r2 = self.read_reg(r2 as _);
                let im = ((inst & 0xfe00_0000) as i32 >> 20) as u64 // [11:5]
                    | ((inst >> 7) & 0x1f) as u64; // [4:0]
                match (inst >> 12) & 7 {
                    $(
                        $f3 => $exec(r1, r2, im),
                    )*
                    f3 => todo!("{inst:08x} s {opc:02x} {f3:01x}"),
                };
            }};
            (b [$($f3: tt $exec: expr),* $(,)?]) => {{
                let r1 = (inst >> 15) & 0x1f;
                let r1 = self.read_reg(r1 as _);
                let r2 = (inst >> 20) & 0x1f;
                let r2 = self.read_reg(r2 as _);
                let im = ((inst & 0x8000_0000) as i32 >> 19) as u64 // [12]
                    | ((inst & 0x80) << 4) as u64 // [11]
                    | ((inst >> 20) & 0x7e0) as u64 // [10:5]
                    | ((inst >> 7) & 0x1e) as u64; // [4:1]
                match (inst >> 12) & 7 {
                    $(
                        $f3 => if $exec(r1, r2) {
                            self.pc += im - 4;
                        },
                    )*
                    f3 => todo!("{inst:08x} b {opc:02x} {f3:01x}"),
                };
            }};
            (u $exec: expr) => {{
                let im = (inst & 0xFFFF_F000) as i32 as u64;
                let rd = (inst >> 7) & 0x1f;
                let v = $exec(im);
                self.write_reg(rd as _, v);
            }};
            (j $exec: expr) => {{
                let im = ((inst & 0x8000_0000) as i32 >> 11) as u64 // [20]
                    | (inst & 0xff000) as u64 // [19:12]
                    | ((inst >> 9) & 0x800) as u64 // [11]
                    | ((inst >> 20) & 0x7fe) as u64; // [10:1]
                let rd = (inst >> 7) & 0x1f;
                let v = $exec(im);
                self.write_reg(rd as _, v);
            }};
            (p [$($priv: tt $f3: tt $f7: tt $rds: tt $r1s: tt $r2s: tt $exec: expr),* $(,)?]) => {{
                let r1s = (inst >> 15) & 0x1f;
                let r1 = self.read_reg(r1s as _);
                let r2s = (inst >> 20) & 0x1f;
                let r2 = self.read_reg(r2s as _);
                let rds = (inst >> 7) & 0x1f;
                let im = (inst as i32 >> 20) as u64;

                let v = match ((inst >> 12) & 7, inst >> 25, rds, r1s, r2s) {
                    $(
                        ($f3, $f7, $rds, $r1s, $r2s) => {
                            if self.mode < Mode::$priv {
                                return Err(Exception::IllegalInst);
                            }

                            $exec(r1, r2, im)?
                        },
                    )*
                    _ => todo!("p {inst:08x} {opc:02x}"),
                };

                if let Some(v) = v {
                    self.write_reg(rds as _, v);
                }
            }};
        }

        match opc {
            0x37 => exec!(u |a| a),
            0x17 => exec!(u |a| self.pc + a - 4),
            0x6f => exec!(j |a| {
                let pc = self.pc;
                core::mem::replace(&mut self.pc, pc + a - 4)
            }),
            0x63 => exec!(b [
                0x0 |a, b| a == b,
                0x1 |a, b| a != b,
                0x4 |a, b| (a as i64) < (b as i64),
                0x5 |a, b| (a as i64) >= (b as i64),
                0x6 |a, b| a < b,
                0x7 |a, b| a >= b,
            ]),
            0x03 => exec!(i [
                0x0 |a, b| self.bus.load_u8(a + b) as i8 as u64,
                0x1 |a, b| self.bus.load_u16(a + b) as i16 as u64,
                0x2 |a, b| self.bus.load_u32(a + b) as i32 as u64,
                0x3 |a, b| self.bus.load_u64(a + b),
                0x4 |a, b| self.bus.load_u8(a + b) as u64,
                0x5 |a, b| self.bus.load_u16(a + b) as u64,
                0x6 |a, b| self.bus.load_u32(a + b) as u64,
            ]),
            0x23 => exec!(s [
                0x0 |a, b, c| self.bus.store_u8(a + c, b as _),
                0x1 |a, b, c| self.bus.store_u16(a + c, b as _),
                0x2 |a, b, c| {
                    println!("{:016x} {}", a + c, b);
                    self.bus.store_u32(a + c, b as _);
                },
                0x3 |a, b, c| self.bus.store_u64(a + c, b as _),
            ]),
            0x13 => exec!(i [
                0x0 |a, b| a + b,
                0x2 |a, b| ((a as i64) < (b as i64)) as u64,
                0x3 |a, b| (a < b) as u64,
                0x4 |a, b| a ^ b,
                0x6 |a, b| a | b,
                0x7 |a, b| a & b,
                0x1 |a, b| a << (b & 0x3f),
                0x5 |a, b| if b >> 6 == 0_u64 {
                    a >> (b & 0x3f)
                } else {
                    ((a as i64) >> (b & 0x3f)) as u64
                },
            ]),
            0x33 => exec!(r [
                0x0 0x00 |a, b| a + b,
                0x0 0x20 |a, b| a - b,
                0x1 0x00 |a, b| a << (b & 0x3f),
                0x2 0x00 |a, b| ((a as i64) < (b as i64)) as u64,
                0x3 0x00 |a, b| (a < b) as u64,
                0x4 0x00 |a, b| a ^ b,
                0x5 0x00 |a, b| a >> (b & 0x3f),
                0x5 0x20 |a, b| ((a as i64) >> (b & 0x3f)) as u64,
                0x6 0x00 |a, b| a | b,
                0x7 0x00 |a, b| a & b,
            ]),
            0x67 => exec!(i [
                0x0 |a, b| core::mem::replace(&mut self.pc, (a + b) & !1),
            ]),
            0x1b => exec!(i [
                0x0 |a, b| (a + b) as i32 as u64,
                0x1 |a, b| (a << (b & 0x1f)) as i32 as u64,
                0x5 |a, b| if b >> 6 == 0_u64 {
                    ((a as u32) >> (b & 0x1f)) as i32 as u64
                } else {
                    ((a as i32) >> (b & 0x1f)) as u64
                },
            ]),
            0x3b => exec!(r [
                0x0 0x00 |a, b| (a + b) as i32 as u64,
                0x0 0x20 |a, b| (a - b) as i32 as u64,
                0x1 0x00 |a, b| (a << (b & 0x1f)) as i32 as u64,
                0x5 0x00 |a, b| ((a as u32) >> (b & 0x1f)) as i32 as u64,
                0x5 0x20 |a, b| ((a as i32) >> (b & 0x1f)) as u64,
            ]),
            0x0f => {},
            0x73 => exec!(p [
                User 0x1 _ dr _ _ |a, _, b| { // csrrw
                    let rv = if dr != 0 {
                        self.csr_read(b)?
                    } else {
                        0
                    };

                    self.csr_write(b, a)?;
                    Ok(Some(rv))
                },
                User 0x5 _ dr a _ |_, _, b| { // csrrwi
                    let rv = if dr != 0 {
                        self.csr_read(b)?
                    } else {
                        0
                    };

                    self.csr_write(b, a as _)?;
                    Ok(Some(rv))
                },
                User 0x2 _ _ sr _ |a, _, b| { // csrrs
                    let rv = self.csr_read(b)?;

                    if sr != 0 {
                        self.csr_write(b, a | rv)?;
                    }

                    Ok(Some(rv))
                },
                User 0x6 _ _ a _ |_, _, b| { // csrrsi
                    let rv = self.csr_read(b)?;

                    if a != 0 {
                        self.csr_write(b, a as u64 | rv)?;
                    }

                    Ok(Some(rv))
                },
                // 0x3 false |a, b, dr, sr| 0, // csrrc
                // 0x7 true |a, b, dr, sr| 0, // csrrci
                Machine 0x0 0x18 0x00 0x00 0x02 |_, _, _| { // mret
                    let epc = self.csr_read_cpu(csr::CSR_MEPC);
                    self.pc = epc;
                    Ok(None)
                },
                User 0x0 0x00 0x00 0x00 0x00 |_, _, _| { // ecall
                    self.exception(self.mode.ecall_exception());
                    Ok(None)
                },
            ]),
            _ => todo!("{inst:08x} {opc:02x}"),
        }

        Ok(())
    }

    fn exception(&mut self, cause: Exception) {
        self.trap(cause as _);
    }

    fn interrupt(&mut self, cause: Interrupt) {
        self.trap(cause as u64 | (1 << 63));
    }

    fn trap(&mut self, cause: u64) {
        self.csr_write_cpu(csr::CSR_MCAUSE, cause);
        self.csr_write_cpu(csr::CSR_MEPC, self.pc);
        self.pc = self.csr_read_cpu(csr::CSR_MTVEC); // TODO: mtvec modes
        // TODO: switch mode
    }

    fn read_reg(&self, r: usize) -> u64 {
        if r == 0 { 0 } else { self.regs[r - 1] }
    }

    fn write_reg(&mut self, r: usize, v: u64) {
        if r != 0 { self.regs[r - 1] = v; }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Mode {
    User = 0,
    Supervisor = 1,
    Hypervisor = 2,
    Machine = 3,
}

impl Mode {
    fn ecall_exception(&self) -> Exception {
        match self {
            Self::User => Exception::EcallFromUser,
            Self::Supervisor => Exception::EcallFromSupervisor,
            Self::Hypervisor => Exception::EcallFromReserved,
            Self::Machine => Exception::EcallFromMachine,
        }
    }
}

enum Exception {
    InstAddrMisalign = 0,
    InstAccessFault = 1,
    IllegalInst = 2,
    Breakpoint = 3,
    LoadAddrMisalign = 4,
    LoadAccessFault = 5,
    StoreAddrMisalign = 6,
    StoreAccessFault = 7,
    EcallFromUser = 8,
    EcallFromSupervisor = 9,
    EcallFromReserved = 10,
    EcallFromMachine = 11,
    InstPageFault = 12,
    LoadPageFault = 13,
    // reserved
    StorePageFault = 15,
    // reserved
    SoftwareCheck = 18,
    HardwareError = 19,
}

enum Interrupt {
    SupervisorSoftwareInt = 1,
    MachineSoftwareInt = 3,
    SupervisorTimerInt = 5,
    MachineTimerInt = 7,
    SupervisorExternalInt = 9,
    MachineExternalInt = 11,
    CounterOverflowInt = 13,
}

use super::*;
use float::Snan;

mod atomic;
mod csr;
mod float;
mod mmu;

pub struct Cpu<'a> {
    bus: &'a mut bus::Bus<'a>,
    io: &'a mut io::Io,

    regs: [u64; 31],
    float_regs: [u64; 32],
    pc: u64,
    mode: Mode,

    csrs: Box<[u64; 4096]>,
    pages: mmu::Paging,
    amo_rs: atomic::ReservationSet,
}

impl<'a> Cpu<'a> {
    pub fn new(bus: &'a mut bus::Bus<'a>, io: &'a mut io::Io) -> Self {
        let mut cpu = Self {
            bus,
            io,

            regs: [0; 31],
            float_regs: [0; 32],
            pc: 0x80000000,
            mode: Mode::Machine,

            csrs: Box::new([0; 4096]),
            pages: mmu::Paging::Bare,
            amo_rs: atomic::ReservationSet::new(),
        };
        cpu.csr_init();
        cpu
    }

    fn step_w_exception(&mut self, testing: bool) -> Result<(), Exception> {
        let inst = self.fetch()?;
        self.execute(inst, testing)?;
        self.check_interrupts();
        Ok(())
    }

    pub fn step(&mut self, testing: bool) {
        if let Err(ex) = self.step_w_exception(testing) {
            self.exception(ex);
        }
    }

    fn fetch(&mut self) -> Result<u32, Exception> {
        println!("{:08x}", self.pc);
        if self.pc & 3 == 0 {
            let i = self.mmu_load_xu32(self.pc);
            self.pc += 4;
            i
        } else {
            Err(Exception::InstAddrMisalign)
        }
    }

    fn execute(&mut self, inst: u32, testing: bool) -> Result<(), Exception> {
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
                        ($f3, $f7) => $exec(r1, r2)?,
                    )*
                    _ => return Err(Exception::IllegalInst),
                };

                self.write_reg(rd as _, v)?;
            }};
            (i [$($f3: tt $exec: expr),* $(,)?]) => { exec!(ix [$($f3 write_reg $exec),*]) };
            (ix [$($f3: tt $wr: tt $exec: expr),* $(,)?]) => {{
                let rs = (inst >> 15) & 0x1f;
                let rs = self.read_reg(rs as _);
                let im = (inst as i32 >> 20) as u64;
                let rd = (inst >> 7) & 0x1f;

                match (inst >> 12) & 7 {
                    $(
                        $f3 => {
                            let v = $exec(rs, im)?;
                            self.$wr(rd as _, v)?;
                        },
                    )*
                    _ => return Err(Exception::IllegalInst),
                }
            }};
            (s [$($f3: tt $exec: expr),* $(,)?]) => { exec!(sx [$($f3 read_reg $exec),*]) };
            (sx [$($f3: tt $m: tt $exec: expr),* $(,)?]) => {{
                let r1 = (inst >> 15) & 0x1f;
                let r1 = self.read_reg(r1 as _);
                let r2 = (inst >> 20) & 0x1f;
                let im = ((inst & 0xfe00_0000) as i32 >> 20) as u64 // [11:5]
                    | ((inst >> 7) & 0x1f) as u64; // [4:0]
                match (inst >> 12) & 7 {
                    $(
                        $f3 => {
                            let r2 = self.$m(r2 as _);
                            $exec(r1, r2, im)?
                        },
                    )*
                    _ => return Err(Exception::IllegalInst),
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
                        $f3 => if $exec(r1, r2)? {
                            self.write_pc(self.pc + im - 4)?;
                        },
                    )*
                    _ => return Err(Exception::IllegalInst),
                };
            }};
            (u $exec: expr) => {{
                let im = (inst & 0xFFFF_F000) as i32 as u64;
                let rd = (inst >> 7) & 0x1f;
                let v = $exec(im)?;
                self.write_reg(rd as _, v)?;
            }};
            (j $exec: expr) => {{
                let im = ((inst & 0x8000_0000) as i32 >> 11) as u64 // [20]
                    | (inst & 0xff000) as u64 // [19:12]
                    | ((inst >> 9) & 0x800) as u64 // [11]
                    | ((inst >> 20) & 0x7fe) as u64; // [10:1]
                let rd = (inst >> 7) & 0x1f;
                let v = $exec(im)?;
                self.write_reg(rd as _, v)?;
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
                    _ => return Err(Exception::IllegalInst),
                };

                if let Some(v) = v {
                    self.write_reg(rds as _, v)?;
                }
            }};
            (amo [$($f3: tt $f5: tt $exec: expr),* $(,)?]) => {{
                let r1 = (inst >> 15) & 0x1f;
                let r1 = self.read_reg(r1 as _);
                let r2 = (inst >> 20) & 0x1f;
                let r2 = self.read_reg(r2 as _);
                let rd = (inst >> 7) & 0x1f;
                let aqrl = atomic::AqRlMode::from_bits(inst >> 25);

                let v = match ((inst >> 12) & 7, inst >> 27) {
                    $(
                        ($f3, $f5) => $exec(r1, r2, aqrl)?,
                    )*
                    _ => return Err(Exception::IllegalInst),
                };

                self.write_reg(rd as _, v)?;
            }};
            (_ getrwf f32) => { (Self::read_float_reg_f32, Self::write_float_reg_f32) };
            (_ getrwf f64) => { (Self::read_float_reg_f64, Self::write_float_reg_f64) };
            (_ getrwf sru2i) => { (Self::read_float_reg_r32_uc, Self::write_reg) };
            (_ getrwf dru2i) => { (Self::read_float_reg_r64_uc, Self::write_reg) };
            (_ getrwf f2i) => { (Self::read_float_reg_f32, Self::write_reg) };
            (_ getrwf d2i) => { (Self::read_float_reg_f64, Self::write_reg) };
            (_ getrwf i2sr) => { (Self::read_reg, Self::write_float_reg_r32) };
            (_ getrwf i2dr) => { (Self::read_reg, Self::write_float_reg_r64) };
            (_ getrwf i2f) => { (Self::read_reg, Self::write_float_reg_f32) };
            (_ getrwf i2d) => { (Self::read_reg, Self::write_float_reg_f64) };
            (_ getrwf sr2sr) => { (Self::read_float_reg_r32, Self::write_float_reg_r32) };
            (_ getrwf dr2dr) => { (Self::read_float_reg_r64, Self::write_float_reg_r64) };
            (_ getrwf f2d) => { (Self::read_float_reg_f32, Self::write_float_reg_f64) };
            (_ getrwf d2f) => { (Self::read_float_reg_f64, Self::write_float_reg_f32) };
            (fop [$($f7: tt $ty: tt $r2: tt $rm: tt $exec: expr),* $(,)?]) => {{
                let r1 = (inst >> 15) & 0x1f;
                let r2 = (inst >> 20) & 0x1f;
                let rd = (inst >> 7) & 0x1f;
                let rm = (inst >> 12) & 7;

                match (inst >> 25, r2, rm) {
                    $(
                        ($f7, $r2, $rm) => {
                            let (r, w) = exec!(_ getrwf $ty);
                            let r1 = r(self, r1 as _);
                            let r2 = r(self, r2 as _);

                            let v = if stringify!($rm) == "_" {
                                float::set_rm!(self rm, || $exec(r1, r2))?
                            } else {
                                $exec(r1, r2)?
                            };

                            w(self, rd as _, v)?;
                        },
                    )*
                    _ => return Err(Exception::IllegalInst),
                }
            }};
            (_ getftyid f32) => { 0 };
            (_ getftyid f64) => { 1 };
            (r4f [$($ty: tt $exec: expr),* $(,)?]) => {{
                let r1 = (inst >> 15) & 0x1f;
                let r2 = (inst >> 20) & 0x1f;
                let r3 = (inst >> 27) & 0x1f;
                let rd = (inst >> 7) & 0x1f;
                let rm = (inst >> 12) & 7;

                match (inst >> 25) & 3 {
                    $(
                        exec!(_ getftyid $ty) => {
                            let (r, w) = exec!(_ getrwf $ty);
                            let r1 = r(self, r1 as _);
                            let r2 = r(self, r2 as _);
                            let r3 = r(self, r3 as _);
                            let v = float::set_rm!(self rm, || $exec(r1, r2, r3))?;
                            w(self, rd as _, v)?;
                        },
                    )*
                    _ => return Err(Exception::IllegalInst),
                }
            }};
        }

        match opc {
            0x37 => exec!(u |a| Ok(a)),
            0x17 => exec!(u |a| Ok(self.pc + a - 4)),
            0x6f => exec!(j |a| {
                self.write_pc(self.pc + a - 4)
            }),
            0x63 => exec!(b [
                0x0 |a, b| Ok(a == b),
                0x1 |a, b| Ok(a != b),
                0x4 |a, b| Ok((a as i64) < (b as i64)),
                0x5 |a, b| Ok((a as i64) >= (b as i64)),
                0x6 |a, b| Ok(a < b),
                0x7 |a, b| Ok(a >= b),
            ]),
            0x03 => exec!(i [
                0x0 |a, b| Ok(self.mmu_load_u8(a + b)? as i8 as u64),
                0x1 |a, b| Ok(self.mmu_load_u16(a + b)? as i16 as u64),
                0x2 |a, b| Ok(self.mmu_load_u32(a + b)? as i32 as u64),
                0x3 |a, b| Ok(self.mmu_load_u64(a + b)?),
                0x4 |a, b| Ok(self.mmu_load_u8(a + b)? as u64),
                0x5 |a, b| Ok(self.mmu_load_u16(a + b)? as u64),
                0x6 |a, b| Ok(self.mmu_load_u32(a + b)? as u64),
            ]),
            0x23 => exec!(s [
                0x0 |a, b, c| self.mmu_store_u8(a + c, b as _),
                0x1 |a, b, c| self.mmu_store_u16(a + c, b as _),
                0x2 |a, b, c| {
                    if testing && (a + c == 0x80001004 || a + c == 0x80002004) && b == 0 {
                        std::process::exit((self.bus.load_u32(a + c - 4).unwrap() - 1).min(1) as _);
                    } else if testing {
                        println!("{:016x} {}", a + c, b);
                    }

                    self.mmu_store_u32(a + c, b as _)
                },
                0x3 |a, b, c| self.mmu_store_u64(a + c, b as _),
            ]),
            0x13 => exec!(i [
                0x0 |a, b| Ok(a + b),
                0x2 |a, b| Ok(((a as i64) < (b as i64)) as u64),
                0x3 |a, b| Ok((a < b) as u64),
                0x4 |a, b| Ok(a ^ b),
                0x6 |a, b| Ok(a | b),
                0x7 |a, b| Ok(a & b),
                0x1 |a, b| Ok(a << (b & 0x3f)),
                0x5 |a, b| Ok(if b >> 6 == 0_u64 {
                    a >> (b & 0x3f)
                } else {
                    ((a as i64) >> (b & 0x3f)) as u64
                }),
            ]),
            0x33 => exec!(r [
                0x0 0x00 |a, b| Ok(a + b),
                0x0 0x20 |a, b| Ok(a - b),
                0x1 0x00 |a, b| Ok(a << (b & 0x3f)),
                0x2 0x00 |a, b| Ok(((a as i64) < (b as i64)) as u64),
                0x3 0x00 |a, b| Ok((a < b) as u64),
                0x4 0x00 |a, b| Ok(a ^ b),
                0x5 0x00 |a, b| Ok(a >> (b & 0x3f)),
                0x5 0x20 |a, b| Ok(((a as i64) >> (b & 0x3f)) as u64),
                0x6 0x00 |a, b| Ok(a | b),
                0x7 0x00 |a, b| Ok(a & b),

                0x0 0x01 |a, b| Ok(a * b),
                0x1 0x01 |a, b| Ok(((a as i64 as u128 * b as i64 as u128) >> 64) as u64),
                0x2 0x01 |a, b| Ok(((a as i64 as u128 * b as u128) >> 64) as u64),
                0x3 0x01 |a, b| Ok(((a as u128 * b as u128) >> 64) as u64),
                0x4 0x01 |a, b| Ok(if b != 0 {
                    (a as i64).wrapping_div(b as i64) as u64
                } else {
                    u64::MAX
                }),
                0x5 0x01 |a: u64, b| Ok(a.checked_div(b).unwrap_or(u64::MAX)),
                0x6 0x01 |a, b| Ok(if b != 0 {
                    (a as i64).wrapping_rem(b as i64) as u64
                } else {
                    a
                }),
                0x7 0x01 |a: u64, b| Ok(a.checked_rem(b).unwrap_or(a)),
            ]),
            0x67 => exec!(i [
                0x0 |a, b| self.write_pc((a + b) & !1),
            ]),
            0x1b => exec!(i [
                0x0 |a, b| Ok((a + b) as i32 as u64),
                0x1 |a, b| Ok((a << (b & 0x1f)) as i32 as u64),
                0x5 |a, b| Ok(if b >> 6 == 0_u64 {
                    ((a as u32) >> (b & 0x1f)) as i32 as u64
                } else {
                    ((a as i32) >> (b & 0x1f)) as u64
                }),
            ]),
            0x3b => exec!(r [
                0x0 0x00 |a, b| Ok((a + b) as i32 as u64),
                0x0 0x20 |a, b| Ok((a - b) as i32 as u64),
                0x1 0x00 |a, b| Ok((a << (b & 0x1f)) as i32 as u64),
                0x5 0x00 |a, b| Ok(((a as u32) >> (b & 0x1f)) as i32 as u64),
                0x5 0x20 |a, b| Ok(((a as i32) >> (b & 0x1f)) as u64),

                0x0 0x01 |a, b| Ok((a as i32 * b as i32) as u64),
                0x4 0x01 |a, b| Ok(if b != 0 {
                    (a as i32).wrapping_div(b as i32) as u64
                } else {
                    u64::MAX
                }),
                0x5 0x01 |a: u64, b| Ok((a as u32).checked_div(b as u32).map(|i| i as i32 as u64).unwrap_or(u64::MAX)),
                0x6 0x01 |a, b| Ok(if b != 0 {
                    (a as i32).wrapping_rem(b as i32) as u64
                } else {
                    a
                }),
                0x7 0x01 |a: u64, b| Ok((a as u32).checked_rem(b as u32).unwrap_or(a as u32) as i32 as u64),
            ]),
            0x0f => {},
            0x2f => exec!(amo [
                0x2 0x02 |a, _, aqrl| Ok(self.atomic_load_u32(a, aqrl)? as i32 as u64),
                0x2 0x03 |a, b, aqrl| Ok(self.atomic_store_u32(a, b as _, aqrl).is_err() as u64),

                0x3 0x02 |a, _, aqrl| Ok(self.atomic_load_u64(a, aqrl)?),
                0x3 0x03 |a, b, aqrl| Ok(self.atomic_store_u64(a, b, aqrl).is_err() as u64),

                0x2 0x01 |a, b, aqrl| Ok(self.atomic_mo_u32(a, aqrl, |_| b as u32)? as i32 as u64),
                0x2 0x00 |a, b, aqrl| Ok(self.atomic_mo_u32(a, aqrl, |a| a + b as u32)? as i32 as u64),
                0x2 0x04 |a, b, aqrl| Ok(self.atomic_mo_u32(a, aqrl, |a| a ^ b as u32)? as i32 as u64),
                0x2 0x0c |a, b, aqrl| Ok(self.atomic_mo_u32(a, aqrl, |a| a & b as u32)? as i32 as u64),
                0x2 0x08 |a, b, aqrl| Ok(self.atomic_mo_u32(a, aqrl, |a| a | b as u32)? as i32 as u64),
                0x2 0x10 |a, b, aqrl| Ok(self.atomic_mo_u32(a, aqrl, |a| (a as i32).min(b as i32) as u32)? as i32 as u64),
                0x2 0x14 |a, b, aqrl| Ok(self.atomic_mo_u32(a, aqrl, |a| (a as i32).max(b as i32) as u32)? as i32 as u64),
                0x2 0x18 |a, b, aqrl| Ok(self.atomic_mo_u32(a, aqrl, |a| a.min(b as u32))? as i32 as u64),
                0x2 0x1c |a, b, aqrl| Ok(self.atomic_mo_u32(a, aqrl, |a| a.max(b as u32))? as i32 as u64),

                0x3 0x01 |a, b, aqrl| Ok(self.atomic_mo_u64(a, aqrl, |_| b)?),
                0x3 0x00 |a, b, aqrl| Ok(self.atomic_mo_u64(a, aqrl, |a| a + b)?),
                0x3 0x04 |a, b, aqrl| Ok(self.atomic_mo_u64(a, aqrl, |a| a ^ b)?),
                0x3 0x0c |a, b, aqrl| Ok(self.atomic_mo_u64(a, aqrl, |a| a & b)?),
                0x3 0x08 |a, b, aqrl| Ok(self.atomic_mo_u64(a, aqrl, |a| a | b)?),
                0x3 0x10 |a, b, aqrl| Ok(self.atomic_mo_u64(a, aqrl, |a| (a as i64).min(b as i64) as u64)?),
                0x3 0x14 |a, b, aqrl| Ok(self.atomic_mo_u64(a, aqrl, |a| (a as i64).max(b as i64) as u64)?),
                0x3 0x18 |a, b, aqrl| Ok(self.atomic_mo_u64(a, aqrl, |a| a.min(b))?),
                0x3 0x1c |a, b, aqrl| Ok(self.atomic_mo_u64(a, aqrl, |a| a.max(b))?),
            ]),
            0x07 => exec!(ix [
                0x2 write_float_reg_r32 |a, b| Ok(self.mmu_load_u32(a + b)?),
                0x3 write_float_reg_r64 |a, b| Ok(self.mmu_load_u64(a + b)?),
            ]),
            0x27 => exec!(sx [
                0x2 read_float_reg_r32_uc |a, b, c| if self.can_use_fp() { self.mmu_store_u32(a + c, b as _) } else { Ok(()) },
                0x3 read_float_reg_r64_uc |a, b, c| if self.can_use_fp() { self.mmu_store_u64(a + c, b) } else { Ok(()) },
            ]),
            0x43 => exec!(r4f [
                f32 |a, b, c| Ok(self.float_do_op_f32(|| a * b + c)),
                f64 |a, b, c| Ok(self.float_do_op_f64(|| a * b + c)),
            ]),
            0x47 => exec!(r4f [
                f32 |a, b, c| Ok(self.float_do_op_f32(|| a * b - c)),
                f64 |a, b, c| Ok(self.float_do_op_f64(|| a * b - c)),
            ]),
            0x4b => exec!(r4f [
                f32 |a, b, c| Ok(self.float_do_op_f32(|| -((a * b) as f32) + c)),
                f64 |a, b, c| Ok(self.float_do_op_f64(|| -((a * b) as f64) + c)),
            ]),
            0x4f => exec!(r4f [
                f32 |a, b, c| Ok(self.float_do_op_f32(|| -((a * b) as f32) - c)),
                f64 |a, b, c| Ok(self.float_do_op_f64(|| -((a * b) as f64) - c)),
            ]),
            0x53 => exec!(fop [
                0x00 f32 _ _ |a, b| Ok(self.float_do_op_f32(|| a + b)),
                0x04 f32 _ _ |a, b| Ok(self.float_do_op_f32(|| a - b)),
                0x08 f32 _ _ |a, b| Ok(self.float_do_op_f32(|| a * b)),
                0x0c f32 _ _ |a, b| Ok(self.float_do_op_f32(|| a / b)),
                0x2c f32 0 _ |a: f32, _| Ok(self.float_do_op_f32(|| a.sqrt())),
                0x10 sr2sr _ 0 |a, b| Ok((a & 0x7fff_ffff) | (b & 0x8000_0000)),
                0x10 sr2sr _ 1 |a, b: u32| Ok((a & 0x7fff_ffff) | (!b & 0x8000_0000)),
                0x10 sr2sr _ 2 |a, b| Ok((a & 0x7fff_ffff) | ((a ^ b) & 0x8000_0000)),
                0x14 f32 _ 0 |a: f32, b: f32| Ok(float::minmax!(f32 self a b min)),
                0x14 f32 _ 1 |a: f32, b: f32| Ok(float::minmax!(f32 self a b max)),
                0x60 f2i 0 _ |a: f32, _| Ok(self.float_do_op(|s| float::cast!(s a f32 i32 s) as u64)),
                0x60 f2i 1 _ |a: f32, _| Ok(self.float_do_op(|s| float::cast!(s a f32 u32 u) as i32 as u64)),
                0x60 f2i 2 _ |a: f32, _| Ok(self.float_do_op(|s| float::cast!(s a f32 i64 s) as u64)),
                0x60 f2i 3 _ |a: f32, _| Ok(self.float_do_op(|s| float::cast!(s a f32 u64 u))),
                0x70 sru2i 0 0 |a, _| Ok(a as i32 as u64),
                0x50 f2i _ 2 |a, b| Ok(self.float_cmp(a, b, a == b, Snan::is_snan)),
                0x50 f2i _ 1 |a, b| Ok(self.float_cmp(a, b, a < b, f32::is_nan)),
                0x50 f2i _ 0 |a, b| Ok(self.float_cmp(a, b, a <= b, f32::is_nan)),
                0x70 f2i 0 1 |a: f32, _| Ok(
                    ((a == f32::NEG_INFINITY) as u64) << 0
                    | ((a.is_sign_negative() && a.is_normal()) as u64) << 1
                    | ((a.is_sign_negative() && a.is_subnormal()) as u64) << 2
                    | ((a.is_sign_negative() && a == -0.0) as u64) << 3
                    | ((a.is_sign_positive() && a == 0.0) as u64) << 4
                    | ((a.is_sign_positive() && a.is_subnormal()) as u64) << 5
                    | ((a.is_sign_positive() && a.is_normal()) as u64) << 6
                    | ((a == f32::INFINITY) as u64) << 7
                    | (a.is_snan() as u64) << 8
                    | ((!a.is_snan() && a.is_nan()) as u64) << 9
                ),
                0x68 i2f 0 _ |a, _| Ok(a as i32 as f32),
                0x68 i2f 1 _ |a, _| Ok(a as u32 as f32),
                0x68 i2f 2 _ |a, _| Ok(a as i64 as f32),
                0x68 i2f 3 _ |a, _| Ok(a as u64 as f32),
                0x78 i2sr 0 0 |a, _| Ok(a as u32),

                0x01 f64 _ _ |a, b| Ok(self.float_do_op_f64(|| a + b)),
                0x05 f64 _ _ |a, b| Ok(self.float_do_op_f64(|| a - b)),
                0x09 f64 _ _ |a, b| Ok(self.float_do_op_f64(|| a * b)),
                0x0d f64 _ _ |a, b| Ok(self.float_do_op_f64(|| a / b)),
                0x2d f64 0 _ |a: f64, _| Ok(self.float_do_op_f64(|| a.sqrt())),
                0x11 dr2dr _ 0 |a, b| Ok((a & 0x7fff_ffff_ffff_ffff) | (b & 0x8000_0000_0000_0000)),
                0x11 dr2dr _ 1 |a, b: u64| Ok((a & 0x7fff_ffff_ffff_ffff) | (!b & 0x8000_0000_0000_0000)),
                0x11 dr2dr _ 2 |a, b| Ok((a & 0x7fff_ffff_ffff_ffff) | ((a ^ b) & 0x8000_0000_0000_0000)),
                0x15 f64 _ 0 |a: f64, b: f64| Ok(float::minmax!(f64 self a b min)),
                0x15 f64 _ 1 |a: f64, b: f64| Ok(float::minmax!(f64 self a b max)),
                0x20 d2f 1 _ |a, _| Ok(self.float_do_op_f32(|| a as f32)),
                0x21 f2d 0 _ |a, _| Ok(self.float_do_op_f64(|| a as f64)),
                0x61 d2i 0 _ |a: f64, _| Ok(self.float_do_op(|s| float::cast!(s a f64 i32 s) as u64)),
                0x61 d2i 1 _ |a: f64, _| Ok(self.float_do_op(|s| float::cast!(s a f64 u32 u) as i32 as u64)),
                0x61 d2i 2 _ |a: f64, _| Ok(self.float_do_op(|s| float::cast!(s a f64 i64 s) as u64)),
                0x61 d2i 3 _ |a: f64, _| Ok(self.float_do_op(|s| float::cast!(s a f64 u64 u))),
                0x71 dru2i 0 0 |a, _| Ok(a),
                0x51 d2i _ 2 |a, b| Ok(self.float_cmp(a, b, a == b, Snan::is_snan)),
                0x51 d2i _ 1 |a, b| Ok(self.float_cmp(a, b, a < b, f64::is_nan)),
                0x51 d2i _ 0 |a, b| Ok(self.float_cmp(a, b, a <= b, f64::is_nan)),
                0x71 d2i 0 1 |a: f64, _| Ok(
                    ((a == f64::NEG_INFINITY) as u64) << 0
                    | ((a.is_sign_negative() && a.is_normal()) as u64) << 1
                    | ((a.is_sign_negative() && a.is_subnormal()) as u64) << 2
                    | ((a.is_sign_negative() && a == -0.0) as u64) << 3
                    | ((a.is_sign_positive() && a == 0.0) as u64) << 4
                    | ((a.is_sign_positive() && a.is_subnormal()) as u64) << 5
                    | ((a.is_sign_positive() && a.is_normal()) as u64) << 6
                    | ((a == f64::INFINITY) as u64) << 7
                    | (a.is_snan() as u64) << 8
                    | ((!a.is_snan() && a.is_nan()) as u64) << 9
                ),
                0x69 i2d 0 _ |a, _| Ok(a as i32 as f64),
                0x69 i2d 1 _ |a, _| Ok(a as u32 as f64),
                0x69 i2d 2 _ |a, _| Ok(a as i64 as f64),
                0x69 i2d 3 _ |a, _| Ok(a as u64 as f64),
                0x79 i2dr 0 0 |a, _| Ok(a),
            ]),
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
                User 0x3 _ _ sr _ |a: u64, _, b| { // csrrc
                    let rv = self.csr_read(b)?;

                    if sr != 0 {
                        self.csr_write(b, !a & rv)?;
                    }

                    Ok(Some(rv))
                },
                User 0x7 _ _ a _ |_, _, b| { // csrrci
                    let rv = self.csr_read(b)?;

                    if a != 0 {
                        self.csr_write(b, !(a as u64) & rv)?;
                    }

                    Ok(Some(rv))
                },
                Supervisor 0x0 0x08 0x00 0x00 0x02 |_, _, _| { // sret
                    if (self.csr_read_cpu(csr::CSR_MSTATUS) >> 22) & 1 == 1 {
                        return Err(Exception::IllegalInst);
                    }

                    let epc = self.csr_read_cpu(csr::CSR_SEPC);
                    self.write_pc(epc)?;

                    let mut mstat = self.csr_read_cpu(csr::CSR_MSTATUS);
                    mstat &= !2;
                    mstat |= (mstat >> 4) & 2; // sIE = sPIE

                    let mode = Mode::from_code((mstat >> 8) & 1);

                    mstat &= !0x20;
                    mstat |= 1 << 5; // sPIE = 1

                    mstat &= !0x100; // sPP = user

                    self.mode = mode;
                    println!("{:?}", self.mode);
                    self.csr_write_cpu(csr::CSR_MSTATUS, mstat);
                    Ok(None)
                },
                Machine 0x0 0x18 0x00 0x00 0x02 |_, _, _| { // mret
                    let epc = self.csr_read_cpu(csr::CSR_MEPC);
                    self.write_pc(epc)?;

                    let mut mstat = self.csr_read_cpu(csr::CSR_MSTATUS);
                    mstat &= !8;
                    mstat |= (mstat >> 4) & 8; // mIE = mPIE

                    let mode = Mode::from_code((mstat >> 11) & 3);

                    mstat &= !0x80;
                    mstat |= 1 << 7; // mPIE = 1

                    mstat &= !0x1800; // mPP = user

                    if mode != Mode::Machine {
                        mstat &= !0x20000; // mPRV = 0
                    }

                    self.mode = mode;
                    println!("{:?}", self.mode);
                    self.csr_write_cpu(csr::CSR_MSTATUS, mstat);
                    Ok(None)
                },
                User 0x0 0x00 0x00 0x00 0x00 |_, _, _| { // ecall
                    self.exception(self.mode.ecall_exception());
                    Ok(None)
                },
                User 0x0 0x00 0x00 0x00 0x01 |_, _, _| { // ebreak
                    self.exception(Exception::Breakpoint);
                    Ok(None)
                },
                User 0x0 0x08 0x00 0x00 0x05 |_, _, _| Ok(None), // wfi
                Supervisor 0x0 0x09 0x00 _ _ |_, _, _| self.flush_mapping().map(|_| None), // sfence.vma
            ]),
            _ => return Err(Exception::IllegalInst),
        }

        Ok(())
    }

    fn exception(&mut self, cause: Exception) {
        println!("{cause:?} {:016x}", self.pc);
        let epc = self.pc;
        let tval = if self.trap(cause as _, csr::CSR_MEDELEG) {
            csr::CSR_MTVAL
        } else {
            csr::CSR_STVAL
        };

        let cause = match cause {
            Exception::IllegalInst => {
                let i = self.mmu_load_xu32(epc - 4).unwrap_or(0);
                println!("{i:08x}");
                i as _
            },
            _ => 0,
        };
        self.csr_write_cpu(tval, cause);
    }

    fn interrupt(&mut self, cause: u64) {
        // println!("{cause} {:016x}", self.pc);
        self.trap(cause | (1 << 63), csr::CSR_MIDELEG);
    }

    fn trap(&mut self, cause: u64, deleg: u64) -> bool {
        let cause_bit = cause & 0x3f;
        let deleg = self.csr_read_cpu(deleg);

        if self.mode != Mode::Machine && (deleg >> cause_bit) & 1 == 1 {
            self.supervisor_trap(cause);
            false
        } else {
            self.machine_trap(cause);
            true
        }
    }

    fn mtvec_jump(&mut self, mtvec: u64, cause: u64) {
        let pc = match mtvec & 3 {
            0 => mtvec,
            1 => (mtvec & !3) + 4 * (cause & 0x7fff_ffff_ffff_ffff),
            _ => unimplemented!(),
        };
        self.pc = pc;

        // println!("{mtvec:016x} {pc:016x}");
    }

    fn machine_trap(&mut self, cause: u64) {
        self.csr_write_cpu(csr::CSR_MCAUSE, cause);
        self.csr_write_cpu(csr::CSR_MEPC, self.pc - 4);

        let mut mstat = self.csr_read_cpu(csr::CSR_MSTATUS);
        mstat &= !0x1880;
        mstat |= (mstat & 8) << 4;
        mstat &= !8;
        mstat |= (self.mode as u64) << 11;
        // mstat &= !0x20000; // HACK: mPRV = 0
        self.csr_write_cpu(csr::CSR_MSTATUS, mstat);

        let mtvec = self.csr_read_cpu(csr::CSR_MTVEC);
        self.mtvec_jump(mtvec, cause);
        self.mode = Mode::Machine;
        println!("{:?}", self.mode);
    }

    fn supervisor_trap(&mut self, cause: u64) {
        self.csr_write_cpu(csr::CSR_SCAUSE, cause);
        self.csr_write_cpu(csr::CSR_SEPC, self.pc - 4);

        let mut mstat = self.csr_read_cpu(csr::CSR_MSTATUS);
        mstat &= !0x120;
        mstat |= (mstat & 2) << 4;
        mstat &= !2;
        mstat |= (self.mode as u64) << 8;
        self.csr_write_cpu(csr::CSR_MSTATUS, mstat);

        let stvec = self.csr_read_cpu(csr::CSR_STVEC);
        self.mtvec_jump(stvec, cause);
        self.mode = Mode::Supervisor;
        println!("{:?}", self.mode);
    }

    fn check_interrupts(&mut self) {
        const CHECK_LIST: &[usize] = &[13, 11, 7, 3, 9, 5, 1];

        let mut mip = self.csr_read_cpu(csr::CSR_MIP);
        let mut mie = self.csr_read_cpu(csr::CSR_MIE);
        let mstat_mie = self.csr_read_cpu(csr::CSR_MSTATUS) & 0x8 != 0;
        let delg = self.csr_read_cpu(csr::CSR_MIDELEG);

        for b in CHECK_LIST.into_iter() {
            let p = (mip >> b) & 1 == 1;
            let e = (mie >> b) & 1 == 1;
            let delg = (delg >> b) & 1 == 1;

            if ((self.mode == Mode::Machine && mstat_mie) || self.mode < Mode::Machine) && (p && e) && !delg {
                self.interrupt(*b as _);
                mip ^= 1 << b;
                mie = 0;
                self.csr_write_cpu(csr::CSR_MIP, mip);
                self.csr_write_cpu(csr::CSR_MIE, mie);
                break;
            }
        }
    }

    fn read_reg(&self, r: usize) -> u64 {
        if r == 0 { 0 } else { self.regs[r - 1] }
    }

    fn write_reg(&mut self, r: usize, v: u64) -> Result<(), Exception> {
        if r != 0 { self.regs[r - 1] = v; }
        Ok(())
    }

    fn write_pc(&mut self, v: u64) -> Result<u64, Exception> {
        if v & 3 == 0 {
            Ok(core::mem::replace(&mut self.pc, v))
        } else {
            Err(Exception::InstAddrMisalign)
        }
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

    fn from_code(c: u64) -> Self {
        match c {
            0 => Self::User,
            1 => Self::Supervisor,
            2 => Self::Hypervisor,
            3 => Self::Machine,
            _ => panic!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Exception {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Interrupt {
    SupervisorSoftwareInt = 1,
    MachineSoftwareInt = 3,
    SupervisorTimerInt = 5,
    MachineTimerInt = 7,
    SupervisorExternalInt = 9,
    MachineExternalInt = 11,
    CounterOverflowInt = 13,
}

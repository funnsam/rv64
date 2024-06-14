use super::*;

pub struct Cpu<'a> {
    bus: &'a mut bus::Bus<'a>,
    io: &'a mut io::Io,

    regs: [u64; 31],
    pc: u64,
}

impl<'a> Cpu<'a> {
    pub fn new(bus: &'a mut bus::Bus<'a>, io: &'a mut io::Io) -> Self {
        Self {
            bus,
            io,

            regs: [0; 31],
            pc: 0,
        }
    }

    pub fn step(&mut self) {
        let inst = self.fetch();
        self.execute(inst);
    }

    fn fetch(&mut self) -> u32 {
        let i = self.bus.load_u32(self.pc);
        self.pc += 4;
        i
    }

    fn execute(&mut self, inst: u32) {
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
                    (f3, f7) => todo!("r {opc:02x} {f3:01x} {f7:02x}"),
                };

                self.write_reg(rd as _, v);
            }};
            (i [$($f3: tt $exec: expr),* $(,)?]) => {{
                let rs = (inst >> 15) & 0x1f;
                let rs = self.read_reg(rs as _);
                let im = (inst as i32 >> 20) as u64;
                let rd = (inst >> 7) & 0x1f;

                let v = match (inst >> 12) & 7 {
                    $(
                        $f3 => $exec(rs, im),
                    )*
                    f3 => todo!("i {opc:02x} {f3:01x}"),
                };

                self.write_reg(rd as _, v);
            }};
            (s [$($f3: tt $exec: expr),* $(,)?]) => {{
                let r1 = (inst >> 15) & 0x1f;
                let r1 = self.read_reg(r1 as _);
                let r2 = (inst >> 20) & 0x1f;
                let r2 = self.read_reg(r2 as _);
                let im = ((inst & 0xe000_0000) as i32 >> 20) as u64 // [11:5]
                    | ((inst >> 7) & 0x1f) as u64; // [4:0]
                match (inst >> 12) & 7 {
                    $(
                        $f3 => $exec(r1, r2, im),
                    )*
                    f3 => todo!("s {opc:02x} {f3:01x}"),
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
                    f3 => todo!("b {opc:02x} {f3:01x}"),
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
        }

        match opc {
            0x37 => exec!(u |a| a),
            0x17 => exec!(u |a| self.pc + a),
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
                0x2 |a, b, c| self.bus.store_u32(a + c, b as _),
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
                0x0 |a, b| core::mem::replace(&mut self.pc, (a + b) & !1 - 4), // TODO: verify
            ]),
            0x73 => exec!(i [
                0x1 |a, b| {
                    println!("{a}");
                    0
                },
            ]),
            _ => todo!("{opc:02x}"),
        }
    }

    fn read_reg(&self, r: usize) -> u64 {
        if r == 0 { 0 } else { self.regs[r - 1] }
    }

    fn write_reg(&mut self, r: usize, v: u64) {
        if r != 0 { self.regs[r - 1] = v; }
    }
}

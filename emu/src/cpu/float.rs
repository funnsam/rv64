use super::*;

impl<'a> Cpu<'a> {
    pub(crate) fn read_float_reg(&self, n: usize) -> u64 { self.float_regs[n] }
    pub(crate) fn write_float_reg(&mut self, n: usize, d: u64) { self.float_regs[n] = d }
}

macro_rules! gen {
    ($t: tt $width: tt $r: tt $w: tt) => {
        impl<'a> Cpu<'a> {
            pub(crate) fn $r(&self, n: usize) -> $t {
                $t::from_bits(self.read_float_reg(n) as $width)
            }

            pub(crate) fn $w(&mut self, n: usize, d: $t) {
                self.write_float_reg(n, d.to_bits() as u64)
            }
        }
    };
}

gen!(f32 u32 read_float_reg_f32 write_float_reg_f32);
gen!(f64 u64 read_float_reg_f64 write_float_reg_f64);

// https://github.com/rust-lang/rust/issues/48825
pub(crate) fn f32_is_signaling_nan(f: f32) -> bool {
    let uf: u32 = f.to_bits();
    let signal_bit = 1 << 22;
    let signal_bit_clear = (uf & signal_bit) == 0;
    f32::is_nan(f) && signal_bit_clear
}

pub(crate) fn f64_is_signaling_nan(f: f64) -> bool {
    let uf: u64 = f.to_bits();
    let signal_bit = 1 << 51;
    let signal_bit_clear = (uf & signal_bit) == 0;
    f64::is_nan(f) && signal_bit_clear
}

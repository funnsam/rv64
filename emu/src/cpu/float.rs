use super::*;

pub(crate) const NV: u64 = 0x10;
pub(crate) const DZ: u64 = 0x08;
pub(crate) const OF: u64 = 0x04;
pub(crate) const UF: u64 = 0x02;
pub(crate) const NX: u64 = 0x01;

impl<'a> Cpu<'a> {
    pub(crate) fn read_float_reg(&self, n: usize) -> u64 { self.float_regs[n] }
    pub(crate) fn write_float_reg(&mut self, n: usize, d: u64) { self.float_regs[n] = d }

    pub(crate) fn float_set_flags(&mut self, f: u64) {
        let o = self.csr_read_cpu(csr::CSR_FCSR);
        self.csr_write_cpu(csr::CSR_FCSR, o | f);
    }

    pub(crate) fn float_cmp<T, F: Fn(T) -> bool>(&mut self, a: T, b: T, c: bool, nv: F) -> u64 {
        if nv(a) || nv(b) {
            self.float_set_flags(NV);
            0
        } else {
            c as _
        }
    }

    pub(crate) fn check_fpu(&mut self) {
        unsafe {
            let f = (fenv::fetestexcept(fenv::FE_INVALID as _) != 0) as u64 * NV
                | (fenv::fetestexcept(fenv::FE_DIVBYZERO as _) != 0) as u64 * DZ
                | (fenv::fetestexcept(fenv::FE_INEXACT as _) != 0) as u64 * NX
                | (fenv::fetestexcept(fenv::FE_OVERFLOW as _) != 0) as u64 * OF
                | (fenv::fetestexcept(fenv::FE_UNDERFLOW as _) != 0) as u64 * UF;
            println!("{f:05b}");
            self.float_set_flags(f);
        }
    }

    pub(crate) fn float_do_op<T, F: Fn(&mut Self) -> T>(&mut self, f: F) -> T {
        unsafe { fenv::feclearexcept(fenv::FE_ALL_EXCEPT as _); }
        let v = f(self);
        v
    }

    pub(crate) fn float_do_op_f32<F: Fn() -> f32>(&mut self, f: F) -> f32 {
        unsafe { fenv::feclearexcept(fenv::FE_ALL_EXCEPT as _); }
        let v = f();
        self.check_fpu();
        if v.is_nan() {
            f32::from_bits(0x7fc0_0000)
        } else {
            v
        }
    }

    pub(crate) fn get_mode(&self, m: u32, d: bool) -> Result<i32, Exception> {
        Ok((match (m, d) {
            (0 | 4, _) => fenv::FE_TONEAREST,
            (1, _) => fenv::FE_TOWARDZERO,
            (2, _) => fenv::FE_DOWNWARD,
            (3, _) => fenv::FE_UPWARD,
            (7, false) => {
                let c = self.csr_read_cpu(csr::CSR_FCSR);
                return self.get_mode(((c >> 5) & 7) as _, true);
            },
            _ => return Err(Exception::IllegalInst),
        }) as _)
    }
}

#[macro_export]
macro_rules! set_rm {
    ($s: tt $m: expr, $f: expr) => {
        unsafe {
            let rm = fenv::fegetround();
            fenv::fesetround($s.get_mode($m, false)?);
            let v = $f();
            fenv::fesetround(rm);
            v
        }
    };
}

#[macro_export]
macro_rules! cast {
    ($s: tt $v: tt $f: tt $t: tt s) => {{
        let v = if $v.is_nan() { $t::MAX } else { $v as $t };
        $s.check_fpu();
        v
    }};
    ($s: tt $v: tt $f: tt $t: tt u) => {{
        let need_set = if $v.fract() == 0.0 && $v.is_sign_negative() {
            $s.float_set_flags($crate::cpu::float::NV);
            false
        } else if $v.fract() != 0.0 {
            $s.float_set_flags($crate::cpu::float::NX);
            false
        } else {
            true
        };
        let v = if $v.is_nan() { $t::MAX } else { $v as $t };
        if need_set && v as $f != $v { $s.check_fpu(); }
        v
    }};
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
pub(crate) fn f32_is_snan(f: f32) -> bool {
    let uf: u32 = f.to_bits();
    let signal_bit = 1 << 22;
    let signal_bit_clear = (uf & signal_bit) == 0;
    f32::is_nan(f) && signal_bit_clear
}

pub(crate) fn f64_is_snan(f: f64) -> bool {
    let uf: u64 = f.to_bits();
    let signal_bit = 1 << 51;
    let signal_bit_clear = (uf & signal_bit) == 0;
    f64::is_nan(f) && signal_bit_clear
}

pub(crate) use {set_rm, cast};

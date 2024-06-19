use super::*;

pub(crate) const NV: u64 = 0x10;
pub(crate) const DZ: u64 = 0x08;
pub(crate) const OF: u64 = 0x04;
pub(crate) const UF: u64 = 0x02;
pub(crate) const NX: u64 = 0x01;

pub(crate) const F32_CNAN: u32 = 0x7fc0_0000;
pub(crate) const F64_CNAN: u64 = 0x7ff8_0000_0000_0000;

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
            f32::from_bits(F32_CNAN)
        } else {
            v
        }
    }

    pub(crate) fn float_do_op_f64<F: Fn() -> f64>(&mut self, f: F) -> f64 {
        unsafe { fenv::feclearexcept(fenv::FE_ALL_EXCEPT as _); }
        let v = f();
        self.check_fpu();
        if v.is_nan() {
            f64::from_bits(F64_CNAN)
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

#[macro_export]
macro_rules! minmax {
    (zh $a: tt $b: tt min) => {
        if $a.is_sign_negative() || $b.is_sign_negative() { -0.0 } else { 0.0 }
    };
    (zh $a: tt $b: tt max) => {
        if $a.is_sign_positive() || $b.is_sign_positive() { 0.0 } else { -0.0 }
    };
    (cnan f32) => {
        f32::from_bits($crate::cpu::float::F32_CNAN)
    };
    (cnan f64) => {
        f64::from_bits($crate::cpu::float::F64_CNAN)
    };
    ($t: tt $n: tt $s: tt $a: tt $b: tt $op: tt) => {{
        if $crate::cpu::float::$n($a) || $crate::cpu::float::$n($b) {
            $s.float_set_flags($crate::cpu::float::NV);
        }

        let v = $a.$op($b);
        if $a.abs() == 0.0 && $b.abs() == 0.0 {
            minmax!(zh $a $b $op)
        } else {
            if v.is_nan() {
                minmax!(cnan $t)
            } else {
                v
            }
        }
    }};
}

macro_rules! gen {
    ($t: tt $width: tt $r: tt $w: tt $rr: tt $rw: tt $cnan: tt) => {
        impl<'a> Cpu<'a> {
            pub(crate) fn $rr(&self, n: usize) -> $width {
                let r = self.read_float_reg(n);

                if r as u64 & (u64::MAX ^ $width::MAX as u64) != (u64::MAX ^ $width::MAX as u64) {
                    $cnan
                } else {
                    r as $width
                }
            }

            pub(crate) fn $rw(&mut self, n: usize, d: $width) {
                self.write_float_reg(n, (u64::MAX ^ $width::MAX as u64) | (d as u64))
            }

            pub(crate) fn $r(&self, n: usize) -> $t { $t::from_bits(self.$rr(n)) }
            pub(crate) fn $w(&mut self, n: usize, d: $t) { self.$rw(n, d.to_bits()) }
        }
    };
}

gen!(f32 u32 read_float_reg_f32 write_float_reg_f32 read_float_reg_r32 write_float_reg_r32 F32_CNAN);
gen!(f64 u64 read_float_reg_f64 write_float_reg_f64 read_float_reg_r64 write_float_reg_r64 F64_CNAN);

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

pub(crate) fn nan_box_u32(d: u32) -> u64 {
    (u64::MAX ^ u32::MAX as u64) | (d as u64)
}

pub(crate) use {cast, set_rm, minmax};
